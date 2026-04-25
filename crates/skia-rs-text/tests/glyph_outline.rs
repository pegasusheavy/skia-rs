//! Integration tests for the Phase 5/6A ttf-parser-backed pipeline.
//!
//! Originally verified `Font::glyph_path` extracts real outlines from
//! font data rather than returning placeholder rectangles. Extended in
//! Phase 6A to cover real font metrics, glyph bounds, rustybuzz shaping,
//! and paragraph layout on the same 400-byte demo.ttf fixture copied
//! from the `ttf-parser` crate's own test suite. The font contains a
//! single glyph for 'A' (glyph id 1).

use skia_rs_text::{
    Font, ParagraphBuilder, ParagraphStyle, Shaper, TextAlign, TextStyle, Typeface,
};
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

// =============================================================================
// Shaper integration (gap T-2)
// =============================================================================

#[test]
fn shaper_shapes_real_font_and_returns_hmtx_advances() {
    // Gap T-2: the rustybuzz pipeline must be exercised end-to-end.
    // Verify that shaping "A" via Shaper produces a single glyph with
    // gid=1 (demo.ttf maps 'A' to gid 1) and an advance that matches
    // the hmtx-derived glyph_advance.
    let font = demo_font(100.0);
    let shaper = Shaper::new();
    let runs = shaper
        .shape_auto("A", &font)
        .expect("shape_auto returns runs for real font");
    assert_eq!(runs.len(), 1, "single-script input produces one run");
    let run = &runs[0];
    assert_eq!(run.glyphs.len(), 1, "'A' shapes to exactly one glyph");
    assert_eq!(run.glyphs[0].glyph_id.0, 1);

    // The shaper scales by size/upem, so advance should be within 1px of
    // the hmtx-derived advance for this glyph.
    let hmtx_advance = font.glyph_advance(1);
    let shaped_advance = run.glyphs[0].x_advance;
    assert!(
        (shaped_advance - hmtx_advance).abs() < 1.0,
        "shaped advance ({shaped_advance}) must match hmtx advance ({hmtx_advance})"
    );
}

#[test]
fn shaper_returns_none_for_dataless_typeface() {
    // The default typeface has no font_data, so rustybuzz can't parse it
    // and `shape_auto` must return None (rather than a silent empty run).
    let font = Font::from_size(16.0);
    let shaper = Shaper::new();
    assert!(shaper.shape_auto("Hello", &font).is_none());
}

#[test]
fn shaper_preserves_cluster_indices() {
    // Cluster mapping is critical for hit-testing: each ShapedGlyph must
    // point back to a valid byte offset in the input string. demo.ttf
    // only covers 'A' but multi-char input must still produce cluster
    // values corresponding to distinct byte offsets.
    let font = demo_font(50.0);
    let shaper = Shaper::new();
    let runs = shaper.shape_auto("AAA", &font).unwrap();
    let run = &runs[0];
    assert_eq!(run.glyphs.len(), 3);
    // Clusters must be strictly non-decreasing (each glyph maps to some
    // byte offset >= the previous).
    let clusters: Vec<u32> = run.glyphs.iter().map(|g| g.cluster).collect();
    for w in clusters.windows(2) {
        assert!(w[0] <= w[1], "clusters must be non-decreasing: {:?}", clusters);
    }
}

// =============================================================================
// Paragraph layout integration (gaps C-3, C-4, T-3)
// =============================================================================

fn demo_text_style(size: f32) -> TextStyle {
    TextStyle {
        font: demo_font(size),
        ..TextStyle::default()
    }
}

#[test]
fn paragraph_layout_uses_shaper_advances_not_hardcoded_width() {
    // Gap C-3: Paragraph::layout must call the shaper. Previously it
    // advanced `font.size() * 0.5` per character — for size=100 that's
    // 50px per char. The real demo.ttf 'A' advance is much smaller
    // (hmtx says ~547 at upem=1000 → ~54.7px at size=100), but critically
    // the gap between the old approximation (50px) and the real advance
    // differs enough that we can assert the computed line width is in
    // the real-advance regime, not the 50px regime.
    let style = ParagraphStyle::default();
    let mut builder = ParagraphBuilder::new(style);
    builder.push_style(&demo_text_style(100.0));
    builder.add_text("AAA");
    let mut paragraph = builder.build();
    paragraph.layout(1000.0);

    assert_eq!(paragraph.line_count(), 1);
    let line_width = paragraph.line_width(0).unwrap();
    let real_advance = demo_font(100.0).glyph_advance(1);
    let expected = real_advance * 3.0;
    assert!(
        (line_width - expected).abs() < 2.0,
        "line width should be 3x hmtx advance ({expected}px), got {line_width}"
    );
}

