# skia-rs-canvas Gap Analysis

**Date:** 2026-04-25
**Reviewer:** Claude (Opus 4.7)

## Summary

**Phase 5 status: COMPLETE — all 22 gaps resolved**

- Total gaps: 22
- Critical gaps resolved: 11/11 ✅
- Nice-to-have gaps resolved: 6/6 ✅
- Test coverage gaps resolved: 5/5 ✅

**Original baseline:**
- Total public functions reviewed: ~145 (`pub fn` across canvas.rs:45, clip.rs:36, simd.rs:7, surface.rs:45, picture.rs:31, raster.rs:28; constructors/accessors counted once)
- Total test functions: 39 (baseline, all passing)
  - canvas.rs: 0
  - lib.rs: 0
  - clip.rs: 7
  - simd.rs: 8
  - picture.rs: 3
  - raster.rs: 14
  - surface.rs: 7

**Current status (as of 0.2.4):**
- Total test functions: 103 in skia-rs-canvas (all passing)
- Estimated complexity: **Medium** (majority of critical gaps are wire-up problems, not missing algorithms)

## Files Reviewed
- [x] lib.rs (31 lines)
- [x] canvas.rs (546 lines)
- [x] surface.rs (1085 lines)
- [x] raster.rs (1859 lines)
- [x] clip.rs (660 lines)
- [x] picture.rs (606 lines)
- [x] simd.rs (639 lines)

## Critical Gaps

### C-1: `Canvas::clear()` is a no-op
**File:** `canvas.rs` (lines 201-204)
**Severity:** Critical
**Status:** ✅ RESOLVED (P5-1 - Canvas unified via Backing enum)
**Description:** `Canvas::clear()` accepts a color and does nothing. `RasterCanvas::clear()` (surface.rs:358-360) already delegates to `PixelBuffer::clear()` which is fully implemented and SIMD-friendly. The high-level `Canvas` type (the one every consumer of this crate actually uses by name - see benches/canvas_benchmarks.rs, Picture::playback) has no pixel backing store, so this is not a simple wire-up - it is an architectural gap: `Canvas` owns a matrix stack and clip stack but no raster target.
**Impact:** Any user of `Canvas` (benches, Picture playback via `DrawCommand::Clear`) silently produces no output. `Picture::playback` is completely broken because every draw command lands on the stub `Canvas`.
**Effort:** Large if Canvas must own a backing target; trivial if Canvas is refactored to carry a `Option<&mut PixelBuffer>` or to delegate to a trait that `RasterCanvas` implements. See Implementation Notes for architecture discussion.

### C-2: `Canvas::draw_color()` is a no-op
**File:** `canvas.rs` (lines 206-209)
**Severity:** Critical
**Status:** ✅ RESOLVED (P5-1)
**Description:** `RasterCanvas::draw_color()` (surface.rs:363-379) has a real implementation that creates a rasterizer and fills the whole device rect. `Canvas::draw_color()` just discards both arguments.
**Impact:** Identical to C-1 - any Canvas consumer gets silently dropped draws; `DrawCommand::DrawColor` playback is a no-op.
**Effort:** Blocked on C-1 architectural decision.

### C-3: `Canvas::draw_point()` is a no-op
**File:** `canvas.rs` (lines 211-214)
**Severity:** Critical
**Status:** ✅ RESOLVED (P5-1)
**Description:** Full implementation exists in `Rasterizer::draw_point()` (raster.rs:426-448) including anti-aliasing and clip coverage. `RasterCanvas::draw_point()` wires to it (surface.rs:382-390). `Canvas::draw_point()` is a stub.
**Impact:** Canvas-level point drawing is dead; Picture::DrawPoint playback silently loses draws.
**Effort:** Blocked on C-1.

### C-4: `Canvas::draw_points()` is a no-op and `PointMode` is inert
**File:** `canvas.rs` (lines 216-219, PointMode defined 537-546)
**Severity:** Critical
**Status:** ✅ RESOLVED (P5-3 - draw_points implemented with full PointMode dispatch)
**Description:** `draw_points()` accepts `PointMode::Points`/`Lines`/`Polygon` and does nothing. Neither `Rasterizer` nor `RasterCanvas` has a corresponding implementation (`pub fn draw_points` search in raster.rs and surface.rs returns zero matches). So this is not wire-up - the logic doesn't exist anywhere below Canvas.
**Impact:** `PointMode` is unreachable; no polyline primitive. Skia's `SkCanvas::drawPoints` is a frequently used primitive (star rendering, dot plots, drag gestures).
**Effort:** 1-2 hours (each mode maps to existing calls - `Points` loops `draw_point`, `Lines` iterates pairs to `draw_line`, `Polygon` iterates consecutive pairs).

