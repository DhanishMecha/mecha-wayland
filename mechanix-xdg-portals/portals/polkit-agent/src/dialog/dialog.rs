use crate::backend::{AuthOutcome, AuthResponse, Identity, RequestHandle};
use assets::BakedFont;
use taffy::prelude::*;
use ui::widgets::{Div, Text};
use ui::{EventCtx, Point, RenderCommand, Widget, WidgetList, WidgetTree};
use utils::Color;

use portal_core::atlas;
use portal_core::widgets::Button;

// ---------------------------------------------------------------------------
// Type aliases for the widget tree
// ---------------------------------------------------------------------------
pub type IconBadgeDiv = Div<(Text,)>;
pub type HeaderDiv = Div<(IconBadgeDiv, Text, Text)>;
pub type ButtonRowDiv = Div<(Button, Button)>;
pub type PasswordInputDiv = Div<(Text,)>;
pub type BodySection = Div<(Text, Text, PasswordInputDiv)>;
pub type ModalDiv = Div<(HeaderDiv, BodySection, ButtonRowDiv)>;
pub type RootDiv = Div<(ModalDiv,)>;

/// The authentication dialog widget.
///
/// Displayed when polkitd sends a `BeginAuthentication` call. Shows the
/// action message, a description of what app is requesting access, and
/// "Cancel" / "Authenticate" buttons.
///
/// NOTE: Password entry is intentionally deferred — the current UI crate
/// does not have a secure text-input widget. For now the dialog shows the
/// auth details and dispatches with a placeholder password. A full
/// implementation will need a masked text-input widget.
pub struct PolkitAuthDialog {
    handle: RequestHandle,
    root: RootDiv,
    cancel_rect: utils::Rect,
    auth_rect: utils::Rect,
    cancel_id: Option<u64>,
    auth_id: Option<u64>,
    /// First unix-user identity, used for the response.
    primary_username: String,
    password: String,
}

impl PolkitAuthDialog {
    pub fn new(
        cookie: impl Into<String>,
        action_id: impl Into<String>,
        message: impl Into<String>,
        icon_name: impl Into<String>,
        identities: &[Identity],
    ) -> Self {
        let font_24 = &atlas::UI_FONT_INTER_24;
        let font_16 = &atlas::UI_FONT_INTER_16;

        let action_id = action_id.into();
        let message = message.into();
        let icon = icon_name.into();

        // Determine the primary username to authenticate as.
        // TODO: Currently we only take the first identity. Support selecting from multiple identities if needed.
        let primary_username = identities
            .iter()
            .find(|id| id.kind == "unix-user")
            .and_then(|id| {
                id.uid().map(|uid| {
                    // Resolve uid → username via /etc/passwd.
                    resolve_username(uid).unwrap_or_else(|| uid.to_string())
                })
            })
            .unwrap_or_else(|| "root".to_string());

        let root = make_root(
            font_24,
            font_16,
            &action_id,
            &message,
            &icon,
            &primary_username,
        );

        Self {
            handle: cookie.into(),
            root,
            cancel_rect: utils::Rect::ZERO,
            auth_rect: utils::Rect::ZERO,
            cancel_id: None,
            auth_id: None,
            primary_username,
            password: String::new(),
        }
    }
}

impl WidgetList for PolkitAuthDialog {
    fn build_children(&mut self, tree: &mut WidgetTree) -> Vec<taffy::NodeId> {
        let child_ids = vec![self.root.build_tree(tree)];
        // Walk the tree to find button node IDs.
        self.cancel_id = Some(self.root.children.0.children.2.children.0.node_id().into());
        self.auth_id = Some(self.root.children.0.children.2.children.1.node_id().into());
        child_ids
    }

    fn render_children(&mut self, tree: &WidgetTree, parent_abs: Point) -> Vec<RenderCommand> {
        let commands = self.root.render_children(tree, parent_abs);
        let cancel_id = self.cancel_id.unwrap_or(0);
        let auth_id = self.auth_id.unwrap_or(0);
        for cmd in &commands {
            if let RenderCommand::RegisterHitArea { id, rect } = cmd {
                if *id == cancel_id {
                    self.cancel_rect = *rect;
                } else if *id == auth_id {
                    self.auth_rect = *rect;
                }
            }
        }
        commands
    }

