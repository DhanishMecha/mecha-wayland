use crate::color::ColorSource;
use crate::font::FontContextExt;
use app::{Build, Context, Handle, Spawner, Widget};
use layout::LayoutStyle;
use theme::{ColorRole, FontWeight, SpawnerThemeExt, TextVariant, ThemeReader};
use widgets::prelude::{
    Text as NativeText, TextAlign, TextContext as NativeTextContext, TextDecoration, TextOverflow,
    TextWrap, VerticalTrim, text as native_text,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub(crate) native: Handle<NativeText>,
    pub(crate) string: String,
    pub(crate) variant: TextVariant,
    pub(crate) emphasised: bool,
    pub(crate) weight: Option<FontWeight>,
    pub(crate) size: Option<f32>,
    pub(crate) color: ColorSource,
    pub(crate) letter_spacing: f32,
    pub(crate) line_height: f32,
    // Forwarded directly to native (no theme resolution needed).
    pub(crate) vertical_trim: VerticalTrim,
    pub(crate) align: TextAlign,
    pub(crate) overflow: TextOverflow,
    pub(crate) decoration: TextDecoration,
    pub(crate) wrap: TextWrap,
    pub(crate) max_lines: Option<usize>,
}

impl Text {
    pub fn native(&self) -> Handle<NativeText> {
        self.native
    }
    pub fn text(&self) -> &str {
        &self.string
    }
    pub fn variant(&self) -> TextVariant {
        self.variant
    }
    pub fn is_emphasised(&self) -> bool {
        self.emphasised
    }
    pub fn color(&self) -> ColorSource {
        self.color
    }
    pub fn weight(&self) -> Option<FontWeight> {
        self.weight
    }
    pub fn size(&self) -> Option<f32> {
        self.size
    }
    pub fn letter_spacing(&self) -> f32 {
        self.letter_spacing
    }
    pub fn line_height(&self) -> f32 {
        self.line_height
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

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &self.string
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string)
    }
}

