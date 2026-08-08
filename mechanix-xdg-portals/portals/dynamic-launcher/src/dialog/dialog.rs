use crate::backend::{DynamicLauncherOutcome, DynamicLauncherResponse, RequestHandle};
use assets::BakedFont;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;

use portal_core::atlas;
use portal_core::widgets::Button;

pub type HeaderSubtitleDiv = Div<(Text, Text)>;
pub type HeaderDiv = Div<(Text, Text, HeaderSubtitleDiv)>;
pub type BodyDiv = Div<(Text, Text, Text)>;
pub type ButtonRowDiv = Div<(Button, Button)>;
pub type ModalDiv = Div<(HeaderDiv, BodyDiv, ButtonRowDiv)>;
pub type RootDiv = Div<(ModalDiv,)>;

pub struct DynamicLauncherDialogUi {
    handle: RequestHandle,
    root: RootDiv,
    deny_rect: utils::Rect,
    allow_rect: utils::Rect,
    deny_id: Option<u64>,
    allow_id: Option<u64>,
}

impl DynamicLauncherDialogUi {
    pub fn new(
        handle: RequestHandle,
        app_id: String,
        name: String,
        launcher_type: u32,
        target: Option<String>,
    ) -> Self {
        let font_24 = &atlas::UI_FONT_INTER_24;
        let font_16 = &atlas::UI_FONT_INTER_16;

        let root = make_root(font_24, font_16, &app_id, &name, launcher_type, &target);

        Self {
            handle,
            root,
            deny_rect: utils::Rect::ZERO,
            allow_rect: utils::Rect::ZERO,
            deny_id: None,
            allow_id: None,
        }
    }
}

impl WidgetList for DynamicLauncherDialogUi {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let child_ids = vec![self.root.build_tree(tree)];
        self.deny_id = Some(self.root.children.0.children.2.children.0.node_id().into());
        self.allow_id = Some(self.root.children.0.children.2.children.1.node_id().into());
        child_ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let commands = self.root.render_children(tree, parent_abs);
        let deny_id = self.deny_id.unwrap_or(0);
        let allow_id = self.allow_id.unwrap_or(0);
        for cmd in &commands {
            if let RenderCommand::RegisterHitArea { id, rect } = cmd {
                if *id == deny_id {
                    self.deny_rect = *rect;
                } else if *id == allow_id {
                    self.allow_rect = *rect;
                }
            }
        }
        commands
    }

    fn on_event(&mut self, ctx: &mut EventCtx) {
        let interactivity = ctx.interactivity();

        if self.allow_rect != utils::Rect::ZERO && interactivity.is_clicked(self.allow_rect) {
            println!("[dynamic-launcher-ui] Allow clicked");
            ctx.dispatch(DynamicLauncherResponse {
                handle: self.handle.clone(),
                outcome: DynamicLauncherOutcome::Granted,
            });
            return;
        }

        if self.deny_rect != utils::Rect::ZERO && interactivity.is_clicked(self.deny_rect) {
            println!("[dynamic-launcher-ui] Deny clicked");
            ctx.dispatch(DynamicLauncherResponse {
                handle: self.handle.clone(),
                outcome: DynamicLauncherOutcome::Denied,
            });
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
    name: &str,
    launcher_type: u32,
    target: &Option<String>,
) -> RootDiv {
    let mut icon_text = Text::new(Style::default());
    icon_text.font = Some(font_24);
    icon_text.text = "🚀".to_string();
    icon_text.color = Color::rgb(0.55, 0.75, 1.0);
    icon_text.z = 0.95;

    let mut title_text = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(8.0_f32),
            bottom: length(4.0_f32),
        },
        ..Default::default()
    });
    title_text.font = Some(font_24);
    title_text.text = "Add Launcher?".to_string();
    title_text.color = Color::WHITE;
    title_text.z = 0.95;

    let subtitle_str = if app_id.is_empty() {
        "An application wants to create a shortcut.".to_string()
    } else {
        format!("{app_id} wants to create a shortcut.")
    };
    let mut subtitle_text = Text::new(Style::default());
    subtitle_text.font = Some(font_16);
    subtitle_text.text = subtitle_str;
    subtitle_text.color = Color::rgb(0.65, 0.65, 0.75);
    subtitle_text.z = 0.95;

    let mut type_str_text = Text::new(Style::default());
    type_str_text.font = Some(font_16);
    type_str_text.text = format!(
        "Type: {}",
        if launcher_type == 2 { "Webapp" } else { "Application" }
    );
    type_str_text.color = Color::rgb(0.55, 0.55, 0.65);
    type_str_text.z = 0.95;

    let subtitle_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        ..Default::default()
    };
    let subtitle_container = Div::new(subtitle_style, (subtitle_text, type_str_text));

    let header_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        ..Default::default()
    };
    let header = Div::new(header_style, (icon_text, title_text, subtitle_container));

    let body_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(12.0_f32),
            bottom: length(16.0_f32),
        },
        ..Default::default()
    };

    let mut name_text = Text::new(Style::default());
    name_text.font = Some(font_16);
    name_text.text = format!("Name: {}", name);
    name_text.color = Color::rgb(0.85, 0.85, 0.9);
    name_text.z = 0.95;

    let mut target_text = Text::new(Style::default());
    target_text.font = Some(font_16);
    target_text.text = if let Some(t) = target {
        format!("Target URI: {}", t)
    } else {
        "".to_string()
    };
    target_text.color = Color::rgb(0.85, 0.85, 0.9);
    target_text.z = 0.95;

    let mut placeholder_text = Text::new(Style::default());
    placeholder_text.font = Some(font_16);
    placeholder_text.text = "".to_string();
    placeholder_text.color = Color::rgb(0.85, 0.85, 0.9);
    placeholder_text.z = 0.95;

    let body = Div::new(body_style, (name_text, target_text, placeholder_text));

    let mut deny_btn = Button::new("Cancel");
    deny_btn.div.color = Color::rgb(0.12, 0.12, 0.15);
    deny_btn.div.border_radius = 10.0;
    deny_btn.div.border_color = Color::rgb(0.22, 0.22, 0.27);
    deny_btn.div.border_thickness = 1.5;
    deny_btn.div.z = 1.0;
    deny_btn.div.children.font = Some(font_16);
    deny_btn.div.children.color = Color::rgb(0.85, 0.85, 0.9);
    deny_btn.div.children.z = 0.5;

    let mut allow_btn = Button::new("Install");
    allow_btn.div.color = Color::rgb(0.1, 0.45, 0.9);
    allow_btn.div.border_radius = 10.0;
    allow_btn.div.border_color = Color::rgb(0.15, 0.55, 1.0);
    allow_btn.div.border_thickness = 1.5;
    allow_btn.div.z = 1.0;
    allow_btn.div.children.font = Some(font_16);
    allow_btn.div.children.color = Color::WHITE;
    allow_btn.div.children.z = 0.5;

    let button_row_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        justify_content: Some(JustifyContent::SpaceBetween),
        size: Size {
            width: percent(1.0_f32),
            height: length(50.0_f32),
        },
        ..Default::default()
    };
    let button_row = Div::new(button_row_style, (deny_btn, allow_btn));

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
            left: length(24.0_f32),
            right: length(24.0_f32),
            top: length(20.0_f32),
            bottom: length(20.0_f32),
        },
        ..Default::default()
    };
    let mut modal = Div::new(modal_style, (header, body, button_row));
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
