use crate::backend::{AppChooserOutcome, AppChooserResponse, RequestHandle};
use assets::BakedFont;
use std::cell::RefCell;
use std::rc::Rc;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{Point, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;

use portal_core::atlas;
use portal_core::widgets::Button;

use super::widgets::{AppRow, AppRows};



// Widget tree type aliases

pub type HeaderDiv = Div<(Text, Text)>;
pub type ListDiv = Div<AppRows>;
pub type FooterDiv = Div<(Button, Button)>;
pub type ModalDiv = Div<(HeaderDiv, ListDiv, FooterDiv)>;
pub type RootDiv = Div<(ModalDiv,)>;

// Main dialog struct

const MAX_ROWS: usize = 8;

pub struct AppChooserDialogUi {
    handle: RequestHandle,
    choices: Vec<String>,
    selected_idx: Option<usize>,
    root: RootDiv,
    cancel_rect: utils::Rect,
    open_rect: utils::Rect,
    row_rects: Vec<(u64, utils::Rect, usize)>, // (node_id, rect, choice_index)
    cancel_id: Option<u64>,
    open_id: Option<u64>,
    /// Shared channel for live choice updates pushed from the UI coordinator.
    pending_choice_update: Rc<RefCell<Option<Vec<String>>>>,
}

impl AppChooserDialogUi {
    pub fn new(
        handle: RequestHandle,
        _app_id: String,
        choices: Vec<String>,
        last_choice: Option<String>,
        content_type: Option<String>,
        uri: Option<String>,
        filename: Option<String>,
        pending_choice_update: Rc<RefCell<Option<Vec<String>>>>,
    ) -> Self {
        let selected_idx = last_choice
            .as_deref()
            .and_then(|lc| choices.iter().position(|c| c == lc));

        // Build the most informative subtitle we can from the available hints.
        // Priority: filename > content_type > uri > generic fallback.
        let subtitle = if let Some(fname) = &filename {
            format!("Choose an application to open \"{fname}\"")
        } else if let Some(ct) = &content_type {
            format!("Choose an application to open: {ct}")
        } else if let Some(u) = &uri {
            format!("Choose an application to open: {u}")
        } else {
            "Choose an application to open this item".to_string()
        };

        let font_24 = &atlas::UI_FONT_INTER_24;
        let font_16 = &atlas::UI_FONT_INTER_16;

        let root = make_root(font_24, font_16, &choices, &subtitle);

        Self {
            handle,
            choices,
            selected_idx,
            root,
            cancel_rect: utils::Rect::ZERO,
            open_rect: utils::Rect::ZERO,
            row_rects: Vec::new(),
            cancel_id: None,
            open_id: None,
            pending_choice_update,
        }
    }

    fn populate_rows(&mut self, tree: &mut WidgetTree) {
        let rows = &mut self.root.children.0.children.1.children.0;
        for (i, row) in rows.iter_mut().enumerate() {
            let app_id = self.choices.get(i).cloned();
            let is_sel = self.selected_idx == Some(i);
            row.update(tree, app_id, is_sel);
        }
    }
}

impl WidgetList for AppChooserDialogUi {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let child_ids = vec![self.root.build_tree(tree)];

        // footer.div = children.0.children.2
        // footer buttons: .children.0 = Cancel, .children.1 = Open
        self.cancel_id = Some(self.root.children.0.children.2.children.0.node_id().into());
        self.open_id = Some(self.root.children.0.children.2.children.1.node_id().into());

        self.populate_rows(tree);
        child_ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let commands = self.root.render_children(tree, parent_abs);

        let cancel_id = self.cancel_id.unwrap_or(0);
        let open_id = self.open_id.unwrap_or(0);

        self.row_rects.clear();
        let rows = &self.root.children.0.children.1.children.0;

        for cmd in &commands {
            if let RenderCommand::RegisterHitArea { id, rect } = cmd {
                if *id == cancel_id {
                    self.cancel_rect = *rect;
                } else if *id == open_id {
                    self.open_rect = *rect;
                } else if let Some(row_idx) =
                    rows.iter().position(|r| u64::from(r.node_id()) == *id)
                {
                    if rows[row_idx].app_id.is_some() {
                        self.row_rects.push((*id, *rect, row_idx));
                    }
                }
            }
        }
        commands
    }

    fn on_event(&mut self, ctx: &mut ui::EventCtx) {
        let interactivity = ctx.interactivity();
        let tree = ctx.tree();

        // Apply any pending choice update pushed from the UI coordinator.
        let new_choices = self.pending_choice_update.borrow_mut().take();
        if let Some(new_choices) = new_choices {
            println!(
                "[app-chooser-ui] UpdateChoices applied: {} choices.",
                new_choices.len()
            );
            // Keep selected_idx valid: if the previously selected app is no
            // longer in the new list, clear the selection.
            self.selected_idx = self
                .selected_idx
                .and_then(|i| self.choices.get(i))
                .and_then(|prev| new_choices.iter().position(|c| c == prev));
            self.choices = new_choices;
            self.populate_rows(tree);
        }
        // Open / confirm button
        if self.open_rect != utils::Rect::ZERO && interactivity.is_clicked(self.open_rect) {
            let chosen = self.selected_idx.and_then(|i| self.choices.get(i)).cloned();
            println!("[app-chooser-ui] Open clicked → {:?}", chosen);
            if let Some(app_id) = chosen {
                ctx.dispatch(AppChooserResponse {
                    handle: self.handle.clone(),
                    outcome: AppChooserOutcome::Chosen(app_id),
                });
            } else {
                // Nothing selected yet — ignore (button should be visually dimmed, but we
                // tolerate the click gracefully by doing nothing).
            }
            return;
        }

        // Cancel button
        if self.cancel_rect != utils::Rect::ZERO && interactivity.is_clicked(self.cancel_rect) {
            println!("[app-chooser-ui] Cancel clicked.");
            ctx.dispatch(AppChooserResponse {
                handle: self.handle.clone(),
                outcome: AppChooserOutcome::Cancelled,
            });
            return;
        }

        // Row selection
        for &(id, rect, row_idx) in &self.row_rects {
            if interactivity.is_clicked(rect) {
                let new_sel = Some(row_idx);
                if self.selected_idx == new_sel {
                    // Double-click semantics: confirm immediately.
                    if let Some(app_id) = self.choices.get(row_idx).cloned() {
                        println!(
                            "[app-chooser-ui] Row {} double-tapped → {}",
                            row_idx, app_id
                        );
                        ctx.dispatch(AppChooserResponse {
                            handle: self.handle.clone(),
                            outcome: AppChooserOutcome::Chosen(app_id),
                        });
                        return;
                    }
                }
                println!("[app-chooser-ui] Row {} selected (id={}).", row_idx, id);
                self.selected_idx = new_sel;
                self.populate_rows(tree);
                return;
            }
        }
    }

    fn wants_input(&self) -> bool {
        true
    }
}

