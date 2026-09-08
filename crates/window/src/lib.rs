//! The window module: the `Window` widget, which window a node belongs to,
//! the list of live windows, and the resize a backend reports.
//!
//! A window's [`Role`] is what it is to the shell, fixed at spawn and given
//! to the builder; [`RequestFrame`] is how a window is asked to be presented
//! again, and [`Frame`] the answer. This module raises the request itself,
//! from `LayoutDone::Recomputed` for every window and from `PaintChanged`
//! for each changed node's window, so layout and paint stay leaves that
//! know no window.
//!
//! Nothing here knows a compositor. A window is an ordinary node whose size
//! is its `LayoutStyle`: the builder writes the size it was given as a
//! request, and whatever presents the window later answers with [`Resized`],
//! which the window handles by rewriting its own style. The module that
//! presents windows and feeds them events is a separate one, built against
//! this surface.

use app::{
    App, Component, Context, Event, Handle, Module, NodeId, Resource, Signal, Spawner, Tick,
    Widget, WidgetBuild,
};
use layout::{LayoutDone, LayoutStyle, px};
use paint::{Paint, PaintChanged, Quad};
use utils::{Edges, Size};

pub mod prelude {
    pub use crate::{
        AllWindowsClosed, Anchor, Frame, Keyboard, Layer, LayerKind, RequestFrame, Resized, Role,
        Window, WindowBuilder, WindowModule, Windows, window, window_of,
    };
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

/// A node presented on its own surface. Holds intent only; everything the
/// presenting side attaches is a component of its own.
pub struct Window {
    pub title: String,
}
impl Widget for Window {}

/// A toplevel with no title, the default style and no background.
pub fn window() -> WindowBuilder {
    WindowBuilder {
        title: String::new(),
        layout: LayoutStyle::default(),
        background: Paint::None,
        role: Role::default(),
    }
}

/// Carries what the spawn site knows about the window: its title, the box
/// it asks for, the paint behind its content, and its role. The builder has
/// no style verbs; a whole `LayoutStyle` is given, and `absolute()` composed
/// onto it so a window is never a flex item of its parent.
pub struct WindowBuilder {
    title: String,
    layout: LayoutStyle,
    background: Paint,
    role: Role,
}

impl WindowBuilder {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    /// The box the window asks for; its size is a request until the
    /// presenting side answers with [`Resized`].
    pub fn layout(mut self, layout: LayoutStyle) -> Self {
        self.layout = layout;
        self
    }
    /// The quad drawn behind the window's content. Without one the window
    /// is transparent, which is what an overlay wants.
    pub fn background(mut self, quad: Quad) -> Self {
        self.background = Paint::Quad(quad);
        self
    }
    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }
}

impl WidgetBuild for WindowBuilder {
    type Widget = Window;

    /// Writes the style absolute, the background and the role, registers
    /// itself in [`Windows`], and listens for [`Resized`].
    fn spawn(self, me: Handle<Window>, s: &mut Spawner<Window>) -> Window {
        s.set_component(self.layout.absolute());
        s.set_component(self.background);
        s.set_component(self.role);
        if let Some(mut windows) = s.resource_mut::<Windows>() {
            windows.push(me.id());
        }
        s.on(me, on_resized);
        Window { title: self.title }
    }
}

/// The space actually given is the window's new size request; the next
/// `Tick` lays it out.
fn on_resized(ctx: &mut Context<Window>, r: &Resized) {
    let Some(style) = ctx.component::<LayoutStyle>().map(|s| (*s).clone()) else {
        return;
    };
    ctx.set_component(style.size(px(r.size.width()), px(r.size.height())));
}

// ---------------------------------------------------------------------------
// Resource, events, signals
// ---------------------------------------------------------------------------

/// Every live window, in spawn order. A window's builder adds it; the system
/// on `Tick` drops what has been removed and adds anything a builder could
/// not (the resource was lent out as it spawned), so the list is exact at
/// the end of every batch.
#[derive(Debug, Default)]
pub struct Windows {
    ids: Vec<NodeId>,
}
impl Resource for Windows {}

impl Windows {
    pub fn iter(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.ids.iter().copied()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    fn push(&mut self, id: NodeId) {
        if !self.ids.contains(&id) {
            self.ids.push(id);
        }
    }
}

/// The nearest window at or above `id`: the node itself if it is a window,
/// else the closest ancestor that is. `None` for the root, for a stale id,
/// and for anything hung outside every window.
pub fn window_of(app: &App, id: NodeId) -> Option<NodeId> {
    let mut at = id;
    loop {
        if app.widget::<Window>(at).is_some() {
            return Some(at);
        }
        let parent = app.parent(at)?;
        if parent == at {
            return None;
        }
        at = parent;
    }
}

/// The space a window was actually given, in logical pixels, never zero.
/// Emitted by the presenting side at the window node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resized {
    pub size: Size,
}
impl Event for Resized {}