### C-5: `Canvas::draw_line()` is a no-op
**File:** `canvas.rs` (lines 221-224)
**Severity:** Critical
**Status:** ✅ RESOLVED (P5-1)
**Description:** Full AA/aliased implementations exist in `Rasterizer::draw_line*` (raster.rs:451-582, Wu's and Bresenham's). `RasterCanvas::draw_line()` already wires through (surface.rs:393-401). Canvas is a stub.
**Impact:** Canvas lines dropped; playback dropped.
**Effort:** Blocked on C-1.

### C-6: `Canvas::draw_rect()` is a no-op
**File:** `canvas.rs` (lines 226-229)
**Severity:** Critical
**Status:** ✅ RESOLVED (P5-1)
**Description:** Full fill/stroke implementations in `Rasterizer::draw_rect` (raster.rs:702-711). `RasterCanvas::draw_rect` wires through (surface.rs:404-412). Canvas is a stub.
**Impact:** No Canvas-level rect drawing; playback broken.
**Effort:** Blocked on C-1.

### C-7: `Canvas::draw_oval()` / `draw_circle()` / `draw_arc()` / `draw_round_rect()` / `draw_path()` are no-ops
**File:** `canvas.rs` (lines 231-261)
**Severity:** Critical
**Status:** ✅ RESOLVED (P5-1)
**Description:** All five primitives have complete implementations in `RasterCanvas` (surface.rs:415-532) which in turn delegate to `Rasterizer` for oval/circle/path, and build bezier paths for round_rect and arc. Canvas is stubbed across the board.
**Impact:** Five of the most-used Skia primitives are unusable via `Canvas`.
**Effort:** Blocked on C-1; each is one-line wire-up once architecture is chosen.

### C-8: `Canvas::clip_rect()` `ClipOp::Difference` is silently ignored
**File:** `canvas.rs` (lines 188-190)
**Severity:** Critical
**Status:** ✅ RESOLVED (P5-2 - Canvas now uses ClipStack with Difference op support)
**Description:** The `Difference` branch is an empty TODO. Passing `ClipOp::Difference` leaves the clip unchanged, which is the opposite of what the caller asked for. `RasterCanvas::clip_rect` (surface.rs:346-355) does not even accept a `ClipOp` argument, so there is no wiring to draw on. This is a real algorithmic gap: difference clipping requires region-based storage (the current Canvas clip stack is a `Vec<Rect>`, which cannot represent the complement of a rect within another rect except as a region).
**Impact:** Difference clips are silently wrong. Consumer asks "clip everything outside this rect" and gets no change.
**Effort:** Medium. Two options:
1. Replace `Canvas::clip_stack: Vec<Rect>` with `Vec<Region>` and use `Region::op_rect(RegionOp::Difference)` (infrastructure exists in skia-rs-core).
2. Replace with `Vec<ClipStack>` from this crate's clip.rs (has `ClipState` and full Difference semantics via `intersect_region` — but note: even `ClipState::intersect_rect` in clip.rs:330-351 does not implement Difference either).

Note that `ClipState::intersect_rect` (clip.rs:330-351) also has no difference path - if difference is handled via `intersect_region` with a computed complement region, the core plumbing is there, but `ClipStack::clip_rect` on clip.rs:456-458 and the matching `intersect_rect` method both need a `ClipOp` parameter.

### C-9: `Canvas::clip_path()` collapses path to its axis-aligned bounding box
**File:** `canvas.rs` (lines 195-199)
**Severity:** Critical
**Status:** ✅ RESOLVED (P5-2 - Canvas now delegates to ClipStack::clip_path)
**Description:** `Canvas::clip_path` calls `self.clip_rect(&path.bounds(), ...)`. A triangle clip path produces a rectangular clip that is strictly larger than the triangle; a rounded-rect clip path produces a straight-edged clip. This is functionally incorrect for any non-rectangular path. `ClipStack::clip_path` in clip.rs:497-522 has a real implementation (AA coverage mask via supersampling, or non-AA path bounds) that Canvas should delegate to.
**Impact:** All non-rectangular clips in Canvas are wrong. Most serious for circular masks, shape reveals, and UI clipping (avatar circles, rounded cards).
**Effort:** Blocked on C-1 architectural decision. Once Canvas has a real clip stack reference, this becomes a one-line delegation.

