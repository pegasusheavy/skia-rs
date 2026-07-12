# Conformance audit fixes — skia-rs vs upstream C++ Skia

A nine-agent audit (2026-07-09) compared every crate against the upstream Skia
checkout vendored at `skia/` and produced ~125 verified findings. This plan
fixes all of them, one task per crate, dependency order.

## Global Constraints

- **Upstream Skia is ground truth.** The full C++ checkout lives at
  `/home/joseph/Projects/skia-rs/skia/`. When a finding cites an upstream file
  (e.g. `src/core/SkMatrix.cpp`), open it and match the exact semantics —
  formulas, constants, sign conventions, defaults, rounding.
- **Every fix gets a regression test** asserting the Skia-conformant behavior,
  derived from the finding's failure scenario. If an existing test codifies the
  deviant behavior, rewrite that test to assert the Skia behavior — do not
  weaken the fix to satisfy the old test.
- **Rounding convention:** byte-domain color math uses Skia's rounded
  `SkMulDiv255Round` semantics (`(a*b + 128) * 257 >> 16`, i.e. round to
  nearest), never truncation.
- **Premultiplied alpha convention:** the pixel pipeline operates on
  premultiplied color; shaders/gradients hand premul to blend code; blend
  formulas take premul inputs (per `skia/src/opts/SkRasterPipeline_opts.h`).
  Fix mismatches by conforming to this convention, not by re-documenting the
  deviation.
- **Behavior breaks are intended.** Paint defaults, clip-op values, N32
  selection etc. change to match Skia even though they alter existing output.
  Update CHANGELOG.md under an "Unreleased" heading with one line per
  user-visible behavior change in your crate.
- Work crate-local: do not edit other crates except where a finding explicitly
  spans crates (each task lists any such exceptions).
- Run the crate's test suite plus `cargo build --workspace` before committing.
  Fix all new compiler warnings your change introduces.
- TDD: write the failing regression test first for each finding, then fix.
  Findings marked "(large)" are implementation work; for those, test-first per
  sub-behavior.

## Task 1: skia-rs-core — geometry, matrices, color, pixel, region

Fix every finding below in `crates/skia-rs-core/src/`.

### Major

- geometry.rs:716 `Rect::is_empty` — NaN handling inverted: NaN rect reports
  non-empty and survives `union()`/`join()`. Rewrite as Skia does:
  `!(left < right && top < bottom)` so any NaN ⇒ empty. Upstream:
  `include/core/SkRect.h` `SkRect::isEmpty()`.
- geometry.rs:1234-1241 `Matrix::invert` — singularity threshold ~2×10⁶ too
  large (`f32::EPSILON*256` absolute). `Matrix::scale(0.005,0.005)` fails to
  invert. Match upstream `sk_inv_determinant` (`src/core/SkMatrix.cpp`):
  compute det in f64, reject only `|det| < (1/4096)³ ≈ 1.45e-11` (the constant
  is SK_ScalarNearlyZero **cubed**), and additionally reject a computed inverse
  with non-finite elements (Skia checks `inv.isFinite()`).
- matrix44.rs:455-461 `Matrix44::invert` — same wrong absolute threshold.
  Match `SkInvert4x4Matrix` (`src/core/SkMatrixInvert.cpp`): double-precision
  determinant, fail only on det == 0 or non-finite result.
- matrix44.rs:375-392 `pre_translate`/`post_translate`/`pre_scale`/`post_scale`
  — pre/post semantics swapped vs `SkM44` (`src/core/SkM44.cpp`):
  `preTranslate` must compute `self * T` (updates last column),
  `postTranslate` computes `T * self`; same for scale. Fix all four and audit
  in-crate callers for compensating swaps.
- geometry.rs:907-916 `RRect::from_rect_xy` (and `from_oval`) — three defects
  vs `SkRRect::setRectXY` (`src/core/SkRRect.cpp`): (a) panics on inverted
  rects (`clamp` with max<min) — must sort/empty-handle like `initializeRect`;
  (b) oversized radii must be scaled by the single factor
  `min(w/(2·xRad), h/(2·yRad))` preserving aspect, not clamped per-axis;
  (c) if **either** radius ≤ 0, both become 0 (square-corner rect).
- pixel.rs:704-714 `convert_row` RGB565→RGBA8888 — missing low-bit
  replication: use `(r << 3) | (r >> 2)` / `(g << 2) | (g >> 4)` per
  `SkR16ToR32`/`SkG16ToG32` (`src/core/SkColorData.h`) so 31→255, not 248.
- pixel.rs:932-952 + 1164 `apply_alpha_conversion` fallback — the generic
  premul/unpremul-in-place assumes 4-byte RGBA but runs on `Argb4444`,
  `Rgba1010102`, `Bgra1010102`, `R16G16B16A16Unorm`, corrupting pixels.
  Implement per-format alpha conversion (unpack → convert with rounding →
  repack) for those formats, or route them through an unpacked intermediate;
  same-type copies must produce correct results for every color type with
  alpha.
- color.rs:1069-1080 `ColorType::n32` — selects by target endianness; Skia
  selects by platform build config: RGBA everywhere except Windows (BGRA).
  Upstream `include/core/SkTypes.h` `SK_R32_SHIFT`. Make `n32()` return
  `Rgba8888` on all targets except Windows (`cfg(target_os = "windows")` ⇒
  `Bgra8888`).

### Minor

- geometry.rs:743-748 `Rect::contains_rect` — must return false if either rect
  is empty (upstream `SkRect::contains(const SkRect&)`).
- geometry.rs:846-855 `Rect::round`, geometry.rs:422-427
  `Size::to_isize_round` — use Skia rounding `floor(x + 0.5)` (half toward
  +∞), and saturating float→int casts (NaN and out-of-range must saturate like
  `sk_float_saturate2int`, not become 0). Apply to `round`, `round_out`,
  `round_in`.
- geometry.rs:474-481 `IRect::from_xywh` — use saturating adds
  (`i32::saturating_add`) like `Sk32_sat_add` in `SkIRect::MakeXYWH`.
- color.rs:700-714 `premultiply_color`, pixel.rs:1164-1178
  `premultiply_in_place` — must round (`SkMulDiv255Round`), not truncate:
  r=3,a=128 ⇒ 2 not 1. Keep `unpremultiply` consistent (already rounded).
