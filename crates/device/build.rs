//! Builds the FBInk shim and links libfbink.a for the Kobo target only.
//! FBINK_DIR: FBInk source tree with Release/libfbink.a (default ~/src/FBInk),
//! built with `make staticlib MINIMAL=true IMAGE=true KOBO=true CROSS_TC=...`.
use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=csrc/sg_fbink.c");
    println!("cargo:rerun-if-env-changed=FBINK_DIR");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_KOBO");
    // The kobo feature, not target_arch, selects the FBInk backend
    // (see docs/adr/0001-platform-seam.md).
    if env::var("CARGO_FEATURE_KOBO").is_err() {
        return;
    }
    let dir = env::var("FBINK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env::var("HOME").unwrap_or_default()).join("src/FBInk"));
    cc::Build::new()
        .file("csrc/sg_fbink.c")
        .include(&dir)
        .define("FBINK_FOR_KOBO", None)
        .flag_if_supported("-std=gnu11")
        .compile("sg_fbink");
    println!(
        "cargo:rustc-link-search=native={}",
        dir.join("Release").display()
    );
    println!("cargo:rustc-link-lib=static=fbink");
}
