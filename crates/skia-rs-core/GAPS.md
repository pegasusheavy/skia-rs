# skia-rs-core Gap Analysis

**Date:** 2026-04-23
**Reviewer:** Claude (Opus 4.6)

## Summary
- Total public functions reviewed: 217 (`pub fn` declarations; includes trait method implementations and derived methods)
- Total test functions: 36 → ~75+ (expanded with gap fix tests)
- Total gaps found: 12
- **Critical gaps resolved: 5/5 ✅**
- **Nice-to-have gaps resolved: 7/7 ✅**
- Estimated complexity: Medium-High
- **Phase 2 status: COMPLETE**
- **P7C (2026-04-26): GAP-C1 resolved — ICC tag table parsing implemented**

## Files Reviewed
- [x] lib.rs
- [x] scalar.rs (orphan file -- not part of module tree)
- [x] color.rs
- [x] geometry.rs
- [x] matrix.rs (orphan file -- not part of module tree)
- [x] matrix44.rs
- [x] region.rs
- [x] pixel.rs

## Critical Gaps

### GAP-C1: `IccProfile::from_bytes()` defaults to sRGB for all parsed profiles
**Status:** ✅ RESOLVED (P7C, 2026-04-26) -- tag table parsing for
rTRC + rXYZ/gXYZ/bXYZ tags; gamut classifier matches sRGB, Display P3,
Adobe RGB, Rec.2020 (else falls back to `ColorGamut::Custom`); transfer
curves recovered from `curv` (count=0/1/tabulated) and `para` (function
types 0-4). Unsupported profiles still fall back to sRGB rather than
returning `None`.
**Location:** `src/color.rs` -- `IccProfile::from_bytes` + `mod icc`
**Tests:** `test_icc_profile_parses_srgb_profile_correctly`,
`test_icc_profile_parses_display_p3_profile`,
`test_icc_profile_rejects_short_buffer`,
`test_icc_profile_rejects_missing_magic` plus fixtures under
`crates/skia-rs-core/tests/fixtures/`.
**Original issue:** The function correctly parsed the ICC header fields
(profile class, color space, PCS) but hardcoded `ColorSpace::srgb()` as the
`embedded_color_space` for every parsed profile, regardless of its actual
transfer function and gamut. A Display P3 ICC profile parsed with
`from_bytes()` would report `is_srgb() == true`.
**Skia reference:** `skia/src/core/SkColorSpace.cpp` --
`SkColorSpace::MakeICC()` parses the tag table (TRC + rXYZ/gXYZ/bXYZ tags) to
extract the actual transfer function and gamut matrix.

### GAP-C2: `Region::contains_rect()` has false negatives for complex regions
**Status:** ✅ RESOLVED - scanline containment algorithm
**Location:** `src/region.rs:171-183`
**Issue:** Semantic gap
**Description:** For complex (multi-rect) regions, `contains_rect` only checks
whether a single component rectangle fully contains the query rect. It does not
handle the case where the query rect is contained by the *union* of multiple
adjacent component rects but not by any single one. The inline comment
acknowledges this: "For complex regions, we'd need a more sophisticated
algorithm / This is a simplified check that may have false negatives." This is
observable behavior, not just an optimization concern -- clipping code that uses
`contains_rect` to skip expensive intersection work will get wrong answers.
**Skia reference:** `skia/src/core/SkRegion.cpp` -- `SkRegion::contains(const
SkIRect&)` uses a scanline-based algorithm that correctly handles this.
**Estimated effort:** 1-2 days
**Dependencies:** None
**Priority:** HIGH

