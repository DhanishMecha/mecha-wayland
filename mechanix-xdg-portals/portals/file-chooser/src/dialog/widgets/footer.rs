use assets::BakedFont;
use portal_core::widgets::Button;
use taffy::prelude::*;
use ui::widgets::Div;
use ui::{ Point, Render, RenderCommand };
use utils::Color;

#[ui::widget]
pub struct Footer {
    #[widget(child)]
    pub div: Div<(Button, Button)>,
}

impl Footer {
    pub fn new(font: &'static BakedFont, accept_label: Option<&str>, default_label: &str) -> Self {
        let label = accept_label.unwrap_or(default_label);
        let mut choose_btn = Button::new(label);
        choose_btn.div.color = Color::rgb(0.08, 0.35, 0.8);
        choose_btn.div.border_color = Color::rgb(0.15, 0.45, 0.9);
        choose_btn.div.border_radius = 10.0;
        choose_btn.div.border_thickness = 1.5;
        choose_btn.div.z = 1.0;
        choose_btn.div.children.font = Some(font);
        choose_btn.div.children.color = Color::WHITE;
        choose_btn.div.children.z = 0.5;

        let mut cancel_btn = Button::new("Cancel");
        cancel_btn.div.color = Color::rgb(0.06, 0.06, 0.07);
        cancel_btn.div.border_color = Color::rgb(0.12, 0.12, 0.14);
        cancel_btn.div.border_radius = 10.0;
        cancel_btn.div.border_thickness = 1.5;
        cancel_btn.div.z = 1.0;
        cancel_btn.div.children.font = Some(font);
        cancel_btn.div.children.color = Color::WHITE;
        cancel_btn.div.children.z = 0.5;

        let div_style = Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            size: Size {
                width: percent(1.0_f32),
                height: length(50.0_f32),
            },
            justify_content: Some(JustifyContent::SpaceBetween),
            align_items: Some(AlignItems::Center),
            ..Default::default()
        };

        let div = Div::new(div_style, (cancel_btn, choose_btn));

        let style = Style {
            display: Display::Flex,
            size: Size {
                width: percent(1.0_f32),
                height: length(50.0_f32),
            },
            ..Default::default()
        };

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style,
            div,
        }
    }
}

impl Render for Footer {
    fn render(&self, _layout: &taffy::Layout, _abs_pos: Point) -> Vec<RenderCommand> {
        Vec::new()
    }
}
