# skia-rs-text Gap Analysis

**Date:** 2026-04-25
**Reviewer:** Claude (Opus 4.7)
**Phase 6A status:** 2026-04-25 — all 14 gaps resolved (partial resolution on
C-5 for COLR v1 gradients and SVG-in-OpenType rendering: the table parsing
is wired and the data is exposed via `glyph_color_layers`/`glyph_svg`, but
high-fidelity gradient rasterization is delegated to downstream paint
pipelines). See per-gap `**Status:**` lines below.

**Phase 7F status:** 2026-04-26 — both Phase 6A follow-ups resolved:
* **P6A-FOLLOWUP-COLR-V1** — `ColorGlyphLayer` now carries typed
  `GlyphPaint` data (Solid / LinearGradient / RadialGradient /
  SweepGradient) with full geometric parameters (3-point linear,
  two-circle radial, center+angles sweep), gradient stops, `extend`
  mode, an effective affine `transform` (y-flipped into screen space),
  an optional `clip_glyph`, and a `composite_mode`. The COLR painter
  walker tracks transform/clip/composite stacks and applies them to
  every emitted layer. Downstream paint pipelines (`skia-rs-paint`
  gradient shaders) can rasterise v1 content faithfully from this data.
* **P6A-FOLLOWUP-SVG-GLYPH** — `skia_rs_svg::glyph_svg_to_dom` added;
  auto-detects gzip magic and decompresses before parsing, returning
  an `SvgDom` ready for rendering via the existing `render_svg_to_canvas`
  pipeline. Lives in `skia-rs-svg` (not `skia-rs-text`) to avoid a
  dependency cycle (`skia-rs-svg` already depends on `skia-rs-text`).

## Summary

- Total public functions reviewed: ~140 (`pub fn` across font.rs, font_mgr.rs, paragraph.rs, shaper.rs, text_blob.rs, typeface.rs)
- Total test functions: 19 baseline → **57 after Phase 6A**
  - font.rs: 4
  - typeface.rs: 3
  - font_mgr.rs: 3 → 8 (+5 covering make_from_data/file and character coverage)
  - shaper.rs: 2
  - paragraph.rs: 4
  - text_blob.rs: 3
  - tests/glyph_outline.rs: 6 → 34 (+28 covering metrics, shaping, paragraph,
    color fonts, and intercept bands)
- Total gaps found: 14
- Critical gaps: 5 — all 5 resolved (C-5 with a documented partial scope for
  COLR v1 gradients).
- Nice-to-have gaps: 5 — all 5 resolved.
- Test coverage gaps: 4 — all 4 resolved.
- Estimated complexity: **Medium** — the font data plumbing (ttf-parser cmap/hmtx/glyph outlines) from Phase 5 is in place; remaining gaps are paragraph/shaping integration and color glyph support.

## Files Reviewed
- [x] lib.rs (27 lines)
- [x] font.rs (655 lines)
- [x] font_mgr.rs (293 lines)
- [x] paragraph.rs (570 lines)
- [x] shaper.rs (417 lines)
- [x] text_blob.rs (287 lines)
- [x] typeface.rs (330 lines)

## Phase 5 context

Earlier Phase 5 work landed real ttf-parser-backed implementations in `font.rs` and `typeface.rs`:
- `Typeface::from_data` actually parses the font and pulls `units_per_em`, glyph count, family name, weight, width, slant.
- `Typeface::char_to_glyph` uses the cmap table via `ttf_parser::Face::glyph_index`.
- `Font::glyph_advance` pulls widths from `hmtx` scaled by `size / units_per_em`.
- `Font::glyph_path` uses `ttf_parser::OutlineBuilder` to generate real y-flipped glyph outlines.
- `Font::measure_text` sums real per-glyph advances.

Those are correct and not re-flagged below. The remaining gaps are in the layers on top: the shaper, paragraph layout, text blobs, font manager, and color/emoji support.

## Critical Gaps

