use crate::backend::types::{PrintOutcome, PrintResponse, RequestHandle};
use crate::dialog::widgets::{OptionRow, OptionRows, PrinterRow, PrinterRows};
use assets::BakedFont;
use std::collections::HashMap;
use std::ops::Deref;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;
use zbus::zvariant::OwnedValue;

use portal_core::atlas;
use portal_core::widgets::Button;

pub type HeaderDiv = Div<(Text, Text, Text)>;
pub type ButtonRowDiv = Div<(Button, Button)>;
pub type PrinterListDiv = Div<PrinterRows>;
pub type OptionListDiv = Div<OptionRows>;

pub type TopSection = Div<(HeaderDiv, Text, PrinterListDiv)>;
pub type MiddleSection = Div<(Text, OptionListDiv)>;

pub type ModalDiv = Div<(TopSection, MiddleSection, ButtonRowDiv)>;
pub type RootDiv = Div<(ModalDiv,)>;

pub struct PrintDialogUi {
    handle: RequestHandle,
    settings: HashMap<String, OwnedValue>,
    page_setup: HashMap<String, OwnedValue>,
    root: RootDiv,
    selected_printer: String,
    printers: Vec<String>,
    cancel_rect: utils::Rect,
    confirm_rect: utils::Rect,
    row_rects: Vec<(u64, utils::Rect, String)>,
    cancel_id: Option<u64>,
    confirm_id: Option<u64>,
}

impl PrintDialogUi {
    pub fn new(
        handle: RequestHandle,
        app_id: String,
        title: String,
        settings: HashMap<String, OwnedValue>,
        page_setup: HashMap<String, OwnedValue>,
        options: HashMap<String, OwnedValue>,
    ) -> Self {
        let font_24 = &atlas::UI_FONT_INTER_24;
        let font_16 = &atlas::UI_FONT_INTER_16;
        let font_14 = &atlas::UI_FONT_INTER_14;

        let printers = crate::helpers::get_printer_list();
        let selected_printer = printers.first().cloned().unwrap_or_default();

        let root = make_root(
            font_24,
            font_16,
            font_14,
            &app_id,
            &title,
            &settings,
            &page_setup,
            &options,
            &printers,
            &selected_printer,
        );

        Self {
            handle,
            settings,
            page_setup,
            root,
            selected_printer,
            printers,
            cancel_rect: utils::Rect::ZERO,
            confirm_rect: utils::Rect::ZERO,
            row_rects: Vec::new(),
            cancel_id: None,
            confirm_id: None,
        }
    }
}

impl WidgetList for PrintDialogUi {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let child_ids = vec![self.root.build_tree(tree)];
        self.cancel_id = Some(self.root.children.0.children.2.children.0.node_id().into());
        self.confirm_id = Some(self.root.children.0.children.2.children.1.node_id().into());

        // Initialize printer rows with their names and selection states
        let rows = &mut self.root.children.0.children.0.children.2.children;
        for (i, row) in rows.0.iter_mut().enumerate() {
            if i < self.printers.len() {
                let name = self.printers[i].clone();
                let is_selected = name == self.selected_printer;
                row.update(tree, Some(name), is_selected);
            } else {
                row.update(tree, None, false);
            }
        }

        child_ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let commands = self.root.render_children(tree, parent_abs);
        let cancel_id = self.cancel_id.unwrap_or(0);
        let confirm_id = self.confirm_id.unwrap_or(0);

