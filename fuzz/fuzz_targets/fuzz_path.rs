#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use skia_rs_core::Scalar;
use skia_rs_path::{FillType, Path, PathBuilder};

#[derive(Debug, Arbitrary)]
enum PathCommand {
    MoveTo { x: Scalar, y: Scalar },
    LineTo { x: Scalar, y: Scalar },
    QuadTo { x1: Scalar, y1: Scalar, x2: Scalar, y2: Scalar },
    CubicTo { x1: Scalar, y1: Scalar, x2: Scalar, y2: Scalar, x3: Scalar, y3: Scalar },
    ConicTo { x1: Scalar, y1: Scalar, x2: Scalar, y2: Scalar, w: Scalar },
    Close,
}

#[derive(Debug, Arbitrary)]
struct PathInput {
    commands: Vec<PathCommand>,
    fill_type: u8, // 0-3 for fill types
}

fn is_valid(v: Scalar) -> bool {
    v.is_finite() && v.abs() < 1e6
}

fuzz_target!(|input: PathInput| {
    // Limit command count to prevent OOM
    if input.commands.len() > 1000 {
        return;
    }

    let mut builder = PathBuilder::new();

    // Set fill type
    let fill_type = match input.fill_type % 4 {
        0 => FillType::Winding,
        1 => FillType::EvenOdd,
        2 => FillType::InverseWinding,
        _ => FillType::InverseEvenOdd,
    };
    builder.fill_type(fill_type);

    // Apply commands
    for cmd in &input.commands {
        match cmd {
            PathCommand::MoveTo { x, y } if is_valid(*x) && is_valid(*y) => {
                builder.move_to(*x, *y);
            }
            PathCommand::LineTo { x, y } if is_valid(*x) && is_valid(*y) => {
                builder.line_to(*x, *y);
            }
            PathCommand::QuadTo { x1, y1, x2, y2 }
                if is_valid(*x1) && is_valid(*y1) && is_valid(*x2) && is_valid(*y2) => {
                builder.quad_to(*x1, *y1, *x2, *y2);
            }
            PathCommand::CubicTo { x1, y1, x2, y2, x3, y3 }
                if is_valid(*x1) && is_valid(*y1) && is_valid(*x2) && is_valid(*y2)
                   && is_valid(*x3) && is_valid(*y3) => {
                builder.cubic_to(*x1, *y1, *x2, *y2, *x3, *y3);
            }
            PathCommand::ConicTo { x1, y1, x2, y2, w }
                if is_valid(*x1) && is_valid(*y1) && is_valid(*x2) && is_valid(*y2)
                   && is_valid(*w) && *w > 0.0 => {
                builder.conic_to(*x1, *y1, *x2, *y2, *w);
            }
            PathCommand::Close => {
                builder.close();
            }
            _ => {} // Skip invalid coordinates
        }
    }

    // Build the path
    let path = builder.build();

    // Test path queries
    let _ = path.is_empty();
    let _ = path.verb_count();
    let _ = path.point_count();
    let _ = path.fill_type();

    // Test bounds computation
    let bounds = path.bounds();
    if !path.is_empty() {
        assert!(bounds.left.is_finite() || path.point_count() == 0);
        assert!(bounds.right.is_finite() || path.point_count() == 0);
    }

    // Test iteration
    let mut iter_count = 0;
    for element in path.iter() {
        let _ = element;
        iter_count += 1;
        if iter_count > 10000 {
            break; // Safety limit
        }
    }

    // Test slices
    let _ = path.verbs();
    let _ = path.points();

    // Test clone
    let cloned = path.clone();
    assert_eq!(cloned.verb_count(), path.verb_count());
    assert_eq!(cloned.point_count(), path.point_count());
});