### C-10: `Canvas::save_layer()` is equivalent to `save()`
**File:** `canvas.rs` (lines 108-112)
**Severity:** Critical
**Status:** ✅ RESOLVED (P5-4 - Offscreen layer allocation and alpha/blend composition on restore)
**Description:** `save_layer` accepts a `SaveLayerRec` with bounds, paint, and flags and ignores all of them - it just calls `save()`. Save-layer semantics in Skia allocate an offscreen buffer, accumulate drawings into it, and compose back with the supplied paint (for filters, blend modes, opacity). None of that happens here.
**Impact:** Every effect that relies on save_layer is broken: group opacity, layer blend modes, filters (blur/drop-shadow when those land), `INIT_WITH_PREVIOUS` read-back. Silent regression - no error indication.
**Effort:** Large (days). Requires:
1. Offscreen `PixelBuffer` per layer in the save stack
2. Redirect draw ops to the top layer's buffer
3. On restore, composite the layer with its paint's blend mode/alpha onto the buffer below
4. Optional bounds clipping during composition
5. Optional `INIT_WITH_PREVIOUS` seeding

### C-11: `Canvas::flush()` is a no-op with a TODO
**File:** `canvas.rs` (lines 411-414)
**Severity:** Critical (for GPU/deferred backends)
**Status:** ✅ RESOLVED (P5-5 - Documented as raster no-op; backend dispatch ready)
**Description:** `flush` has a TODO but no meaningful behaviour for any backend. For a pure software Canvas with no pending operations queue, flush is legitimately a no-op. However, the TODO implies unfinished intent, and GPU surfaces (`GpuContext::flush`/`GpuSurface::flush`, surface.rs:208/235) have the concept defined and will need Canvas::flush to route to them.
**Impact:** Low today (raster-only), but will become critical when GPU/deferred rendering is wired (Phase 6+). At minimum the TODO should be resolved either by (a) implementing a real flush, or (b) replacing the TODO with a doc comment stating raster-only no-op semantics.
**Effort:** 15 minutes (document as raster no-op) or days (full deferred/GPU plumbing).

## Nice-to-Have Gaps

### N-1: `Canvas::clip_rect()` drops the `do_anti_alias` argument
**File:** `canvas.rs` (lines 175-176)
**Severity:** Nice-to-have (API surface is correct, behavior differs from Skia)
**Status:** ✅ RESOLVED (P5-2 - Canvas clip_rect now respects do_anti_alias flag)
**Description:** The `do_anti_alias` parameter is accepted and explicitly discarded via `let _ = do_anti_alias;`. `ClipStack::clip_rect_aa` (clip.rs:461-489) has a full implementation. Once C-1 resolves the Canvas backing, this is a one-line fix: call the `_aa` variant when the flag is set.
**Impact:** AA clips fall back to aliased clips silently.
**Effort:** 15 minutes once C-1 is resolved.

### N-2: `RasterCanvas::clip_rect()` has no `ClipOp` argument
**File:** `surface.rs` (lines 345-355)
**Severity:** Nice-to-have (API asymmetry)
**Status:** ✅ RESOLVED (P5-1 - RasterCanvas deprecated, unified Canvas handles all clip ops)
**Description:** `RasterCanvas::clip_rect(rect: &Rect)` has no `ClipOp` or `anti_alias` parameter. The Canvas-level API takes both. Until these are added, even a correctly wired Canvas cannot pass through the full clip semantics via RasterCanvas.
**Impact:** API mismatch forces Canvas to reimplement clip op logic it could delegate.
**Effort:** 30 minutes - add `op: ClipOp, anti_alias: bool` and route to the ClipStack in the existing Rasterizer field. Note: RasterCanvas currently has a `Vec<Rect>` clip stack (surface.rs:246), not a ClipStack - so the RasterCanvas clip stack also needs an upgrade to support full clip semantics.