- pixel.rs:744-753 `convert_row` RGBA8888→Gray8 — switch luma coefficients to
  BT.709 (0.2126/0.7152/0.0722) per Skia's
  `bt709_luminance_or_luma_to_alpha` stage.
- color.rs:1053-1065 `ColorType::has_alpha` — `R16G16Unorm` and `R16G16Float`
  are alpha-less RG formats (upstream `SkImageInfoPriv.h`); return false.
- region.rs:281-383 — canonicalize after `intersect` and `difference` (not
  just `union`) so `PartialEq`, `is_rect()`, `rect_count()` and iteration
  order match `SkRegion` semantics; iteration must be strict top-to-bottom
  scanline order after all ops.
- pixel.rs:71-86 `ImageInfo::new` — accept zero dimensions as a legal empty
  info (Skia `SkImageInfo::Make(0,0,…)` sentinel); keep rejecting negatives.

### Cross-crate note

Changing `is_empty`, rounding, `n32`, and premul rounding may break tests in
dependent crates; fix those tests to the Skia-conformant expectations as part
of this task (they are covered by "update existing tests" in Global
Constraints). Do not otherwise modify dependent-crate source.

## Task 2: skia-rs-path — path model, stroking, measure, ops, SVG parser

Fix every finding below in `crates/skia-rs-path/src/`.

### Critical

- path.rs:567-635 `contains()` — three defects vs `SkPathPriv::Contains`
  (`src/core/SkPath.cpp` / SkPathPriv): (a) must accumulate **signed** winding
  (+1/−1 per crossing direction), with `Winding` fill testing `w != 0` and
  `EvenOdd` testing parity; (b) inverse fill types: point outside bounds or
  empty path returns `is_inverse_fill_type()`, and final containment is XORed
  with inverse; (c) every contour is **implicitly closed** for hit-testing —
  synthesize the closing edge from last point to contour start like
  `SkPathEdgeIter`. Also use inclusive bounds comparison for the early-out.
- path.rs:487-526 `reverse()` — produces invalid verb streams (`Move` at end,
  spurious/doubled `Close`). Reimplement per
  `SkPathPriv::ReverseAddPath`: contour-by-contour, `Move` first, points in
  reverse order with per-verb point-count handling, `Close` preserved
  per-contour.
- path_utils.rs:149-152, 171-337 — closed-contour stroking never strokes the
  closing segment and adds no join at the start/end vertex. On `Close`, add
  the edge from last point back to contour start, join it to the first edge,
  and close both outer and inner contours (upstream `SkStroke::strokePath`,
  `SkPathStroker::close`).
- path_utils.rs:340-355 — the inner offset contour of a closed stroke must be
  emitted in **reversed** direction so winding cancels and the stroke renders
  as a frame, not a filled slab (upstream appends `fInner` via
  `reversePathTo`). A stroked rect filled with Winding must produce a
  width-2 frame with an empty middle — regression-test exactly that via
  `contains()`.

### Major

- path.rs:460-484 `direction()` — sign convention inverted for y-down
  (canonical `add_rect` CW output reports CCW). Match
  `SkPathPriv::ComputeFirstDirection`: use the dominant-contour
  extreme-point cross test with `cross > 0 ⇒ CW` in y-down device space.
- builder.rs:383-387 `ensure_move()` — after `close()`, a `line_to`/curve must
  inject `moveTo(last_move_point)` starting a new contour (upstream
  `SkPathBuilder::ensureMove`).
- builder.rs:444-510 SVG arc — x-axis-rotation is used for the center but
  dropped when emitting the curve: points are generated on an axis-aligned
  ellipse. Rotate the emitted geometry by φ about the center (or map unit-arc
  segments through a matrix with `preRotate(φ)` like `SkPathBuilder::arcTo`'s
  SVG overload) so the endpoint lands exactly on (x,y) and intermediate
  geometry matches SVG 2 §B.2.3.
- path.rs:296-344 `is_rect()` — must accept the crate's own
  `add_rect` output (Move+3 Lines+Close) and must reject non-rectangular
  H/V staircases (validate 4 direction changes + closure like
  `SkPathPriv::IsRectContour`).
- path.rs:413-451 `convexity()` — must be per-contour and verb-aware: any
  second contour ⇒ Concave; use cross-product sign changes along one contour
  including the closing edges; drop the fixed 0.001 threshold in favor of
  sign tests (upstream `SkPathPriv::ComputeConvexity`).
- effects.rs:276-284 `DashEffect` — the closing segment of a closed contour
  must be dashed continuously (dash distance advances across it), not drawn
  solid/dropped (upstream `SkDashPath::InternalFilter` measures the full
  contour including close).
- path_utils.rs:135-147, 438-465 — conic-to-quad control point is wrong: a
  single-quad approximation of conic (s, c, e, w) must interpolate the conic
  midpoint (control = ((1+w)c + (s+e)/2·(1−w)) … match
  `SkConic::chopIntoQuadsPOW2` in `src/core/SkGeometry.cpp` — subdivide POW2
  style rather than patching the formula). Replace fixed-step flattening
  (4/quad, 8/cubic) with error-driven subdivision.
- builder.rs:413-441 `add_arc_segment` — `sweep == 0` produces 0/0 ⇒ NaN
  control points. Early-return on zero sweep like `SkPathBuilder::addArc`.
- ops.rs:61-65 + 92-97 — `simplify()` must always run the ops machinery (no
  `Union`-with-empty short-circuit) so self-intersections/overlaps resolve.

### Minor

- builder.rs:212-235 `add_arc` |sweep| ≥ 360 — direction from sweep sign;
  oval shortcut only when startAngle is a multiple of 90°, else start point
  honors startAngle (upstream SkPathBuilder::addArc).
- builder.rs:125-142 `add_oval` — use 4 conics of weight √2/2 (upstream
  `gFourQuarterCircleConics`) instead of cubic KAPPA approximation.
- builder.rs:108-113 — repeated `close()` is a no-op; consecutive `move_to`s
  overwrite the pending Move point instead of appending verbs.
- measure.rs:64-97, 122-127 — `get_point_at`/`get_tangent_at` pin distance
  into [0, length] instead of returning None; `get_segment` pins start/stop
  like `SkContourMeasure::getSegment`.
