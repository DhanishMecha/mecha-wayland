//! The ring: the one place the runner waits, and the runner itself.
//!
//! [`Ring`] is a resource over the kernel's submission queue. Anything that
//! wants to be woken submits under a [`Token`] and hears the completion as
//! an [`IoEvent`] signal; a bare wake is a `Nop`, a bounded wait is a
//! `Timeout`, so the runner has no special cases. It is reached only through
//! the app, from a system or a module's install: there is no proxy.
//!
//! [`RingModule`] inserts the ring and sets the runner. The runner signals
//! `Tick`, flushes, signals [`BeforeWait`], flushes, then blocks on the ring;
//! each completion is one `IoEvent`, and the round repeats until a [`Stop`]
//! has been signalled. `BeforeWait` is where a module that buffered requests
//! during the batch sends them, so nothing written by a system on `Tick` or
//! anything `Tick` chained waits for a completion that may never come.

use std::collections::HashMap;
use std::time::Duration;

use app::{App, Module, Resource, Signal, Tick};
use io_uring::{IoUring, opcode, squeue, types::Timespec};

pub mod prelude {
    pub use crate::{BeforeWait, IoEvent, Ring, RingModule, Stop, Token};
}

/// Names one submission, so its completion can be recognised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token(pub u64);

/// One completion, as a signal. `result` is the kernel's: a byte count, a
/// zero, or a negated errno.
#[derive(Debug, Clone, Copy)]
pub struct IoEvent {
    pub token: Token,
    pub result: i32,
}
impl Signal for IoEvent {}

/// Asks the runner to return once the current batch is done. Signalled by
/// whoever decides the app is over, such as the presenting side when the
/// last window closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stop;
impl Signal for Stop {}

/// The runner is about to block on the ring: `Tick` and everything it
/// chained have run. A module that buffered requests during the batch sends
/// them here. Never signalled by the default runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeforeWait;
impl Signal for BeforeWait {}

/// Whether a [`Stop`] has been seen. Private: only the runner reads it.
#[derive(Default)]
struct Stopping(bool);
impl Resource for Stopping {}

/// The resource. Submissions go into the queue at [`Ring::push`];
/// [`Ring::wait`] submits them and blocks for at least one completion.
pub struct Ring {
    ring: IoUring,
    next_token: u64,
    /// Timespecs the kernel is still reading, by token.
    timeouts: HashMap<u64, Box<Timespec>>,
}
impl Resource for Ring {}

impl Ring {
    pub fn new(entries: u32) -> Self {
        Self {
            ring: IoUring::new(entries).expect("failed to create io_uring"),
            next_token: 1,
            timeouts: HashMap::new(),
        }
    }

    /// A wake after `after`, as an `IoEvent` under the returned token. The
    /// timespec is kept here until the completion.
    pub fn timeout(&mut self, after: Duration) -> Token {
        let ts = Box::new(
            Timespec::new()
                .sec(after.as_secs())
                .nsec(after.subsec_nanos()),
        );
        let sqe = opcode::Timeout::new(&*ts as *const Timespec).build();
        let token = self.push(sqe);
        self.timeouts.insert(token.0, ts);
        token
    }

    /// Queues `sqe` under a fresh token and returns it. If the queue is full
    /// the pending entries are submitted first, so a push never fails.
    ///
    /// The entry may point at memory; that memory must stay where it is
    /// until the completion arrives, which is the submitter's promise.
    pub fn push(&mut self, sqe: squeue::Entry) -> Token {
        let token = Token(self.next_token);
        self.next_token += 1;
        let sqe = sqe.user_data(token.0);
        // SAFETY: every entry this crate submits points only at memory the
        // submitter keeps alive until its completion; that contract is the
        // submitter's and is stated at each call site.
        if unsafe { self.ring.submission().push(&sqe) }.is_err() {
            self.ring.submit().expect("io_uring submit");
            unsafe { self.ring.submission().push(&sqe) }
                .expect("submission queue has room after a submit");
        }
        token
    }

    /// Submits what is queued without waiting. The runner calls it once
    /// after a stop, so the last batch's requests leave before the process
    /// does.
    pub fn submit(&mut self) {
        self.ring.submit().expect("io_uring submit");
    }

    /// Submits what is queued and blocks until at least one completion is
    /// in, then drains them all.
    pub fn wait(&mut self) -> Vec<IoEvent> {
        self.ring
            .submit_and_wait(1)
            .expect("io_uring submit_and_wait");
        let mut cq = self.ring.completion();
        cq.sync();
        let events: Vec<IoEvent> = cq
            .map(|cqe| IoEvent {
                token: Token(cqe.user_data()),
                result: cqe.result(),
            })
            .collect();
        for ev in &events {
            self.timeouts.remove(&ev.token.0);
        }
        events
    }
}

/// Inserts the [`Ring`] and sets the runner.
pub struct RingModule {
    pub entries: u32,
}

impl Default for RingModule {
    fn default() -> Self {
        Self { entries: 256 }
    }
}

impl Module for RingModule {
    fn install(self, app: &mut App) {
        app.insert_resource(Ring::new(self.entries))
            .insert_resource(Stopping::default())
            .system(on_stop)
            .set_runner(run);
    }
}

fn on_stop(app: &mut App, _: &Stop) {
    if let Some(mut s) = app.resource_mut::<Stopping>() {
        s.0 = true;
    }
}

fn stopped(app: &App) -> bool {
    app.resource::<Stopping>().is_some_and(|s| s.0)
}

/// One batch: `Tick` and everything it chains, then `BeforeWait` and
/// everything that chains.
fn batch(app: &mut App) {
    app.signal(Tick);
    app.flush();
    app.signal(BeforeWait);
    app.flush();
}

/// The runner. A batch first, so what install and `main` wrote is flushed
/// and laid out before the ring can block on nothing; then wait, one
/// `IoEvent` per completion, a batch, until stopped; then one submit, so
/// the last batch's requests leave.
///
/// The completions are flushed before the batch begins, so everything they
/// chain, a compositor's reply included, is in place when `Tick` runs.
fn run(mut app: App) {
    batch(&mut app);
    while !stopped(&app) {
        let completions = app
            .resource_mut::<Ring>()
            .expect("no Ring holder is alive while the runner waits")
            .wait();
        for c in completions {
            app.signal(c);
        }
        app.flush();
        batch(&mut app);
    }
    if let Some(mut ring) = app.resource_mut::<Ring>() {
        ring.submit();
    }
}
