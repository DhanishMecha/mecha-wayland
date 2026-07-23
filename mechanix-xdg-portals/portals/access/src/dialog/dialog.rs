use crate::backend::{AccessOutcome, AccessResponse, RequestHandle};
use assets::BakedFont;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;

use portal_core::atlas;
use portal_core::widgets::Button;

use super::PENDING_DIALOG;

pub type HeaderDiv = Div<(Text, Text, Text)>;
pub type ButtonRowDiv = Div<(Button, Button)>;
pub type ModalDiv = Div<(HeaderDiv, Text, ButtonRowDiv)>;
pub type RootDiv = Div<(ModalDiv,)>;

pub struct AccessDialogUi {
    handle: RequestHandle,
    root: RootDiv,
    deny_rect: utils::Rect,
    allow_rect: utils::Rect,
    deny_id: Option<u64>,
    allow_id: Option<u64>,
}

impl AccessDialogUi {
    pub fn new(
        handle: RequestHandle,
        app_id: String,
        title: String,
        subtitle: String,
        body: String,
        icon: Option<String>,
        deny_label: Option<String>,
        grant_label: Option<String>,
        choices: Option<Vec<(String, String, Vec<(String, String)>, String)>>,
    ) -> Self {
        let font_24 = &atlas::UI_FONT_INTER_24;
        let font_16 = &atlas::UI_FONT_INTER_16;

        let root = make_root(
            font_24,
            font_16,
            &app_id,
            &title,
            &subtitle,
            &body,
            &icon,
            deny_label.as_deref(),
            grant_label.as_deref(),
            &choices,
        );

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

impl WidgetList for AccessDialogUi {
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
            println!("[access-ui] Allow clicked.");
            PENDING_DIALOG.with(|cell| {
                cell.set(Some(AccessResponse {
                    handle: self.handle.clone(),
                    outcome: AccessOutcome::Granted,
                }));
            });
            return;
        }

        if self.deny_rect != utils::Rect::ZERO && interactivity.is_clicked(self.deny_rect) {
            println!("[access-ui] Deny clicked.");
            PENDING_DIALOG.with(|cell| {
                cell.set(Some(AccessResponse {
                    handle: self.handle.clone(),
                    outcome: AccessOutcome::Denied,
                }));
            });
            return;
        }
    }

    fn wants_input(&self) -> bool {
        true
    }
}

// --- Layout helpers ----------------------------------------------------------
fn make_root(
    font_24: &'static BakedFont,
    font_16: &'static BakedFont,
    app_id: &str,
    title: &str,
    subtitle: &str,
    body: &str,
    _icon: &Option<String>, // TODO: Load icon dynamically from options.icon instead of hardcoded fallback
    deny_label: Option<&str>,
    grant_label: Option<&str>,
    _choices: &Option<Vec<(String, String, Vec<(String, String)>, String)>>, // TODO: Requires custom checkbox/radio UI widgets
) -> RootDiv {
    // Header section: icon placeholder + title + subtitle
    let mut icon_text = Text::new(Style::default());
    icon_text.font = Some(font_24);
    icon_text.text = "🔒".to_string();
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
    title_text.text = title.to_string();
    title_text.color = Color::WHITE;
    title_text.z = 0.95;

    let display_subtitle = if !app_id.is_empty() {
        if subtitle.is_empty() {
            format!("Requested by {app_id}")
        } else {
            format!("{subtitle} • {app_id}")
        }
    } else {
        subtitle.to_string()
    };

    let mut subtitle_text = Text::new(Style::default());
    subtitle_text.font = Some(font_16);
    subtitle_text.text = display_subtitle;
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
            top: length(16.0_f32),
            bottom: length(16.0_f32),
        },
        ..Default::default()
    };
    let header = Div::new(header_style, (icon_text, title_text, subtitle_text));

    // Body text
    let mut body_text = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(24.0_f32),
        },
        ..Default::default()
    });
    body_text.font = Some(font_16);
    body_text.text = body.to_string();
    body_text.color = Color::rgb(0.75, 0.75, 0.82);
    body_text.z = 0.95;

    // Deny / Allow buttons
    let deny_str = deny_label.unwrap_or("Deny");
    let grant_str = grant_label.unwrap_or("Allow");

    let mut deny_btn = Button::new(deny_str);
    deny_btn.div.color = Color::rgb(0.12, 0.12, 0.15);
    deny_btn.div.border_radius = 10.0;
    deny_btn.div.border_color = Color::rgb(0.22, 0.22, 0.27);
    deny_btn.div.border_thickness = 1.5;
    deny_btn.div.z = 1.0;
    deny_btn.div.children.font = Some(font_16);
    deny_btn.div.children.color = Color::rgb(0.85, 0.85, 0.9);
    deny_btn.div.children.z = 0.5;

    let mut allow_btn = Button::new(grant_str);
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
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: length(50.0_f32),
        },
        ..Default::default()
    };
    let button_row = Div::new(button_row_style, (deny_btn, allow_btn));

    // Modal card
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
            top: length(32.0_f32),
            bottom: length(24.0_f32),
        },
        ..Default::default()
    };
    let mut modal = Div::new(modal_style, (header, body_text, button_row));
    modal.color = Color::rgb(0.07, 0.07, 0.09);
    modal.border_color = Color::rgb(0.14, 0.14, 0.18);
    modal.border_radius = 16.0;
    modal.border_thickness = 1.0;
    modal.z = 0.2;

    // Full-screen dimmed backdrop
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