- svg.rs:323-352, 379-406 — smooth `S`/`T` reflection must be gated on the
  previous command kind (`S` reflects only after C/S; `T` only after Q/T;
  otherwise control = current point) per `SkParsePath.cpp`.
- svg.rs:113-118 — numeric data directly after `Z` is a parse error (reject
  the path like upstream), not an implicit lineto.
- builder.rs:378 (used by svg.rs:181) — after `close()` the current point must
  return to the subpath's initial point so subsequent relative/absolute
  commands start from there (upstream `case 'Z': c = first`).
- path.rs:219-241 `bounds()` — non-finite coordinates ⇒ empty bounds (track
  finiteness like `SkPath::isFinite`).
- path_utils.rs:160-166 — zero-length contours with Round/Square caps must
  emit a cap-shaped dot (upstream zero-length-segment handling in SkStroke).
- effects.rs:376-418 `CornerEffect` — round the joint between last and first
  segments of a closed contour (start output at the first segment's midpoint
  like `SkCornerPathEffect`).

## Task 3: skia-rs-paint — blend modes, paint defaults, shaders, filters, SkSL

Fix every finding below in `crates/skia-rs-paint/src/`.

### blend.rs

- blend.rs:255, 302-314 (critical) — soft-light: dark-dst polynomial must be
  `16m³ − 12m² + 3m` (as `(m4*m4 + m4)*(m − 1) + 7m` form upstream) and all
  branches must add the Porter-Duff edge terms `s·(1−da) + d·(1−sa)`. Match
  `BLEND_MODE(softlight)` in `skia/src/opts/SkRasterPipeline_opts.h`.
- blend.rs:213-219 Overlay, 248-254 HardLight — add the missing
  `s·(1−da) + d·(1−sa)` terms outside the branch (upstream
  `BLEND_MODE(hardlight/overlay)`).
- blend.rs:226-237 ColorDodge, 238-247 ColorBurn — general branch drops the
  `da` normalization: dodge is `sa·min(da, d·sa/(sa−s))` + edge terms; burn
  analogous. Match `BLEND_MODE(colorburn/colordodge)`.

### paint.rs

- paint.rs:92 — default `stroke_width` must be `0.0` (hairline), per
  `SkPaint.cpp` `fWidth{0}`.
- paint.rs:96 — default `anti_alias` must be `false`, per SkPaint defaults.
- paint.rs:117-124 (and 388-391) — float→byte color conversion must round,
  not truncate (0.5 ⇒ 128).

### shader.rs

- shader.rs:133-147 `interpolate_gradient_color` — `t` below the first
  explicit position must return the **first** color (implicit stop at 0), and
  positions must be pinned monotonic into [0,1] like
  `SkGradientBaseShader.cpp` (`fFirstStopIsImplicit`, `SkTPin`).
- shader.rs:1029-1040 TwoPointConical — when both roots are valid pick the
  **larger** t (upstream well-behaved/greater case uses `+sqrt`); the smaller
  root only for the swapped/negative-focal case.
- shader.rs:1385-1389 BlendShader / 1832-1836 ComposeShader — children's
  straight-alpha sample outputs must be premultiplied before
  `BlendMode::apply` (which requires premul), then the result handled
  consistently with the sample() contract. Pick one convention for
  `Shader::sample` (premul recommended, matching upstream pipeline colors),
  apply it to ColorShader/gradients/ImageShader uniformly, and fix callers.
- shader.rs:1051 — conical gradient must honor
  `GradientFlags::INTERPOLATE_PREMUL` like linear/radial/sweep.
- shader.rs:277-280 — `Alpha8` sampling: alpha-only sources are
  `(0,0,0,a)` (black with alpha), not white (upstream colorizes by paint
  color at draw time, base decode is black).
- shader.rs:1246-1271 `ImageShader::sample` — honor `SamplingOptions`:
  implement bilinear filtering for `FilterMode::Linear`.

### filter.rs

- filter.rs:278-292 `BlurMaskFilter::apply_mask`, 349-366
  `BlurImageFilter::apply` — convert sigma to box radius per
  `SkBlurMask::ConvertSigmaToRadius` (`radius = (sigma − 0.5)/0.57735` →
  three-box approximation; sigma 0.9 must blur). Do not truncate sigma.
- filter.rs:278-292 — implement `BlurStyle` Solid/Outer/Inner per
  `SkBlurMask` (Solid = src ∪ blur, Outer = blur − src, Inner = blur ∩ src).
- filter.rs:817-842 `ColorFilterImageFilter::apply` — unpremultiply, apply the
  color matrix in unpremul space (translate 0..1), clamp, re-premultiply
  (upstream `SkColorFilters::Matrix` default unpremul + clamp).
- filter.rs:1417-1441 MatrixConvolution — when `convolve_alpha == false`:
  unpremul, convolve RGB, re-premul with original alpha (upstream
  `SkKnownRuntimeEffects.cpp`). Honor `tile_mode` instead of always clamping.
- filter.rs:1495-1522 TileImageFilter — fill only `dst_rect`, not the whole
  buffer.

### runtime_effect.rs

- runtime_effect.rs:248-265 — uniforms pack tightly (`offset += size`,
  4-byte granularity), no 16-byte alignment (upstream
  `SkRuntimeEffect.cpp`).
- runtime_effect.rs:248-287 — child declarations (`uniform shader s;`) are
  children, not uniforms: exclude from `uniforms()` and uniform size/offsets.

### sksl.rs / sksl_interp.rs

- sksl.rs:1727-1741 (large) — parse method-style calls on arbitrary postfix
  expressions so `child.eval(coord)` works, and wire evaluation to the
  `Interp` children (sample the child shader at the given coords).
- sksl_interp.rs:543-565 — implement multi-component swizzle assignment
  (`c.rgb *= 0.5` must work for any swizzle lvalue).
- sksl_interp.rs:624-697 — implement matrix±matrix, matrix·scalar,
  scalar·matrix, vector·matrix, matrix·vector element/linear-algebra ops
  instead of falling through to scalar 0.
- sksl_interp.rs:736-748 — float division/mod by zero follow IEEE (±inf/NaN),
  not 0.
- sksl.rs:1615-1654 — wire `&`, `|`, `^`, `<<`, `>>` into the precedence
  chain (GLSL precedence: shifts above relational; & ^ | between equality and
  &&) and add `half(x)` to constructor tokens.

