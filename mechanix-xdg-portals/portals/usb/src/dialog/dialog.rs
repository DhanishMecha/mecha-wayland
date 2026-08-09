use assets::BakedFont;
use std::collections::HashMap;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;
use zbus::zvariant::OwnedValue;

use crate::backend::types::{RequestHandle, UsbDeviceUiInfo, UsbOutcome, UsbResponse};
use portal_core::atlas;
use portal_core::widgets::Button;

use super::widgets::{UsbRow, UsbRows};

pub type HeaderDiv = Div<(Text, Text, Text)>;
pub type ListDiv = Div<UsbRows>;
pub type FooterDiv = Div<(Button, Button)>;
pub type ModalDiv = Div<(HeaderDiv, ListDiv, FooterDiv)>;
pub type RootDiv = Div<(ModalDiv,)>;

const MAX_ROWS: usize = 6;

pub struct UsbDialogUi {
    handle: RequestHandle,
    devices: Vec<UsbDeviceUiInfo>,
    root: RootDiv,
    cancel_rect: utils::Rect,
    allow_rect: utils::Rect,
    row_rects: Vec<(u64, utils::Rect, usize)>,
    cancel_id: Option<u64>,
    allow_id: Option<u64>,
}

impl UsbDialogUi {
    pub fn new(
        handle: RequestHandle,
        app_id: String,
        devices: Vec<UsbDeviceUiInfo>,
    ) -> Self {
        let font_24 = &atlas::UI_FONT_INTER_24;
        let font_16 = &atlas::UI_FONT_INTER_16;

        let subtitle = if app_id.is_empty() {
            "An application wants to access connected USB devices.".to_string()
        } else {
            format!("{app_id} wants to access connected USB devices.")
        };

        let root = make_root(font_24, font_16, &subtitle);

        Self {
            handle,
            devices,
            root,
            cancel_rect: utils::Rect::ZERO,
            allow_rect: utils::Rect::ZERO,
            row_rects: Vec::new(),
            cancel_id: None,
            allow_id: None,
        }
    }

    fn populate_rows(&mut self, tree: &mut WidgetTree) {
        let rows = &mut self.root.children.0.children.1.children.0;
        for (i, row) in rows.iter_mut().enumerate() {
            if let Some(device) = self.devices.get(i) {
                row.update(
                    tree,
                    Some(device.device_id.clone()),
                    device.name.clone(),
                    device.details.clone(),
                    device.selected,
                );
            } else {
                row.update(tree, None, String::new(), String::new(), false);
            }
        }
    }
}

impl WidgetList for UsbDialogUi {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let child_ids = vec![self.root.build_tree(tree)];
        self.cancel_id = Some(self.root.children.0.children.2.children.0.node_id().into());
        self.allow_id = Some(self.root.children.0.children.2.children.1.node_id().into());
        self.populate_rows(tree);
        child_ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let commands = self.root.render_children(tree, parent_abs);
        let cancel_id = self.cancel_id.unwrap_or(0);
        let allow_id = self.allow_id.unwrap_or(0);

        self.row_rects.clear();
        let rows = &self.root.children.0.children.1.children.0;

        for cmd in &commands {
            if let RenderCommand::RegisterHitArea { id, rect } = cmd {
                let id_val = *id;
                let rect_val = *rect;
                if id_val == cancel_id {
                    self.cancel_rect = rect_val;
                } else if id_val == allow_id {
                    self.allow_rect = rect_val;
                } else if let Some(row_idx) =
                    rows.iter().position(|r| u64::from(r.node_id()) == id_val)
                {
                    if rows[row_idx].device_id.is_some() {
                        self.row_rects.push((id_val, rect_val, row_idx));
                    }
                }
            }
        }
        commands
    }

    fn on_event(&mut self, ctx: &mut EventCtx) {
        let interactivity = ctx.interactivity();
        let tree = ctx.tree();

        // Allow/Confirm button
        if self.allow_rect != utils::Rect::ZERO && interactivity.is_clicked(self.allow_rect) {
            let allowed: Vec<(String, HashMap<String, OwnedValue>)> = self
                .devices
                .iter()
                .filter(|d| d.selected)
                .map(|d| (d.device_id.clone(), d.access_options.clone()))
                .collect();
            println!(
                "[usb-ui] Allow clicked → {} devices permitted",
                allowed.len()
            );
            ctx.dispatch(UsbResponse {
                handle: self.handle.clone(),
                outcome: UsbOutcome::Granted(allowed),
            });
            return;
        }

        // Cancel/Deny button
        if self.cancel_rect != utils::Rect::ZERO && interactivity.is_clicked(self.cancel_rect) {
            println!("[usb-ui] Deny clicked");
            ctx.dispatch(UsbResponse {
                handle: self.handle.clone(),
                outcome: UsbOutcome::Denied,
            });
            return;
        }

        // Row selection toggling
        for &(id, rect, row_idx) in &self.row_rects {
            if interactivity.is_clicked(rect) {
                if let Some(device) = self.devices.get_mut(row_idx) {
                    device.selected = !device.selected;
                    println!(
                        "[usb-ui] Toggled row {} (id={}) to selected={}",
                        row_idx, id, device.selected
                    );
                    self.populate_rows(tree);
                }
                return;
            }
        }
    }

    fn wants_input(&self) -> bool {
        true
    }
}

