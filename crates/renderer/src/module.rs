//! The render module: answers `Frame` by drawing a window's subtree into a
//! DMA-BUF and attaching it to the window's `wl_surface`. Presentation
//! commits right after, so this module never commits and never asks.
//!
//! Every frame is a full redraw: the whole subtree is walked, the whole
//! buffer cleared and repainted, the whole surface damaged. Each window has
//! two buffers, alternated; a buffer the compositor has not released yet is
//! not drawn into: the frame attaches nothing, so presentation commits
//! nothing, and the release re-raises the request, which is answered at
//! once.
//!
//! The walk reads two columns and nothing else: a node's `Layout` says
//! where, its `Paint` says how. A `Quad` fills its box; a run of sprites is
//! placed from the box's content corner. Depth in the tree is the z order,
//! and the solid colour composited behind each node is carried down so text
//! over a known background takes the opaque path.

use std::collections::HashMap;

use app::{App, Module, NodeId, Resource, Tick};
use assets::AtlasData;
use layout::Layout;
use paint::Paint;
use presentation::Surface;
use utils::{Color, Point, Rect};
use wayland::{
    Globals, ObjectId, Proxy, WlBuffer, WlBufferEvent, ZwpLinuxBufferParamsV1Flags,
    ZwpLinuxDmabufV1,
};
use window::{Frame, RequestFrame, Window};

use crate::{
    DmaBuf, RenderableSurface, Renderer,
    commands::{ClearColor, DrawMonochromeSprite, DrawQuad},
};

const DRM_FORMAT_ARGB8888: u32 = 0x3432_5241;

/// One z step per tree level: a child is always strictly above its parent,
/// and the quad interior's own epsilon stays far below this.
const Z_STEP: f32 = 1e-3;

/// One of a window's two buffers: the GL target and the `wl_buffer` the
/// compositor knows it by.
struct Slot {
    surface: RenderableSurface<DmaBuf>,
    buffer: Proxy<WlBuffer>,
    released: bool,
}

/// A window's buffers, at the size they were allocated for.
struct Slots {
    slots: [Slot; 2],
    back: usize,
    size: (u32, u32),
    /// A `Frame` found the back buffer still held by the compositor and was
    /// skipped; the release asks again.
    starved: bool,
}

/// The renderer and every window's buffers, one resource so a frame reaches
/// both at once.
pub struct Render {
    renderer: Renderer,
    pipelines_ready: bool,
    windows: Vec<(NodeId, Slots)>,
    by_buffer: HashMap<ObjectId, (NodeId, usize)>,
}
impl Resource for Render {}

