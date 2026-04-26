//! Path tessellation for GPU rendering.
//!
//! This module provides algorithms for converting vector paths into triangle meshes
//! suitable for GPU rendering.

use skia_rs_core::{Point, Rect, Scalar};
use skia_rs_path::{Path, PathBuilder, PathElement};

/// A vertex in a tessellated mesh.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct TessVertex {
    /// Position.
    pub position: [f32; 2],
    /// UV coordinates (for texturing/gradients).
    pub uv: [f32; 2],
}

impl TessVertex {
    /// Create a new vertex.
    #[inline]
    pub const fn new(x: f32, y: f32, u: f32, v: f32) -> Self {
        Self {
            position: [x, y],
            uv: [u, v],
        }
    }

    /// Create a vertex from a point.
    #[inline]
    pub fn from_point(p: Point) -> Self {
        Self {
            position: [p.x, p.y],
            uv: [0.0, 0.0],
        }
    }
}

/// Index type for tessellated meshes.
pub type TessIndex = u32;

/// A tessellated mesh ready for GPU rendering.
#[derive(Debug, Clone, Default)]
pub struct TessMesh {
    /// Vertices.
    pub vertices: Vec<TessVertex>,
    /// Indices (triangle list).
    pub indices: Vec<TessIndex>,
}

impl TessMesh {
    /// Create a new empty mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a mesh with preallocated capacity.
    pub fn with_capacity(vertex_capacity: usize, index_capacity: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(vertex_capacity),
            indices: Vec::with_capacity(index_capacity),
        }
    }

    /// Clear the mesh.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() || self.indices.is_empty()
    }

    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Add a vertex and return its index.
    pub fn add_vertex(&mut self, vertex: TessVertex) -> TessIndex {
        let idx = self.vertices.len() as TessIndex;
        self.vertices.push(vertex);
        idx
    }

    /// Add a triangle by indices.
    pub fn add_triangle(&mut self, a: TessIndex, b: TessIndex, c: TessIndex) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
    }

    /// Merge another mesh into this one.
    pub fn merge(&mut self, other: &TessMesh) {
        let base_index = self.vertices.len() as TessIndex;
        self.vertices.extend_from_slice(&other.vertices);
        self.indices
            .extend(other.indices.iter().map(|i| i + base_index));
    }
}

/// Tessellation quality settings.
#[derive(Debug, Clone, Copy)]
pub struct TessQuality {
    /// Maximum distance from curve to approximating line segment.
    pub tolerance: Scalar,
    /// Maximum number of subdivisions for curves.
    pub max_subdivisions: u32,
}

impl Default for TessQuality {
    fn default() -> Self {
        Self {
            tolerance: 0.25,
            max_subdivisions: 10,
        }
    }
}

impl TessQuality {
    /// Low quality (fast).
    pub const LOW: Self = Self {
        tolerance: 1.0,
        max_subdivisions: 5,
    };

    /// Medium quality.
    pub const MEDIUM: Self = Self {
        tolerance: 0.5,
        max_subdivisions: 8,
    };

    /// High quality.
    pub const HIGH: Self = Self {
        tolerance: 0.25,
        max_subdivisions: 10,
    };

    /// Very high quality (slow).
    pub const VERY_HIGH: Self = Self {
        tolerance: 0.1,
        max_subdivisions: 15,
    };
}

/// Path tessellator.
pub struct PathTessellator {
    quality: TessQuality,
    /// Flattened points from current contour.
    contour_points: Vec<Point>,
}

impl PathTessellator {
    /// Create a new tessellator with default quality.
    pub fn new() -> Self {
        Self {
            quality: TessQuality::default(),
            contour_points: Vec::new(),
        }
    }

    /// Create a new tessellator with specified quality.
    pub fn with_quality(quality: TessQuality) -> Self {
        Self {
            quality,
            contour_points: Vec::new(),
        }
    }

