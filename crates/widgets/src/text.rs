//! `Text`: one line of glyphs, shaped once at build and re-shaped by its
//! `Context` setters. Single-line only.

use app::{Build, Context, Handle, Spawner, Widget};
use atlas::{Atlas, FontId};
use geometry::{Color, Point, Size};
use layout::{LayoutStyle, Measure};
use paint::{MonochromeSprite, Paint, PaintContext};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalTrim {
    #[default]
    Normal,
    CapHeight,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextOverflow {
    #[default]
    Clip,
    Ellipsis,
    Visible,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    Strikethrough,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextWrap {
    #[default]
    NoWrap,
    Word,
    BreakAll, // or Char
}

pub struct Text {
    font: FontId,
    px: u16,
    color: Color,
    string: String,
    line_height: Option<f32>,
    letter_spacing: Option<f32>,
    vertical_trim: VerticalTrim,
    align: TextAlign,
    overflow: TextOverflow,
    decoration: TextDecoration,
    wrap: TextWrap,
    max_lines: Option<usize>,
}

impl Text {
    pub fn text(&self) -> &str {
        &self.string
    }
    pub fn font(&self) -> FontId {
        self.font
    }
    pub fn size(&self) -> u16 {
        self.px
    }
    pub fn color(&self) -> Color {
        self.color
    }
    pub fn line_height(&self) -> Option<f32> {
        self.line_height
    }
    pub fn letter_spacing(&self) -> Option<f32> {
        self.letter_spacing
    }
    pub fn vertical_trim(&self) -> VerticalTrim {
        self.vertical_trim
    }
    pub fn align(&self) -> TextAlign {
        self.align
    }
    pub fn overflow(&self) -> TextOverflow {
        self.overflow
    }
    pub fn decoration(&self) -> TextDecoration {
        self.decoration
    }
    pub fn wrap(&self) -> TextWrap {
        self.wrap
    }
    pub fn max_lines(&self) -> Option<usize> {
        self.max_lines
    }
}

pub fn text(font: FontId, s: impl Into<String>) -> TextBuilder {
    TextBuilder {
        font,
        string: s.into(),
        style: LayoutStyle::default(),
        px: 16,
        color: Color::WHITE,
        line_height: None,
        letter_spacing: None,
        vertical_trim: VerticalTrim::Normal,
        align: TextAlign::Left,
        overflow: TextOverflow::Clip,
        decoration: TextDecoration::None,
        wrap: TextWrap::NoWrap,
        max_lines: None,
    }
}

pub struct TextBuilder {
    pub(crate) font: FontId,
    pub(crate) string: String,
    pub(crate) style: LayoutStyle,
    pub(crate) px: u16,
    pub(crate) color: Color,
    pub(crate) line_height: Option<f32>,
    pub(crate) letter_spacing: Option<f32>,
    pub(crate) vertical_trim: VerticalTrim,
    pub(crate) align: TextAlign,
    pub(crate) overflow: TextOverflow,
    pub(crate) decoration: TextDecoration,
    pub(crate) wrap: TextWrap,
    pub(crate) max_lines: Option<usize>,
}

impl TextBuilder {
    pub fn font(mut self, font: FontId) -> Self {
        self.font = font;
        self
    }
    pub fn style(mut self, style: LayoutStyle) -> Self {
        self.style = style;
        self
    }
    pub fn size(mut self, px: u16) -> Self {
        self.px = px;
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
    pub fn line_height(mut self, height: f32) -> Self {
        self.line_height = Some(height);
        self
    }
    pub fn letter_spacing(mut self, spacing: f32) -> Self {
        self.letter_spacing = Some(spacing);
        self
    }
    pub fn vertical_trim(mut self, trim: VerticalTrim) -> Self {
        self.vertical_trim = trim;
        self
    }
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }
    pub fn decoration(mut self, decoration: TextDecoration) -> Self {
        self.decoration = decoration;
        self
    }
    pub fn wrap(mut self, wrap: TextWrap) -> Self {
        self.wrap = wrap;
        self
    }
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = Some(max_lines);
        self
    }
}

impl Build for TextBuilder {
    type Widget = Text;
}

impl Widget for Text {
    type Builder = TextBuilder;
    fn build(b: TextBuilder, me: Handle<Text>, s: &mut Spawner<'_, Text>) -> Text {
        *s.component_mut::<LayoutStyle>(me).unwrap() = b.style;
        let (sprites, size) = {
            let mut atlas = s.resource_mut::<Atlas>();
            shape(&mut atlas, b.font, b.px, &b.string, b.color)
        };
        *s.component_mut::<Paint>(me).unwrap() = Paint::Monochrome(sprites);
        *s.component_mut::<Measure>(me).unwrap() = Measure::fixed(size);

        Text {
            font: b.font,
            px: b.px,
            color: b.color,
            string: b.string,
            line_height: b.line_height,
            letter_spacing: b.letter_spacing,
            vertical_trim: b.vertical_trim,
            align: b.align,
            overflow: b.overflow,
            decoration: b.decoration,
            wrap: b.wrap,
            max_lines: b.max_lines,
        }
    }
}

/// One line of `text` in `font` at `px`, tinted `color`: a sprite per
/// glyph with ink, pen-advanced left to right, baselined by the font's
/// ascent, and the line's natural size (`pen` wide, `ascent - descent +
/// gap` tall). A character missing from the font's cmap is skipped
/// entirely — it contributes no width and no sprite, as if it were never
/// in the string. A character present in the cmap but with no ink (such
/// as a space) still advances the pen; only the sprite is skipped.
fn shape(
    atlas: &mut Atlas,
    font: FontId,
    px: u16,
    text: &str,
    color: Color,
) -> (Vec<MonochromeSprite>, Size) {
    let line = atlas.line(font, px);
    let baseline = line.ascent;
    let mut pen = 0.0f32;
    let mut sprites = Vec::new();
    for ch in text.chars() {
        let Some((f, id)) = atlas.lookup(&[font], ch) else {
            continue;
        };
        let g = atlas.glyph(f, id, px);
        if g.tile.bounds.width() > 0.0 {
            sprites.push(MonochromeSprite::new(
                g.tile,
                Point::new(pen + g.left, baseline - g.top),
                Size::new(g.tile.bounds.width(), g.tile.bounds.height()),
                color,
            ));
        }
        pen += g.advance;
    }
    let height = line.ascent - line.descent + line.gap;
    (sprites, Size::new(pen, height))
}

pub trait TextContext {
    fn set_text(&mut self, text: impl Into<String>);
    fn set_size(&mut self, px: u16);
    fn set_color(&mut self, color: Color);
    fn set_font(&mut self, font: FontId);
    fn set_line_height(&mut self, height: f32);
    fn set_letter_spacing(&mut self, spacing: f32);
    fn set_vertical_trim(&mut self, trim: VerticalTrim);
    fn set_align(&mut self, align: TextAlign);
    fn set_overflow(&mut self, overflow: TextOverflow);
    fn set_decoration(&mut self, decoration: TextDecoration);
    fn set_wrap(&mut self, wrap: TextWrap);
    fn set_max_lines(&mut self, max_lines: Option<usize>);
}

impl TextContext for Context<'_, Text> {
    fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.me().string == text {
            return;
        }
        let (font, px, color) = {
            let w = self.me();
            (w.font, w.px, w.color)
        };
        let (sprites, size) = {
            let mut atlas = self.resource_mut::<Atlas>();
            shape(&mut atlas, font, px, &text, color)
        };
        self.set_paint(Paint::Monochrome(sprites));
        *self.component_mut::<Measure>().unwrap() = Measure::fixed(size);
        self.me().string = text;
    }

    fn set_size(&mut self, px: u16) {
        if self.me().px == px {
            return;
        }
        let (font, color, string) = {
            let w = self.me();
            (w.font, w.color, w.string.clone())
        };
        let (sprites, size) = {
            let mut atlas = self.resource_mut::<Atlas>();
            shape(&mut atlas, font, px, &string, color)
        };
        self.set_paint(Paint::Monochrome(sprites));
        *self.component_mut::<Measure>().unwrap() = Measure::fixed(size);
        self.me().px = px;
    }

    fn set_color(&mut self, color: Color) {
        if self.me().color == color {
            return;
        }
        let mut paint = self.paint().clone();
        if let Paint::Monochrome(sprites) = &mut paint {
            for sprite in sprites.iter_mut() {
                sprite.color = color;
            }
        }
        self.set_paint(paint);
        self.me().color = color;
    }

    // Placeholder
    fn set_font(&mut self, font: FontId) {
        if self.me().font == font {
            return;
        }
        self.me().font = font;
    }

    // Placeholder
    fn set_line_height(&mut self, height: f32) {
        if self.me().line_height == Some(height) {
            return;
        }
        self.me().line_height = Some(height);
    }

    // Placeholder
    fn set_letter_spacing(&mut self, spacing: f32) {
        if self.me().letter_spacing == Some(spacing) {
            return;
        }
        self.me().letter_spacing = Some(spacing);
    }

    // Placeholder
    fn set_vertical_trim(&mut self, trim: VerticalTrim) {
        if self.me().vertical_trim == trim {
            return;
        }
        self.me().vertical_trim = trim;
    }

    // Placeholder
    fn set_align(&mut self, align: TextAlign) {
        if self.me().align == align {
            return;
        }
        self.me().align = align;
    }

    // Placeholder
    fn set_overflow(&mut self, overflow: TextOverflow) {
        if self.me().overflow == overflow {
            return;
        }
        self.me().overflow = overflow;
    }

    // Placeholder
    fn set_decoration(&mut self, decoration: TextDecoration) {
        if self.me().decoration == decoration {
            return;
        }
        self.me().decoration = decoration;
    }

    // Placeholder
    fn set_wrap(&mut self, wrap: TextWrap) {
        if self.me().wrap == wrap {
            return;
        }
        self.me().wrap = wrap;
    }

    // Placeholder
    fn set_max_lines(&mut self, max_lines: Option<usize>) {
        if self.me().max_lines == max_lines {
            return;
        }
        self.me().max_lines = max_lines;
    }
}