### C-1: `Font::metrics()` returns hardcoded approximations, ignoring font tables
**File:** `font.rs` (lines 261-281)
**Severity:** Critical
**Status:** RESOLVED — commit "feat(text): derive Font metrics and bounds from real font tables". `Font::metrics` now reads `hhea` ascender/descender/line_gap, `post` underline, and `OS/2` x-height/cap-height/strikeout via `ttf_parser::Face`, scaled by `size/upem`. Dataless typeface keeps the previous approximation as a fallback. Verified by `metrics_come_from_font_tables_not_hardcoded_approximations` (exact values) and `metrics_scale_linearly_with_size` (linear scaling).
**Description:** `metrics()` returns fixed multiples of `self.size` (`ascent = -0.8 * size`, `descent = 0.2 * size`, `cap_height = 0.7 * size`, etc.) with a comment "// Approximate". The `scale = self.size / units_per_em` is computed but then discarded. The function never touches the typeface's font data despite having access to it via `self.typeface.font_data()`. `ttf_parser::Face` exposes `ascender()`, `descender()`, `line_gap()`, `x_height()`, `capital_height()`, `underline_metrics()`, `strikeout_metrics()` — none are consulted.
**Impact:** Every FontMetrics-consuming path produces wrong values for real fonts: paragraph line height, baseline positioning, underline/strikethrough placement, decoration positioning in canvas text drawing. All downstream text layout is therefore off, by a font-specific amount (8% of size for a typical font's real ascent).
**Effort:** Small (~40 lines; parse hhea/OS/2 tables via ttf-parser when font_data is present, fall through to approximation only for the dataless default typeface).

### C-2: `Typeface::is_fixed_pitch()` always returns false
**File:** `typeface.rs` (lines 262-265)
**Severity:** Critical
**Status:** RESOLVED — same commit as C-1. `Typeface::is_fixed_pitch` now reads `Face::is_monospaced()` (the `post.isFixedPitch` flag) when font data is present, and returns `false` for the dataless default typeface. Verified by `is_fixed_pitch_reads_post_table`.
**Description:** Commented as "Would need to parse font tables to determine this." `ttf_parser::Face::is_monospaced()` is available and already used indirectly. Monospaced detection is needed for terminal/code font layout, PDF/font embedding (fixed-pitch flag bit 0), and glyph advance caching.
**Impact:** Any API that relies on monospaced hints (e.g., PDF font flags, canvas layout heuristics) sees all fonts as proportional.
**Effort:** Trivial (one-line parse).

### C-3: Paragraph layout uses character-index glyphs instead of shaped glyphs
**File:** `paragraph.rs` (lines 240-306)
**Severity:** Critical
**Status:** RESOLVED — commit "feat(text): route paragraph layout through rustybuzz shaper". `Paragraph::layout` now shapes every `TextRun` via `Shaper::shape_auto`, breaks the output into styled clusters (visible / whitespace / newline), and packs clusters into lines using the real shaped advances. For the dataless default typeface a cmap+hmtx fallback is used so layout still produces non-empty lines. Verified by `paragraph_layout_uses_shaper_advances_not_hardcoded_width`, `paragraph_wraps_at_word_boundaries`, `paragraph_respects_max_lines_ellipsis_truncation`, and `paragraph_hard_newline_forces_break`.
**Description:** `Paragraph::layout()` does its own glyph mapping via `font.char_to_glyph(c)` per character and advances by a hardcoded `font.size() * 0.5`. It never calls the `Shaper` from `shaper.rs`, so kerning, ligatures, OpenType features, RTL reordering, mark-to-base positioning, Arabic shaping, Indic clusters, CJK advances, and emoji sequences are all ignored.
**Impact:** The rustybuzz integration in `shaper.rs` (which is genuinely functional — it calls `rustybuzz::shape`, returns real advances and positions) is unreachable from the top-level `ParagraphBuilder/Paragraph` API. The entire "rich text paragraph layout" module operates at an ASCII-typewriter level.
**Effort:** Medium (rewrite `Paragraph::layout` to call `Shaper::shape_auto` per run, then feed the shaped glyphs into `TextBlobBuilder` with real positions; ~120 lines).

### C-4: `Paragraph` ignores `TextStyle::font`, `TextStyle::color`, `TextStyle::background_color`, `TextStyle::decoration`
**File:** `paragraph.rs` (lines 224-321, 391-412)
**Severity:** Critical
**Status:** RESOLVED — same commit as C-3. The new `TextLine` carries a list of `LineFragment`, one per contiguous same-style run. Each fragment preserves the originating `TextStyle` (including `color`, `background_color`, `decoration`), and `to_text_blob` emits one `GlyphRun` per fragment using the fragment's font. Verified by `paragraph_preserves_per_span_style_in_text_blob` (emits 2 runs for mixed sizes), `paragraph_color_retained_on_fragments`, and `paragraph_decoration_retained_on_fragments`. Per-fragment background fills and physical decoration rendering remain the responsibility of downstream canvas/paint pipelines — the data is now exposed via `Paragraph::lines()`.
**Description:** `ParagraphStyle`/`TextStyle` carry per-span font, color, background, and decoration, but `layout()` consults only `run.style.font` (for metrics) and `letter_spacing`/`word_spacing`. When `to_text_blob()` emits glyph runs it uses a single `line.font` and drops color, decoration, and background entirely. The `TextDecoration` struct (underline/overline/line-through with color/style/thickness) is pure data with no rendering hook.
**Impact:** Mixed-style rich-text rendering (e.g., bold + italic + underlined + colored spans) collapses to a single-font, foreground-only blob. This is the core feature the module exists to provide.
**Effort:** Medium (emit one glyph run per style change; wire color into a parallel "color per glyph" structure or produce multiple TextBlobs; ~100 lines).

### C-5: `Font::glyph_is_color` and `Font::glyph_image` are placeholders (no COLR/CPAL/CBDT/CBLC/sbix/SVG parsing)
**File:** `font.rs` (lines 471-508)
**Severity:** Critical
**Status:** RESOLVED (with documented partial scope for v1 gradient rasterization) — commit "feat(text): real glyph intercepts and color font table parsing".

Resolved in full:
 * `glyph_is_color` now consults `Face::is_color_glyph` (COLR), `glyph_raster_image` (CBDT/CBLC/sbix/bdat), and `glyph_svg_image` — no heuristics.
 * `glyph_image` returns real raster bitmap payloads tagged with a new `GlyphImageFormat` enum (PNG / Mono / Gray2/4/8 / PremulBgra32). PNG bytes are passed through unchanged for decoding via skia-rs-codec; bitmap formats are returned byte-for-byte from the font with documented premultiplied BGRA semantics.
 * `glyph_svg` returns the raw SVG bytes from the `SVG ` table (possibly gzipped).
 * `glyph_color_layers` decomposes COLR v0 and COLR v1 solid-fill paints into a `Vec<ColorGlyphLayer>` via a `ttf_parser::colr::Painter` walker.
 * `color_palette_count` exposes `CPAL` palette count.

Partial scope (documented in API and tests):
 * ~~COLR v1 gradient paints are still surfaced — the walker records them as layers with `is_gradient = true` and a representative solid color sampled from the first gradient stop.~~ **RESOLVED in Phase 7F** (2026-04-26) — `ColorGlyphLayer` now carries typed `GlyphPaint` data with full gradient geometry (3-point linear, two-circle radial, center+start/end-angle sweep), `GradientStop` vectors, `GradientExtend` wrap mode, an effective affine `transform` (y-flipped into screen space), optional `clip_glyph` id from `PaintGlyph` subtrees, and `composite_mode` from `PaintComposite` subtrees. The `ColorLayerWalker` maintains transform / clip / composite stacks. Backward-compat accessors `ColorGlyphLayer::color()` and `ColorGlyphLayer::is_gradient()` are retained as convenience methods over `layer.paint`.
 * ~~SVG-in-OpenType decompression (SVGZ) and rendering is delegated to `skia-rs-svg`.~~ **RESOLVED in Phase 7F** (2026-04-26) — added `skia_rs_svg::glyph_svg_to_dom(raw: &[u8]) -> Option<SvgDom>` and the lower-level `skia_rs_svg::decode_glyph_svg_bytes(raw: &[u8]) -> Option<Vec<u8>>`. Auto-detects the 0x1f 0x8b gzip magic and decompresses via `flate2::read::GzDecoder`, then parses with the existing SVG parser. Plain-text SVG is handled too. Lives in `skia-rs-svg` (not `skia-rs-text`) to avoid a dependency cycle — `skia-rs-svg` already depends on `skia-rs-text` for glyph outline rendering.

Verified by `glyph_is_color_returns_false_for_outline_only_font`, `glyph_image_returns_none_for_non_color_glyph`, `glyph_color_layers_returns_none_for_outline_only_font`, `color_palette_count_is_none_without_cpal`, and `glyph_svg_returns_none_without_svg_table`.
**Description:** `glyph_is_color` returns `glyph > 0x1000` as a crude guess ("assume high glyph IDs might be emoji"). `glyph_image` synthesizes a solid yellow-ish rectangle with an explicit "placeholder" comment. No actual color font table parsing is performed. `ttf_parser` exposes the `colr` submodule and `tables::cbdt`, `tables::cblc`, `tables::sbix`, `tables::svg` which would feed a real implementation.
**Impact:** Emoji and other color fonts (Noto Color Emoji, Segoe UI Emoji, Apple Color Emoji, twemoji) render as yellow squares. The `GlyphImage` data type is unreachable from any real font.
**Effort:** High (COLR v0 is straightforward layered glyphs with palette; COLR v1 is a full graph with gradients/transforms; CBDT/CBLC and sbix require PNG decode delegation to skia-rs-codec; SVG-in-OpenType requires parsing the embedded SVG and delegating to skia-rs-svg). Realistic scope: COLR v0 + CBDT/CBLC first, then v1 and SVG later. ~400-600 lines.

## Nice-to-Have Gaps

### N-1: `DefaultFontMgr::make_from_data` and `make_from_file` ignore their arguments
**File:** `font_mgr.rs` (lines 175-183)
**Severity:** Nice-to-have
**Status:** RESOLVED — commit "feat(text): wire FontMgr font loading and character-aware fallback". `make_from_data` now delegates to `Typeface::from_data`, returning `None` on parse failure. `make_from_file` reads via `std::fs::read` and propagates I/O errors as `None`. Verified by `make_from_data_parses_real_font`, `make_from_data_rejects_bad_bytes`, and `make_from_file_reads_and_parses`.
**Description:** Both methods return `Typeface::default_typeface()` with an acknowledging "Placeholder" comment. `Typeface::from_data` already exists and parses real font data; calling it here is trivial. `make_from_file` additionally needs `std::fs::read` with error handling.
**Impact:** Users who think they are loading a real font via the `FontMgr` trait silently get the default typeface with no font data, which breaks everything downstream that depends on cmap/hmtx (i.e., every feature from Phase 5).
**Effort:** Trivial (~10 lines).

### N-2: `DefaultFontMgr::match_family_style_character` does not check character coverage
**File:** `font_mgr.rs` (lines 162-173)
**Severity:** Nice-to-have
**Status:** RESOLVED — same commit as N-1. `match_family_style_character` now checks cmap coverage via `Typeface::char_to_glyph`, preferring a typeface in the requested family, then scanning all registered families for a style-nearest covering typeface, before falling back to a style-only match. Verified by `match_family_style_character_prefers_covering_typeface`.
**Description:** Ignores the `bcp47` and `character` parameters. Comment says "A real implementation would check if the character is in the font." The Phase 5 `Typeface::char_to_glyph` returns 0 for missing glyphs, which is the canonical coverage test. Looping over registered families and picking the first whose typeface returns nonzero is straightforward.
**Impact:** Emoji/CJK/script fallback is non-functional from the public `FontMgr` API; callers fall back to the default family regardless of character.
**Effort:** Small (~20 lines).

### N-3: `Font::get_widths` ignores font data and returns `size * 0.5 * scale_x` for every character
**File:** `font.rs` (lines 313-318)
**Severity:** Nice-to-have
**Status:** RESOLVED — same commit as C-1/C-2. `get_widths` now delegates to `glyph_advance`, so per-character widths agree with `measure_text` and come from `hmtx` when present. Verified by `get_widths_delegates_to_hmtx_advances`.
**Description:** The function is documented as a "Simple approximation". Inconsistent with `measure_text` (C-1 Phase 5 fix), which correctly maps chars to glyphs and uses `glyph_advance`. Callers of `get_widths` (for character-level positioning in canvas text) get a uniform fake width.
**Impact:** Canvas text operations that use per-character widths (hit testing, caret positioning) are wrong.
**Effort:** Trivial (delegate to `chars().map(|c| self.glyph_advance(self.char_to_glyph(c)))`).

### N-4: `Font::get_bounds` uses the `get_widths` approximation, not real glyph bounds
**File:** `font.rs` (lines 320-336)
**Severity:** Nice-to-have
**Status:** RESOLVED — same commit as N-3. `Font::get_bounds` and `Font::glyph_bounds` now return `Face::glyph_bounding_box` scaled by `size/upem` and y-flipped into screen space. Verified by `get_bounds_uses_real_glyph_bbox` and `glyph_bounds_returns_real_outline_box`.
**Description:** Same pattern as N-3 — hardcoded `size * 0.5 * scale_x` per character. `ttf_parser::Face::glyph_bounding_box` returns the real per-glyph bbox and could feed a scaled `Rect`.
**Effort:** Small.

### N-5: `Font::glyph_intercepts` returns rectangular-bbox approximation, not real outline intercepts
**File:** `font.rs` (lines 531-561)
**Severity:** Nice-to-have
**Status:** RESOLVED — same commit as C-5. `glyph_intercepts` now flattens each glyph's Path into line segments (recursive de Casteljau for quad/cubic, conics approximated as quads at tolerance max(band/32, 0.25 px)), clips each segment to the y-band, and returns sorted enter/exit x-pairs. Dataless typeface falls back to the bbox test. Verified by `glyph_intercepts_band_spans_whole_glyph`, `glyph_intercepts_empty_for_band_above_glyph`, and `glyph_intercepts_fallback_for_dataless_typeface`.
**Description:** Documented as "Placeholder — returns approximated intercepts" using bbox tests. Real implementation intersects the glyph outline with the horizontal band `[top, bottom]`. Used for underline gap rendering (to avoid drawing through descenders like g/y/p). Current placeholder produces a continuous underline with no gaps.
**Effort:** Medium (walk the `Path` contour, intersect with horizontal lines, return crossing x-values).

## Test Coverage Gaps

### T-1: No tests verify ttf-parser integration for real fonts
**Description:** The Phase 5 work that made `Typeface::from_data`, `Font::glyph_advance`, and `Font::glyph_path` correct has no integration test with an actual TTF file in the test suite. `test_char_to_glyph` (typeface.rs) uses the dataless default typeface and asserts `'A' -> 65` via the ASCII fallback — which exercises the fallback path, not the cmap path. Without a bundled test font, regressions in the ttf-parser wiring would not be caught.
**Effort:** Medium (include a tiny permissive-licensed TTF in test fixtures and add tests that assert specific non-ASCII glyph IDs, real advances, real glyph-path command counts).
**Status:** RESOLVED (in Phase 5, confirmed + extended in Phase 6A). The bundled 400-byte `tests/fixtures/demo.ttf` (copied from ttf-parser's own test suite, public domain) backs 34 integration tests in `tests/glyph_outline.rs` — cmap lookups, hmtx advances, glyph path verb count, metrics, shaping, paragraph layout, color font detection, and intercept bands. Regression coverage for the ttf-parser wiring is comprehensive.

### T-2: No tests for Shaper
**Description:** `shaper.rs` has two tests covering direction and script detection (pure Unicode-range tables). The actual shaping pipeline — `Shaper::shape`, `Shaper::shape_auto`, `create_face`, and conversion of `rustybuzz::glyph_infos`/`glyph_positions` into `ShapedGlyph` — is never exercised.
**Effort:** Medium (requires test font; verify glyph count, positions, and cluster mapping for a known string).
**Status:** RESOLVED — same commit as C-3. Three integration tests drive the rustybuzz path end-to-end: `shaper_shapes_real_font_and_returns_hmtx_advances` (verifies the returned glyph ids and advances match the font's hmtx), `shaper_returns_none_for_dataless_typeface` (negative path when rustybuzz can't parse), and `shaper_preserves_cluster_indices` (cluster mapping invariants for hit testing).

### T-3: Paragraph layout tests only check that `height() > 0` and that the paragraph builds
**Description:** `test_paragraph_layout` asserts the paragraph has nonzero height and laid-out state. No test verifies line count for a known width/text combination, word-wrap boundaries, `max_lines` truncation, alignment (left/right/center), or mixed-style runs.
**Effort:** Small.
**Status:** RESOLVED — same commit as C-3/C-4. Eight paragraph-level tests cover: `paragraph_layout_uses_shaper_advances_not_hardcoded_width`, `paragraph_height_reflects_real_font_metrics`, `paragraph_wraps_at_word_boundaries`, `paragraph_respects_max_lines_ellipsis_truncation`, `paragraph_hard_newline_forces_break`, `paragraph_right_alignment_offsets_fragments`, `paragraph_preserves_per_span_style_in_text_blob`, `paragraph_color_retained_on_fragments`, and `paragraph_decoration_retained_on_fragments`.

### T-4: TextBlob bounds computation never tested for correctness
**Description:** `test_glyph_run_bounds` asserts `width > 0 && height > 0` — it doesn't verify that the bounds actually bracket all glyph positions. The `GlyphRun::bounds` code uses `font.size() * 0.5` as a fake per-glyph width estimate (same as C-3 pattern) and would pass a trivially-width test but fail at anything precise.
**Effort:** Small.
**Status:** RESOLVED — same commit as C-3. `GlyphRun::bounds` now uses per-glyph hmtx advances (not `size * 0.5`), and `text_blob_bounds_bracket_real_glyph_positions` verifies the bounds width covers the last glyph's origin + its real width, and that the top dips above the baseline by the font's real ascent.

## Implementation Notes

### Architecture
Clean layering: `Typeface` (font file) → `Font` (typeface + size/edging/hinting) → `Shaper` (rustybuzz integration) → `Paragraph` (line breaking + alignment) → `TextBlob` (positioned glyph runs). The wiring between `Shaper` and `Paragraph` is the critical gap (C-3).

### Phase 5 quality
The ttf-parser integration in `typeface.rs`/`font.rs` is real and correct: glyph outlines use `ttf_parser::OutlineBuilder` with proper y-flip and `size/upem` scale; cmap lookups via `glyph_index`; hmtx via `glyph_hor_advance`. The glyph path code is the strongest part of the crate.

### rustybuzz integration
`Shaper::shape` is a genuine rustybuzz pipeline — buffer setup, direction, script, language, `rustybuzz::shape()`, and output conversion. The only issue is that `Paragraph::layout` does not use it (C-3). Once `Paragraph` calls the shaper, the crate gets real OpenType shaping for free.

### Font manager is a registry, not a system-font enumerator
`DefaultFontMgr` is an in-memory font registry, not a platform font enumerator. There is no integration with fontconfig (Linux), DirectWrite (Windows), or CoreText (macOS). The `fontdb` crate is already depended on via shaper.rs but not used by `DefaultFontMgr`. System-font enumeration is not currently in-scope; it would be a separate backend module.

### `glyph_is_color` heuristic vs. detection
The "glyph > 0x1000" heuristic is wrong for most fonts (many regular fonts have glyph counts exceeding 4096) and underreaches for small emoji-only fonts. Real detection requires checking whether the font has COLR/CBDT/sbix/SVG tables (`ttf_parser::Face::tables().colr`, etc.).

## Recommendations

### Priority 1: Wire rustybuzz shaping into paragraph layout (C-3, C-4)
The rustybuzz pipeline exists and works — it is simply not connected. Fixing C-3 unlocks real text layout; C-4 unlocks per-span styling. Estimated effort: 2-3 days.
**Done in Phase 6A** — see commit "feat(text): route paragraph layout through rustybuzz shaper".

### Priority 2: Real FontMetrics (C-1, C-2)
Ten-minute fix that immediately improves line-height, baseline, and underline positioning across the entire crate. Do this first; it is the highest value-per-hour item.
**Done in Phase 6A**.

### Priority 3: Wire `FontMgr::make_from_data/make_from_file` to `Typeface::from_data` (N-1)
One-line fix, unblocks public font-loading API.
**Done in Phase 6A**.

### Priority 4: Color glyph / emoji support (C-5)
Scope as COLR v0 + CBDT/CBLC in Phase 6, defer COLR v1 and SVG-in-OpenType. ~400 lines, 3-4 days.
**Done in Phase 6A** for the table-parsing + data-exposure layer (COLR v0/v1 layer decomposition, CBDT/CBLC/sbix/bdat raster extraction, SVG-in-OpenType passthrough). Gradient rasterization for COLR v1 and SVGZ rendering remain as follow-up tasks (P6A-FOLLOWUP-COLR-V1 and P6A-FOLLOWUP-SVG-GLYPH) since they require integration with downstream crates (skia-rs-paint for gradients, skia-rs-svg for SVGZ decode+render).

### Priority 5: Test suite improvements (T-1 through T-4)
Bundle a tiny OFL/SIL-licensed TTF (e.g., stripped-down Roboto or Noto Sans Tofu) in tests and drive the ttf-parser, shaper, and paragraph pipelines against it. Estimated effort: 1 day.
**Done in Phase 6A**. demo.ttf (400-byte public-domain fixture from ttf-parser) backs 34 integration tests covering every code path flagged in T-1..T-4.
