use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::marker::PhantomData;

use crate::component::Components;
use crate::node::{Handler, Node, Nodes};
use crate::resource::Resources;
use crate::widget_store::{AnyWidgetStore, WidgetStore, WidgetWrapper};
use crate::{
    Comp, CompMut, Component, Comps, CompsMut, Context, Event, Handle, NodeId, Res, ResMut,
    Resource, Signal, Tick, Widget, WidgetBuild,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The id names a node that has been removed (or never existed).
    Stale,
    /// The root cannot be removed.
    Root,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::Stale => "node id is stale",
            Error::Root => "the root node cannot be removed",
        })
    }
}

impl std::error::Error for Error {}

/// A system: a plain `fn` that runs for every [`Signal`] of type `S`.
pub type System<S> = fn(&mut App, &S);

/// A queued unit of work: a signal or event with its dispatch baked in, so
/// the queue needs no knowledge of the concrete type.
type Job = Box<dyn FnOnce(&mut App)>;

/// The runtime: a tree of nodes, each backed by a widget stored in a
/// per-type column, plus the systems and queues that drive it.
///
/// The tree is never empty — [`App::new`] creates the root, the one node that
/// has no widget. Every other node is created by [`App::spawn`] under an
/// existing parent and destroyed, with its whole subtree, by [`App::remove`].
///
/// Messages are queued, never run inline: [`App::signal`] and [`App::emit`]
/// push, [`App::flush`] drains. See the crate docs for the model.
pub struct App {
    nodes: Nodes,
    widget_stores: Vec<Box<dyn AnyWidgetStore>>,
    columns: HashMap<TypeId, u64>,
    root: NodeId,
    /// Per signal type, a `Vec<System<S>>` behind `Any`.
    systems: HashMap<TypeId, Box<dyn Any>>,
    resources: Resources,
    components: Components,
    events: VecDeque<Job>,
    signals: VecDeque<Job>,
    runner: fn(App),
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Loops `signal(Tick)` then `flush()` forever. See [`App::set_runner`].
fn default_runner(mut app: App) {
    loop {
        app.signal(Tick);
        app.flush();
    }
}

impl App {
    pub fn new() -> Self {
        let mut nodes = Nodes::new();
        let (index, generation) = nodes.reserve();
        let root = NodeId::new(NodeId::NONE, NodeId::NONE, index, generation);
        nodes.fill(
            index,
            Node {
                id: root,
                type_id: None,
                parent: root,
                children: Vec::new(),
                handlers: Vec::new(),
            },
        );
        Self {
            nodes,
            widget_stores: Vec::new(),
            columns: HashMap::new(),
            root,
            systems: HashMap::new(),
            resources: Resources::new(),
            components: Components::new(),
            events: VecDeque::new(),
            signals: VecDeque::new(),
            runner: default_runner,
        }
    }

    /// Always valid; the only node whose parent is itself.
    pub fn root(&self) -> NodeId {
        self.root
    }

    // ── tree: write ──────────────────────────────────────────────────────────

    /// Build `builder`'s widget (and any subtree it describes) as the last
    /// child of `parent`.
    ///
    /// The node and widget slots are reserved *before* the builder runs, so
    /// the handle it receives is the handle returned here. If a nested
    /// [`Spawner::child`] or [`Spawner::on`] fails, everything built so far
    /// is torn down and the error is returned; the tree is left as it was.
    pub fn spawn<B: WidgetBuild>(
        &mut self,
        parent: impl Into<NodeId>,
        builder: B,
    ) -> Result<Handle<B::Widget>, Error> {
        let parent = parent.into();
        self.nodes.get(parent).ok_or(Error::Stale)?;

        let type_id = TypeId::of::<B::Widget>();
        let column = self.column::<B::Widget>();
        let widget_index = self.widget_store_mut::<B::Widget>(column).reserve();
        let (component_index, generation) = self.nodes.reserve();
        let id = NodeId::new(column, widget_index, component_index, generation);

        self.nodes.fill(
            component_index,
            Node {
                id,
                type_id: Some(type_id),
                parent,
                children: Vec::new(),
                handlers: Vec::new(),
            },
        );
        self.nodes
            .get_mut(parent)
            .expect("parent validated above")
            .children
            .push(id);
        // Before the builder runs, so it can reach its own components.
        self.components.set(component_index, Some(id));

        let handle = Handle::new(id);
        let mut spawner = Spawner {
            app: self,
            me: id,
            failed: None,
            _widget: PhantomData,
        };
        let widget = builder.spawn(handle, &mut spawner);
        if let Some(e) = spawner.failed {
            self.remove(id).expect("partially built node is live");
            return Err(e);
        }

        self.widget_store_mut::<B::Widget>(column).fill(
            widget_index,
            WidgetWrapper {
                node_id: id,
                widget: Some(widget),
            },
        );
        Ok(handle)
    }

