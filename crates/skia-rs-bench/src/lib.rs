//! Benchmark utilities and helpers for skia-rs.
//!
//! This crate provides benchmark harnesses and test data generators
//! for performance testing skia-rs components.

pub mod dm;
pub mod memory;
pub mod skia_comparison;

use rand::Rng;
use rand_xorshift::XorShiftRng;
use skia_rs_core::{Color, Color4f, Matrix, Point, Rect, Scalar};
use skia_rs_path::{Path, PathBuilder};

/// Create a deterministic RNG for reproducible benchmarks.
pub fn create_rng() -> XorShiftRng {
    use rand::SeedableRng;
    XorShiftRng::seed_from_u64(0xDEAD_BEEF_CAFE_BABE)
}

/// Generate random points within bounds.
pub fn random_points(rng: &mut impl Rng, count: usize, bounds: &Rect) -> Vec<Point> {
    (0..count)
        .map(|_| {
            Point::new(
                rng.gen_range(bounds.left..bounds.right),
                rng.gen_range(bounds.top..bounds.bottom),
            )
        })
        .collect()
}

/// Generate random rectangles within bounds.
pub fn random_rects(
    rng: &mut impl Rng,
    count: usize,
    bounds: &Rect,
    max_size: Scalar,
) -> Vec<Rect> {
    (0..count)
        .map(|_| {
            let x = rng.gen_range(bounds.left..bounds.right - max_size);
            let y = rng.gen_range(bounds.top..bounds.bottom - max_size);
            let w = rng.gen_range(1.0..max_size);
            let h = rng.gen_range(1.0..max_size);
            Rect::from_xywh(x, y, w, h)
        })
        .collect()
}

/// Generate random colors.
pub fn random_colors(rng: &mut impl Rng, count: usize) -> Vec<Color> {
    (0..count)
        .map(|_| Color::from_argb(rng.r#gen(), rng.r#gen(), rng.r#gen(), rng.r#gen()))
        .collect()
}

/// Generate random Color4f values.
pub fn random_colors4f(rng: &mut impl Rng, count: usize) -> Vec<Color4f> {
    (0..count)
        .map(|_| {
            Color4f::new(
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
                rng.gen_range(0.0..1.0),
            )
        })
        .collect()
}

/// Generate random transformation matrices.
pub fn random_matrices(rng: &mut impl Rng, count: usize) -> Vec<Matrix> {
    (0..count)
        .map(|_| {
            let tx = rng.gen_range(-1000.0..1000.0);
            let ty = rng.gen_range(-1000.0..1000.0);
            let sx = rng.gen_range(0.1..10.0);
            let sy = rng.gen_range(0.1..10.0);
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);

            Matrix::translate(tx, ty)
                .concat(&Matrix::scale(sx, sy))
                .concat(&Matrix::rotate(angle))
        })
        .collect()
}

/// Generate a simple path with the given number of segments.
pub fn generate_simple_path(segment_count: usize) -> Path {
    let mut builder = PathBuilder::new();
    builder.move_to(0.0, 0.0);

    for i in 0..segment_count {
        let x = (i as Scalar + 1.0) * 10.0;
        let y = if i % 2 == 0 { 50.0 } else { 0.0 };
        builder.line_to(x, y);
    }

    builder.close();
    builder.build()
}

/// Generate a path with mixed curve types.
pub fn generate_complex_path(segment_count: usize) -> Path {
    let mut builder = PathBuilder::new();
    let mut rng = create_rng();

    builder.move_to(0.0, 0.0);

    for i in 0..segment_count {
        let x = (i as Scalar + 1.0) * 20.0;
        let y = rng.gen_range(-50.0..50.0);

        match i % 4 {
            0 => builder.line_to(x, y),
            1 => builder.quad_to(x - 10.0, y + 20.0, x, y),
            2 => builder.cubic_to(x - 15.0, y - 10.0, x - 5.0, y + 30.0, x, y),
            _ => builder.conic_to(x - 10.0, y + 15.0, x, y, 0.707),
        };
    }

    builder.close();
    builder.build()
}

/// Generate a path with multiple contours.
pub fn generate_multi_contour_path(contour_count: usize, segments_per_contour: usize) -> Path {
    let mut builder = PathBuilder::new();
    let mut rng = create_rng();

    for c in 0..contour_count {
        let offset_x = (c as Scalar) * 100.0;
        let offset_y = (c as Scalar) * 50.0;

        builder.move_to(offset_x, offset_y);

        for i in 0..segments_per_contour {
            let x = offset_x + (i as Scalar + 1.0) * 10.0;
            let y = offset_y + rng.gen_range(-20.0..20.0);
            builder.line_to(x, y);
        }

        builder.close();
    }

    builder.build()
}

/// Generate nested rectangles path.
pub fn generate_nested_rects(count: usize, spacing: Scalar) -> Path {
    let mut builder = PathBuilder::new();

    for i in 0..count {
        let offset = i as Scalar * spacing;
        let rect = Rect::new(offset, offset, 100.0 - offset, 100.0 - offset);
        builder.add_rect(&rect);
    }

    builder.build()
}

/// Generate a star path.
pub fn generate_star(points: usize, outer_radius: Scalar, inner_radius: Scalar) -> Path {
    let mut builder = PathBuilder::new();

    let angle_step = std::f32::consts::TAU / (points as Scalar * 2.0);

    for i in 0..(points * 2) {
        let radius = if i % 2 == 0 {
            outer_radius
        } else {
            inner_radius
        };
        let angle = (i as Scalar) * angle_step - std::f32::consts::FRAC_PI_2;
        let x = radius * angle.cos();
        let y = radius * angle.sin();

        if i == 0 {
            builder.move_to(x, y);
        } else {
            builder.line_to(x, y);
        }
    }

    builder.close();
    builder.build()
}

/// Benchmark data sizes for different test scenarios.
pub mod sizes {
    /// Small data set for quick benchmarks.
    pub const SMALL: usize = 100;
    /// Medium data set.
    pub const MEDIUM: usize = 1_000;
    /// Large data set.
    pub const LARGE: usize = 10_000;
    /// Extra large data set for stress testing.
    pub const XLARGE: usize = 100_000;
}

/// Standard canvas sizes for benchmarks.
pub mod canvas_sizes {
    /// Small canvas (icon size).
    pub const SMALL: (i32, i32) = (64, 64);
    /// Medium canvas (thumbnail).
    pub const MEDIUM: (i32, i32) = (256, 256);
    /// HD canvas.
    pub const HD: (i32, i32) = (1920, 1080);
    /// 4K canvas.
    pub const UHD: (i32, i32) = (3840, 2160);
}
