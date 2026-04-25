//! Path measurement and traversal.

use crate::flatten::{flatten_conic_adaptive, flatten_cubic_adaptive, flatten_quad_adaptive};
use crate::{Path, PathElement};
use skia_rs_core::{Matrix, Point, Scalar};

/// Tolerance used when flattening curves for length measurement.
const FLATTEN_TOLERANCE: Scalar = 0.25;

/// A line segment with cumulative length up to its endpoint.
#[derive(Debug, Clone)]
pub(crate) struct Segment {
    pub(crate) start: Point,
    pub(crate) end: Point,
    pub(crate) length: Scalar,
    /// Cumulative length within the contour up to and including this segment.
    pub(crate) cumulative: Scalar,
}

#[derive(Debug)]
pub(crate) struct Contour {
    pub(crate) segments: Vec<Segment>,
    pub(crate) length: Scalar,
}

/// Measures the length of a path and allows querying points along it.
#[derive(Debug)]
pub struct PathMeasure {
    pub(crate) contours: Vec<Contour>,
    contour_lengths: Vec<Scalar>,
    total_length: Scalar,
}

impl PathMeasure {
    /// Create a new path measure.
    pub fn new(path: &Path) -> Self {
        let mut measure = Self {
            contours: Vec::new(),
            contour_lengths: Vec::new(),
            total_length: 0.0,
        };
        measure.compute_lengths(path);
        measure
    }

    /// Get the total length of the path.
    #[inline]
    pub fn length(&self) -> Scalar {
        self.total_length
    }

    /// Get the number of contours.
    #[inline]
    pub fn contour_count(&self) -> usize {
        self.contour_lengths.len()
    }

    /// Get the length of a specific contour.
    pub fn contour_length(&self, index: usize) -> Option<Scalar> {
        self.contour_lengths.get(index).copied()
    }

    /// Get a point at a distance along the path.
    pub fn get_point_at(&self, distance: Scalar) -> Option<Point> {
        if distance < 0.0 || distance > self.total_length {
            return None;
        }
        let _ = distance;
        None // Implemented in Task 10
    }

    /// Get the tangent at a distance along the path.
    pub fn get_tangent_at(&self, distance: Scalar) -> Option<Point> {
        if distance < 0.0 || distance > self.total_length {
            return None;
        }
        let _ = distance;
        None // Implemented in Task 11
    }

    /// Get the transformation matrix at a distance along the path.
    pub fn get_matrix_at(&self, distance: Scalar) -> Option<Matrix> {
        if distance < 0.0 || distance > self.total_length {
            return None;
        }
        let _ = distance;
        None // Implemented in Task 12
    }

    /// Get a segment of the path.
    pub fn get_segment(&self, start: Scalar, end: Scalar) -> Option<Path> {
        if start >= end || start < 0.0 || end > self.total_length {
            return None;
        }
        let _ = (start, end);
        None // Implemented in Task 13
    }