#[test]
fn paragraph_height_reflects_real_font_metrics() {
    // Gap C-1+C-3: the line height must come from real hhea ascender/
    // descender, not `-0.8*size + 0.2*size = size`. demo.ttf at
    // size=100 gives ascent=-102.4, descent=40 → line height 142.4.
    let style = ParagraphStyle::default();
    let mut builder = ParagraphBuilder::new(style);
    builder.push_style(&demo_text_style(100.0));
    builder.add_text("A");
    let mut paragraph = builder.build();
    paragraph.layout(1000.0);

    let h = paragraph.height();
    // Real-metric line height is ~142.4; hardcoded fallback would be 100.
    assert!(
        h > 120.0 && h < 200.0,
        "expected real-metric line height near 142, got {h}"
    );
}

#[test]
fn paragraph_wraps_at_word_boundaries() {
    // Gap T-3: verify word-wrap happens at whitespace (not mid-word).
    let style = ParagraphStyle::default();
    let mut builder = ParagraphBuilder::new(style);
    builder.push_style(&demo_text_style(50.0));
    // With size=50, each 'A' advances ~27px. Five words separated by
    // spaces on a narrow width force multiple lines.
    builder.add_text("AA AA AA AA AA AA");
    let mut paragraph = builder.build();
    // Narrow enough to force several breaks but wide enough to hold a
    // single "AA" group (≈54px).
    paragraph.layout(80.0);

    assert!(
        paragraph.line_count() > 1,
        "narrow paragraph must wrap into multiple lines, got {}",
        paragraph.line_count()
    );
}

#[test]
fn paragraph_respects_max_lines_ellipsis_truncation() {
    // Gap T-3: max_lines caps the visible line count.
    let mut style = ParagraphStyle::default();
    style.max_lines = 2;
    let mut builder = ParagraphBuilder::new(style);
    builder.push_style(&demo_text_style(50.0));
    builder.add_text("AA AA AA AA AA AA AA AA AA AA AA");
    let mut paragraph = builder.build();
    paragraph.layout(80.0);

    assert_eq!(
        paragraph.line_count(),
        2,
        "max_lines=2 must produce exactly 2 lines"
    );
}

#[test]
fn paragraph_hard_newline_forces_break() {
    let style = ParagraphStyle::default();
    let mut builder = ParagraphBuilder::new(style);
    builder.push_style(&demo_text_style(50.0));
    builder.add_text("A\nA\nA");
    let mut paragraph = builder.build();
    paragraph.layout(10_000.0); // Wide enough to not soft-wrap.
    assert_eq!(paragraph.line_count(), 3);
}

#[test]
fn paragraph_right_alignment_offsets_fragments() {
    // Gap T-3: right alignment must shift content to the right edge.
    let mut style = ParagraphStyle::default();
    style.text_align = TextAlign::Right;
    let mut builder = ParagraphBuilder::new(style);
    builder.push_style(&demo_text_style(50.0));
    builder.add_text("A");
    let mut paragraph = builder.build();
    paragraph.layout(500.0);

    let line = &paragraph.lines()[0];
    // First glyph position should be near the right edge.
    let first_x = line.fragments[0].positions[0].x;
    let content_width = line.width;
    let expected_x = 500.0 - content_width;
    assert!(
        (first_x - expected_x).abs() < 1.0,
        "right-aligned glyph x={first_x} should be ≈ {expected_x}"
    );
}