## Task 4: skia-rs-canvas — canvas state, clips, rasterizer, SIMD, pictures

Fix every finding below in `crates/skia-rs-canvas/src/`. This task also
converts the pixel pipeline to premultiplied storage (see "Premul conversion"
at the end — do it first; several findings depend on it).

### Critical

- canvas.rs:394-398 — `restore_to_count(count < 1)` infinite-loops; clamp
  count to 1 like `SkCanvas::restoreToCount`.
- canvas.rs:236-245 — `save()` (and `save_layer()` at 260-299) must return the
  save count **before** the save (`getSaveCount() - 1` semantics; initial
  count 1). Audit in-crate callers/tests for the idiom.
- clip.rs:606-611 — non-AA `clip_path` must scan-convert the actual path into
  the region (reuse the rasterizer's edge machinery to build spans →
  region rects), not the path bounds.
- clip.rs:556-584 — AA `ClipOp::Difference` on a rect must subtract only the
  rect: keep the old clip, zero coverage inside the rect. No pre-intersect.
- clip.rs:615-688 — `clip_path_with_op` Difference: rasterize the actual path
  coverage and subtract it from the existing clip in all state combinations
  (Rect, Region, Mask, RegionAndMask, AA and non-AA). No silent no-ops.
- simd.rs:379-433 `fill_span_blend_neon` — loads 64 B / stores 32 B while
  advancing 16 B: out-of-bounds reads/writes + double blending. Make the loop
  length-exact (process 16 px per iteration with vld4q/vst4q consistently, or
  8 px with vld4/vst4) and add a scalar tail. Add a test comparing NEON vs
  scalar output over odd span lengths.
- canvas.rs:468-481, 484-500 + picture.rs:261-266 — record the `ClipOp` (and
  replay it) in `DrawCommand::ClipRect/ClipPath`.
- raster.rs:665-693 `fill_rect` — under a non-axis-aligned CTM, convert the
  rect to a quad/path and scan-convert; only the axis-aligned fast path may
  use `map_rect`. Same for the clip path in canvas.rs:468-473.

### Major

- raster.rs:259-270 `blend_colors` — implement all missing modes (Modulate,
  Overlay, Darken, Lighten, ColorDodge, ColorBurn, HardLight, SoftLight,
  Difference, Exclusion, Hue, Saturation, Color, Luminosity) by delegating to
  `skia-rs-paint`'s fixed `BlendMode::apply` (Task 3) on premul values —
  no second implementation.
- raster.rs:217-258 — premul the Porter-Duff math (SrcATop = `s·da + d·(1−sa)`
  on premul components, etc.). Falls out of the premul conversion below.
- simd.rs:187-207, 303-373 — span SrcOver must operate on premultiplied src
  (premultiply the solid color once per span). Fix the codified-wrong test at
  simd.rs:643-681. SIMD and scalar and `blend_colors` must agree bit-exactly;
  add a differential test.
- raster.rs:458-510, 696-706, 927-1028 — stroke geometry must honor
  `stroke_width`: build the stroke outline via `skia-rs-path`'s fixed
  `stroke_to_fill` (Task 2) and fill it; width 0 remains hairline.
  `draw_points` Points mode draws width-sized squares (butt/square cap) or
  circles (round cap) per `SkDraw::drawPoints`.
- raster.rs:721-756, 803-816 — circles must map through the full CTM: convert
  to a path (conics) and fill/stroke via the path pipeline when the matrix is
  not a translate+uniform-scale; ellipse under non-uniform scale.
- raster.rs:1038-1097 + 676-684 — `fill_path` must honor `paint.shader()`;
  shader sampling must be in **local** space (map device point through the
  inverse CTM) and go through the clipped blitter. `fill_rect`'s shader
  branch must respect the clip.
- raster.rs:914-924 + 1129-1169 — honor `paint.is_anti_alias()`: route AA
  fills through `fill_path_aa`; fix `fill_path_aa` to remove edges past
  y_max and to apply the clip.
- raster.rs:1387-1423 — implement `InverseWinding`/`InverseEvenOdd`: fill the
  complement of the path within the clip.
- canvas.rs:561-571 `clear()` — clear only within the device clip
  (drawColor(color, Src) semantics).
- canvas.rs:574-592 `draw_color` — fill the device clip regardless of CTM;
  add `draw_paint` doing the same with full paint (shader) support.
- canvas.rs:280-296, 525-538 — `save_layer` bounds are a **content hint in
  local space**: map through the CTM to device bounds, intersect with clip,
  size the layer from that; keep a device-origin offset applied to both
  matrix and clip queries inside the layer.
- canvas.rs:335-391 `composite_layer` — composite through the current clip
  and apply the layer paint's color filter (alpha, blend mode, color filter
  at minimum; image filter if present).
- canvas.rs:1496-1507 `draw_image_rect` — apply paint alpha once, not twice.
- canvas.rs:1444-1512 `draw_image_rect` — route image draws through the
  clipped, matrix-aware pipeline: treat the image as a shader-filled quad so
  rotation and path/AA/difference clips work.
- canvas.rs:1726-1868 `draw_vertices`/`draw_triangle` — apply the clip per
  pixel; use one coverage rule (pixel centers) for both flat and interpolated
  paths; apply paint alpha to vertex colors.
- canvas.rs:2086-2091 `RSXform::to_matrix` — tx/ty go in the translation
  slots applied **last** (`SkMatrix::setRSXform` layout).
- picture.rs:258-260 — playback of `SetMatrix` must compose with the CTM at
  playback start (`setMatrix(initialCTM * recorded)`).
- raster.rs:730-748 `fill_circle` — spans must be disjoint (emit each row
  exactly once); no double blending with translucent paint.
- surface.rs:36-41, 108-121, 156-158 — see premul conversion below; also
  `new_raster` must respect `info.color_type` (at minimum Rgba8888/Bgra8888)
  or reject unsupported types explicitly, and snapshots must carry the true
  color/alpha type.

### Minor

- raster.rs:273-278, simd.rs:490-506 — rounded div-255 everywhere
  (`SkMulDiv255Round`); AVX2 chunk and remainder must use the same formula.
- clip.rs:289, 299-305 (+ core region.rs:56 usage) — non-AA clip rects round
  to nearest (Skia `rect.round()`), not round-out, and containment tests use
  pixel centers.
