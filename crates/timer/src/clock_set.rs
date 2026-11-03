//! Wall-clock set notifications: a [`ClockSetFd`] kept polled on the
//! ring, and a [`WallClockSet`] emitted at every node when it fires.
//! Opt-in through [`WallClockModule`], so only apps that show wall-clock
//! time hold the fd.

use std::{os::fd::AsFd, println};

use app::prelude::*;
use ring::prelude::*;

/// The wall clock was set discontinuously: `clock_settime`, an NTP step,
/// or resume from suspend. Emitted at every node. Timers keep counting on
/// the monotonic clock, so a widget that aligns to wall-clock time
/// re-arms here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct WallClockSet;
impl Event for WallClockSet {}

/// Watches for the wall clock being set and emits [`WallClockSet`] at
/// every node when it is. Installs after `RingModule`; independent of
/// `TimerModule`, though the widgets that react usually use both.
///
/// ```ignore
/// app.add_module(RingModule::default())
///     .add_module(TimerModule)
///     .add_module(WallClockModule);
/// ```
pub struct WallClockModule;

impl Module for WallClockModule {
    fn install(self, app: &mut App) {
        assert!(
            app.has_resource::<Ring>(),
            "WallClockModule installs after RingModule"
        );
        if app.has_resource::<ClockWatch>() {
            return;
        }
        app.insert_resource(ClockWatch::new());
        app.system(submit).system(complete);
    }
}

/// The fd and its poll in flight. `fd` is `None` if the kernel refused
/// it or its poll failed; notifications then stop and timers still run.
struct ClockWatch {
    fd: Option<ClockSetFd>,
    token: Option<Token>,
}
impl Resource for ClockWatch {}

impl ClockWatch {
    fn new() -> Self {
        ClockWatch {
            fd: ClockSetFd::new().ok(),
            token: None,
        }
    }
}

/// Keep one poll on the fd in flight.
fn submit(app: &mut App, _: &BeforeWait) {
    if app.resource::<ClockWatch>().token.is_some() {
        return;
    }
    println!("Submit 1");
    // Out of the resource while the ring borrows the app; put back below.
    let Some(fd) = app.resource_mut::<ClockWatch>().fd.take() else {
        return;
    };
    println!("Submit 2");

    let token = app
        .resource_mut::<Ring>()
        .poll(fd.as_fd(), libc::POLLIN as u32);
    println!("Submit 3");

    let watch = &mut *app.resource_mut::<ClockWatch>();
    watch.fd = Some(fd);
    watch.token = Some(token);
}

/// Finish our poll; tell every node if the clock was set.
fn complete(app: &mut App, e: &IoEvent) {
    if app.resource::<ClockWatch>().token != Some(e.token) {
        return;
    }
    println!("Complete 1");

    app.resource_mut::<Ring>().finish(e.token);
    let watch = &mut *app.resource_mut::<ClockWatch>();
    watch.token = None;
    if e.result < 0 {
        watch.fd = None;
        return;
    }
    println!("Complete 2");

    let set = match watch.fd.as_ref().map(ClockSetFd::reset) {
        Some(Ok(set)) => set,
        _ => {
            watch.fd = None;
            false
        }
    };
    println!("Complete 3");

    if set {
        app.emit_all(WallClockSet);
    }
}
