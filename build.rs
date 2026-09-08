//! Packs the examples' atlas: the fonts in `examples/atlas.toml`, rasterised
//! into one PNG and a generated Rust file the examples `include!`.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let atlas = manifest_dir.join("examples").join("atlas.toml");
    if let Err(e) = assets::builder::pack_atlas(&atlas, &out_dir) {
        panic!("packing {}: {e:?}", atlas.display());
    }
}
