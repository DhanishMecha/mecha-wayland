use super::button::Button;
use super::circle::Circle;
use crate::atlas;
use renderer::commands::Color;
use taffy::prelude::*;
use taffy::{Size, Style};
use ui::widgets::{Div, Text};
use ui::{Damage, OnChange, Point, Render, RenderCommand, Widget, WidgetList};
use utils::Rect as UtilsRect;

/// Event payload for PIN keypad click interaction at point `p`.
#[derive(Clone, Copy, Debug)]
pub struct PinClick(pub Point);

/// Event payload for PIN keypad release interaction.
#[derive(Clone, Copy, Debug)]
pub struct PinRelease;

/// Event payload for PIN keypad hover interaction at point `p`.
#[derive(Clone, Copy, Debug)]
pub struct PinHover(pub Option<Point>);

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
            top: length(16.0_f32),
            bottom: length(16.0_f32),
            left: zero(),
            right: zero(),
        },
        ..Style::default()
    }
}

/// Layout style for the keypad grid layout.
fn keypad_grid_style() -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        align_items: Some(AlignItems::Center),
        gap: Size {
            width: zero(),
            height: length(12.0_f32),
        },
        ..Style::default()
    }
}

/// Layout style for each row of keypad buttons.
fn keypad_row_style() -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        gap: Size {
            width: length(10.0_f32),
            height: zero(),
        },
        ..Style::default()
    }
}

/// Layout style for the empty button spacer.
fn empty_button_style() -> Style {
    Style {
        size: Size {
            width: length(152.0_f32),
            height: length(56.0_f32),
        },
        ..Style::default()
    }
}

/// Layout style for the backspace button.
fn backspace_button_style() -> Style {
    Style {
        display: Display::Flex,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: length(152.0_f32),
            height: length(56.0_f32),
        },
        ..Style::default()
    }
}

/// Layout style for the cancel button container.
fn cancel_container_style() -> Style {
    Style {
        display: Display::Flex,
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        size: Size {
            width: percent(1.0_f32),
            height: auto(),
        },
        margin: Rect {
            top: length(8.0_f32),
            bottom: zero(),
            left: zero(),
            right: zero(),
        },
        ..Style::default()
    }
}

type KeypadRow3 = Div<(Button, Button, Button)>;
type KeypadRow4 = Div<(Div<()>, Button, Button)>;

type CirclesPair = (Circle, Circle);
type CirclesRow = Div<(CirclesPair, CirclesPair)>;

type GridPair1 = (KeypadRow3, KeypadRow3);
type GridPair2 = (KeypadRow3, KeypadRow4);
type KeypadGrid = Div<(GridPair1, GridPair2)>;

