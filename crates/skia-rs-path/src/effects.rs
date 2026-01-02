//! Path effects (dash, corner, discrete, etc.).
//!
//! Path effects modify how a path is stroked or filled. They can be applied
//! to create dashed lines, rounded corners, jittery edges, and more.

use crate::{Path, PathBuilder, PathElement};
use skia_rs_core::{Point, Scalar};
use std::sync::Arc;

/// A path effect that modifies how a path is stroked or filled.
///
/// Corresponds to Skia's `SkPathEffect`.
pub trait PathEffect: Send + Sync + std::fmt::Debug {
    /// Apply the effect to a path.
    fn apply(&self, path: &Path) -> Option<Path>;

    /// Get the kind of path effect for debugging.
    fn effect_kind(&self) -> PathEffectKind;
}

/// Kind of path effect (for debugging/inspection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathEffectKind {
    /// Dash effect.
    Dash,
    /// Corner rounding effect.
    Corner,
    /// Discrete/jitter effect.
    Discrete,
    /// 1D path effect (stamps a path along another).
    Path1D,
    /// 2D path effect (tiles a path in 2D).
    Path2D,
    /// Line 2D effect.
    Line2D,
    /// Trim effect.
    Trim,
    /// Composed effect (one after another).
    Compose,
    /// Sum effect (both applied, results combined).
    Sum,
}

/// A boxed path effect.
pub type PathEffectRef = Arc<dyn PathEffect>;

// =============================================================================
// Dash Effect
// =============================================================================

/// Dash effect that creates dashed/dotted lines.
///
/// Corresponds to Skia's `SkDashPathEffect`.
#[derive(Debug, Clone)]
pub struct DashEffect {
    /// Dash intervals (on, off, on, off, ...).
    intervals: Vec<Scalar>,
    /// Phase offset into the dash pattern.
    phase: Scalar,
    /// Sum of all intervals (cached).
    interval_sum: Scalar,
}

impl DashEffect {
    /// Create a new dash effect.
    ///
    /// `intervals` must have an even number of entries (on/off pairs).
    /// If odd, the pattern is duplicated to make it even.
    pub fn new(intervals: Vec<Scalar>, phase: Scalar) -> Option<Self> {
        if intervals.is_empty() {
            return None;
        }

        // Ensure even number of intervals
        let intervals = if intervals.len() % 2 != 0 {
            let mut doubled = intervals.clone();
            doubled.extend(intervals.iter().cloned());
            doubled
        } else {
            intervals
        };

        // Validate intervals are positive
        for &interval in &intervals {
            if interval < 0.0 {
                return None;
            }
        }

        let interval_sum: Scalar = intervals.iter().sum();
        if interval_sum <= 0.0 {
            return None;
        }

        Some(Self {
            intervals,
            phase,
            interval_sum,
        })
    }

    /// Create a simple dash pattern (dash length, gap length).
    pub fn simple(dash: Scalar, gap: Scalar) -> Option<Self> {
        Self::new(vec![dash, gap], 0.0)
    }

    /// Create a dotted pattern.
    pub fn dotted(dot_size: Scalar, gap: Scalar) -> Option<Self> {
        Self::new(vec![dot_size, gap], 0.0)
    }

    /// Get the intervals.
    pub fn intervals(&self) -> &[Scalar] {
        &self.intervals
    }

    /// Get the phase.
    pub fn phase(&self) -> Scalar {
        self.phase
    }
}

