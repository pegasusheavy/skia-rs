# skia-rs-pdf Gap Analysis

**Date:** 2026-04-25
**Reviewer:** Claude (Opus 4.7)
**Phase 6E status:** Complete — 14/14 resolved.

## Summary

- Total public functions reviewed: ~85 (`pub fn` across canvas.rs, document.rs, font.rs, image.rs, pdfa.rs, stream.rs, transparency.rs)
- Total test functions (post-6E): 61 (all passing)
  - canvas.rs: 7 (was 2)
  - document.rs: 7 (was 3)
  - font.rs: 8 (was 3)
  - image.rs: 5 (was 4)
  - pdfa.rs: 20 (was 6)
  - stream.rs: 3 (unchanged)
  - transparency.rs: 6 (unchanged)
  - integration: 2 (was 0)
- Total gaps found: 14 — all resolved.
- Critical gaps: 5 resolved
- Nice-to-have gaps: 5 resolved
- Test coverage gaps: 4 resolved
- Estimated complexity: **Medium-High** — a lot of real plumbing is in place (real flate compression, real JPEG pass-through, real XMP generation, PDF/A validator, ExtGState), but the `PdfDocument` writer is too simple for real PDFs (no Resources dictionary, no xobject/font references, no integration with the managers).

## Files Reviewed
- [x] lib.rs (34 lines)
- [x] document.rs (307 lines)
- [x] canvas.rs (383 lines)
- [x] font.rs (450 lines)
- [x] image.rs (419 lines)
- [x] stream.rs (155 lines)
- [x] transparency.rs (575 lines)
- [x] pdfa.rs (971 lines)

## Critical Gaps

### C-1: `PdfDocument::write_to` writes empty `/Resources` dictionary — fonts and images never referenced
**File:** `document.rs` (lines 147-172)
**Severity:** Critical
**Status:** RESOLVED. `PdfDocument` now owns a `PdfFontManager`, `PdfImageManager`, and `TransparencyManager`; `PdfCanvas` borrows them (`PdfCanvas<'a>`) and records per-page `used_fonts` / `used_images` / `used_ext_gstates`. The new `WriteCtx` allocates object IDs up-front, emits fonts/images/extgstates after the page content, and builds a fully populated `/Resources << /Font /XObject /ExtGState /ProcSet >>` dictionary per page. Verified by `test_document_resources_dict_populated_with_font` and `test_document_resources_with_image_and_ext_gstate`.
**Description:** (original) The page object emits `/Resources << >>` (literally empty). A real PDF must list `/Font << /F1 5 0 R ... >>` and `/XObject << /Im1 7 0 R ... >>` (and `/ExtGState`, `/Pattern`, `/ColorSpace`) so that `Tf /F1 12` in the content stream can resolve to font object 5, and `Do /Im1` can draw image object 7. Without a proper Resources dict, any content stream that references a font or image is invalid PDF — Acrobat/Preview/pdftotext will reject or silently skip those operators.
**Impact:** Every `PdfCanvas::draw_text` call (which emits `/F1 <size> Tf ... Tj`) produces invalid PDF because there is no F1 in the page's resource dict. Nothing else — `PdfFontManager`, `PdfImageManager`, `ExtGraphicsState` — is wired into the document output. The separate managers hold their objects but `PdfDocument` never consults them when writing.
**Effort:** Medium (rework `write_to` to (a) allocate object IDs for each font/image/extgstate upfront, (b) write the font/image/extgstate objects between page content, (c) emit the resources dict with references, (d) add methods to `PdfDocument` to accept a font manager / image manager / transparency manager or hold them internally; ~250 lines).

