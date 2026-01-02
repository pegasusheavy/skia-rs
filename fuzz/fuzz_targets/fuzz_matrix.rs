#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use skia_rs_core::{Matrix, Point, Rect, Scalar};

#[derive(Debug, Arbitrary)]
struct MatrixInput {
    // Matrix elements
    scale_x: Scalar,
    skew_x: Scalar,
    trans_x: Scalar,
    skew_y: Scalar,
    scale_y: Scalar,
    trans_y: Scalar,
    persp_0: Scalar,
    persp_1: Scalar,
    persp_2: Scalar,
    // Point to transform
    px: Scalar,
    py: Scalar,
    // Rect to transform
    left: Scalar,
    top: Scalar,
    right: Scalar,
    bottom: Scalar,
    // Transform parameters
    angle: Scalar,
    sx: Scalar,
    sy: Scalar,
    kx: Scalar,
    ky: Scalar,
}

fn is_reasonable(v: Scalar) -> bool {
    v.is_finite() && v.abs() < 1e10
}

fuzz_target!(|input: MatrixInput| {
    // Test translation
    if is_reasonable(input.trans_x) && is_reasonable(input.trans_y) {
        let translate = Matrix::translate(input.trans_x, input.trans_y);
        assert!(translate.is_translate());
    }

    // Test scale
    if is_reasonable(input.sx) && is_reasonable(input.sy) {
        let scale = Matrix::scale(input.sx, input.sy);
        let _ = scale;
    }

    // Test rotation
    if is_reasonable(input.angle) {
        let rotate = Matrix::rotate(input.angle);
        let _ = rotate;
    }

    // Test rotation around point
    if is_reasonable(input.angle) && is_reasonable(input.px) && is_reasonable(input.py) {
        let pivot = Point::new(input.px, input.py);
        let rotate_around = Matrix::rotate_around(input.angle, pivot);
        let _ = rotate_around;
    }

    // Test skew
    if is_reasonable(input.kx) && is_reasonable(input.ky) {
        let skew = Matrix::skew(input.kx, input.ky);
        let _ = skew;
    }

    // Test matrix concatenation
    if is_reasonable(input.trans_x) && is_reasonable(input.trans_y) &&
       is_reasonable(input.sx) && is_reasonable(input.sy) {
        let m1 = Matrix::translate(input.trans_x, input.trans_y);
        let m2 = Matrix::scale(input.sx, input.sy);
        let concat = m1.concat(&m2);
        let _ = concat;
    }

    // Test map_point
    if is_reasonable(input.px) && is_reasonable(input.py) &&
       is_reasonable(input.trans_x) && is_reasonable(input.trans_y) {
        let m = Matrix::translate(input.trans_x, input.trans_y);
        let p = Point::new(input.px, input.py);
        let mapped = m.map_point(p);
        let _ = mapped;
    }

    // Test map_rect
    if is_reasonable(input.left) && is_reasonable(input.top) &&
       is_reasonable(input.right) && is_reasonable(input.bottom) &&
       is_reasonable(input.trans_x) && is_reasonable(input.trans_y) {
        let m = Matrix::translate(input.trans_x, input.trans_y);
        let r = Rect::new(input.left, input.top, input.right, input.bottom);
        let mapped = m.map_rect(&r);
        let _ = mapped;
    }

    // Test inversion
    if is_reasonable(input.trans_x) && is_reasonable(input.trans_y) &&
       is_reasonable(input.sx) && is_reasonable(input.sy) &&
       input.sx != 0.0 && input.sy != 0.0 {
        let m = Matrix::translate(input.trans_x, input.trans_y)
            .concat(&Matrix::scale(input.sx, input.sy));

        if let Some(inverse) = m.invert() {
            // Verify round-trip (M * M^-1 ≈ I)
            let product = m.concat(&inverse);
            // Scale components should be near 1
            assert!((product.scale_x() - 1.0).abs() < 1e-3 || !product.scale_x().is_finite());
        }
    }

    // Test identity checks
    let identity = Matrix::IDENTITY;
    assert!(identity.is_identity());
    assert!(identity.is_translate());
});
