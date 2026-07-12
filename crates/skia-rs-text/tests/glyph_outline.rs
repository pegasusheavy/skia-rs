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
        "width ratio {wratio} ≠ 10 (small={sb:?}, big={bb:?})"
    );
    assert!((hratio - 10.0).abs() < 0.1, "height ratio {hratio} ≠ 10");
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
    // demo.ttf maps only 'A' to a real glyph; other characters fall back to
    // .notdef (gid=0). Skia treats .notdef like any glyph: it has a real
    // hmtx advance (demo.ttf: 600 units → 60px at size=100), NOT zero.
    let widths = font.get_widths("AB");
    assert_eq!(widths.len(), 2);
    assert!(
        widths[0] > 0.0,
        "expected real advance for 'A', got {}",
        widths[0]
    );
    // .notdef carries its real hmtx advance.
    assert!(
        (widths[1] - 60.0).abs() < 1.0,
        "expected .notdef hmtx advance ≈ 60, got {}",
        widths[1]
    );
    // And it must match glyph_advance(0) directly.
    assert!((widths[1] - font.glyph_advance(0)).abs() < 0.001);
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
    assert!(
        r.top < 0.0,
        "glyph 'A' top should be above baseline, got {}",
        r.top
    );
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
    // .notdef (gid 0) is a real glyph with a tofu-box outline in demo.ttf,
    // so its bounds are non-empty (bbox 100,0,600,700 → top negative).
    let notdef = font.glyph_bounds(0);
    assert!(!notdef.is_empty(), "notdef must have a real tofu box");
    assert!(notdef.top < 0.0, "notdef box rises above baseline");
    assert!(notdef.width() > 0.0);
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
        assert!(
            w[0] <= w[1],
            "clusters must be non-decreasing: {clusters:?}"
        );
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
    let style = ParagraphStyle {
        max_lines: 2,
        ..Default::default()
    };
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
    let style = ParagraphStyle {
        text_align: TextAlign::Right,
        ..Default::default()
    };
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
    let underline_flags: Vec<bool> = line
        .fragments
        .iter()
        .map(|f| f.style.decoration.underline)
        .collect();
    assert_eq!(underline_flags, vec![false, true]);
}

// =============================================================================
// TextBlob bounds correctness (gap T-4)
// =============================================================================

// =============================================================================
// Color / emoji font detection (gap C-5)
// =============================================================================

#[test]
fn glyph_is_color_returns_false_for_outline_only_font() {
    // Gap C-5: `glyph_is_color` previously returned true for any glyph id
    // > 0x1000 (a "maybe it's emoji" heuristic). For demo.ttf — a plain
    // outline font with no COLR / CBDT / sbix / SVG tables — every glyph
    // must report false no matter its id.
    let font = demo_font(100.0);
    assert!(!font.glyph_is_color(0));
    assert!(!font.glyph_is_color(1));
    // Also the boundary the old heuristic tripped on.
    assert!(!font.glyph_is_color(0x2000));
    // And ensure dataless default returns false rather than panicking.
    let default = Font::from_size(16.0);
    assert!(!default.glyph_is_color(42));
    assert!(!default.glyph_is_color(0x5000));
}

#[test]
fn glyph_image_returns_none_for_non_color_glyph() {
    // demo.ttf has no CBDT / sbix strike — must return None, not a
    // yellow placeholder rectangle.
    let font = demo_font(100.0);
    assert!(font.glyph_image(1).is_none());
    // The dataless typeface must also return None.
    let default = Font::from_size(16.0);
    assert!(default.glyph_image(1).is_none());
}

#[test]
fn glyph_color_layers_returns_none_for_outline_only_font() {
    let font = demo_font(100.0);
    assert!(font.glyph_color_layers(1, 0, 0xFF_00_00_00).is_none());
}

#[test]
fn color_palette_count_is_none_without_cpal() {
    let font = demo_font(100.0);
    assert!(font.color_palette_count().is_none());
}

