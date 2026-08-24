use super::bottombar::BottomBar;
use super::button::Button;
use super::circle::Circle;
use super::numpad::{Numpad, NumpadAction};
use crate::atlas;
use renderer::commands::Color;
use taffy::prelude::*;
use taffy::{Size, Style};
use ui::widgets::{Div, Text};
use ui::{Damage, OnChange, Point, Render, RenderCommand, Widget};
use utils::Rect as UtilsRect;

/// Event payload for PIN keypad click interaction at point `p`.
#[derive(Clone, Copy, Debug)]
pub struct PinClick(pub Point);

/// Event payload for PIN keypad release interaction.
#[derive(Clone, Copy, Debug)]
pub struct PinRelease;

/// Layout style for the primary PIN widget container.
fn pin_container_style() -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        align_items: Some(AlignItems::Center),
        justify_content: Some(JustifyContent::Center),
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        gap: Size {
            width: zero(),
            height: length(24.0_f32),
        },
        ..Style::default()
    }
}

/// Layout style for the row holding the PIN circle indicators.
fn circles_row_style() -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        align_items: Some(AlignItems::Center),
        justify_content: Some(JustifyContent::Center),
        gap: Size {
            width: length(16.0_f32),
            height: zero(),
        },
        margin: Rect {
            top: length(8.0_f32),
            bottom: length(16.0_f32),
            left: zero(),
            right: zero(),
        },
        ..Style::default()
    }
}

type CirclesPair = (Circle, Circle);
type CirclesRow = Div<(CirclesPair, CirclesPair)>;

type PinHeader = (Text, CirclesRow);
type PinFooter = (Numpad, BottomBar);
type PinContainer = Div<(PinHeader, PinFooter)>;

/// Widget representing the PIN entry screen, including keypad buttons and indicator circles.
#[ui::widget]
pub struct PinWidget {
    pub pin: String,
    pub want_back: bool,
    pub active_button: Option<u8>,
    #[widget(child)]
    pub children: PinContainer,
}

impl Render for PinWidget {
    fn render(&self, _layout: &taffy::Layout, _abs_pos: Point) -> Vec<RenderCommand> {
        Vec::new()
    }
}

impl PinWidget {
    /// Creates a new `PinWidget` instance with initialized layout and keypad controls.
    pub fn new() -> Self {
        let mut title = Text::new(Style::default());
        title.font = Some(&atlas::LOCKSCREEN_FONT_GEIST_MONO_12);
        title.color = Color::from_rgb8(140, 140, 145);
        title.text = "ENTER PIN TO UNLOCK".into();

        let circles = Div::new(
            circles_row_style(),
            (
                (Circle::new(), Circle::new()),
                (Circle::new(), Circle::new()),
            ),
        );

        let numpad = Numpad::new();

        let cancel_btn = BottomBar::new("CANCEL");

        let container = Div::new(
            pin_container_style(),
            ((title, circles), (numpad, cancel_btn)),
        );

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style: Style {
                display: Display::None,
                size: Size {
                    width: percent(1.0_f32),
                    height: auto(),
                },
                padding: Rect {
                    left: length(32.0_f32),
                    right: length(32.0_f32),
                    top: zero(),
                    bottom: zero(),
                },
                ..Style::default()
            },
            bounds: UtilsRect::ZERO,
            pending_damage: Damage::None,
            is_opaque: true,
            pin: String::new(),
            want_back: false,
            active_button: None,
            children: container,
        }
    }

    fn hit(&self, p: Point) -> Option<u8> {
        self.children.children.1.0.hit(p)
    }

    /// Returns a mutable reference to the `Button` associated with the specified digit.
    fn button_mut(&mut self, digit: u8) -> Option<&mut Button> {
        self.children.children.1.0.button_mut(digit)
    }

    /// Updates the fill colors of the 4 PIN circle indicators according to current PIN length.
    fn update_circles(&mut self) {
        let len = self.pin.len();
        self.children.children.0.1.children.0.0.set(len >= 1);
        self.children.children.0.1.children.0.1.set(len >= 2);
        self.children.children.0.1.children.1.0.set(len >= 3);
        self.children.children.0.1.children.1.1.set(len >= 4);
    }

    /// Validates the current PIN string when 4 digits are entered and updates status text.
    fn validate_pin(&mut self) {
        if self.pin.len() == 4 {
            if self.pin == "1234" {
                self.children.children.0.0.set("UNLOCKED".to_string());
                self.children.children.0.0.color = Color::from_rgb8(242, 242, 242);
            } else {
                self.children.children.0.0.set("PIN INCORRECT".to_string());
                self.children.children.0.0.color = Color::from_rgb8(242, 242, 242);
                self.pin.clear();
                self.update_circles();
            }
        } else {
            self.children
                .children
                .0
                .0
                .set("ENTER PIN TO UNLOCK".to_string());
            self.children.children.0.0.color = Color::from_rgb8(140, 140, 145);
        }
    }
}

impl OnChange<PinRelease> for PinWidget {
    fn damage(&self, _new: &PinRelease) -> Damage {
        Damage::None
    }

    /// Resets active button color on click release.
    fn change(&mut self, _new: PinRelease) {
        if let Some(old) = self.active_button.take() {
            if let Some(btn) = self.button_mut(old) {
                btn.set(false);
            }
        }
    }
}

impl OnChange<PinClick> for PinWidget {
    fn damage(&self, _new: &PinClick) -> Damage {
        Damage::None
    }

    /// Processes clicks on numeric buttons (0-9), backspace, and cancel controls.
    fn change(&mut self, new: PinClick) {
        let pt = new.0;

        let target = self.hit(pt);
        if let Some(old) = self.active_button {
            if let Some(btn) = self.button_mut(old) {
                btn.set(false);
            }
        }
        if let Some(t) = target {
            if let Some(btn) = self.button_mut(t) {
                btn.set(true);
            }
        }
        self.active_button = target;

        let cancel_btn = &self.children.children.1.1;
        if cancel_btn.own_bounds().contains_point(pt) {
            self.pin.clear();
            self.update_circles();
            self.validate_pin();
            self.want_back = true;
            return;
        }

        let numpad = &self.children.children.1.0;
        if let Some(action) = numpad.hit_action(pt) {
            match action {
                NumpadAction::Digit(ch) => {
                    if self.pin.len() == 4 {
                        self.pin.clear();
                    }
                    if self.pin.len() < 4 {
                        self.pin.push(ch);
                        self.update_circles();
                        self.validate_pin();
                    }
                }
                NumpadAction::Backspace => {
                    if !self.pin.is_empty() {
                        self.pin.pop();
                        self.update_circles();
                        self.validate_pin();
                    }
                }
            }
        }
    }
}
