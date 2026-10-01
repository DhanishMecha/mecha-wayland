use super::button_style::{ButtonOverrides, ButtonSize, ButtonStyle, ButtonVariant};
use crate::div::{Div, DivContext, div};
use crate::icon::{Icon, IconContext, icon};
use crate::state::WidgetState;
use crate::text::{Text, TextContext, text};
use WidgetState::*;
use app::{Build, Context, Handle, Spawner, Widget};
use atlas::{FontId, SpriteId};
use geometry::{Color, Insets, Size};
use interactivity::{Clicked, Enter, Exit, Press, Release};
use layout::{LayoutStyle, Val, px};
use theme::{ColorRole, SpawnerThemeExt, ThemeReader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetButtonState(pub WidgetState);
impl app::Event for SetButtonState {}

/// A Mechanix button widget holding design tokens and interaction state.
#[derive(Debug, Clone)]
pub struct Button {
    pub tokens: ButtonStyle,
    pub overrides: ButtonOverrides,
    pub state: WidgetState,
    pub div_handle: Option<Handle<Div>>,
    pub label_handle: Option<Handle<Text>>,
    pub icon_handle: Option<Handle<Icon>>,
}

/// Create a [`Button`] builder (defaults to filled).
pub fn button(font: FontId, text_content: impl Into<String>) -> ButtonBuilder {
    ButtonBuilder::new(font, text_content)
}

// ButtonBuilder
/// Callback handler type for button click events.
pub type OnClickHandler = Box<dyn FnMut(&mut Context<'_, Button>) + 'static>;

/// Builder for constructing a [`Button`] widget.
pub struct ButtonBuilder {
    font: FontId,
    tokens: ButtonStyle,
    overrides: ButtonOverrides,
    state: WidgetState,
    layout: Option<LayoutStyle>,
    width: Option<Val>,
    height: Option<Val>,
    label: String,
    icon: Option<SpriteId>,
    on_click: Option<OnClickHandler>,
}

impl ButtonBuilder {
    /// Create a filled button builder.
    pub fn new(font: FontId, text_content: impl Into<String>) -> Self {
        Self {
            font,
            tokens: ButtonStyle::filled(),
            overrides: Default::default(),
            state: Default::default(),
            layout: None,
            width: None,
            height: None,
            label: text_content.into(),
            icon: None,
            on_click: None,
        }
    }

    fn build_layout(&self) -> LayoutStyle {
        let mut l = self.layout.clone().unwrap_or_else(|| {
            LayoutStyle::default()
                .row()
                .center()
                .gap(px(8.0))
                .min_height(px(self.tokens.size.height))
                .padding(self.tokens.size.padding)
        });
        if let Some(w) = self.width {
            l = l.width(w);
        }
        if let Some(h) = self.height {
            l = l.height(h);
        }
        l
    }

    // Builder Methods
    /// Set container width.
    pub fn width(mut self, width: impl Into<Val>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Set container height.
    pub fn height(mut self, height: impl Into<Val>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Set button size preset ([`ButtonSize::SMALL`], [`ButtonSize::MEDIUM`], [`ButtonSize::LARGE`], etc.).
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.tokens.size = size;
        self
    }

    /// Set explicit container padding insets.
    pub fn padding(mut self, padding: Insets<Val>) -> Self {
        self.tokens.size.padding = padding;
        self
    }

    /// Set explicit background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.overrides.background_color = Some(color);
        self
    }

    /// Set explicit border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.overrides.border_color = Some(color);
        self
    }

    /// Set explicit border thickness in dp.
    pub fn border_thickness(mut self, thickness: f32) -> Self {
        self.overrides.border_thickness = Some(thickness);
        self
    }

    /// Set explicit border radius in pixels.
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.overrides.border_radius = Some(radius);
        self
    }

    /// Set explicit content / label color.
    pub fn content_color(mut self, color: Color) -> Self {
        self.overrides.content_color = Some(color);
        self
    }

    /// Set whole layout style, resetting explicit `width` and `height` overrides.
    ///
    /// Modifiers chained after `.layout(...)` (e.g. `.width(...)`, `.height(...)`) take precedence.
    pub fn layout(mut self, layout: LayoutStyle) -> Self {
        self.layout = Some(layout);
        self.width = None;
        self.height = None;
        self
    }

    /// Set variant preset.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.tokens = self.tokens.with_variant(variant);
        self.overrides = ButtonOverrides::default();
        self
    }

    /// Replace the entire design tokens specification with a custom [`ButtonStyle`].
    pub fn style(mut self, tokens: ButtonStyle) -> Self {
        self.tokens = tokens;
        self.overrides = ButtonOverrides::default();
        self
    }

    /// Enable accented styling mode.
    pub fn accented(mut self) -> Self {
        self.tokens = self.tokens.accented();
        self.overrides.clear_colors();
        self
    }

    /// Set focus border color role.
    pub fn focus_border_color(mut self, color: ColorRole) -> Self {
        self.tokens.focus_border_color = Some(color);
        self
    }

    /// Set focus border thickness in dp.
    pub fn focus_border_thickness(mut self, thickness: f32) -> Self {
        self.tokens.focus_border_thickness = thickness;
        self
    }

    /// Set label font override.
    pub fn font(mut self, font: FontId) -> Self {
        self.font = font;
        self
    }

    /// Attach an icon sprite displayed before the label.
    pub fn icon(mut self, sprite: SpriteId) -> Self {
        self.icon = Some(sprite);
        self
    }

    /// Set interaction state ([`WidgetState`]).
    pub fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Disable the button initially.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state = if disabled {
            WidgetState::Disabled
        } else {
            WidgetState::Enabled
        };
        self
    }

    /// Attach an `on_click` callback.
    pub fn on_click(mut self, handler: impl FnMut(&mut Context<'_, Button>) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl Build for ButtonBuilder {
    type Widget = Button;
}

// interaction binding
fn bind_pointer_transitions(s: &mut Spawner<'_, Button>, me: Handle<Button>) {
    s.on::<Enter>(me, |ctx, _| {
        if ctx.me().state == Enabled {
            ctx.set_button_state(Hovered);
        }
    });
    s.on::<Exit>(me, |ctx, _| {
        if matches!(ctx.me().state, Hovered | Pressed) {
            ctx.set_button_state(Enabled);
        }
    });
    s.on::<Press>(me, |ctx, _| {
        if matches!(ctx.me().state, Enabled | Hovered | Focused) {
            ctx.set_button_state(Pressed);
        }
    });
    s.on::<Release>(me, |ctx, _| {
        if ctx.me().state == Pressed {
            ctx.set_button_state(Hovered);
        }
    });
}

impl Widget for Button {
    type Builder = ButtonBuilder;

    fn build(b: ButtonBuilder, me: Handle<Button>, s: &mut Spawner<'_, Button>) -> Button {
        let (button_style, font_size) = {
            let theme = s.theme();
            (
                b.tokens.resolve(theme, b.state, &b.overrides),
                resolve_font_size(&b.tokens, theme),
            )
        };

        let div_handle: Handle<Div> = s.spawn(
            me,
            div()
                .style(b.build_layout())
                .background(button_style.background_color)
                .border(button_style.border_thickness, button_style.border_color)
                .radius(button_style.border_radius),
        );

        let icon_handle = b.icon.map(|sprite| {
            let icon_sz = Size::new(b.tokens.size.icon_size, b.tokens.size.icon_size);
            s.spawn(
                div_handle,
                icon(sprite)
                    .size(icon_sz)
                    .color(button_style.content_color),
            )
        });

        let label_handle: Handle<Text> = s.spawn(
            div_handle,
            text(b.font, b.label)
                .size(font_size)
                .color(button_style.content_color),
        );
        // pointer events for hover, pressed, release, exit
        bind_pointer_transitions(s, me);

        if let Some(mut cb) = b.on_click {
            s.on::<Clicked>(me, move |ctx, _| {
                if ctx.me().state != WidgetState::Disabled {
                    cb(ctx);
                }
            });
        }

        s.on::<SetButtonState>(me, |ctx, e| ctx.set_button_state(e.0));
        s.on_theme(me, sync_theme);

        Button {
            tokens: b.tokens,
            overrides: b.overrides,
            state: b.state,
            div_handle: Some(div_handle),
            label_handle: Some(label_handle),
            icon_handle,
        }
    }
}

// ButtonContext

/// Post-spawn setters for [`Button`], implemented on `Context<'_, Button>`.
pub trait ButtonContext {
    /// Enable or disable the button at runtime.
    fn set_disabled(&mut self, disabled: bool);

    /// Set the interaction state directly at runtime.
    fn set_button_state(&mut self, state: WidgetState);

    /// Change the button variant at runtime.
    fn set_variant(&mut self, variant: ButtonVariant);

    /// Set or update the label text at runtime.
    fn set_label(&mut self, text: impl Into<String>);
}

impl ButtonContext for Context<'_, Button> {
    fn set_disabled(&mut self, disabled: bool) {
        let new_state = if disabled {
            WidgetState::Disabled
        } else {
            WidgetState::Enabled
        };
        self.set_button_state(new_state);
    }

    fn set_button_state(&mut self, new_state: WidgetState) {
        if self.me().state == new_state {
            return;
        }
        self.me().state = new_state;
        sync_button(self);
    }

    fn set_variant(&mut self, variant: ButtonVariant) {
        let t = self.me().tokens.with_variant(variant);
        self.me().tokens = t;
        self.me().overrides = ButtonOverrides::default();
        sync_theme(self);
    }

    fn set_label(&mut self, text: impl Into<String>) {
        let text = text.into();
        if let Some(h) = self.me().label_handle
            && let Some(mut c) = self.at(h)
        {
            c.update(|t| t.string = text);
        }
    }
}