    /// Remove `id` and every node under it. Every id in the subtree is stale
    /// afterwards; the freed slots are reused by later spawns. Handlers go
    /// with their nodes; events already queued for them are dropped when the
    /// queue drains.
    pub fn remove(&mut self, id: impl Into<NodeId>) -> Result<(), Error> {
        let id = id.into();
        if id == self.root {
            return Err(Error::Root);
        }
        let parent = self.nodes.get(id).ok_or(Error::Stale)?.parent;

        let siblings = &mut self
            .nodes
            .get_mut(parent)
            .expect("live node has a live parent")
            .children;
        let position = siblings
            .iter()
            .position(|&c| c == id)
            .expect("live node is in its parent's children");
        siblings.remove(position);

        // Iterative post-order: pop a node, free it, push its children — so a
        // child is always freed after its parent has been unlinked from the
        // tree, but every descendant is reached before we return.
        let mut pending = vec![id];
        while let Some(current) = pending.pop() {
            // SAFETY: `current` is either the validated `id` or a child taken
            // from a node we just freed, so it is live. Its widget slot is
            // freed right below, and its children go onto `pending` to be
            // freed by this same loop. It has been unlinked: `id` explicitly
            // above, descendants by virtue of their parent being freed.
            let node = unsafe { self.nodes.free(current.component_index()) };
            if current.has_widget() {
                self.widget_stores[current.widget_column() as usize].free(current.widget_index());
            }
            self.components.set(current.component_index(), None);
            pending.extend(node.children);
        }
        Ok(())
    }

    // ── tree: read ───────────────────────────────────────────────────────────

    /// `None` if `id` is stale. The root's parent is the root.
    pub fn parent(&self, id: impl Into<NodeId>) -> Option<NodeId> {
        self.nodes.get(id.into()).map(|n| n.parent)
    }

    /// In sibling order. `None` if `id` is stale.
    pub fn children(&self, id: impl Into<NodeId>) -> Option<&[NodeId]> {
        self.nodes.get(id.into()).map(|n| n.children.as_slice())
    }

    /// `None` if `id` is stale, is the root, holds a widget of another type,
    /// or is the node whose handler is currently running.
    pub fn widget<W: Widget>(&self, id: impl Into<NodeId>) -> Option<&W> {
        let id = id.into();
        self.check_type::<W>(id)?;
        self.widget_store::<W>(id.widget_column()).get(id)
    }

    /// `None` if `id` is stale, is the root, holds a widget of another type,
    /// or is the node whose handler is currently running.
    pub fn widget_mut<W: Widget>(&mut self, id: impl Into<NodeId>) -> Option<&mut W> {
        let id = id.into();
        self.check_type::<W>(id)?;
        self.widget_store_mut::<W>(id.widget_column()).get_mut(id)
    }

    /// Every live widget of type `W`, with its node. Order is unspecified and
    /// not stable across spawns and removes.
    pub fn widgets<W: Widget>(&mut self) -> impl Iterator<Item = (NodeId, &mut W)> {
        let store = match self.columns.get(&TypeId::of::<W>()) {
            Some(&column) => self.widget_stores[column as usize]
                .as_any_mut()
                .downcast_mut::<WidgetStore<W>>(),
            None => None,
        };
        store.into_iter().flat_map(WidgetStore::iter_mut)
    }

    // ── resources ────────────────────────────────────────────────────────────

    /// Store `resource` as the app's one `R`.
    ///
    /// # Panics
    ///
    /// If an `R` is already present — including one currently lent out to a
    /// [`ResMut`]. A second value of a type is a bug, not a state to recover
    /// from; [`App::remove_resource`] first if replacing is the intent.
    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.resources.insert(resource);
        self
    }