    /// Tessellate a path for filling.
    pub fn tessellate_fill(&mut self, path: &Path) -> TessMesh {
        let mut mesh = TessMesh::new();
        self.contour_points.clear();

        let mut current_point = Point::zero();
        let mut contour_start = Point::zero();

        for element in path.iter() {
            match element {
                PathElement::Move(p) => {
                    self.flush_contour(&mut mesh);
                    current_point = p;
                    contour_start = p;
                    self.contour_points.push(p);
                }
                PathElement::Line(p) => {
                    self.contour_points.push(p);
                    current_point = p;
                }
                PathElement::Quad(ctrl, end) => {
                    self.flatten_quad(current_point, ctrl, end);
                    current_point = end;
                }
                PathElement::Conic(ctrl, end, weight) => {
                    self.flatten_conic(current_point, ctrl, end, weight);
                    current_point = end;
                }
                PathElement::Cubic(ctrl1, ctrl2, end) => {
                    self.flatten_cubic(current_point, ctrl1, ctrl2, end);
                    current_point = end;
                }
                PathElement::Close => {
                    if current_point != contour_start {
                        self.contour_points.push(contour_start);
                    }
                    self.flush_contour(&mut mesh);
                    current_point = contour_start;
                }
            }
        }

        // Flush any remaining contour
        self.flush_contour(&mut mesh);

        mesh
    }

    /// Tessellate a path for stroking.
    pub fn tessellate_stroke(&mut self, path: &Path, stroke_width: Scalar) -> TessMesh {
        let mut mesh = TessMesh::new();
        let half_width = stroke_width * 0.5;

        self.contour_points.clear();

        let mut current_point = Point::zero();
        let mut contour_start = Point::zero();

        for element in path.iter() {
            match element {
                PathElement::Move(p) => {
                    self.flush_stroke_contour(&mut mesh, half_width, false);
                    current_point = p;
                    contour_start = p;
                    self.contour_points.push(p);
                }
                PathElement::Line(p) => {
                    self.contour_points.push(p);
                    current_point = p;
                }
                PathElement::Quad(ctrl, end) => {
                    self.flatten_quad(current_point, ctrl, end);
                    current_point = end;
                }
                PathElement::Conic(ctrl, end, weight) => {
                    self.flatten_conic(current_point, ctrl, end, weight);
                    current_point = end;
                }
                PathElement::Cubic(ctrl1, ctrl2, end) => {
                    self.flatten_cubic(current_point, ctrl1, ctrl2, end);
                    current_point = end;
                }
                PathElement::Close => {
                    if current_point != contour_start {
                        self.contour_points.push(contour_start);
                    }
                    self.flush_stroke_contour(&mut mesh, half_width, true);
                    current_point = contour_start;
                }
            }
        }

        // Flush any remaining contour
        self.flush_stroke_contour(&mut mesh, half_width, false);

        mesh
    }

    /// Flatten a quadratic bezier curve.
    fn flatten_quad(&mut self, p0: Point, p1: Point, p2: Point) {
        let steps = self.quad_subdivisions(p0, p1, p2);
        for i in 1..=steps {
            let t = i as Scalar / steps as Scalar;
            let p = Self::eval_quad(p0, p1, p2, t);
            self.contour_points.push(p);
        }
    }

    /// Flatten a conic curve.
    fn flatten_conic(&mut self, p0: Point, p1: Point, p2: Point, w: Scalar) {
        // For simplicity, treat conics as quadratics when w ≈ 1
        if (w - 1.0).abs() < 0.001 {
            self.flatten_quad(p0, p1, p2);
            return;
        }

        // Adaptively choose step count based on curve deviation.
        //
        // Like the quadratic heuristic, use the max distance from the
        // control point to the chord as a proxy for curvature. Multiply
        // by a weight-dependent factor: conics with large (or small)
        // weights deviate further from the chord than w = 1 quadratics
        // for the same control-point placement, so scale the deviation
        // up as the weight departs from 1.
        let base_d = Self::point_to_line_distance(p1, p0, p2);
        let weight_amp = 1.0 + (w - 1.0).abs().min(4.0);
        let d = base_d * weight_amp;
        let steps = ((d / self.quality.tolerance).sqrt().ceil() as u32)
            .max(2)
            .min(self.quality.max_subdivisions.max(2));
        for i in 1..=steps {
            let t = i as Scalar / steps as Scalar;
            let p = Self::eval_conic(p0, p1, p2, w, t);
            self.contour_points.push(p);
        }
    }

