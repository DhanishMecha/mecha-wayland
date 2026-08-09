use assets::BakedFont;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, Render, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;

/// A single selectable row representing one USB device choice.
#[ui::widget]
pub struct UsbRow {
    /// The USB device ID (None = hidden slot).
    pub device_id: Option<String>,
    pub is_selected: bool,
    #[widget(child)]
    pub div: Div<(Text, Text)>,
}

impl UsbRow {
    pub fn new(font: &'static BakedFont) -> Self {
        let style = Style {
            display: Display::None, // hidden until populated
            size: Size {
                width: percent(1.0_f32),
                height: length(58.0_f32),
            },
            margin: taffy::Rect {
                left: length(0.0_f32),
                right: length(0.0_f32),
                top: length(4.0_f32),
                bottom: length(4.0_f32),
            },
            ..Default::default()
        };

        let mut name_text = Text::new(Style {
            margin: taffy::Rect {
                left: length(0.0_f32),
                right: length(0.0_f32),
                top: length(0.0_f32),
                bottom: length(2.0_f32),
            },
            ..Default::default()
        });
        name_text.font = Some(font);
        name_text.text = String::new();
        name_text.color = Color::WHITE;
        name_text.z = 0.95;

        let mut details_text = Text::new(Style::default());
        details_text.font = Some(font);
        details_text.text = String::new();
        details_text.color = Color::rgb(0.55, 0.55, 0.65);
        details_text.z = 0.95;

        let div_style = Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: Size {
                width: percent(1.0_f32),
                height: percent(1.0_f32),
            },
            justify_content: Some(JustifyContent::Center),
            align_items: Some(AlignItems::Start),
            padding: taffy::Rect {
                left: length(14.0_f32),
                right: length(14.0_f32),
                top: length(0.0_f32),
                bottom: length(0.0_f32),
            },
            ..Default::default()
        };

        let mut div = Div::new(div_style, (name_text, details_text));
        div.color = Color::rgb(0.05, 0.05, 0.07);
        div.border_radius = 8.0;
        div.border_thickness = 1.0;
        div.border_color = Color::rgb(0.10, 0.10, 0.13);
        div.z = 0.8;

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style,
            device_id: None,
            is_selected: false,
            div,
        }
    }

    pub fn update(
        &mut self,
        tree: &mut WidgetTree,
        device_id: Option<String>,
        name: String,
        details: String,
        is_selected: bool,
    ) {
        self.is_selected = is_selected;
        self.device_id = device_id.clone();

        let mut style = self.style.clone();
        if device_id.is_some() {
            style.display = Display::Flex;

            self.div.children.0.set_text(tree, name);
            self.div.children.1.set_text(tree, details);

            if is_selected {
                // High-quality checked look (glassmorphism/blue outline)
                self.div.color = Color::rgb(0.07, 0.22, 0.52);
                self.div.border_color = Color::rgb(0.15, 0.50, 1.0);
            } else {
                // Dim unchecked look
                self.div.color = Color::rgb(0.04, 0.04, 0.06);
                self.div.border_color = Color::rgb(0.09, 0.09, 0.11);
            }
        } else {
            style.display = Display::None;
            self.div.children.0.set_text(tree, String::new());
            self.div.children.1.set_text(tree, String::new());
        }
        self.set_style(tree, style);
    }
}

impl Render for UsbRow {
    fn render(&self, layout: &taffy::Layout, abs_pos: Point) -> Vec<RenderCommand> {
        let id: u64 = self.node_id.into();
        vec![RenderCommand::RegisterHitArea {
            id,
            rect: utils::Rect::new(
                abs_pos.x(),
                abs_pos.y(),
                layout.size.width,
                layout.size.height,
            ),
        }]
    }
}

pub struct UsbRows(pub Vec<UsbRow>);

impl WidgetList for UsbRows {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        self.0.iter_mut().map(|w| w.build_tree(tree)).collect()
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        for w in self.0.iter_mut() {
            let layout = tree.layout(w.node_id()).unwrap();
            commands.extend(w.render_node(layout, tree, parent_abs));
        }
        commands
    }

    fn on_event(&mut self, ctx: &mut EventCtx) {
        for w in self.0.iter_mut() {
            Widget::on_event(w, ctx);
        }
    }
}
