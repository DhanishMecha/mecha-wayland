use crate::backend::types::{NotificationButton, NotificationInfo, NotificationResponse};
use assets::BakedFont;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, Render, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;

use portal_core::atlas;

#[derive(Debug, Clone, Copy)]
pub enum NotifButtonStyle {
    Primary,
    Secondary,
    Close,
}

#[ui::widget]
pub struct NotifButton {
    #[widget(child)]
    pub div: Div<Text>,
}

impl NotifButton {
    pub fn new(
        label: &str,
        width: f32,
        height: f32,
        btn_style: NotifButtonStyle,
        visible: bool,
    ) -> Self {
        let style = Style {
            display: if visible {
                Display::Flex
            } else {
                Display::None
            },
            size: Size {
                width: length(width),
                height: length(height),
            },
            ..Default::default()
        };
        let div_style = Style {
            display: Display::Flex,
            size: Size {
                width: percent(1.0_f32),
                height: percent(1.0_f32),
            },
            align_items: Some(AlignItems::Center),
            justify_content: Some(JustifyContent::Center),
            ..Default::default()
        };
        let mut text = Text::new(Style::default());
        text.text = label.to_string();
        let mut div = Div::new(div_style, text);

        match btn_style {
            NotifButtonStyle::Primary => {
                div.color = Color::rgb(0.1, 0.45, 0.9);
                div.border_radius = 6.0;
                div.border_color = Color::rgb(0.15, 0.55, 1.0);
                div.border_thickness = 1.0;
                div.z = 1.0;
                div.children.font = Some(&atlas::UI_FONT_INTER_16);
                div.children.color = Color::WHITE;
                div.children.z = 0.5;
            }
            NotifButtonStyle::Secondary => {
                div.color = Color::rgb(0.12, 0.12, 0.15);
                div.border_radius = 6.0;
                div.border_color = Color::rgb(0.22, 0.22, 0.27);
                div.border_thickness = 1.0;
                div.z = 1.0;
                div.children.font = Some(&atlas::UI_FONT_INTER_16);
                div.children.color = Color::rgb(0.85, 0.85, 0.9);
                div.children.z = 0.5;
            }
            NotifButtonStyle::Close => {
                div.color = Color::TRANSPARENT;
                div.z = 1.0;
                div.children.font = Some(&atlas::UI_FONT_INTER_16);
                div.children.color = Color::rgb(0.55, 0.55, 0.65);
                div.children.z = 0.5;
            }
        }

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style,
            div,
        }
    }
}

impl Render for NotifButton {
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

#[ui::widget]
pub struct BannerContent {
    #[widget(child)]
    pub div: Div<(Text, Text)>,
}

impl BannerContent {
    pub fn new(style: Style, title_text: Text, body_text: Text) -> Self {
        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style,
            div: Div::new(Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                justify_content: Some(JustifyContent::FlexStart),
                align_items: Some(AlignItems::FlexStart),
                size: Size {
                    width: percent(1.0_f32),
                    height: percent(1.0_f32),
                },
                ..Default::default()
            }, (title_text, body_text)),
        }
    }
}

impl Render for BannerContent {
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

pub type BannerHeaderDiv = Div<(Text, NotifButton)>;
pub type BannerContentDiv = BannerContent;
pub type BannerButtonRowDiv = Div<(NotifButton, NotifButton)>;
pub type BannerDiv = Div<(BannerHeaderDiv, BannerContentDiv, BannerButtonRowDiv)>;
pub type RootDiv = Div<(BannerDiv,)>;

pub struct NotificationBannerUi {
    app_id: String,
    id: String,
    default_action: Option<String>,
    default_action_target: Option<zbus::zvariant::OwnedValue>,
    buttons: Vec<NotificationButton>,

    root: RootDiv,

    close_id: Option<u64>,
    content_id: Option<u64>,
    btn1_id: Option<u64>,
    btn2_id: Option<u64>,

