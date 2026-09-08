use crate::{App, Event, Handle, NodeId, Signal, Widget};

/// What a handler sees: its own widget, its own handle, and a narrow window
/// onto the rest of the app.
///
/// The widget is moved out of its store into the context for the duration
/// of the handler, so `me()` and the app-backed methods never alias, and is
/// moved back when the context drops — also on unwind, so a panicking
/// handler doesn't leave a hole in the tree. The app is private: a handler
/// can read and write *other* widgets, queue events and signals, and nothing
/// else. Structural changes (spawn, remove) are a system's job; a handler
/// asks for them with [`Context::signal`].
pub struct Context<'a, W: Widget> {
    app: &'a mut App,
    handle: Handle<W>,
    /// `Some` until `Drop` takes it.
    widget: Option<W>,
}

impl<'a, W: Widget> Context<'a, W> {
    /// Take `handle`'s widget out of the store. `None` if the node is stale,
    /// holds another type, or its widget is already out (a handler for the
    /// same node is running further up the stack).
    pub(crate) fn take(app: &'a mut App, handle: Handle<W>) -> Option<Self> {
        let widget = app.take_widget::<W>(handle.id())?;
        Some(Self {
            app,
            handle,
            widget: Some(widget),
        })
    }

    /// The widget the event was emitted at.
    #[inline]
    pub fn me(&mut self) -> &mut W {
        self.widget.as_mut().expect("widget is present until drop")
    }

    /// The node the event was emitted at, for naming it as a target.
    #[inline]
    pub fn handle(&self) -> Handle<W> {
        self.handle
    }

    /// Another node's widget. `None` if the handle is stale, or names this
    /// handler's own node (that widget is out of the store; use `me()`).
    pub fn at<V: Widget>(&mut self, handle: Handle<V>) -> Option<&mut V> {
        self.app.widget_mut::<V>(handle)
    }

    /// Queue `event` for `targets`. See [`App::emit`].
    pub fn emit<E: Event>(&mut self, event: E, targets: &[NodeId]) {
        self.app.emit(event, targets)
    }

    /// Queue `event` for every live node. See [`App::emit_all`].
    pub fn emit_all<E: Event>(&mut self, event: E) {
        self.app.emit_all(event)
    }

    /// Queue `signal` for the systems. See [`App::signal`].
    pub fn signal<S: Signal>(&mut self, signal: S) {
        self.app.signal(signal)
    }
}

impl<W: Widget> Drop for Context<'_, W> {
    fn drop(&mut self) {
        if let Some(widget) = self.widget.take() {
            self.app.put_widget_back::<W>(self.handle.id(), widget);
        }
    }
}