#[cfg(test)]
mod tests {
    use atlas::Atlas;
    use geometry::Color;

    use super::*;

    const INTER: &[u8] = include_bytes!("../../atlas/tests/fixtures/Inter-Regular.ttf");

    fn inter() -> (Atlas, FontId) {
        let mut atlas = Atlas::new();
        let font = atlas.add_font(INTER).unwrap();
        (atlas, font)
    }

    #[test]
    fn shape_places_one_sprite_per_glyph_with_ink() {
        let (mut atlas, font) = inter();
        let (sprites, size) = shape(&mut atlas, font, 14, "ab", Color::WHITE);
        assert_eq!(sprites.len(), 2);
        assert!(size.width > 0.0);
        assert!(size.height > 0.0);
        assert_eq!(sprites[0].color, Color::WHITE);
    }

    #[test]
    fn shape_skips_a_space_but_still_advances_the_pen() {
        let (mut atlas, font) = inter();
        let (with_space, wide) = shape(&mut atlas, font, 14, "a b", Color::WHITE);
        let (without_space, narrow) = shape(&mut atlas, font, 14, "ab", Color::WHITE);
        assert_eq!(with_space.len(), 2, "the space has no ink");
        assert_eq!(without_space.len(), 2);
        assert!(
            wide.width > narrow.width,
            "the space's advance still counts"
        );
    }

