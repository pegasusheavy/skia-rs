//! Build script for skia-rs-ffi.
//!
//! Generates `skia_rs.h`, the C API header, via `cbindgen`.
//!
//! The header is emitted in two places:
//! - `$OUT_DIR/skia_rs.h`       — always; consumed by the in-tree C example.
//! - `include/skia_rs.h`        — when `SKIA_RS_FFI_EMIT_HEADER=1` is set in
//!   the environment. This lets CI or a release build refresh the committed
//!   header without forcing every `cargo build` to scribble on the repo.
//!
//! Set `SKIA_RS_FFI_SKIP_CBINDGEN=1` to skip generation entirely (useful for
//! building on systems where the cbindgen crate cannot build, or for
//! `cargo publish` which runs in an isolated environment).

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/abi.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-env-changed=SKIA_RS_FFI_SKIP_CBINDGEN");
    println!("cargo:rerun-if-env-changed=SKIA_RS_FFI_EMIT_HEADER");

    if env::var_os("SKIA_RS_FFI_SKIP_CBINDGEN").is_some() {
        return;
    }

    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml"))
        .unwrap_or_else(|_| cbindgen::Config::default());

    let bindings = match cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
    {
        Ok(b) => b,
        Err(e) => {
            // Non-fatal — header generation is a nice-to-have, not a
            // correctness requirement for the cdylib itself. Emit a warning
            // so CI can surface it without breaking dependent crates that
            // build inside `cargo publish` sandboxes.
            println!("cargo:warning=cbindgen failed: {e}");
            return;
        }
    };

    bindings.write_to_file(out_dir.join("skia_rs.h"));

    if env::var_os("SKIA_RS_FFI_EMIT_HEADER").is_some() {
        let include_dir = crate_dir.join("include");
        std::fs::create_dir_all(&include_dir).ok();
        bindings.write_to_file(include_dir.join("skia_rs.h"));
    }
}
