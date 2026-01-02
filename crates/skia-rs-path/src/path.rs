//! Path data structure and iteration.

use skia_rs_core::{Point, Rect, Scalar};
use smallvec::SmallVec;

/// Path fill type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FillType {
    /// Non-zero winding rule.
    #[default]
    Winding = 0,
    /// Even-odd rule.
    EvenOdd,
    /// Inverse non-zero winding.
    InverseWinding,
    /// Inverse even-odd.
    InverseEvenOdd,
}

impl FillType {
    /// Check if this is an inverse fill type.
    #[inline]
    pub const fn is_inverse(&self) -> bool {
        matches!(self, FillType::InverseWinding | FillType::InverseEvenOdd)
    }

    /// Convert to the inverse fill type.
    #[inline]
    pub const fn inverse(&self) -> Self {
        match self {
            FillType::Winding => FillType::InverseWinding,
            FillType::EvenOdd => FillType::InverseEvenOdd,
            FillType::InverseWinding => FillType::Winding,
            FillType::InverseEvenOdd => FillType::EvenOdd,
        }
    }
}

/// Path verb (command type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Verb {
    /// Move to a point.
    Move = 0,
    /// Line to a point.
    Line,
    /// Quadratic bezier.
    Quad,
    /// Conic (weighted quadratic).
    Conic,
    /// Cubic bezier.
    Cubic,
    /// Close the current contour.
    Close,
}

impl Verb {
    /// Number of points consumed by this verb.
    #[inline]
    pub const fn point_count(&self) -> usize {
        match self {
            Verb::Move | Verb::Line => 1,
            Verb::Quad | Verb::Conic => 2,
            Verb::Cubic => 3,
            Verb::Close => 0,
        }
    }
}

/// Path direction (clockwise or counter-clockwise).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PathDirection {
    /// Clockwise direction.
    #[default]
    CW = 0,
    /// Counter-clockwise direction.
    CCW,
}

/// Path convexity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PathConvexity {
    /// Unknown convexity.
    #[default]
    Unknown = 0,
    /// Path is convex.
    Convex,
    /// Path is concave.
    Concave,
}

/// A 2D geometric path.
#[derive(Debug, Clone, Default)]
pub struct Path {
    /// Path verbs.
    pub(crate) verbs: SmallVec<[Verb; 16]>,
    /// Path points.
    pub(crate) points: SmallVec<[Point; 32]>,
    /// Conic weights.
    pub(crate) conic_weights: SmallVec<[Scalar; 4]>,
    /// Fill type.
    pub(crate) fill_type: FillType,
    /// Cached bounds (lazily computed).
    pub(crate) bounds: Option<Rect>,
    /// Cached convexity.
    pub(crate) convexity: PathConvexity,
}

impl Path {
    /// Create a new empty path.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the fill type.
    #[inline]
    pub fn fill_type(&self) -> FillType {
        self.fill_type
    }

    /// Set the fill type.
    #[inline]
    pub fn set_fill_type(&mut self, fill_type: FillType) {
        self.fill_type = fill_type;
    }

