//! The wlr-layer-shell role: every window whose `Role` is `Layer`.
//!
//! Size and anchors must be sent before the initial commit, so the `Tick`
//! that creates the layer surface reads the window's `LayoutStyle`: a `px`
//! dimension is fixed, anything else is sent as zero, which is the
//! protocol's "stretch between the anchored edges". The compositor's
//! configure then carries the size it chose, and `configured` turns it into
//! `Resized`. Teardown is `LayerSurface`'s `Drop`: the layer surface, then
//! the wl_surface.

use app::{App, Component, Module, NodeId, Tick};
use layout::{LayoutStyle, Val};
use wayland::{
    Globals, Proxy, WlSurface, ZwlrLayerShellV1, ZwlrLayerShellV1Layer, ZwlrLayerSurfaceV1,
    ZwlrLayerSurfaceV1Anchor, ZwlrLayerSurfaceV1Event, ZwlrLayerSurfaceV1KeyboardInteractivity,
};
use window::{Anchor, Keyboard, Layer, LayerKind, Role};

use crate::{Surface, WindowOf, configured, windows};

/// The layer surface of a layer window. Empty on every other node. Dropping
/// it destroys the layer surface, then the surface it was given.
#[derive(Debug, Default)]
pub struct LayerSurface {
    objects: Option<Objects>,
}
impl Component for LayerSurface {}

#[derive(Debug)]
struct Objects {
    surface: Proxy<WlSurface>,
    layer_surface: Proxy<ZwlrLayerSurfaceV1>,
}

impl LayerSurface {
    pub fn layer_surface(&self) -> Option<&Proxy<ZwlrLayerSurfaceV1>> {
        self.objects.as_ref().map(|o| &o.layer_surface)
    }
}

impl Drop for LayerSurface {
    fn drop(&mut self) {
        if let Some(o) = self.objects.take() {
            if o.layer_surface.is_alive() {
                o.layer_surface.destroy();
            }
            if o.surface.is_alive() {
                o.surface.destroy();
            }
        }
    }
}

pub struct LayerShellModule;

impl Module for LayerShellModule {
    fn install(self, app: &mut App) {
        app.register_component::<LayerSurface>()
            .system(on_tick)
            .system(on_event);
    }
}

/// Every layer window that has a surface and no layer surface gets one.
fn on_tick(app: &mut App, _: &Tick) {
    let missing: Vec<(NodeId, Layer)> = {
        let roles = app
            .components::<Role>()
            .expect("no Role writer is alive across a Tick");
        let surfaces = app
            .components::<Surface>()
            .expect("no Surface holder is alive across a Tick");
        let layers = app
            .components::<LayerSurface>()
            .expect("no LayerSurface holder is alive across a Tick");
        windows(app)
            .into_iter()
            .filter_map(|w| match roles.get(w) {
                Some(Role::Layer(layer))
                    if surfaces.get(w).is_some_and(|s| s.proxy.is_some())
                        && layers.get(w).is_some_and(|l| l.objects.is_none()) =>
                {
                    Some((w, layer.clone()))
                }
                _ => None,
            })
            .collect()
    };
    for (window, layer) in missing {
        create(app, window, &layer);
    }
}

fn on_event(app: &mut App, ev: &ZwlrLayerSurfaceV1Event) {
    match ev {
        ZwlrLayerSurfaceV1Event::Configure {
            sender,
            serial,
            width,
            height,
        } => {
            let Some(window) = app
                .resource::<WindowOf>()
                .expect("no WindowOf writer is alive across a configure")
                .get(sender)
            else {
                return;
            };
            sender.ack_configure(*serial);
            configured(app, window, *width, *height);
        }
        // The compositor's close is a remove; the window module does the
        // rest.
        ZwlrLayerSurfaceV1Event::Closed { sender } => {
            let window = app
                .resource::<WindowOf>()
                .expect("no WindowOf writer is alive across a close")
                .get(sender);
            if let Some(window) = window {
                let _ = app.remove(window);
            }
        }
    }
}

fn create(app: &mut App, window: NodeId, layer: &Layer) {
    let shell = app
        .resource::<Globals>()
        .expect("no Globals writer is alive across a Tick")
        .require::<ZwlrLayerShellV1>()
        .clone();
    let surface = app
        .components::<Surface>()
        .expect("no Surface holder is alive across a Tick")
        .get(window)
        .expect("checked by the caller")
        .proxy()
        .clone();
    let layer_surface =
        shell.get_layer_surface(&surface, None, kind(layer.layer), &layer.namespace);

    let (width, height) = app
        .components::<LayoutStyle>()
        .expect("no LayoutStyle writer is alive across a Tick")
        .get(window)
        .map_or((0, 0), |s| (fixed(s.width), fixed(s.height)));
    layer_surface.set_size(width, height);
    layer_surface.set_anchor(anchor(layer.anchor));
    layer_surface.set_exclusive_zone(layer.exclusive_zone);
    layer_surface.set_margin(
        layer.margin.top,
        layer.margin.right,
        layer.margin.bottom,
        layer.margin.left,
    );
    layer_surface.set_keyboard_interactivity(keyboard(layer.keyboard));
    surface.commit();

    app.resource_mut::<WindowOf>()
        .expect("no WindowOf holder is alive across a Tick")
        .insert(&layer_surface, window);
    if let Some(l) = app
        .components_mut::<LayerSurface>()
        .expect("no LayerSurface holder is alive across a Tick")
        .get_mut(window)
    {
        l.objects = Some(Objects {
            surface,
            layer_surface,
        });
    }
}

/// A `px` dimension is the size we want; anything else is the compositor's
/// to fill, which the protocol spells as zero.
fn fixed(v: Val) -> u32 {
    match v {
        Val::Px(n) => n.round().max(0.0) as u32,
        _ => 0,
    }
}

fn kind(k: LayerKind) -> ZwlrLayerShellV1Layer {
    match k {
        LayerKind::Background => ZwlrLayerShellV1Layer::Background,
        LayerKind::Bottom => ZwlrLayerShellV1Layer::Bottom,
        LayerKind::Top => ZwlrLayerShellV1Layer::Top,
        LayerKind::Overlay => ZwlrLayerShellV1Layer::Overlay,
    }
}

fn anchor(a: Anchor) -> ZwlrLayerSurfaceV1Anchor {
    ZwlrLayerSurfaceV1Anchor::from_bits_truncate(a.bits())
}

fn keyboard(k: Keyboard) -> ZwlrLayerSurfaceV1KeyboardInteractivity {
    match k {
        Keyboard::None => ZwlrLayerSurfaceV1KeyboardInteractivity::None,
        Keyboard::Exclusive => ZwlrLayerSurfaceV1KeyboardInteractivity::Exclusive,
        Keyboard::OnDemand => ZwlrLayerSurfaceV1KeyboardInteractivity::OnDemand,
    }
}
