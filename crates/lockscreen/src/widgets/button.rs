use renderer::commands::Color;
use taffy::prelude::*;
use taffy::{Size, Style};
use ui::widgets::{BorderColor, Div, Text};
use ui::{Damage, OnChange, Point, Render, RenderCommand};
use utils::Rect as UtilsRect;

use crate::atlas;

pub fn filled_button_style() -> Style {
    Style {
        display: Display::Flex,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: length(152.0_f32),
            height: length(56.0_f32),
        },
        ..Style::default()
    }
}

pub fn transparent_button_style() -> Style {
    Style {
        display: Display::Flex,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: length(152.0_f32),
            height: length(56.0_f32),
        },
        ..Style::default()
    }
}

pub fn button_style() -> Style {
    filled_button_style()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonType {
    #[default]
    Filled,
    Transparent,
}

#[ui::widget]
pub struct Button {
    pub kind: ButtonType,
    #[widget(child)]
    pub children: Div<(Text,)>,
}

impl Render for Button {
    fn render(&self, _layout: &taffy::Layout, _abs_pos: Point) -> Vec<RenderCommand> {
        Vec::new()
    }
}

impl Button {
    pub fn new(num: &str) -> Self {
        Self::filled(num)
    }

    pub fn filled(num: &str) -> Self {
        Self::filled_with_style(num, filled_button_style())
    }

    pub fn transparent(num: &str) -> Self {
        Self::transparent_with_style(num, transparent_button_style())
    }

    pub fn with_style(num: &str, style: Style) -> Self {
        Self::filled_with_style(num, style)
    }

    pub fn filled_with_style(num: &str, style: Style) -> Self {
        let mut txt = Text::new(Style::default());
        txt.font = Some(&atlas::LOCKSCREEN_FONT_INTER_24);
        txt.color = Color::from_rgb8(242, 242, 242);
        txt.text = num.into();

        let mut btn = Div::new(style.clone(), (txt,));
        btn.color = Color::from_rgb8(13, 13, 13);
        btn.border_color = BorderColor(Color::from_rgb8(28, 28, 28));
        btn.border_thickness = 1.0;
        btn.border_radius = 6.0;

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style,
            bounds: UtilsRect::ZERO,
            pending_damage: Damage::None,
            is_opaque: true,
            kind: ButtonType::Filled,
            children: btn,
        }
    }

    pub fn transparent_with_style(num: &str, style: Style) -> Self {
        let mut txt = Text::new(Style::default());
        txt.font = Some(&atlas::LOCKSCREEN_FONT_INTER_12);
        txt.color = Color::from_rgb8(242, 242, 242);
        txt.text = num.into();

        let mut btn = Div::new(style.clone(), (txt,));
        btn.color = Color::TRANSPARENT;
        btn.border_color = BorderColor(Color::TRANSPARENT);
        btn.border_thickness = 0.0;
        btn.border_radius = 6.0;

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style,
            bounds: UtilsRect::ZERO,
            pending_damage: Damage::None,
            is_opaque: true,
            kind: ButtonType::Transparent,
            children: btn,
        }
    }
}

impl OnChange<bool> for Button {
    fn damage(&self, _new: &bool) -> Damage {
        Damage::None
    }

    fn change(&mut self, is_active: bool) {
        match (self.kind, is_active) {
            (ButtonType::Transparent, _) => {}
            (ButtonType::Filled, true) => {
                self.children.set(Color::from_rgb8(27, 27, 27));
                self.children
                    .set(BorderColor(Color::from_rgb8(138, 138, 136)));
            }
            (ButtonType::Filled, false) => {
                self.children.set(Color::from_rgb8(13, 13, 13));
                self.children.set(BorderColor(Color::from_rgb8(28, 28, 28)));
            }
        }
    }
}
