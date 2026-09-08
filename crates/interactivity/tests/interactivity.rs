//! Pointer input at a window, the pointer it keeps, and the `Clicked` a
//! press becomes: at what, in what order, and when not at all.

use app::prelude::*;
use interactivity::prelude::*;
use layout::prelude::*;
use paint::prelude::*;
use utils::Point;
use widgets::prelude::*;
use window::prelude::*;

// ── fixtures ─────────────────────────────────────────────────────────────────

/// Every `Clicked` any button or window received, in the order received.
#[derive(Default)]
struct Log(Vec<(NodeId, Clicked)>);
impl Resource for Log {}

fn log<W: Widget>(ctx: &mut Context<W>, c: &Clicked) {
    let id = ctx.handle().id();
    ctx.resource_mut::<Log>().unwrap().0.push((id, *c));
}

/// A box that wants to be clicked: it keeps what it got, and logs it.
#[derive(Default)]
struct Button {
    clicks: Vec<Clicked>,
}
impl Widget for Button {}

struct ButtonBuilder(LayoutStyle);

fn button(style: LayoutStyle) -> ButtonBuilder {
    ButtonBuilder(style)
}

impl WidgetBuild for ButtonBuilder {
    type Widget = Button;
    fn spawn(self, me: Handle<Button>, s: &mut Spawner<Button>) -> Button {
        s.set_component(self.0);
        s.on(me, |ctx: &mut Context<Button>, c: &Clicked| {
            ctx.me().clicks.push(*c);
            log(ctx, c);
        });
        Button::default()
    }
}

/// A window that logs its clicks too.
struct Owner;
impl Widget for Owner {}

struct Build(WindowBuilder);
impl WidgetBuild for Build {
    type Widget = Owner;
    fn spawn(self, me: Handle<Owner>, s: &mut Spawner<Owner>) -> Owner {
        let win = s.child(me, self.0);
        s.on(win, log::<Window>);
        Owner
    }
}

fn a_window() -> WindowBuilder {
    window()
        .layout(LayoutStyle::default().size(px(480.0), px(240.0)))
        .background(Quad::new(Color::BLACK))
}

fn app() -> App {
    let mut app = App::new();
    app.add_module(LayoutModule)
        .add_module(PaintModule)
        .add_module(WindowModule)
        .add_module(InteractivityModule)
        .insert_resource(Log::default());
    app
}

fn tick(app: &mut App) {
    app.signal(Tick);
    app.flush();
}

/// A window, laid out, with its node in hand.
fn spawn_window(app: &mut App, w: WindowBuilder) -> NodeId {
    let owner = app.spawn(app.root(), Build(w)).unwrap();
    let win = app.children(owner).unwrap()[0];
    tick(app);
    win
}

fn input(app: &mut App, window: NodeId, input: Input) {
    app.signal(PointerInput { window, input });
    app.flush();
}

fn enter(app: &mut App, window: NodeId, x: f32, y: f32) {
    input(app, window, Input::Enter(Point::new(x, y)));
}

fn press(app: &mut App, window: NodeId, button: MouseButton) {
    input(app, window, Input::Press(button));
}

fn clicked(app: &mut App) -> Vec<(NodeId, Clicked)> {
    std::mem::take(&mut app.resource_mut::<Log>().unwrap().0)
}

fn clicked_at(app: &mut App) -> Vec<NodeId> {
    clicked(app).into_iter().map(|(id, _)| id).collect()
}

fn pointer(app: &App, window: NodeId) -> Pointer {
    app.components::<Pointer>().unwrap()[window].clone()
}

fn square() -> LayoutStyle {
    LayoutStyle::default().size(px(48.0), px(48.0))
}

// ── clicking ─────────────────────────────────────────────────────────────────

#[test]
fn a_press_over_a_node_is_a_click_at_it_and_its_window() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    let b = app.spawn(win, button(square())).unwrap();
    tick(&mut app);

    enter(&mut app, win, 10.0, 10.0);
    press(&mut app, win, MouseButton::Left);
    let c = Clicked {
        button: MouseButton::Left,
        position: Point::new(10.0, 10.0),
    };
    assert_eq!(clicked(&mut app), [(b.id(), c), (win, c)]);
    assert_eq!(app.widget::<Button>(b).unwrap().clicks, [c]);
}

#[test]
fn a_press_beside_a_node_misses_it() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    let b = app.spawn(win, button(square())).unwrap();
    tick(&mut app);

    enter(&mut app, win, 100.0, 10.0);
    press(&mut app, win, MouseButton::Left);
    assert_eq!(clicked_at(&mut app), [win], "only the window is under it");
    assert!(app.widget::<Button>(b).unwrap().clicks.is_empty());
}

