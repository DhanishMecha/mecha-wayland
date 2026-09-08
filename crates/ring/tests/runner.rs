//! A module arms a `Timeout` at install, counts the `IoEvent`s carrying its
//! token, and signals `Stop` on the second `Tick`. `run` returns, the count
//! is one, and `BeforeWait` was signalled once per batch.

use std::time::Duration;

use app::prelude::*;
use ring::{BeforeWait, IoEvent, Ring, RingModule, Stop, Token};

struct Counts {
    armed: Token,
    completed: u32,
    ticks: u32,
    waits: u32,
}
impl Resource for Counts {}

struct Stub;

impl Module for Stub {
    fn install(self, app: &mut App) {
        let armed = app
            .resource_mut::<Ring>()
            .unwrap()
            .timeout(Duration::from_millis(5));
        app.insert_resource(Counts {
            armed,
            completed: 0,
            ticks: 0,
            waits: 0,
        })
        .system(|app: &mut App, ev: &IoEvent| {
            let mut counts = app.resource_mut::<Counts>().unwrap();
            if ev.token == counts.armed {
                counts.completed += 1;
            }
        })
        .system(|app: &mut App, _: &Tick| {
            let ticks = {
                let mut counts = app.resource_mut::<Counts>().unwrap();
                counts.ticks += 1;
                counts.ticks
            };
            if ticks == 2 {
                app.signal(Stop);
            }
        })
        .system(|app: &mut App, _: &BeforeWait| {
            app.resource_mut::<Counts>().unwrap().waits += 1;
        });
    }
}

#[test]
fn timeout_completes_once_and_run_returns() {
    let mut app = App::new();
    app.add_module(RingModule::default()).add_module(Stub);
    // `run` consumes the app, so the counts leave through a channel of one.
    thread_local! {
        static RESULT: std::cell::Cell<(u32, u32, u32)> = const { std::cell::Cell::new((0, 0, 0)) };
    }
    app.system(|app: &mut App, _: &Stop| {
        let c = app.resource::<Counts>().unwrap();
        RESULT.with(|r| r.set((c.completed, c.ticks, c.waits)));
    });
    app.run();
    let (completed, ticks, waits) = RESULT.with(|r| r.get());
    assert_eq!(completed, 1);
    assert_eq!(ticks, 2);
    assert_eq!(waits, 1, "the second batch's BeforeWait comes after Stop");
}

#[test]
#[should_panic(expected = "runner already set")]
fn setting_the_runner_twice_panics() {
    let mut app = App::new();
    app.set_runner(|_| {});
    app.add_module(RingModule::default());
}