### N-3: `RasterCanvas` and `Canvas` duplicate the matrix/clip stack logic
**File:** `canvas.rs` (45-54, 98-172) and `surface.rs` (243-355)
**Severity:** Nice-to-have (architecture)
**Status:** ✅ RESOLVED (P5-1 - Canvas now unified, RasterCanvas = type alias for migration)
**Description:** Both types carry a `matrix_stack: Vec<Matrix>`, a clip stack, and a `save_count: usize`, and reimplement `save`/`restore`/`translate`/`scale`/`rotate`/`concat`/`set_matrix` identically. Any fix to `RasterCanvas` (e.g. upgrading to `ClipStack` per N-2) must be mirrored to `Canvas`. This is exactly the same orphan/duplicate pattern flagged in skia-rs-core GAP-C5 for Matrix. Extracting a common `CanvasState` struct or making one type wrap the other would eliminate the divergence risk.
**Impact:** Divergence over time; developer has to touch two files for any state change. Notable: `RasterCanvas::clip_rect` drops `ClipOp`, while `Canvas::clip_rect` takes it - already diverged.
**Effort:** 2-3 hours (mechanical refactor).

### N-4: `ClipStack::clip_rect_aa()` silently discards the region component of `RegionAndMask`
**File:** `clip.rs` (lines 485-487)
**Severity:** Nice-to-have (correctness edge case)
**Status:** ✅ RESOLVED (P5-7 - Region now intersected in RegionAndMask branch)
**Description:** When the current clip is `RegionAndMask(r, m)` and a rect-AA clip is applied, only the mask `m` is intersected with the new rect mask; the region `r` is ignored. So a subsequent point-contains-check that considers the region would be wrong. Similarly `r` is bound but unused - should trigger a clippy warning suppressed elsewhere.
**Impact:** Rare - only triggers after a specific sequence of region + AA-path + AA-rect clips. The region constraint effectively vanishes for further clip stacking.
**Effort:** 15 minutes - either also update the region (by converting the new AA rect to a region and intersecting) or prefix with `_r` to acknowledge and document the decision.

### N-5: `RasterCanvas::draw_vertices()` applies `colors[i]` incorrectly for strip/fan, and only once for Triangles
**File:** `surface.rs` (lines 748-793)
**Severity:** Nice-to-have (correctness)
**Status:** ✅ RESOLVED (P5-6 - Barycentric per-vertex color interpolation in draw_triangle)
**Description:** For `VertexMode::Triangles` (line 756), `colors.first().copied()` always uses vertex 0's color for every triangle regardless of which triangle is being drawn. For strip/fan (lines 774, 787) only `colors[i]` is sampled (using the current iteration index, which is the vertex index rather than a per-vertex interpolation). Skia's `drawVertices` supports per-vertex color interpolation inside each triangle via barycentric blending in `draw_triangle`. Here `draw_triangle` takes a single `Option<Color>` and fills with that one color (surface.rs:796-871).
**Impact:** Gouraud-style vertex shading produces flat-shaded triangles with one color each. `draw_vertices` becomes useful only for same-color meshes.
**Effort:** 4-6 hours (barycentric interpolation in `draw_triangle` plus signature extension to carry three colors).

### N-6: `RasterCanvas::draw_string()` / `draw_text_blob()` render placeholder rectangles
**File:** `surface.rs` (lines 875-915, 920-970)
**Severity:** Nice-to-have (feature-gated, documented as placeholder)
**Status:** ✅ RESOLVED (P5-10 - Real glyph outlines via ttf-parser + Font::glyph_path())
**Description:** Both methods draw solid rectangles at glyph positions rather than rasterizing glyph outlines. The comments acknowledge it: "placeholder", "A real implementation would use glyph outlines from the font". `skia-rs-text` does have a `Font` type but the glyph-outline-to-path or glyph-bitmap path is not wired through.
**Impact:** Text rendering displays filled boxes. Any test/user that checks pixel colors for text letters will see a solid block.
**Effort:** Days (requires font shaping, glyph outline extraction, path rasterization). Might be tracked under skia-rs-text gaps instead.

## Test Coverage Gaps

