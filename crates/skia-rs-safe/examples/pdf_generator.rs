//! PDF generator example for skia-rs
//!
//! This example demonstrates:
//! - Creating a PDF document
//! - Drawing shapes on a PDF canvas
//! - Adding metadata to the PDF
//! - Saving the PDF to a file

use skia_rs_core::{Color, Point, Rect};
use skia_rs_paint::{Paint, Style};
use skia_rs_path::PathBuilder;
use skia_rs_pdf::{PdfDocument, PdfMetadata};
use std::fs::File;
use std::io::BufWriter;

fn main() {
    println!("skia-rs PDF Generator Example");
    println!("==============================\n");

    // Create a new PDF document
    let mut doc = PdfDocument::new();

    // Set metadata
    doc.metadata_mut().title = Some("skia-rs Example Document".to_string());
    doc.metadata_mut().author = Some("skia-rs Library".to_string());
    doc.metadata_mut().subject = Some("Demonstration of PDF generation".to_string());
    doc.metadata_mut().creator = Some("skia-rs v0.1.0".to_string());
    doc.metadata_mut().keywords = Some("rust, pdf, graphics, skia".to_string());

    println!("Created PDF document with metadata:");
    println!("  Title: skia-rs Example Document");
    println!("  Author: skia-rs Library");

    // Create a US Letter size page (612 x 792 points)
    let page_width = 612.0;
    let page_height = 792.0;

    println!(
        "\nBeginning page 1 ({}x{} points)...",
        page_width, page_height
    );

    let mut canvas = doc.begin_page(page_width, page_height);

    // Draw a header rectangle
    {
        let mut paint = Paint::new();
        paint.set_color32(Color::from_rgb(41, 128, 185));
        paint.set_style(Style::Fill);

        canvas.draw_rect(&Rect::from_xywh(0.0, 0.0, page_width, 80.0), &paint);
        println!("  Drew header rectangle");
    }

    // Draw title text
    {
        let mut paint = Paint::new();
        paint.set_color32(Color::from_rgb(255, 255, 255));

        canvas.draw_text("skia-rs PDF Example", 50.0, 50.0, 24.0, &paint);
        println!("  Drew title text");
    }

    // Draw a subtitle
    {
        let mut paint = Paint::new();
        paint.set_color32(Color::from_rgb(100, 100, 100));

        canvas.draw_text("Pure Rust 2D Graphics Library", 50.0, 120.0, 14.0, &paint);
    }

    // Draw some shapes demonstration
    {
        // Draw section title
        let mut text_paint = Paint::new();
        text_paint.set_color32(Color::from_rgb(30, 30, 50));
        canvas.draw_text("Shape Examples:", 50.0, 170.0, 16.0, &text_paint);

        // Draw a filled rectangle
        let mut fill_paint = Paint::new();
        fill_paint.set_color32(Color::from_rgb(231, 76, 60));
        fill_paint.set_style(Style::Fill);
        canvas.draw_rect(&Rect::from_xywh(50.0, 200.0, 100.0, 80.0), &fill_paint);
        println!("  Drew red filled rectangle");

        // Draw a stroked rectangle
        let mut stroke_paint = Paint::new();
        stroke_paint.set_color32(Color::from_rgb(46, 204, 113));
        stroke_paint.set_style(Style::Stroke);
        stroke_paint.set_stroke_width(3.0);
        canvas.draw_rect(&Rect::from_xywh(180.0, 200.0, 100.0, 80.0), &stroke_paint);
        println!("  Drew green stroked rectangle");

        // Draw a filled circle
        let mut circle_paint = Paint::new();
        circle_paint.set_color32(Color::from_rgb(52, 152, 219));
        circle_paint.set_style(Style::Fill);
        canvas.draw_circle(Point::new(360.0, 240.0), 40.0, &circle_paint);
        println!("  Drew blue filled circle");

        // Draw a stroked circle
        let mut stroked_circle = Paint::new();
        stroked_circle.set_color32(Color::from_rgb(155, 89, 182));
        stroked_circle.set_style(Style::Stroke);
        stroked_circle.set_stroke_width(4.0);
        canvas.draw_circle(Point::new(480.0, 240.0), 40.0, &stroked_circle);
        println!("  Drew purple stroked circle");
    }

    // Draw some lines
    {
        canvas.draw_text("Line Examples:", 50.0, 330.0, 16.0, &Paint::new());

        let mut line_paint = Paint::new();
        line_paint.set_color32(Color::from_rgb(241, 196, 15));
        line_paint.set_style(Style::Stroke);
        line_paint.set_stroke_width(2.0);

        canvas.draw_line(
            Point::new(50.0, 360.0),
            Point::new(200.0, 360.0),
            &line_paint,
        );

        line_paint.set_stroke_width(4.0);
        canvas.draw_line(
            Point::new(50.0, 380.0),
            Point::new(200.0, 380.0),
            &line_paint,
        );

        line_paint.set_stroke_width(6.0);
        canvas.draw_line(
            Point::new(50.0, 400.0),
            Point::new(200.0, 400.0),
            &line_paint,
        );
        println!("  Drew lines with varying widths");
    }

    // Draw a path
    {
        canvas.draw_text("Path Example:", 50.0, 450.0, 16.0, &Paint::new());

        let mut path_builder = PathBuilder::new();
        path_builder
            .move_to(50.0, 500.0)
            .line_to(100.0, 480.0)
            .line_to(150.0, 520.0)
            .line_to(200.0, 480.0)
            .line_to(250.0, 500.0)
            .line_to(200.0, 550.0)
            .line_to(150.0, 530.0)
            .line_to(100.0, 550.0)
            .close();

        let path = path_builder.build();

        let mut path_paint = Paint::new();
        path_paint.set_color32(Color::from_rgb(230, 126, 34));
        path_paint.set_style(Style::Fill);
        canvas.draw_path(&path, &path_paint);
        println!("  Drew an orange path");
    }

    // Draw footer
    {
        let mut footer_paint = Paint::new();
        footer_paint.set_color32(Color::from_rgb(149, 165, 166));
        footer_paint.set_style(Style::Fill);
        canvas.draw_rect(
            &Rect::from_xywh(0.0, page_height - 50.0, page_width, 50.0),
            &footer_paint,
        );

        let mut text_paint = Paint::new();
        text_paint.set_color32(Color::from_rgb(255, 255, 255));
        canvas.draw_text(
            "Generated with skia-rs - Pure Rust 2D Graphics",
            50.0,
            page_height - 20.0,
            12.0,
            &text_paint,
        );
        println!("  Drew footer");
    }

    // End the page
    doc.end_page(canvas);
    println!("\nPage 1 complete!");

    // Add a second page
    println!("\nBeginning page 2...");
    let mut canvas2 = doc.begin_page(page_width, page_height);

    // Draw some content on page 2
    {
        let mut paint = Paint::new();
        paint.set_color32(Color::from_rgb(39, 174, 96));
        paint.set_style(Style::Fill);
        canvas2.draw_rect(&Rect::from_xywh(0.0, 0.0, page_width, 80.0), &paint);

        let mut text_paint = Paint::new();
        text_paint.set_color32(Color::from_rgb(255, 255, 255));
        canvas2.draw_text("Page 2 - Color Palette", 50.0, 50.0, 24.0, &text_paint);
    }

    // Draw color swatches
    {
        let colors = [
            ("Red", Color::from_rgb(231, 76, 60)),
            ("Orange", Color::from_rgb(230, 126, 34)),
            ("Yellow", Color::from_rgb(241, 196, 15)),
            ("Green", Color::from_rgb(46, 204, 113)),
            ("Blue", Color::from_rgb(52, 152, 219)),
            ("Purple", Color::from_rgb(155, 89, 182)),
        ];

        let mut y = 120.0;
        for (name, color) in colors.iter() {
            let mut paint = Paint::new();
            paint.set_color32(*color);
            paint.set_style(Style::Fill);

            canvas2.draw_rect(&Rect::from_xywh(50.0, y, 100.0, 50.0), &paint);

            let mut text_paint = Paint::new();
            text_paint.set_color32(Color::from_rgb(30, 30, 50));
            canvas2.draw_text(name, 170.0, y + 30.0, 14.0, &text_paint);

            y += 70.0;
        }
        println!("  Drew color palette");
    }

    doc.end_page(canvas2);
    println!("Page 2 complete!");

    // Write PDF to file
    let output_path = "pdf_generator_output.pdf";
    let file = File::create(output_path).expect("Failed to create output file");
    let mut writer = BufWriter::new(file);

    doc.write_to(&mut writer).expect("Failed to write PDF");
    println!("\nPDF saved to: {}", output_path);
    println!("Total pages: {}", doc.page_count());

    println!("\nExample complete!");
}