type PinHeader = (Text, CirclesRow);
type PinFooter = (KeypadGrid, Div<(Text,)>);
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
        title.font = Some(&atlas::LOCKSCREEN_FONT_INTER_12);
        title.color = Color::from_rgb8(140, 140, 145);
        title.text = "ENTER PIN TO UNLOCK".into();

        let circles = Div::new(
            circles_row_style(),
            (
                (Circle::new(), Circle::new()),
                (Circle::new(), Circle::new()),
            ),
        );

        let row1 = Div::new(
            keypad_row_style(),
            (Button::new("1"), Button::new("2"), Button::new("3")),
        );
        let row2 = Div::new(
            keypad_row_style(),
            (Button::new("4"), Button::new("5"), Button::new("6")),
        );
        let row3 = Div::new(
            keypad_row_style(),
            (Button::new("7"), Button::new("8"), Button::new("9")),
        );

        let empty_space = Div::new(empty_button_style(), ());
        let row4 = Div::new(
            keypad_row_style(),
            (
                empty_space,
                Button::new("0"),
                Button::transparent_with_style("BACKSPACE", backspace_button_style()),
            ),
        );

        let grid = Div::new(keypad_grid_style(), ((row1, row2), (row3, row4)));

        let mut cancel_txt = Text::new(Style::default());
        cancel_txt.font = Some(&atlas::LOCKSCREEN_FONT_INTER_12);
        cancel_txt.color = Color::from_rgb8(140, 140, 145);
        cancel_txt.text = "CANCEL".into();

        let cancel_btn = Div::new(cancel_container_style(), (cancel_txt,));

        let container = Div::new(
            pin_container_style(),
            ((title, circles), (grid, cancel_btn)),
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
        let grid = &self.children.children.1.0;
        let row1 = &grid.children.0.0;
        let row2 = &grid.children.0.1;
        let row3 = &grid.children.1.0;
        let row4 = &grid.children.1.1;

        if row1.children.0.bounds().contains_point(p) {
            Some(1)
        } else if row1.children.1.bounds().contains_point(p) {
            Some(2)
        } else if row1.children.2.bounds().contains_point(p) {
            Some(3)
        } else if row2.children.0.bounds().contains_point(p) {
            Some(4)
        } else if row2.children.1.bounds().contains_point(p) {
            Some(5)
        } else if row2.children.2.bounds().contains_point(p) {
            Some(6)
        } else if row3.children.0.bounds().contains_point(p) {
            Some(7)
        } else if row3.children.1.bounds().contains_point(p) {
            Some(8)
        } else if row3.children.2.bounds().contains_point(p) {
            Some(9)
        } else if row4.children.1.bounds().contains_point(p) {
            Some(0)
        } else if row4.children.2.bounds().contains_point(p) {
            Some(10)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the `Button` associated with the specified digit.
    fn button_mut(&mut self, digit: u8) -> Option<&mut Button> {
        let grid = &mut self.children.children.1.0;
        match digit {
            1 => Some(&mut grid.children.0.0.children.0),
            2 => Some(&mut grid.children.0.0.children.1),
            3 => Some(&mut grid.children.0.0.children.2),
            4 => Some(&mut grid.children.0.1.children.0),
            5 => Some(&mut grid.children.0.1.children.1),
            6 => Some(&mut grid.children.0.1.children.2),
            7 => Some(&mut grid.children.1.0.children.0),
            8 => Some(&mut grid.children.1.0.children.1),
            9 => Some(&mut grid.children.1.0.children.2),
            0 => Some(&mut grid.children.1.1.children.1),
            10 => Some(&mut grid.children.1.1.children.2),
            _ => None,
        }
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

impl Default for PinWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl OnChange<PinHover> for PinWidget {
    fn damage(&self, _new: &PinHover) -> Damage {
        Damage::None
    }

    /// Hover handler for PIN keypad - hover color effects removed.
    fn change(&mut self, _new: PinHover) {}
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

        let grid = &self.children.children.1.0;
        let row1 = &grid.children.0.0;
        let row2 = &grid.children.0.1;
        let row3 = &grid.children.1.0;
        let row4 = &grid.children.1.1;

        let mut digit_pressed = None;

        if row1.children.0.own_bounds().contains_point(pt) {
            digit_pressed = Some('1');
        } else if row1.children.1.own_bounds().contains_point(pt) {
            digit_pressed = Some('2');
        } else if row1.children.2.own_bounds().contains_point(pt) {
            digit_pressed = Some('3');
        } else if row2.children.0.own_bounds().contains_point(pt) {
            digit_pressed = Some('4');
        } else if row2.children.1.own_bounds().contains_point(pt) {
            digit_pressed = Some('5');
        } else if row2.children.2.own_bounds().contains_point(pt) {
            digit_pressed = Some('6');
        } else if row3.children.0.own_bounds().contains_point(pt) {
            digit_pressed = Some('7');
        } else if row3.children.1.own_bounds().contains_point(pt) {
            digit_pressed = Some('8');
        } else if row3.children.2.own_bounds().contains_point(pt) {
            digit_pressed = Some('9');
        } else if row4.children.1.own_bounds().contains_point(pt) {
            digit_pressed = Some('0');
        } else if row4.children.2.own_bounds().contains_point(pt) {
            if !self.pin.is_empty() {
                self.pin.pop();
                self.update_circles();
                self.validate_pin();
            }
            return;
        }

        if let Some(ch) = digit_pressed {
            if self.pin.len() == 4 {
                self.pin.clear();
            }
            if self.pin.len() < 4 {
                self.pin.push(ch);
                self.update_circles();
                self.validate_pin();
            }
        }
    }
}
