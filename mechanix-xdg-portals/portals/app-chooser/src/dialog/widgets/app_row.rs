use assets::BakedFont;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, Render, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;

/// A single selectable row representing one application choice.
#[ui::widget]
pub struct AppRow {
    /// The app-id this row represents (None = hidden slot).
    pub app_id: Option<String>,
    pub is_selected: bool,
    #[widget(child)]
    pub div: Div<(Text, Text)>,
}

impl AppRow {
    pub fn new(font: &'static BakedFont) -> Self {
        let style = Style {
            display: Display::None, // hidden until populated
            size: Size {
                width: percent(1.0_f32),
                height: length(52.0_f32),
            },
            margin: taffy::Rect {
                left: length(0.0_f32),
                right: length(0.0_f32),
                top: length(3.0_f32),
                bottom: length(3.0_f32),
            },
            ..Default::default()
        };

        let mut icon_text = Text::new(Style {
            min_size: Size {
                width: length(28.0_f32),
                height: auto(),
            },
            ..Default::default()
        });
        icon_text.font = Some(font);
        icon_text.text = "◻ ".to_string();
        icon_text.color = Color::rgb(0.4, 0.65, 1.0);
        icon_text.z = 0.95;

        let mut name_text = Text::new(Style::default());
        name_text.font = Some(font);
        name_text.text = String::new();
        name_text.color = Color::WHITE;
        name_text.z = 0.95;

        let div_style = Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            size: Size {
                width: percent(1.0_f32),
                height: percent(1.0_f32),
            },
            align_items: Some(AlignItems::Center),
            padding: taffy::Rect {
                left: length(14.0_f32),
                right: length(14.0_f32),
                top: length(0.0_f32),
                bottom: length(0.0_f32),
            },
            gap: Size {
                width: length(10.0_f32),
                height: length(0.0_f32),
            },
            ..Default::default()
        };

        let mut div = Div::new(div_style, (icon_text, name_text));
        div.color = Color::rgb(0.05, 0.05, 0.07);
        div.border_radius = 8.0;
        div.border_thickness = 1.0;
        div.border_color = Color::rgb(0.10, 0.10, 0.13);
        div.z = 0.8;

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style,
            app_id: None,
            is_selected: false,
            div,
        }
    }

    pub fn update(&mut self, tree: &mut WidgetTree, app_id: Option<String>, is_selected: bool) {
        self.is_selected = is_selected;
        self.app_id = app_id.clone();

        let mut style = self.style.clone();
        if let Some(ref id) = app_id {
            style.display = Display::Flex;

            // Pretty-print: take the last dot-segment as the display name.
            let display_name = id
                .rsplit('.')
                .next()
                .map(|s| {
                    let mut c = s.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().to_string() + c.as_str(),
                    }
                })
                .unwrap_or_else(|| id.clone());

            self.div.children.1.set_text(tree, display_name);

            if is_selected {
                self.div.color = Color::rgb(0.07, 0.28, 0.65);
                self.div.border_color = Color::rgb(0.15, 0.50, 1.0);
                self.div.children.0.text = "◼ ".to_string();
                self.div.children.0.color = Color::rgb(0.35, 0.75, 1.0);
            } else {
                self.div.color = Color::rgb(0.05, 0.05, 0.07);
                self.div.border_color = Color::rgb(0.10, 0.10, 0.13);
                self.div.children.0.text = "◻ ".to_string();
                self.div.children.0.color = Color::rgb(0.40, 0.65, 1.0);
            }
        } else {
            style.display = Display::None;
            self.div.children.0.set_text(tree, String::new());
            self.div.children.1.set_text(tree, String::new());
        }
        self.set_style(tree, style);
    }
}

impl Render for AppRow {
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

pub struct AppRows(pub Vec<AppRow>);

impl WidgetList for AppRows {
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
