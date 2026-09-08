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
//! - A [`Component`] is per-node data: every node carries one value of
//!   every registered component type. See [Components](#components).
//! - A [`Module`] installs a set of the above into the app in one go. See
//!   [Modules](#modules).
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
//! A runner is set at most once; a second `set_runner` panics.
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
//! # Components
//!
//! [`App::register_component`] gives every node, present and future, a
//! `C::default()`. A component type is one *column*, a `Vec` mirroring the
//! node arena, and columns are lent out whole with the resource rule:
//! [`App::components`] hands out any number of shared [`Comps`] readers,
//! [`App::components_mut`] one exclusive [`CompsMut`] writer, both indexed
//! by `NodeId`. A handler or builder reaches only its own node's value,
//! through [`Context::component`] / [`Spawner::component`] ([`Comp`]) and
//! the `_mut` pair ([`CompMut`]); those hold the column too, so they count
//! as its reader or writer.
//!
//! The tree may change while a column is lent out. The holder does not see
//! nodes spawned or removed meanwhile; the change is applied when the
//! column comes back. Lookups return `None` while lent out. Only two
//! things panic: registering a type twice, and touching a type that was
//! never registered.
//!
//! # Modules
//!
//! The app knows no component, resource, system or runner of its own. Each
//! area of functionality — layout, paint, interactivity, presentation — is
//! a [`Module`]: a value whose `install(self, &mut App)` registers its
//! components, inserts its resources, attaches its systems and, for the
//! one that owns the loop, sets the runner. [`App::add_module`] installs
//! right away, in call order, so a module builds on the modules before it.
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
mod component;
mod context;
mod event;
mod id;
mod module;
mod node;
mod resource;
mod widget;
mod widget_store;

pub use app::{App, Error, Spawner, System};
pub use component::{Comp, CompMut, Component, Comps, CompsMut};
pub use context::Context;
pub use event::{Event, Signal, Tick};
pub use id::{Handle, NodeId};
pub use module::Module;
pub use resource::{Res, ResMut, Resource};
pub use widget::{Widget, WidgetBuild};

pub mod prelude {
    pub use crate::{
        App, Comp, CompMut, Component, Comps, CompsMut, Context, Event, Handle, Module, NodeId,
        Res, ResMut, Resource, Signal, Spawner, Tick, Widget, WidgetBuild,
    };
}
