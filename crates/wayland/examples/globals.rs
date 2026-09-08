//! Connect, bind every known global, print them, exit. Run under a
//! compositor; with `WAYLAND_DISPLAY` unset, install panics.

use app::App;
use ring::RingModule;
use wayland::{
    Globals, WaylandModule, WlCompositor, WlSeat, WlShm, XdgWmBase, ZwlrLayerShellV1,
    ZwpLinuxDmabufV1,
};

fn main() {
    let mut app = App::new();
    app.add_module(RingModule::default())
        .add_module(WaylandModule);

    let globals = app.resource::<Globals>().unwrap();
    for (name, interface, version) in globals.advertised() {
        println!("{name:>3}  {interface:<40} v{version}");
    }
    println!();
    println!(
        "wl_compositor        {:?}",
        globals.get::<WlCompositor>().map(|p| p.object_id())
    );
    println!(
        "wl_shm               {:?}",
        globals.get::<WlShm>().map(|p| p.object_id())
    );
    println!(
        "wl_seat              {:?}",
        globals.get::<WlSeat>().map(|p| p.object_id())
    );
    println!(
        "xdg_wm_base          {:?}",
        globals.get::<XdgWmBase>().map(|p| p.object_id())
    );
    println!(
        "zwlr_layer_shell_v1  {:?}",
        globals.get::<ZwlrLayerShellV1>().map(|p| p.object_id())
    );
    println!(
        "zwp_linux_dmabuf_v1  {:?}",
        globals.get::<ZwpLinuxDmabufV1>().map(|p| p.object_id())
    );
    globals.require::<WlCompositor>();
    globals.require::<XdgWmBase>();
}
