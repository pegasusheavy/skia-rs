#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use skia_rs_core::{Point, Scalar};

#[derive(Debug, Arbitrary)]
struct PointInput {
    x1: Scalar,
    y1: Scalar,
    x2: Scalar,
    y2: Scalar,
    scale: Scalar,
}

fuzz_target!(|input: PointInput| {
    // Skip if any values are NaN or infinity (valid but uninteresting)
    if !input.x1.is_finite() || !input.y1.is_finite() ||
       !input.x2.is_finite() || !input.y2.is_finite() ||
       !input.scale.is_finite() {
        return;
    }

    let p1 = Point::new(input.x1, input.y1);
    let p2 = Point::new(input.x2, input.y2);

    // Test addition
    let sum = p1 + p2;
    assert!(sum.x.is_finite() || input.x1.abs() > 1e30 || input.x2.abs() > 1e30);
    assert!(sum.y.is_finite() || input.y1.abs() > 1e30 || input.y2.abs() > 1e30);

    // Test subtraction
    let diff = p1 - p2;
    assert!(diff.x.is_finite() || input.x1.abs() > 1e30 || input.x2.abs() > 1e30);
    assert!(diff.y.is_finite() || input.y1.abs() > 1e30 || input.y2.abs() > 1e30);

    // Test scaling
    let scaled = p1 * input.scale;
    let _ = scaled; // Just ensure it doesn't panic

    // Test length calculation
    let len = p1.length();
    assert!(len >= 0.0 || len.is_nan());

    // Test normalization
    let normalized = p1.normalize();
    if p1.length() > 0.0 {
        let norm_len = normalized.length();
        // Normalized vector should have length ~1 (or 0 for zero vector)
        assert!(norm_len < 1.5 || p1.length() < 1e-10);
    }

    // Test dot product
    let dot = p1.dot(&p2);
    let _ = dot; // Just ensure it doesn't panic

    // Test cross product
    let cross = p1.cross(&p2);
    let _ = cross; // Just ensure it doesn't panic
});
