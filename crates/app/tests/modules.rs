//! Modules: `install` against the app, in order, and the runner they set.

use app::prelude::*;

// ── fixtures ─────────────────────────────────────────────────────────────────

#[derive(Debug, Default, PartialEq)]
struct Layout {
    x: f32,
    y: f32,
}
impl Component for Layout {}

struct Viewport {
    width: u32,
    height: u32,
}
impl Resource for Viewport {}

/// What was installed, in order. Inserted by whichever module comes first.
#[derive(Default)]
struct Log(Vec<&'static str>);
impl Resource for Log {}

struct Ticks(u32);
impl Resource for Ticks {}

struct Div;
impl Widget for Div {}

struct Sync;
impl Signal for Sync {}

/// Places its node at the module's viewport centre.
struct CenteredDiv;
impl WidgetBuild for CenteredDiv {
    type Widget = Div;
    fn spawn(self, _me: Handle<Div>, s: &mut Spawner<Div>) -> Div {
        let (w, h) = s
            .resource::<Viewport>()
            .map(|v| (v.width, v.height))
            .expect("LayoutModule is installed");
        s.set_component(Layout {
            x: w as f32 / 2.0,
            y: h as f32 / 2.0,
        });
        Div
    }
}

fn log(app: &mut App, entry: &'static str) {
    match app.resource_mut::<Log>() {
        Some(mut log) => log.0.push(entry),
        None => {
            app.insert_resource(Log(vec![entry]));
        }
    }
}

/// A component, a resource configured from the module's fields, and a
/// system that reads both.
struct LayoutModule {
    width: u32,
    height: u32,
}
impl Module for LayoutModule {
    fn install(self, app: &mut App) {
        fn on_sync(app: &mut App, _: &Sync) {
            let viewport = app.resource::<Viewport>().unwrap();
            let mut layouts = app.components_mut::<Layout>().unwrap();
            layouts[app.root()] = Layout {
                x: viewport.width as f32,
                y: viewport.height as f32,
            };
        }
        log(app, "layout");
        app.register_component::<Layout>()
            .insert_resource(Viewport {
                width: self.width,
                height: self.height,
            })
            .system(on_sync);
    }
}

/// Depends on `LayoutModule`: reads its resource at install time.
struct HalfViewportModule;
impl Module for HalfViewportModule {
    fn install(self, app: &mut App) {
        log(app, "half");
        let mut viewport = app
            .resource_mut::<Viewport>()
            .expect("added after LayoutModule");
        viewport.width /= 2;
        viewport.height /= 2;
    }
}

/// Owns the loop: one `Tick`, one flush, done.
struct OneTickRunner;
impl Module for OneTickRunner {
    fn install(self, app: &mut App) {
        fn one_tick(mut app: App) {
            app.signal(Tick);
            app.flush();
        }
        fn count(app: &mut App, _: &Tick) {
            app.resource_mut::<Ticks>().unwrap().0 += 1;
        }
        log(app, "runner");
        app.insert_resource(Ticks(0))
            .system(count)
            .set_runner(one_tick);
    }
}

// ── install ──────────────────────────────────────────────────────────────────

#[test]
fn install_registers_a_component_a_resource_and_a_system() {
    let mut app = App::new();
    app.add_module(LayoutModule {
        width: 800,
        height: 600,
    });

    assert!(app.components::<Layout>().is_some(), "component registered");
    let viewport = app.resource::<Viewport>().expect("resource inserted");
    assert_eq!((viewport.width, viewport.height), (800, 600));
    drop(viewport);

    app.signal(Sync);
    app.flush();
    let layouts = app.components::<Layout>().unwrap();
    assert_eq!(
        layouts[app.root()],
        Layout { x: 800.0, y: 600.0 },
        "system ran"
    );
}

#[test]
fn a_module_is_configured_by_its_fields() {
    let mut app = App::new();
    app.add_module(LayoutModule {
        width: 1,
        height: 2,
    });
    let viewport = app.resource::<Viewport>().unwrap();
    assert_eq!((viewport.width, viewport.height), (1, 2));
}

#[test]
fn add_module_chains_and_installs_in_call_order() {
    let mut app = App::new();
    app.add_module(LayoutModule {
        width: 800,
        height: 600,
    })
    .add_module(HalfViewportModule)
    .add_module(OneTickRunner);
    assert_eq!(
        app.resource::<Log>().unwrap().0,
        ["layout", "half", "runner"]
    );
}

#[test]
fn a_later_module_builds_on_an_earlier_one() {
    let mut app = App::new();
    app.add_module(LayoutModule {
        width: 800,
        height: 600,
    })
    .add_module(HalfViewportModule);
    let viewport = app.resource::<Viewport>().unwrap();
    assert_eq!((viewport.width, viewport.height), (400, 300));
}

#[test]
#[should_panic(expected = "added after LayoutModule")]
fn a_module_added_before_its_dependency_fails_at_install() {
    let mut app = App::new();
    app.add_module(HalfViewportModule);
}

#[test]
#[should_panic(expected = "already registered")]
fn adding_a_module_twice_is_not_deduped() {
    let mut app = App::new();
    app.add_module(LayoutModule {
        width: 800,
        height: 600,
    });
    app.add_module(LayoutModule {
        width: 800,
        height: 600,
    });
}

// ── with the tree ────────────────────────────────────────────────────────────

#[test]
fn builders_use_what_a_module_installed() {
    let mut app = App::new();
    app.add_module(LayoutModule {
        width: 800,
        height: 600,
    });
    let div = app.spawn(app.root(), CenteredDiv).unwrap();
    let layouts = app.components::<Layout>().unwrap();
    assert_eq!(layouts[div], Layout { x: 400.0, y: 300.0 });
}

#[test]
fn nodes_spawned_before_a_module_get_its_component_default() {
    struct Plain;
    impl WidgetBuild for Plain {
        type Widget = Div;
        fn spawn(self, _: Handle<Div>, _: &mut Spawner<Div>) -> Div {
            Div
        }
    }
    let mut app = App::new();
    let early = app.spawn(app.root(), Plain).unwrap();
    app.add_module(LayoutModule {
        width: 800,
        height: 600,
    });
    let late = app.spawn(app.root(), CenteredDiv).unwrap();
    let layouts = app.components::<Layout>().unwrap();
    assert_eq!(layouts[early], Layout::default());
    assert_eq!(layouts[late], Layout { x: 400.0, y: 300.0 });
}

// ── runner ───────────────────────────────────────────────────────────────────

#[test]
fn a_module_sets_the_runner_and_run_returns_when_it_does() {
    thread_local! {
        static REPORT: std::cell::Cell<u32> = const { std::cell::Cell::new(u32::MAX) };
    }
    /// One tick, then report what the module's system counted.
    fn report(mut app: App) {
        app.signal(Tick);
        app.flush();
        REPORT.with(|r| r.set(app.resource::<Ticks>().unwrap().0));
    }
    struct Reporting;
    impl Module for Reporting {
        fn install(self, app: &mut App) {
            fn count(app: &mut App, _: &Tick) {
                app.resource_mut::<Ticks>().unwrap().0 += 1;
            }
            app.insert_resource(Ticks(0))
                .system(count)
                .set_runner(report);
        }
    }
    let mut app = App::new();
    app.add_module(Reporting);
    app.run();
    assert_eq!(REPORT.with(|r| r.get()), 1, "runner ran once and returned");
}

#[test]
#[should_panic(expected = "runner already set")]
fn two_modules_setting_the_runner_is_a_configuration_error() {
    /// Sets the runner and nothing else, so the second install reaches it.
    struct LoopOwner;
    impl Module for LoopOwner {
        fn install(self, app: &mut App) {
            app.set_runner(|_| {});
        }
    }
    let mut app = App::new();
    app.add_module(LoopOwner);
    app.add_module(LoopOwner);
}