    fn compute_lengths(&mut self, path: &Path) {
        let mut current_contour: Option<Contour> = None;
        let mut current_pt = Point::new(0.0, 0.0);
        let mut contour_start = Point::new(0.0, 0.0);

        let push_segment = |contour: &mut Contour, start: Point, end: Point| {
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.0 {
                contour.length += len;
                contour.segments.push(Segment {
                    start,
                    end,
                    length: len,
                    cumulative: contour.length,
                });
            }
        };

        for elem in path.iter() {
            match elem {
                PathElement::Move(p) => {
                    if let Some(c) = current_contour.take() {
                        if c.length > 0.0 {
                            self.contour_lengths.push(c.length);
                            self.total_length += c.length;
                            self.contours.push(c);
                        }
                    }
                    current_contour = Some(Contour {
                        segments: Vec::new(),
                        length: 0.0,
                    });
                    current_pt = p;
                    contour_start = p;
                }
                PathElement::Line(p) => {
                    if let Some(c) = current_contour.as_mut() {
                        push_segment(c, current_pt, p);
                    }
                    current_pt = p;
                }
                PathElement::Quad(ctrl, end) => {
                    if let Some(c) = current_contour.as_mut() {
                        let mut points = Vec::new();
                        flatten_quad_adaptive(&mut points, current_pt, ctrl, end, FLATTEN_TOLERANCE);
                        let mut prev = current_pt;
                        for p in points {
                            push_segment(c, prev, p);
                            prev = p;
                        }
                    }
                    current_pt = end;
                }
                PathElement::Cubic(c1, c2, end) => {
                    if let Some(c) = current_contour.as_mut() {
                        let mut points = Vec::new();
                        flatten_cubic_adaptive(&mut points, current_pt, c1, c2, end, FLATTEN_TOLERANCE);
                        let mut prev = current_pt;
                        for p in points {
                            push_segment(c, prev, p);
                            prev = p;
                        }
                    }
                    current_pt = end;
                }
                PathElement::Conic(ctrl, end, w) => {
                    if let Some(c) = current_contour.as_mut() {
                        let mut points = Vec::new();
                        flatten_conic_adaptive(&mut points, current_pt, ctrl, end, w, FLATTEN_TOLERANCE);
                        let mut prev = current_pt;
                        for p in points {
                            push_segment(c, prev, p);
                            prev = p;
                        }
                    }
                    current_pt = end;
                }
                PathElement::Close => {
                    if let Some(c) = current_contour.as_mut() {
                        if current_pt != contour_start {
                            push_segment(c, current_pt, contour_start);
                        }
                    }
                    current_pt = contour_start;
                }
            }
        }

        if let Some(c) = current_contour.take() {
            if c.length > 0.0 {
                self.contour_lengths.push(c.length);
                self.total_length += c.length;
                self.contours.push(c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PathBuilder;

    #[test]
    fn test_path_measure_empty_path() {
        let path = PathBuilder::new().build();
        let measure = PathMeasure::new(&path);
        assert_eq!(measure.length(), 0.0);
        assert_eq!(measure.contour_count(), 0);
    }

    #[test]
    fn test_path_measure_single_line() {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(100.0, 0.0);
        let path = builder.build();
        let measure = PathMeasure::new(&path);
        assert!((measure.length() - 100.0).abs() < 0.01);
        assert_eq!(measure.contour_count(), 1);
    }

    #[test]
    fn test_path_measure_diagonal_line() {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(3.0, 4.0);
        let path = builder.build();
        let measure = PathMeasure::new(&path);
        assert!((measure.length() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_path_measure_multiple_lines() {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(10.0, 0.0);
        builder.line_to(10.0, 10.0);
        builder.line_to(0.0, 10.0);
        let path = builder.build();
        let measure = PathMeasure::new(&path);
        assert!((measure.length() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_path_measure_quadratic_curve() {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.quad_to(50.0, 0.0, 100.0, 0.0);
        let path = builder.build();
        let measure = PathMeasure::new(&path);
        assert!((measure.length() - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_path_measure_multi_contour() {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(10.0, 0.0);
        builder.move_to(20.0, 0.0);
        builder.line_to(50.0, 0.0);
        let path = builder.build();
        let measure = PathMeasure::new(&path);
        assert!((measure.length() - 40.0).abs() < 0.01);
        assert_eq!(measure.contour_count(), 2);
        assert!((measure.contour_length(0).unwrap() - 10.0).abs() < 0.01);
        assert!((measure.contour_length(1).unwrap() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_path_measure_closed_contour() {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(10.0, 0.0);
        builder.line_to(10.0, 10.0);
        builder.line_to(0.0, 10.0);
        builder.close();
        let path = builder.build();
        let measure = PathMeasure::new(&path);
        // Close adds the segment from (0,10) back to (0,0), length 10
        assert!((measure.length() - 40.0).abs() < 0.01);
    }
}
