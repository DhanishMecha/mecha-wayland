use super::button::Button;
use taffy::prelude::*;
use taffy::{Size, Style};
use ui::widgets::Div;
use ui::{Damage, Point, Render, RenderCommand, Widget, WidgetList};

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

type KeypadRow3 = Div<(Button, Button, Button)>;
type KeypadRow4 = Div<(Div<()>, Button, Button)>;

type GridPair1 = (KeypadRow3, KeypadRow3);
type GridPair2 = (KeypadRow3, KeypadRow4);
type KeypadGrid = Div<(GridPair1, GridPair2)>;

/// Action triggered by interacting with the numpad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumpadAction {
    Digit(char),
    Backspace,
}

/// Widget encapsulating numeric buttons (0-9) and backspace in a grid layout.
#[ui::widget]
pub struct Numpad {
    #[widget(child)]
    pub children: KeypadGrid,
}

impl Render for Numpad {
    fn render(&self, _layout: &taffy::Layout, _abs_pos: Point) -> Vec<RenderCommand> {
        Vec::new()
    }
}

impl Numpad {
    pub fn new() -> Self {
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

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style: keypad_grid_style(),
            bounds: utils::Rect::ZERO,
            pending_damage: Damage::None,
            is_opaque: true,
            children: grid,
        }
    }

    /// Returns the button ID (1-9, 0, 10 for backspace) at point `p`.
    pub fn hit(&self, p: Point) -> Option<u8> {
        let row1 = &self.children.children.0.0;
        let row2 = &self.children.children.0.1;
        let row3 = &self.children.children.1.0;
        let row4 = &self.children.children.1.1;

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

    /// Returns the action (Digit or Backspace) at point `p`.
    pub fn hit_action(&self, p: Point) -> Option<NumpadAction> {
        let row1 = &self.children.children.0.0;
        let row2 = &self.children.children.0.1;
        let row3 = &self.children.children.1.0;
        let row4 = &self.children.children.1.1;

        if row1.children.0.own_bounds().contains_point(p) {
            Some(NumpadAction::Digit('1'))
        } else if row1.children.1.own_bounds().contains_point(p) {
            Some(NumpadAction::Digit('2'))
        } else if row1.children.2.own_bounds().contains_point(p) {
            Some(NumpadAction::Digit('3'))
        } else if row2.children.0.own_bounds().contains_point(p) {
            Some(NumpadAction::Digit('4'))
        } else if row2.children.1.own_bounds().contains_point(p) {
            Some(NumpadAction::Digit('5'))
        } else if row2.children.2.own_bounds().contains_point(p) {
            Some(NumpadAction::Digit('6'))
        } else if row3.children.0.own_bounds().contains_point(p) {
            Some(NumpadAction::Digit('7'))
        } else if row3.children.1.own_bounds().contains_point(p) {
            Some(NumpadAction::Digit('8'))
        } else if row3.children.2.own_bounds().contains_point(p) {
            Some(NumpadAction::Digit('9'))
        } else if row4.children.1.own_bounds().contains_point(p) {
            Some(NumpadAction::Digit('0'))
        } else if row4.children.2.own_bounds().contains_point(p) {
            Some(NumpadAction::Backspace)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the `Button` associated with the specified button ID.
    pub fn button_mut(&mut self, id: u8) -> Option<&mut Button> {
        match id {
            1 => Some(&mut self.children.children.0.0.children.0),
            2 => Some(&mut self.children.children.0.0.children.1),
            3 => Some(&mut self.children.children.0.0.children.2),
            4 => Some(&mut self.children.children.0.1.children.0),
            5 => Some(&mut self.children.children.0.1.children.1),
            6 => Some(&mut self.children.children.0.1.children.2),
            7 => Some(&mut self.children.children.1.0.children.0),
            8 => Some(&mut self.children.children.1.0.children.1),
            9 => Some(&mut self.children.children.1.0.children.2),
            0 => Some(&mut self.children.children.1.1.children.1),
            10 => Some(&mut self.children.children.1.1.children.2),
            _ => None,
        }
    }
}

impl Default for Numpad {
    fn default() -> Self {
        Self::new()
    }
}
