//! Path builder for constructing paths.

use crate::{FillType, Path, Verb};
use skia_rs_core::{Point, Rect, Scalar};

/// Builder for constructing paths.
#[derive(Debug, Clone, Default)]
pub struct PathBuilder {
    path: Path,
    last_move: Option<Point>,
}

impl PathBuilder {
    /// Create a new path builder.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a path builder with specified fill type.
    #[inline]
    pub fn with_fill_type(fill_type: FillType) -> Self {
        let mut builder = Self::new();
        builder.path.fill_type = fill_type;
        builder
    }

    /// Set the fill type.
    #[inline]
    pub fn fill_type(&mut self, fill_type: FillType) -> &mut Self {
        self.path.fill_type = fill_type;
        self
    }

    /// Move to a point.
    #[inline]
    pub fn move_to(&mut self, x: Scalar, y: Scalar) -> &mut Self {
        let p = Point::new(x, y);
        self.path.verbs.push(Verb::Move);
        self.path.points.push(p);
        self.last_move = Some(p);
        self.path.bounds = None;
        self
    }

    /// Line to a point.
    #[inline]
    pub fn line_to(&mut self, x: Scalar, y: Scalar) -> &mut Self {
        self.ensure_move();
        self.path.verbs.push(Verb::Line);
        self.path.points.push(Point::new(x, y));
        self.path.bounds = None;
        self
    }

    /// Quadratic bezier curve.
    #[inline]
    pub fn quad_to(&mut self, x1: Scalar, y1: Scalar, x2: Scalar, y2: Scalar) -> &mut Self {
        self.ensure_move();
        self.path.verbs.push(Verb::Quad);
        self.path.points.push(Point::new(x1, y1));
        self.path.points.push(Point::new(x2, y2));
        self.path.bounds = None;
        self
    }

    /// Conic curve (weighted quadratic).
    #[inline]
    pub fn conic_to(&mut self, x1: Scalar, y1: Scalar, x2: Scalar, y2: Scalar, w: Scalar) -> &mut Self {
        self.ensure_move();
        self.path.verbs.push(Verb::Conic);
        self.path.points.push(Point::new(x1, y1));
        self.path.points.push(Point::new(x2, y2));
        self.path.conic_weights.push(w);
        self.path.bounds = None;
        self
    }

    /// Cubic bezier curve.
    #[inline]
    pub fn cubic_to(&mut self, x1: Scalar, y1: Scalar, x2: Scalar, y2: Scalar, x3: Scalar, y3: Scalar) -> &mut Self {
        self.ensure_move();
        self.path.verbs.push(Verb::Cubic);
        self.path.points.push(Point::new(x1, y1));
        self.path.points.push(Point::new(x2, y2));
        self.path.points.push(Point::new(x3, y3));
        self.path.bounds = None;
        self
    }

    /// Close the current contour.
    #[inline]
    pub fn close(&mut self) -> &mut Self {
        if !self.path.verbs.is_empty() {
            self.path.verbs.push(Verb::Close);
        }
        self
    }

    /// Add a rectangle.
    pub fn add_rect(&mut self, rect: &Rect) -> &mut Self {
        self.move_to(rect.left, rect.top)
            .line_to(rect.right, rect.top)
            .line_to(rect.right, rect.bottom)
            .line_to(rect.left, rect.bottom)
            .close()
    }

