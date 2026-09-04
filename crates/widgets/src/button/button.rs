use super::button_style::{ButtonSize, ButtonStyle, ResolvedButtonStyle};
use assets::BakedFont;
use taffy::prelude::{auto, length};
use taffy::{
    AlignItems, Display, JustifyContent, LengthPercentage, Rect as TaffyRect, Size as TaffySize,
    Style,
};
use theme::{MechanixTheme, WidgetState};
use ui::widget;
use ui::{Color, Damage, OnChange, Point, Rect, Render, RenderCommand, USize};

use crate::Text;

#[widget]
pub struct Button {
    pub theme: MechanixTheme,
    pub font: Option<&'static BakedFont>,
    pub tokens: ButtonStyle,
    pub state: WidgetState,
    pub background_color: Color,
    pub border_color: Color,
    pub border_thickness: f32,
    pub border_radius: Option<f32>,
    pub padding: Option<TaffyRect<LengthPercentage>>,
    #[widget(child)]
    pub label_widget: Text,
}

impl Render for Button {
    fn render(&self, layout: &taffy::Layout, abs_pos: Point) -> Vec<RenderCommand> {
        let _radius = self
            .border_radius
            .unwrap_or_else(|| self.tokens.shape.radius_px(layout.size.height));

        vec![RenderCommand::DrawQuad {
            color: self.background_color,
            border_color: self.border_color,
            origin: abs_pos,
            z: 0.0,
            size: USize::new(layout.size.width, layout.size.height),
            border_radius: 1.0,
            border_thickness: self.border_thickness,
            background: Color::TRANSPARENT,
            is_opaque: true,
        }]
    }

    fn fill(&self) -> Color {
        self.background_color
    }
}

impl Button {
    pub fn new(text: impl Into<String>, theme: &MechanixTheme) -> Self {
        Self::with_tokens(text, theme, ButtonStyle::filled())
    }

    pub fn outlined(text: impl Into<String>, theme: &MechanixTheme) -> Self {
        Self::with_tokens(text, theme, ButtonStyle::outlined())
    }

    fn with_tokens(text: impl Into<String>, theme: &MechanixTheme, tokens: ButtonStyle) -> Self {
        let label = text.into();
        let state = WidgetState::Enabled;
        let style_spec = tokens.resolve(theme, state);

        let label_widget = Text::new(label.clone(), theme)
            .color(style_spec.content_color)
            .variant(tokens.size.typography_variant);

        let container_style = Style {
            display: Display::Flex,
            justify_content: Some(JustifyContent::Center),
            align_items: Some(AlignItems::Center),
            size: TaffySize {
                width: auto(),
                height: length(tokens.size.height),
            },
            padding: style_spec.padding,
            ..Style::default()
        };

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style: container_style,
            bounds: Rect::ZERO,
            pending_damage: Damage::None,
            is_opaque: true,
            theme: theme.clone(),
            font: None,
            tokens,
            state,
            background_color: style_spec.background_color,
            border_color: style_spec.border_color,
            border_thickness: style_spec.border_thickness,
            border_radius: None,
            padding: None,
            label_widget,
        }
    }

    // Builder Methods

    pub fn width(mut self, width: f32) -> Self {
        self.style.size.width = length(width);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.style.size.height = length(height);
        self
    }

    /// Builder: Set the Button size ([`ButtonSize::SMALL`], [`ButtonSize::MEDIUM`], [`ButtonSize::LARGE`], etc.).
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.tokens.size = size;
        self.style.size.height = length(size.height);
        let padding = self.padding.unwrap_or(size.padding);
        self.style.padding = padding;
        self.label_widget = self.label_widget.variant(size.typography_variant);
        self
    }

    pub fn padding(mut self, padding: TaffyRect<LengthPercentage>) -> Self {
        self.padding = Some(padding);
        self.style.padding = padding;
        self
    }

    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn border_thickness(mut self, thickness: f32) -> Self {
        self.border_thickness = thickness;
        self.tokens.border_thickness = thickness;
        self
    }

    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = Some(radius);
        self
    }

    pub fn focus_border_color(mut self, color: theme::ColorVariant) -> Self {
        self.tokens.focus_border_color = Some(color);
        let style_spec = self.tokens.resolve(&self.theme, self.state);
        self.apply_resolved_style(&style_spec);
        self
    }

    pub fn focus_border_thickness(mut self, thickness: f32) -> Self {
        self.tokens.focus_border_thickness = thickness;
        let style_spec = self.tokens.resolve(&self.theme, self.state);
        self.apply_resolved_style(&style_spec);
        self
    }

    pub fn font(mut self, font: &'static BakedFont) -> Self {
        self.font = Some(font);
        self.label_widget = self.label_widget.font(font);
        self
    }

    /// Builder: Set the widget state ([`WidgetState`]).
    pub fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        let style_spec = self.tokens.resolve(&self.theme, state);
        self.apply_resolved_style(&style_spec);
        self
    }

    /// Re-resolve colors and border metrics for a new state and theme.
    pub fn apply_theme(&mut self, theme: &MechanixTheme, state: WidgetState) {
        self.state = state;
        self.theme = theme.clone();
        let style_spec = self.tokens.resolve(theme, state);
        self.apply_resolved_style(&style_spec);
    }

    /// Apply a pre-resolved [`ResolvedButtonStyle`].
    fn apply_resolved_style(&mut self, style: &ResolvedButtonStyle) {
        self.background_color = style.background_color;
        self.border_color = style.border_color;
        self.border_thickness = style.border_thickness;
        self.label_widget.text_style.color = style.content_color;
        let padding = self.padding.unwrap_or(style.padding);
        self.style.padding = padding;
    }
}

impl OnChange<String> for Button {
    fn damage(&self, _new: &String) -> Damage {
        Damage::Layout
    }

    fn change(&mut self, new: String) {
        self.label_widget.set(new);
    }
}

impl OnChange<WidgetState> for Button {
    fn damage(&self, _new: &WidgetState) -> Damage {
        Damage::Paint(self.bounds)
    }

    fn change(&mut self, new: WidgetState) {
        let theme = self.theme.clone();
        self.apply_theme(&theme, new);
    }
}

impl OnChange<Color> for Button {
    fn damage(&self, _new: &Color) -> Damage {
        Damage::Paint(self.bounds)
    }

    fn change(&mut self, new: Color) {
        self.background_color = new;
    }
}

impl OnChange<Option<&'static BakedFont>> for Button {
    fn damage(&self, _new: &Option<&'static BakedFont>) -> Damage {
        Damage::Layout
    }

    fn change(&mut self, new: Option<&'static BakedFont>) {
        self.font = new;
        self.label_widget.font = new;
    }
}

impl OnChange<&'static BakedFont> for Button {
    fn damage(&self, new: &&'static BakedFont) -> Damage {
        self.damage(&Some(*new))
    }

    fn change(&mut self, new: &'static BakedFont) {
        self.change(Some(new));
    }
}
