//! Path utility functions.
//!
//! This module provides utility functions for path manipulation,
//! including stroke-to-fill conversion.

use crate::{Path, PathBuilder, PathElement};
use skia_rs_core::{Point, Scalar};

/// Stroke cap style for stroke-to-fill conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum StrokeCap {
    /// Flat cap - no extension beyond the endpoint.
    #[default]
    Butt = 0,
    /// Round cap - semicircle at each endpoint.
    Round,
    /// Square cap - extends by half the stroke width.
    Square,
}

/// Stroke join style for stroke-to-fill conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum StrokeJoin {
    /// Miter join - sharp corners.
    #[default]
    Miter = 0,
    /// Round join - rounded corners.
    Round,
    /// Bevel join - flat corners.
    Bevel,
}

/// Parameters for stroke-to-fill conversion.
#[derive(Debug, Clone)]
pub struct StrokeParams {
    /// Stroke width.
    pub width: Scalar,
    /// Stroke cap style.
    pub cap: StrokeCap,
    /// Stroke join style.
    pub join: StrokeJoin,
    /// Miter limit (for miter joins).
    pub miter_limit: Scalar,
}

impl Default for StrokeParams {
    fn default() -> Self {
        Self {
            width: 1.0,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
            miter_limit: 4.0,
        }
    }
}

impl StrokeParams {
    /// Create new stroke parameters.
    pub fn new(width: Scalar) -> Self {
        Self {
            width,
            ..Default::default()
        }
    }

    /// Set the stroke cap.
    pub fn with_cap(mut self, cap: StrokeCap) -> Self {
        self.cap = cap;
        self
    }

    /// Set the stroke join.
    pub fn with_join(mut self, join: StrokeJoin) -> Self {
        self.join = join;
        self
    }

    /// Set the miter limit.
    pub fn with_miter_limit(mut self, limit: Scalar) -> Self {
        self.miter_limit = limit;
        self
    }
}

/// Convert a stroked path to a filled path.
///
/// This creates an outline around the input path that, when filled,
/// would produce the same visual result as stroking the original path.
///
/// # Arguments
/// * `path` - The input path to stroke.
/// * `params` - Stroke parameters (width, cap, join, miter limit).
///
/// # Returns
/// The stroked path as a fillable outline, or `None` if the path is empty.
pub fn stroke_to_fill(path: &Path, params: &StrokeParams) -> Option<Path> {
    if path.is_empty() || params.width <= 0.0 {
        return None;
    }

    let half_width = params.width / 2.0;
    let mut builder = PathBuilder::new();

    // Collect path elements into contours
    let mut contours: Vec<Vec<Point>> = Vec::new();
    let mut current_contour: Vec<Point> = Vec::new();
    let mut is_closed = false;

    for element in path.iter() {
        match element {
            PathElement::Move(p) => {
                if !current_contour.is_empty() {
                    contours.push(std::mem::take(&mut current_contour));
                }
                current_contour.push(p);
                is_closed = false;
            }
            PathElement::Line(p) => {
                current_contour.push(p);
            }
            PathElement::Quad(ctrl, end) => {
                // Flatten quadratic to lines
                if let Some(&start) = current_contour.last() {
                    flatten_quad(&mut current_contour, start, ctrl, end, 4);
                }
            }
            PathElement::Cubic(ctrl1, ctrl2, end) => {
                // Flatten cubic to lines
                if let Some(&start) = current_contour.last() {
                    flatten_cubic(&mut current_contour, start, ctrl1, ctrl2, end, 8);
                }
            }
            PathElement::Conic(ctrl, end, weight) => {
                // Flatten conic to lines (approximate as quad)
                if let Some(&start) = current_contour.last() {
                    let mid_ctrl = Point::new(
                        start.x * (1.0 - weight) / 2.0 + ctrl.x * weight + end.x * (1.0 - weight) / 2.0,
                        start.y * (1.0 - weight) / 2.0 + ctrl.y * weight + end.y * (1.0 - weight) / 2.0,
                    );
                    flatten_quad(&mut current_contour, start, mid_ctrl, end, 4);
                }
            }
            PathElement::Close => {
                is_closed = true;
            }
        }
    }

    if !current_contour.is_empty() {
        contours.push(current_contour);
    }

    // Process each contour
    for contour in &contours {
        if contour.len() < 2 {
            continue;
        }

        stroke_contour(&mut builder, contour, is_closed, half_width, params);
    }

    Some(builder.build())
}

