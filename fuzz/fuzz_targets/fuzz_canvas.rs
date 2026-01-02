#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use skia_rs_core::{Color, Point, Rect, Scalar};
use skia_rs_canvas::{Canvas, ClipOp};
use skia_rs_paint::Paint;

#[derive(Debug, Arbitrary)]
enum CanvasCommand {
    Save,
    Restore,
    Translate { dx: Scalar, dy: Scalar },
    Scale { sx: Scalar, sy: Scalar },
    Rotate { degrees: Scalar },
    Skew { sx: Scalar, sy: Scalar },
    ClipRect { left: Scalar, top: Scalar, right: Scalar, bottom: Scalar, intersect: bool },
    Clear { color: u32 },
    DrawRect { left: Scalar, top: Scalar, right: Scalar, bottom: Scalar },
    DrawOval { left: Scalar, top: Scalar, right: Scalar, bottom: Scalar },
    DrawCircle { cx: Scalar, cy: Scalar, radius: Scalar },
    DrawLine { x0: Scalar, y0: Scalar, x1: Scalar, y1: Scalar },
    DrawRoundRect { left: Scalar, top: Scalar, right: Scalar, bottom: Scalar, rx: Scalar, ry: Scalar },
    ResetMatrix,
}

#[derive(Debug, Arbitrary)]
struct CanvasInput {
    width: u16,
    height: u16,
    commands: Vec<CanvasCommand>,
}

fn is_valid(v: Scalar) -> bool {
    v.is_finite() && v.abs() < 1e6
}

fuzz_target!(|input: CanvasInput| {
    // Limit dimensions and command count
    let width = (input.width as i32).clamp(1, 4096);
    let height = (input.height as i32).clamp(1, 4096);

    if input.commands.len() > 500 {
        return;
    }

    let mut canvas = Canvas::new(width, height);
    let paint = Paint::new();

    // Verify initial state
    assert_eq!(canvas.width(), width);
    assert_eq!(canvas.height(), height);
    assert_eq!(canvas.save_count(), 1);

    let mut save_depth = 0i32;

    for cmd in &input.commands {
        match cmd {
            CanvasCommand::Save => {
                canvas.save();
                save_depth += 1;
            }
            CanvasCommand::Restore => {
                if save_depth > 0 {
                    canvas.restore();
                    save_depth -= 1;
                }
            }
            CanvasCommand::Translate { dx, dy } if is_valid(*dx) && is_valid(*dy) => {
                canvas.translate(*dx, *dy);
            }
            CanvasCommand::Scale { sx, sy } if is_valid(*sx) && is_valid(*sy) => {
                canvas.scale(*sx, *sy);
            }
            CanvasCommand::Rotate { degrees } if is_valid(*degrees) => {
                canvas.rotate(*degrees);
            }
            CanvasCommand::Skew { sx, sy } if is_valid(*sx) && is_valid(*sy) => {
                canvas.skew(*sx, *sy);
            }
            CanvasCommand::ClipRect { left, top, right, bottom, intersect }
                if is_valid(*left) && is_valid(*top) && is_valid(*right) && is_valid(*bottom) => {
                let rect = Rect::new(*left, *top, *right, *bottom);
                let op = if *intersect { ClipOp::Intersect } else { ClipOp::Difference };
                canvas.clip_rect(&rect, op, false);
            }
            CanvasCommand::Clear { color } => {
                canvas.clear(Color(*color));
            }
            CanvasCommand::DrawRect { left, top, right, bottom }
                if is_valid(*left) && is_valid(*top) && is_valid(*right) && is_valid(*bottom) => {
                let rect = Rect::new(*left, *top, *right, *bottom);
                canvas.draw_rect(&rect, &paint);
            }
            CanvasCommand::DrawOval { left, top, right, bottom }
                if is_valid(*left) && is_valid(*top) && is_valid(*right) && is_valid(*bottom) => {
                let rect = Rect::new(*left, *top, *right, *bottom);
                canvas.draw_oval(&rect, &paint);
            }
            CanvasCommand::DrawCircle { cx, cy, radius }
                if is_valid(*cx) && is_valid(*cy) && is_valid(*radius) => {
                canvas.draw_circle(Point::new(*cx, *cy), *radius, &paint);
            }
            CanvasCommand::DrawLine { x0, y0, x1, y1 }
                if is_valid(*x0) && is_valid(*y0) && is_valid(*x1) && is_valid(*y1) => {
                canvas.draw_line(Point::new(*x0, *y0), Point::new(*x1, *y1), &paint);
            }
            CanvasCommand::DrawRoundRect { left, top, right, bottom, rx, ry }
                if is_valid(*left) && is_valid(*top) && is_valid(*right) && is_valid(*bottom)
                   && is_valid(*rx) && is_valid(*ry) => {
                let rect = Rect::new(*left, *top, *right, *bottom);
                canvas.draw_round_rect(&rect, *rx, *ry, &paint);
            }
            CanvasCommand::ResetMatrix => {
                canvas.reset_matrix();
            }
            _ => {} // Skip invalid values
        }
    }

    // Verify canvas state
    let _ = canvas.total_matrix();
    let _ = canvas.clip_bounds();
    let _ = canvas.save_count();

    // Restore all saves
    canvas.restore_to_count(1);

    // Flush
    canvas.flush();
});
