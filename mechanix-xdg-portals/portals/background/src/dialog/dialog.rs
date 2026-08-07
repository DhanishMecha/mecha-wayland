use crate::backend::{BackgroundOutcome, BackgroundResponse, RequestHandle};
use assets::BakedFont;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, Render, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;

use portal_core::atlas;

#[ui::widget]
pub struct BgButton {
    #[widget(child)]
    pub div: Div<Text>,
}

impl BgButton {
    pub fn new(label: &str, width: f32) -> Self {
        let style = Style {
            display: Display::Flex,
            size: Size {
                width: length(width),
                height: length(50.0_f32),
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
        let div = Div::new(div_style, text);

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style,
            div,
        }
    }
}

impl Render for BgButton {
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

pub type HeaderDiv = Div<(Text, Text, Text)>;
pub type ButtonRowDiv = Div<(BgButton, BgButton, BgButton)>;
pub type BodyCardDiv = Div<(Text, Text, Text)>;
pub type ModalDiv = Div<(HeaderDiv, BodyCardDiv, ButtonRowDiv)>;
pub type RootDiv = Div<(ModalDiv,)>;

pub struct BackgroundDialogUi {
    handle: RequestHandle,
    root: RootDiv,
    block_rect: utils::Rect,
    allow_once_rect: utils::Rect,
    allow_always_rect: utils::Rect,
    block_id: Option<u64>,
    allow_once_id: Option<u64>,
    allow_always_id: Option<u64>,
}

impl BackgroundDialogUi {
    pub fn new(
        handle: RequestHandle,
        app_id: String,
        name: String,
    ) -> Self {
        let font_24 = &atlas::UI_FONT_INTER_24;
        let font_16 = &atlas::UI_FONT_INTER_16;

        let root = make_root(
            font_24,
            font_16,
            &app_id,
            &name,
        );

        Self {
            handle,
            root,
            block_rect: utils::Rect::ZERO,
            allow_once_rect: utils::Rect::ZERO,
            allow_always_rect: utils::Rect::ZERO,
            block_id: None,
            allow_once_id: None,
            allow_always_id: None,
        }
    }
}

impl WidgetList for BackgroundDialogUi {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let child_ids = vec![self.root.build_tree(tree)];
        self.block_id = Some(self.root.children.0.children.2.children.0.node_id().into());
        self.allow_once_id = Some(self.root.children.0.children.2.children.1.node_id().into());
        self.allow_always_id = Some(self.root.children.0.children.2.children.2.node_id().into());
        child_ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let commands = self.root.render_children(tree, parent_abs);
        let block_id = self.block_id.unwrap_or(0);
        let allow_once_id = self.allow_once_id.unwrap_or(0);
        let allow_always_id = self.allow_always_id.unwrap_or(0);
        
        for cmd in &commands {
            if let RenderCommand::RegisterHitArea { id, rect } = cmd {
                if *id == block_id {
                    self.block_rect = *rect;
                } else if *id == allow_once_id {
                    self.allow_once_rect = *rect;
                } else if *id == allow_always_id {
                    self.allow_always_rect = *rect;
                }
            }
        }
        commands
    }

