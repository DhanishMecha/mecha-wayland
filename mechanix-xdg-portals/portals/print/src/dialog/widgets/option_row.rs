use assets::BakedFont;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{Point, Render, RenderCommand, WidgetTree, Widget};
use utils::Color;

#[ui::widget]
pub struct OptionRow {
    pub key: String,
    pub val: String,
    #[widget(child)]
    pub div: Div<(Text, Text)>,
}

impl OptionRow {
    pub fn new(font: &'static BakedFont, key: String, val: String) -> Self {
        let style = Style {
            display: Display::Flex,
            size: Size {
                width: percent(1.0_f32),
                height: length(20.0_f32),
            },
            margin: taffy::Rect {
                left: length(0.0_f32),
                right: length(0.0_f32),
                top: length(1.0_f32),
                bottom: length(1.0_f32),
            },
            ..Default::default()
        };

        let mut key_text = Text::new(Style {
            margin: taffy::Rect {
                left: length(0.0_f32),
                right: length(8.0_f32),
                top: length(0.0_f32),
                bottom: length(0.0_f32),
            },
            ..Default::default()
        });
        key_text.font = Some(font);
        key_text.text = format!("{}:", key);
        key_text.color = Color::rgb(0.65, 0.65, 0.75);
        key_text.z = 0.95;

        let mut val_text = Text::new(Style::default());
        val_text.font = Some(font);
        val_text.text = val.clone();
        val_text.color = Color::WHITE;
        val_text.z = 0.95;

        let div_style = Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            size: Size {
                width: percent(1.0_f32),
                height: percent(1.0_f32),
            },
            align_items: Some(AlignItems::Center),
            ..Default::default()
        };

        let div = Div::new(div_style, (key_text, val_text));

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style,
            key,
            val,
            div,
        }
    }
}

impl Render for OptionRow {
    fn render(&self, _layout: &taffy::Layout, _abs_pos: Point) -> Vec<RenderCommand> {
        vec![]
    }
}

pub struct OptionRows(pub Vec<OptionRow>);

impl ui::WidgetList for OptionRows {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        self.0
            .iter_mut()
            .map(|w| w.build_tree(tree))
            .collect()
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        for w in self.0.iter_mut() {
            let layout = tree.layout(w.node_id()).unwrap();
            commands.extend(w.render_node(layout, tree, parent_abs));
        }
        commands
    }

    fn on_event(&mut self, ctx: &mut ui::EventCtx) {
        for w in self.0.iter_mut() {
            Widget::on_event(w, ctx);
        }
    }
}
