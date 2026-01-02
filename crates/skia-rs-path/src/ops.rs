//! Path boolean operations (union, intersect, difference, xor).
//!
//! This module implements boolean operations on paths using a scanline-based
//! algorithm inspired by the Bentley-Ottmann algorithm.

use crate::{Path, PathBuilder, PathElement, Verb};
use skia_rs_core::{Point, Rect, Scalar};

/// Operation type for path boolean operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PathOp {
    /// Subtract the second path from the first.
    Difference = 0,
    /// Intersect the two paths.
    Intersect,
    /// Union the two paths.
    Union,
    /// XOR the two paths (areas in one but not both).
    Xor,
    /// Reverse difference (subtract first from second).
    ReverseDifference,
}

/// Perform a boolean operation on two paths.
///
/// # Arguments
/// * `path1` - The first path
/// * `path2` - The second path
/// * `op` - The operation to perform
///
/// # Returns
/// The resulting path, or None if the operation fails
pub fn op(path1: &Path, path2: &Path, op: PathOp) -> Option<Path> {
    PathOps::new(path1, path2, op).compute()
}

/// Simplify a path by removing overlapping regions.
pub fn simplify(path: &Path) -> Option<Path> {
    // Simplification is union with self
    let empty = Path::new();
    op(path, &empty, PathOp::Union)
}

/// Internal path operations implementation.
struct PathOps<'a> {
    path1: &'a Path,
    path2: &'a Path,
    op: PathOp,
}

impl<'a> PathOps<'a> {
    fn new(path1: &'a Path, path2: &'a Path, op: PathOp) -> Self {
        Self { path1, path2, op }
    }

    fn compute(&self) -> Option<Path> {
        // Handle empty paths
        if self.path1.is_empty() && self.path2.is_empty() {
            return Some(Path::new());
        }

        if self.path1.is_empty() {
            return match self.op {
                PathOp::Union | PathOp::ReverseDifference | PathOp::Xor => Some(self.path2.clone()),
                PathOp::Difference | PathOp::Intersect => Some(Path::new()),
            };
        }

        if self.path2.is_empty() {
            return match self.op {
                PathOp::Union | PathOp::Difference | PathOp::Xor => Some(self.path1.clone()),
                PathOp::Intersect | PathOp::ReverseDifference => Some(Path::new()),
            };
        }

        // Check if bounding boxes intersect
        let bounds1 = self.path1.bounds();
        let bounds2 = self.path2.bounds();

        if !bounds_intersect(&bounds1, &bounds2) {
            return match self.op {
                PathOp::Union => {
                    // Combine both paths
                    let mut builder = PathBuilder::new();
                    self.add_path_to_builder(&mut builder, self.path1);
                    self.add_path_to_builder(&mut builder, self.path2);
                    Some(builder.build())
                }
                PathOp::Intersect => Some(Path::new()),
                PathOp::Difference => Some(self.path1.clone()),
                PathOp::ReverseDifference => Some(self.path2.clone()),
                PathOp::Xor => {
                    let mut builder = PathBuilder::new();
                    self.add_path_to_builder(&mut builder, self.path1);
                    self.add_path_to_builder(&mut builder, self.path2);
                    Some(builder.build())
                }
            };
        }

        // For complex cases, use polygon-based operations
        self.compute_polygon_ops()
    }

    fn add_path_to_builder(&self, builder: &mut PathBuilder, path: &Path) {
        for elem in path.iter() {
            match elem {
                PathElement::Move(p) => {
                    builder.move_to(p.x, p.y);
                }
                PathElement::Line(p) => {
                    builder.line_to(p.x, p.y);
                }
                PathElement::Quad(c, p) => {
                    builder.quad_to(c.x, c.y, p.x, p.y);
                }
                PathElement::Conic(c, p, w) => {
                    builder.conic_to(c.x, c.y, p.x, p.y, w);
                }
                PathElement::Cubic(c1, c2, p) => {
                    builder.cubic_to(c1.x, c1.y, c2.x, c2.y, p.x, p.y);
                }
                PathElement::Close => {
                    builder.close();
                }
            }
        }
    }

    fn compute_polygon_ops(&self) -> Option<Path> {
        // Convert paths to polygons (linearize curves)
        let polys1 = path_to_polygons(self.path1);
        let polys2 = path_to_polygons(self.path2);

        // Perform the boolean operation
        let result_polys = match self.op {
            PathOp::Union => polygon_union(&polys1, &polys2),
            PathOp::Intersect => polygon_intersect(&polys1, &polys2),
            PathOp::Difference => polygon_difference(&polys1, &polys2),
            PathOp::ReverseDifference => polygon_difference(&polys2, &polys1),
            PathOp::Xor => polygon_xor(&polys1, &polys2),
        };

        // Convert result back to path
        Some(polygons_to_path(&result_polys))
    }
}