    /// Check if the path is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.verbs.is_empty()
    }

    /// Get the number of verbs.
    #[inline]
    pub fn verb_count(&self) -> usize {
        self.verbs.len()
    }

    /// Get the number of points.
    #[inline]
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Get the bounds of the path.
    pub fn bounds(&self) -> Rect {
        if let Some(bounds) = self.bounds {
            return bounds;
        }

        if self.points.is_empty() {
            return Rect::EMPTY;
        }

        let mut min_x = self.points[0].x;
        let mut min_y = self.points[0].y;
        let mut max_x = min_x;
        let mut max_y = min_y;

        for p in &self.points[1..] {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }

        Rect::new(min_x, min_y, max_x, max_y)
    }

    /// Clear the path.
    #[inline]
    pub fn reset(&mut self) {
        self.verbs.clear();
        self.points.clear();
        self.conic_weights.clear();
        self.bounds = None;
    }

    /// Iterate over the path elements.
    pub fn iter(&self) -> PathIter<'_> {
        PathIter {
            path: self,
            verb_index: 0,
            point_index: 0,
            weight_index: 0,
        }
    }

    /// Get the verbs slice.
    #[inline]
    pub fn verbs(&self) -> &[Verb] {
        &self.verbs
    }

    /// Get the points slice.
    #[inline]
    pub fn points(&self) -> &[Point] {
        &self.points
    }

    /// Get the last point in the path.
    #[inline]
    pub fn last_point(&self) -> Option<Point> {
        self.points.last().copied()
    }

    /// Get the number of contours in the path.
    pub fn contour_count(&self) -> usize {
        self.verbs.iter().filter(|v| **v == Verb::Move).count()
    }

    /// Check if the path is closed.
    pub fn is_closed(&self) -> bool {
        self.verbs.last() == Some(&Verb::Close)
    }

    /// Check if the path represents a line.
    pub fn is_line(&self) -> bool {
        self.verbs.len() == 2
            && self.verbs[0] == Verb::Move
            && self.verbs[1] == Verb::Line
    }

    /// Check if the path represents a rectangle.
    pub fn is_rect(&self) -> Option<Rect> {
        // A rectangle has: Move, Line, Line, Line, Line, Close (or just 4 lines)
        if self.verbs.len() < 5 {
            return None;
        }

        let mut line_count = 0;
        let mut has_close = false;

        for verb in &self.verbs {
            match verb {
                Verb::Move => {}
                Verb::Line => line_count += 1,
                Verb::Close => has_close = true,
                _ => return None,
            }
        }

        if line_count != 4 || !has_close {
            return None;
        }

        // Check if points form a rectangle
        if self.points.len() < 5 {
            return None;
        }

        let p0 = self.points[0];
        let p1 = self.points[1];
        let p2 = self.points[2];
        let p3 = self.points[3];
        let p4 = self.points[4];

        // Check for axis-aligned rectangle
        let is_horizontal_1 = (p0.y - p1.y).abs() < 0.001;
        let is_vertical_2 = (p1.x - p2.x).abs() < 0.001;
        let is_horizontal_3 = (p2.y - p3.y).abs() < 0.001;
        let is_vertical_4 = (p3.x - p4.x).abs() < 0.001;

        if is_horizontal_1 && is_vertical_2 && is_horizontal_3 && is_vertical_4 {
            let left = p0.x.min(p1.x).min(p2.x).min(p3.x);
            let top = p0.y.min(p1.y).min(p2.y).min(p3.y);
            let right = p0.x.max(p1.x).max(p2.x).max(p3.x);
            let bottom = p0.y.max(p1.y).max(p2.y).max(p3.y);
            return Some(Rect::new(left, top, right, bottom));
        }

        None
    }

    /// Check if the path represents an oval.
    pub fn is_oval(&self) -> bool {
        // Ovals typically have 4 cubic curves or 4 conics
        let cubic_count = self.verbs.iter().filter(|v| **v == Verb::Cubic).count();
        let conic_count = self.verbs.iter().filter(|v| **v == Verb::Conic).count();

        (cubic_count == 4 && self.verbs.len() == 6) // Move + 4 Cubic + Close
            || (conic_count == 4 && self.verbs.len() == 6) // Move + 4 Conic + Close
    }

    /// Get the convexity of the path.
    pub fn convexity(&self) -> PathConvexity {
        if self.convexity != PathConvexity::Unknown {
            return self.convexity;
        }

        // Simple convexity check based on cross product signs
        if self.points.len() < 3 {
            return PathConvexity::Convex;
        }

        let mut sign = 0i32;
        let n = self.points.len();

        for i in 0..n {
            let p0 = self.points[i];
            let p1 = self.points[(i + 1) % n];
            let p2 = self.points[(i + 2) % n];

            let cross = (p1.x - p0.x) * (p2.y - p1.y) - (p1.y - p0.y) * (p2.x - p1.x);

            if cross.abs() > 0.001 {
                let current_sign = if cross > 0.0 { 1 } else { -1 };
                if sign == 0 {
                    sign = current_sign;
                } else if sign != current_sign {
                    return PathConvexity::Concave;
                }
            }
        }

        PathConvexity::Convex
    }

    /// Check if the path is convex.
    #[inline]
    pub fn is_convex(&self) -> bool {
        self.convexity() == PathConvexity::Convex
    }

    /// Get the direction of the first contour.
    pub fn direction(&self) -> Option<PathDirection> {
        if self.points.len() < 3 {
            return None;
        }

        // Calculate signed area using shoelace formula
        let mut signed_area = 0.0;
        let n = self.points.len();

        for i in 0..n {
            let p0 = self.points[i];
            let p1 = self.points[(i + 1) % n];
            signed_area += (p1.x - p0.x) * (p1.y + p0.y);
        }

        if signed_area.abs() < 0.001 {
            return None;
        }

        Some(if signed_area > 0.0 {
            PathDirection::CW
        } else {
            PathDirection::CCW
        })
    }

    /// Reverse the path direction.
    pub fn reverse(&mut self) {
        if self.verbs.is_empty() {
            return;
        }

        // Reverse points
        self.points.reverse();

        // Reverse conic weights
        self.conic_weights.reverse();

        // Reverse verbs (keeping structure)
        // This is a simplified implementation
        let mut new_verbs = SmallVec::new();
        let mut i = self.verbs.len();

        while i > 0 {
            i -= 1;
            match self.verbs[i] {
                Verb::Move => {
                    if !new_verbs.is_empty() {
                        new_verbs.push(Verb::Close);
                    }
                    new_verbs.push(Verb::Move);
                }
                Verb::Close => {
                    // Skip, will be added before next Move
                }
                v => new_verbs.push(v),
            }
        }

        if !new_verbs.is_empty() && self.is_closed() {
            new_verbs.push(Verb::Close);
        }

        self.verbs = new_verbs;
        self.bounds = None;
        self.convexity = PathConvexity::Unknown;
    }

    /// Transform the path by a matrix.
    pub fn transform(&mut self, matrix: &skia_rs_core::Matrix) {
        for point in &mut self.points {
            *point = matrix.map_point(*point);
        }
        self.bounds = None;
        self.convexity = PathConvexity::Unknown;
    }

    /// Create a transformed copy of the path.
    pub fn transformed(&self, matrix: &skia_rs_core::Matrix) -> Self {
        let mut result = self.clone();
        result.transform(matrix);
        result
    }

    /// Offset the path by (dx, dy).
    pub fn offset(&mut self, dx: Scalar, dy: Scalar) {
        for point in &mut self.points {
            point.x += dx;
            point.y += dy;
        }
        if let Some(ref mut bounds) = self.bounds {
            bounds.left += dx;
            bounds.right += dx;
            bounds.top += dy;
            bounds.bottom += dy;
        }
    }

    /// Check if a point is inside the path (using fill rule).
    pub fn contains(&self, point: Point) -> bool {
        // Ray casting algorithm
        if !self.bounds().contains(point) {
            return false;
        }

        let mut crossings = 0;
        let mut current = Point::zero();

        for element in self.iter() {
            match element {
                PathElement::Move(p) => current = p,
                PathElement::Line(end) => {
                    if ray_crosses_segment(point, current, end) {
                        crossings += 1;
                    }
                    current = end;
                }
                PathElement::Quad(ctrl, end) => {
                    // Approximate with lines
                    for i in 1..=8 {
                        let t = i as f32 / 8.0;
                        let mt = 1.0 - t;
                        let p = Point::new(
                            mt * mt * current.x + 2.0 * mt * t * ctrl.x + t * t * end.x,
                            mt * mt * current.y + 2.0 * mt * t * ctrl.y + t * t * end.y,
                        );
                        if ray_crosses_segment(point, current, p) {
                            crossings += 1;
                        }
                        current = p;
                    }
                    current = end;
                }
                PathElement::Conic(ctrl, end, _w) => {
                    // Approximate with lines
                    for i in 1..=8 {
                        let t = i as f32 / 8.0;
                        let mt = 1.0 - t;
                        let p = Point::new(
                            mt * mt * current.x + 2.0 * mt * t * ctrl.x + t * t * end.x,
                            mt * mt * current.y + 2.0 * mt * t * ctrl.y + t * t * end.y,
                        );
                        if ray_crosses_segment(point, current, p) {
                            crossings += 1;
                        }
                        current = p;
                    }
                    current = end;
                }
                PathElement::Cubic(c1, c2, end) => {
                    // Approximate with lines
                    for i in 1..=12 {
                        let t = i as f32 / 12.0;
                        let mt = 1.0 - t;
                        let mt2 = mt * mt;
                        let t2 = t * t;
                        let p = Point::new(
                            mt2 * mt * current.x + 3.0 * mt2 * t * c1.x + 3.0 * mt * t2 * c2.x + t2 * t * end.x,
                            mt2 * mt * current.y + 3.0 * mt2 * t * c1.y + 3.0 * mt * t2 * c2.y + t2 * t * end.y,
                        );
                        if ray_crosses_segment(point, current, p) {
                            crossings += 1;
                        }
                        current = p;
                    }
                    current = end;
                }
                PathElement::Close => {}
            }
        }

        match self.fill_type {
            FillType::Winding => crossings != 0,
            FillType::EvenOdd => crossings % 2 != 0,
            FillType::InverseWinding => crossings == 0,
            FillType::InverseEvenOdd => crossings % 2 == 0,
        }
    }

    /// Compute tight bounds (considering curve control points).
    pub fn tight_bounds(&self) -> Rect {
        // For now, same as bounds (which already considers all points)
        self.bounds()
    }

    /// Get the total length of the path.
    pub fn length(&self) -> Scalar {
        let mut total = 0.0;
        let mut current = Point::zero();

        for element in self.iter() {
            match element {
                PathElement::Move(p) => current = p,
                PathElement::Line(end) => {
                    total += current.distance(&end);
                    current = end;
                }
                PathElement::Quad(ctrl, end) => {
                    // Approximate
                    total += current.distance(&ctrl) + ctrl.distance(&end);
                    current = end;
                }
                PathElement::Conic(ctrl, end, _) => {
                    total += current.distance(&ctrl) + ctrl.distance(&end);
                    current = end;
                }
                PathElement::Cubic(c1, c2, end) => {
                    total += current.distance(&c1) + c1.distance(&c2) + c2.distance(&end);
                    current = end;
                }
                PathElement::Close => {}
            }
        }

        total
    }
}

