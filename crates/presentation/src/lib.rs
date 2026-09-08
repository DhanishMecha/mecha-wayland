//! Presentation: what puts a `Window` on the compositor.
//!
//! [`SurfaceModule`] gives every window its `wl_surface`, turns the
//! compositor's frame callback into `Frame`, and owns [`configured`], the
//! one function both shells call for a size the compositor settled.
//! [`XdgModule`] and [`LayerShellModule`] each handle the windows whose
//! `Role` is theirs and ignore the rest; [`SeatModule`] turns the seat's
//! pointer into `PointerInput`. [`PresentationModule`] installs the four in
//! that order.
//!
//! The core has no spawn or removal events, so windows are found on `Tick`:
//! after the window module has brought `Windows` up to date, every window
//! without a surface gets one, and every shell gives its role objects to
//! the windows of its role that lack them. Teardown is the role component's
//! `Drop`: the core resets a removed node's components, and the role
//! component holds its objects and a clone of the `wl_surface`, destroying
//! them in the order the protocol wants, role first. [`Surface`] itself
//! destroys nothing.
//!
//! `SurfaceModule` is also the on-demand frame loop. A `RequestFrame` for a
//! window that is idle, configured and not already drawing is answered with
//! a `Frame` signal, and right behind it a private signal on which the
//! surface is committed: whoever answered `Frame` attached its buffer and
//! marked [`Surface::attached`], and the commit requests a new callback in
//! the same go. A request that arrives while a callback is outstanding, or
//! while a frame is being drawn, only marks the window as wanting another;
//! when `done` comes back, a wanted window goes round again. A request
//! before the first `configured` waits for it, because a buffer attached
//! before the initial configure is a protocol error.
//!
//! Nothing here draws. The renderer's module answers `Frame`; it attaches
//! and damages, and never commits or asks.
//!
//! Install order is the seam: `WindowModule`, then `PresentationModule`.

use std::collections::HashMap;

use app::{App, Component, Module, NodeId, Resource, Signal, Tick};
use layout::Layout;
use ring::Stop;
use utils::Size;
use wayland::{
    Globals, Interface, ObjectId, Proxy, WlCallback, WlCallbackEvent, WlCompositor, WlSurface,
};
use window::{AllWindowsClosed, Frame, RequestFrame, Resized, Window, Windows};

mod layer;
mod seat;
mod xdg;
pub use layer::{LayerShellModule, LayerSurface};
pub use seat::{Seat, SeatModule};
pub use xdg::{Toplevel, XdgModule};

pub mod prelude {
    pub use crate::{
        LayerShellModule, LayerSurface, PresentationModule, Seat, SeatModule, Surface,
        SurfaceModule, Toplevel, XdgModule,
    };
}

// ---------------------------------------------------------------------------
// Surface
// ---------------------------------------------------------------------------

/// The compositor's object behind a window, and where the window is in the
/// frame loop. Empty on every node that is not a window.
#[derive(Debug, Default)]
pub struct Surface {
    proxy: Option<Proxy<WlSurface>>,
    /// The frame callback outstanding, if any.
    pending: Option<Proxy<WlCallback>>,
    /// A `Frame` has been signalled and its commit has not run yet.
    drawing: bool,
    /// A request arrived while a callback was outstanding, a frame was
    /// being drawn, or before the first configure; answered when the
    /// callback returns or the configure arrives.
    wanted: bool,
    /// The compositor has settled a size at least once, so a buffer may be
    /// attached.
    configured: bool,
    /// Whoever answered the current `Frame` attached a buffer. Set by the
    /// renderer inside `Frame`; read and cleared by the commit that
    /// follows. A frame that attached nothing is not committed, because a
    /// commit with no new content earns no callback until the output
    /// repaints for some other reason, and a wanted frame would wait on it.
    pub attached: bool,
}
impl Component for Surface {}

