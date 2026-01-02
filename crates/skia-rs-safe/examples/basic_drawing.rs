//! Basic Drawing Example
//!
//! This example demonstrates the fundamental drawing operations in skia-rs:
//! - Creating a surface
//! - Drawing shapes (rectangles, circles, paths)
//! - Using colors and paint styles
//! - Saving to a file

use skia_rs_canvas::Surface;
use skia_rs_codec::{PngEncoder, ImageEncoder};
use skia_rs_core::{Color, Point, Rect};
use skia_rs_paint::{Paint, Style};
use skia_rs_path::PathBuilder;

fn main() {
    println!("skia-rs Basic Drawing Example");
    println!("==============================\n");

    // Create a 400x300 RGBA surface
    let mut surface = Surface::new_raster_n32_premul(400, 300)
        .expect("Failed to create surface");

    println!("Created {}x{} surface", surface.width(), surface.height());

    // Get a canvas to draw on
    {
        let mut canvas = surface.raster_canvas();

        // Clear with a dark background
        canvas.clear(Color::from_rgb(30, 30, 45));
        println!("Cleared to dark blue background");

        // Draw a filled red rectangle
        let mut paint = Paint::new();
        paint.set_color32(Color::from_rgb(220, 80, 80));
        paint.set_style(Style::Fill);
        paint.set_anti_alias(true);

        let rect = Rect::from_xywh(50.0, 50.0, 100.0, 80.0);
        canvas.draw_rect(&rect, &paint);
        println!("Drew red rectangle at ({}, {})", 50, 50);

        // Draw a stroked blue rectangle
        paint.set_color32(Color::from_rgb(80, 150, 220));
        paint.set_style(Style::Stroke);
        paint.set_stroke_width(4.0);

        let rect2 = Rect::from_xywh(180.0, 50.0, 100.0, 80.0);
        canvas.draw_rect(&rect2, &paint);
        println!("Drew blue stroked rectangle at ({}, {})", 180, 50);

        // Draw a green filled circle
        paint.set_color32(Color::from_rgb(80, 200, 120));
        paint.set_style(Style::Fill);

        canvas.draw_circle(Point::new(100.0, 200.0), 50.0, &paint);
        println!("Drew green circle at ({}, {}) with radius {}", 100, 200, 50);

        // Draw a yellow stroked circle
        paint.set_color32(Color::from_rgb(255, 220, 80));
        paint.set_style(Style::Stroke);
        paint.set_stroke_width(3.0);

        canvas.draw_circle(Point::new(230.0, 200.0), 50.0, &paint);
        println!("Drew yellow stroked circle at ({}, {}) with radius {}", 230, 200, 50);

        // Draw a path (triangle)
        let mut builder = PathBuilder::new();
        builder
            .move_to(320.0, 160.0)
            .line_to(380.0, 260.0)
            .line_to(260.0, 260.0)
            .close();

        let path = builder.build();

        paint.set_color32(Color::from_rgb(180, 100, 220));
        paint.set_style(Style::Fill);
        canvas.draw_path(&path, &paint);
        println!("Drew purple triangle path");

        // Draw a stroked path (star outline)
        paint.set_color32(Color::from_rgb(255, 180, 100));
        paint.set_style(Style::Stroke);
        paint.set_stroke_width(2.0);
        canvas.draw_path(&path, &paint);
    }

    // Save to PNG
    let output_path = "basic_drawing_output.png";
    
    // Get pixel data from surface
    let pixels = surface.pixels();
    let width = surface.width();
    let height = surface.height();
    
    // Create an image for encoding
    let img_info = skia_rs_codec::ImageInfo::new(
        width,
        height,
        skia_rs_core::ColorType::Rgba8888,
        skia_rs_core::AlphaType::Premul,
    );
    
    if let Some(image) = skia_rs_codec::Image::from_raster_data(&img_info, pixels, width as usize * 4) {
        let encoder = PngEncoder::new();
        match encoder.encode_bytes(&image) {
            Ok(png_data) => {
                if let Err(e) = std::fs::write(output_path, &png_data) {
                    eprintln!("Failed to write file: {}", e);
                } else {
                    println!("\nSaved output to: {}", output_path);
                }
            }
            Err(e) => eprintln!("Failed to encode PNG: {}", e),
        }
    }

    println!("\nExample complete!");
}
