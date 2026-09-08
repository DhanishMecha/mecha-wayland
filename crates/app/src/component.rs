//! Per-node data: every node carries one value of every registered
//! [`Component`] type, reached through proxies rather than borrows of the
//! [`App`](crate::App).
//!
//! Each type is one *column*, a `Vec` indexed by node slot that mirrors the
//! node arena. A column is lent out whole, with the same rule as resources:
//! any number of [`Comps`] readers, or exactly one [`CompsMut`] writer. A
//! lookup that can't be satisfied returns `None`, never panics, and the
//! caller drops what it holds and asks again. Only an *unregistered* type
//! panics — that is a setup bug, not a state.
//!
//! [`Comp`] and [`CompMut`] are the same two proxies narrowed to one node:
//! what a handler or builder gets for its own node. They hold the column
//! too, so they count as a reader or the writer of it.
//!
//! The tree may change while a column is lent out. Spawns and removes that
//! happen then are recorded as *pending* and applied the next time the
//! column is held by nobody: on the writer's drop, or on the next lookup
//! once every reader is gone. Until then a holder cannot see the new node
//! (its id resolves to `None`), which is the same "lent out is absent" rule
//! everything else follows.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry as MapEntry;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::rc::Rc;

use crate::NodeId;

/// Marks a type as per-node data. Every node carries one `C`, created by
/// `Default` when the node is spawned (or when `C` is registered, for nodes
/// that already exist) and reset to `Default` when the node is removed.
///
/// A marker, like [`Widget`](crate::Widget): the runtime never calls into a
/// component beyond `Default`.
///
/// ```
/// # use app::Component;
/// #[derive(Default)]
/// struct Layout { x: f32, y: f32 }
/// impl Component for Layout {}
/// ```
pub trait Component: Default + 'static {}

/// One node slot's `C`. `node_id` is `Some` exactly while that slot is
/// occupied; it is what lets a proxy validate a lookup without reaching
/// into the node arena.
struct Entry<C> {
    node_id: Option<NodeId>,
    value: C,
}

/// Every node's `C`, indexed by node slot. Grows to match the arena.
pub(crate) struct Column<C> {
    entries: Vec<Entry<C>>,
}

impl<C: Component> Column<C> {
    fn with_len(len: u64) -> Self {
        let mut entries = Vec::new();
        entries.resize_with(len as usize, Entry::vacant);
        Self { entries }
    }

    /// Mark slot `index` as belonging to `node_id` (or to nobody) with a
    /// fresh default, growing the column if the arena has grown.
    fn set(&mut self, index: u64, node_id: Option<NodeId>) {
        let index = index as usize;
        if index >= self.entries.len() {
            self.entries.resize_with(index + 1, Entry::vacant);
        }
        self.entries[index] = Entry {
            node_id,
            value: C::default(),
        };
    }

    fn entry(&self, id: NodeId) -> Option<&Entry<C>> {
        let entry = self.entries.get(id.component_index() as usize)?;
        (entry.node_id == Some(id)).then_some(entry)
    }

    fn entry_mut(&mut self, id: NodeId) -> Option<&mut Entry<C>> {
        let entry = self.entries.get_mut(id.component_index() as usize)?;
        (entry.node_id == Some(id)).then_some(entry)
    }

    fn get(&self, id: NodeId) -> Option<&C> {
        self.entry(id).map(|e| &e.value)
    }

    fn get_mut(&mut self, id: NodeId) -> Option<&mut C> {
        self.entry_mut(id).map(|e| &mut e.value)
    }

