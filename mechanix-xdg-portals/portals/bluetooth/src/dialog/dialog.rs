use crate::backend::{BluetoothOutcome, BluetoothResponse};
use assets::BakedFont;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;

use super::types::DialogKind;
use portal_core::atlas;
use portal_core::widgets::Button;

// ---------------------------------------------------------------------------
// Widget tree shape
//
// We use nested 3-tuples (max supported by the WidgetList blanket impls).
//
// Modal children:
//   .0 = info Div<(Text, Text)>   (title + subtitle/body text)
//   .1 = spacer/extra Text        (body detail line)
//   .2 = ButtonRow
//
// Root children: (Modal,)
// ---------------------------------------------------------------------------

/// Info block: title + device line stacked vertically.
pub type InfoDiv = Div<(Text, Text)>;
/// Button row for confirm dialogs: two buttons side by side.
pub type TwoButtonRow = Div<(Button, Button)>;
/// Button row for display dialogs: one dismiss button.
pub type OneButtonRow = Div<(Button,)>;

// Modal variants – must differ in types since Rust is monomorphic.
pub type DisplayModal = Div<(InfoDiv, Text, OneButtonRow)>;
pub type ConfirmModal = Div<(InfoDiv, Text, TwoButtonRow)>;

// Root wrappers.
pub type DisplayRoot = Div<(DisplayModal,)>;
pub type ConfirmRoot = Div<(ConfirmModal,)>;

// ---------------------------------------------------------------------------
// Enum over the two layout kinds
// ---------------------------------------------------------------------------

enum Layout {
    Display(DisplayRoot),
    Confirm(ConfirmRoot),
}

pub struct BluetoothDialogUi {
    is_display: bool,
    layout: Layout,
    primary_rect: utils::Rect,
    secondary_rect: utils::Rect,
}

impl BluetoothDialogUi {
    pub fn new(device: &str, kind: &DialogKind) -> Self {
        let font_24 = &atlas::UI_FONT_INTER_24;
        let font_16 = &atlas::UI_FONT_INTER_16;

        let is_display = matches!(
            kind,
            DialogKind::DisplayPinCode { .. } | DialogKind::DisplayPasskey { .. }
        );

        let (title, subtitle, body) = make_strings(device, kind);

        if is_display {
            let layout = make_display_root(font_24, font_16, &title, &subtitle, &body);
            Self {
                is_display: true,
                layout: Layout::Display(layout),
                primary_rect: utils::Rect::ZERO,
                secondary_rect: utils::Rect::ZERO,
            }
        } else {
            let (primary_label, secondary_label) = match kind {
                DialogKind::RequestConfirmation { .. } => ("Confirm", "Reject"),
                _ => ("Allow", "Reject"),
            };
            let layout = make_confirm_root(
                font_24,
                font_16,
                &title,
                &subtitle,
                &body,
                primary_label,
                secondary_label,
            );
            Self {
                is_display: false,
                layout: Layout::Confirm(layout),
                primary_rect: utils::Rect::ZERO,
                secondary_rect: utils::Rect::ZERO,
            }
        }
    }

    /// Walk the rendered commands and record hit rects for our buttons.
    fn collect_rects(&mut self, commands: &[RenderCommand]) {
        let (primary_id, secondary_id) = match &self.layout {
            Layout::Display(r) => {
                // primary = root.children.0 (modal) .children.2 (OneButtonRow) .children.0 (Button)
                let p: u64 = r.children.0.children.2.children.0.node_id().into();
                (p, 0u64)
            }
            Layout::Confirm(r) => {
                let s: u64 = r.children.0.children.2.children.0.node_id().into();
                let p: u64 = r.children.0.children.2.children.1.node_id().into();
                (p, s)
            }
        };

        for cmd in commands {
            if let RenderCommand::RegisterHitArea { id, rect } = cmd {
                if *id == primary_id {
                    self.primary_rect = *rect;
                } else if *id == secondary_id {
                    self.secondary_rect = *rect;
                }
            }
        }
    }
}