- canvas.rs:341-344, 1471-1475 vs 526-534 — use one consistent (rounded)
  layer-origin convention between draw and composite.
- canvas.rs:1925-1939 `draw_round_rect` — clamp radii like
  `SkRRect::setRectXY` and use conic-equivalent geometry (via Task 2's oval
  conics or `RRect`).
- canvas.rs:1941-1985 `draw_arc` — |sweep| ≥ 360 draws the full oval; use
  curve segments (via path arcs), not 10° polylines.
- canvas.rs:784-797 + picture.rs:312-325 — `draw_picture` honors the optional
  paint via an implicit save_layer.
- picture.rs:391-394 — `RecordingCanvas::save` must return proper save-count
  semantics (mirror canvas.rs fixed behavior).
- raster.rs:709-717 — `StrokeAndFill` must not double-blend the overlap:
  build combined geometry (fill ∪ stroke outline) and fill once.

### Premul conversion (do first)

surface.rs:36-41, 108-121 + raster.rs:73, 101-104, 179-189: the buffer is
labeled `AlphaType::Premul` but stores straight alpha. Convert the pipeline to
true premultiplied storage: writes premultiply once at the paint→device
boundary; blends operate on premul (no divide-out in `blend_colors`);
`read_pixels`/snapshot honor the requested alpha type (unpremul conversion on
read when asked). Update simd paths and tests accordingly. This aligns
canvas with Task 3's shader convention and upstream `SkSurface_Raster`.

### Cross-crate exceptions

May update call sites/tests in `skia-rs-codec`, `skia-rs-ffi`, `skia-rs-safe`
that read surface pixels, if the premul conversion changes observable bytes —
behavior must end Skia-conformant (premul stored, unpremul delivered where the
API contract says so, e.g. wasm ImageData).

## Task 5: skia-rs-text — fonts, metrics, shaping, text blobs, COLR

Fix every finding below in `crates/skia-rs-text/src/`.

### Critical

- font.rs:1704-1707 + 1742-1753 — COLR v1 layers all dropped: for COLR v1,
  ttf-parser drives `outline_glyph(gid)` → `push_clip()` → child `paint(...)`
  → `pop_clip()`; `paint()` must not require `current_glyph` — the fill shape
  is the top of the clip stack. Restructure `ColorLayerWalker` to emit a layer
  per painted fill using the active clip glyph. Add a positive COLR v1 test
  (construct a minimal COLR v1 font fixture or use a checked-in test font).
- text_blob.rs:103, 196 — `TextBlob::from_text` / `add_text` must position
  glyphs by real per-glyph advances (`Font::glyph_advance`), not
  `size · 0.5`. Blob width must agree with `Font::measure_text`.

### Major

- font.rs:297-304 — `x_height`/`cap_height` must be **positive** (FreeType
  port convention: `os2->sxHeight/upem · scale`; fallbacks `-ascent · k`).
- shaper.rs:226-228 — negate `y_advance` and `y_offset` from rustybuzz
  (HarfBuzz y-up → Skia y-down; upstream `SkScalarFromHBPosY` is negative);
  fix consumption at paragraph.rs:650.
- font.rs:1673-1687 — COLR v1 sweep angles: ttf-parser returns raw F2Dot14 in
  units of 180°; multiply by 180 to get degrees and drop the extra negation
  (upstream passes the degree angle straight to the sweep shader).
- font.rs:539-541, 571-573, 624-626 — glyph 0 (.notdef) uses its real hmtx
  advance, bounds, and outline (tofu box), like any glyph.
- font.rs:624-644 — `glyph_path` must apply `scale_x` and `skew_x` (compose
  into the size/upem transform) consistently with `glyph_advance`.
- paragraph.rs:167-176 — `push_style` maintains a real stack; `pop()` restores
  the previous style, not default.

### Minor

- font.rs:337-338 — metrics top/bottom from the font bbox
  (`-bbox.yMax/upem·scale`, `-bbox.yMin/upem·scale`); the parsed bbox is
  already cached.
- font.rs:344-363 — `avg_char_width` from OS/2 `xAvgCharWidth`;
  `max_char_width` from bbox width.
- font.rs:310-320 — `underline_position` includes the half-thickness
  adjustment (distance to **top** of stroke).
- font.rs:293 — clamp negative line gap to 0 (`leading = max(leading, 0)`).
- font.rs:763-771 — `GlyphImage::top` converts to y-down (negate bitmap-top).
- font.rs:1756-1763 — represent ClipBox clips distinctly (don't push sentinel
  gid 0); apply or expose the box properly.
- text_blob.rs:46-48 — run bounds use fTop/fBottom-style conservative extents
  (bbox-based metrics from the fix above), not ascent/descent.
- text_blob.rs:124-126 — `unique_id()` from a monotonic atomic counter.
- paragraph.rs:595-603 — empty lines take height from the paragraph/run
  style, not `Font::default()`.
- shaper.rs:216-228 — shaped x positions multiply by `font.scale_x`
  (match upstream `SkShaper_harfbuzz`).

## Task 6: skia-rs-codec — image decoding/encoding

Fix every finding below in `crates/skia-rs-codec/src/`.

### Critical

- codec.rs:235-297 — PNG: enable `Transformations::EXPAND | STRIP_16` (or
  explicitly handle 16-bit/palette): 16-bit PNGs must decode correctly (to
  8-bit via strip), palette PNG-8 and 1/2/4-bit gray must decode (expand).
  Test with generated fixtures for: PNG-8 palette, 16-bit RGB, 4-bit gray.
- codec.rs:1035-1053 — 32-bit BI_RGB BMP: ignore the 4th byte (opaque alpha)
  for standalone BMPs (`kBGRX`); honor alpha only for BMP-in-ICO (upstream
  SkBmpCodec).

### Major

- codec.rs:662-687 — GIF: decode to the logical-screen canvas size,
  compositing the first frame at its left/top offset; `AnimatedImage` carries
  canvas dimensions.
- codec.rs:336-374 — encoders (PNG/WebP/BMP/JPEG) must unpremultiply premul
  input before writing (PNG spec requires unpremul).
- image.rs:517-575 — `make_scaled_with` filters in premul space (premultiply
  before filtering, unpremul after if needed).