/// Check if a horizontal ray from point crosses the segment.
fn ray_crosses_segment(point: Point, p0: Point, p1: Point) -> bool {
    // Ensure p0 is below p1
    let (p0, p1) = if p0.y <= p1.y { (p0, p1) } else { (p1, p0) };

    // Check if point is in y-range of segment
    if point.y < p0.y || point.y >= p1.y {
        return false;
    }

    // Calculate x-coordinate of intersection
    let t = (point.y - p0.y) / (p1.y - p0.y);
    let x_intersect = p0.x + t * (p1.x - p0.x);

    x_intersect > point.x
}

/// A path element from iteration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathElement {
    /// Move to point.
    Move(Point),
    /// Line to point.
    Line(Point),
    /// Quadratic bezier (control, end).
    Quad(Point, Point),
    /// Conic (control, end, weight).
    Conic(Point, Point, Scalar),
    /// Cubic bezier (control1, control2, end).
    Cubic(Point, Point, Point),
    /// Close the path.
    Close,
}

/// Iterator over path elements.
pub struct PathIter<'a> {
    path: &'a Path,
    verb_index: usize,
    point_index: usize,
    weight_index: usize,
}

impl<'a> Iterator for PathIter<'a> {
    type Item = PathElement;

    fn next(&mut self) -> Option<Self::Item> {
        if self.verb_index >= self.path.verbs.len() {
            return None;
        }

        let verb = self.path.verbs[self.verb_index];
        self.verb_index += 1;

        let element = match verb {
            Verb::Move => {
                let p = self.path.points[self.point_index];
                self.point_index += 1;
                PathElement::Move(p)
            }
            Verb::Line => {
                let p = self.path.points[self.point_index];
                self.point_index += 1;
                PathElement::Line(p)
            }
            Verb::Quad => {
                let p1 = self.path.points[self.point_index];
                let p2 = self.path.points[self.point_index + 1];
                self.point_index += 2;
                PathElement::Quad(p1, p2)
            }
            Verb::Conic => {
                let p1 = self.path.points[self.point_index];
                let p2 = self.path.points[self.point_index + 1];
                let w = self.path.conic_weights[self.weight_index];
                self.point_index += 2;
                self.weight_index += 1;
                PathElement::Conic(p1, p2, w)
            }
            Verb::Cubic => {
                let p1 = self.path.points[self.point_index];
                let p2 = self.path.points[self.point_index + 1];
                let p3 = self.path.points[self.point_index + 2];
                self.point_index += 3;
                PathElement::Cubic(p1, p2, p3)
            }
            Verb::Close => PathElement::Close,
        };

        Some(element)
    }
}
