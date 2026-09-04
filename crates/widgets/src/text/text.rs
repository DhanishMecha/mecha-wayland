use super::text_style::{ResolvedTextStyle, TextStyle};
use assets::BakedFont;
use taffy::{AvailableSpace, Layout, Size, Style};
use theme::{FontWeight, MechanixTheme, TextVariant};
use ui::widget;
use ui::{Color, Damage, Measure, OnChange, Point, Rect, Render, RenderCommand};

#[widget(measure)]
#[derive(Clone)]
pub struct Text {
    pub font: Option<&'static BakedFont>,
    pub text: String,
    pub text_style: ResolvedTextStyle,
    pub tokens: TextStyle,
}

impl Text {
    pub fn new(text: impl Into<String>, theme: &MechanixTheme) -> Self {
        let tokens = TextStyle::default();
        let text_style = tokens.resolve(theme);
        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style: Style::default(),
            bounds: Rect::ZERO,
            pending_damage: Damage::None,
            is_opaque: true,
            font: None,
            text: text.into(),
            text_style,
            tokens,
        }
    }
    // Builder Methods
    pub fn font(mut self, font: &'static BakedFont) -> Self {
        self.font = Some(font);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.text_style.color = color;
        self.tokens.color_override = Some(color);
        self
    }

    pub fn variant(mut self, variant: TextVariant) -> Self {
        self.tokens.typography_variant = variant;
        self
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.tokens.weight = Some(weight);
        self
    }

    pub fn emphasised(mut self, emphasised: bool) -> Self {
        self.tokens.is_emphasised = emphasised;
        self
    }

    pub fn line_height(mut self, height: f32) -> Self {
        self.tokens.line_height = Some(height);
        self
    }

    pub fn letter_spacing(mut self, spacing: f32) -> Self {
        self.tokens.letter_spacing = Some(spacing);
        self
    }

    pub fn word_spacing(mut self, spacing: f32) -> Self {
        self.tokens.word_spacing = Some(spacing);
        self
    }

    fn effective_font(&self) -> &'static BakedFont {
        self.font
            .unwrap_or_else(|| crate::fonts::default_font_for_role(self.tokens.typography_variant))
    }

    pub fn apply_theme(&mut self, theme: &MechanixTheme) -> &mut Self {
        self.text_style = self.tokens.resolve(theme);
        self
    }
}

impl OnChange<String> for Text {
    fn damage(&self, _new: &String) -> Damage {
        Damage::Layout
    }
    fn change(&mut self, new: String) {
        self.text = new;
    }
}

impl OnChange<Option<&'static BakedFont>> for Text {
    fn damage(&self, _new: &Option<&'static BakedFont>) -> Damage {
        Damage::Layout
    }
    fn change(&mut self, new: Option<&'static BakedFont>) {
        self.font = new;
    }
}

impl OnChange<&'static BakedFont> for Text {
    fn damage(&self, new: &&'static BakedFont) -> Damage {
        self.damage(&Some(*new))
    }
    fn change(&mut self, new: &'static BakedFont) {
        self.change(Some(new));
    }
}

impl Measure for Text {
    fn measure(
        &self,
        _known_dimensions: Size<Option<f32>>,
        _available_space: Size<AvailableSpace>,
    ) -> Size<f32> {
        let font = self.effective_font();
        Size {
            width: font.measure_width(&self.text),
            height: font.line_height,
        }
    }
}

impl Render for Text {
    fn render(&self, _layout: &Layout, abs_pos: Point) -> Vec<RenderCommand> {
        let font = self.effective_font();
        let origin = Point::new(abs_pos.x(), abs_pos.y() + font.ascent);
        // `z`, `background`, and `is_opaque` are stamped by the render walk.
        vec![RenderCommand::DrawText {
            font,
            text: self.text.clone(),
            origin,
            z: 0.0,
            color: self.text_style.color,
            // TODO
            //  line_height: self.text_style.line_height,
            //  letter_spacing: self.text_style.letter_spacing,
            //  word_spacing: self.text_style.word_spacing,
            //  weight: self.text_style.weight,
            background: Color::TRANSPARENT,
            is_opaque: true,
        }]
    }
}
