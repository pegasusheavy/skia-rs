//! Regression test: the committed C headers must always match what
//! `cbindgen` produces from the crate's current source.
//!
//! `include/skia-rs.h` (repo root) previously drifted so far from the
//! crate's actual exports that it referenced types that don't exist in the
//! current API (e.g. exposing `Paint`/`Path` as public opaque structs that
//! `cbindgen.toml` explicitly excludes) and was missing the vast majority
//! of `sk_*` functions added since it was last regenerated. This test
//! regenerates the header in-memory with the exact same `cbindgen.toml`
//! config used by `build.rs` and fails if either committed copy —
//! `crates/skia-rs-ffi/include/skia_rs.h` or the repo-root
//! `include/skia-rs.h` — has a different *set* of declarations than a fresh
//! generation, so drift is caught by `cargo test` instead of silently
//! accumulating.
//!
//! The comparison is order-independent (sorted line multiset) rather than a
//! literal byte diff: cbindgen's internal item ordering is not guaranteed
//! stable across separate invocations (observed in practice — the same
//! source produced headers with e.g. `FilterMode` and `MipmapMode` swapped
//! between runs), so a byte-for-byte comparison is flaky. Comparing the
//! sorted set of non-blank lines still catches real drift (a function
//! added/removed/renamed, a changed signature or constant value) while
//! tolerating harmless item reordering.
//!
//! Regenerate both committed copies with:
//! `SKIA_RS_FFI_EMIT_HEADER=1 cargo build -p skia-rs-ffi` (you may need to
//! `touch crates/skia-rs-ffi/build.rs` first to force the build script to
//! rerun if only the committed header itself changed).

use std::collections::BTreeSet;
use std::path::PathBuf;

fn generate_header() -> String {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml"))
        .expect("cbindgen.toml must parse");

    let bindings = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("cbindgen generation must succeed");

    let mut buf = Vec::new();
    bindings.write(&mut buf);
    String::from_utf8(buf).expect("generated header must be valid UTF-8")
}

/// Sorted multiset of non-blank lines, for order-independent comparison.
fn line_set(text: &str) -> BTreeSet<&str> {
    text.lines().filter(|l| !l.trim().is_empty()).collect()
}

fn assert_same_declarations(committed: &str, fresh: &str, path_hint: &str) {
    let committed_lines = line_set(committed);
    let fresh_lines = line_set(fresh);

    let missing_from_committed: Vec<_> = fresh_lines.difference(&committed_lines).collect();
    let extra_in_committed: Vec<_> = committed_lines.difference(&fresh_lines).collect();

    assert!(
        missing_from_committed.is_empty() && extra_in_committed.is_empty(),
        "{path_hint} is stale — regenerate with \
         `SKIA_RS_FFI_EMIT_HEADER=1 cargo build -p skia-rs-ffi`\n\
         lines missing from committed header: {missing_from_committed:#?}\n\
         stale lines only in committed header: {extra_in_committed:#?}"
    );
}

#[test]
fn crate_local_header_is_up_to_date() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let committed = std::fs::read_to_string(crate_dir.join("include/skia_rs.h"))
        .expect("crates/skia-rs-ffi/include/skia_rs.h must exist");
    let fresh = generate_header();
    assert_same_declarations(
        &committed,
        &fresh,
        "crates/skia-rs-ffi/include/skia_rs.h",
    );
}

#[test]
fn root_header_is_up_to_date() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_dir
        .ancestors()
        .nth(2)
        .expect("crate_dir must be two levels under the workspace root")
        .to_path_buf();
    let committed = std::fs::read_to_string(workspace_root.join("include/skia-rs.h"))
        .expect("include/skia-rs.h must exist at the workspace root");
    let fresh = generate_header();
    assert_same_declarations(&committed, &fresh, "include/skia-rs.h");
}

#[test]
fn both_committed_headers_declare_the_same_things() {
    // The root header is meant to be a copy of the crate-local one; keep
    // them from drifting apart from each other too.
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_dir
        .ancestors()
        .nth(2)
        .expect("crate_dir must be two levels under the workspace root")
        .to_path_buf();
    let local = std::fs::read_to_string(crate_dir.join("include/skia_rs.h"))
        .expect("crates/skia-rs-ffi/include/skia_rs.h must exist");
    let root = std::fs::read_to_string(workspace_root.join("include/skia-rs.h"))
        .expect("include/skia-rs.h must exist at the workspace root");
    assert_same_declarations(&root, &local, "include/skia-rs.h vs skia_rs.h");
}