impl Render {
    /// The renderer itself, for uploading an atlas after install.
    pub fn renderer_mut(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    fn slots_mut(&mut self, window: NodeId) -> Option<&mut Slots> {
        self.windows
            .iter_mut()
            .find(|(w, _)| *w == window)
            .map(|(_, s)| s)
    }

    fn alloc(&mut self, dmabuf: &Proxy<ZwpLinuxDmabufV1>, width: u32, height: u32) -> Slots {
        let slots = std::array::from_fn(|_| {
            let surface = self
                .renderer
                .create_surface::<DmaBuf>(width, height)
                .expect("DmaBuf surface allocation failed");
            let params = dmabuf.create_params();
            let modifier = surface.backend.modifier;
            params.add(
                std::os::fd::AsFd::as_fd(&surface.backend.prime_fd),
                0,
                0,
                surface.backend.stride,
                (modifier >> 32) as u32,
                (modifier & 0xffff_ffff) as u32,
            );
            let buffer = params.create_immed(
                width as i32,
                height as i32,
                DRM_FORMAT_ARGB8888,
                ZwpLinuxBufferParamsV1Flags::empty(),
            );
            params.destroy();
            Slot {
                surface,
                buffer,
                released: true,
            }
        });
        Slots {
            slots,
            back: 0,
            size: (width, height),
            starved: false,
        }
    }

    fn free(&mut self, slots: Slots) {
        for slot in slots.slots {
            if let Some(id) = slot.buffer.object_id() {
                self.by_buffer.remove(&id);
            }
            if slot.buffer.is_alive() {
                slot.buffer.destroy();
            }
            self.renderer.destroy_surface(slot.surface);
        }
    }

    /// Drop the buffers of every window that is gone.
    fn sweep(&mut self, app: &App) {
        let mut i = 0;
        while i < self.windows.len() {
            if app.widget::<Window>(self.windows[i].0).is_some() {
                i += 1;
            } else {
                let (_, slots) = self.windows.swap_remove(i);
                self.free(slots);
            }
        }
    }
}

/// One thing the walk found to draw, in window coordinates.
enum Item {
    Quad(DrawQuad),
    Sprite {
        atlas: assets::AtlasId,
        region: Rect,
        origin: Point,
        z: f32,
        size: utils::Size,
        color: Color,
        background: Color,
    },
}

/// Everything a window's subtree draws, in preorder, origins relative to
/// the window, z from depth, and the composited background each item sits
/// on.
fn collect(app: &App, window: NodeId) -> Vec<Item> {
    let layouts = app
        .components::<Layout>()
        .expect("no Layout writer is alive across a Frame");
    let paints = app
        .components::<Paint>()
        .expect("no Paint writer is alive across a Frame");
    let Some(origin) = layouts.get(window).map(|l| l.rect.origin) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    // Preorder with an explicit stack: (node, depth, background).
    let mut stack = vec![(window, 0u32, Color::TRANSPARENT)];
    while let Some((id, depth, background)) = stack.pop() {
        let (Some(layout), Some(paint)) = (layouts.get(id), paints.get(id)) else {
            continue;
        };
        let rect = layout.rect;
        let z = depth as f32 * Z_STEP;
        let mut next_background = background;
        match paint {
            Paint::None => {}
            Paint::Quad(q) => {
                if !q.is_invisible() {
                    out.push(Item::Quad(DrawQuad {
                        color: q.color,
                        border_color: q.border.map_or(Color::TRANSPARENT, |b| b.color),
                        origin: Point::new(rect.x() - origin.x(), rect.y() - origin.y()),
                        z,
                        size: rect.size,
                        border_radius: q.radius,
                        border_thickness: q.border.map_or(0.0, |b| b.width),
                        background,
                        is_opaque: true,
                    }));
                }
                next_background = q.color.over(background);
            }
            Paint::Sprites(run) => {
                let content = layout.content();
                for s in run {
                    if s.is_invisible() {
                        continue;
                    }
                    out.push(Item::Sprite {
                        atlas: s.atlas,
                        region: Rect::xywh(s.region.x, s.region.y, s.region.w, s.region.h),
                        origin: Point::new(
                            content.x() + s.offset.x() - origin.x(),
                            content.y() + s.offset.y() - origin.y(),
                        ),
                        z,
                        size: s.size,
                        color: s.color,
                        background,
                    });
                }
            }
        }
        // Reversed, so the first child is popped first and preorder holds.
        // A nested window is presented on its own surface, not in ours.
        for &child in app.children(id).unwrap_or(&[]).iter().rev() {
            if app.widget::<Window>(child).is_some() {
                continue;
            }
            stack.push((child, depth + 1, next_background));
        }
    }
    out
}

/// Creates the renderer at install and answers every `Frame`. The atlases
/// given are uploaded at install; a sprite from an atlas that was never
/// uploaded panics naming it. Added after `PresentationModule`.
#[derive(Default)]
pub struct RenderModule {
    atlases: Vec<&'static AtlasData>,
}

impl RenderModule {
    pub fn new() -> Self {
        Self::default()
    }

