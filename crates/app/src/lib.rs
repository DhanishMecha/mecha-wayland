//! The UI runtime: a tree of nodes, each backed by a widget, driven by
//! signals and events.
//!
//! # Model
//!
//! - [`App`] owns the tree. It is never empty: the root exists from
//!   [`App::new`] and is the one node with no widget.
//! - A [`NodeId`] names a node. A [`Handle`] is a `NodeId` that also knows
//!   the widget type at that node; both address the node slot and the widget
//!   slot, plus a generation so a removed node's id never resolves to
//!   whatever reuses its slot.
//! - Widgets are stored *by type*, one column per type. A [`Widget`] is a
//!   marker; the runtime never calls into one.
//! - A [`WidgetBuild`] describes a widget, the subtree under it, and the
//!   handlers wired to that subtree. It runs with a [`Spawner`] that can only
//!   attach children, register handlers, and reach resources.
//! - A [`Resource`] is an app-wide singleton, one per type, kept beside the
//!   tree. See [Resources](#resources).
//!
//! # Messages
//!
//! Two kinds, two consumers:
//!
//! | message    | consumer  | scope      | queued by                            |
//! |------------|-----------|------------|--------------------------------------|
//! | [`Signal`] | *systems* | the app    | [`App::signal`], [`Context::signal`] |
//! | [`Event`]  | *handlers*| named nodes| [`App::emit`], [`App::emit_all`], and the same on [`Context`] |
//!
//! A **system** is a plain `fn(&mut App, &S)` registered with
//! [`App::system`]; every system for `S` runs, in registration order, for
//! every `S` signalled. A **handler** is a plain `fn(&mut Context<W>, &E)`
//! registered on one node by [`Spawner::on`]; it runs, with that node's
//! widget in hand, when an `E` is emitted at that node. Neither can capture.
//!
//! Nothing runs inline. `signal` and `emit` push onto two queues, and
//! [`App::flush`] drains them: events first, then one signal, then events
//! again, until both are empty. [`App::run`] hands the app to a runner
//! ([`App::set_runner`]); the default one loops `signal(Tick)`, `flush()`.
//!
//! # Resources
//!
//! [`App::insert_resource`] stores one value per type; [`App::resource`]
//! hands out any number of shared [`Res`] readers, and [`App::resource_mut`]
//! one exclusive [`ResMut`] writer. Neither proxy borrows the app, so a
//! system can hold one and still spawn, emit, or look up widgets.
//! [`Context`] and [`Spawner`] offer the same two lookups.
//!
//! The rule is the one widgets already follow: a value that is lent out is
//! absent. `resource_mut` returns `None` while any `Res` of that type is
//! alive; `resource` and `resource_mut` return `None` while a `ResMut` is
//! out; [`App::remove_resource`] returns `None` and changes nothing in
//! either case. Drop the proxy and ask again. Only `insert_resource` panics,
//! and only on a duplicate.
//!
//! # Quick start
//!
//! ```
//! use app::prelude::*;
//!
//! struct Counter(u32);
//! impl Widget for Counter {}
//!
//! struct Inc;
//! impl Event for Inc {}
//!
//! struct Bump;
//! impl Signal for Bump {}
//!
//! struct CounterBuilder;
//! impl WidgetBuild for CounterBuilder {
//!     type Widget = Counter;
//!     fn spawn(self, me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
//!         s.on(me, |ctx: &mut Context<Counter>, _: &Inc| ctx.me().0 += 1);
//!         Counter(0)
//!     }
//! }
//!
//! fn on_bump(app: &mut App, _: &Bump) {
//!     let all: Vec<_> = app.widgets::<Counter>().map(|(id, _)| id).collect();
//!     app.emit(Inc, &all);
//! }
//!
//! let mut app = App::new();
//! app.system(on_bump);
//! let counter = app.spawn(app.root(), CounterBuilder).unwrap();
//!
//! app.signal(Bump);
//! app.flush();
//! assert_eq!(app.widget::<Counter>(counter).unwrap().0, 1);
//! ```

mod app;
mod context;
mod event;
mod id;
mod node;
mod resource;
mod widget;
mod widget_store;

pub use app::{App, Error, Spawner, System};
pub use context::Context;
pub use event::{Event, Signal, Tick};
pub use id::{Handle, NodeId};
pub use resource::{Res, ResMut, Resource};
pub use widget::{Widget, WidgetBuild};

pub mod prelude {
    pub use crate::{
        App, Context, Event, Handle, NodeId, Res, ResMut, Resource, Signal, Spawner, Tick, Widget,
        WidgetBuild,
    };
}
