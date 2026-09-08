//! The UI runtime: a tree of nodes, each backed by a widget.
//!
//! # Model
//!
//! - [`App`] owns the tree. It is never empty: the root exists from
//!   [`App::new`] and is the one node with no widget.
//! - A [`NodeId`] names a node. It addresses both the node slot and the
//!   widget slot, plus a generation so a removed node's id never resolves to
//!   whatever reuses its slot.
//! - Widgets are stored *by type*, one column per type, so later machinery
//!   can run one handler over every widget of a kind at once. A
//!   [`Widget`] is a marker; the runtime never calls into one.
//! - A [`WidgetBuild`] describes a widget and the subtree under it. It runs
//!   with a [`Spawner`] that can only attach children.
//!
//! # Quick start
//!
//! ```
//! use app::prelude::*;
//!
//! struct Div;
//! impl Widget for Div {}
//!
//! struct DivBuilder;
//! impl WidgetBuild for DivBuilder {
//!     type Widget = Div;
//!     fn spawn(self, _me: NodeId, _s: &mut Spawner) -> Div { Div }
//! }
//!
//! let mut app = App::new();
//! let div = app.spawn(app.root(), DivBuilder).unwrap();
//!
//! assert_eq!(app.parent(div), Some(app.root()));
//! assert!(app.widget::<Div>(div).is_some());
//!
//! app.remove(div).unwrap();
//! assert!(app.widget::<Div>(div).is_none());
//! ```

mod app;
mod id;
mod node;
mod widget;
mod widget_store;

pub use app::{App, Error, Spawner};
pub use id::NodeId;
pub use widget::{Widget, WidgetBuild};

pub mod prelude {
    pub use crate::{App, NodeId, Spawner, Widget, WidgetBuild};
}
