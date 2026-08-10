use crate::backend::types::{GlobalShortcutsOutcome, GlobalShortcutsResponse};
use assets::BakedFont;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;

use portal_core::atlas;
use portal_core::widgets::Button;

pub type HeaderDiv = Div<(Text, Text, Text)>;
pub type BodyCardDiv = Div<ShortcutsList>;
pub type ButtonRowDiv = Div<DynamicButtonRow>;
pub type ModalDiv = Div<(HeaderDiv, BodyCardDiv, ButtonRowDiv)>;
pub type RootDiv = Div<(ModalDiv,)>;

pub struct ShortcutsList {
    pub children: Vec<Text>,
}

impl ShortcutsList {
    pub fn new(children: Vec<Text>) -> Self {
        Self { children }
    }
}

impl WidgetList for ShortcutsList {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let mut ids = Vec::new();
        for child in &mut self.children {
            ids.push(child.build_tree(tree));
        }
        ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        for child in &mut self.children {
            let layout = tree.layout(child.node_id()).unwrap();
            commands.extend(child.render_node(layout, tree, parent_abs));
        }
        commands
    }
}

pub struct DynamicButtonRow {
    pub cancel_btn: Button,
    pub confirm_btn: Option<Button>,
}

impl DynamicButtonRow {
    pub fn new(cancel_btn: Button, confirm_btn: Option<Button>) -> Self {
        Self { cancel_btn, confirm_btn }
    }
}

impl WidgetList for DynamicButtonRow {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let mut ids = vec![self.cancel_btn.build_tree(tree)];
        if let Some(confirm) = &mut self.confirm_btn {
            ids.push(confirm.build_tree(tree));
        }
        ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let mut commands = self.cancel_btn.render_node(
            tree.layout(self.cancel_btn.node_id()).unwrap(),
            tree,
            parent_abs
        );
        if let Some(confirm) = &mut self.confirm_btn {
            commands.extend(confirm.render_node(
                tree.layout(confirm.node_id()).unwrap(),
                tree,
                parent_abs
            ));
        }
        commands
    }
}

pub struct GlobalShortcutsDialogUi {
    handle: String,
    root: RootDiv,
    cancel_rect: utils::Rect,
    confirm_rect: utils::Rect,
    cancel_id: Option<u64>,
    confirm_id: Option<u64>,
}

impl GlobalShortcutsDialogUi {
    pub fn new(
        handle: String,
        app_id: String,
        title: String,
        shortcuts: Vec<(String, String)>,
        icon: &str,
        cancel_label: &str,
        confirm_label: &str,
    ) -> Self {
        let font_24 = &atlas::UI_FONT_INTER_24;
        let font_16 = &atlas::UI_FONT_INTER_16;

        let root = make_root(
            font_24,
            font_16,
            &app_id,
            &title,
            shortcuts,
            icon,
            cancel_label,
            confirm_label,
        );

        Self {
            handle,
            root,
            cancel_rect: utils::Rect::ZERO,
            confirm_rect: utils::Rect::ZERO,
            cancel_id: None,
            confirm_id: None,
        }
    }
}

impl WidgetList for GlobalShortcutsDialogUi {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let child_ids = vec![self.root.build_tree(tree)];
        self.cancel_id = Some(self.root.children.0.children.2.children.cancel_btn.node_id().into());
        self.confirm_id = self.root.children.0.children.2.children.confirm_btn.as_ref().map(|btn| btn.node_id().into());
        child_ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let commands = self.root.render_children(tree, parent_abs);
        let cancel_id = self.cancel_id.unwrap_or(0);
        let confirm_id = self.confirm_id.unwrap_or(0);
        for cmd in &commands {
            if let RenderCommand::RegisterHitArea { id, rect } = cmd {
                if *id == cancel_id {
                    self.cancel_rect = *rect;
                } else if confirm_id > 0 && *id == confirm_id {
                    self.confirm_rect = *rect;
                }
            }
        }
        commands
    }

    fn on_event(&mut self, ctx: &mut EventCtx) {
        let interactivity = ctx.interactivity();
        if self.confirm_rect.width() > 0.0 && self.confirm_rect.height() > 0.0 && interactivity.is_clicked(self.confirm_rect) {
            println!("[global-shortcuts-ui] Confirmed for handle={}", self.handle);
            ctx.dispatch(GlobalShortcutsResponse {
                handle: self.handle.clone(),
                outcome: GlobalShortcutsOutcome::Granted,
            });
            return;
        }

        if self.cancel_rect.width() > 0.0 && self.cancel_rect.height() > 0.0 && interactivity.is_clicked(self.cancel_rect) {
            println!("[global-shortcuts-ui] Cancelled/Closed for handle={}", self.handle);
            ctx.dispatch(GlobalShortcutsResponse {
                handle: self.handle.clone(),
                outcome: GlobalShortcutsOutcome::Denied,
            });
            return;
        }
    }

    fn wants_input(&self) -> bool {
        true
    }
}