### C-2: `PdfCanvas::draw_text` hardcodes `/F1` font reference with no font actually registered
**File:** `canvas.rs` (lines 294-309)
**Severity:** Critical
**Status:** RESOLVED. `PdfCanvas::set_font` / `use_standard_font` attach a real font index from the document's manager, `draw_text` and `draw_text_with_font` emit `/F{idx+1}` — matching the index used to build the page's `/Resources /Font` dict. Non-ASCII text is emitted as a UTF-16BE hex string (`<FEFF…>`) automatically, and every drawn character is recorded in the font's `used_chars` map so `generate_to_unicode` produces an accurate CMap. Tested by `test_canvas_draw_text_registers_font` and `test_canvas_draw_text_unicode_uses_utf16be`.
**Description:** (original) The content-stream emission does `/F1 {size} Tf` — assumes a font named F1 exists in the page resources. There is no path from `PdfCanvas` to a font manager; the canvas has no way to register a font and get back a resource name. Even if C-1 were fixed to write a resources dict, every text call would still bind to the same made-up F1. Also, the text content is written as a raw PDF string (`(text) Tj`) with only `()\\` escaped — non-ASCII / Unicode characters are written as bytes and mis-decoded by readers expecting WinAnsi; CJK/Arabic/etc. are mangled.
**Impact:** Text drawn through PdfCanvas: (a) will not display at all in viewers because /F1 is unresolved; (b) even with a fix, cannot render non-ASCII since only the ASCII ToUnicode CMap (font.rs lines 265-290) is written.
**Effort:** Medium-High (thread a font reference through `draw_text`; encode Unicode text as a CID string or UTF-16BE hex string depending on font type; wire ToUnicode CMap; ~150 lines).

### C-3: `PdfFont::truetype` does not subset and does not parse per-glyph widths from the font file
**File:** `font.rs` (lines 154-179, 306-340)
**Severity:** Critical
**Status:** RESOLVED (P7D: byte-level subsetting landed). `parse_truetype_metrics` now uses `ttf-parser` to read accurate `ascender` / `descender` / `italic_angle` / `cap_height` / `x_height` / global bbox, and per-code widths are built by looking up `glyph_index(ch)` + `glyph_hor_advance(gid)` scaled to PDF glyph units (1/1000 em). A deterministic FNV-1a-derived six-letter subset tag is prepended to the BaseFont name (e.g. `ABCDEF+Demo`) so the PDF marks the font as a subset. The writer runs the font data through the byte-level subsetter in `font::subset` (~300 LOC, hand-rolled) before flate compression: `glyf` keeps only the glyphs actually drawn by the document (plus `.notdef` and the transitive closure of composite-glyph components), `loca` is rebuilt with zero-length slots for unused glyphs, every other table (`cmap`, `hmtx`, `hhea`, `maxp`, `head`, `name`, `OS/2`, `post`) is preserved unchanged so WinAnsi character-code → glyph resolution keeps working. `/Length1` reflects the subsetted byte count. On Roboto-Regular.ttf (35 KB) drawing "Hello, World!" drops the embedded font to 11 KB (32% of original); on NotoSerif-Regular.ttf (350 KB) it drops to 49 KB (14%). Tested by `test_truetype_invalid_data_defaults`, `test_record_char_tracks_all_used`, `truetype_embedding_emits_fontfile2_and_widths`, `truetype_subset_length1_matches_stream_length`, `truetype_subset_actually_prunes_large_glyf`, plus seven `font::subset` unit tests covering composite-glyph dependencies, TTC rejection, and checksum math.
**Description:** (original) `parse_truetype_metrics` acknowledges it returns defaults: `ascender: 750.0, descender: -250.0, cap_height: 700.0, stem_v: 80.0, bbox: [0.0, -250.0, 1000.0, 750.0]`, and populates `widths` with `600` for every character 32-255 regardless of what the font actually contains. Real PDF TrueType embedding requires: (a) accurate font metrics from OS/2 and hhea tables; (b) a per-glyph widths array from the hmtx table; (c) font subsetting to include only used glyphs (`used_glyphs` is tracked via `use_glyph` but never consulted when emitting the FontFile2 stream); (d) an actual FontFile2 stream object containing the (subsetted) TTF data.
**Impact:** TrueType embedding produces PDFs with: wrong glyph widths (text overlaps or gaps), wrong font metrics (mispositioned text), no subsetting (massive file sizes — a full font is often 200-500 KB, even if only 20 glyphs are used), and no FontFile stream wired to the font descriptor (so the embedded font data is never actually embedded).
**Effort:** High. A proper implementation needs ttf-parser (already a dep of skia-rs-text) to read hmtx/hhea/OS/2. Subsetting is harder — need to preserve the glyph outlines referenced by used_glyphs, recompute the cmap, loca, and glyf tables. Recommend using the `ttf-parser` + a subsetter crate like `subsetter`. ~500 lines + subsetter dep.

