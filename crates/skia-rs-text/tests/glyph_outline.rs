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

#[test]
fn metrics_come_from_font_tables_not_hardcoded_approximations() {
    // demo.ttf has a real hhea table; ascender/descender/line_gap must be
    // pulled from it and scaled by size/upem. The previous hardcoded
    // `-0.8*size, +0.2*size, 0.0` approximation was flagged as gap C-1.
    //
    // demo.ttf: upem=1000, hhea.ascender=1024, hhea.descender=-400,
    //           hhea.line_gap=0. At size=1000 these scale 1:1.
    let font = demo_font(1000.0);
    let m = font.metrics();

    // Screen-space: ascent = -ascender (y-axis inverted).
    assert!(
        (m.ascent - -1024.0).abs() < 0.5,
        "expected ascent ≈ -1024, got {}",
        m.ascent
    );
    assert!(
        (m.descent - 400.0).abs() < 0.5,
        "expected descent ≈ 400 (from hhea.descender=-400), got {}",
        m.descent
    );
    assert!(
        m.leading.abs() < 0.5,
        "expected leading ≈ 0, got {}",
        m.leading
    );

    // Crucially, these values are NOT the old hardcoded approximations
    // (which would be -0.8*1000 = -800 and 0.2*1000 = 200). Anything near
    // those numbers would indicate the fallback path was hit.
    assert!(
        (m.ascent + 800.0).abs() > 50.0,
        "ascent must not equal the -0.8*size approximation"
    );
    assert!(
        (m.descent - 200.0).abs() > 50.0,
        "descent must not equal the +0.2*size approximation"
    );
}

#[test]
fn metrics_scale_linearly_with_size() {
    // Font tables are in font-space units and must scale by size/upem.
    let small = demo_font(10.0).metrics();
    let big = demo_font(100.0).metrics();

    let ratio = big.ascent / small.ascent;
    assert!(
        (ratio - 10.0).abs() < 0.05,
        "ascent should scale 10x with size (got ratio {ratio}: {:?} vs {:?})",
        big.ascent,
        small.ascent
    );
    let dr = big.descent / small.descent;
    assert!(
        (dr - 10.0).abs() < 0.05,
        "descent should scale 10x (got {dr})"
    );
}

#[test]
fn dataless_typeface_still_returns_approximate_metrics() {
    // Gap C-1 fix must NOT regress the dataless-default-typeface path:
    // callers that constructed a Font via `Font::from_size(...)` with no
    // backing font data still get the pre-existing approximation so their
    // layout numbers don't collapse to zero.
    let font = Font::from_size(16.0);
    let m = font.metrics();
    assert!(m.ascent < 0.0);
    assert!(m.descent > 0.0);
    assert!(m.line_height() > 0.0);
}

#[test]
fn is_fixed_pitch_reads_post_table() {
    // Gap C-2: Typeface::is_fixed_pitch must consult the post table via
    // ttf-parser rather than always returning false. demo.ttf is a
    // proportional (non-monospaced) font so this returns false — but
    // importantly it returns *from the parsed data*, exercising the new
    // code path rather than the "always-false" short-circuit.
    let tf = Typeface::from_data(DEMO_TTF.to_vec()).expect("parse");
    assert!(!tf.is_fixed_pitch(), "demo.ttf is proportional");
    // The dataless default typeface cannot read a table; must still
    // return false (no crash, no false-positive).
    let default = Typeface::default_typeface();
    assert!(!default.is_fixed_pitch());
}

#[test]
fn get_widths_delegates_to_hmtx_advances() {
    // Gap N-3: `get_widths` must return the same per-glyph advances as
    // `glyph_advance`, not a uniform `size * 0.5` per character.
    let font = demo_font(100.0);
    // demo.ttf maps only 'A' to a real glyph; other characters are .notdef
    // (gid=0) whose advance is 0.
    let widths = font.get_widths("AB");
    assert_eq!(widths.len(), 2);
    assert!(
        widths[0] > 0.0,
        "expected real advance for 'A', got {}",
        widths[0]
    );
    assert!(
        widths[1].abs() < 0.001,
        "expected 0 advance for missing 'B', got {}",
        widths[1]
    );
}

#[test]
fn get_bounds_uses_real_glyph_bbox() {
    // Gap N-4: bounds must come from ttf_parser::Face::glyph_bounding_box
    // scaled + y-flipped — not a synthetic ascent×descent rectangle.
    let font = demo_font(100.0);
    let bounds = font.get_bounds("A");
    assert_eq!(bounds.len(), 1);
    let r = bounds[0];
    // 'A' ascends above the baseline, so top is negative in screen space.
    assert!(r.top < 0.0, "glyph 'A' top should be above baseline, got {}", r.top);
    // 'A' rests on or near the baseline.
    assert!(
        r.bottom.abs() < 5.0,
        "glyph 'A' bottom should sit near baseline, got {}",
        r.bottom
    );
    // Width is non-zero and corresponds to the outline, not the advance.
    assert!(r.width() > 0.0);
}

#[test]
fn glyph_bounds_returns_real_outline_box() {
    let font = demo_font(100.0);
    let bbox = font.glyph_bounds(1);
    // Same constraints as get_bounds but for a single glyph id.
    assert!(bbox.top < 0.0);
    assert!(bbox.width() > 0.0);
    // Missing glyph yields empty rect.
    assert_eq!(font.glyph_bounds(0), skia_rs_core::Rect::EMPTY);
}
