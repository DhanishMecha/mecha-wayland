use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;

use crate::node::{Node, Nodes};
use crate::widget_store::{AnyWidgetStore, WidgetStore, WidgetWrapper};
use crate::{NodeId, Widget, WidgetBuild};

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

/// The runtime: a tree of nodes, each backed by a widget stored in a
/// per-type column.
///
/// The tree is never empty — [`App::new`] creates the root, the one node that
/// has no widget. Every other node is created by [`App::spawn`] under an
/// existing parent and destroyed, with its whole subtree, by [`App::remove`].
pub struct App {
    nodes: Nodes,
    widget_stores: Vec<Box<dyn AnyWidgetStore>>,
    columns: HashMap<TypeId, u64>,
    root: NodeId,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
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
                type_id: None,
                parent: root,
                children: Vec::new(),
            },
        );
        Self {
            nodes,
            widget_stores: Vec::new(),
            columns: HashMap::new(),
            root,
        }
    }

    /// Always valid; the only node whose parent is itself.
    pub fn root(&self) -> NodeId {
        self.root
    }

    // ── write ────────────────────────────────────────────────────────────────

    /// Build `builder`'s widget (and any subtree it describes) as the last
    /// child of `parent`.
    ///
    /// The node and widget slots are reserved *before* the builder runs, so
    /// the id it receives is the id returned here. If a nested
    /// [`Spawner::child`] fails, everything built so far is torn down and the
    /// error is returned; the tree is left as it was.
    pub fn spawn<B: WidgetBuild>(&mut self, parent: NodeId, builder: B) -> Result<NodeId, Error> {
        self.nodes.get(parent).ok_or(Error::Stale)?;

        let type_id = TypeId::of::<B::Widget>();
        let column = self.column::<B::Widget>();
        let widget_index = self.widget_store_mut::<B::Widget>(column).reserve();
        let (component_index, generation) = self.nodes.reserve();
        let id = NodeId::new(column, widget_index, component_index, generation);

        self.nodes.fill(
            component_index,
            Node {
                type_id: Some(type_id),
                parent,
                children: Vec::new(),
            },
        );
        self.nodes
            .get_mut(parent)
            .expect("parent validated above")
            .children
            .push(id);

        let mut spawner = Spawner {
            app: self,
            failed: None,
        };
        let widget = builder.spawn(id, &mut spawner);
        if let Some(e) = spawner.failed {
            self.remove(id).expect("partially built node is live");
            return Err(e);
        }

        self.widget_store_mut::<B::Widget>(column).fill(
            widget_index,
            WidgetWrapper {
                node_id: id,
                widget,
            },
        );
        Ok(id)
    }

    /// Remove `id` and every node under it. Every id in the subtree is stale
    /// afterwards; the freed slots are reused by later spawns.
    pub fn remove(&mut self, id: NodeId) -> Result<(), Error> {
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
            pending.extend(node.children);
        }
        Ok(())
    }

    // ── read ─────────────────────────────────────────────────────────────────

    /// `None` if `id` is stale. The root's parent is the root.
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(id).map(|n| n.parent)
    }

    /// In sibling order. `None` if `id` is stale.
    pub fn children(&self, id: NodeId) -> Option<&[NodeId]> {
        self.nodes.get(id).map(|n| n.children.as_slice())
    }

    /// `None` if `id` is stale, is the root, or holds a widget of another type.
    pub fn widget<W: Widget>(&self, id: NodeId) -> Option<&W> {
        self.check_type::<W>(id)?;
        self.widget_store::<W>(id.widget_column()).get(id)
    }

    /// `None` if `id` is stale, is the root, or holds a widget of another type.
    pub fn widget_mut<W: Widget>(&mut self, id: NodeId) -> Option<&mut W> {
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

/// What a [`WidgetBuild`] gets to touch while it runs: the ability to attach
/// children, and nothing else.
pub struct Spawner<'a> {
    app: &'a mut App,
    failed: Option<Error>,
}

impl Spawner<'_> {
    /// Build `builder` as the last child of `parent`, which should be the
    /// `me` the builder was given or an id returned by an earlier `child`.
    ///
    /// A builder has no way to propagate an error, so if `parent` is stale
    /// this returns an id that matches nothing and remembers the error;
    /// further `child` calls are no-ops and the enclosing [`App::spawn`]
    /// returns `Err` after tearing down whatever was built.
    pub fn child<B: WidgetBuild>(&mut self, parent: NodeId, builder: B) -> NodeId {
        if self.failed.is_some() {
            return NodeId::INVALID;
        }
        match self.app.spawn(parent, builder) {
            Ok(id) => id,
            Err(e) => {
                self.failed = Some(e);
                NodeId::INVALID
            }
        }
    }
}