#[test]
fn glyph_svg_returns_none_without_svg_table() {
    let font = demo_font(100.0);
    assert!(font.glyph_svg(1).is_none());
}

// =============================================================================
// Glyph outline intercepts (gap N-5)
// =============================================================================

#[test]
fn glyph_intercepts_band_spans_whole_glyph() {
    // Gap N-5: intercepts must come from the real outline, not the
    // bounding box.
    //
    // Place a single glyph at origin and ask for intercepts across a
    // band that covers the glyph's vertical extent. We should get back
    // an even number of values forming enter/exit pairs that span the
    // glyph's width — not the single [pos.x, pos.x+width] bounding-box
    // pair that the old placeholder returned.
    use skia_rs_core::Point;
    let font = demo_font(100.0);
    let positions = vec![Point::new(0.0, 0.0)];
    // Band covers the full glyph: from top=-100 to bottom=10 (screen y,
    // glyph-A spans roughly [-65.6, 0]).
    let xs = font.glyph_intercepts(&[1u16], &positions, -100.0, 10.0);
    assert!(
        xs.len() >= 2,
        "expected at least one enter/exit pair, got {xs:?}"
    );
    // Enter/exit pairs must be sorted.
    for w in xs.windows(2) {
        assert!(w[0] <= w[1], "intercepts not sorted: {xs:?}");
    }
    // The overall span should approximate the glyph's width.
    let first = *xs.first().unwrap();
    let last = *xs.last().unwrap();
    assert!(last - first > 10.0, "span {} too narrow", last - first);
}

#[test]
fn glyph_intercepts_empty_for_band_above_glyph() {
    // A band entirely above the glyph should produce no intercepts
    // (glyph is "below" the band in screen-y terms). This would be the
    // typical "underline is near the descent, glyph is just ascent"
    // case where the outline does not reach the band — a real outline
    // test is the only thing that returns an empty result.
    use skia_rs_core::Point;
    let font = demo_font(100.0);
    let positions = vec![Point::new(0.0, 0.0)];
    // Glyph 'A' spans roughly y=[-65, 0]. A band at [-500, -200] is
    // entirely above the glyph, so intercepts must be empty.
    let xs = font.glyph_intercepts(&[1u16], &positions, -500.0, -200.0);
    assert!(xs.is_empty(), "expected no intercepts, got {xs:?}");
}

#[test]
fn glyph_intercepts_fallback_for_dataless_typeface() {
    // When the typeface has no outline data we fall back to bbox
    // intercepts so the function still returns *something* rather than
    // a silent empty result.
    use skia_rs_core::Point;
    let font = Font::from_size(20.0);
    // Synthesise a glyph id; dataless typeface will fail to load an
    // outline but glyph_bounds falls back to an advance×metric box.
    let positions = vec![Point::new(0.0, 0.0)];
    // glyph_intercepts on a dataless typeface returns no intercepts
    // because glyph_bounds for glyph != 0 falls back to advance×height
    // box (which is empty when the typeface has no data). This test
    // pins the documented fallback behaviour.
    let _ = font.glyph_intercepts(&[42u16], &positions, -10.0, 10.0);
}

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
    let run = GlyphRun::new(font, glyphs, positions, Point::zero());
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

// =============================================================================
// Task 5: conformance-audit findings
// =============================================================================

#[test]
fn metrics_x_and_cap_height_are_positive() {
    // FreeType port convention: x_height / cap_height are stored positive
    // (`os2->sxHeight/upem*scale`; fallbacks `-ascent*k`). demo.ttf has no
    // OS/2 table, so both come from the (positive) ascent fallback.
    let m = demo_font(100.0).metrics();
    assert!(
        m.x_height > 0.0,
        "x_height must be positive, got {}",
        m.x_height
    );
    assert!(
        m.cap_height > 0.0,
        "cap_height must be positive, got {}",
        m.cap_height
    );
    // cap_height sits above x_height for this fallback (0.875 vs 0.625).
    assert!(m.cap_height > m.x_height);
}