fn make_root(
    font_24: &'static BakedFont,
    font_16: &'static BakedFont,
    subtitle: &str,
) -> RootDiv {
    // Header Icon
    let mut icon_text = Text::new(Style::default());
    icon_text.font = Some(font_24);
    icon_text.text = "🔌".to_string();
    icon_text.color = Color::rgb(0.55, 0.75, 1.0);
    icon_text.z = 0.95;

    // Header Title
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
    title_text.text = "Allow USB Access?".to_string();
    title_text.color = Color::WHITE;
    title_text.z = 0.95;

    // Header Subtitle
    let mut subtitle_text = Text::new(Style::default());
    subtitle_text.font = Some(font_16);
    subtitle_text.text = subtitle.to_string();
    subtitle_text.color = Color::rgb(0.65, 0.65, 0.75);
    subtitle_text.z = 0.95;

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
    let header = Div::new(header_style, (icon_text, title_text, subtitle_text));

    // Device List Container
    let list_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: percent(1.0_f32),
            height: length((MAX_ROWS as f32) * 66.0 + 8.0),
        },
        padding: taffy::Rect {
            left: length(8.0_f32),
            right: length(8.0_f32),
            top: length(4.0_f32),
            bottom: length(4.0_f32),
        },
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(12.0_f32),
            bottom: length(16.0_f32),
        },
        ..Default::default()
    };

    let rows: Vec<UsbRow> = (0..MAX_ROWS).map(|_| UsbRow::new(font_16)).collect();
    let mut list_div = Div::new(list_style, UsbRows(rows));
    list_div.color = Color::rgb(0.03, 0.03, 0.04);
    list_div.border_radius = 10.0;
    list_div.border_color = Color::rgb(0.09, 0.09, 0.12);
    list_div.border_thickness = 1.0;
    list_div.z = 0.5;

    // Footer Cancel
    let mut cancel_btn = Button::new("Deny");
    cancel_btn.div.color = Color::rgb(0.12, 0.12, 0.15);
    cancel_btn.div.border_radius = 10.0;
    cancel_btn.div.border_color = Color::rgb(0.22, 0.22, 0.27);
    cancel_btn.div.border_thickness = 1.5;
    cancel_btn.div.z = 1.0;
    cancel_btn.div.children.font = Some(font_16);
    cancel_btn.div.children.color = Color::rgb(0.85, 0.85, 0.9);
    cancel_btn.div.children.z = 0.5;

    // Footer Allow
    let mut allow_btn = Button::new("Allow");
    allow_btn.div.color = Color::rgb(0.1, 0.45, 0.9);
    allow_btn.div.border_radius = 10.0;
    allow_btn.div.border_color = Color::rgb(0.15, 0.55, 1.0);
    allow_btn.div.border_thickness = 1.5;
    allow_btn.div.z = 1.0;
    allow_btn.div.children.font = Some(font_16);
    allow_btn.div.children.color = Color::WHITE;
    allow_btn.div.children.z = 0.5;

    let footer_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        size: Size {
            width: percent(1.0_f32),
            height: length(50.0_f32),
        },
        justify_content: Some(JustifyContent::SpaceBetween),
        align_items: Some(AlignItems::Center),
        ..Default::default()
    };
    let footer = Div::new(footer_style, (cancel_btn, allow_btn));

    // Modal Div
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
    let mut modal = Div::new(modal_style, (header, list_div, footer));
    modal.color = Color::rgb(0.07, 0.07, 0.09);
    modal.border_color = Color::rgb(0.14, 0.14, 0.18);
    modal.border_radius = 16.0;
    modal.border_thickness = 1.0;
    modal.z = 0.2;

    // Full screen backdrop
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
