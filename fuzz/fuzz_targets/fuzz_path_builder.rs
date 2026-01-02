#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use skia_rs_core::{Rect, Scalar};
use skia_rs_path::PathBuilder;

#[derive(Debug, Arbitrary)]
enum ShapeCommand {
    Rect { left: Scalar, top: Scalar, right: Scalar, bottom: Scalar },
    Oval { left: Scalar, top: Scalar, right: Scalar, bottom: Scalar },
    Circle { cx: Scalar, cy: Scalar, radius: Scalar },
    RoundRect { left: Scalar, top: Scalar, right: Scalar, bottom: Scalar, rx: Scalar, ry: Scalar },
}

#[derive(Debug, Arbitrary)]
struct ShapeInput {
    shapes: Vec<ShapeCommand>,
}

fn is_valid(v: Scalar) -> bool {
    v.is_finite() && v.abs() < 1e6
}

fuzz_target!(|input: ShapeInput| {
    // Limit shape count
    if input.shapes.len() > 100 {
        return;
    }

    let mut builder = PathBuilder::new();

    for shape in &input.shapes {
        match shape {
            ShapeCommand::Rect { left, top, right, bottom }
                if is_valid(*left) && is_valid(*top) && is_valid(*right) && is_valid(*bottom) => {
                let rect = Rect::new(*left, *top, *right, *bottom);
                builder.add_rect(&rect);
            }
            ShapeCommand::Oval { left, top, right, bottom }
                if is_valid(*left) && is_valid(*top) && is_valid(*right) && is_valid(*bottom) => {
                let rect = Rect::new(*left, *top, *right, *bottom);
                builder.add_oval(&rect);
            }
            ShapeCommand::Circle { cx, cy, radius }
                if is_valid(*cx) && is_valid(*cy) && is_valid(*radius) && *radius >= 0.0 => {
                builder.add_circle(*cx, *cy, *radius);
            }
            ShapeCommand::RoundRect { left, top, right, bottom, rx, ry }
                if is_valid(*left) && is_valid(*top) && is_valid(*right) && is_valid(*bottom)
                   && is_valid(*rx) && is_valid(*ry) => {
                let rect = Rect::new(*left, *top, *right, *bottom);
                builder.add_round_rect(&rect, *rx, *ry);
            }
            _ => {} // Skip invalid inputs
        }
    }

    // Build and verify
    let path = builder.build();

    // Verify path integrity
    let _ = path.is_empty();
    let _ = path.bounds();
    let _ = path.verb_count();

    // Iteration should work
    for element in path.iter() {
        let _ = element;
    }
});