    /// A shared read of `R`. Any number may be alive at once. `None` if no
    /// `R` was inserted or a [`ResMut<R>`] is currently out.
    pub fn resource<R: Resource>(&self) -> Option<Res<R>> {
        self.resources.get()
    }

    /// An exclusive write to `R`. `None` if no `R` was inserted, a
    /// [`ResMut<R>`] is already out, or any [`Res<R>`] is alive. The value
    /// leaves the map for the proxy's lifetime and returns when it drops.
    pub fn resource_mut<R: Resource>(&mut self) -> Option<ResMut<R>> {
        self.resources.get_mut()
    }

    /// Take `R` out for good. `None`, with nothing changed, if no `R` was
    /// inserted or it is lent out to any proxy right now.
    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        self.resources.remove()
    }

    // ── components ───────────────────────────────────────────────────────────

    /// Give every node a `C`, now and for every node spawned later. Nodes
    /// that already exist, the root included, get `C::default()`.
    ///
    /// Only the app registers: a column is app-wide state, like a system.
    /// Every component accessor panics for a type that was never
    /// registered, so do this at setup, before the first signal.
    ///
    /// # Panics
    ///
    /// If `C` is already registered.
    pub fn register_component<C: Component>(&mut self) -> &mut Self {
        self.components
            .register::<C>(self.nodes.len(), self.nodes.live_ids());
        self
    }

    /// A shared read of every node's `C`. Any number may be alive at once.
    /// `None` if a [`CompsMut<C>`] is currently out.
    ///
    /// # Panics
    ///
    /// If `C` was never registered.
    pub fn components<C: Component>(&self) -> Option<Comps<C>> {
        self.components.get()
    }

    /// An exclusive write to every node's `C`. `None` if a [`CompsMut<C>`]
    /// is already out, or any [`Comps<C>`] / [`Comp<C>`] is alive. The
    /// column leaves the map for the proxy's lifetime and returns when it
    /// drops; spawns and removes in between are applied on the way back.
    ///
    /// # Panics
    ///
    /// If `C` was never registered.
    pub fn components_mut<C: Component>(&mut self) -> Option<CompsMut<C>> {
        self.components.get_mut()
    }

    /// One node's `C`, shared. See [`Context::component`].
    pub(crate) fn component_at<C: Component>(&self, id: NodeId) -> Option<Comp<C>> {
        self.components.get_at(id)
    }

    /// One node's `C`, exclusive. See [`Context::component_mut`].
    pub(crate) fn component_at_mut<C: Component>(&mut self, id: NodeId) -> Option<CompMut<C>> {
        self.components.get_at_mut(id)
    }

    // ── systems ──────────────────────────────────────────────────────────────

    /// Register `system` to run for every signal of type `S`, after any
    /// system registered for `S` before it. There is no removal, and no
    /// dedupe: registering the same fn twice runs it twice.
    pub fn system<S: Signal>(&mut self, system: System<S>) -> &mut Self {
        self.systems
            .entry(TypeId::of::<S>())
            .or_insert_with(|| Box::new(Vec::<System<S>>::new()))
            .downcast_mut::<Vec<System<S>>>()
            .expect("systems column keyed by its signal type")
            .push(system);
        self
    }

    /// Queue `signal` for its systems. Nothing runs until [`App::flush`].
    pub fn signal<S: Signal>(&mut self, signal: S) {
        self.signals
            .push_back(Box::new(move |app| app.run_systems(&signal)));
    }

    fn run_systems<S: Signal>(&mut self, signal: &S) {
        // Looked up afresh each step: a system registered from inside a
        // system for the same `S` runs in this same pass, in order.
        let mut i = 0;
        loop {
            let system = self
                .systems
                .get(&TypeId::of::<S>())
                .and_then(|column| column.downcast_ref::<Vec<System<S>>>())
                .and_then(|systems| systems.get(i).copied());
            match system {
                Some(system) => system(self, signal),
                None => return,
            }
            i += 1;
        }
    }

    // ── events ───────────────────────────────────────────────────────────────

    /// Queue `event` for each of `targets`, in order. Nothing runs until
    /// [`App::flush`]. A target that is stale by then, is the root, or has
    /// no handler for `E` is skipped.
    pub fn emit<E: Event>(&mut self, event: E, targets: &[NodeId]) {
        let targets = targets.to_vec();
        self.events.push_back(Box::new(move |app| {
            for id in targets {
                app.run_handlers(id, &event);
            }
        }));
    }

    /// Queue `event` for every node live right now, in slot order. Only nodes
    /// with a handler for `E` do anything.
    pub fn emit_all<E: Event>(&mut self, event: E) {
        let targets = self.nodes.live_ids();
        self.events.push_back(Box::new(move |app| {
            for id in targets {
                app.run_handlers(id, &event);
            }
        }));
    }

    fn run_handlers<E: Event>(&mut self, id: NodeId, event: &E) {
        // Nobody can add or remove handlers on a node while its own handlers
        // run (only a `Spawner` registers, and handlers can't spawn), so the
        // list can be moved out and moved back without anyone noticing. The
        // guard moves it back on every exit, a panicking handler included,
        // so an unwind caught higher up doesn't strip the node of behaviour.
        struct Restore<'a> {
            app: &'a mut App,
            id: NodeId,
            handlers: Vec<Handler>,
        }
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                if let Some(node) = self.app.nodes.get_mut(self.id) {
                    node.handlers = std::mem::take(&mut self.handlers);
                }
            }
        }

        let Some(node) = self.nodes.get_mut(id) else {
            return;
        };
        let handlers = std::mem::take(&mut node.handlers);
        let mut guard = Restore {
            app: self,
            id,
            handlers,
        };
        // Disjoint field borrows: the list is read while the app is lent out.
        let Restore { app, handlers, .. } = &mut guard;
        for handler in handlers.iter().filter(|h| h.event == TypeId::of::<E>()) {
            (handler.run)(app, id, event);
        }
    }

    /// Move `id`'s widget out for a handler. See [`WidgetWrapper`].
    pub(crate) fn take_widget<W: Widget>(&mut self, id: NodeId) -> Option<W> {
        self.check_type::<W>(id)?;
        self.widget_store_mut::<W>(id.widget_column()).take(id)
    }

    /// Put a taken widget back; dropped if the node is gone.
    pub(crate) fn put_widget_back<W: Widget>(&mut self, id: NodeId, widget: W) {
        if self.check_type::<W>(id).is_some() {
            self.widget_store_mut::<W>(id.widget_column())
                .put_back(id, widget);
        }
    }

    // ── flush and run ────────────────────────────────────────────────────────

    /// Drain both queues until they are empty. Events take priority: a
    /// signal is only run when no event is pending, and anything a handler
    /// or system queues is picked up in the same call.
    pub fn flush(&mut self) {
        loop {
            if let Some(job) = self.events.pop_front() {
                job(self);
            } else if let Some(job) = self.signals.pop_front() {
                job(self);
            } else {
                return;
            }
        }
    }

    /// Replace the runner that [`App::run`] hands the app to. The default
    /// loops `signal(Tick)` then `flush()` forever.
    pub fn set_runner(&mut self, runner: fn(App)) -> &mut Self {
        self.runner = runner;
        self
    }

    /// Hand the app to its runner. Returns when the runner does.
    pub fn run(self) {
        (self.runner)(self)
    }

    // ── internals ────────────────────────────────────────────────────────────

    fn check_type<W: Widget>(&self, id: NodeId) -> Option<()> {
        let node = self.nodes.get(id)?;
        (node.type_id == Some(TypeId::of::<W>())).then_some(())
    }

    /// The column for `W`, allocated on first sight.
    fn column<W: Widget>(&mut self) -> u64 {
        if let Some(&column) = self.columns.get(&TypeId::of::<W>()) {
            return column;
        }
        let column = self.widget_stores.len() as u64;
        self.widget_stores.push(Box::new(WidgetStore::<W>::new()));
        self.columns.insert(TypeId::of::<W>(), column);
        column
    }

    fn widget_store<W: Widget>(&self, column: u64) -> &WidgetStore<W> {
        self.widget_stores[column as usize]
            .as_any()
            .downcast_ref()
            .expect("column type checked by caller")
    }

    fn widget_store_mut<W: Widget>(&mut self, column: u64) -> &mut WidgetStore<W> {
        self.widget_stores[column as usize]
            .as_any_mut()
            .downcast_mut()
            .expect("column type checked by caller")
    }
}