impl PathEffect for DashEffect {
    fn apply(&self, path: &Path) -> Option<Path> {
        if path.is_empty() {
            return None;
        }

        let mut builder = PathBuilder::new();
        let mut current_pos = Point::zero();
        let mut contour_start = Point::zero();
        #[allow(unused_assignments)]
        let mut _distance_along_contour = 0.0_f32;

        // Adjust phase to be within the interval sum
        let mut phase = self.phase % self.interval_sum;
        if phase < 0.0 {
            phase += self.interval_sum;
        }

        // Find starting interval index and offset
        let (mut interval_idx, mut interval_offset) = {
            let mut accumulated = 0.0;
            let mut idx = 0;
            while accumulated + self.intervals[idx] <= phase {
                accumulated += self.intervals[idx];
                idx = (idx + 1) % self.intervals.len();
            }
            (idx, phase - accumulated)
        };

        let mut is_on = interval_idx % 2 == 0;
        let mut remaining_in_interval = self.intervals[interval_idx] - interval_offset;

        for element in path.iter() {
            match element {
                PathElement::Move(p) => {
                    current_pos = p;
                    contour_start = p;
                    _distance_along_contour = 0.0;
                    // Reset dash state for new contour
                    let mut accumulated = 0.0;
                    interval_idx = 0;
                    while accumulated + self.intervals[interval_idx] <= phase {
                        accumulated += self.intervals[interval_idx];
                        interval_idx = (interval_idx + 1) % self.intervals.len();
                    }
                    is_on = interval_idx % 2 == 0;
                    remaining_in_interval = self.intervals[interval_idx] - (phase - accumulated);
                    if is_on {
                        builder.move_to(p.x, p.y);
                    }
                }
                PathElement::Line(end) => {
                    let segment_length = current_pos.distance(&end);
                    let mut t = 0.0;

                    while t < 1.0 {
                        let remaining_segment = segment_length * (1.0 - t);

                        if remaining_in_interval >= remaining_segment {
                            // Finish this segment
                            if is_on {
                                builder.line_to(end.x, end.y);
                            }
                            remaining_in_interval -= remaining_segment;
                            t = 1.0;
                        } else {
                            // Partial segment
                            let dt = remaining_in_interval / segment_length;
                            let mid = Point::new(
                                current_pos.x + (end.x - current_pos.x) * (t + dt),
                                current_pos.y + (end.y - current_pos.y) * (t + dt),
                            );

                            if is_on {
                                builder.line_to(mid.x, mid.y);
                            }

                            // Move to next interval
                            interval_idx = (interval_idx + 1) % self.intervals.len();
                            is_on = interval_idx % 2 == 0;
                            remaining_in_interval = self.intervals[interval_idx];

                            if is_on {
                                builder.move_to(mid.x, mid.y);
                            }

                            t += dt;
                        }
                    }

                    current_pos = end;
                }
                PathElement::Close => {
                    // Handle closing line if needed
                    if current_pos != contour_start {
                        let segment_length = current_pos.distance(&contour_start);
                        if segment_length > 0.0 && is_on {
                            builder.line_to(contour_start.x, contour_start.y);
                        }
                    }
                }
                // For curves, we approximate with lines (simplified)
                PathElement::Quad(ctrl, end) => {
                    // Subdivide and treat as lines
                    let steps = 8;
                    for i in 1..=steps {
                        let t = i as Scalar / steps as Scalar;
                        let p = quadratic_point(current_pos, ctrl, end, t);
                        if is_on {
                            builder.line_to(p.x, p.y);
                        }
                    }
                    current_pos = end;
                }
                PathElement::Conic(ctrl, end, _weight) => {
                    // Approximate as quad
                    let steps = 8;
                    for i in 1..=steps {
                        let t = i as Scalar / steps as Scalar;
                        let p = quadratic_point(current_pos, ctrl, end, t);
                        if is_on {
                            builder.line_to(p.x, p.y);
                        }
                    }
                    current_pos = end;
                }
                PathElement::Cubic(ctrl1, ctrl2, end) => {
                    let steps = 12;
                    for i in 1..=steps {
                        let t = i as Scalar / steps as Scalar;
                        let p = cubic_point(current_pos, ctrl1, ctrl2, end, t);
                        if is_on {
                            builder.line_to(p.x, p.y);
                        }
                    }
                    current_pos = end;
                }
            }
        }

        Some(builder.build())
    }

    fn effect_kind(&self) -> PathEffectKind {
        PathEffectKind::Dash
    }
}

// =============================================================================
// Corner Effect
// =============================================================================

/// Corner path effect that rounds corners.
///
/// Corresponds to Skia's `SkCornerPathEffect`.
#[derive(Debug, Clone, Copy)]
pub struct CornerEffect {
    /// Radius for corner rounding.
    radius: Scalar,
}