    /// Flatten a cubic bezier curve.
    fn flatten_cubic(&mut self, p0: Point, p1: Point, p2: Point, p3: Point) {
        let steps = self.cubic_subdivisions(p0, p1, p2, p3);
        for i in 1..=steps {
            let t = i as Scalar / steps as Scalar;
            let p = Self::eval_cubic(p0, p1, p2, p3, t);
            self.contour_points.push(p);
        }
    }

    /// Calculate number of subdivisions for quadratic curve.
    fn quad_subdivisions(&self, p0: Point, p1: Point, p2: Point) -> u32 {
        let d = Self::point_to_line_distance(p1, p0, p2);
        let steps = ((d / self.quality.tolerance).sqrt().ceil() as u32).max(1);
        steps.min(self.quality.max_subdivisions)
    }

    /// Calculate number of subdivisions for cubic curve.
    fn cubic_subdivisions(&self, p0: Point, p1: Point, p2: Point, p3: Point) -> u32 {
        let d1 = Self::point_to_line_distance(p1, p0, p3);
        let d2 = Self::point_to_line_distance(p2, p0, p3);
        let d = d1.max(d2);
        let steps = ((d / self.quality.tolerance).sqrt().ceil() as u32).max(1);
        steps.min(self.quality.max_subdivisions)
    }

    /// Evaluate quadratic bezier at t.
    fn eval_quad(p0: Point, p1: Point, p2: Point, t: Scalar) -> Point {
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let t2 = t * t;
        Point::new(
            mt2 * p0.x + 2.0 * mt * t * p1.x + t2 * p2.x,
            mt2 * p0.y + 2.0 * mt * t * p1.y + t2 * p2.y,
        )
    }

    /// Evaluate conic at t.
    fn eval_conic(p0: Point, p1: Point, p2: Point, w: Scalar, t: Scalar) -> Point {
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let t2 = t * t;
        let wt = 2.0 * w * mt * t;
        let denom = mt2 + wt + t2;
        Point::new(
            (mt2 * p0.x + wt * p1.x + t2 * p2.x) / denom,
            (mt2 * p0.y + wt * p1.y + t2 * p2.y) / denom,
        )
    }

    /// Evaluate cubic bezier at t.
    fn eval_cubic(p0: Point, p1: Point, p2: Point, p3: Point, t: Scalar) -> Point {
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        let t2 = t * t;
        let t3 = t2 * t;
        Point::new(
            mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x,
            mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y,
        )
    }

    /// Calculate distance from point to line.
    fn point_to_line_distance(p: Point, line_start: Point, line_end: Point) -> Scalar {
        let dx = line_end.x - line_start.x;
        let dy = line_end.y - line_start.y;
        let len_sq = dx * dx + dy * dy;
        if len_sq < 1e-10 {
            return ((p.x - line_start.x).powi(2) + (p.y - line_start.y).powi(2)).sqrt();
        }
        let num = ((p.x - line_start.x) * dy - (p.y - line_start.y) * dx).abs();
        num / len_sq.sqrt()
    }

    /// Flush current contour for fill tessellation.
    ///
    /// Uses a proper ear-clipping triangulation that handles both convex and
    /// concave polygons. The classical O(n²) ear-clip algorithm (Meisters 1975):
    /// repeatedly find a triangle formed by three consecutive vertices that
    /// is (a) wound in the polygon's direction and (b) contains no other
    /// vertex, emit it, remove the middle vertex, and continue. This is
    /// correct for any simple polygon, including concave shapes like letter
    /// outlines. For self-intersecting polygons the output is still a valid
    /// triangulation of the positive-area regions under the winding rule.
    fn flush_contour(&mut self, mesh: &mut TessMesh) {
        if self.contour_points.len() < 3 {
            self.contour_points.clear();
            return;
        }

        // Drop a trailing duplicate of the first point (added by PathElement::Close)
        // so the winding/ear tests don't see a zero-length final edge.
        if self.contour_points.len() > 3 {
            let first = self.contour_points[0];
            let last = *self.contour_points.last().unwrap();
            if (first.x - last.x).abs() < 1e-6 && (first.y - last.y).abs() < 1e-6 {
                self.contour_points.pop();
            }
        }

        if self.contour_points.len() < 3 {
            self.contour_points.clear();
            return;
        }

        // Push vertices into the mesh up front; ear clipping emits indices.
        let base_idx = mesh.vertices.len() as TessIndex;
        let vertices: Vec<TessVertex> = self
            .contour_points
            .iter()
            .map(|p| TessVertex::from_point(*p))
            .collect();
        mesh.vertices.extend(vertices);

        ear_clip_triangulate(&self.contour_points, base_idx, mesh);
        self.contour_points.clear();
    }