    close_rect: utils::Rect,
    content_rect: utils::Rect,
    btn1_rect: utils::Rect,
    btn2_rect: utils::Rect,
}

impl NotificationBannerUi {
    pub fn new(app_id: String, id: String, info: NotificationInfo) -> Self {
        let font_16 = &atlas::UI_FONT_INTER_16;

        // TODO: icon render is not supported
        let root = make_root(font_16, &app_id, &info.title, &info.body, &info.buttons);

        Self {
            app_id,
            id,
            default_action: info.default_action,
            default_action_target: info.default_action_target,
            buttons: info.buttons,
            root,
            close_id: None,
            content_id: None,
            btn1_id: None,
            btn2_id: None,
            close_rect: utils::Rect::ZERO,
            content_rect: utils::Rect::ZERO,
            btn1_rect: utils::Rect::ZERO,
            btn2_rect: utils::Rect::ZERO,
        }
    }
}

impl WidgetList for NotificationBannerUi {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let child_ids = vec![self.root.build_tree(tree)];

        self.close_id = Some(self.root.children.0.children.0.children.1.node_id().into());
        self.content_id = Some(self.root.children.0.children.1.node_id().into());
        self.btn1_id = Some(self.root.children.0.children.2.children.0.node_id().into());
        self.btn2_id = Some(self.root.children.0.children.2.children.1.node_id().into());

        child_ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let commands = self.root.render_children(tree, parent_abs);

        let close_id = self.close_id.unwrap_or(0);
        let content_id = self.content_id.unwrap_or(0);
        let btn1_id = self.btn1_id.unwrap_or(0);
        let btn2_id = self.btn2_id.unwrap_or(0);

        for cmd in &commands {
            if let RenderCommand::RegisterHitArea { id, rect } = cmd {
                if *id == close_id {
                    self.close_rect = *rect;
                } else if *id == content_id {
                    self.content_rect = *rect;
                } else if *id == btn1_id {
                    self.btn1_rect = *rect;
                } else if *id == btn2_id {
                    self.btn2_rect = *rect;
                }
            }
        }
        commands
    }

    fn on_event(&mut self, ctx: &mut EventCtx) {
        let interactivity = ctx.interactivity();

        // 1. Close button
        if self.close_rect != utils::Rect::ZERO && interactivity.is_clicked(self.close_rect) {
            println!("[notification-ui] Close button clicked");
            ctx.dispatch(NotificationResponse {
                app_id: self.app_id.clone(),
                id: self.id.clone(),
                action: "dismissed".to_string(),
                parameter: vec![],
            });
            return;
        }

        // 2. Action Button 1
        if self.btn1_rect != utils::Rect::ZERO && interactivity.is_clicked(self.btn1_rect) {
            if let Some(btn) = self.buttons.get(0) {
                println!(
                    "[notification-ui] Action Button 1 clicked: action={}",
                    btn.action
                );
                ctx.dispatch(NotificationResponse {
                    app_id: self.app_id.clone(),
                    id: self.id.clone(),
                    action: btn.action.clone(),
                    parameter: btn.target.clone().map(|t| vec![t]).unwrap_or_default(),
                });
                return;
            }
        }

        // 3. Action Button 2
        if self.btn2_rect != utils::Rect::ZERO && interactivity.is_clicked(self.btn2_rect) {
            if let Some(btn) = self.buttons.get(1) {
                println!(
                    "[notification-ui] Action Button 2 clicked: action={}",
                    btn.action
                );
                ctx.dispatch(NotificationResponse {
                    app_id: self.app_id.clone(),
                    id: self.id.clone(),
                    action: btn.action.clone(),
                    parameter: btn.target.clone().map(|t| vec![t]).unwrap_or_default(),
                });
                return;
            }
        }

        // 4. Default Action (body click)
        if self.content_rect != utils::Rect::ZERO && interactivity.is_clicked(self.content_rect) {
            if let Some(ref action) = self.default_action {
                println!(
                    "[notification-ui] Default body action clicked: action={}",
                    action
                );
                ctx.dispatch(NotificationResponse {
                    app_id: self.app_id.clone(),
                    id: self.id.clone(),
                    action: action.clone(),
                    parameter: self
                        .default_action_target
                        .clone()
                        .map(|t| vec![t])
                        .unwrap_or_default(),
                });
            } else {
                println!("[notification-ui] No default action. Dismissing banner on body click.");
                ctx.dispatch(NotificationResponse {
                    app_id: self.app_id.clone(),
                    id: self.id.clone(),
                    action: "dismissed".to_string(),
                    parameter: vec![],
                });
            }
            return;
        }
    }

