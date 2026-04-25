# skia-rs-path Gap Analysis

**Date:** 2026-04-23
**Reviewer:** Claude (Opus 4.6)

## Summary

**Phase 3 status: COMPLETE (with documented deferrals)**

- Total public functions reviewed: 106
- Total gaps found: 14
- **Critical gaps resolved: 4/6** (GAP-C4, C5 deferred to boolean ops subplan)
- **Nice-to-have gaps resolved: 6/8** (GAP-N2, N3 deferred - control-polygon approximation works for typical use)
- **Overall resolution: 10/14, 4 deferred**

## Files Reviewed

- [x] lib.rs (29 lines) - module re-exports
- [x] path.rs (643 lines) - core Path type and iteration
- [x] builder.rs (523 lines) - PathBuilder for construction
- [x] ops.rs (650 lines) - boolean operations (union, intersect, difference, xor)
- [x] effects.rs (1144 lines) - path effects (dash, corner, discrete, trim, compose, sum, 1D/2D)
- [x] measure.rs (89 lines) - **COMPLETE STUB** - arc length parameterization
- [x] svg.rs (472 lines) - SVG path string parsing
- [x] path_utils.rs (484 lines) - stroke to fill conversion

## Critical Gaps

### GAP-C1: PathMeasure (COMPLETE STUB)

**Status:** RESOLVED (Tasks 8-13 - PathMeasure fully implemented with adaptive curve flattening)

**Location:** `src/measure.rs`
**Issue:** Entire module is stubbed -- all methods return `None` or `0.0`
**Description:** PathMeasure provides arc length parameterization for paths, allowing
queries like "give me the point 50% along this path" or "extract a subsegment from
distance A to B". Currently non-functional. The `compute_lengths()` method sets
`total_length` to `0.0` unconditionally, and every query method (`get_point_at`,
`get_tangent_at`, `get_matrix_at`, `get_segment`) returns `None` after a bounds
check against the always-zero total_length (meaning the bounds check itself always
triggers the early return for any positive distance).

**Stubbed functions:**
- `compute_lengths()` - Sets total_length to 0.0, never populates contour_lengths
- `get_point_at(distance)` - Returns None (TODO comment on line 48)
- `get_tangent_at(distance)` - Returns None (TODO comment on line 58)
- `get_matrix_at(distance)` - Returns None (TODO comment on line 68)
- `get_segment(start, end)` - Returns None (TODO comment on line 78)

**Compiler warning:** `field 'path' is never read` on the stored path, confirming
that no method actually accesses the path data.

**Skia reference:**
- `skia/src/core/SkPathMeasure.cpp` (~500 lines)
- `skia/src/core/SkContourMeasure.cpp`
- `skia/src/core/SkGeometry.cpp` (curve evaluation)

**Algorithm required:**
1. Curve flattening: Convert Bezier curves to polylines with adaptive tolerance
2. Length computation: Build cumulative length table per contour
3. Point interpolation: Binary search cumulative table + linear interpolation within segment
4. Tangent calculation: Derivative evaluation at the located parameter
5. Matrix generation: Combine position + tangent into an affine transform
6. Segment extraction: Build new Path from distance range, splitting curves at boundaries

**Estimated effort:** 2-3 weeks (most complex gap in path crate)
**Dependencies:**
- Curve subdivision utilities (de Casteljau splitting at arbitrary t)
- Binary search helpers for cumulative distance table
**Priority:** HIGH - Blocks animation, path following, dashed path rendering
**Downstream impact:** `Path1DEffect::apply()` (line 800-839) calls `PathMeasure::new`,
`measure.length()`, `measure.get_point_at()`, and `measure.get_tangent_at()` -- all
of which silently fail, making Path1DEffect non-functional as well.
**Test data:** skia/tests/PathMeasureTest.cpp has reference test cases

---

### GAP-C2: TrimEffect::apply() is a no-op