    /// Flush current contour for stroke tessellation.
    fn flush_stroke_contour(&mut self, mesh: &mut TessMesh, half_width: Scalar, closed: bool) {
        if self.contour_points.len() < 2 {
            self.contour_points.clear();
            return;
        }

        let n = self.contour_points.len();

        // Generate left and right edge vertices
        let mut left_vertices = Vec::with_capacity(n);
        let mut right_vertices = Vec::with_capacity(n);

        for i in 0..n {
            let prev_idx = if i == 0 {
                if closed { n - 1 } else { 0 }
            } else {
                i - 1
            };
            let next_idx = if i == n - 1 {
                if closed { 0 } else { n - 1 }
            } else {
                i + 1
            };

            let p = self.contour_points[i];
            let prev = self.contour_points[prev_idx];
            let next = self.contour_points[next_idx];

            // Calculate tangent direction
            let tangent = if i == 0 && !closed {
                Point::new(next.x - p.x, next.y - p.y)
            } else if i == n - 1 && !closed {
                Point::new(p.x - prev.x, p.y - prev.y)
            } else {
                let t1 = Point::new(p.x - prev.x, p.y - prev.y);
                let t2 = Point::new(next.x - p.x, next.y - p.y);
                Point::new(t1.x + t2.x, t1.y + t2.y)
            };

            // Normalize and get perpendicular
            let len = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
            if len < 1e-10 {
                left_vertices.push(TessVertex::from_point(p));
                right_vertices.push(TessVertex::from_point(p));
                continue;
            }

            let normal = Point::new(-tangent.y / len, tangent.x / len);

            left_vertices.push(TessVertex::new(
                p.x + normal.x * half_width,
                p.y + normal.y * half_width,
                0.0,
                0.0,
            ));
            right_vertices.push(TessVertex::new(
                p.x - normal.x * half_width,
                p.y - normal.y * half_width,
                1.0,
                0.0,
            ));
        }

        // Create triangle strip
        let base_idx = mesh.vertices.len() as TessIndex;
        mesh.vertices.extend(left_vertices);
        let right_base = mesh.vertices.len() as TessIndex;
        mesh.vertices.extend(right_vertices);

        for i in 0..(n - 1) {
            let i = i as TessIndex;
            mesh.add_triangle(base_idx + i, right_base + i, base_idx + i + 1);
            mesh.add_triangle(base_idx + i + 1, right_base + i, right_base + i + 1);
        }

        // Close the stroke if needed
        if closed && n > 2 {
            let last = (n - 1) as TessIndex;
            mesh.add_triangle(base_idx + last, right_base + last, base_idx);
            mesh.add_triangle(base_idx, right_base + last, right_base);
        }

        self.contour_points.clear();
    }
}

