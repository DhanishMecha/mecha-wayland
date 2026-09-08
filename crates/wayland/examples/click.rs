//! Drive the compositor's pointer: warp it to a point and, optionally,
//! click. For exercising a window under test from a script.
//!
//! ```text
//! click X Y [X_EXTENT Y_EXTENT] [--press]
//! ```
//!
//! `X`, `Y` are absolute over an `X_EXTENT` by `Y_EXTENT` frame the
//! compositor maps onto its layout; the extents default to the point
//! itself doubled, which is only right for calibration. With `--press`, a
//! left button press and release follow the motion.

use std::time::Duration;

use app::App;
use ring::{Ring, RingModule};
use wayland::{Globals, WaylandModule, WlPointerButtonState, WlSeat, ZwlrVirtualPointerManagerV1};

const BTN_LEFT: u32 = 0x110;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let press = args.iter().any(|a| a == "--press");
    let nums: Vec<u32> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(|a| a.parse().expect("a number"))
        .collect();
    let [x, y, rest @ ..] = nums.as_slice() else {
        eprintln!("usage: click X Y [X_EXTENT Y_EXTENT] [--press]");
        std::process::exit(2);
    };
    let (x_extent, y_extent) = match rest {
        [w, h, ..] => (*w, *h),
        _ => (x * 2, y * 2),
    };

    let mut app = App::new();
    app.add_module(RingModule::default())
        .add_module(WaylandModule);

    {
        let globals = app.resource::<Globals>().unwrap();
        let manager = globals.require::<ZwlrVirtualPointerManagerV1>();
        let pointer = manager.create_virtual_pointer(globals.get::<WlSeat>());
        pointer.motion_absolute(0, *x, *y, x_extent, y_extent);
        pointer.frame();
        if press {
            pointer.button(1, BTN_LEFT, WlPointerButtonState::Pressed);
            pointer.frame();
            pointer.button(2, BTN_LEFT, WlPointerButtonState::Released);
            pointer.frame();
        }
        pointer.destroy();
    }
    // One round trip so the requests leave before the process does.
    app.resource_mut::<Ring>()
        .unwrap()
        .timeout(Duration::from_millis(50));
    app.system(|app: &mut App, _: &ring::IoEvent| app.signal(ring::Stop));
    app.run();
}