### T-1: `canvas.rs` has zero tests
**File:** `canvas.rs` (546 lines, 45 public functions)
**Severity:** Test gap
**Status:** ✅ RESOLVED (P5-12 - Matrix stack, clip, draw dispatch, and Picture round-trip tests added)
**Description:** Zero `#[test]` attributes in canvas.rs. Even the plain matrix/clip stack operations (save/restore, translate+draw position) have no unit test. The pass-through draw methods are currently stubs so there is nothing meaningful to test, but `save`, `restore`, `restore_to_count`, `save_count`, `translate`, `scale`, `rotate`, `skew`, `concat`, `set_matrix`, `reset_matrix`, `clip_rect` (Intersect branch), `quick_reject`, and `quick_reject_path` all should have coverage now.
**Effort:** 2-3 hours.

### T-2: `save_layer`, `SaveLayerFlags`, `SaveLayerRec` untested
**File:** `canvas.rs` (lines 19-40, 108-112)
**Severity:** Test gap
**Status:** ✅ RESOLVED (P5-12 - SaveLayerRec composition + alpha/blend tests added)
**Description:** No tests for the save-layer types. Even if the implementation is stubbed (C-10), the parameter plumbing and const definitions should have coverage. Once C-10 lands, layer compositing tests are essential.
**Effort:** 1 hour now; more after C-10.

### T-3: `ImageLattice`, `RSXform`, `FilterMode`, `PointMode`, `TextAlign` untested
**File:** `canvas.rs` (lines 434-546)
**Severity:** Test gap
**Status:** ✅ RESOLVED (P5-12 + inline - RSXform, FilterMode, PointMode tests added)
**Description:** Five supporting types with constructors and some compute logic (notably `RSXform::from_radians`, `RSXform::to_matrix`) have zero tests. `RSXform::to_matrix` concatenates three matrices in an order that is not visually obvious - easy to break without tests.
**Effort:** 1 hour.

### T-4: `draw_image_lattice`, `draw_atlas`, `draw_patch`, `draw_annotation` untested
**File:** `canvas.rs` (lines 309-359)
**Severity:** Test gap
**Status:** ✅ RESOLVED (P5-9 - All four implemented and tested)
**Description:** Four draw methods that are effectively placeholders (bodies are empty or `let _ = ...;`). Ideally they should be implemented (N-level) or at minimum ticketed with explicit test failures marking them as unimplemented.
**Effort:** N/A until implementations land.

### T-5: Picture playback round-trip untested for most command variants
**File:** `picture.rs` (lines 555-606)
**Severity:** Test gap
**Status:** ✅ RESOLVED (P5-12 - 8 pixel-level round-trip tests added for DrawCommand variants)
**Description:** Three tests exist (`test_picture_recorder`, `test_picture_playback`, `test_nested_pictures`), but `test_picture_playback` only checks that the matrix was modified. There are 20+ `DrawCommand` variants; only `Save`, `Restore`, `Translate`, `DrawRect` are exercised. Playback correctness for `ClipRect`, `ClipPath`, `Clear`, `DrawColor`, `DrawCircle`, `DrawPath`, `DrawArc`, etc. is unverified. Since Canvas draw methods are all stubs (C-1..C-7), playback of draw commands is untested and also broken. The recording side works; the playback side does not.
**Effort:** 3-4 hours once C-1..C-10 resolved.

## Implementation Notes

### Topic: `Canvas` vs `RasterCanvas` architectural split (root cause of C-1..C-7, C-9, C-10)
**Background:** There are two distinct canvas types in the crate:
1. **`Canvas`** (canvas.rs:43-415) - owns matrix stack, clip stack, save count, width, height. No pixel buffer. All draw methods are TODO stubs. This is the type re-exported at the crate root and used by `Picture::playback`, benchmarks, and the skia-rs-safe wrapper.
2. **`RasterCanvas`** (surface.rs:243-971) - owns matrix stack, clip stack, save count, AND a `&mut PixelBuffer`. All draw methods are fully implemented and delegate to `Rasterizer`.

So `Canvas` is a *partial* state container with no raster target, and `RasterCanvas` is the "real" canvas. They duplicate the state-management logic (save/restore/translate/etc.) and only `RasterCanvas` can actually render.