#[test]
fn a_press_outside_the_window_is_nothing() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    enter(&mut app, win, 500.0, 10.0);
    press(&mut app, win, MouseButton::Left);
    assert!(clicked(&mut app).is_empty());
}

#[test]
fn a_click_goes_deepest_first_and_the_window_last() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    let outer = app
        .spawn(win, button(LayoutStyle::default().size(px(96.0), px(96.0))))
        .unwrap();
    let inner = app.spawn(outer, button(square())).unwrap();
    let label = app.spawn(inner, text("x").font(&FONT)).unwrap();
    tick(&mut app);

    enter(&mut app, win, 2.0, 2.0);
    press(&mut app, win, MouseButton::Left);
    assert_eq!(
        clicked_at(&mut app),
        [inner.id(), outer.id(), win],
        "the label has no handler and is passed over"
    );
    let _ = label;
}

#[test]
fn a_later_sibling_over_an_earlier_one_is_clicked_first() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    let under = app.spawn(win, button(square().absolute())).unwrap();
    let over = app.spawn(win, button(square().absolute())).unwrap();
    tick(&mut app);

    enter(&mut app, win, 10.0, 10.0);
    press(&mut app, win, MouseButton::Left);
    assert_eq!(clicked_at(&mut app), [over.id(), under.id(), win]);
}

#[test]
fn an_absolute_child_outside_its_parent_is_still_hit() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    let parent = app.spawn(win, button(square())).unwrap();
    let child = app
        .spawn(
            parent,
            button(
                square()
                    .absolute()
                    .inset(edges(px(100.0), auto(), auto(), px(100.0))),
            ),
        )
        .unwrap();
    tick(&mut app);
    assert_eq!(
        app.components::<Layout>().unwrap()[child].rect,
        utils::Rect::xywh(100.0, 100.0, 48.0, 48.0)
    );

    enter(&mut app, win, 110.0, 110.0);
    press(&mut app, win, MouseButton::Left);
    assert_eq!(
        clicked_at(&mut app),
        [child.id(), win],
        "the parent does not contain the point; the child does"
    );
}

#[test]
fn a_hidden_node_is_never_hit() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    let b = app.spawn(win, button(square().hidden())).unwrap();
    tick(&mut app);

    enter(&mut app, win, 0.0, 0.0);
    press(&mut app, win, MouseButton::Left);
    assert_eq!(clicked_at(&mut app), [win]);
    assert!(app.widget::<Button>(b).unwrap().clicks.is_empty());
}

#[test]
fn a_window_with_an_inset_is_hit_in_its_own_coordinates() {
    let mut app = app();
    let win = spawn_window(
        &mut app,
        a_window().layout(
            LayoutStyle::default()
                .size(px(480.0), px(240.0))
                .inset(edges(px(50.0), auto(), auto(), px(100.0))),
        ),
    );
    let b = app.spawn(win, button(square())).unwrap();
    tick(&mut app);
    assert_eq!(
        app.components::<Layout>().unwrap()[b].rect,
        utils::Rect::xywh(100.0, 50.0, 48.0, 48.0),
        "laid out where the window is"
    );

    enter(&mut app, win, 10.0, 10.0);
    press(&mut app, win, MouseButton::Left);
    assert_eq!(clicked_at(&mut app), [b.id(), win]);
}

#[test]
fn the_pointer_moves_between_presses() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    let b = app.spawn(win, button(square())).unwrap();
    tick(&mut app);

    enter(&mut app, win, 100.0, 100.0);
    press(&mut app, win, MouseButton::Left);
    assert_eq!(clicked_at(&mut app), [win]);
    input(&mut app, win, Input::Release(MouseButton::Left));
    input(&mut app, win, Input::Move(Point::new(10.0, 10.0)));
    press(&mut app, win, MouseButton::Right);
    assert_eq!(
        clicked(&mut app),
        [
            (
                b.id(),
                Clicked {
                    button: MouseButton::Right,
                    position: Point::new(10.0, 10.0),
                }
            ),
            (
                win,
                Clicked {
                    button: MouseButton::Right,
                    position: Point::new(10.0, 10.0),
                }
            ),
        ]
    );
}

#[test]
fn a_press_before_entering_or_after_leaving_is_nothing() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    press(&mut app, win, MouseButton::Left);
    assert!(clicked(&mut app).is_empty(), "never entered");

    enter(&mut app, win, 10.0, 10.0);
    input(&mut app, win, Input::Leave);
    press(&mut app, win, MouseButton::Left);
    assert!(clicked(&mut app).is_empty(), "left");
}

#[test]
fn a_release_is_not_a_click() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    enter(&mut app, win, 10.0, 10.0);
    input(&mut app, win, Input::Release(MouseButton::Left));
    assert!(clicked(&mut app).is_empty());
}

