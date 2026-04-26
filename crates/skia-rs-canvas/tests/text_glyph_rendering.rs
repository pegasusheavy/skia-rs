//! Verifies `Canvas::draw_string` renders actual glyph outlines instead of
//! placeholder rectangles.
//!
//! Uses a 400-byte fixture font copied from the `ttf-parser` crate's own
//! test suite. The font contains a single glyph for 'A' (glyph id 1) with
//! a triangular body — exactly what we need to show that "glyph rendering
//! is not a solid rect".

#![cfg(feature = "text")]

use skia_rs_canvas::{Canvas, raster::PixelBuffer};
use skia_rs_core::Color;
use skia_rs_paint::Paint;
use skia_rs_text::{Font, Typeface};
use std::sync::Arc;

const DEMO_TTF: &[u8] = include_bytes!("fixtures/demo.ttf");

fn demo_font(size: f32) -> Font {
    let tf = Typeface::from_data(DEMO_TTF.to_vec()).expect("parse demo.ttf");
    Font::new(Arc::new(tf), size)
}

#[test]
fn draw_string_renders_non_rectangular_glyph() {
    // Big canvas and big font — a 200pt 'A' gives us plenty of pixels to
    // sample inside vs. outside the triangular body.
    let mut buffer = PixelBuffer::new(400, 400);
    {
        let mut canvas = Canvas::new_raster(&mut buffer);
        let font = demo_font(200.0);
        let mut paint = Paint::new();
        paint.set_color(Color::WHITE.into());
        canvas.draw_string("A", 100.0, 300.0, &font, &paint);
    }

    // Gather stats inside the glyph's approximate bounding box. For demo.ttf
    // at 200pt the 'A' occupies roughly x in [100, 210] and y in [170, 300].
    let (mut painted, mut empty) = (0usize, 0usize);
    for y in 170..300 {
        for x in 100..210 {
            let c = buffer.get_pixel(x, y).unwrap_or(Color::BLACK);
            if c.red() > 100 {
                painted += 1;
            } else {
                empty += 1;
            }
        }
    }

    assert!(
        painted > 0,
        "glyph 'A' should produce some painted pixels"
    );
    assert!(
        empty > 0,
        "glyph 'A' must have unfilled pixels within its bbox — a solid \
         rectangle would have zero. (painted={painted}, empty={empty})"
    );
    // Sanity: the 'A' is an outline glyph, neither mostly filled nor empty.
    let total = painted + empty;
    let ratio = painted as f32 / total as f32;
    assert!(
        (0.05..0.95).contains(&ratio),
        "expected a non-trivial mix of painted/empty; got ratio={ratio}"
    );
}

#[test]
fn draw_string_no_op_for_dataless_typeface() {
    // Without font data there is no outline to render. Previously the
    // canvas drew a filled rectangle per glyph; it must now draw nothing.
    let mut buffer = PixelBuffer::new(100, 100);
    {
        let mut canvas = Canvas::new_raster(&mut buffer);
        let font = Font::from_size(24.0); // default, dataless typeface
        let mut paint = Paint::new();
        paint.set_color(Color::WHITE.into());
        canvas.draw_string("Hello", 10.0, 50.0, &font, &paint);
    }

    for y in 0..100 {
        for x in 0..100 {
            let c = buffer.get_pixel(x, y).unwrap_or(Color::BLACK);
            assert_eq!(
                c.red(),
                0,
                "pixel ({x},{y}) should be untouched when no font data is \
                 available, but got red={}",
                c.red()
            );
        }
    }
}

#[test]
fn draw_text_blob_renders_non_rectangular_glyph() {
    use skia_rs_core::Point;
    use skia_rs_text::{TextBlob, TextBlobBuilder};

    let font = demo_font(200.0);
    let blob: TextBlob = {
        let mut b = TextBlobBuilder::new();
        b.add_positioned_run(&font, &[1], &[Point::new(100.0, 300.0)]);
        b.build().expect("blob build")
    };

    let mut buffer = PixelBuffer::new(400, 400);
    {
        let mut canvas = Canvas::new_raster(&mut buffer);
        let mut paint = Paint::new();
        paint.set_color(Color::WHITE.into());
        canvas.draw_text_blob(&blob, 0.0, 0.0, &paint);
    }

    let (mut painted, mut empty) = (0usize, 0usize);
    for y in 170..300 {
        for x in 100..210 {
            let c = buffer.get_pixel(x, y).unwrap_or(Color::BLACK);
            if c.red() > 100 {
                painted += 1;
            } else {
                empty += 1;
            }
        }
    }
    assert!(painted > 0, "text_blob should paint some pixels");
    assert!(
        empty > 0,
        "text_blob must have unfilled pixels within its bbox"
    );
}
