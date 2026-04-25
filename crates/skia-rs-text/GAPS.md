# skia-rs-text Gap Analysis

**Date:** 2026-04-25
**Reviewer:** Claude (Opus 4.7)

## Summary

- Total public functions reviewed: ~140 (`pub fn` across font.rs, font_mgr.rs, paragraph.rs, shaper.rs, text_blob.rs, typeface.rs)
- Total test functions: 19 (all passing)
  - font.rs: 4
  - typeface.rs: 3
  - font_mgr.rs: 3
  - shaper.rs: 2
  - paragraph.rs: 4
  - text_blob.rs: 3
- Total gaps found: 14
- Critical gaps: 5 (functional correctness blockers)
- Nice-to-have gaps: 5
- Test coverage gaps: 4
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
**Description:** `metrics()` returns fixed multiples of `self.size` (`ascent = -0.8 * size`, `descent = 0.2 * size`, `cap_height = 0.7 * size`, etc.) with a comment "// Approximate". The `scale = self.size / units_per_em` is computed but then discarded. The function never touches the typeface's font data despite having access to it via `self.typeface.font_data()`. `ttf_parser::Face` exposes `ascender()`, `descender()`, `line_gap()`, `x_height()`, `capital_height()`, `underline_metrics()`, `strikeout_metrics()` — none are consulted.
**Impact:** Every FontMetrics-consuming path produces wrong values for real fonts: paragraph line height, baseline positioning, underline/strikethrough placement, decoration positioning in canvas text drawing. All downstream text layout is therefore off, by a font-specific amount (8% of size for a typical font's real ascent).
**Effort:** Small (~40 lines; parse hhea/OS/2 tables via ttf-parser when font_data is present, fall through to approximation only for the dataless default typeface).

### C-2: `Typeface::is_fixed_pitch()` always returns false
**File:** `typeface.rs` (lines 262-265)
**Severity:** Critical
**Description:** Commented as "Would need to parse font tables to determine this." `ttf_parser::Face::is_monospaced()` is available and already used indirectly. Monospaced detection is needed for terminal/code font layout, PDF/font embedding (fixed-pitch flag bit 0), and glyph advance caching.
**Impact:** Any API that relies on monospaced hints (e.g., PDF font flags, canvas layout heuristics) sees all fonts as proportional.
**Effort:** Trivial (one-line parse).

### C-3: Paragraph layout uses character-index glyphs instead of shaped glyphs
**File:** `paragraph.rs` (lines 240-306)
**Severity:** Critical
**Description:** `Paragraph::layout()` does its own glyph mapping via `font.char_to_glyph(c)` per character and advances by a hardcoded `font.size() * 0.5`. It never calls the `Shaper` from `shaper.rs`, so kerning, ligatures, OpenType features, RTL reordering, mark-to-base positioning, Arabic shaping, Indic clusters, CJK advances, and emoji sequences are all ignored.
**Impact:** The rustybuzz integration in `shaper.rs` (which is genuinely functional — it calls `rustybuzz::shape`, returns real advances and positions) is unreachable from the top-level `ParagraphBuilder/Paragraph` API. The entire "rich text paragraph layout" module operates at an ASCII-typewriter level.
**Effort:** Medium (rewrite `Paragraph::layout` to call `Shaper::shape_auto` per run, then feed the shaped glyphs into `TextBlobBuilder` with real positions; ~120 lines).

### C-4: `Paragraph` ignores `TextStyle::font`, `TextStyle::color`, `TextStyle::background_color`, `TextStyle::decoration`
**File:** `paragraph.rs` (lines 224-321, 391-412)
**Severity:** Critical
**Description:** `ParagraphStyle`/`TextStyle` carry per-span font, color, background, and decoration, but `layout()` consults only `run.style.font` (for metrics) and `letter_spacing`/`word_spacing`. When `to_text_blob()` emits glyph runs it uses a single `line.font` and drops color, decoration, and background entirely. The `TextDecoration` struct (underline/overline/line-through with color/style/thickness) is pure data with no rendering hook.
**Impact:** Mixed-style rich-text rendering (e.g., bold + italic + underlined + colored spans) collapses to a single-font, foreground-only blob. This is the core feature the module exists to provide.
**Effort:** Medium (emit one glyph run per style change; wire color into a parallel "color per glyph" structure or produce multiple TextBlobs; ~100 lines).

### C-5: `Font::glyph_is_color` and `Font::glyph_image` are placeholders (no COLR/CPAL/CBDT/CBLC/sbix/SVG parsing)
**File:** `font.rs` (lines 471-508)
**Severity:** Critical
**Description:** `glyph_is_color` returns `glyph > 0x1000` as a crude guess ("assume high glyph IDs might be emoji"). `glyph_image` synthesizes a solid yellow-ish rectangle with an explicit "placeholder" comment. No actual color font table parsing is performed. `ttf_parser` exposes the `colr` submodule and `tables::cbdt`, `tables::cblc`, `tables::sbix`, `tables::svg` which would feed a real implementation.
**Impact:** Emoji and other color fonts (Noto Color Emoji, Segoe UI Emoji, Apple Color Emoji, twemoji) render as yellow squares. The `GlyphImage` data type is unreachable from any real font.
**Effort:** High (COLR v0 is straightforward layered glyphs with palette; COLR v1 is a full graph with gradients/transforms; CBDT/CBLC and sbix require PNG decode delegation to skia-rs-codec; SVG-in-OpenType requires parsing the embedded SVG and delegating to skia-rs-svg). Realistic scope: COLR v0 + CBDT/CBLC first, then v1 and SVG later. ~400-600 lines.

## Nice-to-Have Gaps

### N-1: `DefaultFontMgr::make_from_data` and `make_from_file` ignore their arguments
**File:** `font_mgr.rs` (lines 175-183)
**Severity:** Nice-to-have
**Description:** Both methods return `Typeface::default_typeface()` with an acknowledging "Placeholder" comment. `Typeface::from_data` already exists and parses real font data; calling it here is trivial. `make_from_file` additionally needs `std::fs::read` with error handling.
**Impact:** Users who think they are loading a real font via the `FontMgr` trait silently get the default typeface with no font data, which breaks everything downstream that depends on cmap/hmtx (i.e., every feature from Phase 5).
**Effort:** Trivial (~10 lines).

### N-2: `DefaultFontMgr::match_family_style_character` does not check character coverage
**File:** `font_mgr.rs` (lines 162-173)
**Severity:** Nice-to-have
**Description:** Ignores the `bcp47` and `character` parameters. Comment says "A real implementation would check if the character is in the font." The Phase 5 `Typeface::char_to_glyph` returns 0 for missing glyphs, which is the canonical coverage test. Looping over registered families and picking the first whose typeface returns nonzero is straightforward.
**Impact:** Emoji/CJK/script fallback is non-functional from the public `FontMgr` API; callers fall back to the default family regardless of character.
**Effort:** Small (~20 lines).

### N-3: `Font::get_widths` ignores font data and returns `size * 0.5 * scale_x` for every character
**File:** `font.rs` (lines 313-318)
**Severity:** Nice-to-have
**Description:** The function is documented as a "Simple approximation". Inconsistent with `measure_text` (C-1 Phase 5 fix), which correctly maps chars to glyphs and uses `glyph_advance`. Callers of `get_widths` (for character-level positioning in canvas text) get a uniform fake width.
**Impact:** Canvas text operations that use per-character widths (hit testing, caret positioning) are wrong.
**Effort:** Trivial (delegate to `chars().map(|c| self.glyph_advance(self.char_to_glyph(c)))`).

### N-4: `Font::get_bounds` uses the `get_widths` approximation, not real glyph bounds
**File:** `font.rs` (lines 320-336)
**Severity:** Nice-to-have
**Description:** Same pattern as N-3 — hardcoded `size * 0.5 * scale_x` per character. `ttf_parser::Face::glyph_bounding_box` returns the real per-glyph bbox and could feed a scaled `Rect`.
**Effort:** Small.

### N-5: `Font::glyph_intercepts` returns rectangular-bbox approximation, not real outline intercepts
**File:** `font.rs` (lines 531-561)
**Severity:** Nice-to-have
**Description:** Documented as "Placeholder — returns approximated intercepts" using bbox tests. Real implementation intersects the glyph outline with the horizontal band `[top, bottom]`. Used for underline gap rendering (to avoid drawing through descenders like g/y/p). Current placeholder produces a continuous underline with no gaps.
**Effort:** Medium (walk the `Path` contour, intersect with horizontal lines, return crossing x-values).

## Test Coverage Gaps

### T-1: No tests verify ttf-parser integration for real fonts
**Description:** The Phase 5 work that made `Typeface::from_data`, `Font::glyph_advance`, and `Font::glyph_path` correct has no integration test with an actual TTF file in the test suite. `test_char_to_glyph` (typeface.rs) uses the dataless default typeface and asserts `'A' -> 65` via the ASCII fallback — which exercises the fallback path, not the cmap path. Without a bundled test font, regressions in the ttf-parser wiring would not be caught.
**Effort:** Medium (include a tiny permissive-licensed TTF in test fixtures and add tests that assert specific non-ASCII glyph IDs, real advances, real glyph-path command counts).

### T-2: No tests for Shaper
**Description:** `shaper.rs` has two tests covering direction and script detection (pure Unicode-range tables). The actual shaping pipeline — `Shaper::shape`, `Shaper::shape_auto`, `create_face`, and conversion of `rustybuzz::glyph_infos`/`glyph_positions` into `ShapedGlyph` — is never exercised.
**Effort:** Medium (requires test font; verify glyph count, positions, and cluster mapping for a known string).

### T-3: Paragraph layout tests only check that `height() > 0` and that the paragraph builds
**Description:** `test_paragraph_layout` asserts the paragraph has nonzero height and laid-out state. No test verifies line count for a known width/text combination, word-wrap boundaries, `max_lines` truncation, alignment (left/right/center), or mixed-style runs.
**Effort:** Small.

### T-4: TextBlob bounds computation never tested for correctness
**Description:** `test_glyph_run_bounds` asserts `width > 0 && height > 0` — it doesn't verify that the bounds actually bracket all glyph positions. The `GlyphRun::bounds` code uses `font.size() * 0.5` as a fake per-glyph width estimate (same as C-3 pattern) and would pass a trivially-width test but fail at anything precise.
**Effort:** Small.

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

### Priority 2: Real FontMetrics (C-1, C-2)
Ten-minute fix that immediately improves line-height, baseline, and underline positioning across the entire crate. Do this first; it is the highest value-per-hour item.

### Priority 3: Wire `FontMgr::make_from_data/make_from_file` to `Typeface::from_data` (N-1)
One-line fix, unblocks public font-loading API.

### Priority 4: Color glyph / emoji support (C-5)
Scope as COLR v0 + CBDT/CBLC in Phase 6, defer COLR v1 and SVG-in-OpenType. ~400 lines, 3-4 days.

### Priority 5: Test suite improvements (T-1 through T-4)
Bundle a tiny OFL/SIL-licensed TTF (e.g., stripped-down Roboto or Noto Sans Tofu) in tests and drive the ttf-parser, shaper, and paragraph pipelines against it. Estimated effort: 1 day.