- lazy_image.rs:212-248 — concurrent generation: waiters block on the
  generating thread's result (condvar or `OnceLock`-style) instead of
  erroring.
- lazy_image.rs:292-306 — `peek_pixels` returns the pixmap when pixels are
  already generated (no decode side effect), else None; make the doc example
  true.
- codec.rs:1184-1196, 1229-1230, 1012 — malformed ICO/BMP must error, not
  panic/abort: bounds-check entry offsets, validate header size and
  dimensions (reject non-positive/absurd dims before allocating; cap by
  data length).
- image.rs:293-297, generator.rs:98-104 — unique IDs from a monotonic atomic
  counter (no pointer addresses).

### Minor

- (Task 1 fixes the shared premul rounding in core; verify codec paths pick
  it up — differential test premul(200, a=130) == 102.)
- image.rs:499-506 — nearest scaling samples pixel centers
  (`floor((dst + 0.5) · scale)`).
- codec.rs:106-113 — WBMP sniffing accepts multibyte width/height ints.
- codec.rs:1000-1005 — BI_BITFIELDS: read and apply the channel masks.
- codec.rs:2319 — `height.unsigned_abs()` (no `.abs()` panic on i32::MIN).
- codec.rs:1014-1095 — truncated BMP reports incomplete input (error or
  explicit partial-decode result), not silent success.
- codec.rs:2281-2311 — JPEG dimension scan: handle all SOF markers
  (C0-C3, C5-C7, C9-CB, CD-CF) and skip RST/standalone markers (0xD0-0xD9
  have no length payload).
- lazy_image.rs:322-334, gpu_image.rs:529-539 — `read_pixels` returns false
  (or copies partially and reports it) when the destination doesn't fit;
  no silent partial success.
- animation.rs:90-92 — document/implement `DisposalMethod::Background` as
  clear-to-transparent (never the GIF background color).

## Task 7: skia-rs-gpu — wgpu/vulkan backends, tessellation, atlas, gradients

Fix every finding below in `crates/skia-rs-gpu/src/`.

### Critical

- vulkan_backend.rs:613 — remove the `CString::from_raw` over the stack
  array; parse `device_name` by reading the NUL-terminated bytes
  (`CStr::from_bytes_until_nul` on the array) with no ownership.
- wgpu_backend.rs:673-677 — build pipeline layouts from real bind group
  layouts matching the shaders' `@group(0)` declarations (uniform buffer for
  solid/gradient; texture+sampler+uniforms for textured); register_pipeline
  must succeed for every `paint_to_pipeline` output. Add an executor test
  that registers and draws each builtin pipeline headlessly (skip if no
  adapter).
- pipeline.rs:625-643 — `PipelineKey` must cover every field that affects the
  compiled pipeline: stencil front/back ops+compares+masks, depth
  compare/write, blend operations and alpha components, write mask, topology,
  cull mode, entry points, vertex attribute formats.
- stencil_cover.rs:208-280 — emit the closing fan triangle (origin,
  last_point, first_point) at Close/contour end.
- paint_bridge.rs:100-256 + shader.rs:66-78, 157-200 — premul end-to-end:
  shaders output premultiplied color (multiply rgb by a in FS or pack premul
  uniforms), and `blend_mode_to_state` sets per-mode **alpha** blend
  components (not hardwired SrcOver) per Ganesh's Porter-Duff table.

### Major

- wgpu_backend.rs:972-988 — wire the stencil surface: passes attach
  `depth_stencil_attachment` when the pipeline needs it; stencil-then-cover
  must be executable (clear stencil between path draws).
- tessellation.rs:173-218, 394-426 (large) — fill tessellation must apply the
  fill rule across all contours (holes must stay holes). Minimum conforming
  approach: flatten all contours, run them through the stencil-cover path by
  default, and keep direct triangulation only for single-contour convex
  paths.
- tessellation.rs:104-146, 271-333, 783-808 — device-space flattening
  tolerance: scale tolerance through the view matrix
  (`GrPathUtils::scaleToleranceToSrc` equivalent); drop hard subdivision
  caps in favor of tolerance-driven counts (with a sane upper bound like
  Skia's kMaxPointsPerCurve).
- stencil_cover.rs:20-27 — inverse fill types: cover pass tests `Equal 0`
  (winding-inverse) / even-odd-inverse accordingly, covering clip bounds.
- atlas.rs:229-233 + glyph_cache.rs:196-213 — TooLarge check includes padding
  (`width + 2·padding > config.width`); the insert retry loop must terminate
  (return TooLarge instead of looping).
- glyph_cache.rs:193-213, 232-241 — eviction frees atlas regions (row-based
  free lists or generation-checked repack); `insert` must not reset the atlas
  mid-frame without invalidating outstanding UVs — bump `atlas_generation`
  and make `GlyphBatch` validation mandatory (return stale markers).
- gradient.rs:123-146 — premultiply **after** transfer encoding
  (`srgb(r)·a`), or better: store premul in linear and encode at sample time
  consistent with the shader; match upstream gradient texture generation.
- gradient.rs:228-236 — sweep t starts at the +x axis:
  `t = atan2(-dy, -dx)/(2π) + 0.5` form used upstream (`xy_to_unit_angle`);
  make t=0 at +x, increasing clockwise in y-down space.
- sdf.rs:121-165, 284-292 — inside is **high** (>128 texel values), matching
  `SkDistanceFieldGen`; fix `sdf_to_texture` mapping and any consumers.
- tiling.rs:81-145, 169-178 — mixed tile modes: per-axis handling (a Clamp
  axis clamps UV within a single edge tile; only Repeat/Mirror axes tile).
- paint_bridge.rs:630-665 — implement `as_linear_gradient` /
  `as_radial_gradient` / `as_sweep_gradient` so real gradient geometry
  (points/radius/angles + stops) flows to the GPU uniforms instead of the
  unit-segment fallback.

### Minor

- wgpu_backend.rs:249-256, 289-319, 1126-1133 — for `*UnormSrgb` targets,
  convert clear colors sRGB→linear before passing to `wgpu::Color`.
- wgpu_backend.rs:972-995 — MSAA: pipelines take the surface sample count;
  passes set resolve_target for multisampled surfaces.
- command.rs:766-770 — record a real `DispatchCompute` command (executor may
  reject unsupported, but never replay as a draw).
- command.rs:212-219 — `ScissorRect::from_rect` clamps the box (shrink
  width/height when left/top < 0; clamp to framebuffer on use).
- tessellation.rs:429-512 — miter joins scale by `1/cos(θ/2)` with miter
  limit; implement bevel fallback; honor caps (at least butt/square/round on
  open contours).
- tessellation.rs:204-211 — a `Line` after `Close` without `Move` starts the
  new contour at the previous contour's start point (Skia post-close rule:
  new contour starts at the close point) — match SkPath semantics (inject
  move to last_move_point) consistent with Task 2 builder fix.