impl Surface {
    /// The surface, for a shell or a renderer that needs it.
    ///
    /// # Panics
    ///
    /// On a node that has none: one that is not a window, or a window the
    /// `Tick` has not reached yet.
    pub fn proxy(&self) -> &Proxy<WlSurface> {
        self.proxy
            .as_ref()
            .expect("node has no wl_surface: is it a window, and has a Tick run since it spawned?")
    }

    /// Whether the compositor has settled a size for the window.
    pub fn is_configured(&self) -> bool {
        self.configured
    }
}

/// Which window a compositor object belongs to, for events that name only
/// their sender. Every shell writes its role objects in; `SurfaceModule`
/// writes the surface and each frame callback.
#[derive(Default)]
pub struct WindowOf {
    map: HashMap<ObjectId, NodeId>,
}
impl Resource for WindowOf {}

impl WindowOf {
    pub fn insert<T: Interface>(&mut self, p: &Proxy<T>, window: NodeId) {
        if let Some(id) = p.object_id() {
            self.map.insert(id, window);
        }
    }

    pub fn remove<T: Interface>(&mut self, p: &Proxy<T>) {
        if let Some(id) = p.object_id() {
            self.map.remove(&id);
        }
    }

    pub fn get<T: Interface>(&self, p: &Proxy<T>) -> Option<NodeId> {
        p.object_id().and_then(|id| self.map.get(&id)).copied()
    }

    /// Drops every entry whose window is gone.
    fn sweep(&mut self, app: &App) {
        self.map.retain(|_, w| is_window(app, *w));
    }
}

/// True for a live window node.
pub fn is_window(app: &App, id: NodeId) -> bool {
    app.widget::<Window>(id).is_some()
}

/// Every live window, in spawn order, as the window module lists them.
pub(crate) fn windows(app: &App) -> Vec<NodeId> {
    app.resource::<Windows>()
        .expect("no Windows writer is alive across a signal")
        .iter()
        .collect()
}

/// The private second half of a frame: the commit after whoever draws has
/// answered `Frame`. Signalled right behind `Frame`, so the two run back
/// to back.
struct AfterFrame(NodeId);
impl Signal for AfterFrame {}

pub struct SurfaceModule;

impl Module for SurfaceModule {
    fn install(self, app: &mut App) {
        app.register_component::<Surface>()
            .insert_resource(WindowOf::default())
            .system(on_tick)
            .system(on_request_frame)
            .system(after_frame)
            .system(on_callback_done)
            .system(on_all_windows_closed);
    }
}

/// Every window without a surface gets one; every object of a window that
/// is gone is forgotten.
fn on_tick(app: &mut App, _: &Tick) {
    let missing: Vec<NodeId> = {
        let surfaces = app
            .components::<Surface>()
            .expect("no Surface holder is alive across a Tick");
        windows(app)
            .into_iter()
            .filter(|w| surfaces.get(*w).is_some_and(|s| s.proxy.is_none()))
            .collect()
    };
    for window in missing {
        let surface = app
            .resource::<Globals>()
            .expect("no Globals writer is alive across a Tick")
            .require::<WlCompositor>()
            .create_surface();
        app.resource_mut::<WindowOf>()
            .expect("no WindowOf holder is alive across a Tick")
            .insert(&surface, window);
        if let Some(s) = app
            .components_mut::<Surface>()
            .expect("no Surface holder is alive across a Tick")
            .get_mut(window)
        {
            s.proxy = Some(surface);
        }
    }
    let mut map = app
        .resource_mut::<WindowOf>()
        .expect("no WindowOf holder is alive across a Tick");
    map.sweep(app);
}

/// Answered at once when the window is idle; folded into what is already
/// under way otherwise.
fn on_request_frame(app: &mut App, RequestFrame(window): &RequestFrame) {
    let window = *window;
    {
        let mut surfaces = app
            .components_mut::<Surface>()
            .expect("no Surface holder is alive across a RequestFrame");
        let Some(s) = surfaces.get_mut(window) else {
            return;
        };
        if s.proxy.is_none() || s.pending.is_some() || s.drawing || !s.configured {
            s.wanted = true;
            return;
        }
        s.drawing = true;
    }
    app.signal(Frame(window));
    app.signal(AfterFrame(window));
}