    /// An atlas the windows draw from: a font's glyphs, some icons.
    pub fn atlas(mut self, atlas: &'static AtlasData) -> Self {
        self.atlases.push(atlas);
        self
    }
}

impl Module for RenderModule {
    fn install(self, app: &mut App) {
        let mut renderer = Renderer::new().expect("renderer");
        for atlas in self.atlases {
            renderer
                .upload_atlas(atlas)
                .unwrap_or_else(|e| panic!("cannot upload atlas {:?}: {e}", atlas.id));
        }
        app.insert_resource(Render {
            renderer,
            pipelines_ready: false,
            windows: Vec::new(),
            by_buffer: HashMap::new(),
        })
        .system(on_frame)
        .system(on_release)
        .system(on_tick);
    }
}

fn on_frame(app: &mut App, Frame(window): &Frame) {
    let window = *window;
    let Some(rect) = app
        .components::<Layout>()
        .expect("no Layout writer is alive across a Frame")
        .get(window)
        .map(|l| l.rect)
    else {
        return;
    };
    let (width, height) = (rect.width().round() as u32, rect.height().round() as u32);
    if width == 0 || height == 0 {
        return;
    }
    let items = collect(app, window);
    let dmabuf = app
        .resource::<Globals>()
        .expect("no Globals writer is alive across a Frame")
        .require::<ZwpLinuxDmabufV1>()
        .clone();
    let Some(surface) = app
        .components::<Surface>()
        .expect("no Surface holder is alive across a Frame")
        .get(window)
        .map(|s| s.proxy().clone())
    else {
        return;
    };

    let attached = {
        let mut render = app
            .resource_mut::<Render>()
            .expect("no Render holder is alive across a Frame");
        if render
            .slots_mut(window)
            .is_some_and(|s| s.size != (width, height))
        {
            let i = render
                .windows
                .iter()
                .position(|(w, _)| *w == window)
                .expect("found above");
            let (_, old) = render.windows.swap_remove(i);
            render.free(old);
        }
        if render.slots_mut(window).is_none() {
            let slots = render.alloc(&dmabuf, width, height);
            for (i, slot) in slots.slots.iter().enumerate() {
                if let Some(id) = slot.buffer.object_id() {
                    render.by_buffer.insert(id, (window, i));
                }
            }
            render.windows.push((window, slots));
        }

        let Render {
            renderer,
            pipelines_ready,
            windows,
            ..
        } = &mut *render;
        let slots = &mut windows
            .iter_mut()
            .find(|(w, _)| *w == window)
            .expect("allocated above")
            .1;
        let back = slots.back;
        if !slots.slots[back].released {
            slots.starved = true;
            false
        } else {
            renderer.active_surface(&slots.slots[back].surface);
            if !*pipelines_ready {
                renderer.init_pipelines();
                *pipelines_ready = true;
            }
            renderer.set_scissor(None);
            renderer.send_command(ClearColor(Color::TRANSPARENT));
            for item in items {
                match item {
                    Item::Quad(q) => renderer.send_command(q),
                    Item::Sprite {
                        atlas,
                        region,
                        origin,
                        z,
                        size,
                        color,
                        background,
                    } => {
                        let texture_id = renderer.texture_id(atlas).unwrap_or_else(|| {
                            panic!(
                                "atlas {atlas:?} was never uploaded: give it to RenderModule::atlas"
                            )
                        });
                        renderer.send_command(DrawMonochromeSprite {
                            texture_id,
                            region,
                            origin,
                            z,
                            size,
                            color,
                            background,
                            is_opaque: true,
                        });
                    }
                }
            }
            renderer.render_frame();
            renderer.finish();

            surface.attach(Some(&slots.slots[back].buffer), 0, 0);
            surface.damage(0, 0, width as i32, height as i32);
            slots.slots[back].released = false;
            slots.starved = false;
            slots.back ^= 1;
            true
        }
    };

    if attached
        && let Some(s) = app
            .components_mut::<Surface>()
            .expect("no Surface holder is alive across a Frame")
            .get_mut(window)
    {
        s.attached = true;
    }
}

/// The compositor gave a buffer back. A window that was skipped for want
/// of this buffer asks again.
fn on_release(app: &mut App, ev: &WlBufferEvent) {
    let WlBufferEvent::Release { sender } = ev;
    let starved = {
        let mut render = app
            .resource_mut::<Render>()
            .expect("no Render holder is alive across a release");
        let Some((window, i)) = sender
            .object_id()
            .and_then(|id| render.by_buffer.get(&id))
            .copied()
        else {
            return;
        };
        let Some(slots) = render.slots_mut(window) else {
            return;
        };
        slots.slots[i].released = true;
        if slots.starved && slots.back == i {
            slots.starved = false;
            Some(window)
        } else {
            None
        }
    };
    if let Some(window) = starved {
        app.signal(RequestFrame(window));
    }
}

fn on_tick(app: &mut App, _: &Tick) {
    let mut render = app
        .resource_mut::<Render>()
        .expect("no Render holder is alive across a Tick");
    render.sweep(app);
}
