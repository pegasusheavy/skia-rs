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

        // Subdivide the conic for better approximation
        let steps = (self.quality.max_subdivisions as usize).max(8);
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

    /// Flush current contour for fill tessellation using ear clipping.
    fn flush_contour(&mut self, mesh: &mut TessMesh) {
        if self.contour_points.len() < 3 {
            self.contour_points.clear();
            return;
        }

        // Simple ear clipping triangulation
        let vertices: Vec<TessVertex> = self
            .contour_points
            .iter()
            .map(|p| TessVertex::from_point(*p))
            .collect();

        let base_idx = mesh.vertices.len() as TessIndex;
        mesh.vertices.extend(vertices);

        // Triangulate using fan (works for convex, approximation for concave)
        let n = self.contour_points.len();
        for i in 1..(n - 1) {
            mesh.add_triangle(
                base_idx,
                base_idx + i as TessIndex,
                base_idx + (i + 1) as TessIndex,
            );
        }

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
}