/// Whoever drew has attached, or not. An attached buffer is committed with
/// a callback requested in the same go; a frame that attached nothing is
/// not committed, and whoever skipped it asks again when it can draw.
fn after_frame(app: &mut App, AfterFrame(window): &AfterFrame) {
    let window = *window;
    let proxy = {
        let mut surfaces = app
            .components_mut::<Surface>()
            .expect("no Surface holder is alive after a Frame");
        let Some(s) = surfaces.get_mut(window) else {
            return;
        };
        s.drawing = false;
        if !s.attached {
            s.wanted = false;
            return;
        }
        s.attached = false;
        s.proxy.clone()
    };
    let Some(proxy) = proxy else {
        return;
    };
    let callback = proxy.frame();
    proxy.commit();
    app.resource_mut::<WindowOf>()
        .expect("no WindowOf holder is alive after a Frame")
        .insert(&callback, window);
    if let Some(s) = app
        .components_mut::<Surface>()
        .expect("no Surface holder is alive after a Frame")
        .get_mut(window)
    {
        s.pending = Some(callback);
    }
}

/// The compositor is done with the last frame. A wanted window goes round
/// again; an unwanted one goes idle.
fn on_callback_done(app: &mut App, ev: &WlCallbackEvent) {
    let WlCallbackEvent::Done { sender, .. } = ev;
    let window = {
        let mut map = app
            .resource_mut::<WindowOf>()
            .expect("no WindowOf holder is alive across a callback");
        let Some(window) = map.get(sender) else {
            return;
        };
        map.remove(sender);
        window
    };
    let wanted = {
        let mut surfaces = app
            .components_mut::<Surface>()
            .expect("no Surface holder is alive across a callback");
        let Some(s) = surfaces.get_mut(window) else {
            return;
        };
        if s.pending.as_ref() != Some(sender) {
            return;
        }
        s.pending = None;
        std::mem::take(&mut s.wanted)
    };
    if wanted {
        app.signal(RequestFrame(window));
    }
}

/// The last window closing is the app's end.
fn on_all_windows_closed(app: &mut App, _: &AllWindowsClosed) {
    app.signal(Stop);
}

/// The compositor settled a size for `window`. A zero dimension means "keep
/// yours" and is resolved from the window's `Layout`, which is valid because
/// layout ran at the end of the batch that spawned it. If the size differs,
/// `Resized` is emitted and nothing is presented yet: the window rewrites
/// its style, layout recomputes on the next `Tick`, and that recompute
/// requests the frame, so the first buffer is already the configured size.
/// If it does not, a frame is requested now. Either way the commit that
/// follows is what acknowledges the configure on the wire.
pub fn configured(app: &mut App, window: NodeId, width: u32, height: u32) {
    let Some(current) = app
        .components::<Layout>()
        .expect("no Layout writer is alive across a configure")
        .get(window)
        .map(|l| l.rect)
    else {
        return;
    };
    let size = Size::new(
        if width == 0 {
            current.width()
        } else {
            width as f32
        },
        if height == 0 {
            current.height()
        } else {
            height as f32
        },
    );
    {
        let mut surfaces = app
            .components_mut::<Surface>()
            .expect("no Surface holder is alive across a configure");
        let Some(s) = surfaces.get_mut(window) else {
            return;
        };
        s.configured = true;
        s.wanted = false;
    }
    if size != current.size {
        app.emit(Resized { size }, &[window]);
        return;
    }
    app.signal(RequestFrame(window));
}

// ---------------------------------------------------------------------------
// The bundle of modules
// ---------------------------------------------------------------------------

/// `SurfaceModule`, the shells, then the seat, in the order they must
/// install. Added after `WindowModule` and `InteractivityModule`.
pub struct PresentationModule;

impl Module for PresentationModule {
    fn install(self, app: &mut App) {
        app.add_module(SurfaceModule)
            .add_module(XdgModule)
            .add_module(LayerShellModule)
            .add_module(SeatModule);
    }
}