- gradient.rs:268-284 — Repeat LUT: last texel holds t→1⁻ color (sample the
  half-texel-centered ramp), no first-stop wraparound seam.
- sdf.rs:131-164 — measure distance to the mask **edge** (±0.5 px offset
  correction) per `SkDistanceFieldGen`.
- atlas.rs:407-412 — `compact()` must not re-place an unplaceable entry at
  stale coordinates: drop it and report eviction.
- atlas.rs:42-49, 295-305 — inset `uv_rect` by half a texel; zero region
  texels (or full padding rows) on reset/compact to avoid stale bleed.
- metal_backend.rs:198-211, 547-548 — gate `Depth24Unorm_Stencil8` on
  `depth24Stencil8PixelFormatSupported` (fall back to Depth32Float_Stencil8);
  report Tier1 argument buffers as available.

## Task 8: skia-rs-svg — parser, DOM, renderer

Fix every finding below in `crates/skia-rs-svg/src/`. (Path-crate SVG parser
findings were fixed in Task 2; this task may rely on them.)

### Major

- parser.rs:445-446 — resolve percentage lengths against the viewport per
  SVG 1.1 §7.10 (width % of viewport width, height % of height, other % of
  normalized diagonal), like `SkSVGLengthContext::resolve`.
- dom.rs:125 + render.rs:135-144 — implement presentation-attribute
  inheritance: a presentation context stack; unspecified fill/stroke/
  stroke-width/opacity-related properties inherit from the parent; `fill`
  default black lives at the root, not per node.
