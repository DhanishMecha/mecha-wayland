use super::button_style::{ButtonSize, ButtonStyle, ResolvedButtonStyle};
use app::{Context, Handle, Spawner, Widget, WidgetBuild};
use assets::BakedFont;
use layout::{LayoutStyle, Val, px};
use paint::{Paint, Quad};
use theme::{MechanixTheme, ThemeColor};
use utils::{Color, Edges};

use crate::text::{Text, text};
use crate::utils::WidgetState;
use crate::{SetColor, SetText};

/// Event to update the interaction state ([`WidgetState`]) of a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetButtonState(pub WidgetState);
impl app::Event for SetButtonState {}

/// A Material 3 button widget holding design tokens and interaction state.
#[derive(Debug, Clone)]
pub struct Button {
    /// Optional font override for label text.
    pub font: Option<&'static BakedFont>,
    /// Concrete resolved button visual parameters.
    pub button_style: ResolvedButtonStyle,
    /// Material 3 design tokens.
    pub tokens: ButtonStyle,
    /// Active interaction state.
    pub state: WidgetState,
    /// Handle to the spawned child label text node.
    pub label_handle: Option<Handle<Text>>,
}

impl Widget for Button {}

impl Button {
    /// Create a new filled button builder with explicit theme.
    pub fn new(text_content: impl Into<String>, theme: &MechanixTheme) -> ButtonBuilder {
        ButtonBuilder::new(text_content, theme)
    }

    /// Create a new outlined button builder with explicit theme.
    pub fn outlined(text_content: impl Into<String>, theme: &MechanixTheme) -> ButtonBuilder {
        ButtonBuilder::outlined(text_content, theme)
    }

    /// Create a filled button builder with default theme.
    pub fn builder(text_content: impl Into<String>) -> ButtonBuilder {
        button(text_content)
    }

    /// Create an outlined button builder with default theme.
    pub fn builder_outlined(text_content: impl Into<String>) -> ButtonBuilder {
        button_outlined(text_content)
    }
}

/// Create a filled button builder with default theme.
pub fn button(text_content: impl Into<String>) -> ButtonBuilder {
    ButtonBuilder::new(text_content, &MechanixTheme::dark())
}

/// Create an outlined button builder with default theme.
pub fn button_outlined(text_content: impl Into<String>) -> ButtonBuilder {
    ButtonBuilder::outlined(text_content, &MechanixTheme::dark())
}

/// Builder for constructing a [`Button`] widget.
pub struct ButtonBuilder {
    font: Option<&'static BakedFont>,
    tokens: ButtonStyle,
    state: WidgetState,
    layout: LayoutStyle,
    label: String,
    custom_background: Option<Color>,
    custom_border_color: Option<Color>,
    custom_border_radius: Option<f32>,
}

impl ButtonBuilder {
    /// Create a filled button builder.
    pub fn new(text_content: impl Into<String>, theme: &MechanixTheme) -> Self {
        Self::with_tokens(text_content, theme, ButtonStyle::filled())
    }

    /// Create an outlined button builder.
    pub fn outlined(text_content: impl Into<String>, theme: &MechanixTheme) -> Self {
        Self::with_tokens(text_content, theme, ButtonStyle::outlined())
    }

    /// Create a button builder with custom token specification.
    pub fn with_tokens(
        text_content: impl Into<String>,
        theme: &MechanixTheme,
        tokens: ButtonStyle,
    ) -> Self {
        let state = WidgetState::Enabled;
        let style_spec = tokens.resolve(theme, state);
        let layout = LayoutStyle::default()
            .row()
            .center()
            .height(px(tokens.size.height))
            .padding(style_spec.padding);

        Self {
            font: None,
            tokens,
            state,
            layout,
            label: text_content.into(),
            custom_background: None,
            custom_border_color: None,
            custom_border_radius: None,
        }
    }

    // ── Builder Methods ──────────────────────────────────────────────────────

    /// Set container width.
    pub fn width(mut self, width: Val) -> Self {
        self.layout = self.layout.width(width);
        self
    }

    /// Set container height.
    pub fn height(mut self, height: Val) -> Self {
        self.layout = self.layout.height(height);
        self
    }

