use super::text_style::{ResolvedTextStyle, TextStyle};
use app::{Context, Handle, Spawner, Widget, WidgetBuild};
use assets::{BakedFont, SpriteRegion};
use layout::{LayoutStyle, Measure};
use paint::{MonochromeSprite, Paint};
use theme::{FontWeight, MechanixTheme, TextVariant};
use utils::{Point, Size};

use crate::{SetColor, SetFont, SetText, utils::ColorSource};

/// A run of glyphs styled by Material 3 tokens or explicit overrides.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    /// Optional font override.
    pub font: Option<&'static BakedFont>,
    /// Raw string content.
    pub text: String,
    /// Resolved visual parameters.
    pub text_style: ResolvedTextStyle,
    /// Material 3 design tokens.
    pub tokens: TextStyle,
}

impl Widget for Text {}

impl Text {
    /// Create a new [`TextBuilder`] with the specified text string.
    pub fn new(text: impl Into<String>) -> TextBuilder {
        TextBuilder::new(text)
    }

    /// Create a new [`TextBuilder`] for method-chaining configuration.
    pub fn builder(text: impl Into<String>) -> TextBuilder {
        TextBuilder::new(text)
    }

    /// Returns the effective baked font: either the explicit font or the default for the typography role.
    pub fn effective_font(&self) -> &'static BakedFont {
        self.font
            .unwrap_or_else(|| crate::fonts::default_font_for_role(self.tokens.typography_variant))
    }

    /// Calculate the run's layout advance by the font's line height.
    pub fn measure(&self) -> Measure {
        let font = self.effective_font();
        Measure(Some(Size::new(
            font.measure_width(&self.text),
            font.line_height,
        )))
    }

    /// Generate monochrome sprites for every visible glyph in the text run.
    pub fn paint(&self) -> Paint {
        let font = self.effective_font();
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
                    color: self.text_style.color,
                });
            }
            pen += glyph.advance;
        }
        Paint::Sprites(run)
    }

    /// Re-resolve style tokens for the active theme.
    pub fn apply_theme(&mut self, theme: &MechanixTheme) -> &mut Self {
        self.text_style = self.tokens.resolve(theme);
        self
    }
}

/// Create a text builder with M3 defaults (`OnSurface` color, `BodyMedium` variant).
pub fn text(text_content: impl Into<String>) -> TextBuilder {
    TextBuilder::new(text_content)
}

/// Builder for constructing a [`Text`] widget.
pub struct TextBuilder {
    text_content: String,
    font: Option<&'static BakedFont>,
    tokens: TextStyle,
    layout: LayoutStyle,
}

impl TextBuilder {
    /// Create a new builder with default M3 tokens (`BodyMedium`, `OnSurface`).
    pub fn new(text_content: impl Into<String>) -> Self {
        Self {
            text_content: text_content.into(),
            font: None,
            tokens: TextStyle::default(),
            layout: LayoutStyle::default(),
        }
    }

    /// Set an explicit baked font override.
    pub fn font(mut self, font: &'static BakedFont) -> Self {
        self.font = Some(font);
        self
    }

    /// Set the text color (accepts either a concrete [`Color`] or a [`ThemeColor`] token role).
    pub fn color(mut self, color: impl Into<ColorSource>) -> Self {
        self.tokens.color = color.into();
        self
    }

    /// Set the Material 3 typography variant role (e.g. `TitleMedium`, `LabelSmall`).
    pub fn variant(mut self, variant: TextVariant) -> Self {
        self.tokens.typography_variant = variant;
        self
    }

    /// Set an explicit font weight override.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.tokens.weight = Some(weight);
        self
    }

    /// Enable or disable the emphasised typography weight variant.
    pub fn emphasised(mut self, emphasised: bool) -> Self {
        self.tokens.is_emphasised = emphasised;
        self
    }

    /// Set an explicit line height override in pixels.
    pub fn line_height(mut self, height: f32) -> Self {
        self.tokens.line_height = Some(height);
        self
    }

    /// Set an explicit letter spacing override in pixels.
    pub fn letter_spacing(mut self, spacing: f32) -> Self {
        self.tokens.letter_spacing = Some(spacing);
        self
    }

    /// Set an explicit word spacing override in pixels.
    pub fn word_spacing(mut self, spacing: f32) -> Self {
        self.tokens.word_spacing = Some(spacing);
        self
    }

    /// Set the layout style.
    pub fn layout(mut self, layout: LayoutStyle) -> Self {
        self.layout = layout;
        self
    }

    /// Construct the standalone [`Text`] struct value.
    pub fn build(self) -> Text {
        let dark = MechanixTheme::dark();
        let text_style = self.tokens.resolve(&dark);
        Text {
            font: self.font,
            text: self.text_content,
            text_style,
            tokens: self.tokens,
        }
    }
}

impl WidgetBuild for TextBuilder {
    type Widget = Text;
    fn spawn(self, me: Handle<Text>, s: &mut Spawner<Text>) -> Text {
        use theme::SpawnerThemeExt;
        let text_style = s.with_theme(|theme| self.tokens.resolve(theme));

        let layout = self.layout.clone();
        let widget = Text {
            font: self.font,
            text: self.text_content,
            text_style,
            tokens: self.tokens,
        };

        s.set_component(layout);
        s.set_component(widget.measure());
        s.set_component(widget.paint());

        s.on(me, on_set_text);
        s.on(me, on_set_font);
        s.on(me, on_set_color);
        s.on(me, on_apply_theme);

        widget
    }
}

fn on_set_text(ctx: &mut Context<Text>, e: &SetText) {
    ctx.me().text = e.0.clone();
    sync_text(ctx);
}

fn on_set_font(ctx: &mut Context<Text>, e: &SetFont) {
    ctx.me().font = Some(e.0);
    sync_text(ctx);
}

fn on_set_color(ctx: &mut Context<Text>, e: &SetColor) {
    ctx.me().text_style.color = e.0;
    ctx.me().tokens.color = e.0.into();
    sync_text(ctx);
}

use theme::ContextThemeExt;

fn on_apply_theme(ctx: &mut Context<Text>, _: &theme::ApplyTheme) {
    let theme = ctx.theme();
    ctx.me().apply_theme(&theme);
    sync_text(ctx);
}

fn sync_text(ctx: &mut Context<Text>) {
    let measure = ctx.me().measure();
    let paint = ctx.me().paint();
    ctx.set_component(measure);
    ctx.set_component(paint);
}