impl WidgetList for BluetoothDialogUi {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        match &mut self.layout {
            Layout::Display(r) => vec![r.build_tree(tree)],
            Layout::Confirm(r) => vec![r.build_tree(tree)],
        }
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let commands = match &mut self.layout {
            Layout::Display(r) => r.render_children(tree, parent_abs),
            Layout::Confirm(r) => r.render_children(tree, parent_abs),
        };
        self.collect_rects(&commands);
        commands
    }

    fn on_event(&mut self, ctx: &mut EventCtx) {
        // Primary button: Dismiss (display) or Confirm/Allow (request)
        if self.primary_rect != utils::Rect::ZERO
            && ctx.interactivity().is_clicked(self.primary_rect)
        {
            let outcome = if self.is_display {
                BluetoothOutcome::Dismissed
            } else {
                BluetoothOutcome::Accepted
            };
            ctx.dispatch(BluetoothResponse { outcome });
        } else if !self.is_display
            && self.secondary_rect != utils::Rect::ZERO
            && ctx.interactivity().is_clicked(self.secondary_rect)
        {
            ctx.dispatch(BluetoothResponse {
                outcome: BluetoothOutcome::Rejected,
            });
        }
    }

    fn wants_input(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// String helpers
// ---------------------------------------------------------------------------

fn short_device(device: &str) -> String {
    let addr = device
        .rsplit('/')
        .next()
        .unwrap_or(device)
        .replace("dev_", "")
        .replace('_', ":");
    if addr.is_empty() {
        device.to_string()
    } else {
        addr
    }
}

fn make_strings(device: &str, kind: &DialogKind) -> (String, String, String) {
    let dev = short_device(device);
    match kind {
        DialogKind::DisplayPinCode { pincode } => (
            "Bluetooth Pairing".into(),
            format!("Device: {dev}"),
            format!("PIN Code: {pincode}"),
        ),
        DialogKind::DisplayPasskey { passkey, entered } => (
            "Bluetooth Pairing".into(),
            format!("Device: {dev}"),
            format!("Passkey: {:06}  ({entered} entered)", passkey),
        ),
        DialogKind::RequestConfirmation { passkey } => (
            "Confirm Pairing".into(),
            format!("Device: {dev}"),
            format!("Does this passkey match?  {:06}", passkey),
        ),
        DialogKind::RequestAuthorization => (
            "Confirm Pairing".into(),
            format!("Device: {dev}"),
            "Allow this device to connect?".into(),
        ),
        DialogKind::AuthorizeService { uuid } => (
            "Bluetooth Service".into(),
            format!("Device: {dev}"),
            format!("Allow service:\n{uuid}"),
        ),
        DialogKind::RequestPinCode => (
            "Bluetooth Pairing".into(),
            format!("Device: {dev}"),
            "Enter PIN code:".into(),
        ),
        DialogKind::RequestPasskey => (
            "Bluetooth Pairing".into(),
            format!("Device: {dev}"),
            "Enter passkey (0–999999):".into(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Reusable widget builders
// ---------------------------------------------------------------------------

fn make_text(font: &'static BakedFont, s: &str, color: Color) -> Text {
    let style = Style {
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        ..Default::default()
    };
    let mut t = Text::new(style);
    t.font = Some(font);
    t.text = s.to_string();
    t.color = color;
    t.z = 0.95;
    t
}

fn make_info_div(
    font_24: &'static BakedFont,
    font_16: &'static BakedFont,
    title: &str,
    subtitle: &str,
) -> InfoDiv {
    let info_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: zero(),
            height: length(6.0_f32),
        },
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        ..Default::default()
    };
    Div::new(
        info_style,
        (
            make_text(font_24, title, Color::WHITE),
            make_text(font_16, subtitle, Color::rgb(0.55, 0.55, 0.65)),
        ),
    )
}

fn make_btn(font: &'static BakedFont, label: &str, bg: Color, border: Color) -> Button {
    let mut b = Button::new(label);
    b.div.color = bg;
    b.div.border_color = border;
    b.div.border_radius = 10.0;
    b.div.border_thickness = 1.5;
    b.div.z = 1.0;
    b.div.children.font = Some(font);
    b.div.children.color = Color::WHITE;
    b.div.children.z = 0.5;
    b
}

fn row_style() -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        size: Size {
            width: percent(1.0_f32),
            height: length(50.0_f32),
        },
        justify_content: Some(JustifyContent::SpaceBetween),
        align_items: Some(AlignItems::Center),
        ..Default::default()
    }
}

fn modal_style() -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::SpaceBetween),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: length(520.0_f32),
            height: length(320.0_f32),
        },
        padding: taffy::Rect {
            left: length(40.0_f32),
            right: length(40.0_f32),
            top: length(36.0_f32),
            bottom: length(36.0_f32),
        },
        ..Default::default()
    }
}

fn root_style() -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: percent(1.0_f32),
        },
        ..Default::default()
    }
}

fn style_modal<T: WidgetList>(children: T) -> Div<T> {
    let mut m = Div::new(modal_style(), children);
    m.color = Color::rgb(0.07, 0.07, 0.09);
    m.border_color = Color::rgb(0.14, 0.14, 0.18);
    m.border_radius = 16.0;
    m.border_thickness = 1.0;
    m.z = 0.2;
    m
}

fn style_root<T: WidgetList>(children: T) -> Div<T> {
    let mut r = Div::new(root_style(), children);
    r.color = Color::TRANSPARENT;
    r.z = 0.1;
    r
}

// ---------------------------------------------------------------------------
// Full layout builders
// ---------------------------------------------------------------------------

fn make_display_root(
    font_24: &'static BakedFont,
    font_16: &'static BakedFont,
    title: &str,
    subtitle: &str,
    body: &str,
) -> DisplayRoot {
    let info = make_info_div(font_24, font_16, title, subtitle);
    let body_text = make_text(font_16, body, Color::rgb(0.85, 0.85, 0.95));
    let dismiss = make_btn(
        font_16,
        "Dismiss",
        Color::rgb(0.22, 0.22, 0.28),
        Color::rgb(0.35, 0.35, 0.42),
    );
    let btn_row = Div::new(row_style(), (dismiss,));
    let modal = style_modal((info, body_text, btn_row));
    style_root((modal,))
}

fn make_confirm_root(
    font_24: &'static BakedFont,
    font_16: &'static BakedFont,
    title: &str,
    subtitle: &str,
    body: &str,
    primary_label: &str,
    secondary_label: &str,
) -> ConfirmRoot {
    let info = make_info_div(font_24, font_16, title, subtitle);
    let body_text = make_text(font_16, body, Color::rgb(0.85, 0.85, 0.95));
    let primary = make_btn(
        font_16,
        primary_label,
        Color::rgb(0.1, 0.45, 0.9),
        Color::rgb(0.15, 0.55, 1.0),
    );
    let secondary = make_btn(
        font_16,
        secondary_label,
        Color::rgb(0.12, 0.12, 0.15),
        Color::rgb(0.22, 0.22, 0.27),
    );
    let btn_row = Div::new(row_style(), (secondary, primary));
    let modal = style_modal((info, body_text, btn_row));
    style_root((modal,))
}