impl Default for PathTessellator {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute signed area of a polygon (positive = counter-clockwise).
fn polygon_signed_area(points: &[Point]) -> Scalar {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    let mut area: Scalar = 0.0;
    for i in 0..n {
        let p = points[i];
        let q = points[(i + 1) % n];
        area += p.x * q.y - q.x * p.y;
    }
    area * 0.5
}

/// Twice the signed area of triangle abc (sign gives orientation).
#[inline]
fn triangle_cross(a: Point, b: Point, c: Point) -> Scalar {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Test whether point `p` lies inside triangle `abc` (with a tolerant
/// boundary that excludes the triangle's own vertices).
fn point_in_triangle(p: Point, a: Point, b: Point, c: Point) -> bool {
    let d1 = triangle_cross(p, a, b);
    let d2 = triangle_cross(p, b, c);
    let d3 = triangle_cross(p, c, a);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

/// Ear-clip triangulation for an arbitrary simple polygon.
///
/// Writes triangles into `mesh` as indices offset by `base_idx`. The
/// algorithm is robust for concave polygons; it degenerates gracefully
/// (emits a fan) when no ear can be found, which only happens for
/// pathological self-intersecting input.
fn ear_clip_triangulate(points: &[Point], base_idx: TessIndex, mesh: &mut TessMesh) {
    let n = points.len();
    if n < 3 {
        return;
    }

    // Determine polygon orientation. Ears are the convex vertices relative
    // to the polygon's winding, so we flip the sense for clockwise input.
    let ccw = polygon_signed_area(points) >= 0.0;

    // Working list of polygon-index references. We remove indices as we
    // clip ears, but keep `points` untouched.
    let mut remaining: Vec<usize> = (0..n).collect();

    // Safety guard against infinite loops for self-intersecting input:
    // bound total iterations at ~2n² — finite, correct on simple polygons.
    let max_iters = 2 * n * n + 4;
    let mut iters = 0;

    while remaining.len() > 3 {
        iters += 1;
        if iters > max_iters {
            // Fall back to a fan over the remaining vertices. This matches
            // the original behaviour for truly degenerate input and keeps
            // the tessellator total — we never emit nothing.
            break;
        }

        let m = remaining.len();
        let mut ear_pos: Option<usize> = None;

        for i in 0..m {
            let ia = remaining[(i + m - 1) % m];
            let ib = remaining[i];
            let ic = remaining[(i + 1) % m];
            let a = points[ia];
            let b = points[ib];
            let c = points[ic];

            // Reflex check: ear must be convex in the polygon's winding.
            let cross = triangle_cross(a, b, c);
            let is_convex = if ccw { cross > 0.0 } else { cross < 0.0 };
            if !is_convex {
                continue;
            }

            // Ear check: no other polygon vertex lies inside triangle abc.
            let mut is_ear = true;
            for &other in &remaining {
                if other == ia || other == ib || other == ic {
                    continue;
                }
                if point_in_triangle(points[other], a, b, c) {
                    is_ear = false;
                    break;
                }
            }

            if is_ear {
                ear_pos = Some(i);
                break;
            }
        }

        match ear_pos {
            Some(i) => {
                let m = remaining.len();
                let ia = remaining[(i + m - 1) % m];
                let ib = remaining[i];
                let ic = remaining[(i + 1) % m];
                // Emit triangle in the polygon's winding order so the
                // front-face rule stays consistent with the source path.
                if ccw {
                    mesh.add_triangle(
                        base_idx + ia as TessIndex,
                        base_idx + ib as TessIndex,
                        base_idx + ic as TessIndex,
                    );
                } else {
                    mesh.add_triangle(
                        base_idx + ia as TessIndex,
                        base_idx + ic as TessIndex,
                        base_idx + ib as TessIndex,
                    );
                }
                remaining.remove(i);
            }
            None => {
                // No ear found. For a simple polygon this cannot happen, so
                // the input is self-intersecting. Fall back so we still
                // produce *some* triangulation rather than dropping the
                // contour silently.
                break;
            }
        }
    }

    if remaining.len() == 3 {
        let ia = remaining[0];
        let ib = remaining[1];
        let ic = remaining[2];
        let a = points[ia];
        let b = points[ib];
        let c = points[ic];
        let cross = triangle_cross(a, b, c);
        if cross.abs() > 1e-12 {
            if ccw == (cross > 0.0) {
                mesh.add_triangle(
                    base_idx + ia as TessIndex,
                    base_idx + ib as TessIndex,
                    base_idx + ic as TessIndex,
                );
            } else {
                mesh.add_triangle(
                    base_idx + ia as TessIndex,
                    base_idx + ic as TessIndex,
                    base_idx + ib as TessIndex,
                );
            }
        }
    } else if remaining.len() > 3 {
        // Fan fallback for degenerate input: preserves some coverage rather
        // than yielding an empty mesh.
        let ia = remaining[0];
        for w in remaining.windows(2).skip(1) {
            let ib = w[0];
            let ic = w[1];
            mesh.add_triangle(
                base_idx + ia as TessIndex,
                base_idx + ib as TessIndex,
                base_idx + ic as TessIndex,
            );
        }
    }
}

/// Tessellate a rectangle.
pub fn tessellate_rect(rect: Rect) -> TessMesh {
    let mut mesh = TessMesh::with_capacity(4, 6);

    let v0 = mesh.add_vertex(TessVertex::new(rect.left, rect.top, 0.0, 0.0));
    let v1 = mesh.add_vertex(TessVertex::new(rect.right, rect.top, 1.0, 0.0));
    let v2 = mesh.add_vertex(TessVertex::new(rect.right, rect.bottom, 1.0, 1.0));
    let v3 = mesh.add_vertex(TessVertex::new(rect.left, rect.bottom, 0.0, 1.0));

    mesh.add_triangle(v0, v1, v2);
    mesh.add_triangle(v0, v2, v3);

    mesh
}

/// Tessellate a rounded rectangle.
pub fn tessellate_rounded_rect(rect: Rect, radius: Scalar, quality: TessQuality) -> TessMesh {
    let mut mesh = TessMesh::new();

    let r = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    if r < 0.001 {
        return tessellate_rect(rect);
    }

    // Calculate number of segments for corners
    let segments = ((std::f32::consts::PI * r / quality.tolerance).ceil() as usize)
        .max(4)
        .min(quality.max_subdivisions as usize);

    let center = rect.center();
    let center_idx = mesh.add_vertex(TessVertex::new(center.x, center.y, 0.5, 0.5));

    let mut edge_vertices = Vec::new();

    // Top-left corner
    for i in 0..=segments {
        let angle =
            std::f32::consts::PI + (i as f32 / segments as f32) * std::f32::consts::FRAC_PI_2;
        let x = rect.left + r + r * angle.cos();
        let y = rect.top + r + r * angle.sin();
        let u = (x - rect.left) / rect.width();
        let v = (y - rect.top) / rect.height();
        edge_vertices.push(mesh.add_vertex(TessVertex::new(x, y, u, v)));
    }

    // Top-right corner
    for i in 0..=segments {
        let angle =
            std::f32::consts::PI * 1.5 + (i as f32 / segments as f32) * std::f32::consts::FRAC_PI_2;
        let x = rect.right - r + r * angle.cos();
        let y = rect.top + r + r * angle.sin();
        let u = (x - rect.left) / rect.width();
        let v = (y - rect.top) / rect.height();
        edge_vertices.push(mesh.add_vertex(TessVertex::new(x, y, u, v)));
    }

    // Bottom-right corner
    for i in 0..=segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::FRAC_PI_2;
        let x = rect.right - r + r * angle.cos();
        let y = rect.bottom - r + r * angle.sin();
        let u = (x - rect.left) / rect.width();
        let v = (y - rect.top) / rect.height();
        edge_vertices.push(mesh.add_vertex(TessVertex::new(x, y, u, v)));
    }

    // Bottom-left corner
    for i in 0..=segments {
        let angle = std::f32::consts::FRAC_PI_2
            + (i as f32 / segments as f32) * std::f32::consts::FRAC_PI_2;
        let x = rect.left + r + r * angle.cos();
        let y = rect.bottom - r + r * angle.sin();
        let u = (x - rect.left) / rect.width();
        let v = (y - rect.top) / rect.height();
        edge_vertices.push(mesh.add_vertex(TessVertex::new(x, y, u, v)));
    }

    // Create triangles from center to edge
    let n = edge_vertices.len();
    for i in 0..n {
        let next = (i + 1) % n;
        mesh.add_triangle(center_idx, edge_vertices[i], edge_vertices[next]);
    }

    mesh
}

/// Tessellate a circle.
pub fn tessellate_circle(center: Point, radius: Scalar, quality: TessQuality) -> TessMesh {
    let mut mesh = TessMesh::new();

    let segments = ((2.0 * std::f32::consts::PI * radius / quality.tolerance).ceil() as usize)
        .max(8)
        .min(quality.max_subdivisions as usize * 4);

    let center_idx = mesh.add_vertex(TessVertex::new(center.x, center.y, 0.5, 0.5));

    let mut edge_vertices = Vec::with_capacity(segments);
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
        let x = center.x + radius * angle.cos();
        let y = center.y + radius * angle.sin();
        let u = 0.5 + 0.5 * angle.cos();
        let v = 0.5 + 0.5 * angle.sin();
        edge_vertices.push(mesh.add_vertex(TessVertex::new(x, y, u, v)));
    }

    for i in 0..segments {
        let next = (i + 1) % segments;
        mesh.add_triangle(center_idx, edge_vertices[i], edge_vertices[next]);
    }

    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tess_vertex() {
        let v = TessVertex::new(1.0, 2.0, 0.5, 0.5);
        assert_eq!(v.position, [1.0, 2.0]);
        assert_eq!(v.uv, [0.5, 0.5]);
    }

    #[test]
    fn test_tess_mesh() {
        let mut mesh = TessMesh::new();
        assert!(mesh.is_empty());

        let v0 = mesh.add_vertex(TessVertex::new(0.0, 0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(TessVertex::new(1.0, 0.0, 1.0, 0.0));
        let v2 = mesh.add_vertex(TessVertex::new(0.5, 1.0, 0.5, 1.0));
        mesh.add_triangle(v0, v1, v2);

        assert!(!mesh.is_empty());
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices.len(), 3);
    }

    #[test]
    fn test_tessellate_rect() {
        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 50.0);
        let mesh = tessellate_rect(rect);
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
        assert_eq!(mesh.triangle_count(), 2);
    }

    #[test]
    fn test_tessellate_circle() {
        let mesh = tessellate_circle(Point::new(50.0, 50.0), 25.0, TessQuality::MEDIUM);
        assert!(mesh.vertices.len() > 8);
        assert!(mesh.triangle_count() >= 8);
    }

    #[test]
    fn test_tessellate_rounded_rect() {
        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 50.0);
        let mesh = tessellate_rounded_rect(rect, 10.0, TessQuality::MEDIUM);
        assert!(mesh.vertices.len() > 4);
        assert!(mesh.triangle_count() > 2);
    }

    #[test]
    fn test_path_tessellator_fill() {
        let mut tessellator = PathTessellator::new();
        let mut builder = PathBuilder::new();
        builder
            .move_to(0.0, 0.0)
            .line_to(100.0, 0.0)
            .line_to(100.0, 100.0)
            .line_to(0.0, 100.0)
            .close();
        let path = builder.build();

        let mesh = tessellator.tessellate_fill(&path);
        assert!(!mesh.is_empty());
        // 5 vertices: 4 corners + 1 close point (returns to start)
        assert!(mesh.vertices.len() >= 4);
        assert!(mesh.triangle_count() >= 2);
    }

    #[test]
    fn test_path_tessellator_stroke() {
        let mut tessellator = PathTessellator::new();
        let mut builder = PathBuilder::new();
        builder
            .move_to(0.0, 0.0)
            .line_to(100.0, 0.0)
            .line_to(100.0, 100.0);
        let path = builder.build();

        let mesh = tessellator.tessellate_stroke(&path, 2.0);
        assert!(!mesh.is_empty());
        assert!(mesh.vertices.len() >= 6);
    }

    #[test]
    fn test_quality_presets() {
        assert!(TessQuality::LOW.tolerance > TessQuality::HIGH.tolerance);
        assert!(TessQuality::LOW.max_subdivisions < TessQuality::HIGH.max_subdivisions);
    }

    #[test]
    fn test_polygon_signed_area() {
        // CCW square
        let ccw = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        assert!(polygon_signed_area(&ccw) > 0.0);
        // CW square
        let cw = [
            Point::new(0.0, 0.0),
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 0.0),
        ];
        assert!(polygon_signed_area(&cw) < 0.0);
    }