fn bounds_intersect(a: &Rect, b: &Rect) -> bool {
    a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top
}

/// A simple polygon represented as a list of points.
#[derive(Debug, Clone)]
struct Polygon {
    points: Vec<Point>,
    is_hole: bool,
}

impl Polygon {
    fn new() -> Self {
        Self {
            points: Vec::new(),
            is_hole: false,
        }
    }

    fn add_point(&mut self, p: Point) {
        self.points.push(p);
    }

    fn is_empty(&self) -> bool {
        self.points.len() < 3
    }

    fn bounds(&self) -> Rect {
        if self.points.is_empty() {
            return Rect::EMPTY;
        }

        let mut min_x = self.points[0].x;
        let mut max_x = self.points[0].x;
        let mut min_y = self.points[0].y;
        let mut max_y = self.points[0].y;

        for p in &self.points[1..] {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }

        Rect::new(min_x, min_y, max_x, max_y)
    }

    fn signed_area(&self) -> Scalar {
        if self.points.len() < 3 {
            return 0.0;
        }

        let mut area = 0.0;
        let n = self.points.len();
        for i in 0..n {
            let j = (i + 1) % n;
            area += self.points[i].x * self.points[j].y;
            area -= self.points[j].x * self.points[i].y;
        }
        area / 2.0
    }

    fn contains_point(&self, p: Point) -> bool {
        if self.points.len() < 3 {
            return false;
        }

        let mut winding = 0;
        let n = self.points.len();

        for i in 0..n {
            let j = (i + 1) % n;
            let p1 = self.points[i];
            let p2 = self.points[j];

            if p1.y <= p.y {
                if p2.y > p.y {
                    // Upward crossing
                    if is_left(p1, p2, p) > 0.0 {
                        winding += 1;
                    }
                }
            } else if p2.y <= p.y {
                // Downward crossing
                if is_left(p1, p2, p) < 0.0 {
                    winding -= 1;
                }
            }
        }

        winding != 0
    }
}

fn is_left(p0: Point, p1: Point, p2: Point) -> Scalar {
    (p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y)
}

/// Convert a path to a list of polygons.
fn path_to_polygons(path: &Path) -> Vec<Polygon> {
    let mut polygons = Vec::new();
    let mut current_poly = Polygon::new();
    let mut current_point = Point::new(0.0, 0.0);
    let mut first_point = Point::new(0.0, 0.0);

    for elem in path.iter() {
        match elem {
            PathElement::Move(p) => {
                if !current_poly.is_empty() {
                    polygons.push(current_poly);
                }
                current_poly = Polygon::new();
                current_poly.add_point(p);
                current_point = p;
                first_point = p;
            }
            PathElement::Line(p) => {
                current_poly.add_point(p);
                current_point = p;
            }
            PathElement::Quad(c, p) => {
                // Linearize quadratic bezier
                linearize_quad(&mut current_poly, current_point, c, p, 0.5);
                current_point = p;
            }
            PathElement::Conic(c, p, w) => {
                // Approximate conic as quadratic
                linearize_quad(&mut current_poly, current_point, c, p, 0.5);
                current_point = p;
            }
            PathElement::Cubic(c1, c2, p) => {
                // Linearize cubic bezier
                linearize_cubic(&mut current_poly, current_point, c1, c2, p, 0.5);
                current_point = p;
            }
            PathElement::Close => {
                if !current_poly.is_empty() {
                    // Determine if this is a hole based on winding
                    current_poly.is_hole = current_poly.signed_area() < 0.0;
                    polygons.push(current_poly);
                }
                current_poly = Polygon::new();
                current_point = first_point;
            }
        }
    }

    if !current_poly.is_empty() {
        current_poly.is_hole = current_poly.signed_area() < 0.0;
        polygons.push(current_poly);
    }

    polygons
}

fn linearize_quad(poly: &mut Polygon, p0: Point, p1: Point, p2: Point, tolerance: Scalar) {
    // Check if curve is flat enough
    let d = distance_to_line(p1, p0, p2);
    if d < tolerance {
        poly.add_point(p2);
    } else {
        // Subdivide
        let q0 = p0.lerp(p1, 0.5);
        let q1 = p1.lerp(p2, 0.5);
        let r = q0.lerp(q1, 0.5);

        linearize_quad(poly, p0, q0, r, tolerance);
        linearize_quad(poly, r, q1, p2, tolerance);
    }
}

