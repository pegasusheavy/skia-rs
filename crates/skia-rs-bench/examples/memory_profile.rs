//! Memory profiling example for skia-rs.
//!
//! This example measures memory allocation patterns for common operations.
//!
//! Usage:
//!   cargo run --example memory_profile -p skia-rs-bench --release

use skia_rs_bench::memory::{self, MemoryProfile};
use skia_rs_canvas::Surface;
use skia_rs_core::{Color, Point, Rect};
use skia_rs_paint::Paint;
use skia_rs_path::PathBuilder;

fn main() {
    println!("skia-rs Memory Profile");
    println!("======================\n");

    // Print type sizes
    println!("Type Sizes:");
    memory::size_of::print_summary();
    println!();

    // Memory profiling
    let mut profile = MemoryProfile::new();

    // ==========================================================================
    // Surface Allocation
    // ==========================================================================
    println!("Profiling surface allocation...");

    profile.measure("Surface 64x64", || {
        let _ = Surface::new_raster_n32_premul(64, 64);
    });

    profile.measure("Surface 256x256", || {
        let _ = Surface::new_raster_n32_premul(256, 256);
    });

    profile.measure("Surface 1024x1024", || {
        let _ = Surface::new_raster_n32_premul(1024, 1024);
    });

    profile.measure("Surface 1920x1080", || {
        let _ = Surface::new_raster_n32_premul(1920, 1080);
    });

    // ==========================================================================
    // Path Construction
    // ==========================================================================
    println!("Profiling path construction...");

    profile.measure("Path 10 lines", || {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        for i in 0..10 {
            builder.line_to(i as f32 * 10.0, (i % 2) as f32 * 50.0);
        }
        builder.close();
        let _ = builder.build();
    });

    profile.measure("Path 100 lines", || {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        for i in 0..100 {
            builder.line_to(i as f32 * 10.0, (i % 2) as f32 * 50.0);
        }
        builder.close();
        let _ = builder.build();
    });

    profile.measure("Path 1000 lines", || {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        for i in 0..1000 {
            builder.line_to(i as f32 * 10.0, (i % 2) as f32 * 50.0);
        }
        builder.close();
        let _ = builder.build();
    });

    profile.measure("Path 100 cubics", || {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        for i in 0..100 {
            let x = i as f32 * 10.0;
            builder.cubic_to(x, 10.0, x + 5.0, -10.0, x + 10.0, 0.0);
        }
        builder.close();
        let _ = builder.build();
    });

    // ==========================================================================
    // Paint
    // ==========================================================================
    println!("Profiling paint...");

    profile.measure("Paint create", || {
        let _ = Paint::new();
    });

    let paint = Paint::new();
    profile.measure("Paint clone", || {
        let _ = paint.clone();
    });

    // ==========================================================================
    // Drawing Operations
    // ==========================================================================
    println!("Profiling drawing operations...");

    let mut surface = Surface::new_raster_n32_premul(1000, 1000).unwrap();
    let paint = Paint::new();

    // Clear surface first
    {
        let mut canvas = surface.raster_canvas();
        canvas.clear(Color::WHITE);
    }

    profile.measure("Draw 100 rects", || {
        let mut canvas = surface.raster_canvas();
        for i in 0..100 {
            let rect = Rect::from_xywh((i * 9) as f32, (i * 9) as f32, 50.0, 50.0);
            canvas.draw_rect(&rect, &paint);
        }
    });

    profile.measure("Draw 100 circles", || {
        let mut canvas = surface.raster_canvas();
        for i in 0..100 {
            canvas.draw_circle(Point::new((i * 10) as f32, (i * 10) as f32), 20.0, &paint);
        }
    });

    // Create a path for repeated drawing
    let mut builder = PathBuilder::new();
    for i in 0..5 {
        let angle = (i as f32 * 72.0 - 90.0).to_radians();
        let x = 50.0 * angle.cos();
        let y = 50.0 * angle.sin();
        if i == 0 {
            builder.move_to(x + 500.0, y + 500.0);
        } else {
            builder.line_to(x + 500.0, y + 500.0);
        }
    }
    builder.close();
    let star = builder.build();

    profile.measure("Draw path 100x", || {
        let mut canvas = surface.raster_canvas();
        for _ in 0..100 {
            canvas.draw_path(&star, &paint);
        }
    });

    // ==========================================================================
    // Batch Operations
    // ==========================================================================
    println!("Profiling batch operations...");

    profile.measure("Alloc 1000 Points", || {
        let _points: Vec<Point> = (0..1000).map(|i| Point::new(i as f32, i as f32)).collect();
    });

    profile.measure("Alloc 1000 Rects", || {
        let _rects: Vec<Rect> = (0..1000)
            .map(|i| Rect::from_xywh(i as f32, i as f32, 10.0, 10.0))
            .collect();
    });

    profile.measure("Alloc 1000 Colors", || {
        let _colors: Vec<Color> = (0..1000)
            .map(|i| Color::from_argb(255, (i % 256) as u8, ((i / 256) % 256) as u8, 0))
            .collect();
    });

    // ==========================================================================
    // Report
    // ==========================================================================
    println!("\n{}", profile.report());

    // Summary calculations
    println!("\nMemory Efficiency Notes:");
    println!("------------------------");
    println!(
        "- Surface 1920x1080 = {} bytes pixel buffer",
        1920 * 1080 * 4
    );
    println!(
        "- Path with N lines ≈ {} + N * {} bytes",
        memory::size_of::path(),
        16
    ); // Approx
    println!("- Paint base struct = {} bytes", memory::size_of::paint());
    println!();

    // Estimate for common operations
    let surface_4k = 3840 * 2160 * 4;
    println!("Estimated memory for common scenarios:");
    println!("- 4K surface: {} MB", surface_4k as f64 / 1024.0 / 1024.0);
    println!(
        "- 10 layers of 1080p: {} MB",
        10.0 * 1920.0 * 1080.0 * 4.0 / 1024.0 / 1024.0
    );
    println!(
        "- 1000 complex paths (100 segments each): ~{} KB",
        1000 * (memory::size_of::path() + 100 * 16) / 1024
    );
}
