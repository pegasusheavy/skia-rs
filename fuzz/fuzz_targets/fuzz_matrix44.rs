#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use skia_rs_core::{Matrix44, Point3};

#[derive(Debug, Arbitrary)]
enum TransformOp {
    Translate(f32, f32, f32),
    Scale(f32, f32, f32),
    RotateX(f32),
    RotateY(f32),
    RotateZ(f32),
}

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    operations: Vec<TransformOp>,
    test_point: (f32, f32, f32),
}

fuzz_target!(|input: FuzzInput| {
    // Limit number of operations
    if input.operations.len() > 50 {
        return;
    }

    // Skip invalid inputs
    for op in &input.operations {
        match op {
            TransformOp::Translate(x, y, z) |
            TransformOp::Scale(x, y, z) => {
                if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                    return;
                }
            }
            TransformOp::RotateX(a) |
            TransformOp::RotateY(a) |
            TransformOp::RotateZ(a) => {
                if !a.is_finite() {
                    return;
                }
            }
        }
    }

    if !input.test_point.0.is_finite() || !input.test_point.1.is_finite() || !input.test_point.2.is_finite() {
        return;
    }

    // Build matrix through operations
    let mut matrix = Matrix44::identity();
    for op in input.operations {
        let transform = match op {
            TransformOp::Translate(x, y, z) => Matrix44::translate(x, y, z),
            TransformOp::Scale(x, y, z) => {
                // Avoid degenerate scales
                let sx = if x.abs() < 0.001 { 1.0 } else { x };
                let sy = if y.abs() < 0.001 { 1.0 } else { y };
                let sz = if z.abs() < 0.001 { 1.0 } else { z };
                Matrix44::scale(sx, sy, sz)
            }
            TransformOp::RotateX(a) => Matrix44::rotate_x(a),
            TransformOp::RotateY(a) => Matrix44::rotate_y(a),
            TransformOp::RotateZ(a) => Matrix44::rotate_z(a),
        };
        matrix = matrix.concat(&transform);
    }

    // Test point mapping
    let point = Point3::new(input.test_point.0, input.test_point.1, input.test_point.2);
    let _mapped = matrix.map_point3(point);

    // Test matrix properties
    let _det = matrix.determinant();
    let _transposed = matrix.transpose();

    // Test inversion (may fail for singular matrices)
    if let Some(inverted) = matrix.invert() {
        let identity_ish = matrix.concat(&inverted);
        // The result should be close to identity
        let _ = identity_ish;
    }
});
