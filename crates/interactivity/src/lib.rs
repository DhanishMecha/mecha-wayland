//! The interactivity module: pointer input in, [`Clicked`] out.
//!
//! Nothing here knows a compositor. The presenting side reports what a
//! pointer did at a window as a [`PointerInput`] signal, in that window's
//! own coordinates. This module keeps one [`Pointer`] per window from those
//! reports and, on a press, finds every node under the window whose box
//! contains the point and emits [`Clicked`] there, deepest and topmost
//! first, the window itself last. A widget that wants to be clicked
//! registers a handler for `Clicked` and nothing else; one that does not
//! is passed over, so a label inside a button costs the button nothing.
//!
//! A click is a press, as it was in the previous stack: the button going
//! down over the box, not the release. Only the pointer is handled for now.
//! Touch, keyboard and gestures come back the same way, as signals from
//! the presenting side.
//!
//! Added after `LayoutModule`, whose `Layout` is what a box is.

use std::collections::HashMap;

use app::{App, Component, Event, Module, NodeId, Signal};
use layout::Layout;
use utils::Point;

pub mod prelude {
    pub use crate::{Clicked, Input, InteractivityModule, MouseButton, Pointer, PointerInput};
}

// ---------------------------------------------------------------------------
// Input from the presenting side
// ---------------------------------------------------------------------------

/// What a pointer did at a window, in that window's own coordinates.
/// Signalled by the presenting side and by nothing else. `window` is the
/// node whose surface the pointer is over; a stale one is ignored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerInput {
    pub window: NodeId,
    pub input: Input,
}
impl Signal for PointerInput {}

/// One thing a pointer does. A press or release before any `Enter` has no
/// position to happen at and is ignored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    /// The pointer came over the window, here.
    Enter(Point),
    Move(Point),
    /// The pointer went away. Whatever it held is forgotten, since the
    /// release will be reported to whoever has it now.
    Leave,
    Press(MouseButton),
    Release(MouseButton),
}

/// A pointer button, by name where Linux has one. `From<u32>` reads the
/// `BTN_*` codes of `input-event-codes.h`, which is what a compositor
/// reports; the presenting side converts, so nothing here sees a code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Side,
    Extra,
    Forward,
    Back,
    Task,
    /// Linux `BTN_0..BTN_9`.
    Numbered(u8),
    /// Linux `BTN_TRIGGER_HAPPY1..40`.
    ExtraButton(u8),
    /// Anything else, kept rather than dropped.
    Unknown(u32),
}

impl From<u32> for MouseButton {
    fn from(code: u32) -> Self {
        match code {
            0x110 => MouseButton::Left,
            0x111 => MouseButton::Right,
            0x112 => MouseButton::Middle,
            0x113 => MouseButton::Side,
            0x114 => MouseButton::Extra,
            0x115 => MouseButton::Forward,
            0x116 => MouseButton::Back,
            0x117 => MouseButton::Task,
            0x100..=0x109 => MouseButton::Numbered((code - 0x100) as u8),
            0x2c0..=0x2e7 => MouseButton::ExtraButton((code - 0x2c0 + 1) as u8),
            other => MouseButton::Unknown(other),
        }
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// A window's pointer: where it is and what it holds, in the window's own
/// coordinates. Dense, like every component, but written only at window
/// nodes; everywhere else it stays the default, a pointer that is not
/// there.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pointer {
    position: Option<Point>,
    pressed: HashMap<MouseButton, Point>,
}
impl Component for Pointer {}

impl Pointer {
    /// Where the pointer is; `None` while it is not over the window.
    pub fn position(&self) -> Option<Point> {
        self.position
    }

    /// Whether `button` is held.
    pub fn is_pressed(&self, button: MouseButton) -> bool {
        self.pressed.contains_key(&button)
    }

    /// Where `button` went down, while it is held.
    pub fn pressed_at(&self, button: MouseButton) -> Option<Point> {
        self.pressed.get(&button).copied()
    }

    /// Every held button and where it went down, in no particular order.
    pub fn pressed(&self) -> impl Iterator<Item = (MouseButton, Point)> + '_ {
        self.pressed.iter().map(|(b, p)| (*b, *p))
    }
}

// ---------------------------------------------------------------------------
// Event
// ---------------------------------------------------------------------------

/// A button went down over this node. Emitted at every node under the
/// window whose box contains the point, deepest and topmost first and the
/// window last, so a handler on a container sees the clicks on everything
/// inside it. `position` is where, in the window's coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clicked {
    pub button: MouseButton,
    pub position: Point,
}
impl Event for Clicked {}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

/// Registers [`Pointer`] and the system on [`PointerInput`] that keeps it
/// and emits [`Clicked`]. Added after `LayoutModule`.
pub struct InteractivityModule;

impl Module for InteractivityModule {
    fn install(self, app: &mut App) {
        app.register_component::<Pointer>().system(on_pointer_input);
    }
}

/// Every input updates the window's pointer; a press over the window is
/// also a click, at whatever is under it.
fn on_pointer_input(app: &mut App, report: &PointerInput) {
    let press = {
        let mut pointers = app
            .components_mut::<Pointer>()
            .expect("no Pointer holder is alive across a signal");
        let Some(pointer) = pointers.get_mut(report.window) else {
            return;
        };
        match report.input {
            Input::Enter(at) | Input::Move(at) => {
                pointer.position = Some(at);
                None
            }
            Input::Leave => {
                pointer.position = None;
                pointer.pressed.clear();
                None
            }
            Input::Press(button) => pointer.position.map(|at| {
                pointer.pressed.insert(button, at);
                (button, at)
            }),
            Input::Release(button) => {
                pointer.pressed.remove(&button);
                None
            }
        }
    };
    let Some((button, position)) = press else {
        return;
    };
    let targets = hit(app, report.window, position);
    if !targets.is_empty() {
        app.emit(Clicked { button, position }, &targets);
    }
}

/// Every node under `window` whose box contains `at`, deepest and topmost
/// first: the reverse of a preorder walk, so a child comes before its
/// parent and a later sibling, drawn over an earlier one, before it. A
/// node with an empty box is never hit, and a subtree is walked whether or
/// not its root was, since an absolute child may lie outside its parent.
///
/// `at` is in the window's coordinates and the boxes are absolute, so it
/// is offset by the window's own origin before the comparison.
fn hit(app: &App, window: NodeId, at: Point) -> Vec<NodeId> {
    let layouts = app
        .components::<Layout>()
        .expect("no Layout writer is alive across a signal");
    let Some(origin) = layouts.get(window).map(|l| l.rect.origin) else {
        return Vec::new();
    };
    let at = Point(at.0 + origin.0);
    let mut hits = Vec::new();
    let mut stack = vec![window];
    while let Some(id) = stack.pop() {
        if layouts
            .get(id)
            .is_some_and(|l| !l.rect.is_empty() && l.rect.contains_point(at))
        {
            hits.push(id);
        }
        if let Some(children) = app.children(id) {
            stack.extend(children.iter().rev());
        }
    }
    hits.reverse();
    hits
}
