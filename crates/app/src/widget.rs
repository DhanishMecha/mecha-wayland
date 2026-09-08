use crate::{Handle, Spawner};

/// Marks a type as storable in an [`App`](crate::App) tree.
///
/// This is a marker: the runtime never calls anything on a widget. Behaviour
/// is attached per node, by the handlers a [`WidgetBuild`] registers through
/// its [`Spawner`].
///
/// ```
/// # use app::Widget;
/// struct Div;
/// impl Widget for Div {}
/// ```
pub trait Widget: 'static {}

/// Describes how to produce a [`Widget`], the subtree under it, and the
/// handlers wired to that subtree.
///
/// [`App::spawn`](crate::App::spawn) allocates the node first — so `me` is
/// final when `spawn` runs — then calls this, then stores the returned
/// widget. Children are attached and handlers registered through `s`; the
/// builder can only name `me` and the handles `s.child` handed back, which
/// keeps a builder a pure description of its own subtree.
///
/// ```
/// # use app::{App, Context, Event, Handle, Spawner, Widget, WidgetBuild};
/// struct Div;
/// impl Widget for Div {}
/// struct Text(String);
/// impl Widget for Text {}
///
/// struct Rename(&'static str);
/// impl Event for Rename {}
///
/// struct DivBuilder;
/// impl WidgetBuild for DivBuilder {
///     type Widget = Div;
///     fn spawn(self, _me: Handle<Div>, _s: &mut Spawner<Div>) -> Div { Div }
/// }
///
/// struct TextBuilder(&'static str);
/// impl WidgetBuild for TextBuilder {
///     type Widget = Text;
///     fn spawn(self, me: Handle<Text>, s: &mut Spawner<Text>) -> Text {
///         s.on(me, |ctx: &mut Context<Text>, e: &Rename| ctx.me().0 = e.0.into());
///         Text(self.0.into())
///     }
/// }
///
/// /// A composite: one `Div` node with a `Text` child.
/// struct LabelBuilder(&'static str);
/// impl WidgetBuild for LabelBuilder {
///     type Widget = Div;
///     fn spawn(self, me: Handle<Div>, s: &mut Spawner<Div>) -> Div {
///         s.child(me, TextBuilder(self.0));
///         Div
///     }
/// }
///
/// let mut app = App::new();
/// let label = app.spawn(app.root(), LabelBuilder("hi")).unwrap();
/// let text = app.children(label).unwrap()[0];
/// app.emit(Rename("bye"), &[text]);
/// app.flush();
/// assert_eq!(app.widget::<Text>(text).unwrap().0, "bye");
/// ```
pub trait WidgetBuild {
    type Widget: Widget;

    fn spawn(self, me: Handle<Self::Widget>, s: &mut Spawner<Self::Widget>) -> Self::Widget;
}
