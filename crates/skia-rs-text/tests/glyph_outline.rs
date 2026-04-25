//! Verifies `Font::glyph_path` extracts real outlines from font data
//! rather than returning placeholder rectangles.
//!
//! Uses a 400-byte fixture font copied from the `ttf-parser` crate's own
//! test suite. The font contains a single glyph for 'A' (glyph id 1).

use skia_rs_text::{Font, Typeface};
use std::sync::Arc;

const DEMO_TTF: &[u8] = include_bytes!("fixtures/demo.ttf");

fn demo_font(size: f32) -> Font {
    let tf = Typeface::from_data(DEMO_TTF.to_vec()).expect("parse demo.ttf");
    Font::new(Arc::new(tf), size)
}

#[test]
fn typeface_from_data_parses_upem_and_glyph_count() {
    let tf = Typeface::from_data(DEMO_TTF.to_vec()).expect("parse");
    assert_eq!(tf.units_per_em(), 1000, "demo.ttf has upem=1000");
    assert_eq!(tf.glyph_count(), 2, "demo.ttf has 2 glyphs (.notdef + 'A')");
}

#[test]
fn char_to_glyph_uses_cmap_when_data_present() {
    let tf = Typeface::from_data(DEMO_TTF.to_vec()).expect("parse");
    // 'A' is the only glyph in demo.ttf with a cmap entry; it maps to gid 1.
    assert_eq!(tf.char_to_glyph('A'), 1);
    // Other characters have no cmap entry and must return .notdef (0).
    assert_eq!(tf.char_to_glyph('B'), 0);
    assert_eq!(tf.char_to_glyph('0'), 0);
}

#[test]
fn glyph_path_returns_none_for_dataless_typeface() {
    // The default typeface has no font data, so glyph_path must return
    // None — never a placeholder rectangle.
    let font = Font::from_size(16.0);
    assert!(
        font.glyph_path(1).is_none(),
        "dataless typeface must not fake a glyph outline"
    );
}

#[test]
fn glyph_path_extracts_real_outline() {
    let font = demo_font(100.0);
    let path = font.glyph_path(1).expect("'A' glyph has an outline");

    // The outline must have multiple verbs (ttf-parser reports 15 ops for
    // the 'A' in demo.ttf). A placeholder rectangle would be exactly 5
    // verbs (move + 3 line + close). Anything materially larger proves
    // we're pulling real data.
    let verb_count = path.verb_count();
    assert!(
        verb_count > 6,
        "expected more than a rectangle (got {verb_count} verbs)"
    );
}

#[test]
fn glyph_path_scales_with_font_size() {
    // Bounding box should scale linearly with the requested font size,
    // because the path is emitted in `size / units_per_em` units.
    let small = demo_font(10.0).glyph_path(1).unwrap();
    let big = demo_font(100.0).glyph_path(1).unwrap();

    let sb = small.bounds();
    let bb = big.bounds();

    // 10x font size ⇒ ~10x bbox width/height, allow 1% slack for rounding.
    let wratio = bb.width() / sb.width();
    let hratio = bb.height() / sb.height();
    assert!(
        (wratio - 10.0).abs() < 0.1,
        "width ratio {wratio} ≠ 10 (small={:?}, big={:?})",
        sb,
        bb
    );
    assert!(
        (hratio - 10.0).abs() < 0.1,
        "height ratio {hratio} ≠ 10"
    );
}

#[test]
fn glyph_path_y_axis_is_flipped_into_screen_space() {
    // Font outlines are y-up; we flip to y-down so the glyph origin sits
    // on the baseline with the cap extending into negative y. The
    // bounding box should therefore straddle the baseline at y=0 with a
    // negative `top` value (glyph rises above baseline).
    let font = demo_font(100.0);
    let path = font.glyph_path(1).unwrap();
    let bounds = path.bounds();
    assert!(
        bounds.top < 0.0,
        "glyph 'A' should extend above baseline (bounds.top={})",
        bounds.top
    );
    // y_min in demo.ttf is 0, so bottom should be ≈ 0 after flipping.
    assert!(
        bounds.bottom.abs() < 1.0,
        "glyph 'A' should rest on baseline (bounds.bottom={})",
        bounds.bottom
    );
}
