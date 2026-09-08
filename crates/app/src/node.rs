use std::any::{Any, TypeId};

use crate::{App, NodeId};

/// An erased per-node handler. Built by [`Spawner::on`](crate::Spawner::on)
/// around the user's plain `fn`; it downcasts the event, takes the widget out
/// of its store, builds the [`Context`](crate::Context), and puts the widget
/// back. Registration order is preserved by the `Vec` it lives in.
pub(crate) struct Handler {
    pub event: TypeId,
    pub run: HandlerFn,
}

pub(crate) type HandlerFn = Box<dyn Fn(&mut App, NodeId, &dyn Any)>;

/// Tree bookkeeping for one node. The widget itself lives in a
/// [`WidgetStore`](crate::widget_store::WidgetStore); the node only records which one.
pub(crate) struct Node {
    /// The node's own id, so a slot walk can name it.
    pub id: NodeId,
    /// `None` exactly for the root.
    pub type_id: Option<TypeId>,
    /// The root is its own parent; every other node has a real one.
    pub parent: NodeId,
    /// In sibling order.
    pub children: Vec<NodeId>,
    /// In registration order. Empty for the root.
    pub handlers: Vec<Handler>,
}

struct Slot {
    /// Bumped every time the slot is freed. An id is live only if its
    /// generation matches the slot's *and* the slot is occupied. At one bump
    /// per free, a `u64` never wraps in practice.
    generation: u64,
    node: Option<Node>,
}

/// Slot arena for nodes. Freed slots are reused (LIFO) before the arena grows.
pub(crate) struct Nodes {
    slots: Vec<Slot>,
    free: Vec<u64>,
}

impl Nodes {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    /// Claim an empty slot. Returns `(index, generation)`; the slot stays
    /// unoccupied until [`Nodes::fill`].
    pub fn reserve(&mut self) -> (u64, u64) {
        if let Some(index) = self.free.pop() {
            return (index, self.slots[index as usize].generation);
        }
        self.slots.push(Slot {
            generation: 0,
            node: None,
        });
        (self.slots.len() as u64 - 1, 0)
    }

    /// Number of slots, occupied or not. A component column is sized to it.
    pub fn len(&self) -> u64 {
        self.slots.len() as u64
    }

    pub fn fill(&mut self, index: u64, node: Node) {
        let slot = &mut self.slots[index as usize];
        debug_assert!(slot.node.is_none());
        slot.node = Some(node);
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        let slot = self.slots.get(id.component_index() as usize)?;
        if slot.generation != id.generation() {
            return None;
        }
        slot.node.as_ref()
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        let slot = self.slots.get_mut(id.component_index() as usize)?;
        if slot.generation != id.generation() {
            return None;
        }
        slot.node.as_mut()
    }

    /// Every live node's id, in slot order. Used by
    /// [`App::emit_all`](crate::App::emit_all); the ids are snapshotted at
    /// emit time so the walk is stable while handlers run.
    pub fn live_ids(&self) -> Vec<NodeId> {
        self.slots
            .iter()
            .filter_map(|slot| slot.node.as_ref().map(|node| node.id))
            .collect()
    }

    /// Empty one slot and invalidate every id that names it. Returns the
    /// node so the caller can continue into its children.
    ///
    /// # Safety
    ///
    /// This frees exactly one node slot and nothing else. The caller must:
    ///
    /// - have validated `index` through [`Nodes::get`] for a live id;
    /// - free the returned node's `children` (recursively) — otherwise they
    ///   and their widgets leak, unreachable from the tree;
    /// - free the matching widget slot in the node's store;
    /// - have unlinked the node from its parent's `children`.
    ///
    /// [`App::remove`](crate::App::remove) is the only intended caller; it
    /// does all four for a whole subtree.
    pub unsafe fn free(&mut self, index: u64) -> Node {
        let slot = &mut self.slots[index as usize];
        let node = slot.node.take().expect("freeing an empty node slot");
        slot.generation += 1;
        self.free.push(index);
        node
    }
}