- render.rs:246-250, 452 — group `opacity` composites via save_layer with
  alpha (leaf-only paint-alpha shortcut allowed only when the group has a
  single drawable child, per Skia's optimization).
- parser.rs:529-533 — `currentColor` resolves against the inherited `color`
  property (default black), not None.
- render.rs:197-209 — `<polyline>` (and `<polygon>`) fill by default; fill
  and stroke both render.
- render.rs:85-96 — implement `preserveAspectRatio` (xMidYMid meet default:
  uniform min-scale + centering translation; `none`: non-uniform;
  meet/slice + all alignments per SkSVGPreserveAspectRatio).
- glyph_svg.rs:78-95 — SVG glyph documents render in font units scaled by
  ppem/upem (glyph em box → target size), not stretched to the canvas.

### Minor

- render.rs:—(styles) — apply parsed `fill-rule`, `stroke-dasharray`,
  `stroke-dashoffset`, `stroke-linecap`, `stroke-linejoin` to the
  paint/path (dash via Task 2's DashEffect; fill-rule via FillType::EvenOdd).
- render.rs:533-545 — objectBoundingBox radial gradients map the unit circle
  through the bbox transform (elliptical for non-square bounds); percentage
  radius against the normalized diagonal.
- render.rs:487-507 — compose bbox-matrix × gradientTransform in that order
  for OBB gradients.
- render.rs:296-334 — `build_clip_path` honors child transforms for all shape
  kinds in `<clipPath>`.
- render.rs:241-245 — `<use>` applies translate(x, y) and guards recursion
  (depth cap or visited set — malformed cycles must not stack-overflow).
- css.rs:392-415, 371-375 — fill-opacity/stroke-opacity multiply into the
  paint alpha; apply declarations in document/cascade order (keep an ordered
  list, not HashMap iteration).
- parser.rs:510-514 — parse the `url(#id) fallback` paint grammar; use the
  fallback color when the reference is missing.
- export.rs:677-689 — export conics via ConvertConicToQuads-equivalent
  subdivision (Task 2 provides conic→quad), not a naive quad.

## Task 9: skia-rs-pdf — PDF backend

Fix every finding below in `crates/skia-rs-pdf/src/`.

### Critical

- canvas.rs:439-452 — text under the page y-flip CTM needs the compensating
  text matrix `1 0 skew -1 x y Tm` (upstream SkPDFDevice GlyphPositioner);
  glyphs must render upright.
- canvas.rs:471 — images need the unit-square counter-flip
  (`setScale(1,-1); postTranslate(0,1)` before the placement cm).

### Major

- canvas.rs:447-451 — non-ASCII text with simple fonts: either encode
  single-byte (WinAnsi subset with escapes) or switch the run to a Type0
  font; never UTF-16BE hex strings against a simple font.
- canvas.rs:325-393, 543-549 — honor path fill type: `f*`/`B*`/`W*` for
  even-odd.
- font.rs:341-343 — ToUnicode CMap for single-byte fonts uses 1-byte
  codespace `<00> <FF>` and 2-hex-digit bfchar codes.
- document.rs:804-809 + font.rs:281-287 — Type0 fonts must include the
  `/DescendantFonts` array (CIDFontType2 with CIDToGIDMap, W array,
  CIDSystemInfo) per PDF 32000-1 §9.7.6.

### Minor

- transparency.rs:136-142 — `ExtGraphicsState::cache_key` covers soft_mask,
  alpha_is_shape, text_knockout, overprint.
- font.rs:92-95 — Symbol/ZapfDingbats: omit /Encoding (use built-in).
- document.rs:659-673, 847 — PDF/A: embed a real minimal sRGB ICC profile
  (a valid ICC v2 sRGB profile blob checked into the crate) or stop claiming
  PDF/A conformance when no profile is available.

## Task 10: skia-rs-skottie — Lottie player

Fix every finding below in `crates/skia-rs-skottie/src/`. Most real Lottie
files currently render blank; this task makes the common path work.

### Critical

- keyframe.rs:178 — `as_vec2()` accepts Vec3 (take first two components) so
  position/anchor from Bodymovin 3-component arrays work.
- keyframe.rs:452-463 + shapes.rs:341-348, mask.rs:121-128 — parse bezier
  path values (`{"i","o","v","c"}`) into `KeyframeValue::Path` (build the
  cubic contour: vertex + out-tangent → next vertex + in-tangent; `c` closes)
  and interpolate between path keyframes point-wise; `"sh"` shapes and masks
  must produce real geometry.
- shapes.rs:111-113, 906-912 — parse group `"tr"` into a real Transform
  (anchor/position/scale/rotation/opacity, animated) and apply it to the
  group's children; group opacity multiplies.

### Major

- model.rs:339-341 — stroke dash: `"d"` on strokes is the dash array (list of
  {n,nm,v} elements) — separate it from the shape `direction` field (custom
  deserializer or untagged enum); dashed strokes must parse and apply
  (gaps/dashes/offset via Task 2's DashEffect).
- render.rs:122-135 + layers.rs:384 — layer transforms/opacity/masks evaluate
  at unadjusted comp time; only precomp **content** gets the `st`/`sr`
  remap (upstream Layer.cpp/PrecompLayer.cpp).
- layers.rs:373, 384 + model.rs:64-132 — parse `"sr"` (stretch) and `"tm"`
  (time remap); remap math is `(t − st)/sr`; `tm` (animated, seconds)
  overrides the linear mapping for precomp content.
- keyframe.rs:393-400 — a trailing `{"t":N}`-only keyframe inherits the
  previous keyframe's end value.
- shapes.rs:561 — parse fill rule `"r"` (2 = even-odd) and set it on the
  path/paint.
- render.rs:133-139 + mask.rs:250-328 — implement mask modes: Add masks
  union, Subtract subtracts, Intersect intersects; honor `inv` and mask
  opacity (alpha mask via save_layer where needed); remove the identity
  stubs.
- render.rs:117 — apply layer parenting: compose the parent transform chain
  (guard against cycles).
- model.rs:327-335 + render.rs:322-330 — parse trim-path `s`/`e`/`o`/`m` and
  implement trim via path measure (Task 2's measure): offset 360° = one full
  loop; m:1 simultaneous, m:2 individual.
- render.rs:271-277 — implement gradient fills/strokes: parse `g` (stop
  count + interleaved pos/rgb [+ alpha tail]), `s`/`e` points, `t` type
  (1 linear, 2 radial), build the matching shader.
- layers.rs:371-372 — implement track mattes: `td` source layer is hidden and
  applied as an alpha/luma mask to the `tt` consumer.

### Minor

- transform.rs:208, 277 — skew is `Skew(tan(−radians(pin(sk, −85, 85))))`
  (negated, pinned) per upstream Transform.cpp.
- model.rs:201-221 — parse `ti`/`to` and interpolate position along the
  spatial bezier.
- shapes.rs:222-235 — rounded-rect corners via circular arcs (RRect
  geometry), matching AE start point/winding.

## Task 11: skia-rs-ffi, skia-rs-safe, headers — C ABI and safety

Fix every finding below.

### Critical

- ffi lib.rs:1880, 1908, 3063, 3173 — enum parameters received by value from
  C (`sk_clip_op_t`, `sk_region_op_t`, `sk_trim_mode_t`, and audit for any
  others) become raw `u32` parameters decoded via match (like
  `decode_tile_mode`), returning an error/no-op on out-of-range. Make
  `test_canvas_clip_rect_rejects_invalid_op` actually pass an invalid op.

### Major

- ffi lib.rs:277-284 — `sk_clip_op_t`: match upstream `SkClipOp`
  (`kDifference = 0`, `kIntersect = 1`). Update the generated header and any
  in-repo callers.
- ffi lib.rs:491-501 — `decode_color_type`: match `SkColorType` numbering
  (`kRGB_888x = 5`, `kBGRA_8888 = 6`, …); unknown values return an error
  (null/false), never silently RGBA.
- safe wasm.rs:102-108 — `get_image_data`: no R/B swap (surface is RGBA);
  unpremultiply for `ImageData` (expects unpremul). After Task 4's premul
  conversion, read via the unpremul read-pixels path.
- ffi lib.rs:1661-1689 — implement the documented surface lock: track lock
  state; second `lock_canvas` returns null until unlock; `sk_surface_draw_*`
  while locked returns an error (or document + enforce whichever contract
  upstream C API has).
- ffi lib.rs:1388, 1404-1408, 2505-2508, 2570-2574 — support in-place
  matrix ops safely: read inputs into locals (copy) before writing through
  the result pointer; never hold `&`/`&mut` simultaneously (use raw-pointer
  reads `ptr::read` then write).
- include/skia-rs.h:94 — regenerate/replace the stale committed root header:
  make it match the ffi crate's current exports (or delete it in favor of
  `crates/skia-rs-ffi/include/skia_rs.h` with a build-script copy); it must
  compile as C. Wire header generation into CI/build so it can't drift
  (`SKIA_RS_FFI_EMIT_HEADER=1` in a checked build step or a test comparing
  the committed header to a fresh generation).

### Minor

- ffi lib.rs:829-853 — decode all 29 blend modes (0-28); out-of-range errors.
- ffi lib.rs:1467-1470 — `sk_version` from `env!("CARGO_PKG_VERSION")`.
- ffi lib.rs:1825-1831 — `sk_canvas_clear` respects the clip (route through
  the fixed canvas `clear`).
- ffi lib.rs:29-31, 237-249 — fix the `sk_refcnt_get_count` doc claim (it
  reads 4 bytes; the tag is best-effort, not memory-safe against invalid
  pointers).
- ffi lib.rs:2785-2806 — recording canvas holds a liveness-checked handle
  (generation counter or weak reference) so use-after-recorder-delete
  returns an error instead of UB.
- ffi abi.rs:243-254 — size assertion made pointer-width aware; fix or drop
  the false "matches SkImageInfo layout" claim.
- safe android.rs:851 — fix the test (`R5G6B5_UNORM`); align
  `BitmapConfig`/`HardwareBufferFormat` repr values with the real Android
  constants (ANDROID_BITMAP_FORMAT_*, AHARDWAREBUFFER_FORMAT_*); remove the
  nonexistent `R4G4B4A4_UNORM = 5`; `HardwareBuffer::new` on Android must
  actually allocate (or return None until implemented — no fake Some).