    fn iter(&self) -> impl Iterator<Item = (NodeId, &C)> {
        self.entries
            .iter()
            .filter_map(|e| Some((e.node_id?, &e.value)))
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = (NodeId, &mut C)> {
        self.entries
            .iter_mut()
            .filter_map(|e| Some((e.node_id?, &mut e.value)))
    }
}

impl<C: Component> Entry<C> {
    fn vacant() -> Self {
        Self {
            node_id: None,
            value: C::default(),
        }
    }
}

/// Apply one slot change to an erased column. Stored per column at
/// registration so the map can catch up on pending changes without knowing
/// the type.
fn apply<C: Component>(column: &mut dyn Any, index: u64, node_id: Option<NodeId>) {
    column
        .downcast_mut::<Column<C>>()
        .expect("column slot keyed by its type")
        .set(index, node_id);
}

/// One registered type. `column` is `None` while a writer holds it; the key
/// stays so a second registration can't slip in underneath.
struct ColumnSlot {
    column: Option<Rc<dyn Any>>,
    /// Slot changes that happened while the column was lent out or shared,
    /// oldest first. Drained whenever the column is held by nobody.
    pending: Vec<(u64, Option<NodeId>)>,
    apply: fn(&mut dyn Any, u64, Option<NodeId>),
}

impl ColumnSlot {
    /// Apply every pending change if nothing else holds the column.
    fn catch_up(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let Some(rc) = self.column.as_mut() else {
            return;
        };
        let Some(column) = Rc::get_mut(rc) else {
            return;
        };
        for (index, node_id) in self.pending.drain(..) {
            (self.apply)(column, index, node_id);
        }
    }
}

type Map = Rc<RefCell<HashMap<TypeId, ColumnSlot>>>;

/// The map behind every component proxy. Shared by `Rc` so a proxy can
/// outlive any borrow of the app and still find its way back.
#[derive(Default)]
pub(crate) struct Components {
    map: Map,
}

fn slot<C: Component>(map: &mut HashMap<TypeId, ColumnSlot>) -> &mut ColumnSlot {
    map.get_mut(&TypeId::of::<C>()).unwrap_or_else(|| {
        panic!(
            "component `{}` is not registered; call `App::register_component` first",
            std::any::type_name::<C>()
        )
    })
}

impl Components {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create the column for `C`, sized to an arena of `len` slots, with
    /// every id in `live` marked occupied.
    ///
    /// # Panics
    ///
    /// If `C` is already registered.
    pub fn register<C: Component>(&mut self, len: u64, live: impl IntoIterator<Item = NodeId>) {
        let mut map = self.map.borrow_mut();
        let MapEntry::Vacant(vacant) = map.entry(TypeId::of::<C>()) else {
            panic!(
                "component `{}` is already registered",
                std::any::type_name::<C>()
            );
        };
        let mut column = Column::<C>::with_len(len);
        for id in live {
            column.set(id.component_index(), Some(id));
        }
        vacant.insert(ColumnSlot {
            column: Some(Rc::new(column)),
            pending: Vec::new(),
            apply: apply::<C>,
        });
    }

    /// Record that node slot `index` now belongs to `node_id` (or to nobody)
    /// in every column: applied at once where the column is free, queued
    /// where it is lent out or shared.
    pub fn set(&mut self, index: u64, node_id: Option<NodeId>) {
        for slot in self.map.borrow_mut().values_mut() {
            slot.pending.push((index, node_id));
            slot.catch_up();
        }
    }

    /// The whole `C` column, shared. `None` if a [`CompsMut`] holds it.
    pub fn get<C: Component>(&self) -> Option<Comps<C>> {
        let mut map = self.map.borrow_mut();
        let slot = slot::<C>(&mut map);
        slot.catch_up();
        let column = Rc::clone(slot.column.as_ref()?)
            .downcast::<Column<C>>()
            .expect("column slot keyed by its type");
        Some(Comps { column })
    }

    /// The whole `C` column, exclusive. `None` if a [`CompsMut`] holds it or
    /// any [`Comps`] / [`Comp`] is alive.
    pub fn get_mut<C: Component>(&mut self) -> Option<CompsMut<C>> {
        let mut map = self.map.borrow_mut();
        let slot = slot::<C>(&mut map);
        // The map's own clone is the one strong reference a free column
        // has; anything more is a live reader.
        if Rc::strong_count(slot.column.as_ref()?) != 1 {
            return None;
        }
        slot.catch_up();
        let column = slot
            .column
            .take()
            .expect("checked above")
            .downcast::<Column<C>>()
            .expect("column slot keyed by its type");
        Some(CompsMut {
            map: Rc::clone(&self.map),
            column: Some(column),
        })
    }

    /// `id`'s `C`, shared. `None` if the column is held by a writer, or if
    /// `id` isn't visible in it (stale, or spawned while it was shared).
    pub fn get_at<C: Component>(&self, id: NodeId) -> Option<Comp<C>> {
        let all = self.get::<C>()?;
        all.get(id)?;
        Some(Comp { all, id })
    }

    /// `id`'s `C`, exclusive. `None` under the same conditions as
    /// [`Components::get_mut`], or if `id` isn't visible in the column.
    pub fn get_at_mut<C: Component>(&mut self, id: NodeId) -> Option<CompMut<C>> {
        let all = self.get_mut::<C>()?;
        all.get(id)?;
        Some(CompMut { all, id })
    }
}

// ── column proxies ───────────────────────────────────────────────────────────

/// A shared read of every node's `C`. Any number may be alive at once;
/// while any is, [`components_mut`](crate::App::components_mut) returns
/// `None`, and nodes spawned or removed meanwhile are not reflected until
/// every reader is gone.
pub struct Comps<C: Component> {
    column: Rc<Column<C>>,
}

impl<C: Component> Comps<C> {
    /// `None` if `id` is stale, or was spawned while the column was shared.
    pub fn get(&self, id: impl Into<NodeId>) -> Option<&C> {
        self.column.get(id.into())
    }

    /// Every node visible in the column, in slot order, root included.
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &C)> {
        self.column.iter()
    }
}

