use crate::{NodeId, Spawner};

/// Marks a type as storable in an [`App`](crate::App) tree.
///
/// This is a marker: the runtime never calls anything on a widget. Behaviour
/// is attached later, per widget type, by the event machinery.
///
/// ```
/// # use app::Widget;
/// struct Div;
/// impl Widget for Div {}
/// ```
pub trait Widget: 'static {}

/// Describes how to produce a [`Widget`] and, optionally, the subtree under it.
///
/// [`App::spawn`](crate::App::spawn) allocates the node first — so `me` is
/// final when `spawn` runs — then calls this, then stores the returned
/// widget. Children are attached through `s`; the builder can only name `me`
/// and the ids `s.child` handed back, which keeps a builder a pure description
/// of its own subtree.
///
/// ```
/// # use app::{App, NodeId, Spawner, Widget, WidgetBuild};
/// struct Div;
/// impl Widget for Div {}
/// struct Text(String);
/// impl Widget for Text {}
///
/// struct DivBuilder;
/// impl WidgetBuild for DivBuilder {
///     type Widget = Div;
///     fn spawn(self, _me: NodeId, _s: &mut Spawner) -> Div { Div }
/// }
///
/// struct TextBuilder(&'static str);
/// impl WidgetBuild for TextBuilder {
///     type Widget = Text;
///     fn spawn(self, _me: NodeId, _s: &mut Spawner) -> Text { Text(self.0.into()) }
/// }
///
/// /// A composite: one `Div` node with a `Text` child.
/// struct LabelBuilder(&'static str);
/// impl WidgetBuild for LabelBuilder {
///     type Widget = Div;
///     fn spawn(self, me: NodeId, s: &mut Spawner) -> Div {
///         s.child(me, TextBuilder(self.0));
///         Div
///     }
/// }
///
/// let mut app = App::new();
/// let label = app.spawn(app.root(), LabelBuilder("hi")).unwrap();
/// assert_eq!(app.children(label).unwrap().len(), 1);
/// ```
pub trait WidgetBuild {
    type Widget: Widget;

    fn spawn(self, me: NodeId, s: &mut Spawner) -> Self::Widget;
}