### GAP-C3: `Region::union()` does not merge overlapping rectangles
**Status:** ✅ RESOLVED - canonicalize_rects scanline merge
**Location:** `src/region.rs:285-298`
**Issue:** Semantic gap -- rectangle list grows unboundedly
**Description:** The `union` method simply appends the other region's rects to
`self.rects` without any coalescing. Repeated unions cause the rect list to grow
linearly with each operation, and operations like `contains`, `intersects_rect`,
and `difference` iterate over every rect, making them O(n) per prior union.
Skia's `SkRegion` merges rects into a canonical scanline-sorted, non-overlapping
form. The current implementation also violates the implicit invariant that
component rects should not overlap, which can cause `contains_rect` to
double-count area and `intersect` to produce duplicate fragments.
**Skia reference:** `skia/src/core/SkRegion.cpp` -- `SkRegion::op()` uses
`SkRegion::Oper` to produce a canonical scanline representation.
**Estimated effort:** 3-5 days (scanline merge algorithm)
**Dependencies:** None
**Priority:** HIGH

### GAP-C4: `geometry::Matrix::map_point()` division by zero when w == 0 with perspective
**Status:** ✅ RESOLVED - w==0 guard added
**Location:** `src/geometry.rs:1168-1173`
**Issue:** Missing validation -- divide-by-zero
**Description:** When the perspective row produces w=0 for a given input point,
the code performs `x / w` and `y / w`, yielding `+inf` or `-inf` (or NaN if
numerator is also 0). This is technically not a panic (IEEE 754), but
downstream code that expects finite coordinates will misbehave silently. The
standalone `matrix.rs` file handles this correctly (line 150: `let w_inv = if
w != 0.0 { 1.0 / w } else { 0.0 };`), but the version in `geometry.rs` does
not.
**Skia reference:** `skia/src/core/SkMatrix.cpp` -- `SkMatrix::mapPointsInternal`
guards against w==0 by returning (0,0).
**Estimated effort:** 30 minutes
**Dependencies:** None
**Priority:** HIGH

### GAP-C5: Duplicate `Matrix` type defined in two files
**Status:** ✅ RESOLVED - orphan files removed
**Location:** `src/geometry.rs:984-1233` and `src/matrix.rs:1-212`
**Issue:** Architectural -- dead code / confusing duplication
**Description:** There are two independent `Matrix` implementations. The one
in `geometry.rs` is the live version (exported via `lib.rs`). The one in
`matrix.rs` is an orphan file not declared as a module in `lib.rs`. The two
implementations differ in representation (`values: [Scalar; 9]` array vs named
fields) and behavior (the geometry version skips the w==0 guard; the matrix.rs
version includes it; the matrix.rs version takes `kx`/`ky` directly in `skew()`
while geometry takes `kx.tan()`/`ky.tan()`). If either is updated independently
the divergence grows. Similarly, `scalar.rs` is an orphan module with a
duplicate `Scalar` type alias and utility functions that mirror the inline
definition in `lib.rs`.
**Skia reference:** N/A (architectural issue)
**Estimated effort:** 1 hour (delete orphans or consolidate)
**Dependencies:** Must decide which representation to keep before removing the other
**Priority:** HIGH

## Nice-to-Have Gaps

### GAP-N1: `geometry::Matrix::skew()` uses `tan()` instead of raw skew values
**Status:** ✅ RESOLVED - skew now takes raw factors
**Location:** `src/geometry.rs:1073-1077`
**Issue:** API deviation from Skia
**Description:** The `skew()` constructor in `geometry.rs` applies `kx.tan()`
and `ky.tan()` to the input values, treating the inputs as angles in radians.
Skia's `SkMatrix::MakeSkew(sx, sy)` takes the raw skew factors directly
(i.e., the tangent is already expected to be pre-computed by the caller). The
standalone `matrix.rs` version correctly takes raw values. This means the
geometry version is API-incompatible with Skia for callers who pass raw skew
factors.
**Skia reference:** `skia/include/core/SkMatrix.h` -- `static SkMatrix
MakeSkew(SkScalar kx, SkScalar ky)` places kx/ky directly in the matrix.
**Estimated effort:** 15 minutes (change to raw values, update any callers)
**Dependencies:** GAP-C5 (consolidate Matrix types first)
**Priority:** MEDIUM