#[test]
fn input_at_a_stale_window_is_ignored() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    app.remove(win).unwrap();
    tick(&mut app);
    enter(&mut app, win, 10.0, 10.0);
    press(&mut app, win, MouseButton::Left);
    assert!(clicked(&mut app).is_empty());
}

#[test]
fn each_window_has_its_own_pointer() {
    let mut app = app();
    let a = spawn_window(&mut app, a_window());
    let b = spawn_window(&mut app, a_window());
    let in_a = app.spawn(a, button(square())).unwrap();
    let in_b = app.spawn(b, button(square())).unwrap();
    tick(&mut app);

    enter(&mut app, a, 10.0, 10.0);
    press(&mut app, b, MouseButton::Left);
    assert!(clicked(&mut app).is_empty(), "b's pointer never entered");
    enter(&mut app, b, 10.0, 10.0);
    press(&mut app, b, MouseButton::Left);
    assert_eq!(clicked_at(&mut app), [in_b.id(), b]);
    assert!(app.widget::<Button>(in_a).unwrap().clicks.is_empty());
}

// ── the pointer ──────────────────────────────────────────────────────────────

#[test]
fn the_pointer_is_kept_at_the_window() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    assert_eq!(pointer(&app, win), Pointer::default(), "not there yet");
    assert_eq!(pointer(&app, win).position(), None);

    enter(&mut app, win, 10.0, 20.0);
    assert_eq!(pointer(&app, win).position(), Some(Point::new(10.0, 20.0)));
    input(&mut app, win, Input::Move(Point::new(30.0, 40.0)));
    assert_eq!(pointer(&app, win).position(), Some(Point::new(30.0, 40.0)));

    press(&mut app, win, MouseButton::Left);
    input(&mut app, win, Input::Move(Point::new(50.0, 60.0)));
    let p = pointer(&app, win);
    assert!(p.is_pressed(MouseButton::Left));
    assert!(!p.is_pressed(MouseButton::Right));
    assert_eq!(
        p.pressed_at(MouseButton::Left),
        Some(Point::new(30.0, 40.0)),
        "where it went down, not where it is"
    );
    assert_eq!(
        p.pressed().collect::<Vec<_>>(),
        [(MouseButton::Left, Point::new(30.0, 40.0))]
    );

    input(&mut app, win, Input::Release(MouseButton::Left));
    assert!(!pointer(&app, win).is_pressed(MouseButton::Left));
    assert_eq!(
        pointer(&app, win).position(),
        Some(Point::new(50.0, 60.0)),
        "a release moves nothing"
    );
}

#[test]
fn leaving_forgets_the_position_and_what_was_held() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    enter(&mut app, win, 10.0, 20.0);
    press(&mut app, win, MouseButton::Left);
    input(&mut app, win, Input::Leave);
    assert_eq!(pointer(&app, win), Pointer::default());
}

#[test]
fn the_root_and_ordinary_nodes_carry_an_absent_pointer() {
    let mut app = app();
    let win = spawn_window(&mut app, a_window());
    let b = app.spawn(win, button(square())).unwrap();
    enter(&mut app, win, 10.0, 20.0);
    assert_eq!(pointer(&app, app.root()), Pointer::default());
    assert_eq!(pointer(&app, b.id()), Pointer::default());
}

#[test]
fn linux_button_codes_read_by_name() {
    assert_eq!(MouseButton::from(0x110), MouseButton::Left);
    assert_eq!(MouseButton::from(0x111), MouseButton::Right);
    assert_eq!(MouseButton::from(0x112), MouseButton::Middle);
    assert_eq!(MouseButton::from(0x113), MouseButton::Side);
    assert_eq!(MouseButton::from(0x114), MouseButton::Extra);
    assert_eq!(MouseButton::from(0x115), MouseButton::Forward);
    assert_eq!(MouseButton::from(0x116), MouseButton::Back);
    assert_eq!(MouseButton::from(0x117), MouseButton::Task);
    assert_eq!(MouseButton::from(0x103), MouseButton::Numbered(3));
    assert_eq!(MouseButton::from(0x2c0), MouseButton::ExtraButton(1));
    assert_eq!(MouseButton::from(0x2e7), MouseButton::ExtraButton(40));
    assert_eq!(MouseButton::from(0x999), MouseButton::Unknown(0x999));
}

// ── a font, for the label ────────────────────────────────────────────────────

use assets::{AtlasId, BakedFont, GlyphInfo};

const fn glyphs() -> [GlyphInfo; 95] {
    let g = GlyphInfo {
        x: 0.0,
        y: 0.0,
        w: 6.0,
        h: 10.0,
        bearing_x: 1.0,
        bearing_y: -2.0,
        advance: 8.0,
    };
    [g; 95]
}

static FONT: BakedFont = BakedFont {
    atlas_id: AtlasId(1),
    size: 12.0,
    line_height: 16.0,
    ascent: 12.0,
    glyphs: glyphs(),
};
