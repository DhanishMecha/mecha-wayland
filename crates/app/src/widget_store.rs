use std::any::Any;

use crate::{NodeId, Widget};

/// A widget plus the id of the node it belongs to. The back-pointer is what
/// lets a store validate a lookup and lets per-column iteration report which
/// node each widget is.
pub(crate) struct WidgetWrapper<W> {
    pub node_id: NodeId,
    pub widget: W,
}

/// Object-safe view of a [`WidgetStore`], so `App` can hold one column per widget
/// type without knowing the types, and can free a slot during subtree removal
/// without knowing what's in it.
pub(crate) trait AnyWidgetStore {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn as_any(&self) -> &dyn Any;
    fn free(&mut self, index: u64);
}

/// One column: every live widget of type `W`. Freed slots are reused (LIFO)
/// before the column grows. Slot order is not tree order and not stable.
pub(crate) struct WidgetStore<W> {
    slots: Vec<Option<WidgetWrapper<W>>>,
    free: Vec<u64>,
}

impl<W: Widget> WidgetStore<W> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    /// Claim an empty slot without filling it. See [`WidgetStore::fill`].
    pub fn reserve(&mut self) -> u64 {
        if let Some(index) = self.free.pop() {
            return index;
        }
        self.slots.push(None);
        self.slots.len() as u64 - 1
    }

    pub fn fill(&mut self, index: u64, wrapper: WidgetWrapper<W>) {
        let slot = &mut self.slots[index as usize];
        debug_assert!(slot.is_none());
        *slot = Some(wrapper);
    }

    /// The node's generation has already been checked by the caller; the
    /// `node_id` comparison here guards the widget slot having been reused
    /// under a node that (through a bug) still points at it.
    pub fn get(&self, id: NodeId) -> Option<&W> {
        let wrapper = self.slots.get(id.widget_index() as usize)?.as_ref()?;
        (wrapper.node_id == id).then_some(&wrapper.widget)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut W> {
        let wrapper = self.slots.get_mut(id.widget_index() as usize)?.as_mut()?;
        (wrapper.node_id == id).then_some(&mut wrapper.widget)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (NodeId, &mut W)> {
        self.slots
            .iter_mut()
            .filter_map(|slot| slot.as_mut().map(|w| (w.node_id, &mut w.widget)))
    }
}

impl<W: Widget> AnyWidgetStore for WidgetStore<W> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn free(&mut self, index: u64) {
        // A reserved-but-never-filled slot (a build that failed part-way)
        // is already `None`; freeing it is the same operation.
        self.slots[index as usize] = None;
        self.free.push(index);
    }
}
