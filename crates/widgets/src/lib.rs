//! The widgets: what a spawn site names when it wants something ordinary.
//! [`Div`] is a box with a look, [`Text`] a run of glyphs, [`Icon`] one
//! sprite. Each is complete when its components are: its box is the dense
//! `LayoutStyle`, its look the `Paint` its builder writes, and for the two
//! that have content, its size the `Measure` derived from it.
//!
//! A widget is changed by an event emitted at its node — [`SetLayout`],
//! [`SetQuad`], [`SetText`], [`SetFont`], [`SetColor`], [`SetSprite`] —
//! handled by the handler its builder registered, which rewrites the
//! widget's components. Nothing outside a handler writes another node's
//! components, so restyling a sibling is `emit`, never a reach.

use app::{Context, Handle, Spawner, Widget, WidgetBuild};
use assets::{AtlasId, BakedFont, SpriteRegion};
use layout::{Align, Display, Justify, LayoutStyle, Measure, Val, px};
use paint::{MonochromeSprite, Paint, Quad};
use utils::{Color, Edges, Point, Size};

pub mod prelude {
    pub use crate::{
        Div, DivBuilder, Icon, IconBuilder, SetColor, SetFont, SetLayout, SetQuad, SetSprite,
        SetText, Sprite, Text, TextBuilder, div, icon, text,
    };
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Replace a node's whole `LayoutStyle`. Handled by [`Div`].
#[derive(Debug, Clone, PartialEq)]
pub struct SetLayout(pub LayoutStyle);
impl app::Event for SetLayout {}

/// Replace a div's quad. The style's border edges follow the quad's border
/// width, as they do at spawn, because a drawn border takes layout space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetQuad(pub Quad);
impl app::Event for SetQuad {}

/// Replace a text's string. Handled by [`Text`]; its measure and paint
/// follow.
#[derive(Debug, Clone, PartialEq)]
pub struct SetText(pub String);
impl app::Event for SetText {}

/// Replace a text's font. Handled by [`Text`]; its measure and paint follow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetFont(pub &'static BakedFont);
impl app::Event for SetFont {}

/// Replace the tint. Handled by [`Text`] and [`Icon`]; the paint follows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetColor(pub Color);
impl app::Event for SetColor {}

/// Replace an icon's sprite. Handled by [`Icon`]; its measure and paint
/// follow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetSprite(pub Sprite);
impl app::Event for SetSprite {}

// ---------------------------------------------------------------------------
// Div
// ---------------------------------------------------------------------------

/// A box with a look. It has nothing of its own: its box is the dense
/// `LayoutStyle`, its look is the `Paint` its builder writes, and its
/// children are whoever spawns them. Complete because its components are.
pub struct Div;
impl Widget for Div {}

/// A div with the default style and a quad that draws nothing.
pub fn div() -> DivBuilder {
    DivBuilder {
        layout: LayoutStyle::default(),
        quad: Quad::default(),
    }
}

/// Carries a whole `LayoutStyle` and a whole `Quad`, and mirrors every verb
/// of both, each forwarding to one of them. `spawn` writes the whole style,
/// so a div's style is the builder's and nobody else's.
pub struct DivBuilder {
    layout: LayoutStyle,
    quad: Quad,
}

impl WidgetBuild for DivBuilder {
    type Widget = Div;
    fn spawn(self, me: Handle<Div>, s: &mut Spawner<Div>) -> Div {
        s.set_component(self.layout);
        s.set_component(Paint::Quad(self.quad));
        s.on(me, on_set_layout);
        s.on(me, on_set_quad);
        Div
    }
}

fn on_set_layout(ctx: &mut Context<Div>, e: &SetLayout) {
    ctx.set_component(e.0.clone());
}

fn on_set_quad(ctx: &mut Context<Div>, e: &SetQuad) {
    let width = e.0.border.map_or(0.0, |b| b.width);
    if let Some(style) = ctx.component::<LayoutStyle>().map(|s| (*s).clone()) {
        ctx.set_component(style.border(Edges::all(px(width))));
    }
    ctx.set_component(Paint::Quad(e.0));
}

/// Whole values, for a caller that built one elsewhere.
impl DivBuilder {
    pub fn layout(mut self, l: LayoutStyle) -> Self {
        self.layout = l;
        self
    }
    pub fn paint(mut self, q: Quad) -> Self {
        self.quad = q;
        self
    }
}

/// The `Quad` verbs. `border` is the one verb both types own: a drawn border
/// takes layout space, so it writes the quad's border and the style's border
/// edges together.
impl DivBuilder {
    pub fn color(mut self, c: Color) -> Self {
        self.quad = self.quad.color(c);
        self
    }
    pub fn radius(mut self, r: f32) -> Self {
        self.quad = self.quad.radius(r);
        self
    }
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.quad = self.quad.border(width, color);
        self.layout = self.layout.border(Edges::all(px(width)));
        self
    }
}