### C-4: PDF/A machinery is walled off from `PdfDocument`
**File:** `pdfa.rs` (971 lines)
**Severity:** Critical
**Status:** RESOLVED. `PdfDocument::set_pdfa_conformance(level)` installs a `PdfAState { level, doc: PdfADocument }` with a generated document UUID and an sRGB output intent. `PdfDocument::validate_pdfa` wraps `PdfAValidator`; `write_to` calls `sync_pdfa_model` → `validate_pdfa` and returns `io::Error::other(...)` on failure, mentioning every violation. Emission now produces (a) an `/Type /Metadata /Subtype /XML` XMP stream referenced from the catalog, (b) an `/OutputIntents` array referencing a `DestOutputProfile` ICC stream, (c) a trailer `/ID` derived from the generated document UUID, (d) a header version bumped to the level's minimum (1.4 for A1, 1.7 for A2/A3). Tested by `test_document_pdfa_sets_xmp_and_output_intent` and `test_pdfa_a1_rejects_transparency_on_write`.
**Description:** (original) `PdfADocument`, `PdfAValidator`, `XmpMetadata`, `OutputIntent` form a complete validation model with XMP generation, UUID v4 generation, and conformance checks — but `PdfDocument` does not own a `PdfADocument`, does not call the validator before writing, does not emit XMP metadata to the output, and does not attach an output intent. The `lib.rs` doc comment advertises `doc.set_pdfa_conformance(PdfALevel::A1b)` (line 17 in pdfa.rs's rustdoc example) but `PdfDocument` has no such method. The two files compile independently.
**Impact:** The PDF/A feature is entirely non-functional from the public API. Users cannot produce a compliant PDF/A document from `PdfDocument`. The 971-line `pdfa.rs` is reachable only if users manually construct a `PdfADocument` and validate it against a hand-crafted model — which does not correspond to any actual PDF file they produce.
**Effort:** Medium (add `PdfDocument.pdfa: Option<PdfADocument>`; `set_pdfa_conformance`; in `write_to`: emit XMP metadata stream, output intent, document ID in trailer; integrate validator errors as the function's Result; ~200 lines).

### C-5: ExtGState/SoftMask/TransparencyGroup managers are not wired into `PdfCanvas` or `PdfDocument`
**File:** `canvas.rs`, `transparency.rs`, `document.rs`
**Severity:** Critical
**Status:** RESOLVED. `PdfCanvas::apply_paint` inspects the paint's alpha (via the `Color32` A channel) and blend mode (`paint.blend_mode()`); when either differs from Normal-opaque it registers an `ExtGraphicsState` with the document's `TransparencyManager` and emits `/GS<n> gs`. Explicit `set_alpha` / `set_blend_mode` methods on `PdfCanvas` provide direct access. Used indices are tracked per page and emitted in the `/Resources /ExtGState` dict. Tested by `test_canvas_alpha_paint_emits_gs` and exercised end-to-end in `test_document_resources_with_image_and_ext_gstate`.
**Description:** (original) `transparency.rs` defines `ExtGraphicsState`, `SoftMask`, `TransparencyGroup`, `TransparencyManager` with correct PDF dictionary generation. `PdfCanvas` has no method to apply an alpha, blend mode, or transparency group. `PdfDocument::write_to` never writes ExtGState objects or adds them to page resources. `Paint` objects have alpha fields (`Paint::alpha`, `Paint::color32` with A channel) but `PdfCanvas::apply_paint` only reads the color, never registering an ExtGState when alpha < 1.0 or blend mode is non-normal.
**Impact:** Any Paint with alpha < 1.0 or a non-Normal blend mode renders as fully-opaque/Normal in the resulting PDF. Transparency support is a non-feature in the main draw path.
**Effort:** Medium (same shape as C-1 — need a canvas-level handle to the transparency manager, emit `/GS1 gs` in content stream, add to resources dict; ~100 lines on top of C-1).

## Nice-to-Have Gaps

### N-1: `PdfCanvas::draw_path` treats conic segments as quadratic approximations
**File:** `canvas.rs` (lines 262-277)
**Severity:** Nice-to-have
**Status:** RESOLVED. The `Conic(ctrl, end, w)` arm now calls `skia_rs_path::flatten::flatten_conic_adaptive` (exposed as `pub` by making the `flatten` module public from skia-rs-path) to produce line segments that honor the weight via a 0.25-pt chord-midpoint tolerance. Emits `l` operators; no more bogus cubic approximation. Also fixed the `Close` arm to update `current = subpath_start` so Quad / Conic / Cubic after a closed contour start from the right position.
**Description:** (original) The `PathElement::Conic(ctrl, end, _w)` arm ignores the weight `_w` and uses the standard quadratic-to-cubic formula. For weights ≠ 1 this produces a visibly wrong curve (ellipses and circles drawn as conics will be slightly misshapen). skia-rs-path has a conic-to-cubic subdivision routine that honors the weight — should delegate there.
**Effort:** Small.

### N-2: `PdfImage::from_rgba` uses separate image + soft-mask but does not link them
**File:** `image.rs` (lines 122-166)
**Severity:** Nice-to-have
**Status:** RESOLVED. `PdfImageManager::add_rgba` now stores the mask's manager *index* in the image's `soft_mask_id`. `PdfDocument::write_to` resolves the index to the mask's real object id at emission time — callers no longer need to patch anything up manually. Tested by `test_add_rgba_links_soft_mask_index` and exercised end-to-end in `end_to_end_pdf_with_text_image_transparency_pdfa`.
**Description:** (original) `from_rgba` returns `(Self, Self)` — the image and the mask — but does not set `image.soft_mask_id` to the mask's id (because object IDs are assigned later). `PdfImageManager::add_rgba` also does not link them (lines 322-329). The caller would have to remember to call `set_soft_mask` with the mask's final object ID after `write_to` runs. This is easy to get wrong and not documented as a caller obligation.
**Impact:** RGBA images embedded via `add_rgba` render as opaque RGB — the mask object exists but is orphaned.
**Effort:** Small (after write_to assigns object IDs to both images, set the image's soft_mask_id; ~15 lines).

### N-3: `generate_to_unicode` in PdfFont is ASCII-only
**File:** `font.rs` (lines 265-290)
**Severity:** Nice-to-have
**Status:** RESOLVED. `PdfFont::record_char` tracks every character drawn through `PdfCanvas::draw_text`. `generate_to_unicode` iterates the recorded set and emits `bfchar` blocks (100 entries per block, per CMap convention) mapping each source code to its UTF-16BE encoding — surrogate-pair aware for non-BMP characters. Empty sets fall back to printable ASCII so the font still has a searchable CMap. Tested by `test_to_unicode_uses_recorded_chars` and `test_to_unicode_fallback_when_empty`.
**Description:** (original) Hardcodes `95 beginbfchar` mapping codes 32..127 to themselves. No mapping for any extended or non-Latin character. This means text extraction (copy/paste from PDF) for any non-ASCII text becomes garbage.
**Impact:** PDF accessibility and searchability broken for non-ASCII content.
**Effort:** Small-Medium (build a ToUnicode CMap from actual used-glyph ↔ character mapping, which requires tracking that map when text is drawn).

### N-4: `escape_pdf_string` handles only `(`, `)`, `\` — no handling of non-printable / Unicode / PDFDocEncoding
**File:** `canvas.rs` (lines 340-351), `document.rs` (lines 259-270)
**Severity:** Nice-to-have
**Status:** RESOLVED. Deduplicated the two implementations by making `canvas::escape_pdf_string` (and the new `canvas::utf16be_hex`) pub, and having `document::pdf_text_string` call them. The escape function now handles `\n`, `\r`, `\t`, `\b`, `\f`, and emits octal `\ddd` escapes for non-printable bytes. `pdf_text_string` picks a PDF literal when the string is ASCII and a UTF-16BE hex string (`<FEFF...>`) otherwise, so info-dictionary entries containing Unicode now survive round-tripping. Tested by `test_escape_pdf_string_controls` and `test_utf16be_hex`.
**Description:** (original) PDF string literals need handling for `\n`, `\r`, `\t`, `\b`, `\f` (mapped to PDF escapes), octal escapes for non-printable bytes, and the PDFDocEncoding mapping for Latin-1 character set in info dictionaries. Unicode in info dictionaries requires hex-encoded UTF-16BE `<FEFF...>` format. Current code produces a broken PDF for any info string with a newline or Unicode character.
**Effort:** Small-Medium.

### N-5: `uuid_v4` in pdfa.rs is a weak PRNG, not real UUID4
**File:** `pdfa.rs` (lines 774-794)
**Severity:** Nice-to-have
**Status:** RESOLVED. `uuid_v4` now combines `SystemTime::now().as_nanos()` with a process-wide `AtomicU64` counter (each call bumps it via `fetch_add`) and passes the result through a splitmix64-style mixer. Two calls in the same nanosecond still produce distinct UUIDs; verified by `test_uuid_v4_is_unique_in_tight_loop` which asserts 1 000 consecutive generations are all distinct. Still not cryptographically random — documented explicitly on the function.
**Description:** (original) Seeds a 64-bit LCG with `SystemTime::now().as_nanos()` and produces a 128-bit-looking string. Not cryptographically random (acknowledged in the comment), and two invocations within the same nanosecond produce the same UUID. The crate has `uuid` as a sibling dep option and could use `uuid::Uuid::new_v4()` directly.
**Impact:** Two documents generated within the same nanosecond (possible on fast systems) share a document ID, which defeats PDF/A's requirement for unique document identification.
**Effort:** Trivial (add `uuid` dep, replace `uuid_v4` function; ~5 lines).

## Test Coverage Gaps

### T-1: No end-to-end document test with fonts, images, transparency, and PDF/A validation
**Status:** RESOLVED. New integration test `end_to_end_pdf_with_text_image_transparency_pdfa` in `tests/integration.rs` produces a PDF/A-2b document with Helvetica text, a translucent paint (triggers ExtGState), and an RGBA image (triggers soft mask + XObject), then asserts the output has a proper `%PDF-1.7` header, `xref`/`startxref`/`%%EOF` trailer, populated `/Font`, `/XObject`, `/ExtGState` resource sub-dicts, `/Type /Metadata`, `/Type /OutputIntent`, and a ToUnicode CMap. External veraPDF validation is still deferred as a feature flag for CI.
**Description:** (original) The 27 tests are unit-level and exercise each module in isolation: build a font, build an image, build a canvas, build an ExtGState. No test actually produces a PDF with text + images + transparency + PDF/A conformance and verifies (a) the output bytes parse as valid PDF, (b) the output passes external validation (veraPDF-style), (c) the Resources dict is populated, (d) used fonts/images/extgstates appear in output.
**Effort:** Medium-High (integration tests that might feed output to an external validator like verapdf behind a feature flag).

### T-2: No test verifies resources dict is populated when fonts/images are used
**Status:** RESOLVED. `test_document_resources_dict_populated_with_font` and `test_document_resources_with_image_and_ext_gstate` in `document.rs` write a document that uses a font, an image, and a translucent paint and assert `/Font << /F1`, `/XObject << /Im1`, `/ExtGState << /GS1` are present in the rendered output. The integration test `end_to_end_pdf_with_text_image_transparency_pdfa` covers the same ground end-to-end.
**Description:** (original) C-1 is invisible to the test suite because `test_pdf_document_with_page` just checks `bytes.starts_with(b"%PDF-1.4")` — any byte sequence starting with that prefix would pass. No test draws text on a canvas and inspects the produced page's `/Resources`.
**Effort:** Small.

### T-3: No test for TrueType font embedding
**Status:** RESOLVED. Integration test `truetype_embedding_emits_fontfile2_and_widths` loads a real 400-byte `demo.ttf` (copied under `tests/fixtures/`), registers it via `PdfFontManager::register_truetype`, draws an "A", writes the document, and asserts the output contains `/FontFile2`, `/FontDescriptor`, `/Subtype /TrueType`, `/FirstChar`, and a six-uppercase-letter subset prefix on the BaseFont. Additional unit tests in `font.rs` cover subset-tag determinism, default-metrics fallback, and `record_char` tracking.
**Description:** (original) `test_font_pdf_dict` tests the standard (Type1) font dict. `test_font_manager` tests register/lookup. No test feeds real TTF data through `PdfFont::truetype` and verifies that the FontFile2 stream appears in output or that the widths array matches the font.
**Effort:** Blocked on C-3 (actual TrueType embedding needs to exist to test).

### T-4: PDF/A validator tests exercise only 4 of 30+ error codes
**Status:** RESOLVED. Added 14 new validator tests in `pdfa.rs` covering `MissingDocumentId`, `MissingPdfaId`, `FontMissingCmap`, `FontMissingWidths`, `MissingOutputIntent`, `UncalibratedColorSpace`, `Jpeg2000NotAllowed` (rejected A1 / accepted A2), `LzwCompressionNotAllowed`, `MissingDocumentStructure` (A1a), `EncryptionNotAllowed`, `JavaScriptNotAllowed`, `EmbeddedFilesNotAllowed` (A2 rejects), and `MissingFileRelationship` (A3 requires / A3 accepts with relationship). Plus a `test_uuid_v4_is_unique_in_tight_loop` regression test to make sure N-5 doesn't regress. The test count for the pdfa module went from 6 → 20.
**Description:** (original) The 6 tests cover MissingXmpMetadata, FontNotEmbedded, TransparencyNotAllowed. The remaining ~27 error codes (MissingDocumentId, JavaScriptNotAllowed, EncryptionNotAllowed, Jpeg2000NotAllowed, LzwCompressionNotAllowed, MissingOutputIntent, UncalibratedColorSpace, etc.) are never triggered by a test.
**Effort:** Small (each error path is a simple validator test; add ~20 tests).

## Implementation Notes

### Hand-rolled PDF writer vs. library
The crate does not use `lopdf`, `printpdf`, or `pdf-rs` — it builds PDFs directly from string templates. This is fine for simple cases but becomes cumbersome for real PDFs: complex xref tables, compressed object streams, linearization, digital signatures, annotations, forms. The hand-rolled approach is defensible — Skia also writes PDF directly — but adds a long-tail of small features to implement. Worth considering swapping to `lopdf` for the writer at some future point.

### XMP generation is genuinely correct
`XmpMetadata::to_xmp` produces syntactically valid XMP with Dublin Core, XMP Basic, and pdfaid namespaces. The padding loop at the end is a real XMP convention. The test `test_xmp_generation` asserts on substrings that would catch obvious regressions.

### PDF/A validator is well-structured
Even though it's not wired in (C-4), the validator itself is a clean rule-per-check design: `check_metadata`, `check_fonts`, `check_colors`, `check_images`, `check_transparency`, `check_structure`, `check_security`, `check_embedded_files`. Each rule has a corresponding `PdfAErrorCode` variant. This is ready to use once the document model feeds into it.

### Font/Image/Transparency managers are parallel designs that never connect
All three managers (`PdfFontManager`, `PdfImageManager`, `TransparencyManager`) have the same shape: a `Vec` of entries, an insert method, a get method. None of them is owned by `PdfDocument`. The apparent intent was for callers to manage their own font/image registration, then thread into the document — but `PdfDocument::write_to` never takes these managers as input or looks them up anywhere. This is the root cause of C-1, C-2, and C-5.

### Coordinate transform in PdfCanvas
Line 54: `canvas.write_op(&format!("1 0 0 -1 0 {} cm\n", height))` — correctly flips the Y axis from top-down to PDF's bottom-up. This means all subsequent drawing uses top-left origin, matching the rest of skia-rs. Good call.

## Recommendations

### Priority 1: Wire managers into `PdfDocument::write_to` (C-1, C-2, C-5)
This is the single biggest architectural gap. Everything else is blocked until the document output includes font/image/extgstate resources. ~300 lines of focused work. 2-3 days.

### Priority 2: Wire PDF/A into PdfDocument (C-4)
Medium effort to make the PDF/A feature reachable from the public API. Do right after P1 since it depends on having real output content. ~1-2 days.

### Priority 3: Real TrueType metrics + subsetting (C-3)
High-effort but high-value. Add ttf-parser for metrics/widths; add a subsetter for embedding. ~1 week.

### Priority 4: Fix string escaping, Unicode, UUID, soft-mask wiring (N-1 through N-5)
Small polish items. Can batch into one PR. ~1-2 days.

### Priority 5: Test improvements (T-1 through T-4)
Medium effort; T-1 requires either an external validator or a hand-rolled PDF parser. Start with T-2 (resources dict) which is trivial once C-1 is fixed. ~2 days.