fn make_root(
    font_24: &'static BakedFont,
    font_16: &'static BakedFont,
    app_id: &str,
    title: &str,
    shortcuts: Vec<(String, String)>,
    icon: &str,
    cancel_label: &str,
    confirm_label: &str,
) -> RootDiv {
    let mut icon_text = Text::new(Style::default());
    icon_text.font = Some(font_24);
    icon_text.text = icon.to_string();
    icon_text.color = Color::rgb(0.55, 0.75, 1.0);
    icon_text.z = 0.95;

    let mut title_text = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(4.0_f32),
            bottom: length(2.0_f32),
        },
        ..Default::default()
    });
    title_text.font = Some(font_24);
    title_text.text = title.to_string();
    title_text.color = Color::WHITE;
    title_text.z = 0.95;

    let subtitle_str = if app_id.is_empty() {
        "An application wants to register global shortcuts.".to_string()
    } else {
        format!("Requested by {app_id}")
    };

    let mut subtitle_text = Text::new(Style::default());
    subtitle_text.font = Some(font_16);
    subtitle_text.text = subtitle_str;
    subtitle_text.color = Color::rgb(0.65, 0.65, 0.75);
    subtitle_text.z = 0.95;

    let header_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        align_items: Some(AlignItems::Center),
        justify_content: Some(JustifyContent::Center),
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        padding: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(10.0_f32),
            bottom: length(8.0_f32),
        },
        ..Default::default()
    };
    let header = Div::new(header_style, (icon_text, title_text, subtitle_text));

    // Build the dynamic list of Text widgets for shortcuts
    let mut children = Vec::new();
    for (description, preferred_trigger) in shortcuts {
        let mut text_widget = Text::new(Style {
            margin: taffy::Rect {
                left: length(0.0_f32),
                right: length(0.0_f32),
                top: length(0.0_f32),
                bottom: length(8.0_f32),
            },
            ..Default::default()
        });
        text_widget.font = Some(font_16);
        text_widget.text = format!("• {}: {}", description, preferred_trigger);
        text_widget.color = Color::rgb(0.75, 0.75, 0.82);
        text_widget.z = 0.95;
        children.push(text_widget);
    }
    let body_list = ShortcutsList::new(children);

    let body_card_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(12.0_f32),
        },
        ..Default::default()
    };
    let body_card = Div::new(body_card_style, body_list);

    let mut cancel_btn = Button::new(cancel_label);
    cancel_btn.div.color = Color::rgb(0.12, 0.12, 0.15);
    cancel_btn.div.border_radius = 10.0;
    cancel_btn.div.border_color = Color::rgb(0.22, 0.22, 0.27);
    cancel_btn.div.border_thickness = 1.5;
    cancel_btn.div.z = 1.0;
    cancel_btn.div.children.font = Some(font_16);
    cancel_btn.div.children.color = Color::rgb(0.85, 0.85, 0.9);
    cancel_btn.div.children.z = 0.5;

    let confirm_btn = if !confirm_label.is_empty() {
        let mut btn = Button::new(confirm_label);
        btn.div.color = Color::rgb(0.1, 0.45, 0.9);
        btn.div.border_radius = 10.0;
        btn.div.border_color = Color::rgb(0.15, 0.55, 1.0);
        btn.div.border_thickness = 1.5;
        btn.div.z = 1.0;
        btn.div.children.font = Some(font_16);
        btn.div.children.color = Color::WHITE;
        btn.div.children.z = 0.5;
        Some(btn)
    } else {
        None
    };

    let button_row_justify = if confirm_label.is_empty() {
        JustifyContent::Center
    } else {
        JustifyContent::SpaceBetween
    };

    let button_row_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        justify_content: Some(button_row_justify),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: length(50.0_f32),
        },
        ..Default::default()
    };
    let button_row_list = DynamicButtonRow::new(cancel_btn, confirm_btn);
    let button_row = Div::new(button_row_style, button_row_list);

    let modal_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::SpaceBetween),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: percent(1.0_f32),
        },
        padding: taffy::Rect {
            left: length(20.0_f32),
            right: length(20.0_f32),
            top: length(20.0_f32),
            bottom: length(16.0_f32),
        },
        ..Default::default()
    };
    let mut modal = Div::new(modal_style, (header, body_card, button_row));
    modal.color = Color::rgb(0.07, 0.07, 0.09);
    modal.border_color = Color::rgb(0.14, 0.14, 0.18);
    modal.border_radius = 16.0;
    modal.border_thickness = 1.0;
    modal.z = 0.2;

    let root_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: percent(1.0_f32),
        },
        ..Default::default()
    };
    let mut root = Div::new(root_style, (modal,));
    root.color = Color::rgba(0.0, 0.0, 0.0, 0.65);
    root.z = 0.1;

    root
}
