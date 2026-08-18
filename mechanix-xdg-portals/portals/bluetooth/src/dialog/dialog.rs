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

fn lookup_bluetooth_service_name(uuid: &str) -> Option<&'static str> {
    let s = uuid.trim().to_lowercase();
    let short_hex = if s.len() == 36 && s.ends_with("-0000-1000-8000-00805f9b34fb") {
        &s[4..8]
    } else if s.starts_with("0x") {
        s.trim_start_matches("0x")
    } else {
        &s
    };

    match short_hex {
        // Service Classes / Profiles (0x1100 - 0x113F, 0x1200)
        "1101" => Some("Serial Port (SPP)"),
        "1102" => Some("LAN Access (PPP)"),
        "1103" => Some("Dial-Up Networking (DUN)"),
        "1104" => Some("IrMC Sync"),
        "1105" => Some("Object Push (OPP)"),
        "1106" => Some("File Transfer (FTP)"),
        "1107" => Some("IrMC Sync Command"),
        "1108" => Some("Headset (HSP)"),
        "1109" => Some("Cordless Telephony"),
        "110a" => Some("Audio Source (A2DP)"),
        "110b" => Some("Audio Sink (A2DP)"),
        "110c" => Some("A/V Remote Control Target"),
        "110d" => Some("Advanced Audio Distribution (A2DP)"),
        "110e" => Some("A/V Remote Control (AVRCP)"),
        "110f" => Some("A/V Remote Control Controller"),
        "1110" => Some("Intercom"),
        "1111" => Some("Fax"),
        "1112" => Some("Headset Audio Gateway"),
        "1115" => Some("Personal Area Networking (PANU)"),
        "1116" => Some("Network Access Point (NAP)"),
        "1117" => Some("Group Ad-hoc Network (GN)"),
        "111a" => Some("Basic Imaging (BIP)"),
        "111e" => Some("Hands-Free (HFP)"),
        "111f" => Some("Hands-Free Audio Gateway"),
        "1122" => Some("Basic Printing (BPP)"),
        "1124" => Some("Human Interface Device (HID)"),
        "1125" => Some("Hardcopy Cable Replacement (HCRP)"),
        "112d" => Some("SIM Access (SAP)"),
        "112f" => Some("Phonebook Access Server (PBAP)"),
        "1130" => Some("Phonebook Access Client (PBAP)"),
        "1131" => Some("Phonebook Access (PBAP)"),
        "1132" => Some("Message Access Server (MAP)"),
        "1133" => Some("Message Notification Server (MAP)"),
        "1134" => Some("Message Access Profile (MAP)"),
        "1135" => Some("GNSS Server"),
        "1200" => Some("Device Information (PnP)"),

        // GATT Services (0x1800 - 0x1855)
        "1800" => Some("Generic Access"),
        "1801" => Some("Generic Attribute"),
        "1802" => Some("Immediate Alert"),
        "1803" => Some("Link Loss"),
        "1804" => Some("Tx Power"),
        "1805" => Some("Current Time Service"),
        "1806" => Some("Reference Time Update"),
        "1807" => Some("Next DST Change"),
        "1808" => Some("Glucose Service"),
        "1809" => Some("Health Thermometer"),
        "180a" => Some("Device Information"),
        "180d" => Some("Heart Rate Monitor"),
        "180e" => Some("Phone Alert Status"),
        "180f" => Some("Battery Service"),
        "1810" => Some("Blood Pressure"),
        "1811" => Some("Alert Notification"),
        "1812" => Some("Human Interface Device (HID LE)"),
        "1813" => Some("Scan Parameters"),
        "1814" => Some("Running Speed and Cadence"),
        "1815" => Some("Automation IO"),
        "1816" => Some("Cycling Speed and Cadence"),
        "1818" => Some("Cycling Power"),
        "1819" => Some("Location and Navigation"),
        "181a" => Some("Environmental Sensing"),
        "181b" => Some("Body Composition"),
        "181c" => Some("User Data"),
        "181d" => Some("Weight Scale"),
        "181e" => Some("Bond Management"),
        "181f" => Some("Continuous Glucose"),
        "1820" => Some("IP Support Service"),
        "1821" => Some("Indoor Positioning"),
        "1822" => Some("Pulse Oximeter"),
        "1823" => Some("Fitness Machine"),
        "1824" => Some("Mesh Provisioning"),
        "1825" => Some("Mesh Proxy"),
        "1826" => Some("Reconnection Configuration"),
        "1827" => Some("Volume Control"),
        "1828" => Some("Volume Offset Control"),
        "1829" => Some("Coordinated Audio Stream"),
        "1843" => Some("Audio Input Control"),
        "1844" => Some("Hearing Access"),
        "184e" => Some("Media Control"),
        "184f" => Some("Generic Media Control"),
        "1853" => Some("Common Audio"),
        "1854" => Some("Telephony and Media Audio"),
        "1855" => Some("Public Broadcast Announcement"),

        _ => None,
    }
}

fn format_service_prompt(uuid: &str) -> (String, String) {
    if let Some(service_name) = lookup_bluetooth_service_name(uuid) {
        (
            service_name.to_string(),
            format!("Allow access to {service_name}?"),
        )
    } else {
        (
            "Bluetooth Service".into(),
            format!("Allow service:\n{uuid}"),
        )
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
        DialogKind::AuthorizeService { uuid } => {
            let (title, body) = format_service_prompt(uuid);
            (title, format!("Device: {dev}"), body)
        }
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