`Surface::canvas(&self) -> Canvas` (surface.rs:62-65) makes the problem worse - it returns a `Canvas` with no link to the surface's `PixelBuffer`, so the returned canvas cannot draw to the surface that produced it. `Surface::raster_canvas(&mut self) -> RasterCanvas<'_>` is the one that works.

**Resolution options:**
1. **Make `Canvas` abstract over a backing target.** Give Canvas an `enum Backing { Raster(PixelBuffer), Gpu(Box<dyn GpuSurface>), Picture(Vec<DrawCommand>), Null }`. Every draw method matches on `self.backing` and dispatches. RasterCanvas becomes `Canvas::raster(buffer)`. This unifies the API and lets Picture recording reuse the Canvas interface directly.
2. **Delete `Canvas` and make `RasterCanvas` the only type.** Rename `RasterCanvas` to `Canvas`. Update call sites. Accept that picture recording keeps its own separate `RecordingCanvas`.
3. **Keep both but make `Canvas` a thin wrapper over `RasterCanvas`.** Canvas holds `Option<&mut PixelBuffer>` and forwards draw methods when Some. Lifetimes become awkward.

Option 1 is cleanest for future backends (GPU, PDF, SVG). Option 2 is the smallest code change. The choice drives the complexity estimate for C-1..C-7.

### Topic: Picture playback depends on Canvas draw methods working
**Background:** `Picture::playback(&self, canvas: &mut Canvas)` takes the stub `Canvas` by design. Every `DrawCommand` variant's `execute()` (picture.rs:216-319) calls Canvas methods. All draw commands are currently no-ops at playback time. This means the entire picture-as-display-list feature is non-functional for any draw command. Recording works (`RecordingCanvas` appends to a vec); playback is silent no-op.

The fix for this is the same architectural decision that fixes C-1..C-7. Once Canvas can actually draw, playback works.

### Topic: Clip stack is a degenerate `Vec<Rect>` in Canvas
**Background:** `Canvas::clip_stack: Vec<Rect>` (canvas.rs:47) cannot represent:
- Non-rectangular clips (paths, regions)
- Difference-op clips
- Anti-aliased clip edges

The crate already provides a proper `ClipStack` (clip.rs:394-534) that `Rasterizer` uses (raster.rs:298). Canvas should be migrated to use `ClipStack` for consistency and to enable C-8, C-9, N-1.

### Topic: SIMD path has a documented shortcut for semi-transparent fill
**Background:** `fill_span_blend_sse41` (simd.rs:213-295) blends all channels "identically" using a single `_mm_set_epi16` constant that interleaves RGBA values - the comment acknowledges "The interleaved layout makes per-channel extraction complex". AVX2 variant (line 303-371) does proper per-channel extraction. SSE4.1 variant likely produces incorrect colour values for semi-transparent blends because the alpha multiplier is applied uniformly instead of per-channel. Tested via `test_fill_span_blend_scalar` only (scalar path), never via the SSE path (test_fill_span_solid_various_sizes runs through SIMD dispatch but only asserts `chunk[0] > 50 && chunk[0] < 200`, which is loose enough to pass incorrect blending). This is a latent correctness bug in the SIMD path - runs only when CPU has SSE4.2 but not AVX2, with len >= 4. This could be promoted to a Critical gap after pixel-exact regression testing but is currently marked as implementation detail given the loose test coverage.

## Recommendations

Prioritised by impact and dependency order:

1. **Resolve Canvas architecture (C-1..C-7, C-9, C-10).** Pick option 1 or 2 from Implementation Notes. This unblocks 9 critical gaps and enables Picture playback.
2. **Upgrade Canvas clip stack to `ClipStack` (C-8, N-1).** Requires a `ClipStack`-based clip representation; unblocks difference clipping, AA clipping, and path clipping.
3. **Implement `draw_points` (C-4).** Independent of the architectural work; can be done as soon as `draw_point` and `draw_line` are available at the Canvas layer.
4. **Document or implement `flush` (C-11).** If raster-only in the near term, replace the TODO with a doc note; otherwise wire to GpuSurface.
5. **Fix `draw_vertices` color handling (N-5).** Per-triangle colour indexing and barycentric interpolation.
6. **Fix `clip_rect_aa` region-mask branch (N-4).** One-liner.
7. **Unify `Canvas` and `RasterCanvas` state (N-3).** Architectural cleanup; optional if option 2 above is chosen.
8. **Add `ClipOp` to `RasterCanvas::clip_rect` (N-2).** Enables Canvas-level ClipOp without reimplementing.
9. **Expand test coverage (T-1..T-5).** 5-8 hours to reach parity with other crates.
10. **Verify SSE4.1 fill_span_blend correctness (Implementation Notes).** Add a test that validates semi-transparent blend on the SIMD path; promote to Critical if the test fails.
