use crate::backend::types::{RequestHandle, WallpaperOutcome, WallpaperResponse};
use assets::BakedFont;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;

use portal_core::atlas;
use portal_core::widgets::Button;

pub type HeaderDiv = Div<(Text, Text, Text)>;
pub type ButtonRowDiv = Div<(Button, Button)>;
pub type BodyCardDiv = Div<(Text, Text)>;
pub type ModalDiv = Div<(HeaderDiv, BodyCardDiv, ButtonRowDiv)>;
pub type RootDiv = Div<(ModalDiv,)>;

pub struct WallpaperDialogUi {
    handle: RequestHandle,
    root: RootDiv,
    cancel_rect: utils::Rect,
    confirm_rect: utils::Rect,
    cancel_id: Option<u64>,
    confirm_id: Option<u64>,
}

impl WallpaperDialogUi {
    pub fn new(
        handle: RequestHandle,
        app_id: String,
        title: String,
        body: String,
        uri: String,
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
            &body,
            &uri,
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

impl WidgetList for WallpaperDialogUi {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let child_ids = vec![self.root.build_tree(tree)];
        self.cancel_id = Some(self.root.children.0.children.2.children.0.node_id().into());
        self.confirm_id = Some(self.root.children.0.children.2.children.1.node_id().into());
        child_ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let commands = self.root.render_children(tree, parent_abs);
        let cancel_id = self.cancel_id.unwrap_or(0);
        let confirm_id = self.confirm_id.unwrap_or(0);
        for cmd in &commands {
            if let RenderCommand::RegisterHitArea { id, rect } = cmd {
                let id_val = *id;
                let rect_val = *rect;
                if id_val == cancel_id {
                    self.cancel_rect = rect_val;
                } else if id_val == confirm_id {
                    self.confirm_rect = rect_val;
                }
            }
        }
        commands
    }

    fn on_event(&mut self, ctx: &mut EventCtx) {
        let interactivity = ctx.interactivity();
        if self.confirm_rect != utils::Rect::ZERO && interactivity.is_clicked(self.confirm_rect) {
            println!("[wallpaper-ui] Confirmed for handle={}", self.handle);
            ctx.dispatch(WallpaperResponse {
                handle: self.handle.clone(),
                outcome: WallpaperOutcome::Granted,
            });
            return;
        }

        if self.cancel_rect != utils::Rect::ZERO && interactivity.is_clicked(self.cancel_rect) {
            println!("[wallpaper-ui] Cancelled for handle={}", self.handle);
            ctx.dispatch(WallpaperResponse {
                handle: self.handle.clone(),
                outcome: WallpaperOutcome::Denied,
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
    body: &str,
    uri: &str,
    icon: &str,
    cancel_label: &str,
    confirm_label: &str,
) -> RootDiv {
    // Icon
    let mut icon_text = Text::new(Style::default());
    icon_text.font = Some(font_24);
    icon_text.text = icon.to_string();
    icon_text.color = Color::rgb(0.55, 0.75, 1.0);
    icon_text.z = 0.95;

    // Title
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

    // Subtitle — show requesting app id
    let subtitle_str = if app_id.is_empty() {
        "An application wants to set the desktop background.".to_string()
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
            top: length(8.0_f32),
            bottom: length(8.0_f32),
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
            bottom: length(12.0_f32),
        },
        ..Default::default()
    });
    body_text.font = Some(font_16);
    body_text.text = format!("Target: {body}");
    body_text.color = Color::rgb(0.75, 0.75, 0.82);
    body_text.z = 0.95;

    // URI text
    let mut uri_text = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(0.0_f32),
        },
        ..Default::default()
    });
    uri_text.font = Some(font_16);
    uri_text.text = format!("URI: {uri}");
    uri_text.color = Color::rgb(0.45, 0.7, 0.95);
    uri_text.z = 0.95;

    // Body card
    let body_card_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        padding: taffy::Rect {
            left: length(12.0_f32),
            right: length(12.0_f32),
            top: length(12.0_f32),
            bottom: length(12.0_f32),
        },
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(12.0_f32),
        },
        ..Default::default()
    };
    let mut body_card = Div::new(body_card_style, (body_text, uri_text));
    body_card.color = Color::rgb(0.10, 0.10, 0.12);
    body_card.border_radius = 10.0;
    body_card.border_color = Color::rgb(0.16, 0.16, 0.20);
    body_card.border_thickness = 1.0;
    body_card.z = 0.9;

    // Cancel button (left, neutral)
    let mut cancel_btn = Button::new(cancel_label);
    cancel_btn.div.color = Color::rgb(0.12, 0.12, 0.15);
    cancel_btn.div.border_radius = 10.0;
    cancel_btn.div.border_color = Color::rgb(0.22, 0.22, 0.27);
    cancel_btn.div.border_thickness = 1.5;
    cancel_btn.div.z = 1.0;
    cancel_btn.div.children.font = Some(font_16);
    cancel_btn.div.children.color = Color::rgb(0.85, 0.85, 0.9);
    cancel_btn.div.children.z = 0.5;

    // Confirm button (right, accent)
    let mut confirm_btn = Button::new(confirm_label);
    confirm_btn.div.color = Color::rgb(0.1, 0.45, 0.9);
    confirm_btn.div.border_radius = 10.0;
    confirm_btn.div.border_color = Color::rgb(0.15, 0.55, 1.0);
    confirm_btn.div.border_thickness = 1.5;
    confirm_btn.div.z = 1.0;
    confirm_btn.div.children.font = Some(font_16);
    confirm_btn.div.children.color = Color::WHITE;
    confirm_btn.div.children.z = 0.5;

    let button_row_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        justify_content: Some(JustifyContent::SpaceBetween),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: length(44.0_f32),
        },
        ..Default::default()
    };
    let button_row = Div::new(button_row_style, (cancel_btn, confirm_btn));

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
            left: length(20.0_f32),
            right: length(20.0_f32),
            top: length(16.0_f32),
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
