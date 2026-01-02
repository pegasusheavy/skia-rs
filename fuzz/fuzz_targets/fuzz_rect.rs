#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use skia_rs_core::{Point, Rect, Scalar};

#[derive(Debug, Arbitrary)]
struct RectInput {
    left: Scalar,
    top: Scalar,
    right: Scalar,
    bottom: Scalar,
    // Second rect for operations
    left2: Scalar,
    top2: Scalar,
    right2: Scalar,
    bottom2: Scalar,
    // Point for contains test
    px: Scalar,
    py: Scalar,
    // Offsets
    dx: Scalar,
    dy: Scalar,
}

fuzz_target!(|input: RectInput| {
    // Skip infinite values
    if !input.left.is_finite() || !input.top.is_finite() ||
       !input.right.is_finite() || !input.bottom.is_finite() {
        return;
    }

    let rect1 = Rect::new(input.left, input.top, input.right, input.bottom);
    let rect2 = Rect::new(input.left2, input.top2, input.right2, input.bottom2);

    // Test width/height
    let width = rect1.width();
    let height = rect1.height();
    assert!(width.is_finite());
    assert!(height.is_finite());

    // Test is_empty
    let _ = rect1.is_empty();

    // Test size
    let size = rect1.size();
    assert!(size.width.is_finite());
    assert!(size.height.is_finite());

    // Test contains
    if input.px.is_finite() && input.py.is_finite() {
        let point = Point::new(input.px, input.py);
        let _ = rect1.contains(point);
    }

    // Test intersect
    if input.left2.is_finite() && input.top2.is_finite() &&
       input.right2.is_finite() && input.bottom2.is_finite() {
        let intersection = rect1.intersect(&rect2);
        if let Some(r) = intersection {
            // Intersection should be contained in both rects
            assert!(r.left >= rect1.left.min(rect2.left) - 1e-6);
            assert!(r.right <= rect1.right.max(rect2.right) + 1e-6);
        }
    }

    // Test join
    if input.left2.is_finite() && input.top2.is_finite() &&
       input.right2.is_finite() && input.bottom2.is_finite() {
        let joined = rect1.join(&rect2);
        // Join should contain both rects
        if !rect1.is_empty() && !rect2.is_empty() {
            assert!(joined.left <= rect1.left + 1e-6);
            assert!(joined.left <= rect2.left + 1e-6);
            assert!(joined.right >= rect1.right - 1e-6);
            assert!(joined.right >= rect2.right - 1e-6);
        }
    }

    // Test offset
    if input.dx.is_finite() && input.dy.is_finite() {
        let offset = rect1.offset(input.dx, input.dy);
        assert!(offset.left.is_finite());
        assert!(offset.top.is_finite());
    }

    // Test inset
    if input.dx.is_finite() && input.dy.is_finite() {
        let inset = rect1.inset(input.dx, input.dy);
        let _ = inset; // Just ensure it doesn't panic
    }

    // Test from_xywh
    let from_xywh = Rect::from_xywh(input.left, input.top, width, height);
    let _ = from_xywh;
});