        self.row_rects.clear();
        let rows = &self.root.children.0.children.0.children.2.children;
        for cmd in &commands {
            if let RenderCommand::RegisterHitArea { id, rect } = cmd {
                let id_val = *id;
                let rect_val = *rect;
                if id_val == cancel_id {
                    self.cancel_rect = rect_val;
                } else if id_val == confirm_id {
                    self.confirm_rect = rect_val;
                } else if let Some(row) = rows.0.iter().find(|r| u64::from(r.node_id()) == id_val) {
                    if !row.name.is_empty() {
                        self.row_rects.push((id_val, rect_val, row.name.clone()));
                    }
                }
            }
        }
        commands
    }

    fn on_event(&mut self, ctx: &mut EventCtx) {
        let interactivity = ctx.interactivity();
        let tree = ctx.tree();

        // 1. Check Confirm/Print button
        if self.confirm_rect != utils::Rect::ZERO && interactivity.is_clicked(self.confirm_rect) {
            println!(
                "[print-ui] Confirmed printing to: {}",
                self.selected_printer
            );
            ctx.dispatch(PrintResponse {
                handle: self.handle.clone(),
                outcome: PrintOutcome::Granted {
                    selected_printer: self.selected_printer.clone(),
                    settings: self.settings.clone(),
                    page_setup: self.page_setup.clone(),
                },
            });
            return;
        }

        // 2. Check Cancel button
        if self.cancel_rect != utils::Rect::ZERO && interactivity.is_clicked(self.cancel_rect) {
            println!("[print-ui] Cancelled printing for handle={}", self.handle);
            ctx.dispatch(PrintResponse {
                handle: self.handle.clone(),
                outcome: PrintOutcome::Denied,
            });
            return;
        }

        // 3. Check Printer Row clicks
        for (_id, rect, name) in &self.row_rects {
            if interactivity.is_clicked(*rect) {
                println!("[print-ui] Selected printer: {}", name);
                self.selected_printer = name.clone();
                let rows = &mut self.root.children.0.children.0.children.2.children;
                for row in &mut rows.0 {
                    let name_opt = Some(row.name.clone());
                    let is_selected = row.name == self.selected_printer;
                    row.update(tree, name_opt, is_selected);
                }
                return;
            }
        }
    }

    fn wants_input(&self) -> bool {
        true
    }
}

// --- Layout helpers

fn format_value(val: &OwnedValue) -> String {
    let val_ref = val.deref();
    match val_ref {
        zbus::zvariant::Value::Str(s) => s.as_str().to_string(),
        zbus::zvariant::Value::U32(v) => v.to_string(),
        zbus::zvariant::Value::I32(v) => v.to_string(),
        zbus::zvariant::Value::U64(v) => v.to_string(),
        zbus::zvariant::Value::I64(v) => v.to_string(),
        zbus::zvariant::Value::Bool(v) => v.to_string(),
        zbus::zvariant::Value::F64(v) => format!("{:.2}", v),
        _ => format!("{:?}", val_ref),
    }
}