fn stroke_contour(
    builder: &mut PathBuilder,
    points: &[Point],
    is_closed: bool,
    half_width: Scalar,
    params: &StrokeParams,
) {
    if points.len() < 2 {
        return;
    }

    let n = points.len();

    // Compute normals for each segment
    let mut normals: Vec<Point> = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let dx = points[i + 1].x - points[i].x;
        let dy = points[i + 1].y - points[i].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.0 {
            normals.push(Point::new(-dy / len, dx / len));
        } else {
            normals.push(Point::new(0.0, 1.0));
        }
    }

    if normals.is_empty() {
        return;
    }

    // Build left side (offset by +half_width)
    let mut left_side: Vec<Point> = Vec::with_capacity(n);
    // Build right side (offset by -half_width)
    let mut right_side: Vec<Point> = Vec::with_capacity(n);

    // First point
    let first_normal = normals[0];
    left_side.push(Point::new(
        points[0].x + first_normal.x * half_width,
        points[0].y + first_normal.y * half_width,
    ));
    right_side.push(Point::new(
        points[0].x - first_normal.x * half_width,
        points[0].y - first_normal.y * half_width,
    ));

    // Interior points with join handling
    for i in 1..n - 1 {
        let n1 = normals[i - 1];
        let n2 = normals[i];

        // Average normal for the join
        let avg = Point::new(n1.x + n2.x, n1.y + n2.y);
        let avg_len = avg.length();

        if avg_len > 0.001 {
            let scale = half_width / avg_len;
            let offset = Point::new(avg.x * scale, avg.y * scale);

            match params.join {
                StrokeJoin::Miter => {
                    // Compute miter length
                    let miter_len = 1.0 / (avg_len / 2.0);
                    if miter_len <= params.miter_limit {
                        left_side.push(Point::new(
                            points[i].x + offset.x * miter_len,
                            points[i].y + offset.y * miter_len,
                        ));
                        right_side.push(Point::new(
                            points[i].x - offset.x * miter_len,
                            points[i].y - offset.y * miter_len,
                        ));
                    } else {
                        // Fallback to bevel
                        left_side.push(Point::new(
                            points[i].x + n1.x * half_width,
                            points[i].y + n1.y * half_width,
                        ));
                        left_side.push(Point::new(
                            points[i].x + n2.x * half_width,
                            points[i].y + n2.y * half_width,
                        ));
                        right_side.push(Point::new(
                            points[i].x - n1.x * half_width,
                            points[i].y - n1.y * half_width,
                        ));
                        right_side.push(Point::new(
                            points[i].x - n2.x * half_width,
                            points[i].y - n2.y * half_width,
                        ));
                    }
                }
                StrokeJoin::Bevel => {
                    left_side.push(Point::new(
                        points[i].x + n1.x * half_width,
                        points[i].y + n1.y * half_width,
                    ));
                    left_side.push(Point::new(
                        points[i].x + n2.x * half_width,
                        points[i].y + n2.y * half_width,
                    ));
                    right_side.push(Point::new(
                        points[i].x - n1.x * half_width,
                        points[i].y - n1.y * half_width,
                    ));
                    right_side.push(Point::new(
                        points[i].x - n2.x * half_width,
                        points[i].y - n2.y * half_width,
                    ));
                }
                StrokeJoin::Round => {
                    // Simplified: use multiple points to approximate round join
                    left_side.push(Point::new(
                        points[i].x + offset.x,
                        points[i].y + offset.y,
                    ));
                    right_side.push(Point::new(
                        points[i].x - offset.x,
                        points[i].y - offset.y,
                    ));
                }
            }
        } else {
            // Parallel segments, use normal offset
            left_side.push(Point::new(
                points[i].x + n1.x * half_width,
                points[i].y + n1.y * half_width,
            ));
            right_side.push(Point::new(
                points[i].x - n1.x * half_width,
                points[i].y - n1.y * half_width,
            ));
        }
    }

    // Last point
    let last_normal = normals[normals.len() - 1];
    left_side.push(Point::new(
        points[n - 1].x + last_normal.x * half_width,
        points[n - 1].y + last_normal.y * half_width,
    ));
    right_side.push(Point::new(
        points[n - 1].x - last_normal.x * half_width,
        points[n - 1].y - last_normal.y * half_width,
    ));

    // Build the outline path
    if is_closed {
        // For closed paths, connect left to right
        if !left_side.is_empty() {
            builder.move_to(left_side[0].x, left_side[0].y);
            for p in &left_side[1..] {
                builder.line_to(p.x, p.y);
            }
            builder.close();
        }
        if !right_side.is_empty() {
            builder.move_to(right_side[0].x, right_side[0].y);
            for p in &right_side[1..] {
                builder.line_to(p.x, p.y);
            }
            builder.close();
        }
    } else {
        // For open paths, create a single outline with caps
        if !left_side.is_empty() {
            builder.move_to(left_side[0].x, left_side[0].y);

            // Add start cap
            add_cap(builder, points[0], normals[0], half_width, params.cap, true);

            // Left side (forward)
            for p in &left_side {
                builder.line_to(p.x, p.y);
            }

            // Add end cap
            add_cap(builder, points[n - 1], normals[normals.len() - 1], half_width, params.cap, false);

            // Right side (reverse)
            for p in right_side.iter().rev() {
                builder.line_to(p.x, p.y);
            }

            builder.close();
        }
    }
}