/// The `LayoutStyle` verbs, one to one, in the order of the originals so a
/// missing one is easy to spot. `border` is above; `fill` is layout's
/// "the whole of the parent", and a colour is `color`.
macro_rules! mirror {
    ($builder:ident: $( $name:ident ( $( $arg:ident : $ty:ty ),* ) ),* $(,)?) => {
        impl $builder {
            $(
                pub fn $name(mut self $(, $arg: $ty)*) -> Self {
                    self.layout = self.layout.$name($($arg),*);
                    self
                }
            )*
        }
    };
}

mirror! { DivBuilder:
    display(d: Display), flex(), block(), hidden(),
    row(), column(),
    absolute(), relative(),
    justify(j: Justify), align_content(j: Justify), align_items(a: Align), align_self(a: Align), center(),
    wrap(),
    width(v: Val), height(v: Val), size(w: Val, h: Val), fill(),
    min_width(v: Val), min_height(v: Val), max_width(v: Val), max_height(v: Val),
    flex_basis(v: Val), grow(g: f32), shrink(s: f32),
    gap(v: Val), row_gap(v: Val), column_gap(v: Val),
    padding(e: Edges<Val>), padding_all(v: Val), margin(e: Edges<Val>), inset(e: Edges<Val>),
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

/// A run of glyphs in one font and one colour. Its `Measure` is the run's
/// advance by the font's line height, and its `Paint` one sprite per
/// visible glyph, placed from the font's metrics with the baseline at the
/// font's ascent. Without a font it measures nothing and draws nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub font: Option<&'static BakedFont>,
    pub text: String,
    pub color: Color,
}
impl Widget for Text {}

impl Text {
    /// The run's advance by the font's line height; `None` without a font.
    pub fn measure(&self) -> Measure {
        Measure(
            self.font
                .map(|font| Size::new(font.measure_width(&self.text), font.line_height)),
        )
    }

    /// One sprite per glyph with a bitmap, pen-advanced from the content
    /// box's left edge, baseline at the font's ascent. Characters outside
    /// the font's printable ASCII range are skipped, as the font has no
    /// glyph for them.
    pub fn paint(&self) -> Paint {
        let Some(font) = self.font else {
            return Paint::None;
        };
        let mut pen = 0.0;
        let mut run = Vec::new();
        for ch in self.text.chars() {
            let code = ch as u32;
            if !(32..=126).contains(&code) {
                continue;
            }
            let glyph = &font.glyphs[(code - 32) as usize];
            if glyph.w > 0.0 && glyph.h > 0.0 {
                run.push(MonochromeSprite {
                    atlas: font.atlas_id,
                    region: SpriteRegion {
                        x: glyph.x,
                        y: glyph.y,
                        w: glyph.w,
                        h: glyph.h,
                    },
                    offset: Point::new(
                        pen + glyph.bearing_x,
                        font.ascent - glyph.bearing_y - glyph.h,
                    ),
                    size: Size::new(glyph.w, glyph.h),
                    color: self.color,
                });
            }
            pen += glyph.advance;
        }
        Paint::Sprites(run)
    }
}

/// A white text with no font and the default style.
pub fn text(text: impl Into<String>) -> TextBuilder {
    TextBuilder {
        text: Text {
            font: None,
            text: text.into(),
            color: Color::WHITE,
        },
        layout: LayoutStyle::default(),
    }
}

pub struct TextBuilder {
    text: Text,
    layout: LayoutStyle,
}

