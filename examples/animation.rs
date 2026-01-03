//! Animation example for skia-rs.
//!
//! Demonstrates various animation techniques including:
//! - Basic motion (translate, rotate, scale)
//! - Easing functions
//! - Color animation
//! - Path morphing
//! - Particle systems
//!
//! Run with: `cargo run --example animation --release`

use skia_rs_canvas::Surface;
use skia_rs_core::{Color, Color4f, Point, Rect};
use skia_rs_paint::{Paint, Style};
use skia_rs_path::PathBuilder;

use std::f32::consts::{PI, TAU};

// =============================================================================
// Easing Functions
// =============================================================================

/// Linear interpolation (no easing).
fn linear(t: f32) -> f32 {
    t
}

/// Ease in (slow start).
fn ease_in_quad(t: f32) -> f32 {
    t * t
}

/// Ease out (slow end).
fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Ease in-out (slow start and end).
fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

/// Ease in cubic.
fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

/// Ease out cubic.
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// Ease in-out cubic.
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Elastic ease out (bouncy).
fn ease_out_elastic(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * TAU / 3.0).sin() + 1.0
    }
}

/// Bounce ease out.
fn ease_out_bounce(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;

    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

// =============================================================================
// Animation Helpers
// =============================================================================

/// Interpolate between two values.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Interpolate between two colors.
fn lerp_color(a: Color4f, b: Color4f, t: f32) -> Color4f {
    Color4f::new(
        lerp(a.r, b.r, t),
        lerp(a.g, b.g, t),
        lerp(a.b, b.b, t),
        lerp(a.a, b.a, t),
    )
}

/// Interpolate between two points.
fn lerp_point(a: Point, b: Point, t: f32) -> Point {
    Point::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t))
}

// =============================================================================
// Demo Scenes
// =============================================================================

/// Draw the easing functions comparison.
fn draw_easing_demo(canvas: &mut skia_rs_canvas::RasterCanvas, frame: usize, total_frames: usize) {
    let t = (frame % total_frames) as f32 / total_frames as f32;

    let easings: &[(&str, fn(f32) -> f32)] = &[
        ("Linear", linear),
        ("Ease In", ease_in_quad),
        ("Ease Out", ease_out_quad),
        ("Ease In-Out", ease_in_out_cubic),
        ("Elastic", ease_out_elastic),
        ("Bounce", ease_out_bounce),
    ];

    let mut paint = Paint::new();
    paint.set_anti_alias(true);

    let row_height = 80.0;
    let start_x = 100.0;
    let end_x = 700.0;
    let ball_radius = 15.0;

    for (i, (name, easing)) in easings.iter().enumerate() {
        let y = 60.0 + i as f32 * row_height;
        let eased_t = easing(t);
        let x = lerp(start_x, end_x, eased_t);

        // Label
        paint.set_color32(Color::from_rgb(100, 100, 100));
        // Note: In real code, use text rendering
        // canvas.draw_string(name, Point::new(10.0, y + 5.0), &font, &paint);

        // Track line
        paint.set_style(Style::Stroke);
        paint.set_stroke_width(2.0);
        paint.set_color32(Color::from_rgb(200, 200, 200));
        canvas.draw_line(
            Point::new(start_x, y),
            Point::new(end_x, y),
            &paint,
        );

        // Ball
        paint.set_style(Style::Fill);
        let hue = (i as f32 / easings.len() as f32) * 360.0;
        paint.set_color32(hsl_to_color(hue, 0.7, 0.5));
        canvas.draw_circle(Point::new(x, y), ball_radius, &paint);

        // Progress indicator
        paint.set_style(Style::Stroke);
        paint.set_stroke_width(1.0);
        paint.set_color32(Color::from_argb(100, 0, 0, 0));
        canvas.draw_line(
            Point::new(start_x, y - 20.0),
            Point::new(start_x + (end_x - start_x) * t, y - 20.0),
            &paint,
        );
    }
}

/// Draw rotating shapes.
fn draw_rotation_demo(canvas: &mut skia_rs_canvas::RasterCanvas, frame: usize, total_frames: usize) {
    let t = frame as f32 / total_frames as f32;
    let angle = t * TAU;

    let center = Point::new(400.0, 300.0);
    let mut paint = Paint::new();
    paint.set_anti_alias(true);

    // Outer rotating squares
    for i in 0..8 {
        let offset_angle = angle + (i as f32 / 8.0) * TAU;
        let distance = 150.0;
        let x = center.x + distance * offset_angle.cos();
        let y = center.y + distance * offset_angle.sin();

        canvas.save();
        canvas.translate(x, y);
        canvas.rotate(angle * 2.0 + i as f32 * PI / 4.0);

        let hue = (i as f32 / 8.0) * 360.0;
        paint.set_color32(hsl_to_color(hue, 0.8, 0.6));

        let rect = Rect::from_xywh(-20.0, -20.0, 40.0, 40.0);
        canvas.draw_rect(&rect, &paint);

        canvas.restore();
    }

    // Center pulsing circle
    let pulse = (angle * 3.0).sin() * 0.3 + 1.0;
    let radius = 50.0 * pulse;
    paint.set_color32(Color::from_argb(200, 255, 100, 100));
    canvas.draw_circle(center, radius, &paint);
}

