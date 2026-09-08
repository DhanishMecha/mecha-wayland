//! Units of functionality that extend the runtime by installing into the
//! [`App`](crate::App).

use crate::App;

/// A unit that extends the runtime by installing into the [`App`]: it
/// registers components, inserts resources, attaches systems, and may set
/// the runner. Nothing else.
///
/// The app knows no component, resource or system of its own; layout,
/// paint, interactivity and presentation are each a module written against
/// `App`'s surface. Wiring behaviour to a node is a builder's job, not a
/// module's — a module supplies what every node carries and what every
/// system reads, and a builder configures it per node.
///
/// [`App::add_module`] calls `install` right away, so a module sees
/// everything the modules added before it put in place, and can build on
/// it: a module that needs another's resource is a module that must be
/// added after it. There is no dependency graph and no dedupe — adding a
/// module twice installs it twice, and the first duplicate `register`,
/// `insert` or `set_runner` panics.
///
/// A module is consumed by `install`: configuration travels in its fields
/// and is gone once installed.
///
/// ```
/// use app::prelude::*;
///
/// #[derive(Default)]
/// struct Layout { x: f32, y: f32 }
/// impl Component for Layout {}
///
/// struct Viewport { width: u32, height: u32 }
/// impl Resource for Viewport {}
///
/// fn relayout(app: &mut App, _: &Tick) {
///     let viewport = app.resource::<Viewport>().unwrap();
///     let mut layouts = app.components_mut::<Layout>().unwrap();
///     layouts[app.root()].x = viewport.width as f32 / 2.0;
///     layouts[app.root()].y = viewport.height as f32 / 2.0;
/// }
///
/// struct LayoutModule { width: u32, height: u32 }
/// impl Module for LayoutModule {
///     fn install(self, app: &mut App) {
///         app.register_component::<Layout>()
///             .insert_resource(Viewport { width: self.width, height: self.height })
///             .system(relayout);
///     }
/// }
///
/// let mut app = App::new();
/// app.add_module(LayoutModule { width: 800, height: 600 });
/// app.signal(Tick);
/// app.flush();
/// let layouts = app.components::<Layout>().unwrap();
/// assert_eq!((layouts[app.root()].x, layouts[app.root()].y), (400.0, 300.0));
/// ```
pub trait Module {
    /// Install into `app`. Called once, by [`App::add_module`], at the
    /// moment the module is added.
    fn install(self, app: &mut App);
}