### GAP-N2: No `RRect` validation for clamping radii to half-dimensions
**Status:** ✅ RESOLVED - radii clamped to half-dimensions
**Location:** `src/geometry.rs:903-908`
**Issue:** Missing validation
**Description:** `RRect::from_rect_xy()` does not clamp radii to
`[0, width/2]` and `[0, height/2]`. If a caller passes radii larger than half
the rect dimension, the resulting RRect has geometrically invalid corners.
Skia's `SkRRect::setRectXY` clamps radii to fit.
**Skia reference:** `skia/src/core/SkRRect.cpp` -- `SkRRect::setRectXY()`
**Estimated effort:** 30 minutes
**Dependencies:** None
**Priority:** MEDIUM

### GAP-N3: `Matrix44::get()`/`set()` do not bounds-check row/col indices
**Status:** ✅ RESOLVED - bounds checks added
**Location:** `src/matrix44.rs:275-283`
**Issue:** Panic risk
**Description:** `get(row, col)` accesses `self.values[col * 4 + row]` without
checking that row < 4 and col < 4. Passing row=4 or col=4 causes an
out-of-bounds panic. While this is standard Rust behavior (panic on OOB), Skia
returns 0 for out-of-range indices in debug builds and has undefined behavior
in release. Adding a bounds check or using `debug_assert!` would improve the
error message.
**Estimated effort:** 15 minutes
**Dependencies:** None
**Priority:** LOW

### GAP-N4: `Matrix44::ortho()` and `perspective()` have no guard for zero-width ranges
**Status:** ✅ RESOLVED - ortho_checked/perspective_checked added
**Location:** `src/matrix44.rs:207-231` and `src/matrix44.rs:234-266`
**Issue:** Missing validation
**Description:** `ortho()` divides by `(right - left)`, `(top - bottom)`, and
`(far - near)`. `perspective()` divides by `(near - far)` and computes
`1.0 / tan(fov_y/2)`. If any range is zero (e.g., `left == right`) or fov_y
is 0, the result contains `inf` or `NaN`. Skia does not validate either, but
documenting the preconditions or returning `Option<Self>` would be more
Rust-idiomatic.
**Estimated effort:** 30 minutes
**Dependencies:** None
**Priority:** LOW

### GAP-N5: `geometry::Matrix` inverse uses hard-coded epsilon (1e-10) for singularity check
**Status:** ✅ RESOLVED - epsilon = Scalar::EPSILON * 256
**Location:** `src/geometry.rs:1212`
**Issue:** Numerical robustness
**Description:** The determinant threshold `1e-10` is arbitrary and may be too
tight for f32 arithmetic (f32 epsilon is ~1.19e-7). Matrices with very small
but legitimate determinants will incorrectly return `None`, while matrices
near-singular at ~1e-9 will succeed but produce numerically garbage inverses.
Skia uses `SK_ScalarNearlyZero` (1/4096 = ~2.44e-4) for its comparisons.
**Skia reference:** `skia/src/core/SkMatrix.cpp` -- uses
`SkScalarNearlyZero(det)` for the singularity check.
**Estimated effort:** 15 minutes
**Dependencies:** None
**Priority:** MEDIUM

### GAP-N6: `convert_pixels()` does not handle alpha type conversion
**Status:** ✅ RESOLVED - alpha conversion in convert_pixels
**Location:** `src/pixel.rs:565-617`
**Issue:** Semantic gap
**Description:** The function converts between color types (e.g. RGBA8888 to
BGRA8888) but ignores the `alpha_type` of source and destination. Converting
from `Premul` to `Unpremul` (or vice versa) silently passes through without
adjusting pixel values. Skia's `SkConvertPixels()` handles alpha type
mismatches by applying premultiplication or unpremultiplication during the
conversion.
**Skia reference:** `skia/src/core/SkConvertPixels.cpp`
**Estimated effort:** 1-2 hours
**Dependencies:** None
**Priority:** MEDIUM

