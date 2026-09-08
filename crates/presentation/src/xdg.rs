//! The xdg-shell role: every window whose `Role` is `Toplevel`.
//!
//! On `Tick` a toplevel window without its objects gets an `xdg_surface`
//! and an `xdg_toplevel`, the title from `Window`, and an initial commit
//! with no buffer, which is what asks the compositor for its first
//! configure. A toplevel configure only stores what was offered; the
//! `xdg_surface` configure that ends the sequence acks and calls
//! `configured`. Teardown is `Toplevel`'s `Drop`: toplevel, then
//! xdg_surface, then the wl_surface.

use app::{App, Component, Module, NodeId, Tick};
use wayland::{
    Globals, Proxy, WlSurface, XdgSurface, XdgSurfaceEvent, XdgToplevel, XdgToplevelEvent,
    XdgWmBase, XdgWmBaseEvent,
};
use window::{Role, Window};

use crate::{Surface, WindowOf, configured, windows};

/// The xdg objects of a toplevel window and the last size the compositor
/// offered. Empty on every other node. Dropping it destroys the objects,
/// role first, then the surface it was given.
#[derive(Debug, Default)]
pub struct Toplevel {
    objects: Option<Objects>,
    /// What the last `xdg_toplevel.configure` offered; zero means ours to
    /// choose.
    pub offered: (u32, u32),
    pub states: Vec<u8>,
}
impl Component for Toplevel {}

#[derive(Debug)]
struct Objects {
    surface: Proxy<WlSurface>,
    xdg_surface: Proxy<XdgSurface>,
    toplevel: Proxy<XdgToplevel>,
}

impl Toplevel {
    pub fn xdg_surface(&self) -> Option<&Proxy<XdgSurface>> {
        self.objects.as_ref().map(|o| &o.xdg_surface)
    }

    pub fn toplevel(&self) -> Option<&Proxy<XdgToplevel>> {
        self.objects.as_ref().map(|o| &o.toplevel)
    }
}

impl Drop for Toplevel {
    fn drop(&mut self) {
        if let Some(o) = self.objects.take() {
            if o.toplevel.is_alive() {
                o.toplevel.destroy();
            }
            if o.xdg_surface.is_alive() {
                o.xdg_surface.destroy();
            }
            if o.surface.is_alive() {
                o.surface.destroy();
            }
        }
    }
}

pub struct XdgModule;

impl Module for XdgModule {
    fn install(self, app: &mut App) {
        app.register_component::<Toplevel>()
            .system(on_tick)
            .system(on_ping)
            .system(on_toplevel_event)
            .system(on_surface_event);
    }
}

/// Every toplevel window that has a surface and no xdg objects gets them.
fn on_tick(app: &mut App, _: &Tick) {
    let missing: Vec<NodeId> = {
        let roles = app
            .components::<Role>()
            .expect("no Role writer is alive across a Tick");
        let surfaces = app
            .components::<Surface>()
            .expect("no Surface holder is alive across a Tick");
        let toplevels = app
            .components::<Toplevel>()
            .expect("no Toplevel holder is alive across a Tick");
        windows(app)
            .into_iter()
            .filter(|w| {
                roles.get(*w) == Some(&Role::Toplevel)
                    && surfaces.get(*w).is_some_and(|s| s.proxy.is_some())
                    && toplevels.get(*w).is_some_and(|t| t.objects.is_none())
            })
            .collect()
    };
    for window in missing {
        create(app, window);
    }
}

fn on_ping(_: &mut App, ev: &XdgWmBaseEvent) {
    let XdgWmBaseEvent::Ping { sender, serial } = ev;
    sender.pong(*serial);
}

fn on_toplevel_event(app: &mut App, ev: &XdgToplevelEvent) {
    match ev {
        XdgToplevelEvent::Configure {
            sender,
            width,
            height,
            states,
        } => {
            let Some(window) = app
                .resource::<WindowOf>()
                .expect("no WindowOf writer is alive across a configure")
                .get(sender)
            else {
                return;
            };
            if let Some(t) = app
                .components_mut::<Toplevel>()
                .expect("no Toplevel holder is alive across a configure")
                .get_mut(window)
            {
                t.offered = (*width as u32, *height as u32);
                t.states = states.clone();
            }
        }
        // The compositor's close is a remove; the window module does the
        // rest, and the last window closing stops the app.
        XdgToplevelEvent::Close { sender } => {
            let window = app
                .resource::<WindowOf>()
                .expect("no WindowOf writer is alive across a close")
                .get(sender);
            if let Some(window) = window {
                let _ = app.remove(window);
            }
        }
        _ => {}
    }
}

fn on_surface_event(app: &mut App, ev: &XdgSurfaceEvent) {
    let XdgSurfaceEvent::Configure { sender, serial } = ev;
    let Some(window) = app
        .resource::<WindowOf>()
        .expect("no WindowOf writer is alive across a configure")
        .get(sender)
    else {
        return;
    };
    sender.ack_configure(*serial);
    let (w, h) = app
        .components::<Toplevel>()
        .expect("no Toplevel holder is alive across a configure")
        .get(window)
        .map_or((0, 0), |t| t.offered);
    configured(app, window, w, h);
}

fn create(app: &mut App, window: NodeId) {
    let wm_base = app
        .resource::<Globals>()
        .expect("no Globals writer is alive across a Tick")
        .require::<XdgWmBase>()
        .clone();
    let surface = app
        .components::<Surface>()
        .expect("no Surface holder is alive across a Tick")
        .get(window)
        .expect("checked by the caller")
        .proxy()
        .clone();
    let title = app
        .widget::<Window>(window)
        .map(|w| w.title.clone())
        .unwrap_or_default();
    let xdg_surface = wm_base.get_xdg_surface(&surface);
    let toplevel = xdg_surface.get_toplevel();
    toplevel.set_title(&title);
    toplevel.set_app_id(&title);
    surface.commit();

    {
        let mut map = app
            .resource_mut::<WindowOf>()
            .expect("no WindowOf holder is alive across a Tick");
        map.insert(&xdg_surface, window);
        map.insert(&toplevel, window);
    }
    if let Some(t) = app
        .components_mut::<Toplevel>()
        .expect("no Toplevel holder is alive across a Tick")
        .get_mut(window)
    {
        t.objects = Some(Objects {
            surface,
            xdg_surface,
            toplevel,
        });
        t.offered = (0, 0);
        t.states.clear();
    }
}
