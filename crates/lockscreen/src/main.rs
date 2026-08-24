pub mod atlas {
    include!(concat!(env!("OUT_DIR"), "/lockscreen_gen.rs"));
}

pub mod widgets;

use app::prelude::*;
use io_ring::Ring;
use renderer::commands::Color;

use taffy::prelude::*;
use taffy::{Size, Style};
use timer::{Absolute, Clock, Timer, TimerEvent};
use ui::{Damage, OnChange, Point, Render, RenderCommand, Widget};
use utils::Rect as UtilsRect;
use wayland::{WlPointerButtonState, WlPointerEvent, WlTouchEvent};
use window_manager::prelude::*;

use widgets::{
    BottomBar, DateTime, DateTimeChanged, DateTimeTick, DateTimeUpdate, PinClick, PinRelease,
    PinWidget, TimeWidget,
};

const BG: Color = Color::from_rgb8(0, 0, 0);
const BTN_LEFT: u32 = 0x110;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenState {
    HoldToExpand,
    PinLock,
}

fn root_style() -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::SpaceBetween),
        align_items: Some(AlignItems::Start),
        size: Size {
            width: percent(1.0_f32),
            height: percent(1.0_f32),
        },
        padding: Rect {
            left: length(48.0_f32),
            right: length(48.0_f32),
            top: length(48.0_f32),
            bottom: length(48.0_f32),
        },
        gap: Size {
            width: zero(),
            height: zero(),
        },
        ..Style::default()
    }
}

#[ui::widget]
struct Lockscreen {
    screen: ScreenState,
    #[widget(child)]
    children: (TimeWidget, PinWidget, BottomBar),
}

impl Render for Lockscreen {
    fn render(&self, _layout: &taffy::Layout, _abs_pos: Point) -> Vec<RenderCommand> {
        Vec::new()
    }
}

impl Lockscreen {
    fn new() -> Self {
        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style: root_style(),
            bounds: UtilsRect::ZERO,
            pending_damage: Damage::None,
            is_opaque: true,
            screen: ScreenState::HoldToExpand,
            children: (
                TimeWidget::new(),
                PinWidget::new(),
                BottomBar::new("HOLD TO EXPAND"),
            ),
        }
    }
}

/// Updates date and time.
impl OnChange<DateTimeUpdate> for Lockscreen {
    fn damage(&self, _new: &DateTimeUpdate) -> Damage {
        Damage::None
    }

    fn change(&mut self, new: DateTimeUpdate) {
        self.children.0.set(new);
    }
}

/// Handles click interactions and state transitions for the Lockscreen.
impl OnChange<PinClick> for Lockscreen {
    fn damage(&self, _new: &PinClick) -> Damage {
        Damage::None
    }

    fn change(&mut self, new: PinClick) {
        match self.screen {
            ScreenState::HoldToExpand => {
                self.screen = ScreenState::PinLock;

                let mut pin_style = self.children.1.style().clone();
                pin_style.display = Display::Flex;
                self.children.1.set(pin_style);

                self.children.2.set(self.screen);
            }
            ScreenState::PinLock => {
                if self.children.2.own_bounds().contains_point(new.0) {
                    self.children.1.reset();
                    self.screen = ScreenState::HoldToExpand;

                    let mut pin_style = self.children.1.style().clone();
                    pin_style.display = Display::None;
                    self.children.1.set(pin_style);

                    self.children.2.set(self.screen);
                    return;
                }

                self.children.1.set(new);
            }
        }
    }
}

/// Handles PIN keypad release.
impl OnChange<PinRelease> for Lockscreen {
    fn damage(&self, _new: &PinRelease) -> Damage {
        Damage::None
    }

    fn change(&mut self, new: PinRelease) {
        self.children.1.set(new);
    }
}

#[derive(State)]
struct LockscreenState {
    ring: Ring,
    wm: WindowManager,
    timer: Timer,
    #[lens(skip)]
    handle: WindowHandle<Lockscreen>,
    #[lens(skip)]
    pointer: Point,
    datetime: DateTime,
}

fn main() {
    let ring = Ring::default();
    let mut wm = WindowManager::new(ring.proxy());
    wm.upload_atlas(&atlas::LOCKSCREEN);

    let handle = wm.spawn_window(
        WindowSettings {
            width: 540,
            height: 620,
            clear_color: BG,
            kind: WindowKind::Xdg {
                title: "lockscreen".into(),
            },
            touch_config: None,
            gesture_config: None,
        },
        Lockscreen::new(),
    );

    let timer = Timer::new(ring.proxy());

    let state = LockscreenState {
        ring,
        wm,
        timer,
        handle,
        pointer: Point::new(-1.0, -1.0),
        datetime: DateTime::new(),
    };

    let mut app = app::App::new(state)
        .mount(io_ring::module())
        .mount(window_manager::module())
        .mount(timer::module())
        .mount(DateTime::module())
        .mount(
            app::Module::new()
                .on(on_pointer)
                .on(on_touch)
                .on(on_start)
                .on(on_timer)
                .on(on_datetime_changed),
        );

    app.dispatch(&app::Start);
    loop {
        app.dispatch(&app::PrePoll);
        app.dispatch(&app::Poll);
    }
}

fn schedule_next_update(s: &mut LockscreenState) {
    let next_at = s.datetime.next_deadline();
    s.timer.start_deadline(Absolute {
        at: next_at,
        clock: Clock::Realtime,
    });
}

fn on_start(s: &mut LockscreenState, _: &app::Start) -> DateTimeTick {
    schedule_next_update(s);
    DateTimeTick
}

fn on_timer(s: &mut LockscreenState, _ev: &TimerEvent) -> DateTimeTick {
    schedule_next_update(s);
    DateTimeTick
}

fn on_datetime_changed(s: &mut LockscreenState, _: &DateTimeChanged) {
    update_time(s);
}

fn update_time(s: &mut LockscreenState) {
    let time = DateTime::format("%H:%M");
    let date = DateTime::format("%a %d");
    s.handle.set(DateTimeUpdate { time, date }, &mut s.wm);
}

fn handle_click(s: &mut LockscreenState) {
    s.handle.set(PinClick(s.pointer), &mut s.wm);
}

fn handle_release(s: &mut LockscreenState) {
    s.handle.set(PinRelease, &mut s.wm);
}

fn on_touch(s: &mut LockscreenState, ev: &WlTouchEvent) {
    match ev {
        WlTouchEvent::Down { x, y, .. } => {
            s.pointer = Point::new(*x, *y);
            handle_click(s);
        }
        WlTouchEvent::Motion { x, y, .. } => {
            s.pointer = Point::new(*x, *y);
        }
        WlTouchEvent::Up { .. } | WlTouchEvent::Cancel { .. } => {
            handle_release(s);
        }
        _ => {}
    }
}

fn on_pointer(s: &mut LockscreenState, ev: &WlPointerEvent) {
    match ev {
        WlPointerEvent::Enter {
            surface_x,
            surface_y,
            ..
        }
        | WlPointerEvent::Motion {
            surface_x,
            surface_y,
            ..
        } => {
            s.pointer = Point::new(*surface_x, *surface_y);
        }
        WlPointerEvent::Leave { .. } => {
            handle_release(s);
        }
        WlPointerEvent::Button {
            state: WlPointerButtonState::Pressed,
            button,
            ..
        } if *button == BTN_LEFT => {
            handle_click(s);
        }
        WlPointerEvent::Button {
            state: WlPointerButtonState::Released,
            button,
            ..
        } if *button == BTN_LEFT => {
            handle_release(s);
        }
        _ => {}
    }
}

