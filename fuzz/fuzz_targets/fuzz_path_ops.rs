#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use skia_rs_path::{op, PathBuilder, PathOp};
use skia_rs_core::Rect;

#[derive(Debug, Arbitrary)]
struct PathOpsInput {
    // First path segments
    path1_ops: Vec<PathCmd>,
    // Second path segments
    path2_ops: Vec<PathCmd>,
    // Operation to perform
    op: FuzzPathOp,
}

#[derive(Debug, Arbitrary)]
enum PathCmd {
    MoveTo { x: f32, y: f32 },
    LineTo { x: f32, y: f32 },
    QuadTo { cx: f32, cy: f32, x: f32, y: f32 },
    CubicTo { c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32 },
    Close,
    Rect { x: f32, y: f32, w: f32, h: f32 },
    Circle { cx: f32, cy: f32, r: f32 },
}

#[derive(Debug, Arbitrary)]
enum FuzzPathOp {
    Union,
    Intersect,
    Difference,
    Xor,
    ReverseDifference,
}

fn clamp(v: f32) -> f32 {
    if !v.is_finite() {
        0.0
    } else {
        v.clamp(-10000.0, 10000.0)
    }
}

fn build_path(ops: &[PathCmd]) -> skia_rs_path::Path {
    let mut builder = PathBuilder::new();

    for op in ops.iter().take(100) { // Limit complexity
        match op {
            PathCmd::MoveTo { x, y } => {
                builder.move_to(clamp(*x), clamp(*y));
            }
            PathCmd::LineTo { x, y } => {
                builder.line_to(clamp(*x), clamp(*y));
            }
            PathCmd::QuadTo { cx, cy, x, y } => {
                builder.quad_to(clamp(*cx), clamp(*cy), clamp(*x), clamp(*y));
            }
            PathCmd::CubicTo { c1x, c1y, c2x, c2y, x, y } => {
                builder.cubic_to(
                    clamp(*c1x), clamp(*c1y),
                    clamp(*c2x), clamp(*c2y),
                    clamp(*x), clamp(*y),
                );
            }
            PathCmd::Close => {
                builder.close();
            }
            PathCmd::Rect { x, y, w, h } => {
                let w = clamp(*w).abs().max(0.1);
                let h = clamp(*h).abs().max(0.1);
                builder.add_rect(&Rect::from_xywh(clamp(*x), clamp(*y), w, h));
            }
            PathCmd::Circle { cx, cy, r } => {
                let r = clamp(*r).abs().max(0.1);
                builder.add_circle(clamp(*cx), clamp(*cy), r);
            }
        }
    }

    builder.build()
}

fuzz_target!(|input: PathOpsInput| {
    // Limit path complexity
    if input.path1_ops.len() > 100 || input.path2_ops.len() > 100 {
        return;
    }

    let path1 = build_path(&input.path1_ops);
    let path2 = build_path(&input.path2_ops);

    let path_op = match input.op {
        FuzzPathOp::Union => PathOp::Union,
        FuzzPathOp::Intersect => PathOp::Intersect,
        FuzzPathOp::Difference => PathOp::Difference,
        FuzzPathOp::Xor => PathOp::Xor,
        FuzzPathOp::ReverseDifference => PathOp::ReverseDifference,
    };

    // Perform operation - should never panic
    let _ = op(&path1, &path2, path_op);
});