    fn wants_input(&self) -> bool {
        true
    }
}

fn make_root(
    font_16: &'static BakedFont,
    app_id: &str,
    title: &str,
    body: &str,
    buttons: &[NotificationButton],
) -> RootDiv {
    // Header
    let header_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        justify_content: Some(JustifyContent::SpaceBetween),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: length(24.0_f32),
        },
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(8.0_f32),
        },
        ..Default::default()
    };

    let mut app_name_text = Text::new(Style::default());
    app_name_text.font = Some(font_16);
    app_name_text.text = if app_id.is_empty() {
        "Notification".to_string()
    } else {
        app_id.to_string()
    };
    app_name_text.color = Color::rgb(0.65, 0.65, 0.75);
    app_name_text.z = 0.95;

    let close_btn = NotifButton::new("✕", 24.0, 24.0, NotifButtonStyle::Close, true);
    let header = Div::new(header_style, (app_name_text, close_btn));

    // Content
    let content_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::FlexStart),
        align_items: Some(AlignItems::FlexStart),
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        ..Default::default()
    };

    let mut title_text = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(4.0_f32),
        },
        ..Default::default()
    });
    title_text.font = Some(font_16);
    title_text.text = title.to_string();
    title_text.color = Color::WHITE;
    title_text.z = 0.95;

    let mut body_text = Text::new(Style::default());
    body_text.font = Some(font_16);
    body_text.text = body.to_string();
    body_text.color = Color::rgb(0.75, 0.75, 0.82);
    body_text.z = 0.95;

    let content = BannerContent::new(content_style, title_text, body_text);

    // Button Row
    let button_row_style = Style {
        display: if buttons.is_empty() {
            Display::None
        } else {
            Display::Flex
        },
        flex_direction: FlexDirection::Row,
        justify_content: Some(JustifyContent::FlexEnd),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: length(36.0_f32),
        },
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(16.0_f32),
            bottom: length(0.0_f32),
        },
        gap: Size {
            width: length(12.0_f32),
            height: length(0.0_f32),
        },
        ..Default::default()
    };

    let btn1_label = buttons.get(0).map(|b| b.label.as_str()).unwrap_or("");
    let btn1_visible = buttons.get(0).is_some();
    let btn1 = NotifButton::new(
        btn1_label,
        120.0,
        36.0,
        NotifButtonStyle::Secondary,
        btn1_visible,
    );

    let btn2_label = buttons.get(1).map(|b| b.label.as_str()).unwrap_or("");
    let btn2_visible = buttons.get(1).is_some();
    let btn2 = NotifButton::new(
        btn2_label,
        120.0,
        36.0,
        NotifButtonStyle::Primary,
        btn2_visible,
    );

    let button_row = Div::new(button_row_style, (btn1, btn2));

    // Banner Container Card
    let justify_content = if buttons.is_empty() {
        JustifyContent::FlexStart
    } else {
        JustifyContent::SpaceBetween
    };

    let banner_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(justify_content),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: percent(1.0_f32),
        },
        padding: taffy::Rect {
            left: length(16.0_f32),
            right: length(16.0_f32),
            top: length(12.0_f32),
            bottom: length(12.0_f32),
        },
        ..Default::default()
    };

    let mut banner = Div::new(banner_style, (header, content, button_row));
    banner.color = Color::rgba(0.07, 0.07, 0.09, 0.95);
    banner.border_color = Color::rgb(0.14, 0.14, 0.18);
    banner.border_radius = 16.0;
    banner.border_thickness = 1.0;
    banner.z = 0.2;

    // Full-screen transparent backdrop
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
    let mut root = Div::new(root_style, (banner,));
    root.color = Color::BLACK;
    root.z = 0.1;

    root
}