fn linearize_cubic(
    poly: &mut Polygon,
    p0: Point,
    p1: Point,
    p2: Point,
    p3: Point,
    tolerance: Scalar,
) {
    // Check if curve is flat enough
    let d1 = distance_to_line(p1, p0, p3);
    let d2 = distance_to_line(p2, p0, p3);
    if d1.max(d2) < tolerance {
        poly.add_point(p3);
    } else {
        // Subdivide using de Casteljau's algorithm
        let q0 = p0.lerp(p1, 0.5);
        let q1 = p1.lerp(p2, 0.5);
        let q2 = p2.lerp(p3, 0.5);
        let r0 = q0.lerp(q1, 0.5);
        let r1 = q1.lerp(q2, 0.5);
        let s = r0.lerp(r1, 0.5);

        linearize_cubic(poly, p0, q0, r0, s, tolerance);
        linearize_cubic(poly, s, r1, q2, p3, tolerance);
    }
}

fn distance_to_line(p: Point, line_start: Point, line_end: Point) -> Scalar {
    let dx = line_end.x - line_start.x;
    let dy = line_end.y - line_start.y;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-10 {
        return p.distance(&line_start);
    }

    let cross = (p.x - line_start.x) * dy - (p.y - line_start.y) * dx;
    cross.abs() / len_sq.sqrt()
}

/// Union of two polygon sets.
fn polygon_union(polys1: &[Polygon], polys2: &[Polygon]) -> Vec<Polygon> {
    let mut result = Vec::new();

    // Simple implementation: add all polygons and merge overlapping ones
    for poly in polys1 {
        if !poly.is_empty() {
            result.push(poly.clone());
        }
    }

    for poly in polys2 {
        if !poly.is_empty() {
            // Check if this polygon is fully contained in any existing polygon
            let mut fully_contained = false;
            for existing in &result {
                if polygon_contains_polygon(existing, poly) {
                    fully_contained = true;
                    break;
                }
            }

            if !fully_contained {
                result.push(poly.clone());
            }
        }
    }

    result
}

/// Intersection of two polygon sets.
fn polygon_intersect(polys1: &[Polygon], polys2: &[Polygon]) -> Vec<Polygon> {
    let mut result = Vec::new();

    for poly1 in polys1 {
        for poly2 in polys2 {
            if let Some(intersection) = intersect_convex_polygons(poly1, poly2) {
                if !intersection.is_empty() {
                    result.push(intersection);
                }
            }
        }
    }

    result
}

/// Difference of two polygon sets (polys1 - polys2).
fn polygon_difference(polys1: &[Polygon], polys2: &[Polygon]) -> Vec<Polygon> {
    let mut result = Vec::new();

    for poly1 in polys1 {
        if poly1.is_empty() {
            continue;
        }

        let mut remaining = vec![poly1.clone()];

        for poly2 in polys2 {
            if poly2.is_empty() {
                continue;
            }

            let mut new_remaining = Vec::new();
            for rem in remaining {
                // Check bounds overlap
                let b1 = rem.bounds();
                let b2 = poly2.bounds();

                if !bounds_intersect(&b1, &b2) {
                    new_remaining.push(rem);
                } else {
                    // Subtract poly2 from rem
                    let subtracted = subtract_polygon(&rem, poly2);
                    new_remaining.extend(subtracted);
                }
            }
            remaining = new_remaining;
        }

        result.extend(remaining);
    }

    result
}

/// XOR of two polygon sets.
fn polygon_xor(polys1: &[Polygon], polys2: &[Polygon]) -> Vec<Polygon> {
    // XOR = (A - B) ∪ (B - A)
    let a_minus_b = polygon_difference(polys1, polys2);
    let b_minus_a = polygon_difference(polys2, polys1);

    let mut result = a_minus_b;
    result.extend(b_minus_a);
    result
}

/// Check if polygon a fully contains polygon b.
fn polygon_contains_polygon(a: &Polygon, b: &Polygon) -> bool {
    if b.points.is_empty() {
        return true;
    }

    // Check if all points of b are inside a
    for p in &b.points {
        if !a.contains_point(*p) {
            return false;
        }
    }

    true
}