// Layout

fn make_root(
    font_24: &'static BakedFont,
    font_16: &'static BakedFont,
    choices: &[String],
    subtitle: &str,
) -> RootDiv {
    // Header
    let mut title_text = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(6.0_f32),
        },
        ..Default::default()
    });
    title_text.font = Some(font_24);
    title_text.text = "Open With".to_string();
    title_text.color = Color::WHITE;
    title_text.z = 0.95;

    let mut subtitle_text = Text::new(Style::default());
    subtitle_text.font = Some(font_16);
    subtitle_text.text = subtitle.to_string();
    subtitle_text.color = Color::rgb(0.60, 0.60, 0.70);
    subtitle_text.z = 0.95;

    let header_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: percent(1.0_f32),
            height: length(64.0_f32),
        },
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Start),
        ..Default::default()
    };
    let header = Div::new(header_style, (title_text, subtitle_text));

    // App list
    let list_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        size: Size {
            width: percent(1.0_f32),
            // Each row is ~58 px (52 + 3 + 3 margins), leave space for header + footer.
            height: length((MAX_ROWS as f32) * 58.0 + 16.0),
        },
        padding: taffy::Rect {
            left: length(8.0_f32),
            right: length(8.0_f32),
            top: length(8.0_f32),
            bottom: length(8.0_f32),
        },
        ..Default::default()
    };

    // Pre-allocate MAX_ROWS slots; populate them after build via `populate_rows`.
    let rows: Vec<AppRow> = (0..MAX_ROWS).map(|_| AppRow::new(font_16)).collect();
    let _ = choices; // choices are applied in populate_rows after the tree is built

    let mut list_div = Div::new(list_style, AppRows(rows));
    list_div.color = Color::rgb(0.03, 0.03, 0.04);
    list_div.border_radius = 10.0;
    list_div.border_color = Color::rgb(0.09, 0.09, 0.12);
    list_div.border_thickness = 1.0;
    list_div.z = 0.5;

    // Footer (Cancel / Open)
    let mut cancel_btn = Button::new("Cancel");
    cancel_btn.div.color = Color::rgb(0.10, 0.10, 0.13);
    cancel_btn.div.border_radius = 10.0;
    cancel_btn.div.border_color = Color::rgb(0.20, 0.20, 0.25);
    cancel_btn.div.border_thickness = 1.5;
    cancel_btn.div.z = 1.0;
    cancel_btn.div.children.font = Some(font_16);
    cancel_btn.div.children.color = Color::rgb(0.82, 0.82, 0.88);
    cancel_btn.div.children.z = 0.5;

    let mut open_btn = Button::new("Open");
    open_btn.div.color = Color::rgb(0.10, 0.40, 0.88);
    open_btn.div.border_radius = 10.0;
    open_btn.div.border_color = Color::rgb(0.15, 0.52, 1.0);
    open_btn.div.border_thickness = 1.5;
    open_btn.div.z = 1.0;
    open_btn.div.children.font = Some(font_16);
    open_btn.div.children.color = Color::WHITE;
    open_btn.div.children.z = 0.5;

    let footer_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        size: Size {
            width: percent(1.0_f32),
            height: length(56.0_f32),
        },
        justify_content: Some(JustifyContent::SpaceBetween),
        align_items: Some(AlignItems::Center),
        ..Default::default()
    };
    let footer = Div::new(footer_style, (cancel_btn, open_btn));

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
            top: length(24.0_f32),
            bottom: length(20.0_f32),
        },
        ..Default::default()
    };
    let mut modal = Div::new(modal_style, (header, list_div, footer));
    modal.color = Color::rgb(0.06, 0.06, 0.08);
    modal.border_color = Color::rgb(0.13, 0.13, 0.17);
    modal.border_radius = 0.0;
    modal.border_thickness = 0.0;
    modal.z = 0.2;

    // Full-screen backdrop
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
    root.color = Color::rgba(0.0, 0.0, 0.0, 0.55);
    root.z = 0.1;

    root
}