impl CornerEffect {
    /// Create a new corner effect.
    pub fn new(radius: Scalar) -> Option<Self> {
        if radius <= 0.0 {
            return None;
        }
        Some(Self { radius })
    }

    /// Get the radius.
    pub fn radius(&self) -> Scalar {
        self.radius
    }
}

impl PathEffect for CornerEffect {
    fn apply(&self, path: &Path) -> Option<Path> {
        if path.is_empty() {
            return None;
        }

        let mut builder = PathBuilder::new();
        let elements: Vec<_> = path.iter().collect();

        let mut i = 0;
        while i < elements.len() {
            match elements[i] {
                PathElement::Move(p) => {
                    builder.move_to(p.x, p.y);
                    i += 1;
                }
                PathElement::Line(end) => {
                    // Look ahead for corner
                    let prev_end = if i > 0 {
                        get_end_point(&elements[i - 1])
                    } else {
                        None
                    };

                    let next_start = if i + 1 < elements.len() {
                        get_end_point(&elements[i + 1])
                    } else {
                        None
                    };

                    if let (Some(start), Some(next)) = (prev_end, next_start) {
                        // Round the corner at 'end'
                        let v1 = Point::new(start.x - end.x, start.y - end.y);
                        let v2 = Point::new(next.x - end.x, next.y - end.y);

                        let len1 = v1.length();
                        let len2 = v2.length();

                        if len1 > 0.0 && len2 > 0.0 {
                            let radius = self.radius.min(len1 / 2.0).min(len2 / 2.0);

                            let t1 = Point::new(
                                end.x + v1.x / len1 * radius,
                                end.y + v1.y / len1 * radius,
                            );
                            let t2 = Point::new(
                                end.x + v2.x / len2 * radius,
                                end.y + v2.y / len2 * radius,
                            );

                            builder.line_to(t1.x, t1.y);
                            builder.quad_to(end.x, end.y, t2.x, t2.y);
                        } else {
                            builder.line_to(end.x, end.y);
                        }
                    } else {
                        builder.line_to(end.x, end.y);
                    }
                    i += 1;
                }
                PathElement::Quad(ctrl, end) => {
                    builder.quad_to(ctrl.x, ctrl.y, end.x, end.y);
                    i += 1;
                }
                PathElement::Conic(ctrl, end, w) => {
                    builder.conic_to(ctrl.x, ctrl.y, end.x, end.y, w);
                    i += 1;
                }
                PathElement::Cubic(ctrl1, ctrl2, end) => {
                    builder.cubic_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, end.x, end.y);
                    i += 1;
                }
                PathElement::Close => {
                    builder.close();
                    i += 1;
                }
            }
        }

        Some(builder.build())
    }

    fn effect_kind(&self) -> PathEffectKind {
        PathEffectKind::Corner
    }
}

// =============================================================================
// Discrete Effect
// =============================================================================

/// Discrete path effect that adds random displacement (jitter).
///
/// Corresponds to Skia's `SkDiscretePathEffect`.
#[derive(Debug, Clone, Copy)]
pub struct DiscreteEffect {
    /// Segment length for subdividing the path.
    seg_length: Scalar,
    /// Maximum deviation from the original path.
    deviation: Scalar,
    /// Random seed.
    seed: u32,
}

impl DiscreteEffect {
    /// Create a new discrete effect.
    pub fn new(seg_length: Scalar, deviation: Scalar, seed: u32) -> Option<Self> {
        if seg_length <= 0.0 {
            return None;
        }
        Some(Self {
            seg_length,
            deviation,
            seed,
        })
    }

    /// Get the segment length.
    pub fn seg_length(&self) -> Scalar {
        self.seg_length
    }

    /// Get the deviation.
    pub fn deviation(&self) -> Scalar {
        self.deviation
    }

    /// Simple pseudo-random number generator.
    fn random(&self, seed: u32) -> Scalar {
        // Simple LCG
        let n = seed.wrapping_mul(1103515245).wrapping_add(12345);
        ((n >> 16) & 0x7FFF) as Scalar / 32767.0 * 2.0 - 1.0
    }
}