    fn on_event(&mut self, ctx: &mut EventCtx) {
        let interactivity = ctx.interactivity();

        // Capture keyboard events
        let keyboard = &interactivity.keyboard;
        let shift = keyboard.modifiers().shift;
        let mut password_changed = false;

        for key in keyboard.just_pressed_keys() {
            if key.0 == 14 {
                // Backspace
                self.password.pop();
                password_changed = true;
            } else if key.0 == 28 {
                // Enter
                println!(
                    "[polkit-dialog] Enter pressed, submitting password for cookie={}",
                    self.handle
                );
                ctx.dispatch(AuthResponse {
                    cookie: self.handle.clone(),
                    outcome: AuthOutcome::Authenticate {
                        username: self.primary_username.clone(),
                        password: zeroize::Zeroizing::new(self.password.clone()),
                    },
                });
                return;
            } else if let Some(c) = keycode_to_char(key.0, shift) {
                self.password.push(c);
                password_changed = true;
            }
        }

        if password_changed {
            let tree = ctx.tree();
            self.root
                .children
                .0
                .children
                .1
                .children
                .2
                .children
                .0
                .set_text(
                    tree,
                    format!("Password: {}", "*".repeat(self.password.len())),
                );
        }

        if self.auth_rect != utils::Rect::ZERO && interactivity.is_clicked(self.auth_rect) {
            println!(
                "[polkit-dialog] Authenticate clicked for cookie={}",
                self.handle
            );
            ctx.dispatch(AuthResponse {
                cookie: self.handle.clone(),
                outcome: AuthOutcome::Authenticate {
                    username: self.primary_username.clone(),
                    password: zeroize::Zeroizing::new(self.password.clone()),
                },
            });
            return;
        }

        if self.cancel_rect != utils::Rect::ZERO && interactivity.is_clicked(self.cancel_rect) {
            println!("[polkit-dialog] Cancel clicked for cookie={}", self.handle);
            ctx.dispatch(AuthResponse {
                cookie: self.handle.clone(),
                outcome: AuthOutcome::Cancelled,
            });
        }
    }