**Status:** RESOLVED (Task 14 - TrimEffect uses PathMeasure::get_segment)

**Location:** `src/effects.rs:594-603`
**Issue:** `TrimEffect::apply()` returns `Some(path.clone())` unconditionally -- it
never trims anything.
**Description:** The comment on line 596-597 acknowledges this: "This is a simplified
implementation / A full implementation would use PathMeasure". Since PathMeasure
itself is stubbed (GAP-C1), the trim effect has no way to compute which portion of
the path to keep. The struct correctly stores `start`, `end`, and `mode` fields, and
validation in `new()` is correct, but `apply()` does no actual work.

**Skia reference:** `skia/src/effects/SkTrimPathEffect.cpp`
**Estimated effort:** 1-2 days (once PathMeasure is implemented)
**Dependencies:** GAP-C1 (PathMeasure)
**Priority:** HIGH - Required for path animation workflows (e.g., drawing-on effects)

---

### GAP-C3: Path1DEffect::apply() silently fails due to PathMeasure stub

**Status:** RESOLVED (auto-fixed by GAP-C1 - Path1DEffect already uses PathMeasure)

**Location:** `src/effects.rs:798-845`
**Issue:** The implementation is structurally correct but non-functional because it
depends on PathMeasure (GAP-C1). `measure.length()` always returns `0.0`, so the
`while distance < length` loop (line 815) never executes, and the function returns
an empty path (Some(builder.build()) with nothing added).

**Estimated effort:** 0 (fix falls out of GAP-C1 implementation)
**Dependencies:** GAP-C1 (PathMeasure)
**Priority:** HIGH - stamps/repeating patterns along a path are a core drawing feature

---

### GAP-C4: Boolean ops `subtract_polygon()` is incomplete

**Status:** DEFERRED to dedicated boolean ops subplan (limitation documented in public API)

**Location:** `src/ops.rs:530-552`
**Issue:** The `subtract_polygon()` function only handles the trivial case where the
clip fully contains the subject (returns empty). For partial overlaps, it returns the
unmodified subject polygon (line 551: `vec![subject.clone()]`). The comments on lines
546-550 explicitly acknowledge this limitation: "For a proper implementation, we would:
1. Find all intersection points 2. Build a planar graph 3. Walk the graph to find
result polygons / For now, return subject if not fully contained."

**Impact:** `PathOp::Difference`, `PathOp::ReverseDifference`, and `PathOp::Xor` all
produce incorrect results when shapes partially overlap. Only fully-disjoint or
fully-contained cases work correctly.

