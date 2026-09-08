//! The layout module: `LayoutStyle` and `Measure` in, `Layout` out, through
//! taffy's algorithms over our own columns.
//!
//! Dirty by comparison, run once on `Tick`. The core has no change events,
//! so the module keeps a private copy of what the last pass saw of every
//! node — its style, its measure and its child list — and the system on
//! `Tick` compares. A node whose copy differs empties its `LayoutCache` and
//! its ancestors', and the pass runs; a tick where nothing differs costs the
//! comparison and nothing more. Either way [`LayoutDone`] is signalled, so
//! anything after layout can chain on it unconditionally.
//!
//! The pass builds a short-lived view over the tree and the module's columns
//! and implements taffy's low-level traits on it, with the node slot as
//! taffy's id. `LayoutStyle` is our own type; taffy's style traits are
//! implemented on it, converting per getter, so no taffy type is a field and
//! none appears in a builder.

mod style;
mod tree;

use app::{App, Component, Module, NodeId, Signal, Tick};
use taffy::compute::{compute_root_layout, round_layout};
use taffy::geometry::Size as TSize;
use taffy::style_helpers::TaffyMaxContent;
use taffy::tree::NodeId as TaffyId;
use utils::{Edges, Point, Rect, Size};

pub use style::{
    Align, Direction, Display, Justify, LayoutStyle, Position, Val, Wrap, auto, percent, px,
};
use tree::LayoutTree;

pub mod prelude {
    pub use crate::{
        Align, Direction, Display, Justify, Layout, LayoutDone, LayoutModule, LayoutStyle, Measure,
        Position, Wrap, auto, percent, px,
    };
    pub use utils::{Edges, edges};
}

impl Component for LayoutStyle {}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// A node's intrinsic size, in logical pixels. `None` measures as zero. A
/// widget that has one writes it whenever the state it derives from changes.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Measure(pub Option<Size>);
impl Component for Measure {}

/// The box layout resolved for a node: absolute, rounded, in logical pixels.
/// `padding` and `border` are what the content box is inset by.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Layout {
    pub rect: Rect,
    pub padding: Edges<f32>,
    pub border: Edges<f32>,
}
impl Component for Layout {}

impl Layout {
    /// The rect inside padding and border.
    pub fn content(&self) -> Rect {
        let l = self.padding.left + self.border.left;
        let t = self.padding.top + self.border.top;
        let r = self.padding.right + self.border.right;
        let b = self.padding.bottom + self.border.bottom;
        Rect::new(
            self.rect.x() + l,
            self.rect.y() + t,
            (self.rect.width() - l - r).max(0.0),
            (self.rect.height() - t - b).max(0.0),
        )
    }
}

/// What layout measured for a node under given constraints. The only notion
/// of dirty layout has: emptied for a node and its ancestors on a change.
#[derive(Default)]
pub struct LayoutCache(taffy::tree::Cache);
impl Component for LayoutCache {}

/// The box before rounding, which taffy's rounding reads. Persists across
/// frames because a cached subtree is not revisited.
#[derive(Default, Clone, Copy)]
struct Unrounded(taffy::tree::Layout);
impl Component for Unrounded {}

/// What the last pass saw of a node. Compared on every `Tick`; a difference
/// is the change the core has no event for.
#[derive(Default)]
struct Seen {
    style: LayoutStyle,
    measure: Measure,
    children: Vec<NodeId>,
}
impl Component for Seen {}

// ---------------------------------------------------------------------------
// Signal
// ---------------------------------------------------------------------------

/// Layout has run for this tick. Always signalled, so paint can chain on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDone {
    /// Nothing had changed; every `Layout` is as it was.
    Unchanged,
    Recomputed,
}
impl Signal for LayoutDone {}

// ---------------------------------------------------------------------------
// The module
// ---------------------------------------------------------------------------

/// Registers the layout columns and the system on `Tick` that runs the pass.
pub struct LayoutModule;

impl Module for LayoutModule {
    fn install(self, app: &mut App) {
        app.register_component::<LayoutStyle>()
            .register_component::<Measure>()
            .register_component::<Layout>()
            .register_component::<LayoutCache>()
            .register_component::<Unrounded>()
            .register_component::<Seen>()
            .system(on_tick);
    }
}

/// The end of a batch: one pass, however many writes the batch made.
fn on_tick(app: &mut App, _: &Tick) {
    if !find_dirty(app) {
        app.signal(LayoutDone::Unchanged);
        return;
    }
    let root = app.root();
    {
        let mut tree = LayoutTree::new(app);
        let root_id = TaffyId::from(root.component_index());
        compute_root_layout(&mut tree, root_id, TSize::MAX_CONTENT);
        round_layout(&mut tree, root_id);
        tree.absolutize(root, Point::ZERO);
    }
    app.signal(LayoutDone::Recomputed);
}

/// Compare every live node with what the last pass saw. A node whose style,
/// measure or child list differs has its cache and its ancestors' emptied,
/// and the copy refreshed. Returns whether anything differed.
///
/// A fresh node has an empty cache already; what needs emptying is its
/// parent's, and the parent's child list is what differs. A removed node is
/// gone from the columns; its former parent's child list is what differs.
fn find_dirty(app: &mut App) -> bool {
    let styles = app
        .components::<LayoutStyle>()
        .expect("no LayoutStyle writer is alive across a Tick");
    let measures = app
        .components::<Measure>()
        .expect("no Measure writer is alive across a Tick");
    let mut seen = app
        .components_mut::<Seen>()
        .expect("layout's private column is never lent out");
    let mut caches = app
        .components_mut::<LayoutCache>()
        .expect("no LayoutCache holder is alive across a Tick");

    let mut dirty = false;
    for (id, style) in styles.iter() {
        let Some(measure) = measures.get(id) else {
            continue;
        };
        let Some(seen) = seen.get_mut(id) else {
            continue;
        };
        let children = app.children(id).unwrap_or(&[]);
        if seen.style == *style && seen.measure == *measure && seen.children == children {
            continue;
        }
        seen.style = style.clone();
        seen.measure = *measure;
        seen.children = children.to_vec();
        clear_up(app, &mut caches, id);
        dirty = true;
    }
    dirty
}

/// Empties the cache of `id` and every ancestor. Walks all the way up rather
/// than stopping at an already-empty cache, so a fresh node under a laid-out
/// parent still reaches the root.
fn clear_up(app: &App, caches: &mut app::CompsMut<LayoutCache>, id: NodeId) {
    let mut at = id;
    loop {
        if let Some(cache) = caches.get_mut(at) {
            cache.0.clear();
        }
        match app.parent(at) {
            Some(parent) if parent != at => at = parent,
            _ => return,
        }
    }
}