// helpers

fn resolve_font_size(tokens: &ButtonStyle, theme: &theme::MechanixTheme) -> u16 {
    tokens
        .size
        .typography_variant
        .resolve(&theme.typography)
        .font_size as u16
}

fn sync_button(ctx: &mut Context<'_, Button>) {
    let (tokens, state, overrides) = {
        let me = ctx.me();
        (me.tokens, me.state, me.overrides)
    };
    let style_spec = tokens.resolve(ctx.theme(), state, &overrides);

    if let Some(h) = ctx.me().div_handle
        && let Some(mut c) = ctx.at(h)
    {
        c.set_background(style_spec.background_color);
        c.set_border(style_spec.border_thickness, style_spec.border_color);
        c.set_radius(style_spec.border_radius);
    }

    let fg = style_spec.content_color;
    if let Some(h) = ctx.me().label_handle
        && let Some(mut c) = ctx.at(h)
    {
        c.update(|t| t.color = fg);
    }
    if let Some(h) = ctx.me().icon_handle
        && let Some(mut c) = ctx.at(h)
    {
        c.set_color(fg);
    }
}

fn sync_theme(ctx: &mut Context<'_, Button>) {
    sync_button(ctx);
    let tokens = ctx.me().tokens;
    let font_size = resolve_font_size(&tokens, ctx.theme());
    if let Some(h) = ctx.me().label_handle
        && let Some(mut c) = ctx.at(h)
    {
        c.update(|t| t.px = font_size);
    }
    if let Some(h) = ctx.me().icon_handle
        && let Some(mut c) = ctx.at(h)
    {
        c.set_size(geometry::Size::new(
            tokens.size.icon_size,
            tokens.size.icon_size,
        ));
    }
}