**Skia reference:** `skia/src/pathops/SkPathOpsCurve.cpp` and related pathops files
(~15k lines total; Skia's pathops is one of its most complex subsystems)

**Algorithm required:** Full Weiler-Atherton or Greiner-Hormann polygon clipping,
or adaptation of Skia's sweep-line approach for robustness with curves.

**Estimated effort:** 2-3 weeks for polygon-based clipping; substantially more for
a robust implementation handling curves, degeneracies, and self-intersections
**Priority:** HIGH - Boolean operations are fundamental to vector graphics editing

---

### GAP-C5: `intersect_convex_polygons()` only works for convex polygons

**Status:** DEFERRED to dedicated boolean ops subplan (limitation documented in public API)

**Location:** `src/ops.rs:476-528`
**Issue:** The Sutherland-Hodgman algorithm used here only produces correct results
when both input polygons are convex. General paths (circles, bezier shapes, any
concave polygon) will produce incorrect intersection results when linearized into
concave polygons.

**Impact:** Any non-trivial `PathOp::Intersect` operation on real-world paths will
be incorrect.

**Skia reference:** Skia uses a full sweep-line approach in `skia/src/pathops/`
**Estimated effort:** Included in GAP-C4 (same underlying algorithm limitation)
**Priority:** HIGH

---

### GAP-C6: DashEffect does not handle curve segments correctly

**Status:** RESOLVED (Task 15 - DashEffect pre-flattens curves to lines)

**Location:** `src/effects.rs:224-260`
**Issue:** When the DashEffect encounters Quad, Conic, or Cubic path elements, it
subdivides them into fixed-step line approximations (8 steps for quads/conics, 12
for cubics) but does NOT apply the dash interval logic to those subdivided segments.
It only checks `if is_on` and draws all subdivision steps when on, skipping all when
off. This means dashes will never start or end in the middle of a curve segment --
the dash state only transitions at Line segment boundaries.

For short line segments the effect is passable, but for long curves a single dash
interval may need to start or stop partway through the curve.

**Skia reference:** `skia/src/effects/SkDashPathEffect.cpp` flattens curves first,
then applies dash logic to the resulting line segments uniformly.

**Estimated effort:** 3-5 days
**Priority:** MEDIUM-HIGH - Produces visibly incorrect output on curved paths

---

## Nice-to-Have Gaps

### GAP-N1: `Path::tight_bounds()` is identical to `bounds()`

**Status:** RESOLVED (Task 7 - tight_bounds uses quad/cubic extrema)

**Location:** `src/path.rs:513-517`
**Issue:** The comment says "For now, same as bounds (which already considers all
points)" but this is incorrect for curves. The tight bounds of a cubic Bezier curve
should be computed from the curve's extrema (roots of the derivative), not from the
control points. Control point bounds are an overestimate.

**Skia reference:** `SkPath::computeTightBounds()` in `skia/src/core/SkPath.cpp`
**Estimated effort:** 2-3 days (requires solving quadratic/cubic root finding)
**Priority:** LOW - bounds() is always a valid superset; tight_bounds() is an optimization

---

### GAP-N2: `Path::length()` uses control polygon approximation for curves

**Status:** DEFERRED (control-polygon approximation works for typical use)

**Location:** `src/path.rs:520-549`
**Issue:** For Quad/Conic curves, length is approximated as
`distance(start, ctrl) + distance(ctrl, end)` which is the control polygon length --
always an overestimate. For Cubic, it uses
`distance(start, c1) + distance(c1, c2) + distance(c2, end)`. A more accurate
approach would use adaptive subdivision or Gauss-Legendre quadrature.

**Estimated effort:** 2-3 days
**Priority:** LOW - The approximation is serviceable for many use cases

---

### GAP-N3: `Path::contains()` uses fixed-step curve linearization

**Status:** DEFERRED (fixed-step linearization works for typical use)

**Location:** `src/path.rs:445-500`
**Issue:** Conic containment check (lines 461-476) ignores the weight parameter,
treating conics as quadratic beziers. The approximation uses 8 steps for quads/conics
and 12 for cubics regardless of curve complexity. Adaptive subdivision based on
curvature would be more accurate.

**Estimated effort:** 1-2 days
**Priority:** LOW - Fixed-step approximation works for typical use cases

---

### GAP-N4: `Path::is_oval()` uses heuristic detection

**Status:** RESOLVED (Task 5 - verifies cardinal-point endpoints)

**Location:** `src/path.rs:278-285`
**Issue:** The check only counts verb types (4 cubics or 4 conics + move + close)
without verifying the control points actually form an ellipse. A path with 4 arbitrary
cubic segments and a close verb would incorrectly return `true`.

**Estimated effort:** 1 day
**Priority:** LOW - Rarely affects correctness in practice

---

### GAP-N5: `Path::convexity()` does not cache its result

**Status:** RESOLVED (Task 6 - cached via AtomicU8)

**Location:** `src/path.rs:288-319`
**Issue:** The method checks `self.convexity != PathConvexity::Unknown` at the start
but never stores the computed result back into `self.convexity`. The struct has the
field, and the method takes `&self` (not `&mut self`), so caching would require
interior mutability (Cell/RefCell) or changing the API. Each call recomputes from
scratch. Also, the convexity check only considers points, not the curve segments
between them.

