use assets::BakedFont;
use interactivity::InteractivityState;
use std::path::PathBuf;
use taffy::prelude::*;
use ui::widgets::{ Div, Text };
use ui::{ Point, Render, RenderCommand, Widget, WidgetTree };
use utils::Color;

#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

// --- FileRow Widget ---------------------------------------------------------
#[ui::widget]
pub struct FileRow {
    pub name: String,
    pub path: Option<PathBuf>,
    pub is_dir: bool,
    pub is_selected: bool,
    #[widget(child)]
    pub div: Div<(Text, Text)>,
}

impl FileRow {
    pub fn new(font: &'static BakedFont) -> Self {
        let style = Style {
            display: Display::Flex,
            size: Size {
                width: percent(1.0_f32),
                height: length(36.0_f32),
            },
            margin: taffy::Rect {
                left: length(0.0_f32),
                right: length(0.0_f32),
                top: length(2.0_f32),
                bottom: length(2.0_f32),
            },
            ..Default::default()
        };

        let mut type_text = Text::new(Style::default());
        type_text.font = Some(font);
        type_text.text = String::new();
        type_text.color = Color::rgb(0.3, 0.6, 1.0);
        type_text.z = 0.95;

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
                left: length(12.0_f32),
                right: length(12.0_f32),
                top: length(0.0_f32),
                bottom: length(0.0_f32),
            },
            ..Default::default()
        };

        let mut div = Div::new(div_style, (type_text, name_text));
        div.color = Color::rgb(0.04, 0.04, 0.05);
        div.border_radius = 6.0;
        div.border_thickness = 1.0;
        div.border_color = Color::rgb(0.1, 0.1, 0.12);
        div.z = 0.8;

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style,
            name: String::new(),
            path: None,
            is_dir: false,
            is_selected: false,
            div,
        }
    }

    pub fn update(&mut self, tree: &mut WidgetTree, entry: Option<FileEntry>, is_selected: bool) {
        self.is_selected = is_selected;

        let mut style = self.style.clone();
        if let Some(e) = entry {
            style.display = Display::Flex;
            self.name = e.name.clone();
            self.path = Some(e.path);
            self.is_dir = e.is_dir;

            let (icon, icon_color) = if e.is_dir {
                if e.name == ".." {
                    ("^ ", Color::rgb(0.3, 0.65, 1.0))
                } else {
                    ("> ", Color::rgb(0.3, 0.65, 1.0))
                }
            } else {
                ("  ", Color::rgb(0.7, 0.7, 0.8))
            };

            self.div.children.0.set_text(tree, icon.to_string());
            self.div.children.0.color = icon_color;
            self.div.children.1.set_text(tree, e.name);

            if is_selected {
                self.div.color = Color::rgb(0.08, 0.25, 0.6);
                self.div.border_color = Color::rgb(0.15, 0.45, 0.9);
            } else {
                self.div.color = Color::rgb(0.04, 0.04, 0.05);
                self.div.border_color = Color::rgb(0.1, 0.1, 0.12);
            }
        } else {
            style.display = Display::None;
            self.name = String::new();
            self.path = None;
            self.is_dir = false;
            self.div.children.0.set_text(tree, String::new());
            self.div.children.1.set_text(tree, String::new());
        }

        self.set_style(tree, style);
    }
}

impl ui::Render for FileRow {
    fn render(&self, layout: &taffy::Layout, abs_pos: Point) -> Vec<RenderCommand> {
        let id: u64 = self.node_id.into();
        vec![RenderCommand::RegisterHitArea {
            id,
            rect: utils::Rect::new(abs_pos.x(), abs_pos.y(), layout.size.width, layout.size.height),
        }]
    }
}

// --- Wrapper for a dynamic list of FileRows ---------------------------------
pub struct FileRows(pub Vec<FileRow>);

impl ui::WidgetList for FileRows {
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

    fn on_event(&mut self, interactivity: &InteractivityState, tree: &mut WidgetTree) -> bool {
        let mut handled = false;
        for w in self.0.iter_mut() {
            if Widget::on_event(w, interactivity, tree) {
                handled = true;
            }
        }
        handled
    }
}
