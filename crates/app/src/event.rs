//! The two kinds of message in the runtime, and the built-in ones.
//!
//! - A [`Signal`] is app-level: it goes to every *system* registered for its
//!   type, through [`App::signal`](crate::App::signal).
//! - An [`Event`] is node-level: it goes to the *handlers* of the nodes it is
//!   emitted at, through [`App::emit`](crate::App::emit) and
//!   [`App::emit_all`](crate::App::emit_all).
//!
//! Both are markers. Any `'static` type can be one, or both.

/// An app-level message consumed by systems.
///
/// ```
/// # use app::Signal;
/// struct Resized { width: u32, height: u32 }
/// impl Signal for Resized {}
/// ```
pub trait Signal: 'static {}

/// A node-level message consumed by handlers.
///
/// ```
/// # use app::Event;
/// struct Click;
/// impl Event for Click {}
/// ```
pub trait Event: 'static {}

/// The built-in signal the default runner sends once per loop iteration.
/// See [`App::run`](crate::App::run).
pub struct Tick;
impl Signal for Tick {}