impl PathEffect for DiscreteEffect {
    fn apply(&self, path: &Path) -> Option<Path> {
        if path.is_empty() {
            return None;
        }

        let mut builder = PathBuilder::new();
        let mut current_pos = Point::zero();
        let mut seed = self.seed;

        for element in path.iter() {
            match element {
                PathElement::Move(p) => {
                    let dx = self.random(seed) * self.deviation;
                    seed = seed.wrapping_add(1);
                    let dy = self.random(seed) * self.deviation;
                    seed = seed.wrapping_add(1);
                    builder.move_to(p.x + dx, p.y + dy);
                    current_pos = p;
                }
                PathElement::Line(end) => {
                    let length = current_pos.distance(&end);
                    let num_segments = (length / self.seg_length).ceil() as usize;
                    let num_segments = num_segments.max(1);

                    for i in 1..=num_segments {
                        let t = i as Scalar / num_segments as Scalar;
                        let x = current_pos.x + (end.x - current_pos.x) * t;
                        let y = current_pos.y + (end.y - current_pos.y) * t;

                        let dx = self.random(seed) * self.deviation;
                        seed = seed.wrapping_add(1);
                        let dy = self.random(seed) * self.deviation;
                        seed = seed.wrapping_add(1);

                        builder.line_to(x + dx, y + dy);
                    }
                    current_pos = end;
                }
                PathElement::Close => {
                    builder.close();
                }
                // For curves, subdivide into lines first
                PathElement::Quad(ctrl, end) => {
                    let steps = (quadratic_length(current_pos, ctrl, end) / self.seg_length).ceil()
                        as usize;
                    let steps = steps.max(4);

                    for i in 1..=steps {
                        let t = i as Scalar / steps as Scalar;
                        let p = quadratic_point(current_pos, ctrl, end, t);

                        let dx = self.random(seed) * self.deviation;
                        seed = seed.wrapping_add(1);
                        let dy = self.random(seed) * self.deviation;
                        seed = seed.wrapping_add(1);

                        builder.line_to(p.x + dx, p.y + dy);
                    }
                    current_pos = end;
                }
                PathElement::Conic(ctrl, end, _w) => {
                    let steps = (quadratic_length(current_pos, ctrl, end) / self.seg_length).ceil()
                        as usize;
                    let steps = steps.max(4);

                    for i in 1..=steps {
                        let t = i as Scalar / steps as Scalar;
                        let p = quadratic_point(current_pos, ctrl, end, t);

                        let dx = self.random(seed) * self.deviation;
                        seed = seed.wrapping_add(1);
                        let dy = self.random(seed) * self.deviation;
                        seed = seed.wrapping_add(1);

                        builder.line_to(p.x + dx, p.y + dy);
                    }
                    current_pos = end;
                }
                PathElement::Cubic(ctrl1, ctrl2, end) => {
                    let steps = (cubic_length(current_pos, ctrl1, ctrl2, end) / self.seg_length)
                        .ceil() as usize;
                    let steps = steps.max(4);

                    for i in 1..=steps {
                        let t = i as Scalar / steps as Scalar;
                        let p = cubic_point(current_pos, ctrl1, ctrl2, end, t);

                        let dx = self.random(seed) * self.deviation;
                        seed = seed.wrapping_add(1);
                        let dy = self.random(seed) * self.deviation;
                        seed = seed.wrapping_add(1);

                        builder.line_to(p.x + dx, p.y + dy);
                    }
                    current_pos = end;
                }
            }
        }

        Some(builder.build())
    }

    fn effect_kind(&self) -> PathEffectKind {
        PathEffectKind::Discrete
    }
}

// =============================================================================
// Trim Effect
// =============================================================================

/// Trim effect that shows only a portion of the path.
///
/// Corresponds to Skia's `SkTrimPathEffect`.
#[derive(Debug, Clone, Copy)]
pub struct TrimEffect {
    /// Start of the visible portion (0.0 - 1.0).
    start: Scalar,
    /// End of the visible portion (0.0 - 1.0).
    end: Scalar,
    /// Trim mode.
    mode: TrimMode,
}

/// Trim mode for path effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TrimMode {
    /// Normal trim (from start to end).
    #[default]
    Normal,
    /// Inverted trim (everything except start to end).
    Inverted,
}

