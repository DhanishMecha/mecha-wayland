use crate::ScreenState;
use crate::atlas;
use renderer::commands::Color;
use taffy::prelude::*;
use taffy::{Size, Style};
use ui::widgets::{Div, Text};
use ui::{Damage, OnChange, Point, Render, RenderCommand};

fn bottom_container_style() -> Style {
    Style {
        display: Display::Flex,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        ..Style::default()
    }
}

/// Widget representing bottom bar controls (e.g., "HOLD TO EXPAND", "CANCEL").
#[ui::widget]
pub struct BottomBar {
    #[widget(child)]
    pub children: Div<(Text,)>,
}

impl Render for BottomBar {
    fn render(&self, _layout: &taffy::Layout, _abs_pos: Point) -> Vec<RenderCommand> {
        Vec::new()
    }
}

impl BottomBar {
    pub fn new(text: &str) -> Self {
        let mut bottom_widget = Text::new(Style::default());
        bottom_widget.font = Some(&atlas::LOCKSCREEN_FONT_GEIST_MONO_12);
        bottom_widget.color = Color::from_rgb8(140, 140, 145);
        bottom_widget.text = text.into();

        let bottom_container = Div::new(bottom_container_style(), (bottom_widget,));

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style: bottom_container_style(),
            bounds: utils::Rect::ZERO,
            pending_damage: Damage::None,
            is_opaque: true,
            children: bottom_container,
        }
    }

    /// Sets the text content for the bottom bar.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.children.children.0.set(text.into());
    }
}

impl OnChange<ScreenState> for BottomBar {
    fn damage(&self, _new: &ScreenState) -> Damage {
        Damage::None
    }

    fn change(&mut self, state: ScreenState) {
        match state {
            ScreenState::HoldToExpand => self.set_text("HOLD TO EXPAND"),
            ScreenState::PinLock => self.set_text("CANCEL"),
        }
    }
}