**Estimated effort:** 1 day
**Priority:** LOW

---

### GAP-N6: Conic handling in `ops.rs` ignores weight

**Status:** RESOLVED (Task 4 - linearize_conic uses weighted rational form)

**Location:** `src/ops.rs:275-278`
**Issue:** When linearizing conics in `path_to_polygons()`, the weight parameter `w`
is captured but unused (compiler warning: `unused variable: w` on line 275). The conic
is treated as an unweighted quadratic. For weights far from 1.0 (e.g., exact circular
arcs with w = sqrt(2)/2), this produces noticeable geometric error.

**Estimated effort:** 1 day
**Priority:** LOW

---

### GAP-N7: `StrokeJoin::Round` in `path_utils.rs` is under-approximated

**Status:** RESOLVED (Task 3 - generates arc segments)

**Location:** `src/path_utils.rs:281-284`
**Issue:** The round join implementation uses a single averaged offset point instead of
generating an arc of line segments (as done for round caps on lines 390-406). This
produces a straight-line join rather than a curved one, defeating the purpose of
round joins.

**Estimated effort:** 1 day
**Priority:** MEDIUM - Round joins are visually common

---

### GAP-N8: `stroke_to_fill()` uses last contour's `is_closed` for all contours

**Status:** RESOLVED (Task 2 - is_closed tracked per contour)

**Location:** `src/path_utils.rs:109, 160-166`
**Issue:** The `is_closed` flag is updated in the iteration loop (line 149) but only
the final value is used when stroking all contours (line 165). If a path has multiple
contours with different open/closed states, only the last contour's state applies to
all of them. Each contour should track its own closed state.

**Estimated effort:** 0.5 days
**Priority:** MEDIUM - Bug for multi-contour paths

---

## Test Coverage Gaps

### Modules with zero tests:
- **path.rs** - 30 public functions, 0 tests. None of the following are tested:
  `bounds()`, `is_rect()`, `is_oval()`, `convexity()`, `direction()`, `reverse()`,
  `transform()`, `contains()`, `length()`, `tight_bounds()`, `iter()`
- **builder.rs** - 24 public functions, 0 tests. None of the builder methods
  (`move_to`, `line_to`, `quad_to`, `cubic_to`, `add_rect`, `add_oval`,
  `add_circle`, `add_round_rect`, `add_arc`, `arc_to`, relative methods, etc.)
  have any tests.
- **measure.rs** - 8 public functions, 0 tests. The entire stub is untested.

### Modules with minimal tests:
- **ops.rs** - 4 tests covering only: empty paths, non-overlapping union,
  non-overlapping intersect, and polygon contains_point. No tests for:
  overlapping operations, difference, xor, reverse-difference, curve linearization,
  or the polygon clipping algorithm.
- **effects.rs** - 6 tests covering only: DashEffect construction, odd interval
  doubling, CornerEffect construction, DiscreteEffect construction, make_*
  convenience functions, and compose effect. No tests for: `apply()` on any effect,
  TrimEffect, SumEffect, Path1DEffect, Path2DEffect, Line2DEffect, or actual
  path transformation correctness.
- **svg.rs** - 5 tests covering: simple M/L/Z, relative commands, C curves, arcs,
  H/V lines. No tests for: S (smooth cubic), T (smooth quad), error cases
  (malformed input, missing moveto, invalid numbers), edge cases (empty string,
  only whitespace, repeated commands).
- **path_utils.rs** - 3 tests: stroke_to_fill with a line, stroke_to_fill with
  a triangle, and StrokeParams builder. No tests for: round caps, square caps,
  miter limit fallback to bevel, curves in stroke input, multi-contour strokes.

### Estimated test coverage by function count:
- Functions with at least one test: ~15 / 106 (~14%)
- Functions with no test coverage: ~91 / 106 (~86%)