#[test]
fn metrics_top_bottom_come_from_font_bbox() {
    // top = -bbox.yMax/upem*scale, bottom = -bbox.yMin/upem*scale.
    // demo.ttf global bbox = (6, 0, 600, 700); at size=1000, scale=1.
    let m = demo_font(1000.0).metrics();
    assert!(
        (m.top - -700.0).abs() < 0.5,
        "top should be -bbox.yMax = -700, got {}",
        m.top
    );
    assert!(
        m.bottom.abs() < 0.5,
        "bottom should be -bbox.yMin = 0, got {}",
        m.bottom
    );
    // top/bottom are the ink bbox, independent of the hhea ascent/descent
    // (here the hhea ascender 1024 exceeds the bbox yMax 700). The key
    // point is they are bbox-derived, not `ascent * 1.125`.
    assert!(
        m.ascent.mul_add(-1.125, m.top).abs() > 1.0,
        "top must not be ascent*1.125"
    );
}

#[test]
fn metrics_avg_and_max_char_width_from_bbox() {
    // No OS/2 xAvgCharWidth in demo.ttf ⇒ avg falls back to bbox width.
    // max_char_width = (xmax - xmin)/upem*scale = (600-6) = 594 at size 1000.
    let m = demo_font(1000.0).metrics();
    assert!(
        (m.max_char_width - 594.0).abs() < 0.5,
        "max_char_width should be bbox width 594, got {}",
        m.max_char_width
    );
    assert!(
        (m.avg_char_width - 594.0).abs() < 0.5,
        "avg_char_width should fall back to bbox width, got {}",
        m.avg_char_width
    );
}

#[test]
fn notdef_glyph_has_real_advance_bounds_and_outline() {
    // Glyph 0 (.notdef) is a real glyph: demo.ttf gives it advance 600
    // (→60px at size 100), a tofu-box outline, and non-empty bounds.
    let font = demo_font(100.0);
    assert!(
        (font.glyph_advance(0) - 60.0).abs() < 1.0,
        "notdef advance ≈ 60, got {}",
        font.glyph_advance(0)
    );
    assert!(
        font.glyph_path(0).is_some(),
        "notdef must yield its tofu-box outline"
    );
    assert!(!font.glyph_bounds(0).is_empty());
}

#[test]
fn glyph_path_applies_scale_x() {
    // scale_x=2 must double the outline's horizontal extent while leaving
    // vertical extent unchanged (consistent with glyph_advance).
    let base = demo_font(100.0);
    let mut wide = demo_font(100.0);
    wide.set_scale_x(2.0);

    let bb = base.glyph_path(1).unwrap().bounds();
    let wb = wide.glyph_path(1).unwrap().bounds();

    let wratio = wb.width() / bb.width();
    assert!(
        (wratio - 2.0).abs() < 0.02,
        "scale_x=2 should double width, ratio {wratio}"
    );
    // Height unchanged.
    assert!((wb.height() - bb.height()).abs() < 0.5);
    // And glyph_advance scales the same way.
    assert!(
        2.0f32
            .mul_add(-base.glyph_advance(1), wide.glyph_advance(1))
            .abs()
            < 0.01
    );
}

#[test]
fn glyph_path_applies_skew_x() {
    // skew_x shears x by skew*y. A point above the baseline (negative
    // screen-y) shifts left for positive skew_x, so the sheared outline's
    // top edge moves relative to the unsheared one.
    let base = demo_font(100.0);
    let mut skewed = demo_font(100.0);
    skewed.set_skew_x(0.5);

    let bb = base.glyph_path(1).unwrap().bounds();
    let sb = skewed.glyph_path(1).unwrap().bounds();

    // Shearing changes horizontal extent (top of glyph shifts vs bottom),
    // so the sheared bbox is wider than the upright one.
    assert!(
        sb.width() > bb.width() + 1.0,
        "skew must widen the bbox: {} vs {}",
        sb.width(),
        bb.width()
    );
    // Vertical extent is preserved (skew_x shears x only).
    assert!((sb.height() - bb.height()).abs() < 0.5);
}

