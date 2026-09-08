use std::any::TypeId;

use crate::NodeId;

/// Tree bookkeeping for one node. The widget itself lives in a
/// [`WidgetStore`](crate::widget_store::WidgetStore); the node only records which one.
pub(crate) struct Node {
    /// `None` exactly for the root.
    pub type_id: Option<TypeId>,
    /// The root is its own parent; every other node has a real one.
    pub parent: NodeId,
    /// In sibling order.
    pub children: Vec<NodeId>,
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