    /// Add an oval inscribed in the rectangle.
    pub fn add_oval(&mut self, rect: &Rect) -> &mut Self {
        let cx = (rect.left + rect.right) / 2.0;
        let cy = (rect.top + rect.bottom) / 2.0;
        let rx = rect.width() / 2.0;
        let ry = rect.height() / 2.0;

        // Magic number for circular arc approximation
        const KAPPA: Scalar = 0.5522847498;
        let kx = rx * KAPPA;
        let ky = ry * KAPPA;

        self.move_to(cx + rx, cy)
            .cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry)
            .cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy)
            .cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry)
            .cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy)
            .close()
    }

    /// Add a circle.
    pub fn add_circle(&mut self, cx: Scalar, cy: Scalar, radius: Scalar) -> &mut Self {
        self.add_oval(&Rect::new(cx - radius, cy - radius, cx + radius, cy + radius))
    }

    /// Add a rounded rectangle.
    pub fn add_round_rect(&mut self, rect: &Rect, rx: Scalar, ry: Scalar) -> &mut Self {
        if rx <= 0.0 || ry <= 0.0 {
            return self.add_rect(rect);
        }

        let rx = rx.min(rect.width() / 2.0);
        let ry = ry.min(rect.height() / 2.0);

        const KAPPA: Scalar = 0.5522847498;
        let kx = rx * KAPPA;
        let ky = ry * KAPPA;

        self.move_to(rect.left + rx, rect.top)
            .line_to(rect.right - rx, rect.top)
            .cubic_to(rect.right - rx + kx, rect.top, rect.right, rect.top + ry - ky, rect.right, rect.top + ry)
            .line_to(rect.right, rect.bottom - ry)
            .cubic_to(rect.right, rect.bottom - ry + ky, rect.right - rx + kx, rect.bottom, rect.right - rx, rect.bottom)
            .line_to(rect.left + rx, rect.bottom)
            .cubic_to(rect.left + rx - kx, rect.bottom, rect.left, rect.bottom - ry + ky, rect.left, rect.bottom - ry)
            .line_to(rect.left, rect.top + ry)
            .cubic_to(rect.left, rect.top + ry - ky, rect.left + rx - kx, rect.top, rect.left + rx, rect.top)
            .close()
    }

    /// Add an arc as a new contour.
    ///
    /// The arc is inscribed in the oval bounded by `oval`, starting at `start_angle`
    /// and sweeping `sweep_angle` degrees. Angles are measured in degrees, with
    /// 0 at the 3 o'clock position, increasing clockwise.
    pub fn add_arc(&mut self, oval: &Rect, start_angle: Scalar, sweep_angle: Scalar) -> &mut Self {
        if sweep_angle.abs() >= 360.0 {
            return self.add_oval(oval);
        }

        let cx = oval.center().x;
        let cy = oval.center().y;
        let rx = oval.width() / 2.0;
        let ry = oval.height() / 2.0;

        // Convert to radians
        let start_rad = start_angle.to_radians();
        let sweep_rad = sweep_angle.to_radians();

        // Starting point
        let start_x = cx + rx * start_rad.cos();
        let start_y = cy + ry * start_rad.sin();
        self.move_to(start_x, start_y);

        // Add arc segments (approximate with cubics)
        self.add_arc_to_impl(cx, cy, rx, ry, start_rad, sweep_rad);

        self
    }

    /// Arc to a point using radii and rotation.
    ///
    /// This matches the SVG arc command semantics.
    pub fn arc_to(
        &mut self,
        rx: Scalar,
        ry: Scalar,
        x_axis_rotate: Scalar,
        large_arc: bool,
        sweep: bool,
        x: Scalar,
        y: Scalar,
    ) -> &mut Self {
        self.ensure_move();

        // Get current point
        let current = self.current_point();
        if current.x == x && current.y == y {
            return self;
        }

        // Handle degenerate cases
        if rx == 0.0 || ry == 0.0 {
            return self.line_to(x, y);
        }

        // Convert to center parameterization and add cubics
        self.svg_arc_to_cubics(current.x, current.y, rx.abs(), ry.abs(), x_axis_rotate, large_arc, sweep, x, y);

        self
    }

    /// Add relative line.
    pub fn r_line_to(&mut self, dx: Scalar, dy: Scalar) -> &mut Self {
        let current = self.current_point();
        self.line_to(current.x + dx, current.y + dy)
    }

    /// Add relative move.
    pub fn r_move_to(&mut self, dx: Scalar, dy: Scalar) -> &mut Self {
        let current = self.current_point();
        self.move_to(current.x + dx, current.y + dy)
    }

    /// Add relative quadratic.
    pub fn r_quad_to(&mut self, dx1: Scalar, dy1: Scalar, dx2: Scalar, dy2: Scalar) -> &mut Self {
        let current = self.current_point();
        self.quad_to(
            current.x + dx1,
            current.y + dy1,
            current.x + dx2,
            current.y + dy2,
        )
    }

    /// Add relative cubic.
    pub fn r_cubic_to(
        &mut self,
        dx1: Scalar,
        dy1: Scalar,
        dx2: Scalar,
        dy2: Scalar,
        dx3: Scalar,
        dy3: Scalar,
    ) -> &mut Self {
        let current = self.current_point();
        self.cubic_to(
            current.x + dx1,
            current.y + dy1,
            current.x + dx2,
            current.y + dy2,
            current.x + dx3,
            current.y + dy3,
        )
    }

    /// Add a line from the path.
    pub fn add_line(&mut self, p0: Point, p1: Point) -> &mut Self {
        self.move_to(p0.x, p0.y).line_to(p1.x, p1.y)
    }

    /// Add a polygon.
    pub fn add_polygon(&mut self, points: &[Point], close: bool) -> &mut Self {
        if points.is_empty() {
            return self;
        }

        self.move_to(points[0].x, points[0].y);
        for p in &points[1..] {
            self.line_to(p.x, p.y);
        }
        if close {
            self.close();
        }
        self
    }

    /// Add another path to this builder.
    pub fn add_path(&mut self, path: &Path) -> &mut Self {
        for element in path.iter() {
            match element {
                crate::PathElement::Move(p) => {
                    self.move_to(p.x, p.y);
                }
                crate::PathElement::Line(p) => {
                    self.line_to(p.x, p.y);
                }
                crate::PathElement::Quad(p1, p2) => {
                    self.quad_to(p1.x, p1.y, p2.x, p2.y);
                }
                crate::PathElement::Conic(p1, p2, w) => {
                    self.conic_to(p1.x, p1.y, p2.x, p2.y, w);
                }
                crate::PathElement::Cubic(p1, p2, p3) => {
                    self.cubic_to(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
                }
                crate::PathElement::Close => {
                    self.close();
                }
            }
        }
        self
    }

    /// Build the path.
    #[inline]
    pub fn build(self) -> Path {
        self.path
    }

    /// Get the current point (last point in the path).
    pub fn current_point(&self) -> Point {
        self.path.points.last().copied().unwrap_or(Point::zero())
    }

    /// Ensure we have an initial move command.
    fn ensure_move(&mut self) {
        if self.last_move.is_none() {
            self.move_to(0.0, 0.0);
        }
    }

    /// Internal helper to add arc segments as cubic beziers.
    fn add_arc_to_impl(
        &mut self,
        cx: Scalar,
        cy: Scalar,
        rx: Scalar,
        ry: Scalar,
        start_angle: Scalar,
        sweep_angle: Scalar,
    ) {
        // Break arc into segments of at most 90 degrees
        let num_segments = ((sweep_angle.abs() / (std::f32::consts::FRAC_PI_2)).ceil() as i32).max(1);
        let segment_angle = sweep_angle / num_segments as Scalar;

        let mut angle = start_angle;
        for _ in 0..num_segments {
            let end_angle = angle + segment_angle;
            self.add_arc_segment(cx, cy, rx, ry, angle, end_angle);
            angle = end_angle;
        }
    }

    /// Add a single arc segment (at most 90 degrees) as a cubic bezier.
    fn add_arc_segment(
        &mut self,
        cx: Scalar,
        cy: Scalar,
        rx: Scalar,
        ry: Scalar,
        start_angle: Scalar,
        end_angle: Scalar,
    ) {
        let sweep = end_angle - start_angle;
        let half_sweep = sweep / 2.0;

        // Control point distance factor
        let k = (4.0 / 3.0) * (1.0 - half_sweep.cos()) / half_sweep.sin();

        let (sin_start, cos_start) = start_angle.sin_cos();
        let (sin_end, cos_end) = end_angle.sin_cos();

        let x0 = cx + rx * cos_start;
        let y0 = cy + ry * sin_start;
        let x1 = x0 - k * rx * sin_start;
        let y1 = y0 + k * ry * cos_start;
        let x3 = cx + rx * cos_end;
        let y3 = cy + ry * sin_end;
        let x2 = x3 + k * rx * sin_end;
        let y2 = y3 - k * ry * cos_end;

        self.cubic_to(x1, y1, x2, y2, x3, y3);
    }

    /// Convert SVG arc to cubic bezier segments.
    fn svg_arc_to_cubics(
        &mut self,
        x1: Scalar,
        y1: Scalar,
        mut rx: Scalar,
        mut ry: Scalar,
        phi: Scalar,
        large_arc: bool,
        sweep: bool,
        x2: Scalar,
        y2: Scalar,
    ) {
        // Based on W3C SVG arc implementation notes
        let phi_rad = phi.to_radians();
        let (sin_phi, cos_phi) = phi_rad.sin_cos();

        // Step 1: Compute (x1', y1')
        let dx = (x1 - x2) / 2.0;
        let dy = (y1 - y2) / 2.0;
        let x1p = cos_phi * dx + sin_phi * dy;
        let y1p = -sin_phi * dx + cos_phi * dy;

        // Scale radii if needed
        let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
        if lambda > 1.0 {
            let sqrt_lambda = lambda.sqrt();
            rx *= sqrt_lambda;
            ry *= sqrt_lambda;
        }

        // Step 2: Compute (cx', cy')
        let rx2 = rx * rx;
        let ry2 = ry * ry;
        let x1p2 = x1p * x1p;
        let y1p2 = y1p * y1p;

        let mut sq = ((rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2) / (rx2 * y1p2 + ry2 * x1p2)).max(0.0);
        sq = sq.sqrt();
        if large_arc == sweep {
            sq = -sq;
        }

        let cxp = sq * rx * y1p / ry;
        let cyp = -sq * ry * x1p / rx;

        // Step 3: Compute (cx, cy)
        let cx = cos_phi * cxp - sin_phi * cyp + (x1 + x2) / 2.0;
        let cy = sin_phi * cxp + cos_phi * cyp + (y1 + y2) / 2.0;

        // Step 4: Compute angles
        let theta1 = angle_between(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
        let mut dtheta = angle_between(
            (x1p - cxp) / rx,
            (y1p - cyp) / ry,
            (-x1p - cxp) / rx,
            (-y1p - cyp) / ry,
        );

        if !sweep && dtheta > 0.0 {
            dtheta -= std::f32::consts::TAU;
        } else if sweep && dtheta < 0.0 {
            dtheta += std::f32::consts::TAU;
        }

        // Generate arc segments
        self.add_arc_to_impl(cx, cy, rx, ry, theta1, dtheta);
    }
}

/// Compute angle between two vectors.
fn angle_between(ux: Scalar, uy: Scalar, vx: Scalar, vy: Scalar) -> Scalar {
    let n = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
    if n == 0.0 {
        return 0.0;
    }
    let c = (ux * vx + uy * vy) / n;
    let s = ux * vy - uy * vx;
    s.atan2(c.clamp(-1.0, 1.0))
}