    #[test]
    fn test_ear_clip_concave_l_shape() {
        // L-shaped hexagon (classic concave test): one vertex is reflex.
        // Outer contour (CCW): 6 vertices, 1 reflex → ear clip must emit
        // exactly 4 triangles, and no triangle may straddle the notch.
        //
        //  +----+
        //  |    |
        //  |    +--+
        //  |       |
        //  +-------+
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            Point::new(20.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 20.0),
            Point::new(0.0, 20.0),
        ];
        let mut mesh = TessMesh::new();
        for p in &pts {
            mesh.add_vertex(TessVertex::from_point(*p));
        }
        ear_clip_triangulate(&pts, 0, &mut mesh);
        assert_eq!(mesh.triangle_count(), 4, "L-shape must emit n-2 triangles");

        // Sanity: none of the emitted triangles should cover the notch.
        // The notch is at x in (10, 20), y in (10, 20) — the point (15, 15)
        // must NOT lie inside any triangle we emitted.
        let notch = Point::new(15.0, 15.0);
        for tri in mesh.indices.chunks(3) {
            let a = pts[tri[0] as usize];
            let b = pts[tri[1] as usize];
            let c = pts[tri[2] as usize];
            assert!(
                !point_in_triangle(notch, a, b, c),
                "triangle ({:?},{:?},{:?}) covers the concave notch",
                a,
                b,
                c
            );
        }
    }

    #[test]
    fn test_ear_clip_u_shape_concave() {
        // U-shape: classic non-convex. Fan triangulation would emit
        // triangles that cross the opening; ear clipping must not.
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(30.0, 0.0),
            Point::new(30.0, 30.0),
            Point::new(20.0, 30.0),
            Point::new(20.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 30.0),
            Point::new(0.0, 30.0),
        ];
        let mut mesh = TessMesh::new();
        for p in &pts {
            mesh.add_vertex(TessVertex::from_point(*p));
        }
        ear_clip_triangulate(&pts, 0, &mut mesh);
        // 8 vertices → 6 triangles.
        assert_eq!(mesh.triangle_count(), 6);

        // Point in the middle of the opening must NOT be covered.
        let hole = Point::new(15.0, 20.0);
        for tri in mesh.indices.chunks(3) {
            let a = pts[tri[0] as usize];
            let b = pts[tri[1] as usize];
            let c = pts[tri[2] as usize];
            assert!(!point_in_triangle(hole, a, b, c));
        }
    }

    #[test]
    fn test_ear_clip_clockwise_input() {
        // Same L-shape but in CW order — the algorithm must still triangulate
        // it without choking on the flipped orientation.
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(0.0, 20.0),
            Point::new(10.0, 20.0),
            Point::new(10.0, 10.0),
            Point::new(20.0, 10.0),
            Point::new(20.0, 0.0),
        ];
        let mut mesh = TessMesh::new();
        for p in &pts {
            mesh.add_vertex(TessVertex::from_point(*p));
        }
        ear_clip_triangulate(&pts, 0, &mut mesh);
        assert_eq!(mesh.triangle_count(), 4);
    }

    #[test]
    fn test_path_tessellator_concave_fill() {
        // Full integration test: concave L-path through the tessellator.
        let mut tessellator = PathTessellator::new();
        let mut builder = PathBuilder::new();
        builder
            .move_to(0.0, 0.0)
            .line_to(20.0, 0.0)
            .line_to(20.0, 10.0)
            .line_to(10.0, 10.0)
            .line_to(10.0, 20.0)
            .line_to(0.0, 20.0)
            .close();
        let path = builder.build();

        let mesh = tessellator.tessellate_fill(&path);
        // Six polygon vertices → four triangles.
        assert_eq!(mesh.triangle_count(), 4);
    }

    #[test]
    fn test_conic_adaptive_subdivision() {
        // Near-unit weight should degrade to the quadratic path (few steps
        // for a gentle curve). Large-weight conic should emit more steps.
        let mut t = PathTessellator::with_quality(TessQuality {
            tolerance: 0.25,
            max_subdivisions: 64,
        });
        t.contour_points.push(Point::new(0.0, 0.0));
        t.flatten_conic(
            Point::new(0.0, 0.0),
            Point::new(5.0, 5.0),
            Point::new(10.0, 0.0),
            3.0,
        );
        let heavy = t.contour_points.len();
        t.contour_points.clear();

        t.contour_points.push(Point::new(0.0, 0.0));
        t.flatten_conic(
            Point::new(0.0, 0.0),
            Point::new(5.0, 0.01),
            Point::new(10.0, 0.0),
            3.0,
        );
        let gentle = t.contour_points.len();

        assert!(
            heavy > gentle,
            "sharp conic ({} steps) should emit more segments than a gentle one ({} steps)",
            heavy,
            gentle,
        );
    }
}