    fn on_event(&mut self, ctx: &mut EventCtx) {
        let interactivity = ctx.interactivity();
        
        if self.allow_always_rect != utils::Rect::ZERO && interactivity.is_clicked(self.allow_always_rect) {
            println!("[background-ui] Allow always clicked.");
            ctx.dispatch(BackgroundResponse {
                handle: self.handle.clone(),
                outcome: BackgroundOutcome::Allow,
            });
            return;
        }

        if self.allow_once_rect != utils::Rect::ZERO && interactivity.is_clicked(self.allow_once_rect) {
            println!("[background-ui] Allow once clicked.");
            ctx.dispatch(BackgroundResponse {
                handle: self.handle.clone(),
                outcome: BackgroundOutcome::AllowThisInstance,
            });
            return;
        }

        if self.block_rect != utils::Rect::ZERO && interactivity.is_clicked(self.block_rect) {
            println!("[background-ui] Block clicked.");
            ctx.dispatch(BackgroundResponse {
                handle: self.handle.clone(),
                outcome: BackgroundOutcome::Forbid,
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
    name: &str,
) -> RootDiv {
    // Header section: icon + title + subtitle
    let mut icon_text = Text::new(Style::default());
    icon_text.font = Some(font_24);
    icon_text.text = "⚙️".to_string();
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
    title_text.text = "Background Activity".to_string();
    title_text.color = Color::WHITE;
    title_text.z = 0.95;

    let mut subtitle_text = Text::new(Style::default());
    subtitle_text.font = Some(font_16);
    subtitle_text.text = format!("Requested by {app_id}");
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
            top: length(6.0_f32),
            bottom: length(6.0_f32),
        },
        ..Default::default()
    };
    let header = Div::new(header_style, (icon_text, title_text, subtitle_text));

    // Body text split into three lines for precise manual wrapping
    let mut body_text1 = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(2.0_f32),
        },
        ..Default::default()
    });
    body_text1.font = Some(font_16);
    body_text1.text = format!("The application \"{}\"", name);
    body_text1.color = Color::rgb(0.8, 0.8, 0.88);
    body_text1.z = 0.95;

    let mut body_text2 = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(2.0_f32),
        },
        ..Default::default()
    });
    body_text2.font = Some(font_16);
    body_text2.text = "wants to run in the background.".to_string();
    body_text2.color = Color::rgb(0.8, 0.8, 0.88);
    body_text2.z = 0.95;

    let mut body_text3 = Text::new(Style::default());
    body_text3.font = Some(font_16);
    body_text3.text = "If forbidden, it might stop working.".to_string();
    body_text3.color = Color::rgb(0.8, 0.8, 0.88);
    body_text3.z = 0.95;

    // Body container to group and layout the body text lines nicely
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
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(0.0_f32),
        },
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(16.0_f32),
        },
        ..Default::default()
    };
    let mut body_card = Div::new(body_card_style, (body_text1, body_text2, body_text3));
    body_card.z = 0.95;

    // Three buttons (width = 130.0)
    let mut block_btn = BgButton::new("Block", 130.0);
    block_btn.div.color = Color::rgb(0.15, 0.08, 0.08);
    block_btn.div.border_radius = 10.0;
    block_btn.div.border_color = Color::rgb(0.3, 0.15, 0.15);
    block_btn.div.border_thickness = 1.0;
    block_btn.div.z = 1.0;
    block_btn.div.children.font = Some(font_16);
    block_btn.div.children.color = Color::rgb(1.0, 0.55, 0.55);
    block_btn.div.children.z = 0.5;

    let mut allow_once_btn = BgButton::new("Allow once", 130.0);
    allow_once_btn.div.color = Color::rgb(0.12, 0.13, 0.18);
    allow_once_btn.div.border_radius = 10.0;
    allow_once_btn.div.border_color = Color::rgb(0.24, 0.26, 0.32);
    allow_once_btn.div.border_thickness = 1.0;
    allow_once_btn.div.z = 1.0;
    allow_once_btn.div.children.font = Some(font_16);
    allow_once_btn.div.children.color = Color::rgb(0.85, 0.88, 0.95);
    allow_once_btn.div.children.z = 0.5;

    let mut allow_always_btn = BgButton::new("Allow always", 130.0);
    allow_always_btn.div.color = Color::rgb(0.12, 0.5, 0.95);
    allow_always_btn.div.border_radius = 10.0;
    allow_always_btn.div.border_color = Color::rgb(0.25, 0.65, 1.0);
    allow_always_btn.div.border_thickness = 1.0;
    allow_always_btn.div.z = 1.0;
    allow_always_btn.div.children.font = Some(font_16);
    allow_always_btn.div.children.color = Color::WHITE;
    allow_always_btn.div.children.z = 0.5;

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
    let button_row = Div::new(button_row_style, (block_btn, allow_once_btn, allow_always_btn));

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
    modal.color = Color::rgb(0.08, 0.08, 0.11);
    modal.border_color = Color::rgb(0.18, 0.2, 0.26);
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