fn add_cap(
    builder: &mut PathBuilder,
    center: Point,
    normal: Point,
    half_width: Scalar,
    cap: StrokeCap,
    is_start: bool,
) {
    match cap {
        StrokeCap::Butt => {
            // No extension
        }
        StrokeCap::Square => {
            // Extend by half_width in the direction perpendicular to normal
            let dir = if is_start {
                Point::new(-normal.y, normal.x)
            } else {
                Point::new(normal.y, -normal.x)
            };
            let ext = Point::new(dir.x * half_width, dir.y * half_width);
            builder.line_to(center.x + normal.x * half_width + ext.x, center.y + normal.y * half_width + ext.y);
            builder.line_to(center.x - normal.x * half_width + ext.x, center.y - normal.y * half_width + ext.y);
        }
        StrokeCap::Round => {
            // Approximate semicircle with line segments
            let steps = 8;
            let start_angle = if is_start {
                normal.y.atan2(normal.x)
            } else {
                (-normal.y).atan2(-normal.x)
            };

            for i in 0..=steps {
                let t = i as Scalar / steps as Scalar;
                let angle = start_angle + t * std::f32::consts::PI;
                let x = center.x + angle.cos() * half_width;
                let y = center.y + angle.sin() * half_width;
                builder.line_to(x, y);
            }
        }
    }
}

fn flatten_quad(points: &mut Vec<Point>, p0: Point, p1: Point, p2: Point, steps: usize) {
    for i in 1..=steps {
        let t = i as Scalar / steps as Scalar;
        let mt = 1.0 - t;
        let x = mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x;
        let y = mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y;
        points.push(Point::new(x, y));
    }
}

fn flatten_cubic(points: &mut Vec<Point>, p0: Point, p1: Point, p2: Point, p3: Point, steps: usize) {
    for i in 1..=steps {
        let t = i as Scalar / steps as Scalar;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let t2 = t * t;
        let x = mt2 * mt * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t2 * t * p3.x;
        let y = mt2 * mt * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t2 * t * p3.y;
        points.push(Point::new(x, y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stroke_to_fill_line() {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(100.0, 0.0);
        let path = builder.build();

        let params = StrokeParams::new(10.0);
        let stroked = stroke_to_fill(&path, &params).unwrap();

        assert!(!stroked.is_empty());
    }

    #[test]
    fn test_stroke_to_fill_triangle() {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(100.0, 0.0);
        builder.line_to(50.0, 100.0);
        builder.close();
        let path = builder.build();

        let params = StrokeParams::new(5.0)
            .with_join(StrokeJoin::Round);
        let stroked = stroke_to_fill(&path, &params).unwrap();

        assert!(!stroked.is_empty());
    }

    #[test]
    fn test_stroke_params() {
        let params = StrokeParams::new(2.0)
            .with_cap(StrokeCap::Round)
            .with_join(StrokeJoin::Bevel)
            .with_miter_limit(10.0);

        assert_eq!(params.width, 2.0);
        assert_eq!(params.cap, StrokeCap::Round);
        assert_eq!(params.join, StrokeJoin::Bevel);
        assert_eq!(params.miter_limit, 10.0);
    }
}