    /// Set button size preset ([`ButtonSize::SMALL`], [`ButtonSize::MEDIUM`], [`ButtonSize::LARGE`], etc.).
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.tokens.size = size;
        self.layout = self.layout.height(px(size.height)).padding(size.padding);
        self
    }

    /// Set explicit container padding.
    pub fn padding(mut self, padding: Edges<Val>) -> Self {
        self.tokens.size.padding = padding;
        self.layout = self.layout.padding(padding);
        self
    }

    /// Set explicit background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.custom_background = Some(color);
        self
    }

    /// Set explicit border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.custom_border_color = Some(color);
        self
    }

    /// Set explicit border thickness in dp.
    pub fn border_thickness(mut self, thickness: f32) -> Self {
        self.tokens.border_thickness = thickness;
        self
    }

    /// Set explicit border radius in pixels.
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.custom_border_radius = Some(radius);
        self
    }

    /// Enable or disable accented styling mode.
    pub fn set_accented(&mut self, accented: bool) -> &mut Self {
        let is_outlined =
            self.tokens.background_color.is_none() || self.tokens.border_color.is_some();
        if is_outlined {
            if accented {
                self.tokens.border_color = Some(ThemeColor::Primary);
                self.tokens.label_color = ThemeColor::Primary;
                self.tokens.hover_state_layer.color_variant = ThemeColor::Primary;
                self.tokens.focus_state_layer.color_variant = ThemeColor::Primary;
                self.tokens.pressed_state_layer.color_variant = ThemeColor::Primary;
            } else {
                self.tokens.border_color = Some(ThemeColor::Outline);
                self.tokens.label_color = ThemeColor::OnPrimary;
                self.tokens.hover_state_layer.color_variant = ThemeColor::OnSurfaceVariant;
                self.tokens.focus_state_layer.color_variant = ThemeColor::OnSurfaceVariant;
                self.tokens.pressed_state_layer.color_variant = ThemeColor::OnSurfaceVariant;
            }
        } else if accented {
            self.tokens.background_color = Some(ThemeColor::Primary);
            self.tokens.label_color = ThemeColor::OnPrimary;
            self.tokens.hover_state_layer.color_variant = ThemeColor::OnPrimary;
            self.tokens.focus_state_layer.color_variant = ThemeColor::OnPrimary;
            self.tokens.pressed_state_layer.color_variant = ThemeColor::OnPrimary;
        } else {
            self.tokens.background_color = Some(ThemeColor::SecondaryFixedDim);
            self.tokens.label_color = ThemeColor::OnPrimary;
        }
        self
    }

    /// Enable accented styling mode.
    pub fn accented(mut self) -> Self {
        self.set_accented(true);
        self
    }

    /// Set focus border color role.
    pub fn focus_border_color(mut self, color: ThemeColor) -> Self {
        self.tokens.focus_border_color = Some(color);
        self
    }

    /// Set focus border thickness in dp.
    pub fn focus_border_thickness(mut self, thickness: f32) -> Self {
        self.tokens.focus_border_thickness = thickness;
        self
    }

    /// Set label font override.
    pub fn font(mut self, font: &'static BakedFont) -> Self {
        self.font = Some(font);
        self
    }

    /// Set interaction state ([`WidgetState`]).
    pub fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Set whole layout style.
    pub fn layout(mut self, layout: LayoutStyle) -> Self {
        self.layout = layout;
        self
    }

    /// Apply theme and interaction state.
    pub fn apply_theme(mut self, _theme: &MechanixTheme, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Build resolved button visual parameters based on current configuration.
    pub fn resolve_style(&self, theme: &MechanixTheme) -> ResolvedButtonStyle {
        let mut style = self.tokens.resolve(theme, self.state);
        if let Some(bg) = self.custom_background {
            style.background_color = bg;
        }
        if let Some(bc) = self.custom_border_color {
            style.border_color = bc;
        }
        if let Some(r) = self.custom_border_radius {
            style.border_radius = Some(r);
        }
        style
    }

    /// Construct the standalone [`Button`] struct value.
    pub fn build(self) -> Button {
        let dark = MechanixTheme::dark();
        let button_style = self.resolve_style(&dark);
        Button {
            font: self.font,
            button_style,
            tokens: self.tokens,
            state: self.state,
            label_handle: None,
        }
    }
}

impl WidgetBuild for ButtonBuilder {
    type Widget = Button;
    fn spawn(self, me: Handle<Button>, s: &mut Spawner<Button>) -> Button {
        use theme::SpawnerThemeExt;
        let theme = s.theme();
        let button_style = self.resolve_style(&theme);
        let layout = self.layout.clone().padding(button_style.padding);

        let radius = button_style.border_radius.unwrap_or(0.0);
        let quad = Quad::new(button_style.background_color)
            .border(button_style.border_thickness, button_style.border_color)
            .radius(radius);

        s.set_component(layout);
        s.set_component(Paint::Quad(quad));

        let mut label_builder = text(self.label.clone())
            .variant(self.tokens.size.typography_variant)
            .color(button_style.content_color);
        if let Some(font) = self.font {
            label_builder = label_builder.font(font);
        }
        let text_child = s.child(me, label_builder);

        s.on(me, on_set_button_state);
        s.on(me, on_set_text);
        s.on(me, on_apply_theme);

        Button {
            font: self.font,
            button_style,
            tokens: self.tokens,
            state: self.state,
            label_handle: Some(text_child),
        }
    }
}

fn on_set_button_state(ctx: &mut Context<Button>, e: &SetButtonState) {
    ctx.me().state = e.0;
    sync_button(ctx);
}

fn on_set_text(ctx: &mut Context<Button>, e: &SetText) {
    if let Some(text_child) = ctx.me().label_handle {
        ctx.emit(e.clone(), &[text_child.into()]);
    }
}

fn on_apply_theme(ctx: &mut Context<Button>, _: &theme::ApplyTheme) {
    sync_button(ctx);
}

fn sync_button(ctx: &mut Context<Button>) {
    use theme::ContextThemeExt;
    let theme = ctx.theme();

    let state = ctx.me().state;
    let tokens = ctx.me().tokens;
    let prev_radius = ctx.me().button_style.border_radius;
    let text_child = ctx.me().label_handle;

    let mut style_spec = tokens.resolve(&theme, state);
    if style_spec.border_radius.is_none() {
        style_spec.border_radius = prev_radius;
    }
    ctx.me().button_style = style_spec;

    let radius = style_spec.border_radius.unwrap_or(0.0);
    let quad = Quad::new(style_spec.background_color)
        .border(style_spec.border_thickness, style_spec.border_color)
        .radius(radius);

    if let Some(layout) = ctx.component::<LayoutStyle>().map(|s| (*s).clone()) {
        ctx.set_component(layout.padding(style_spec.padding));
    }
    ctx.set_component(Paint::Quad(quad));
    if let Some(text_child) = text_child {
        ctx.emit(SetColor(style_spec.content_color), &[text_child.into()]);
    }
}
