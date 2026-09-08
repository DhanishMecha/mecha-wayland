use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// Identifies one node in an [`App`](crate::App) tree.
///
/// | field             | addresses                                   |
/// |-------------------|---------------------------------------------|
/// | `widget_column`   | which widget store (one per widget type)    |
/// | `widget_index`    | slot inside that store                      |
/// | `component_index` | slot in the node arena                      |
/// | `generation`      | reuse count of the node slot                |
///
/// The root node has no widget; both widget fields are `u64::MAX` for it.
///
/// An id stays valid until its node is removed. Every accessor on `App` that
/// takes a `NodeId` returns `None`/`Err` for an id whose node is gone, even if
/// the slot has since been reused — the `generation` field is what catches that.
///
/// A [`Handle`] is a `NodeId` that also remembers the widget type; every
/// method that takes a `NodeId` accepts a `Handle` too.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId {
    widget_column: u64,
    widget_index: u64,
    component_index: u64,
    generation: u64,
}

impl NodeId {
    /// Sentinel for "no slot" in any of the three index fields.
    pub(crate) const NONE: u64 = u64::MAX;

    /// An id that matches nothing. Handed out by [`Spawner::child`](crate::Spawner::child)
    /// once a build has already failed.
    pub(crate) const INVALID: NodeId = NodeId::new(Self::NONE, Self::NONE, Self::NONE, Self::NONE);

    pub(crate) const fn new(
        widget_column: u64,
        widget_index: u64,
        component_index: u64,
        generation: u64,
    ) -> Self {
        Self {
            widget_column,
            widget_index,
            component_index,
            generation,
        }
    }

    #[inline]
    pub const fn widget_column(self) -> u64 {
        self.widget_column
    }

    #[inline]
    pub const fn widget_index(self) -> u64 {
        self.widget_index
    }

    #[inline]
    pub const fn component_index(self) -> u64 {
        self.component_index
    }

    #[inline]
    pub const fn generation(self) -> u64 {
        self.generation
    }

    /// `false` only for the root node.
    #[inline]
    pub const fn has_widget(self) -> bool {
        self.widget_column != Self::NONE
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::INVALID {
            return f.write_str("NodeId(INVALID)");
        }
        let mut s = f.debug_struct("NodeId");
        if self.has_widget() {
            s.field("widget_column", &self.widget_column)
                .field("widget_index", &self.widget_index);
        }
        s.field("component_index", &self.component_index)
            .field("generation", &self.generation)
            .finish()
    }
}

/// A [`NodeId`] that remembers which widget type lives at the node.
///
/// Returned by [`App::spawn`](crate::App::spawn) and
/// [`Spawner::child`](crate::Spawner::child), and required by
/// [`Spawner::on`](crate::Spawner::on) and [`Context::at`](crate::Context::at)
/// so the widget type of a handler or lookup is checked at compile time.
///
/// `Copy` regardless of `W`. Converts into a `NodeId` with [`Handle::id`] or
/// `.into()`, so anything that takes a `NodeId` takes a handle.
pub struct Handle<W> {
    id: NodeId,
    _widget: PhantomData<fn() -> W>,
}

impl<W> Handle<W> {
    pub(crate) const INVALID: Handle<W> = Handle::new(NodeId::INVALID);

    pub(crate) const fn new(id: NodeId) -> Self {
        Self {
            id,
            _widget: PhantomData,
        }
    }

    #[inline]
    pub const fn id(self) -> NodeId {
        self.id
    }
}

impl<W> From<Handle<W>> for NodeId {
    fn from(h: Handle<W>) -> NodeId {
        h.id
    }
}

// Manual impls: derives would add `W: Copy` etc. bounds, but the phantom
// carries no `W` value.
impl<W> Clone for Handle<W> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<W> Copy for Handle<W> {}
impl<W> PartialEq for Handle<W> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<W> Eq for Handle<W> {}
impl<W> Hash for Handle<W> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}
impl<W> PartialOrd for Handle<W> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<W> Ord for Handle<W> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}
impl<W> fmt::Debug for Handle<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle<{}>({:?})", std::any::type_name::<W>(), self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_shaped_id_has_no_widget() {
        let root = NodeId::new(NodeId::NONE, NodeId::NONE, 0, 0);
        assert!(!root.has_widget());
        assert!(NodeId::new(0, 0, 1, 0).has_widget());
        assert_ne!(root, NodeId::INVALID);
    }

    #[test]
    fn handle_is_copy_and_converts() {
        struct NotCopy(#[allow(dead_code)] String);
        let h: Handle<NotCopy> = Handle::new(NodeId::new(0, 0, 1, 0));
        let h2 = h;
        assert_eq!(h, h2);
        let id: NodeId = h.into();
        assert_eq!(id, h.id());
    }
}