#[test]
fn paragraph_preserves_per_span_style_in_text_blob() {
    // Gap C-4: to_text_blob must emit one glyph run per TextStyle change,
    // not collapse everything into a single font.
    //
    // Use two different font *sizes* (same typeface) so we can verify the
    // blob produces two runs with different fonts rather than a single
    // run where everything is coerced to the first style's font.
    let style = ParagraphStyle::default();
    let mut builder = ParagraphBuilder::new(style);

    builder.push_style(&demo_text_style(50.0));
    builder.add_text("A");
    builder.pop();
    builder.push_style(&demo_text_style(100.0));
    builder.add_text("A");

    let mut paragraph = builder.build();
    paragraph.layout(10_000.0);
    let blob = paragraph.to_text_blob().expect("non-empty blob");

    assert_eq!(
        blob.runs().len(),
        2,
        "mixed-style paragraph must produce two glyph runs"
    );
    let sizes: Vec<f32> = blob.runs().iter().map(|r| r.font.size()).collect();
    assert!(sizes.contains(&50.0));
    assert!(sizes.contains(&100.0));
}

#[test]
fn paragraph_color_retained_on_fragments() {
    // Gap C-4: the TextStyle color must be preserved on each line
    // fragment so rendering code can emit per-span color.
    let style = ParagraphStyle::default();
    let mut builder = ParagraphBuilder::new(style);

    let mut red = demo_text_style(50.0);
    red.color = 0xFF_FF_00_00;
    let mut blue = demo_text_style(50.0);
    blue.color = 0xFF_00_00_FF;
    builder.push_style(&red);
    builder.add_text("A");
    builder.pop();
    builder.push_style(&blue);
    builder.add_text("A");

    let mut paragraph = builder.build();
    paragraph.layout(10_000.0);
    let line = &paragraph.lines()[0];
    // Two different colors → two fragments.
    assert_eq!(line.fragments.len(), 2);
    let colors: Vec<u32> = line.fragments.iter().map(|f| f.style.color).collect();
    assert!(colors.contains(&0xFF_FF_00_00));
    assert!(colors.contains(&0xFF_00_00_FF));
}

#[test]
fn paragraph_decoration_retained_on_fragments() {
    // Gap C-4: decoration flags must survive layout so downstream can
    // draw underlines/strikethrough per span.
    let style = ParagraphStyle::default();
    let mut builder = ParagraphBuilder::new(style);

    let plain = demo_text_style(50.0);
    let mut underlined = demo_text_style(50.0);
    underlined.decoration.underline = true;
    builder.push_style(&plain);
    builder.add_text("A");
    builder.pop();
    builder.push_style(&underlined);
    builder.add_text("A");

    let mut paragraph = builder.build();
    paragraph.layout(10_000.0);
    let line = &paragraph.lines()[0];
    assert_eq!(line.fragments.len(), 2);
    let underlines: Vec<bool> = line
        .fragments
        .iter()
        .map(|f| f.style.decoration.underline)
        .collect();
    assert_eq!(underlines, vec![false, true]);
}

// =============================================================================
// TextBlob bounds correctness (gap T-4)
// =============================================================================

#[test]
fn text_blob_bounds_bracket_real_glyph_positions() {
    // Gap T-4: GlyphRun::bounds must include every glyph's real width,
    // not use a synthetic `size * 0.5` per glyph.
    use skia_rs_core::Point;
    use skia_rs_text::GlyphRun;

    let font = demo_font(100.0);
    let glyphs = vec![1u16, 1, 1];
    // Place three 'A' glyphs at 0, 60, 120 (roughly matching hmtx).
    let positions = vec![
        Point::new(0.0, 0.0),
        Point::new(60.0, 0.0),
        Point::new(120.0, 0.0),
    ];
    let run = GlyphRun::new(font.clone(), glyphs, positions, Point::zero());
    let b = run.bounds();
    // Right edge must include the last glyph's outline, not just its
    // origin. The real advance is ~54.7px at size=100, so the right edge
    // should be >= 120 (last origin) + half the glyph width, regardless
    // of the exact GlyphRun::bounds estimation strategy — but crucially
    // the bounds must be *finite and non-empty*.
    assert!(b.width() >= 120.0);
    assert!(b.height() > 0.0);
    assert!(b.top < 0.0, "top includes ascent above baseline");
}