/// Draw a morphing shape.
fn draw_morph_demo(canvas: &mut skia_rs_canvas::RasterCanvas, frame: usize, total_frames: usize) {
    let t = (frame % total_frames) as f32 / total_frames as f32;
    let eased = ease_in_out_cubic(t);

    let center = Point::new(400.0, 300.0);
    let size = 100.0;

    // Morph between circle, square, and star
    let phase = (frame / total_frames) % 3;
    let morph_t = eased;

    let mut builder = PathBuilder::new();

    match phase {
        0 => {
            // Circle to square
            let corner_radius = lerp(size, 0.0, morph_t);
            builder.add_round_rect(
                &Rect::from_xywh(center.x - size, center.y - size, size * 2.0, size * 2.0),
                corner_radius,
                corner_radius,
            );
        }
        1 => {
            // Square to star
            let points = 4;
            let inner_ratio = lerp(1.0, 0.4, morph_t);

            for i in 0..(points * 2) {
                let is_outer = i % 2 == 0;
                let radius = if is_outer { size } else { size * inner_ratio };
                let angle = (i as f32 / (points * 2) as f32) * TAU - PI / 2.0
                    + (if is_outer { 0.0 } else { PI / points as f32 });
                let x = center.x + radius * angle.cos();
                let y = center.y + radius * angle.sin();

                if i == 0 {
                    builder.move_to(x, y);
                } else {
                    builder.line_to(x, y);
                }
            }
            builder.close();
        }
        _ => {
            // Star back to circle (approximate with many points)
            let points = lerp(8.0, 64.0, morph_t) as usize;
            let inner_ratio = lerp(0.4, 1.0, morph_t);

            for i in 0..(points * 2) {
                let is_outer = i % 2 == 0;
                let radius = if is_outer { size } else { size * inner_ratio };
                let angle = (i as f32 / (points * 2) as f32) * TAU - PI / 2.0;
                let x = center.x + radius * angle.cos();
                let y = center.y + radius * angle.sin();

                if i == 0 {
                    builder.move_to(x, y);
                } else {
                    builder.line_to(x, y);
                }
            }
            builder.close();
        }
    }

    let path = builder.build();

    let mut paint = Paint::new();
    paint.set_anti_alias(true);

    // Gradient-like color based on time
    let hue = (frame as f32 / 10.0) % 360.0;
    paint.set_color32(hsl_to_color(hue, 0.7, 0.5));
    canvas.draw_path(&path, &paint);

    // Outline
    paint.set_style(Style::Stroke);
    paint.set_stroke_width(3.0);
    paint.set_color32(hsl_to_color(hue + 180.0, 0.7, 0.3));
    canvas.draw_path(&path, &paint);
}

/// Simple particle system.
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max_life: f32,
    size: f32,
    hue: f32,
}

impl Particle {
    fn new(x: f32, y: f32, angle: f32, speed: f32, life: f32, size: f32, hue: f32) -> Self {
        Self {
            x,
            y,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed,
            life,
            max_life: life,
            size,
            hue,
        }
    }

    fn update(&mut self, dt: f32) {
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.vy += 100.0 * dt; // gravity
        self.life -= dt;
    }

    fn is_alive(&self) -> bool {
        self.life > 0.0
    }

    fn draw(&self, canvas: &mut skia_rs_canvas::RasterCanvas, paint: &mut Paint) {
        let alpha = (self.life / self.max_life * 255.0) as u8;
        let size = self.size * (self.life / self.max_life);

        paint.set_color32(hsl_to_color_alpha(self.hue, 0.8, 0.6, alpha));
        canvas.draw_circle(Point::new(self.x, self.y), size, paint);
    }
}

fn draw_particles_demo(
    canvas: &mut skia_rs_canvas::RasterCanvas,
    particles: &mut Vec<Particle>,
    frame: usize,
) {
    let dt = 1.0 / 60.0;

    // Spawn new particles
    if frame % 2 == 0 {
        let angle = (frame as f32 * 0.1).sin() * PI / 4.0 - PI / 2.0;
        let spread = 0.3;

        for i in 0..5 {
            let a = angle + (i as f32 - 2.0) * spread * 0.2;
            let speed = 200.0 + (i as f32) * 20.0;
            let hue = (frame as f32 * 2.0 + i as f32 * 30.0) % 360.0;

            particles.push(Particle::new(
                400.0,
                550.0,
                a,
                speed,
                1.5,
                8.0,
                hue,
            ));
        }
    }

    // Update particles
    for particle in particles.iter_mut() {
        particle.update(dt);
    }

    // Remove dead particles
    particles.retain(|p| p.is_alive());

    // Draw particles
    let mut paint = Paint::new();
    paint.set_anti_alias(true);

    for particle in particles.iter() {
        particle.draw(canvas, &mut paint);
    }
}