    fn wants_input(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------

fn make_root(
    font_24: &'static BakedFont,
    font_16: &'static BakedFont,
    action_id: &str,
    message: &str,
    _icon_name: &str, // TODO: load themed icon when icon infrastructure is available
    username: &str,
) -> RootDiv {
    // ---- Header: icon placeholder + action title + "Authentication Required" label ----
    let mut icon_text = Text::new(Style::default());
    icon_text.font = Some(font_24);
    icon_text.text = "🔐".to_string();
    icon_text.color = Color::rgb(0.38, 0.69, 1.0);
    icon_text.z = 0.95;

    let icon_badge_style = Style {
        display: Display::Flex,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: length(48.0_f32),
            height: length(48.0_f32),
        },
        margin: taffy::Rect {
            left: auto(),
            right: auto(),
            top: length(0.0_f32),
            bottom: length(8.0_f32),
        },
        ..Default::default()
    };
    let mut icon_badge = Div::new(icon_badge_style, (icon_text,));
    icon_badge.color = Color::rgba(0.1, 0.45, 0.9, 0.15);
    icon_badge.border_color = Color::rgba(0.25, 0.65, 1.0, 0.35);
    icon_badge.border_radius = 24.0;
    icon_badge.border_thickness = 1.5;
    icon_badge.z = 0.9;

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
    title_text.text = "Authentication Required".to_string();
    title_text.color = Color::WHITE;
    title_text.z = 0.95;

    let mut action_label = Text::new(Style::default());
    action_label.font = Some(font_16);
    action_label.text = format!("Action: {action_id}");
    action_label.color = Color::rgb(0.68, 0.72, 0.83);
    action_label.z = 0.95;

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
    let header = Div::new(header_style, (icon_badge, title_text, action_label));

    // ---- Body: message + "Authenticating as <user>" ----
    let mut message_text = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(8.0_f32),
        },
        ..Default::default()
    });
    message_text.font = Some(font_16);
    message_text.text = message.to_string();
    message_text.color = Color::rgb(0.85, 0.88, 0.92);
    message_text.z = 0.95;

    let mut user_label = Text::new(Style {
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(16.0_f32),
        },
        ..Default::default()
    });
    user_label.font = Some(font_16);
    user_label.text = format!("Authenticating as: {username}");
    user_label.color = Color::rgb(0.95, 0.72, 0.38); // rich amber
    user_label.z = 0.95;

    let mut password_text = Text::new(Style {
        margin: taffy::Rect {
            left: length(16.0_f32),
            right: length(16.0_f32),
            top: length(0.0_f32),
            bottom: length(0.0_f32),
        },
        ..Default::default()
    });
    password_text.font = Some(font_16);
    password_text.text = "Password: ".to_string();
    password_text.color = Color::rgb(0.55, 0.60, 0.75); // cool grey prefix
    password_text.z = 0.95;

    let input_container_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: length(48.0_f32),
        },
        margin: taffy::Rect {
            left: length(0.0_f32),
            right: length(0.0_f32),
            top: length(0.0_f32),
            bottom: length(16.0_f32),
        },
        ..Default::default()
    };
    let mut password_input_container = Div::new(input_container_style, (password_text,));
    password_input_container.color = Color::rgb(0.05, 0.06, 0.08); // Obsidian deep dark input
    password_input_container.border_color = Color::rgb(0.24, 0.30, 0.45); // cool electric blue border
    password_input_container.border_radius = 10.0;
    password_input_container.border_thickness = 1.5;
    password_input_container.z = 0.9;

    let body_style = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        align_items: Some(AlignItems::FlexStart),
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        ..Default::default()
    };
    let body = Div::new(
        body_style,
        (message_text, user_label, password_input_container),
    );

    // ---- Buttons ----
    let mut cancel_btn = Button::new("Cancel");
    cancel_btn.div.color = Color::rgb(0.14, 0.16, 0.20);
    cancel_btn.div.border_radius = 12.0;
    cancel_btn.div.border_color = Color::rgb(0.22, 0.25, 0.32);
    cancel_btn.div.border_thickness = 1.5;
    cancel_btn.div.z = 1.0;
    cancel_btn.div.children.font = Some(font_16);
    cancel_btn.div.children.color = Color::rgb(0.75, 0.78, 0.88);
    cancel_btn.div.children.z = 0.5;

    let mut auth_btn = Button::new("Authenticate");
    auth_btn.div.color = Color::rgb(0.12, 0.53, 0.90);
    auth_btn.div.border_radius = 12.0;
    auth_btn.div.border_color = Color::rgb(0.25, 0.65, 1.0);
    auth_btn.div.border_thickness = 1.5;
    auth_btn.div.z = 1.0;
    auth_btn.div.children.font = Some(font_16);
    auth_btn.div.children.color = Color::WHITE;
    auth_btn.div.children.z = 0.5;

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
    let button_row = Div::new(button_row_style, (cancel_btn, auth_btn));

    // ---- Modal card ----
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
    let mut modal = Div::new(modal_style, (header, body, button_row));
    modal.color = Color::rgb(0.09, 0.10, 0.13); // Obsidian Slate
    modal.border_color = Color::rgb(0.18, 0.20, 0.26); // Subtle metallic border
    modal.border_radius = 20.0;
    modal.border_thickness = 1.5;
    modal.z = 0.2;

    // ---- Full-screen dimmed backdrop ----
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
    root.color = Color::rgba(0.02, 0.02, 0.03, 0.82); // beautiful translucent dimming
    root.z = 0.1;

    root
}

/// Resolve a uid to a username by reading /etc/passwd.
/// Returns None if not found.
fn resolve_username(uid: u32) -> Option<String> {
    let contents = std::fs::read_to_string("/etc/passwd").ok()?;
    for line in contents.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 {
            if let Ok(entry_uid) = parts[2].parse::<u32>() {
                if entry_uid == uid {
                    return Some(parts[0].to_string());
                }
            }
        }
    }
    None
}

