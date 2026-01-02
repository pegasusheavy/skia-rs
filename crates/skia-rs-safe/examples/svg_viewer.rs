//! SVG viewer example for skia-rs
//!
//! This example demonstrates:
//! - Parsing SVG content
//! - Rendering SVG to a raster surface
//! - Saving the result as PNG
//!
//! Usage:
//!   cargo run --example svg_viewer
//!   cargo run --example svg_viewer -- path/to/file.svg

use skia_rs_canvas::Surface;
use skia_rs_codec::{ImageEncoder, ImageInfo, PngEncoder};
use skia_rs_core::{AlphaType, Color, ColorType};
use skia_rs_svg::{parse_svg, render_svg_to_surface};
use std::fs::{self, File};
use std::io::BufWriter;

/// Sample SVG for demonstration
const SAMPLE_SVG: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<svg viewBox="0 0 400 300" xmlns="http://www.w3.org/2000/svg">
  <!-- Background -->
  <rect width="400" height="300" fill="#1a1a2e"/>

  <!-- Sun/Moon -->
  <circle cx="320" cy="60" r="40" fill="#edf2f4"/>

  <!-- Mountains -->
  <polygon points="0,300 100,150 200,300" fill="#16213e"/>
  <polygon points="100,300 200,100 300,300" fill="#0f3460"/>
  <polygon points="200,300 350,120 500,300" fill="#16213e"/>

  <!-- Water reflection -->
  <rect x="0" y="250" width="400" height="50" fill="#0f3460" opacity="0.5"/>

  <!-- Stars -->
  <circle cx="50" cy="40" r="2" fill="#edf2f4"/>
  <circle cx="80" cy="70" r="1.5" fill="#edf2f4"/>
  <circle cx="150" cy="30" r="2" fill="#edf2f4"/>
  <circle cx="200" cy="60" r="1" fill="#edf2f4"/>
  <circle cx="250" cy="25" r="1.5" fill="#edf2f4"/>
  <circle cx="100" cy="100" r="1" fill="#edf2f4"/>

  <!-- Trees -->
  <g transform="translate(30, 200)">
    <rect x="10" y="30" width="6" height="20" fill="#3d2914"/>
    <polygon points="13,0 0,30 26,30" fill="#2d6a4f"/>
  </g>

  <g transform="translate(60, 210)">
    <rect x="8" y="25" width="5" height="15" fill="#3d2914"/>
    <polygon points="10,0 0,25 20,25" fill="#40916c"/>
  </g>

  <!-- Title text (using rect as placeholder since text support is basic) -->
  <rect x="100" y="270" width="200" height="4" fill="#e94560" rx="2"/>

  <!-- Logo-like element -->
  <g transform="translate(350, 250)">
    <circle cx="25" cy="25" r="20" fill="none" stroke="#e94560" stroke-width="3"/>
    <line x1="15" y1="25" x2="35" y2="25" stroke="#e94560" stroke-width="3"/>
    <line x1="25" y1="15" x2="25" y2="35" stroke="#e94560" stroke-width="3"/>
  </g>
</svg>"##;

fn main() {
    println!("skia-rs SVG Viewer Example");
    println!("==========================\n");

    // Check for command line argument
    let args: Vec<String> = std::env::args().collect();
    let svg_content = if args.len() > 1 {
        let path = &args[1];
        println!("Loading SVG from: {}", path);
        match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Failed to read SVG file: {}", e);
                eprintln!("Using sample SVG instead.\n");
                SAMPLE_SVG.to_string()
            }
        }
    } else {
        println!("Using built-in sample SVG (pass a file path as argument to load custom SVG)");
        SAMPLE_SVG.to_string()
    };

    // Parse the SVG
    println!("\nParsing SVG...");
    match parse_svg(&svg_content) {
        Ok(dom) => {
            println!("SVG parsed successfully!");

            let view_box = dom.get_view_box();
            println!(
                "  ViewBox: {} x {}",
                view_box.width(),
                view_box.height()
            );

            // Create output surface
            let width = 800;
            let height = 600;

            let mut surface = Surface::new_raster_n32_premul(width, height)
                .expect("Failed to create surface");

            println!("\nCreated {}x{} surface", width, height);

            // Clear with a background color
            {
                let mut canvas = surface.raster_canvas();
                canvas.clear(Color::from_rgb(26, 26, 46));
            }

            // Render SVG
            println!("Rendering SVG...");
            render_svg_to_surface(&dom, &mut surface);
            println!("SVG rendered!");

            // Save output
            let pixels = surface.pixels();
            let row_bytes = width as usize * 4;

            let output_path = "svg_viewer_output.png";
            let file = File::create(output_path).expect("Failed to create output file");
            let ref mut writer = BufWriter::new(file);

            let img_info = ImageInfo::new(width, height, ColorType::Rgba8888, AlphaType::Premul);

            if let Some(image) =
                skia_rs_codec::Image::from_raster_data(&img_info, pixels, row_bytes)
            {
                let encoder = PngEncoder::new();
                encoder.encode(&image, writer).expect("Failed to encode PNG");
                println!("\nSaved output to: {}", output_path);
            } else {
                eprintln!("Failed to create image from surface pixels");
            }
        }
        Err(e) => {
            eprintln!("Failed to parse SVG: {}", e);
        }
    }

    println!("\nExample complete!");
}