/// Draw a loading spinner.
fn draw_spinner(canvas: &mut skia_rs_canvas::RasterCanvas, frame: usize, total_frames: usize) {
    let t = frame as f32 / total_frames as f32;
    let center = Point::new(400.0, 300.0);
    let radius = 80.0;
    let dot_count = 12;
    let dot_radius = 10.0;

    let mut paint = Paint::new();
    paint.set_anti_alias(true);

    for i in 0..dot_count {
        let angle = (i as f32 / dot_count as f32) * TAU - PI / 2.0;
        let x = center.x + radius * angle.cos();
        let y = center.y + radius * angle.sin();

        // Fade based on position relative to "current" position
        let current_pos = t * dot_count as f32;
        let distance = ((i as f32 - current_pos + dot_count as f32) % dot_count as f32) / dot_count as f32;
        let alpha = (1.0 - distance).powi(2);
        let size = dot_radius * (0.5 + alpha * 0.5);

        paint.set_color32(Color::from_argb(
            (alpha * 255.0) as u8,
            100,
            150,
            255,
        ));
        canvas.draw_circle(Point::new(x, y), size, &paint);
    }
}

// =============================================================================
// Color Helpers
// =============================================================================

fn hsl_to_color(h: f32, s: f32, l: f32) -> Color {
    hsl_to_color_alpha(h, s, l, 255)
}

fn hsl_to_color_alpha(h: f32, s: f32, l: f32, a: u8) -> Color {
    let h = h % 360.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Color::from_argb(
        a,
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

// =============================================================================
// Main
// =============================================================================

fn main() {
    println!("skia-rs Animation Demo");
    println!("======================\n");

    let width = 800;
    let height = 600;
    let total_frames = 120;

    let mut surface = Surface::new_raster_n32_premul(width, height)
        .expect("Failed to create surface");

    // Particle system state
    let mut particles: Vec<Particle> = Vec::new();

    println!("Generating {} frames of animation...\n", total_frames * 4);

    // Demo 1: Easing functions
    println!("1. Easing Functions Demo");
    for frame in 0..total_frames {
        let mut canvas = surface.raster_canvas();
        canvas.clear(Color::WHITE);
        draw_easing_demo(&mut canvas, frame, total_frames);

        if frame % 30 == 0 {
            print!(".");
        }
    }
    println!(" done");

    // Demo 2: Rotation
    println!("2. Rotation Demo");
    for frame in 0..total_frames {
        let mut canvas = surface.raster_canvas();
        canvas.clear(Color::from_rgb(20, 20, 40));
        draw_rotation_demo(&mut canvas, frame, total_frames);

        if frame % 30 == 0 {
            print!(".");
        }
    }
    println!(" done");

    // Demo 3: Shape morphing
    println!("3. Shape Morph Demo");
    for frame in 0..(total_frames * 3) {
        let mut canvas = surface.raster_canvas();
        canvas.clear(Color::from_rgb(240, 240, 245));
        draw_morph_demo(&mut canvas, frame, total_frames);

        if frame % 30 == 0 {
            print!(".");
        }
    }
    println!(" done");

    // Demo 4: Particles
    println!("4. Particle System Demo");
    particles.clear();
    for frame in 0..total_frames {
        let mut canvas = surface.raster_canvas();
        canvas.clear(Color::from_rgb(10, 10, 20));
        draw_particles_demo(&mut canvas, &mut particles, frame);

        if frame % 30 == 0 {
            print!(".");
        }
    }
    println!(" done");

    // Demo 5: Loading spinner
    println!("5. Loading Spinner Demo");
    for frame in 0..total_frames {
        let mut canvas = surface.raster_canvas();
        canvas.clear(Color::WHITE);
        draw_spinner(&mut canvas, frame, total_frames);

        if frame % 30 == 0 {
            print!(".");
        }
    }
    println!(" done");

    println!("\n✅ Animation demo complete!");
    println!("\nAnimation techniques demonstrated:");
    println!("  • Easing functions (linear, quad, cubic, elastic, bounce)");
    println!("  • Rotation and transformation");
    println!("  • Shape morphing");
    println!("  • Particle systems");
    println!("  • Loading spinners");
    println!("\nIn a real application, you would:");
    println!("  • Save frames to files (PNG sequence)");
    println!("  • Encode to video (FFmpeg)");
    println!("  • Display in a window (winit/SDL2)");
    println!("  • Stream to a canvas (web)");
}