impl TrimEffect {
    /// Create a new trim effect.
    pub fn new(start: Scalar, end: Scalar, mode: TrimMode) -> Option<Self> {
        if start < 0.0 || start > 1.0 || end < 0.0 || end > 1.0 {
            return None;
        }
        Some(Self { start, end, mode })
    }

    /// Get the start position.
    pub fn start(&self) -> Scalar {
        self.start
    }

    /// Get the end position.
    pub fn end(&self) -> Scalar {
        self.end
    }

    /// Get the trim mode.
    pub fn mode(&self) -> TrimMode {
        self.mode
    }
}

impl PathEffect for TrimEffect {
    fn apply(&self, path: &Path) -> Option<Path> {
        // This is a simplified implementation
        // A full implementation would use PathMeasure
        Some(path.clone())
    }

    fn effect_kind(&self) -> PathEffectKind {
        PathEffectKind::Trim
    }
}

// =============================================================================
// Compose Effect
// =============================================================================

/// Compose effect that applies one effect, then another.
///
/// Corresponds to Skia's `SkPathEffect::MakeCompose`.
#[derive(Debug)]
pub struct ComposeEffect {
    outer: PathEffectRef,
    inner: PathEffectRef,
}

impl ComposeEffect {
    /// Create a composed effect: outer(inner(path)).
    pub fn new(outer: PathEffectRef, inner: PathEffectRef) -> Self {
        Self { outer, inner }
    }
}

impl PathEffect for ComposeEffect {
    fn apply(&self, path: &Path) -> Option<Path> {
        let intermediate = self.inner.apply(path)?;
        self.outer.apply(&intermediate)
    }

    fn effect_kind(&self) -> PathEffectKind {
        PathEffectKind::Compose
    }
}

// =============================================================================
// Sum Effect
// =============================================================================

/// Sum effect that applies both effects and combines the results.
///
/// Corresponds to Skia's `SkPathEffect::MakeSum`.
#[derive(Debug)]
pub struct SumEffect {
    first: PathEffectRef,
    second: PathEffectRef,
}

impl SumEffect {
    /// Create a sum effect.
    pub fn new(first: PathEffectRef, second: PathEffectRef) -> Self {
        Self { first, second }
    }
}

impl PathEffect for SumEffect {
    fn apply(&self, path: &Path) -> Option<Path> {
        let path1 = self.first.apply(path);
        let path2 = self.second.apply(path);

        match (path1, path2) {
            (Some(p1), Some(p2)) => {
                // Combine the paths
                let mut builder = PathBuilder::new();
                builder.add_path(&p1);
                builder.add_path(&p2);
                Some(builder.build())
            }
            (Some(p), None) | (None, Some(p)) => Some(p),
            (None, None) => None,
        }
    }