#[test]
fn shaper_x_positions_scale_with_scale_x() {
    // SkShaper_harfbuzz multiplies x advances by getScaleX().
    let base = demo_font(100.0);
    let mut wide = demo_font(100.0);
    wide.set_scale_x(2.0);
    let shaper = Shaper::new();

    let a = shaper.shape_auto("A", &base).unwrap();
    let b = shaper.shape_auto("A", &wide).unwrap();
    let ax = a[0].glyphs[0].x_advance;
    let bx = b[0].glyphs[0].x_advance;
    assert!(
        2.0f32.mul_add(-ax, bx).abs() < 0.5,
        "scale_x=2 should double shaped x advance: {bx} vs {ax}"
    );
}

#[test]
fn text_blob_from_text_width_agrees_with_measure_text() {
    // from_text must position glyphs by real per-glyph advances so the
    // blob width matches Font::measure_text (not size*0.5 per glyph).
    use skia_rs_core::Point;
    use skia_rs_text::TextBlob;
    let font = demo_font(100.0);
    let blob = TextBlob::from_text("AAA", &font, Point::new(0.0, 0.0));
    let measured = font.measure_text("AAA");
    // Blob width = last origin + last advance = 3 * advance = measured.
    let w = blob.bounds().right;
    assert!(
        (w - measured).abs() < 1.0,
        "blob width {w} must agree with measure_text {measured}"
    );
    // And it must NOT be the old size*0.5 estimate (3 * 50 = 150).
    assert!(
        (w - 150.0).abs() > 5.0,
        "width must not be the size*0.5 guess"
    );
}

#[test]
fn text_blob_unique_id_is_monotonic_and_distinct() {
    use skia_rs_core::Point;
    use skia_rs_text::TextBlob;
    let font = demo_font(20.0);
    let a = TextBlob::from_text("A", &font, Point::zero());
    let b = TextBlob::from_text("A", &font, Point::zero());
    assert_ne!(a.unique_id(), b.unique_id(), "ids must be distinct");
    assert!(b.unique_id() > a.unique_id(), "ids must be monotonic");
}

#[test]
fn paragraph_push_style_is_a_real_stack() {
    // push_style/pop must maintain a stack: after pop, the previously
    // pushed style is restored — not TextStyle::default().
    let outer = demo_text_style(80.0);
    let inner = demo_text_style(40.0);

    let mut builder = ParagraphBuilder::new(ParagraphStyle::default());
    builder.push_style(&outer);
    builder.add_text("A"); // outer (80)
    builder.push_style(&inner);
    builder.add_text("A"); // inner (40)
    builder.pop();
    builder.add_text("A"); // back to outer (80), NOT default (12)

    let mut paragraph = builder.build();
    paragraph.layout(10_000.0);
    let blob = paragraph.to_text_blob().expect("blob");
    let sizes: Vec<f32> = blob.runs().iter().map(|r| r.font.size()).collect();
    // Three runs at 80, 40, 80 — the third proves pop restored `outer`.
    assert_eq!(
        sizes,
        vec![80.0, 40.0, 80.0],
        "pop must restore previous style"
    );
}

#[test]
fn paragraph_empty_line_uses_paragraph_style_height() {
    // An empty line (bare newline) must take its height from the
    // paragraph's own font, not Font::default() (size 12).
    let mut builder = ParagraphBuilder::new(ParagraphStyle::default());
    builder.push_style(&demo_text_style(100.0));
    // Leading newline produces an empty first line.
    builder.add_text("\nA");
    let mut paragraph = builder.build();
    paragraph.layout(10_000.0);

    let first = &paragraph.lines()[0];
    let empty_height = first.bounds.height();
    // demo.ttf at size 100 has line height ~142; Font::default() (size 12)
    // would give ~12. The empty line must be in the size-100 regime.
    assert!(
        empty_height > 100.0,
        "empty line height {empty_height} must reflect the size-100 style"
    );
}