    #[test]
    fn shape_of_empty_text_is_a_zero_width_run_at_the_line_height() {
        let (mut atlas, font) = inter();
        let (sprites, size) = shape(&mut atlas, font, 14, "", Color::WHITE);
        assert!(sprites.is_empty());
        assert_eq!(size.width, 0.0);
        assert!(
            size.height > 0.0,
            "the line height does not depend on content"
        );
    }

    #[test]
    fn shape_skips_a_character_missing_from_the_cmap_and_contributes_no_width() {
        let (mut atlas, font) = inter();
        // U+4E2D ('中') is not in Inter-Regular's cmap.
        assert!(
            atlas.lookup(&[font], '\u{4e2d}').is_none(),
            "test assumes Inter-Regular has no glyph for U+4E2D"
        );
        let (with_missing, wider) = shape(&mut atlas, font, 14, "a\u{4e2d}b", Color::WHITE);
        let (_, narrower) = shape(&mut atlas, font, 14, "ab", Color::WHITE);
        assert_eq!(
            with_missing.len(),
            2,
            "the missing character paints no sprite"
        );
        assert_eq!(
            wider.width, narrower.width,
            "a character missing from the cmap contributes zero width, unlike a space"
        );
    }

    #[test]
    fn builder_verbs_set_the_right_fields() {
        let (_, font) = inter();
        let b = text(font, "hi")
            .size(24)
            .color(Color::BLACK)
            .line_height(32.0)
            .letter_spacing(1.5)
            .vertical_trim(VerticalTrim::CapHeight)
            .align(TextAlign::Center)
            .overflow(TextOverflow::Ellipsis)
            .decoration(TextDecoration::Underline)
            .wrap(TextWrap::Word)
            .max_lines(3);
        assert_eq!(b.string, "hi");
        assert_eq!(b.px, 24);
        assert_eq!(b.color, Color::BLACK);
        assert_eq!(b.font, font);
        assert_eq!(b.line_height, Some(32.0));
        assert_eq!(b.letter_spacing, Some(1.5));
        assert_eq!(b.vertical_trim, VerticalTrim::CapHeight);
        assert_eq!(b.align, TextAlign::Center);
        assert_eq!(b.overflow, TextOverflow::Ellipsis);
        assert_eq!(b.decoration, TextDecoration::Underline);
        assert_eq!(b.wrap, TextWrap::Word);
        assert_eq!(b.max_lines, Some(3));
    }
}