    fn effect_kind(&self) -> PathEffectKind {
        PathEffectKind::Sum
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn get_end_point(element: &PathElement) -> Option<Point> {
    match element {
        PathElement::Move(p) => Some(*p),
        PathElement::Line(p) => Some(*p),
        PathElement::Quad(_, p) => Some(*p),
        PathElement::Conic(_, p, _) => Some(*p),
        PathElement::Cubic(_, _, p) => Some(*p),
        PathElement::Close => None,
    }
}

fn quadratic_point(p0: Point, p1: Point, p2: Point, t: Scalar) -> Point {
    let mt = 1.0 - t;
    Point::new(
        mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
        mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
    )
}

fn cubic_point(p0: Point, p1: Point, p2: Point, p3: Point, t: Scalar) -> Point {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let t2 = t * t;
    Point::new(
        mt2 * mt * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t2 * t * p3.x,
        mt2 * mt * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t2 * t * p3.y,
    )
}

fn quadratic_length(p0: Point, p1: Point, p2: Point) -> Scalar {
    // Approximate with chord + control polygon
    let chord = p0.distance(&p2);
    let polygon = p0.distance(&p1) + p1.distance(&p2);
    (chord + polygon) / 2.0
}

fn cubic_length(p0: Point, p1: Point, p2: Point, p3: Point) -> Scalar {
    // Approximate with chord + control polygon
    let chord = p0.distance(&p3);
    let polygon = p0.distance(&p1) + p1.distance(&p2) + p2.distance(&p3);
    (chord + polygon) / 2.0
}

// =============================================================================
// Path1D Effect
// =============================================================================

/// Style for how the stamped path is placed along the input path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Path1DStyle {
    /// Translate the path shape.
    #[default]
    Translate,
    /// Rotate the path shape to follow the path direction.
    Rotate,
    /// Morph the path shape to follow the path curvature.
    Morph,
}

/// 1D path effect that stamps a path along another path.
///
/// Corresponds to Skia's `SkPath1DPathEffect`.
#[derive(Debug, Clone)]
pub struct Path1DEffect {
    /// The path to stamp.
    path: Path,
    /// Distance between stamps (advance).
    advance: Scalar,
    /// Initial phase offset.
    phase: Scalar,
    /// How to place the stamped path.
    style: Path1DStyle,
}

impl Path1DEffect {
    /// Create a new 1D path effect.
    ///
    /// - `path`: The path to stamp along the input path.
    /// - `advance`: Distance between each stamp.
    /// - `phase`: Initial offset along the path.
    /// - `style`: How to orient the stamped path.
    pub fn new(path: Path, advance: Scalar, phase: Scalar, style: Path1DStyle) -> Option<Self> {
        if advance <= 0.0 || path.is_empty() {
            return None;
        }
        Some(Self {
            path,
            advance,
            phase,
            style,
        })
    }

    /// Get the stamped path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the advance distance.
    pub fn advance(&self) -> Scalar {
        self.advance
    }

    /// Get the phase.
    pub fn phase(&self) -> Scalar {
        self.phase
    }

    /// Get the style.
    pub fn style(&self) -> Path1DStyle {
        self.style
    }
}

impl PathEffect for Path1DEffect {
    fn apply(&self, src_path: &Path) -> Option<Path> {
        use crate::PathMeasure;

        if src_path.is_empty() {
            return None;
        }

        let mut builder = PathBuilder::new();
        let measure = PathMeasure::new(src_path);
        let length = measure.length();

        if length <= 0.0 {
            return None;
        }

        let mut distance = self.phase;
        while distance < length {
            // Get position and tangent at this distance
            let pos = measure.get_point_at(distance);
            let tangent = measure.get_tangent_at(distance);

            if let (Some(pos), Some(tangent)) = (pos, tangent) {
                let transform = match self.style {
                    Path1DStyle::Translate => skia_rs_core::Matrix::translate(pos.x, pos.y),
                    Path1DStyle::Rotate | Path1DStyle::Morph => {
                        let angle = tangent.y.atan2(tangent.x);
                        // First rotate, then translate
                        let rotation = skia_rs_core::Matrix::rotate(angle);
                        let translation = skia_rs_core::Matrix::translate(pos.x, pos.y);
                        translation.concat(&rotation)
                    }
                };

                // Transform and add the stamped path
                let transformed = self.path.transformed(&transform);
                builder.add_path(&transformed);
            }
            distance += self.advance;
        }

        Some(builder.build())
    }

    fn effect_kind(&self) -> PathEffectKind {
        PathEffectKind::Path1D
    }
}

// =============================================================================
// Path2D Effect
// =============================================================================

/// 2D path effect that tiles a path in a 2D pattern.
///
/// Corresponds to Skia's `SkPath2DPathEffect`.
#[derive(Debug, Clone)]
pub struct Path2DEffect {
    /// The transformation matrix for the tile.
    matrix: skia_rs_core::Matrix,
    /// The path to tile.
    path: Path,
}

impl Path2DEffect {
    /// Create a new 2D path effect.
    ///
    /// - `matrix`: The matrix defining the tiling pattern.
    /// - `path`: The path to tile.
    pub fn new(matrix: skia_rs_core::Matrix, path: Path) -> Option<Self> {
        if path.is_empty() {
            return None;
        }
        Some(Self { matrix, path })
    }

    /// Get the matrix.
    pub fn matrix(&self) -> &skia_rs_core::Matrix {
        &self.matrix
    }