fn keycode_to_char(keycode: u32, shift: bool) -> Option<char> {
    let c = match keycode {
        2 => {
            if shift {
                '!'
            } else {
                '1'
            }
        }
        3 => {
            if shift {
                '@'
            } else {
                '2'
            }
        }
        4 => {
            if shift {
                '#'
            } else {
                '3'
            }
        }
        5 => {
            if shift {
                '$'
            } else {
                '4'
            }
        }
        6 => {
            if shift {
                '%'
            } else {
                '5'
            }
        }
        7 => {
            if shift {
                '^'
            } else {
                '6'
            }
        }
        8 => {
            if shift {
                '&'
            } else {
                '7'
            }
        }
        9 => {
            if shift {
                '*'
            } else {
                '8'
            }
        }
        10 => {
            if shift {
                '('
            } else {
                '9'
            }
        }
        11 => {
            if shift {
                ')'
            } else {
                '0'
            }
        }
        12 => {
            if shift {
                '_'
            } else {
                '-'
            }
        }
        13 => {
            if shift {
                '+'
            } else {
                '='
            }
        }

        16 => {
            if shift {
                'Q'
            } else {
                'q'
            }
        }
        17 => {
            if shift {
                'W'
            } else {
                'w'
            }
        }
        18 => {
            if shift {
                'E'
            } else {
                'e'
            }
        }
        19 => {
            if shift {
                'R'
            } else {
                'r'
            }
        }
        20 => {
            if shift {
                'T'
            } else {
                't'
            }
        }
        21 => {
            if shift {
                'Y'
            } else {
                'y'
            }
        }
        22 => {
            if shift {
                'U'
            } else {
                'u'
            }
        }
        23 => {
            if shift {
                'I'
            } else {
                'i'
            }
        }
        24 => {
            if shift {
                'O'
            } else {
                'o'
            }
        }
        25 => {
            if shift {
                'P'
            } else {
                'p'
            }
        }
        26 => {
            if shift {
                '{'
            } else {
                '['
            }
        }
        27 => {
            if shift {
                '}'
            } else {
                ']'
            }
        }

        30 => {
            if shift {
                'A'
            } else {
                'a'
            }
        }
        31 => {
            if shift {
                'S'
            } else {
                's'
            }
        }
        32 => {
            if shift {
                'D'
            } else {
                'd'
            }
        }
        33 => {
            if shift {
                'F'
            } else {
                'f'
            }
        }
        34 => {
            if shift {
                'G'
            } else {
                'g'
            }
        }
        35 => {
            if shift {
                'H'
            } else {
                'h'
            }
        }
        36 => {
            if shift {
                'J'
            } else {
                'j'
            }
        }
        37 => {
            if shift {
                'K'
            } else {
                'k'
            }
        }
        38 => {
            if shift {
                'L'
            } else {
                'l'
            }
        }
        39 => {
            if shift {
                ':'
            } else {
                ';'
            }
        }
        40 => {
            if shift {
                '"'
            } else {
                '\''
            }
        }

        44 => {
            if shift {
                'Z'
            } else {
                'z'
            }
        }
        45 => {
            if shift {
                'X'
            } else {
                'x'
            }
        }
        46 => {
            if shift {
                'C'
            } else {
                'c'
            }
        }
        47 => {
            if shift {
                'V'
            } else {
                'v'
            }
        }
        48 => {
            if shift {
                'B'
            } else {
                'b'
            }
        }
        49 => {
            if shift {
                'N'
            } else {
                'n'
            }
        }
        50 => {
            if shift {
                'M'
            } else {
                'm'
            }
        }
        51 => {
            if shift {
                '<'
            } else {
                ','
            }
        }
        52 => {
            if shift {
                '>'
            } else {
                '.'
            }
        }
        53 => {
            if shift {
                '?'
            } else {
                '/'
            }
        }

        57 => ' ',
        _ => return None,
    };
    Some(c)
}