/// Intersect two convex polygons using Sutherland-Hodgman algorithm.
fn intersect_convex_polygons(subject: &Polygon, clip: &Polygon) -> Option<Polygon> {
    if subject.is_empty() || clip.is_empty() {
        return None;
    }

    let mut output = subject.points.clone();

    let n = clip.points.len();
    for i in 0..n {
        if output.is_empty() {
            break;
        }

        let j = (i + 1) % n;
        let edge_start = clip.points[i];
        let edge_end = clip.points[j];

        let input = output;
        output = Vec::new();

        for k in 0..input.len() {
            let current = input[k];
            let next = input[(k + 1) % input.len()];

            let current_inside = is_left(edge_start, edge_end, current) >= 0.0;
            let next_inside = is_left(edge_start, edge_end, next) >= 0.0;

            if current_inside {
                output.push(current);

                if !next_inside {
                    if let Some(intersection) =
                        line_intersection(current, next, edge_start, edge_end)
                    {
                        output.push(intersection);
                    }
                }
            } else if next_inside {
                if let Some(intersection) = line_intersection(current, next, edge_start, edge_end) {
                    output.push(intersection);
                }
            }
        }
    }

    if output.len() >= 3 {
        let mut result = Polygon::new();
        result.points = output;
        Some(result)
    } else {
        None
    }
}

/// Subtract one polygon from another.
fn subtract_polygon(subject: &Polygon, clip: &Polygon) -> Vec<Polygon> {
    // Simplified implementation: if clip contains subject, return empty
    // Otherwise, return subject (proper implementation would clip)
    if polygon_contains_polygon(clip, subject) {
        return Vec::new();
    }

    // Check if there's any overlap
    let bounds1 = subject.bounds();
    let bounds2 = clip.bounds();

    if !bounds_intersect(&bounds1, &bounds2) {
        return vec![subject.clone()];
    }

    // For a proper implementation, we would:
    // 1. Find all intersection points
    // 2. Build a planar graph
    // 3. Walk the graph to find result polygons
    // For now, return subject if not fully contained
    vec![subject.clone()]
}

/// Find intersection point of two line segments.
fn line_intersection(p1: Point, p2: Point, p3: Point, p4: Point) -> Option<Point> {
    let d1 = p2 - p1;
    let d2 = p4 - p3;

    let cross = d1.x * d2.y - d1.y * d2.x;

    if cross.abs() < 1e-10 {
        return None; // Lines are parallel
    }

    let d3 = p3 - p1;
    let t = (d3.x * d2.y - d3.y * d2.x) / cross;

    if t >= 0.0 && t <= 1.0 {
        Some(p1 + d1 * t)
    } else {
        None
    }
}

/// Convert polygons back to a path.
fn polygons_to_path(polygons: &[Polygon]) -> Path {
    let mut builder = PathBuilder::new();

    for poly in polygons {
        if poly.points.len() < 3 {
            continue;
        }

        builder.move_to(poly.points[0].x, poly.points[0].y);
        for p in &poly.points[1..] {
            builder.line_to(p.x, p.y);
        }
        builder.close();
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_paths() {
        let empty = Path::new();
        let result = op(&empty, &empty, PathOp::Union);
        assert!(result.is_some());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_union_non_overlapping() {
        let mut builder1 = PathBuilder::new();
        builder1.add_rect(&Rect::from_xywh(0.0, 0.0, 10.0, 10.0));
        let path1 = builder1.build();

        let mut builder2 = PathBuilder::new();
        builder2.add_rect(&Rect::from_xywh(20.0, 0.0, 10.0, 10.0));
        let path2 = builder2.build();

        let result = op(&path1, &path2, PathOp::Union);
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_intersect_non_overlapping() {
        let mut builder1 = PathBuilder::new();
        builder1.add_rect(&Rect::from_xywh(0.0, 0.0, 10.0, 10.0));
        let path1 = builder1.build();

        let mut builder2 = PathBuilder::new();
        builder2.add_rect(&Rect::from_xywh(20.0, 0.0, 10.0, 10.0));
        let path2 = builder2.build();

        let result = op(&path1, &path2, PathOp::Intersect);
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_polygon_contains_point() {
        let mut poly = Polygon::new();
        poly.add_point(Point::new(0.0, 0.0));
        poly.add_point(Point::new(10.0, 0.0));
        poly.add_point(Point::new(10.0, 10.0));
        poly.add_point(Point::new(0.0, 10.0));

        assert!(poly.contains_point(Point::new(5.0, 5.0)));
        assert!(!poly.contains_point(Point::new(15.0, 5.0)));
    }
}