/// What a [`WidgetBuild`] gets to touch while it runs: attach children,
/// register handlers, read or write resources, and read or write its own
/// node's components — nothing else.
///
/// `W` is the widget being built. It is not used yet; it is there so later
/// conveniences can be typed on the builder's own widget.
pub struct Spawner<'a, W: Widget> {
    app: &'a mut App,
    /// The node being built.
    me: NodeId,
    failed: Option<Error>,
    _widget: PhantomData<fn() -> W>,
}

impl<W: Widget> Spawner<'_, W> {
    /// Build `builder` as the last child of `parent`, which should be the
    /// `me` the builder was given or a handle returned by an earlier `child`.
    ///
    /// A builder has no way to propagate an error, so if `parent` is stale
    /// this returns a handle that matches nothing and remembers the error;
    /// further `child` and `on` calls are no-ops and the enclosing
    /// [`App::spawn`] returns `Err` after tearing down whatever was built.
    pub fn child<B: WidgetBuild>(
        &mut self,
        parent: impl Into<NodeId>,
        builder: B,
    ) -> Handle<B::Widget> {
        if self.failed.is_some() {
            return Handle::INVALID;
        }
        match self.app.spawn(parent, builder) {
            Ok(handle) => handle,
            Err(e) => {
                self.failed = Some(e);
                Handle::INVALID
            }
        }
    }

    /// Run `handler` whenever an `E` is emitted at `target`, after any
    /// handler for `E` registered on it earlier. `target` is `me` or a handle
    /// an earlier `child` returned.
    ///
    /// `handler` is a plain `fn`: it cannot capture. Whatever it needs lives
    /// in the widget, or arrives in the event.
    pub fn on<V: Widget, E: Event>(&mut self, target: Handle<V>, handler: fn(&mut Context<V>, &E)) {
        if self.failed.is_some() {
            return;
        }
        let id = target.id();
        let Some(node) = self.app.nodes.get_mut(id) else {
            self.failed = Some(Error::Stale);
            return;
        };
        node.handlers.push(Handler {
            event: TypeId::of::<E>(),
            run: Box::new(move |app: &mut App, id: NodeId, event: &dyn Any| {
                let event = event
                    .downcast_ref::<E>()
                    .expect("handler selected by event TypeId");
                // The context holds the widget and returns it on drop,
                // unwinding included.
                let Some(mut ctx) = Context::take(app, Handle::<V>::new(id)) else {
                    return;
                };
                handler(&mut ctx, event);
            }),
        });
    }

    /// A shared read of a resource. See [`App::resource`].
    pub fn resource<R: Resource>(&self) -> Option<Res<R>> {
        self.app.resource()
    }

    /// An exclusive write to a resource. See [`App::resource_mut`]. A builder
    /// writing a resource is a side effect beyond its own subtree; keep it to
    /// registration-style bookkeeping.
    pub fn resource_mut<R: Resource>(&mut self) -> Option<ResMut<R>> {
        self.app.resource_mut()
    }

    /// A shared read of this node's `C`. See [`Context::component`].
    ///
    /// # Panics
    ///
    /// If `C` was never registered.
    pub fn component<C: Component>(&self) -> Option<Comp<C>> {
        self.app.component_at(self.me)
    }

    /// An exclusive write to this node's `C`. See [`Context::component_mut`].
    ///
    /// # Panics
    ///
    /// If `C` was never registered.
    pub fn component_mut<C: Component>(&mut self) -> Option<CompMut<C>> {
        self.app.component_at_mut(self.me)
    }

    /// Replace this node's `C`, returning the old value. The usual way for a
    /// builder to configure its own node. `None`, and nothing changes, under
    /// the same conditions as [`Spawner::component_mut`].
    ///
    /// # Panics
    ///
    /// If `C` was never registered.
    pub fn set_component<C: Component>(&mut self, component: C) -> Option<C> {
        let mut current = self.component_mut::<C>()?;
        Some(std::mem::replace(&mut *current, component))
    }
}
