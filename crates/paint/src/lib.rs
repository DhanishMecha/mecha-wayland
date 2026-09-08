//! The paint module: what a node looks like, held as one [`Paint`] per node.
//!
//! This crate is to a renderer what `window` is to a compositor: the
//! headless abstraction of what to draw, with no connection to anything that
//! draws it. A `Paint` says *how* a node looks and never *where*; the where
//! is the node's `Layout`, and a renderer reads the two together. It is
//! resolved, not descriptive: nothing interprets it beyond placing it.
//!
//! The set of primitives is closed: a [`Quad`] that fills the node's box, or
//! a run of [`MonochromeSprite`]s placed inside it. A glyph and an icon are
//! the same thing to a renderer, so text is a run of many and an icon a run
//! of one.
//!
//! The core has no change events, so the module keeps a private copy of
//! every node's paint and compares on `Tick`; the nodes that differ are
//! signalled as one [`PaintChanged`]. Who asks for a frame on that is the
//! window module's business.

use app::{App, Component, Module, NodeId, Signal, Tick};
use assets::{AtlasId, SpriteRegion};
use utils::{Color, Point, Size};

pub mod prelude {
    pub use crate::{Border, MonochromeSprite, Paint, PaintChanged, PaintModule, Quad};
    pub use utils::Color;
}

/// The one primitive a node draws, or nothing. Dense: every node has one,
/// and the default draws nothing. A node that wants two visuals is two
/// nodes.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Paint {
    #[default]
    None,
    Quad(Quad),
    /// Tinted atlas regions placed inside the node's content box: a text
    /// node's glyphs, or an icon's one sprite. An empty run draws nothing.
    Sprites(Vec<MonochromeSprite>),
}
impl Component for Paint {}

/// A filled box: a colour, a corner radius and an optional border. It
/// always fills the node's `Layout` rect, so it carries no position or size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
    pub color: Color,
    pub radius: f32,
    pub border: Option<Border>,
}

/// The line drawn along a quad's edge, inside its rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub width: f32,
    pub color: Color,
}

impl Default for Quad {
    /// A quad that draws nothing: transparent, square, unbordered. What a
    /// positioning-only container carries.
    fn default() -> Self {
        Self::new(Color::TRANSPARENT)
    }
}

/// The verbs, by value, so a quad is written in one expression.
impl Quad {
    pub const fn new(color: Color) -> Self {
        Self {
            color,
            radius: 0.0,
            border: None,
        }
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border = Some(Border { width, color });
        self
    }
    /// True when a renderer would leave no pixel behind: no visible fill
    /// and no visible border.
    pub fn is_invisible(&self) -> bool {
        self.color.a <= 0.0
            && self
                .border
                .is_none_or(|b| b.width <= 0.0 || b.color.a <= 0.0)
    }
}

/// A region of an atlas, tinted, at a place inside the node. The region is
/// in atlas pixels; `offset` and `size` are logical pixels, the offset from
/// the top-left of the node's content box (`Layout::content`). Resolved by
/// whoever writes it, so a renderer places it and does nothing else.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonochromeSprite {
    pub atlas: AtlasId,
    pub region: SpriteRegion,
    pub offset: Point,
    pub size: Size,
    pub color: Color,
}

impl MonochromeSprite {
    /// True when a renderer would leave no pixel behind: a fully transparent
    /// tint or an empty box.
    pub fn is_invisible(&self) -> bool {
        self.color.a <= 0.0 || self.size.width() <= 0.0 || self.size.height() <= 0.0
    }
}

impl Paint {
    /// True when a renderer would leave no pixel behind.
    pub fn is_invisible(&self) -> bool {
        match self {
            Paint::None => true,
            Paint::Quad(q) => q.is_invisible(),
            Paint::Sprites(run) => run.iter().all(MonochromeSprite::is_invisible),
        }
    }
}

/// The nodes whose `Paint` differs from the last tick, in slot order.
/// Signalled once per `Tick` on which any differs, and never empty.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintChanged(pub Vec<NodeId>);
impl Signal for PaintChanged {}

/// What the last tick saw of a node's paint.
#[derive(Default)]
struct Seen(Paint);
impl Component for Seen {}

/// Registers [`Paint`] and the system on `Tick` that signals
/// [`PaintChanged`]. Who asks for a frame on that is the window module's
/// business.
pub struct PaintModule;

impl Module for PaintModule {
    fn install(self, app: &mut App) {
        app.register_component::<Paint>()
            .register_component::<Seen>()
            .system(on_tick);
    }
}

fn on_tick(app: &mut App, _: &Tick) {
    let changed = {
        let paints = app
            .components::<Paint>()
            .expect("no Paint writer is alive across a Tick");
        let mut seen = app
            .components_mut::<Seen>()
            .expect("paint's private column is never lent out");
        let mut changed = Vec::new();
        for (id, paint) in paints.iter() {
            let Some(seen) = seen.get_mut(id) else {
                continue;
            };
            if seen.0 != *paint {
                seen.0 = paint.clone();
                changed.push(id);
            }
        }
        changed
    };
    if !changed.is_empty() {
        app.signal(PaintChanged(changed));
    }
}
