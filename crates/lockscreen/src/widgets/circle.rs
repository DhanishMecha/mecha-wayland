use renderer::commands::Color;
use taffy::prelude::*;
use taffy::{Size, Style};
use ui::widgets::{BorderColor, Div};
use ui::{Damage, OnChange, Point, Render, RenderCommand};
use utils::Rect as UtilsRect;

pub fn circle_style() -> Style {
    Style {
        size: Size {
            width: length(14.0_f32),
            height: length(14.0_f32),
        },
        ..Style::default()
    }
}

#[ui::widget]
pub struct Circle {
    #[widget(child)]
    pub children: Div<()>,
}

impl Render for Circle {
    fn render(&self, _layout: &taffy::Layout, _abs_pos: Point) -> Vec<RenderCommand> {
        Vec::new()
    }
}

impl Circle {
    pub fn new() -> Self {
        let mut c = Div::new(circle_style(), ());
        c.border_color = BorderColor(Color::from_rgb8(140, 140, 145));
        c.border_thickness = 1.5;
        c.border_radius = 7.0;
        c.color = Color::TRANSPARENT;

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style: circle_style(),
            bounds: UtilsRect::ZERO,
            pending_damage: Damage::None,
            is_opaque: true,
            children: c,
        }
    }
}

impl Default for Circle {
    fn default() -> Self {
        Self::new()
    }
}

impl OnChange<Color> for Circle {
    fn damage(&self, _new: &Color) -> Damage {
        Damage::None
    }
    fn change(&mut self, new: Color) {
        self.children.set(new);
    }
}

impl OnChange<bool> for Circle {
    fn damage(&self, _new: &bool) -> Damage {
        Damage::None
    }

    fn change(&mut self, is_filled: bool) {
        let color = if is_filled {
            Color::from_rgb8(217, 217, 217)
        } else {
            Color::TRANSPARENT
        };
        self.children.set(color);
    }
}