### GAP-N7: `scalar.rs` orphan not exposed or deleted
**Status:** ✅ RESOLVED - scalar.rs orphan removed
**Location:** `src/scalar.rs` (entire file)
**Issue:** Dead code
**Description:** `scalar.rs` defines `Scalar`, 7 constants, and 4 utility
functions, none of which are accessible because the module is not declared in
`lib.rs`. The utility functions (`scalar_nearly_zero`, `scalar_nearly_equal`,
`scalar_is_finite`, `scalar_interp`) could be useful throughout the crate.
Either expose the module or delete the file to avoid confusion.
**Estimated effort:** 15 minutes
**Dependencies:** GAP-C5 (decision about orphan files)
**Priority:** LOW

## Test Coverage Gaps

**Modules with no test coverage:**
- `scalar.rs` -- Orphan file, 0 tests. Even if exposed, the functions are trivial wrappers.
- `matrix.rs` -- Orphan file, 0 tests. The live `Matrix` in `geometry.rs` has 3 tests.
- `lib.rs` -- No tests (only re-exports and the `AsScalar` trait; low risk).

**Functions with no unit tests (in live modules):**

color.rs (13 tests, but missing coverage for):
- `unpremultiply_color()` -- tested only indirectly via premultiply roundtrip
- `Color4f::unpremul()` -- no test for divide-by-zero (alpha==0) edge case
- `Color4f::lerp()` -- not tested
- `color4f_srgb_to_linear()` / `color4f_linear_to_srgb()` -- not directly tested
- `rgb_to_xyz()` / `xyz_to_rgb()` -- not tested
- `rgb_to_lab()` / `lab_to_rgb()` -- not tested
- `IccProfile::from_bytes()` -- tested (P7C: sRGB and Display P3 fixtures plus short-buffer / missing-magic rejection)
- `ColorSpace::display_p3()` / `ColorSpace::srgb_linear()` -- not tested
- `ColorType::has_alpha()` -- not tested
- `ColorType::n32()` -- not tested

geometry.rs (5 tests, but missing coverage for):
- `Point::normalize()` -- not tested (including zero-length edge case)
- `Point::cross()`, `Point::distance()`, `Point::scale()`, `Point::lerp()` -- not tested
- `Point3` (all methods) -- not tested
- `ISize`, `Size` (all methods) -- not tested
- `IRect::from_xywh()`, `IRect::from_size()`, `IRect::union()`, `IRect::offset()`, `IRect::inset()`, `IRect::contains()` -- not tested
- `Rect::contains_rect()`, `Rect::intersects()`, `Rect::round_out()`, `Rect::round_in()`, `Rect::round()` -- not tested
- `RRect` (all methods) -- not tested
- `Matrix::rotate()`, `Matrix::rotate_around()`, `Matrix::skew()`, `Matrix::scale()` -- not tested
- `Matrix::concat()`, `Matrix::map_rect()`, `Matrix::determinant()` -- not tested
- `Matrix::is_translate()`, `Matrix::is_scale_translate()` -- not tested
- Operator impls for `Point` (`Add`, `Sub`, `Mul`, etc.) -- not tested

matrix44.rs (5 tests, but missing coverage for):
- `rotate_x()`, `rotate_y()`, `rotate_z()`, `rotate()` -- not tested
- `look_at()`, `perspective()`, `ortho()` -- not tested
- `pre_translate()`, `post_translate()`, `pre_scale()`, `post_scale()` -- not tested
- `transpose()` -- not tested
- `determinant()` -- only tested indirectly through `invert()`
- `map_point()` (2D) -- not tested

region.rs (7 tests, but missing coverage for):
- `contains_rect()` -- not tested (especially the complex-region false-negative case)
- `intersects_rect()`, `intersects_region()` -- not tested
- `set_region()` -- not tested
- `RegionOp::Xor`, `RegionOp::ReverseDifference`, `RegionOp::Replace` -- not tested
- `from_rect_f()` -- not tested

