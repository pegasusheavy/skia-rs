//! Conformance test image generator.
//!
//! Generates reference images for cross-backend conformance testing.
//!
//! Usage:
//!   cargo run --example conformance_gen -- --backend raster --output-dir conformance_output

use skia_rs_canvas::Surface;
use skia_rs_core::{Color, Point, Rect};
use skia_rs_paint::{Paint, Style};
use skia_rs_path::PathBuilder;

use std::env;
use std::fs;
use std::path::Path;

/// Test case definition.
struct TestCase {
    name: &'static str,
    width: i32,
    height: i32,
    draw: fn(&mut Surface),
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut backend = "raster".to_string();
    let mut output_dir = "conformance_output".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--backend" => {
                if i + 1 < args.len() {
                    backend = args[i + 1].clone();
                    i += 1;
                }
            }
            "--output-dir" => {
                if i + 1 < args.len() {
                    output_dir = args[i + 1].clone();
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    println!("Conformance Test Generator");
    println!("Backend: {}", backend);
    println!("Output: {}", output_dir);

    // Create output directory
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Define test cases
    let test_cases = vec![
        TestCase {
            name: "solid_rect",
            width: 200,
            height: 200,
            draw: draw_solid_rect,
        },
        TestCase {
            name: "stroked_rect",
            width: 200,
            height: 200,
            draw: draw_stroked_rect,
        },
        TestCase {
            name: "circle_fill",
            width: 200,
            height: 200,
            draw: draw_circle_fill,
        },
        TestCase {
            name: "circle_stroke",
            width: 200,
            height: 200,
            draw: draw_circle_stroke,
        },
        TestCase {
            name: "oval",
            width: 200,
            height: 200,
            draw: draw_oval,
        },
        TestCase {
            name: "lines",
            width: 200,
            height: 200,
            draw: draw_lines,
        },
        TestCase {
            name: "path_triangle",
            width: 200,
            height: 200,
            draw: draw_path_triangle,
        },
        TestCase {
            name: "path_star",
            width: 200,
            height: 200,
            draw: draw_path_star,
        },
        TestCase {
            name: "path_bezier",
            width: 200,
            height: 200,
            draw: draw_path_bezier,
        },
        TestCase {
            name: "overlapping_shapes",
            width: 200,
            height: 200,
            draw: draw_overlapping_shapes,
        },
        TestCase {
            name: "antialiased_shapes",
            width: 200,
            height: 200,
            draw: draw_antialiased_shapes,
        },
        TestCase {
            name: "alpha_blending",
            width: 200,
            height: 200,
            draw: draw_alpha_blending,
        },
        TestCase {
            name: "stroke_widths",
            width: 200,
            height: 200,
            draw: draw_stroke_widths,
        },
        TestCase {
            name: "nested_paths",
            width: 200,
            height: 200,
            draw: draw_nested_paths,
        },
        TestCase {
            name: "rounded_rect",
            width: 200,
            height: 200,
            draw: draw_rounded_rect,
        },
    ];

    // Generate test images
    for test in &test_cases {
        print!("Generating {}... ", test.name);

        let mut surface = Surface::new_raster_n32_premul(test.width, test.height)
            .expect("Failed to create surface");

        // Clear with white
        {
            let mut canvas = surface.raster_canvas();
            canvas.clear(Color::WHITE);
        }

        // Run the draw function
        (test.draw)(&mut surface);

        // Save the image
        let output_path = Path::new(&output_dir).join(format!(
            "{}_{}.png",
            backend, test.name
        ));

        // Write raw pixels (would need PNG encoder for actual use)
        let pixels = surface.pixels();
        let data_path = output_path.with_extension("raw");
        fs::write(&data_path, pixels).expect("Failed to write raw pixels");

        // Also write metadata
        let meta_path = output_path.with_extension("json");
        let metadata = format!(
            r#"{{"name":"{}","backend":"{}","width":{},"height":{},"format":"rgba8"}}"#,
            test.name, backend, test.width, test.height
        );
        fs::write(&meta_path, metadata).expect("Failed to write metadata");

        println!("done");
    }

    println!("\nGenerated {} test images", test_cases.len());
}

// =============================================================================
// Test Case Drawing Functions
// =============================================================================

fn draw_solid_rect(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();
    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(255, 0, 0));
    canvas.draw_rect(&Rect::from_xywh(50.0, 50.0, 100.0, 100.0), &paint);
}

fn draw_stroked_rect(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();
    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(0, 0, 255));
    paint.set_style(Style::Stroke);
    paint.set_stroke_width(4.0);
    canvas.draw_rect(&Rect::from_xywh(50.0, 50.0, 100.0, 100.0), &paint);
}

fn draw_circle_fill(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();
    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(0, 255, 0));
    canvas.draw_circle(Point::new(100.0, 100.0), 50.0, &paint);
}

fn draw_circle_stroke(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();
    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(255, 0, 255));
    paint.set_style(Style::Stroke);
    paint.set_stroke_width(3.0);
    canvas.draw_circle(Point::new(100.0, 100.0), 50.0, &paint);
}

fn draw_oval(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();
    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(255, 128, 0));
    canvas.draw_oval(&Rect::from_xywh(30.0, 60.0, 140.0, 80.0), &paint);
}