    /// Get the tiled path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl PathEffect for Path2DEffect {
    fn apply(&self, src_path: &Path) -> Option<Path> {
        if src_path.is_empty() {
            return None;
        }

        let bounds = src_path.bounds();
        if bounds.is_empty() {
            return None;
        }
        let mut builder = PathBuilder::new();

        // Compute the inverse matrix to find tile positions
        let inverse = self.matrix.invert()?;

        // Transform bounds to tile space
        let transformed_bounds = inverse.map_rect(&bounds);

        // Compute tile range
        let start_x = transformed_bounds.left.floor() as i32 - 1;
        let end_x = transformed_bounds.right.ceil() as i32 + 1;
        let start_y = transformed_bounds.top.floor() as i32 - 1;
        let end_y = transformed_bounds.bottom.ceil() as i32 + 1;

        // Limit iterations for safety
        let max_tiles = 1000;
        let mut tile_count = 0;

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if tile_count >= max_tiles {
                    break;
                }

                // Transform tile to world space
                let tile_offset = skia_rs_core::Matrix::translate(x as Scalar, y as Scalar);
                let tile_matrix = self.matrix.concat(&tile_offset);
                let transformed_path = self.path.transformed(&tile_matrix);

                builder.add_path(&transformed_path);
                tile_count += 1;
            }
        }

        Some(builder.build())
    }

    fn effect_kind(&self) -> PathEffectKind {
        PathEffectKind::Path2D
    }
}

// =============================================================================
// Line2D Effect
// =============================================================================

/// 2D line effect that fills a region with parallel lines.
///
/// Corresponds to Skia's `SkLine2DPathEffect`.
#[derive(Debug, Clone, Copy)]
pub struct Line2DEffect {
    /// Line width.
    width: Scalar,
    /// The transformation matrix for the line pattern.
    matrix: skia_rs_core::Matrix,
}

impl Line2DEffect {
    /// Create a new 2D line effect.
    ///
    /// - `width`: The width of the lines.
    /// - `matrix`: The matrix defining the line pattern orientation and spacing.
    pub fn new(width: Scalar, matrix: skia_rs_core::Matrix) -> Option<Self> {
        if width <= 0.0 {
            return None;
        }
        Some(Self { width, matrix })
    }

    /// Get the line width.
    pub fn width(&self) -> Scalar {
        self.width
    }

    /// Get the matrix.
    pub fn matrix(&self) -> &skia_rs_core::Matrix {
        &self.matrix
    }
}

impl PathEffect for Line2DEffect {
    fn apply(&self, src_path: &Path) -> Option<Path> {
        if src_path.is_empty() {
            return None;
        }

        let bounds = src_path.bounds();
        if bounds.is_empty() {
            return None;
        }
        let mut builder = PathBuilder::new();

        // Compute the inverse matrix
        let inverse = self.matrix.invert()?;

        // Transform bounds to line space
        let transformed_bounds = inverse.map_rect(&bounds);

        // Generate horizontal lines in transformed space
        let start_y = transformed_bounds.top.floor() as i32 - 1;
        let end_y = transformed_bounds.bottom.ceil() as i32 + 1;

        // Limit iterations for safety
        let max_lines = 500;
        let mut line_count = 0;

        for y in start_y..=end_y {
            if line_count >= max_lines {
                break;
            }

            // Create a line from left to right
            let y_pos = y as Scalar;
            let p0 = Point::new(transformed_bounds.left - 10.0, y_pos);
            let p1 = Point::new(transformed_bounds.right + 10.0, y_pos);

            // Transform back to world space
            let world_p0 = self.matrix.map_point(p0);
            let world_p1 = self.matrix.map_point(p1);

            // Create a rectangle for the line (with width)
            let half_width = self.width / 2.0;
            let dx = world_p1.x - world_p0.x;
            let dy = world_p1.y - world_p0.y;
            let len = (dx * dx + dy * dy).sqrt();

            if len > 0.0 {
                let nx = -dy / len * half_width;
                let ny = dx / len * half_width;

                builder.move_to(world_p0.x + nx, world_p0.y + ny);
                builder.line_to(world_p1.x + nx, world_p1.y + ny);
                builder.line_to(world_p1.x - nx, world_p1.y - ny);
                builder.line_to(world_p0.x - nx, world_p0.y - ny);
                builder.close();
            }

            line_count += 1;
        }

        Some(builder.build())
    }