/// Please present this window again. Signalled by this module when a layout
/// is recomputed or a paint changes, and by the presenting side when a size
/// is settled; cheap to repeat, since the presenting side holds at most one
/// outstanding request per window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestFrame(pub NodeId);
impl Signal for RequestFrame {}

/// Draw this window now: the presenting side's answer to a [`RequestFrame`],
/// signalled by it and by nothing else, at once when the window is idle and
/// after the compositor's callback when it is not. Whoever draws attaches
/// its buffer inside this signal; the presenting side commits right after.
/// Not when layout runs; that is `Tick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame(pub NodeId);
impl Signal for Frame {}

/// The last window is gone. Signalled once, on the `Tick` that finds
/// [`Windows`] empty after it held something; a runner that wants to stop
/// on it listens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllWindowsClosed;
impl Signal for AllWindowsClosed {}

// ---------------------------------------------------------------------------
// Role
// ---------------------------------------------------------------------------

/// What a window is to the shell that shows it. Fixed for the window's life,
/// so it is given to the builder, never set after. The default is an
/// ordinary toplevel. A component, so the module that handles each role
/// reads it off the window node.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Role {
    /// An ordinary window the compositor places and decorates.
    #[default]
    Toplevel,
    /// A panel or overlay anchored to screen edges.
    Layer(Layer),
}
impl Component for Role {}

/// How a layer window sits on the screen. A dimension the window's
/// `LayoutStyle` gives as `px` is fixed; anything else stretches between the
/// anchored edges.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub layer: LayerKind,
    pub anchor: Anchor,
    /// Space the compositor keeps clear for this window along its anchored
    /// edge; zero asks for none, negative asks not to be pushed by others.
    pub exclusive_zone: i32,
    pub keyboard: Keyboard,
    /// Margins from the anchored edges, in logical pixels.
    pub margin: Edges<i32>,
    pub namespace: String,
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            layer: LayerKind::Top,
            anchor: Anchor::empty(),
            exclusive_zone: 0,
            keyboard: Keyboard::None,
            margin: Edges::default(),
            namespace: String::new(),
        }
    }
}

/// The stacking layer, bottom to top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    Background,
    Bottom,
    Top,
    Overlay,
}

bitflags::bitflags! {
    /// The screen edges a layer window is attached to.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Anchor: u32 {
        const TOP = 1;
        const BOTTOM = 2;
        const LEFT = 4;
        const RIGHT = 8;
    }
}

/// Whether a layer window takes keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyboard {
    None,
    Exclusive,
    OnDemand,
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

/// Registers [`Role`], inserts [`Windows`], and attaches the systems that
/// keep the list and raise [`RequestFrame`]. Added after `LayoutModule` and
/// `PaintModule`, whose signals it listens for.
pub struct WindowModule;

impl Module for WindowModule {
    fn install(self, app: &mut App) {
        app.register_component::<Role>()
            .insert_resource(Windows::default())
            .system(sync_windows)
            .system(on_layout_done)
            .system(on_paint_changed);
    }
}

/// Every window that is live is listed, in the order first seen; nothing
/// else is. The last window closing is signalled.
fn sync_windows(app: &mut App, _: &Tick) {
    let live: Vec<NodeId> = app.widgets::<Window>().map(|(id, _)| id).collect();
    let closed_all = {
        let mut windows = app
            .resource_mut::<Windows>()
            .expect("no Windows holder is alive across a Tick");
        let before = windows.len();
        windows.ids.retain(|w| live.contains(w));
        for id in live {
            windows.push(id);
        }
        before > 0 && windows.is_empty()
    };
    if closed_all {
        app.signal(AllWindowsClosed);
    }
}

/// A relayout may have moved anything, and layout does not say what, so
/// every window is asked for a frame; presentation folds repeats. Spawn and
/// removal end here too, since both dirty layout.
fn on_layout_done(app: &mut App, done: &LayoutDone) {
    if *done != LayoutDone::Recomputed {
        return;
    }
    let windows: Vec<NodeId> = app
        .resource::<Windows>()
        .expect("no Windows writer is alive across a signal")
        .iter()
        .collect();
    for w in windows {
        app.signal(RequestFrame(w));
    }
}

/// Each changed node's window is asked for a frame, once per window per
/// batch of changes. A node outside every window asks nothing; a relayout
/// covers what its spawn dirtied.
fn on_paint_changed(app: &mut App, changed: &PaintChanged) {
    let mut asked: Vec<NodeId> = Vec::new();
    for &id in &changed.0 {
        if let Some(w) = window_of(app, id)
            && !asked.contains(&w)
        {
            asked.push(w);
        }
    }
    for w in asked {
        app.signal(RequestFrame(w));
    }
}