## Implementation Notes

### Unused dependencies
The `Cargo.toml` declares `thiserror`, `arrayvec`, and `proptest` (dev) as
dependencies but none are used in any source file. `serde` is feature-gated but
also unused. These should be cleaned up.

### Compiler warnings
The crate produces 4 warnings:
1. `unused import: Verb` in ops.rs:6
2. `variable does not need to be mutable` in effects.rs:142 (`interval_offset`)
3. `unused variable: w` in ops.rs:275 (conic weight -- see GAP-N6)
4. `field 'path' is never read` in measure.rs:9 (confirms PathMeasure stub)

### Cascade effect from PathMeasure stub
Three pieces of functionality are blocked by GAP-C1:
- `TrimEffect::apply()` (GAP-C2) -- explicitly deferred per comment
- `Path1DEffect::apply()` (GAP-C3) -- calls PathMeasure, silently returns empty
- Any future path-animation or path-following API

### Pattern: Fixed-step curve approximation
Multiple modules approximate curves with fixed subdivision steps (8 for quads, 12
for cubics) rather than adaptive flattening. This appears in: `path.rs` (contains),
`effects.rs` (DashEffect, DiscreteEffect for curves), `path_utils.rs` (flatten_quad,
flatten_cubic). The `ops.rs` module is the exception -- it uses adaptive tolerance-
based subdivision via `linearize_quad()` and `linearize_cubic()`. Unifying on
adaptive flattening across all modules would improve both accuracy and performance.

### Boolean ops architectural limitation
The current boolean ops implementation converts all paths to polygons, then operates
on the polygon level using simple containment checks and Sutherland-Hodgman clipping.
This approach fundamentally cannot handle concave polygons or partial overlaps
correctly. A production-quality implementation would need either:
- Weiler-Atherton algorithm for polygon clipping
- A sweep-line approach (Bentley-Ottmann) for robust intersection finding
- Or wrapping an existing robust library (e.g., `geo` crate with `BoolOps` trait)

## Recommendations

1. **Implement PathMeasure** (GAP-C1, 2-3 weeks) -- Highest priority. Unblocks
   GAP-C2 and GAP-C3 automatically. Start with linear segments, then add curve
   flattening with adaptive tolerance.

2. **Fix TrimEffect** (GAP-C2, 1-2 days) -- Quick win once PathMeasure ships.

3. **Fix DashEffect curve handling** (GAP-C6, 3-5 days) -- Flatten curves to lines
   before applying dash logic, matching Skia's approach.

4. **Fix stroke_to_fill multi-contour bug** (GAP-N8, 0.5 days) -- Track is_closed
   per contour. Quick correctness fix.

5. **Fix round join approximation** (GAP-N7, 1 day) -- Generate arc segments for
   round joins to match the round cap implementation.

6. **Improve boolean ops** (GAP-C4/C5, 2-3 weeks minimum) -- Most architecturally
   significant gap. Consider whether to build from scratch or integrate an existing
   polygon clipping library. Evaluate the `geo` crate's boolean ops as a potential
   dependency to avoid reimplementing Bentley-Ottmann.

7. **Add comprehensive tests** (~2 weeks) -- Current 14% test coverage is far below
   what is needed for a geometry library. Prioritize:
   - PathMeasure (as part of implementation)
   - Boolean ops correctness (overlapping shapes)
   - SVG parsing error cases
   - Path containment for all fill types
   - DashEffect on curved paths

8. **Clean up unused dependencies** (0.5 days) -- Remove thiserror, arrayvec from
   Cargo.toml; add proptest-based fuzz tests or remove proptest.

9. **Fix compiler warnings** (0.5 days) -- Address all 4 existing warnings.

10. **Unify curve flattening** (1-2 days) -- Extract the adaptive tolerance-based
    linearization from `ops.rs` into a shared utility and use it everywhere.