fn make_root(
    font_24: &'static BakedFont,
    font_16: &'static BakedFont,
    font_14: &'static BakedFont,
    app_id: &str,
    title: &str,
    settings: &HashMap<String, OwnedValue>,
    page_setup: &HashMap<String, OwnedValue>,
    options: &HashMap<String, OwnedValue>,
    printers: &[String],
    _selected_printer: &str,
) -> RootDiv {
    // Icon
    let mut icon_text = Text::new(Style::default());
    icon_text.font = Some(font_24);
    icon_text.text = "".to_string();
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
    title_text.text = if title.is_empty() {
        "Print Document".to_string()
    } else {
        title.to_string()
    };
    title_text.color = Color::WHITE;
    title_text.z = 0.95;

    // Subtitle — show requesting app id
    let subtitle_str = if app_id.is_empty() {
        "An application wants to print a document.".to_string()
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
            top: length(12.0_f32),
            bottom: length(12.0_f32),
        },
        ..Default::default()
    };
    let header = Div::new(header_style, (icon_text, title_text, subtitle_text));

    // Section 1 Label: "Select Printer"
    let mut printers_label = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(8.0_f32),
            bottom: length(8.0_f32),
        },
        ..Default::default()
    });
    printers_label.font = Some(font_16);
    printers_label.text = "Select Printer:".to_string();
    printers_label.color = Color::rgb(0.85, 0.85, 0.9);
    printers_label.z = 0.95;

    // Printer List Div
    let list_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: percent(1.0_f32),
            height: length(140.0_f32),
        },
        padding: taffy::Rect {
            left: length(8.0_f32),
            right: length(8.0_f32),
            top: length(8.0_f32),
            bottom: length(8.0_f32),
        },
        ..Default::default()
    };
    let mut rows = Vec::new();
    for _ in 0..printers.len().min(5) {
        rows.push(PrinterRow::new(font_14));
    }
    let mut list_div = Div::new(list_style, PrinterRows(rows));
    list_div.color = Color::rgb(0.02, 0.02, 0.03);
    list_div.border_radius = 8.0;
    list_div.border_color = Color::rgb(0.08, 0.08, 0.1);
    list_div.border_thickness = 1.0;
    list_div.z = 0.5;

    // Section 2 Label: "Selected Options"
    let mut options_label = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(12.0_f32),
            bottom: length(8.0_f32),
        },
        ..Default::default()
    });
    options_label.font = Some(font_16);
    options_label.text = "Selected Options:".to_string();
    options_label.color = Color::rgb(0.85, 0.85, 0.9);
    options_label.z = 0.95;

    // Options List Div
    let opt_list_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: percent(1.0_f32),
            height: length(140.0_f32),
        },
        padding: taffy::Rect {
            left: length(12.0_f32),
            right: length(12.0_f32),
            top: length(8.0_f32),
            bottom: length(8.0_f32),
        },
        ..Default::default()
    };

    let mut opt_rows = Vec::new();
    for (k, v) in settings {
        opt_rows.push(OptionRow::new(
            font_14,
            format!("Setting: {}", k),
            format_value(v),
        ));
    }
    for (k, v) in page_setup {
        opt_rows.push(OptionRow::new(
            font_14,
            format!("Page: {}", k),
            format_value(v),
        ));
    }
    for (k, v) in options {
        opt_rows.push(OptionRow::new(
            font_14,
            format!("Option: {}", k),
            format_value(v),
        ));
    }
    if opt_rows.is_empty() {
        opt_rows.push(OptionRow::new(
            font_14,
            "No options selected".to_string(),
            "Default".to_string(),
        ));
    }

    let mut opt_list_div = Div::new(opt_list_style, OptionRows(opt_rows));
    opt_list_div.color = Color::rgb(0.02, 0.02, 0.03);
    opt_list_div.border_radius = 8.0;
    opt_list_div.border_color = Color::rgb(0.08, 0.08, 0.1);
    opt_list_div.border_thickness = 1.0;
    opt_list_div.z = 0.5;

    // Cancel button (left, neutral)
    let mut cancel_btn = Button::new("Cancel");
    cancel_btn.div.color = Color::rgb(0.12, 0.12, 0.15);
    cancel_btn.div.border_radius = 10.0;
    cancel_btn.div.border_color = Color::rgb(0.22, 0.22, 0.27);
    cancel_btn.div.border_thickness = 1.5;
    cancel_btn.div.z = 1.0;
    cancel_btn.div.children.font = Some(font_16);
    cancel_btn.div.children.color = Color::rgb(0.85, 0.85, 0.9);
    cancel_btn.div.children.z = 0.5;

    // Confirm button (right, accent)
    let mut confirm_btn = Button::new("Print");
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
            height: length(50.0_f32),
        },
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(16.0_f32),
            bottom: length(0.0_f32),
        },
        ..Default::default()
    };
    let button_row = Div::new(button_row_style, (cancel_btn, confirm_btn));

    // Construct nested sections to stay within the max 3-element tuple support of the UI framework
    let top_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        ..Default::default()
    };
    let top_section = Div::new(top_style, (header, printers_label, list_div));

    let middle_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        ..Default::default()
    };
    let middle_section = Div::new(middle_style, (options_label, opt_list_div));

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
            top: length(24.0_f32),
            bottom: length(24.0_f32),
        },
        ..Default::default()
    };
    let mut modal = Div::new(modal_style, (top_section, middle_section, button_row));
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