impl TextBuilder {
    pub fn font(mut self, font: &'static BakedFont) -> Self {
        self.text.font = Some(font);
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.text.color = c;
        self
    }
    /// The whole style, for a caller that built one elsewhere. A text is a
    /// leaf sized by its measure, so the default is usually right.
    pub fn layout(mut self, l: LayoutStyle) -> Self {
        self.layout = l;
        self
    }
}

impl WidgetBuild for TextBuilder {
    type Widget = Text;
    fn spawn(self, me: Handle<Text>, s: &mut Spawner<Text>) -> Text {
        s.set_component(self.layout);
        s.set_component(self.text.measure());
        s.set_component(self.text.paint());
        s.on(me, |ctx: &mut Context<Text>, e: &SetText| {
            ctx.me().text = e.0.clone();
            sync_text(ctx);
        });
        s.on(me, |ctx: &mut Context<Text>, e: &SetFont| {
            ctx.me().font = Some(e.0);
            sync_text(ctx);
        });
        s.on(me, |ctx: &mut Context<Text>, e: &SetColor| {
            ctx.me().color = e.0;
            sync_text(ctx);
        });
        self.text
    }
}

/// Rewrites the measure and paint from the widget's state.
fn sync_text(ctx: &mut Context<Text>) {
    let measure = ctx.me().measure();
    let paint = ctx.me().paint();
    ctx.set_component(measure);
    ctx.set_component(paint);
}

// ---------------------------------------------------------------------------
// Icon
// ---------------------------------------------------------------------------

/// A single monochrome sprite in an atlas: which atlas, and the region
/// within it. The `Icon` analogue of `Text`'s font.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    pub atlas: AtlasId,
    pub region: SpriteRegion,
}

/// A monochrome sprite, tinted by `color`, natural-sized from its atlas
/// region: its `Measure` is the region's size and its `Paint` one sprite at
/// the content box's origin. Without a sprite it measures nothing and draws
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Icon {
    pub sprite: Option<Sprite>,
    pub color: Color,
}
impl Widget for Icon {}

impl Icon {
    /// The region's size; `None` without a sprite.
    pub fn measure(&self) -> Measure {
        Measure(self.sprite.map(|s| Size::new(s.region.w, s.region.h)))
    }

    /// One sprite at the content box's origin, at the region's size.
    pub fn paint(&self) -> Paint {
        match self.sprite {
            None => Paint::None,
            Some(s) => Paint::Sprites(vec![MonochromeSprite {
                atlas: s.atlas,
                region: s.region,
                offset: Point::ZERO,
                size: Size::new(s.region.w, s.region.h),
                color: self.color,
            }]),
        }
    }
}

/// A white icon with no sprite and the default style.
pub fn icon() -> IconBuilder {
    IconBuilder {
        icon: Icon {
            sprite: None,
            color: Color::WHITE,
        },
        layout: LayoutStyle::default(),
    }
}

pub struct IconBuilder {
    icon: Icon,
    layout: LayoutStyle,
}

impl IconBuilder {
    pub fn sprite(mut self, sprite: Sprite) -> Self {
        self.icon.sprite = Some(sprite);
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.icon.color = c;
        self
    }
    /// The whole style, for a caller that built one elsewhere.
    pub fn layout(mut self, l: LayoutStyle) -> Self {
        self.layout = l;
        self
    }
}

impl WidgetBuild for IconBuilder {
    type Widget = Icon;
    fn spawn(self, me: Handle<Icon>, s: &mut Spawner<Icon>) -> Icon {
        s.set_component(self.layout);
        s.set_component(self.icon.measure());
        s.set_component(self.icon.paint());
        s.on(me, |ctx: &mut Context<Icon>, e: &SetSprite| {
            ctx.me().sprite = Some(e.0);
            sync_icon(ctx);
        });
        s.on(me, |ctx: &mut Context<Icon>, e: &SetColor| {
            ctx.me().color = e.0;
            sync_icon(ctx);
        });
        self.icon
    }
}

/// Rewrites the measure and paint from the widget's state.
fn sync_icon(ctx: &mut Context<Icon>) {
    let measure = ctx.me().measure();
    let paint = ctx.me().paint();
    ctx.set_component(measure);
    ctx.set_component(paint);
}