    fn effect_kind(&self) -> PathEffectKind {
        PathEffectKind::Line2D
    }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/// Create a dash path effect.
pub fn make_dash(intervals: Vec<Scalar>, phase: Scalar) -> Option<PathEffectRef> {
    DashEffect::new(intervals, phase).map(|e| Arc::new(e) as PathEffectRef)
}

/// Create a corner path effect.
pub fn make_corner(radius: Scalar) -> Option<PathEffectRef> {
    CornerEffect::new(radius).map(|e| Arc::new(e) as PathEffectRef)
}

/// Create a discrete path effect.
pub fn make_discrete(seg_length: Scalar, deviation: Scalar, seed: u32) -> Option<PathEffectRef> {
    DiscreteEffect::new(seg_length, deviation, seed).map(|e| Arc::new(e) as PathEffectRef)
}

/// Create a trim path effect.
pub fn make_trim(start: Scalar, end: Scalar, mode: TrimMode) -> Option<PathEffectRef> {
    TrimEffect::new(start, end, mode).map(|e| Arc::new(e) as PathEffectRef)
}

/// Compose two path effects.
pub fn make_compose(outer: PathEffectRef, inner: PathEffectRef) -> PathEffectRef {
    Arc::new(ComposeEffect::new(outer, inner))
}

/// Sum two path effects.
pub fn make_sum(first: PathEffectRef, second: PathEffectRef) -> PathEffectRef {
    Arc::new(SumEffect::new(first, second))
}

/// Create a 1D path effect.
pub fn make_path_1d(
    path: Path,
    advance: Scalar,
    phase: Scalar,
    style: Path1DStyle,
) -> Option<PathEffectRef> {
    Path1DEffect::new(path, advance, phase, style).map(|e| Arc::new(e) as PathEffectRef)
}

/// Create a 2D path effect.
pub fn make_path_2d(matrix: skia_rs_core::Matrix, path: Path) -> Option<PathEffectRef> {
    Path2DEffect::new(matrix, path).map(|e| Arc::new(e) as PathEffectRef)
}

/// Create a 2D line effect.
pub fn make_line_2d(width: Scalar, matrix: skia_rs_core::Matrix) -> Option<PathEffectRef> {
    Line2DEffect::new(width, matrix).map(|e| Arc::new(e) as PathEffectRef)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dash_effect() {
        let dash = DashEffect::new(vec![10.0, 5.0], 0.0).unwrap();
        assert_eq!(dash.intervals().len(), 2);
        assert_eq!(dash.phase(), 0.0);
    }

    #[test]
    fn test_dash_odd_intervals() {
        let dash = DashEffect::new(vec![10.0, 5.0, 3.0], 0.0).unwrap();
        // Should be doubled to make even
        assert_eq!(dash.intervals().len(), 6);
    }

    #[test]
    fn test_corner_effect() {
        let corner = CornerEffect::new(5.0).unwrap();
        assert_eq!(corner.radius(), 5.0);
    }

    #[test]
    fn test_discrete_effect() {
        let discrete = DiscreteEffect::new(10.0, 5.0, 42).unwrap();
        assert_eq!(discrete.seg_length(), 10.0);
        assert_eq!(discrete.deviation(), 5.0);
    }

    #[test]
    fn test_make_functions() {
        assert!(make_dash(vec![10.0, 5.0], 0.0).is_some());
        assert!(make_corner(5.0).is_some());
        assert!(make_discrete(10.0, 5.0, 0).is_some());
        assert!(make_trim(0.0, 1.0, TrimMode::Normal).is_some());
    }

    #[test]
    fn test_compose_effect() {
        let dash = make_dash(vec![10.0, 5.0], 0.0).unwrap();
        let corner = make_corner(5.0).unwrap();
        let composed = make_compose(dash, corner);
        assert_eq!(composed.effect_kind(), PathEffectKind::Compose);
    }
}