pixel.rs (6 tests, but missing coverage for):
- `ImageInfo::new_srgb()`, `new_alpha8()`, `new_n32()`, `new_n32_opaque()` -- not tested
- `ImageInfo::compute_byte_size()`, `min_byte_size()`, `bounds()` -- not tested
- `ImageInfo::with_alpha_type()`, `with_color_type()`, `with_color_space()`, `with_dimensions()` -- not tested
- `Pixmap::row()`, `Pixmap::pixel_addr()` -- not tested
- `Bitmap::row()`, `Bitmap::row_mut()`, `Bitmap::as_pixmap()`, `Bitmap::erase()`, `Bitmap::fill()` -- not tested
- `convert_pixels()` -- only RGBA<->BGRA tested; RGB888, Gray8, Alpha8, RGB565 conversions not tested
- `unpremultiply_in_place()` and `premultiply_in_place()` edge cases (a==0, a==255) -- not tested

## Implementation Notes

### Topic: Orphan source files (matrix.rs, scalar.rs)
**Background:** Two source files exist in `src/` that are not declared as
modules in `lib.rs`. They are completely dead code -- the compiler never sees
them. The `Matrix` type from `geometry.rs` and the `Scalar` type alias from
`lib.rs` are the live versions. The orphan files appear to be earlier
iterations that were superseded when the types were moved into `geometry.rs`
and `lib.rs` respectively.
**Complexity:** Trivial to resolve
**Notes:** The `matrix.rs` orphan has some desirable properties (named fields
for readability, correct w==0 guard) that the live `geometry.rs` version
lacks. Consider adopting those improvements when consolidating.

### Topic: Region scanline representation
**Background:** The current `Region` stores an unordered `Vec<IRect>` with no
invariant about non-overlap or scanline sorting. All region operations (union,
intersect, difference, xor) operate by brute-force pairwise comparison. This
is O(n*m) for most operations and produces an ever-growing rect list for
unions.
**Skia references:** `skia/src/core/SkRegion.cpp`, `SkRegion_path.cpp`
**Complexity:** Major (3-5 days) -- implementing a proper scanline-based
region algorithm with band merging
**Notes:** This is the largest single gap in the core crate. The current
implementation is correct for simple cases (single rects, small unions) but
will degrade for real-world clip regions. Prioritize this before any code
that uses regions for complex clipping.

### Topic: ICC profile tag table parsing
**Background:** The ICC profile parser reads only the 128-byte header. Real
ICC profiles encode the transfer function and gamut in tagged data (TRC tags,
XYZ primary tags) within the profile body. Without parsing these tags, all
non-sRGB profiles are misidentified.
**Skia references:** `skia/src/core/SkColorSpace.cpp`,
`skia/src/core/SkICCProfile.cpp`
**Complexity:** Medium (2-3 days)
**Notes:** Consider using the `lcms2` crate for full ICC support rather than
re-implementing tag parsing from scratch. Alternatively, support only the most
common profiles (sRGB, Display P3, Adobe RGB) by matching known profile
hashes.

## Recommendations

1. **Fix `geometry::Matrix::map_point()` w==0 guard** (GAP-C4) -- 30 minutes,
   eliminates a divide-by-zero in the most-used matrix operation
2. **Remove or consolidate orphan files** (GAP-C5, GAP-N7) -- 1 hour, reduces
   confusion and prevents divergent implementations
3. **Fix `Matrix::skew()` to match Skia API** (GAP-N1) -- 15 minutes, after
   orphan consolidation
4. **Add RRect radius clamping** (GAP-N2) -- 30 minutes, prevents invalid
   geometry
5. **Add alpha-type handling to `convert_pixels()`** (GAP-N6) -- 1-2 hours,
   correctness for any real pixel pipeline
6. **Address `Region::contains_rect()` false negatives** (GAP-C2) -- 1-2 days,
   correctness issue for clipping
7. **Implement Region scanline merge** (GAP-C3) -- 3-5 days, foundational for
   non-trivial clip regions
8. **Implement ICC tag table parsing** (GAP-C1) -- 2-3 days, required for
   color-managed workflows
9. **Expand test coverage** -- 1-2 days, targeting the 60+ untested public
   functions listed above