impl<C: Component, I: Into<NodeId>> Index<I> for Comps<C> {
    type Output = C;

    /// # Panics
    ///
    /// If `id` is not visible in the column; see [`Comps::get`].
    fn index(&self, id: I) -> &C {
        self.get(id).expect("indexed with a stale node id")
    }
}

/// An exclusive write to every node's `C`. While one is alive the column is
/// out of the map, so every other lookup of `C` returns `None`. Drop puts
/// it back, unwinding included, after applying any spawns and removes that
/// happened in the meantime.
pub struct CompsMut<C: Component> {
    map: Map,
    /// `Some` until `Drop` takes it.
    column: Option<Rc<Column<C>>>,
}

impl<C: Component> CompsMut<C> {
    #[inline]
    fn column(&self) -> &Column<C> {
        self.column.as_ref().expect("column is present until drop")
    }

    #[inline]
    fn column_mut(&mut self) -> &mut Column<C> {
        // Unique by construction: `get_mut` only takes an `Rc` with a
        // strong count of 1, and nothing can clone it while it's here.
        Rc::get_mut(self.column.as_mut().expect("column is present until drop"))
            .expect("CompsMut holds the only reference")
    }

    /// `None` if `id` is stale, or was spawned while the column was out.
    pub fn get(&self, id: impl Into<NodeId>) -> Option<&C> {
        self.column().get(id.into())
    }

    /// `None` if `id` is stale, or was spawned while the column was out.
    pub fn get_mut(&mut self, id: impl Into<NodeId>) -> Option<&mut C> {
        self.column_mut().get_mut(id.into())
    }

    /// Every node visible in the column, in slot order, root included.
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &C)> {
        self.column().iter()
    }

    /// Every node visible in the column, in slot order, root included.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (NodeId, &mut C)> {
        self.column_mut().iter_mut()
    }
}

impl<C: Component, I: Into<NodeId>> Index<I> for CompsMut<C> {
    type Output = C;

    /// # Panics
    ///
    /// If `id` is not visible in the column; see [`CompsMut::get`].
    fn index(&self, id: I) -> &C {
        self.get(id).expect("indexed with a stale node id")
    }
}

impl<C: Component, I: Into<NodeId>> IndexMut<I> for CompsMut<C> {
    /// # Panics
    ///
    /// If `id` is not visible in the column; see [`CompsMut::get_mut`].
    fn index_mut(&mut self, id: I) -> &mut C {
        self.get_mut(id).expect("indexed with a stale node id")
    }
}

impl<C: Component> Drop for CompsMut<C> {
    fn drop(&mut self) {
        let column = self.column.take().expect("dropped once");
        let mut map = self.map.borrow_mut();
        if let Some(slot) = map.get_mut(&TypeId::of::<C>()) {
            debug_assert!(
                slot.column.is_none(),
                "tombstone overwritten while lent out"
            );
            slot.column = Some(column);
            slot.catch_up();
        }
    }
}

// ── single-node proxies ──────────────────────────────────────────────────────

/// A shared read of one node's `C`: what [`Context::component`](crate::Context::component)
/// and [`Spawner::component`](crate::Spawner::component) hand out. Holds the
/// whole column, so it counts as a [`Comps`] reader for lending purposes.
pub struct Comp<C: Component> {
    all: Comps<C>,
    id: NodeId,
}

impl<C: Component> Comp<C> {
    /// The node this is the component of.
    #[inline]
    pub fn id(&self) -> NodeId {
        self.id
    }
}

impl<C: Component> Deref for Comp<C> {
    type Target = C;

    #[inline]
    fn deref(&self) -> &C {
        // Checked on construction; a shared column cannot change underneath.
        self.all
            .get(self.id)
            .expect("entry checked on construction")
    }
}

/// An exclusive write to one node's `C`: what
/// [`Context::component_mut`](crate::Context::component_mut) and
/// [`Spawner::component_mut`](crate::Spawner::component_mut) hand out. Holds
/// the whole column, so it counts as the [`CompsMut`] writer for lending
/// purposes.
pub struct CompMut<C: Component> {
    all: CompsMut<C>,
    id: NodeId,
}

impl<C: Component> CompMut<C> {
    /// The node this is the component of.
    #[inline]
    pub fn id(&self) -> NodeId {
        self.id
    }
}

impl<C: Component> Deref for CompMut<C> {
    type Target = C;

    #[inline]
    fn deref(&self) -> &C {
        // Checked on construction; only this proxy can change the column.
        self.all
            .get(self.id)
            .expect("entry checked on construction")
    }
}

impl<C: Component> DerefMut for CompMut<C> {
    #[inline]
    fn deref_mut(&mut self) -> &mut C {
        self.all
            .get_mut(self.id)
            .expect("entry checked on construction")
    }
}
