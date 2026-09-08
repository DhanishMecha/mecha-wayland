//! The seat: the compositor's pointer, reported to the interactivity module
//! as `PointerInput` in the window's own coordinates.
//!
//! The seat announces its capabilities after it is bound, so the pointer is
//! taken when they arrive and released when they go. A pointer event names
//! its surface only on enter; the window it entered is remembered until it
//! leaves, and motion and buttons are reported against it. Button codes are
//! Linux's, which `MouseButton` reads.

use app::{App, Module, NodeId, Resource};
use interactivity::{Input, MouseButton, PointerInput};
use utils::Point;
use wayland::{WlPointer, WlPointerButtonState, WlPointerEvent, WlSeatCapability, WlSeatEvent};

use crate::WindowOf;

/// The seat's pointer, once the seat has offered one, and the window it is
/// over.
#[derive(Default)]
pub struct Seat {
    pointer: Option<wayland::Proxy<WlPointer>>,
    focus: Option<NodeId>,
}
impl Resource for Seat {}

impl Seat {
    /// The window the pointer is over, if any.
    pub fn focus(&self) -> Option<NodeId> {
        self.focus
    }
}

pub struct SeatModule;

impl Module for SeatModule {
    fn install(self, app: &mut App) {
        app.insert_resource(Seat::default())
            .system(on_seat_event)
            .system(on_pointer_event);
    }
}

fn on_seat_event(app: &mut App, ev: &WlSeatEvent) {
    let WlSeatEvent::Capabilities {
        sender,
        capabilities,
    } = ev
    else {
        return;
    };
    let mut seat = app
        .resource_mut::<Seat>()
        .expect("no Seat holder is alive across a seat event");
    let has_pointer = capabilities.contains(WlSeatCapability::Pointer);
    match (&seat.pointer, has_pointer) {
        (None, true) => seat.pointer = Some(sender.get_pointer()),
        (Some(p), false) => {
            if p.is_alive() {
                p.release();
            }
            seat.pointer = None;
            seat.focus = None;
        }
        _ => {}
    }
}

fn on_pointer_event(app: &mut App, ev: &WlPointerEvent) {
    let report = {
        let mut seat = app
            .resource_mut::<Seat>()
            .expect("no Seat holder is alive across a pointer event");
        match ev {
            WlPointerEvent::Enter {
                surface,
                surface_x,
                surface_y,
                ..
            } => {
                seat.focus = app
                    .resource::<WindowOf>()
                    .expect("no WindowOf holder is alive across a pointer event")
                    .get(surface);
                seat.focus.map(|window| PointerInput {
                    window,
                    input: Input::Enter(Point::new(*surface_x, *surface_y)),
                })
            }
            WlPointerEvent::Leave { .. } => seat.focus.take().map(|window| PointerInput {
                window,
                input: Input::Leave,
            }),
            WlPointerEvent::Motion {
                surface_x,
                surface_y,
                ..
            } => seat.focus.map(|window| PointerInput {
                window,
                input: Input::Move(Point::new(*surface_x, *surface_y)),
            }),
            WlPointerEvent::Button { button, state, .. } => seat.focus.map(|window| {
                let button = MouseButton::from(*button);
                PointerInput {
                    window,
                    input: match state {
                        WlPointerButtonState::Pressed => Input::Press(button),
                        WlPointerButtonState::Released => Input::Release(button),
                    },
                }
            }),
            _ => None,
        }
    };
    if let Some(report) = report {
        app.signal(report);
    }
}