fn draw_lines(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();
    let mut paint = Paint::new();
    paint.set_style(Style::Stroke);
    paint.set_stroke_width(2.0);

    for i in 0..10 {
        let color = Color::from_rgb((i * 25) as u8, 0, (255 - i * 25) as u8);
        paint.set_color32(color);
        let y = 20.0 + i as f32 * 18.0;
        canvas.draw_line(Point::new(20.0, y), Point::new(180.0, y + 40.0), &paint);
    }
}

fn draw_path_triangle(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();

    let mut builder = PathBuilder::new();
    builder.move_to(100.0, 30.0);
    builder.line_to(170.0, 170.0);
    builder.line_to(30.0, 170.0);
    builder.close();

    let path = builder.build();
    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(100, 150, 200));
    canvas.draw_path(&path, &paint);
}

fn draw_path_star(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();

    let mut builder = PathBuilder::new();
    let cx = 100.0;
    let cy = 100.0;
    let outer_r = 80.0;
    let inner_r = 35.0;

    for i in 0..5 {
        let angle_outer = (i as f32 * 72.0 - 90.0).to_radians();
        let angle_inner = (i as f32 * 72.0 + 36.0 - 90.0).to_radians();

        let ox = cx + outer_r * angle_outer.cos();
        let oy = cy + outer_r * angle_outer.sin();
        let ix = cx + inner_r * angle_inner.cos();
        let iy = cy + inner_r * angle_inner.sin();

        if i == 0 {
            builder.move_to(ox, oy);
        } else {
            builder.line_to(ox, oy);
        }
        builder.line_to(ix, iy);
    }
    builder.close();

    let path = builder.build();
    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(255, 215, 0)); // Gold
    canvas.draw_path(&path, &paint);
}

fn draw_path_bezier(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();

    let mut builder = PathBuilder::new();
    builder.move_to(20.0, 100.0);
    builder.cubic_to(80.0, 20.0, 120.0, 180.0, 180.0, 100.0);

    let path = builder.build();
    let mut paint = Paint::new();
    paint.set_style(Style::Stroke);
    paint.set_stroke_width(4.0);
    paint.set_color32(Color::from_rgb(0, 100, 200));
    canvas.draw_path(&path, &paint);
}

fn draw_overlapping_shapes(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();

    let mut paint = Paint::new();

    // Red rectangle
    paint.set_color32(Color::from_rgb(255, 0, 0));
    canvas.draw_rect(&Rect::from_xywh(30.0, 30.0, 100.0, 100.0), &paint);

    // Green circle (overlapping)
    paint.set_color32(Color::from_rgb(0, 255, 0));
    canvas.draw_circle(Point::new(130.0, 130.0), 60.0, &paint);

    // Blue rectangle (overlapping both)
    paint.set_color32(Color::from_rgb(0, 0, 255));
    canvas.draw_rect(&Rect::from_xywh(80.0, 80.0, 100.0, 100.0), &paint);
}

fn draw_antialiased_shapes(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();
    let mut paint = Paint::new();
    paint.set_anti_alias(true);
    paint.set_color32(Color::from_rgb(50, 100, 150));

    // Diagonal line (shows AA effect)
    paint.set_style(Style::Stroke);
    paint.set_stroke_width(1.0);
    canvas.draw_line(Point::new(10.0, 10.0), Point::new(190.0, 190.0), &paint);

    // Small circles
    paint.set_style(Style::Fill);
    for i in 0..5 {
        let x = 30.0 + i as f32 * 35.0;
        canvas.draw_circle(Point::new(x, 50.0), 10.0, &paint);
    }
}

fn draw_alpha_blending(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();
    let mut paint = Paint::new();

    // Background rectangle
    paint.set_color32(Color::from_rgb(200, 200, 200));
    canvas.draw_rect(&Rect::from_xywh(20.0, 20.0, 160.0, 160.0), &paint);

    // Semi-transparent red
    paint.set_color32(Color::from_argb(128, 255, 0, 0));
    canvas.draw_circle(Point::new(80.0, 100.0), 50.0, &paint);

    // Semi-transparent blue
    paint.set_color32(Color::from_argb(128, 0, 0, 255));
    canvas.draw_circle(Point::new(120.0, 100.0), 50.0, &paint);
}

fn draw_stroke_widths(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();
    let mut paint = Paint::new();
    paint.set_style(Style::Stroke);
    paint.set_color32(Color::from_rgb(0, 0, 0));

    let widths = [1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 10.0];

    for (i, &width) in widths.iter().enumerate() {
        let y = 20.0 + i as f32 * 25.0;
        paint.set_stroke_width(width);
        canvas.draw_line(Point::new(20.0, y), Point::new(180.0, y), &paint);
    }
}

fn draw_nested_paths(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();

    // Outer rectangle with hole (using even-odd fill)
    let mut builder = PathBuilder::new();
    builder.add_rect(&Rect::from_xywh(20.0, 20.0, 160.0, 160.0));
    builder.add_rect(&Rect::from_xywh(60.0, 60.0, 80.0, 80.0));

    let path = builder.build();
    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(100, 200, 100));
    canvas.draw_path(&path, &paint);
}

fn draw_rounded_rect(surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();

    let mut builder = PathBuilder::new();
    builder.add_round_rect(&Rect::from_xywh(30.0, 30.0, 140.0, 140.0), 20.0, 20.0);

    let path = builder.build();

    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(150, 100, 200));
    canvas.draw_path(&path, &paint);

    // Stroke outline
    paint.set_style(Style::Stroke);
    paint.set_stroke_width(3.0);
    paint.set_color32(Color::from_rgb(50, 0, 100));
    canvas.draw_path(&path, &paint);
}