pub fn text(s: impl Into<String>) -> TextBuilder {
    TextBuilder::new(s)
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBuilder {
    string: String,
    style: LayoutStyle,
    variant: TextVariant,
    emphasised: bool,
    weight: Option<FontWeight>,
    size: Option<f32>,
    color: ColorSource,
    letter_spacing: Option<f32>,
    line_height: Option<f32>,
    vertical_trim: VerticalTrim,
    align: TextAlign,
    overflow: TextOverflow,
    decoration: TextDecoration,
    wrap: TextWrap,
    max_lines: Option<usize>,
}

impl TextBuilder {
    pub fn new(s: impl Into<String>) -> Self {
        Self {
            string: s.into(),
            style: LayoutStyle::default(),
            variant: TextVariant::BodyLarge,
            emphasised: false,
            weight: None,
            size: None,
            color: ColorRole::OnSurface.into(),
            letter_spacing: None,
            line_height: None,
            vertical_trim: VerticalTrim::Normal,
            align: TextAlign::Left,
            overflow: TextOverflow::Clip,
            decoration: TextDecoration::None,
            wrap: TextWrap::NoWrap,
            max_lines: None,
        }
    }

    pub fn variant(mut self, variant: TextVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn emphasised(mut self) -> Self {
        self.emphasised = true;
        self
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn size(mut self, px: f32) -> Self {
        self.size = Some(px);
        self
    }

    pub fn color(mut self, color: impl Into<ColorSource>) -> Self {
        self.color = color.into();
        self
    }

    pub fn style(mut self, style: LayoutStyle) -> Self {
        self.style = style;
        self
    }

    pub fn letter_spacing(mut self, spacing: f32) -> Self {
        self.letter_spacing = Some(spacing);
        self
    }

    pub fn line_height(mut self, height: f32) -> Self {
        self.line_height = Some(height);
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

    pub fn max_lines(mut self, n: usize) -> Self {
        self.max_lines = Some(n);
        self
    }
}

impl Build for TextBuilder {
    type Widget = Text;
}

impl Widget for Text {
    type Builder = TextBuilder;

    fn build(b: TextBuilder, me: Handle<Self>, s: &mut Spawner<'_, Self>) -> Self {
        let typo = s.typography(b.variant);
        let effective_weight = b.weight.unwrap_or(if b.emphasised {
            typo.weight_emphasised
        } else {
            typo.weight
        });
        let font = s.font(effective_weight);
        let px = b.size.unwrap_or(typo.font_size).round() as u16;
        let letter_spacing = b.letter_spacing.unwrap_or(typo.letter_spacing);
        let line_height = b.line_height.unwrap_or(typo.line_height);
        let color = b.color.resolve(s);

        *s.component_mut::<LayoutStyle>(me).unwrap() = b.style;

        let native = s.spawn(
            me,
            native_text(font, &b.string)
                .size(px)
                .color(color)
                .letter_spacing(letter_spacing)
                .line_height(line_height)
                .vertical_trim(b.vertical_trim)
                .align(b.align)
                .overflow(b.overflow)
                .decoration(b.decoration)
                .wrap(b.wrap)
                .max_lines(b.max_lines.unwrap_or(usize::MAX)),
        );

        s.on_theme(me, move |ctx| {
            let is_role = ctx.me().color.is_theme_role();
            let live_color = if is_role {
                Some(ctx.me().color.resolve(ctx))
            } else {
                None
            };
            let variant = ctx.me().variant;
            let typo = ctx.typography(variant);
            let px = ctx.me().size.unwrap_or(typo.font_size).round() as u16;
            let emphasised = ctx.me().emphasised;
            let effective_weight = ctx.me().weight.unwrap_or(if emphasised {
                typo.weight_emphasised
            } else {
                typo.weight
            });
            let new_font = ctx.font(effective_weight);
            let native = ctx.me().native;
            if let Some(mut native_ctx) = ctx.at(native) {
                if let Some(c) = live_color {
                    native_ctx.set_color(c);
                }
                native_ctx.set_size(px);
                native_ctx.set_font(new_font);
            }
        });

        Text {
            native,
            string: b.string,
            variant: b.variant,
            emphasised: b.emphasised,
            weight: b.weight,
            size: b.size,
            color: b.color,
            letter_spacing,
            line_height,
            vertical_trim: b.vertical_trim,
            align: b.align,
            overflow: b.overflow,
            decoration: b.decoration,
            wrap: b.wrap,
            max_lines: b.max_lines,
        }
    }
}

pub trait TextContextExt {
    fn set_text(&mut self, text: impl Into<String>);
    fn set_variant(&mut self, variant: TextVariant);
    fn set_emphasised(&mut self, emphasised: bool);
    fn set_color(&mut self, color: impl Into<ColorSource>);
    fn set_weight(&mut self, weight: impl Into<Option<FontWeight>>);
    fn set_size(&mut self, px: impl Into<Option<f32>>);
    fn set_letter_spacing(&mut self, spacing: f32);
    fn set_line_height(&mut self, height: f32);
    fn set_vertical_trim(&mut self, trim: VerticalTrim);
    fn set_align(&mut self, align: TextAlign);
    fn set_overflow(&mut self, overflow: TextOverflow);
    fn set_decoration(&mut self, decoration: TextDecoration);
    fn set_wrap(&mut self, wrap: TextWrap);
    fn set_max_lines(&mut self, max_lines: Option<usize>);
}

impl TextContextExt for Context<'_, Text> {
    fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.me().string = text.clone();
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_text(text);
        }
    }

    fn set_variant(&mut self, variant: TextVariant) {
        self.me().variant = variant;
        let typo = self.typography(variant);
        self.me().letter_spacing = typo.letter_spacing;
        self.me().line_height = typo.line_height;
        let px = self.me().size.unwrap_or(typo.font_size).round() as u16;
        let emphasised = self.me().emphasised;
        let effective_weight = self.me().weight.unwrap_or(if emphasised {
            typo.weight_emphasised
        } else {
            typo.weight
        });
        let new_font = self.font(effective_weight);
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_size(px);
            ctx.set_font(new_font);
            ctx.set_letter_spacing(typo.letter_spacing);
            ctx.set_line_height(typo.line_height);
        }
    }

    fn set_emphasised(&mut self, emphasised: bool) {
        self.me().emphasised = emphasised;
        let variant = self.me().variant;
        let typo = self.typography(variant);
        let effective_weight = self.me().weight.unwrap_or(if emphasised {
            typo.weight_emphasised
        } else {
            typo.weight
        });
        let new_font = self.font(effective_weight);
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_font(new_font);
        }
    }

    fn set_color(&mut self, color: impl Into<ColorSource>) {
        let source = color.into();
        self.me().color = source;
        let resolved = source.resolve(self);
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_color(resolved);
        }
    }

    fn set_weight(&mut self, weight: impl Into<Option<FontWeight>>) {
        let weight = weight.into();
        self.me().weight = weight;
        let variant = self.me().variant;
        let emphasised = self.me().emphasised;
        let typo = self.typography(variant);
        let effective_weight = weight.unwrap_or(if emphasised {
            typo.weight_emphasised
        } else {
            typo.weight
        });
        let new_font = self.font(effective_weight);
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_font(new_font);
        }
    }

    fn set_size(&mut self, px: impl Into<Option<f32>>) {
        let size = px.into();
        self.me().size = size;
        let variant = self.me().variant;
        let typo = self.typography(variant);
        let resolved_px = size.unwrap_or(typo.font_size).round() as u16;
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_size(resolved_px);
        }
    }

    fn set_letter_spacing(&mut self, spacing: f32) {
        self.me().letter_spacing = spacing;
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_letter_spacing(spacing);
        }
    }

    fn set_line_height(&mut self, height: f32) {
        self.me().line_height = height;
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_line_height(height);
        }
    }

    fn set_vertical_trim(&mut self, trim: VerticalTrim) {
        self.me().vertical_trim = trim;
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_vertical_trim(trim);
        }
    }

    fn set_align(&mut self, align: TextAlign) {
        self.me().align = align;
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_align(align);
        }
    }

    fn set_overflow(&mut self, overflow: TextOverflow) {
        self.me().overflow = overflow;
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_overflow(overflow);
        }
    }

    fn set_decoration(&mut self, decoration: TextDecoration) {
        self.me().decoration = decoration;
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_decoration(decoration);
        }
    }

    fn set_wrap(&mut self, wrap: TextWrap) {
        self.me().wrap = wrap;
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_wrap(wrap);
        }
    }

    fn set_max_lines(&mut self, max_lines: Option<usize>) {
        self.me().max_lines = max_lines;
        let native = self.me().native;
        if let Some(mut ctx) = self.at(native) {
            ctx.set_max_lines(max_lines);
        }
    }
}
