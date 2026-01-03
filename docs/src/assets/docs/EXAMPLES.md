# Examples Gallery

A collection of skia-rs examples demonstrating various features and use cases.

## Getting Started

### Hello World

The simplest possible example - create a surface and fill it with color.

```rust
use skia_rs_canvas::Surface;
use skia_rs_core::Color;

fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 300)
        .expect("Failed to create surface");

    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::from_rgb(100, 149, 237)); // Cornflower blue

    println!("Created a {}x{} surface", surface.width(), surface.height());
}
```

### Drawing Shapes

Basic shape drawing with different styles.

```rust
use skia_rs_canvas::Surface;
use skia_rs_core::{Color, Point, Rect};
use skia_rs_paint::{Paint, Style};

fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 400).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    // Filled rectangle
    let mut fill_paint = Paint::new();
    fill_paint.set_color32(Color::from_rgb(255, 100, 100));
    canvas.draw_rect(&Rect::from_xywh(50.0, 50.0, 100.0, 80.0), &fill_paint);

    // Stroked rectangle
    let mut stroke_paint = Paint::new();
    stroke_paint.set_color32(Color::from_rgb(100, 100, 255));
    stroke_paint.set_style(Style::Stroke);
    stroke_paint.set_stroke_width(4.0);
    canvas.draw_rect(&Rect::from_xywh(200.0, 50.0, 100.0, 80.0), &stroke_paint);

    // Filled circle
    fill_paint.set_color32(Color::from_rgb(100, 200, 100));
    canvas.draw_circle(Point::new(100.0, 250.0), 60.0, &fill_paint);

    // Stroked circle
    stroke_paint.set_color32(Color::from_rgb(200, 100, 200));
    canvas.draw_circle(Point::new(280.0, 250.0), 60.0, &stroke_paint);
}
```

## Paths

### Custom Shapes with PathBuilder

```rust
use skia_rs_canvas::Surface;
use skia_rs_core::{Color, Rect};
use skia_rs_paint::Paint;
use skia_rs_path::PathBuilder;

fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 400).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    // Triangle
    let mut builder = PathBuilder::new();
    builder.move_to(200.0, 50.0);
    builder.line_to(350.0, 300.0);
    builder.line_to(50.0, 300.0);
    builder.close();

    let triangle = builder.build();

    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(255, 200, 0));
    canvas.draw_path(&triangle, &paint);
}
```

### Star Shape

```rust
fn create_star(cx: f32, cy: f32, outer_r: f32, inner_r: f32, points: usize) -> Path {
    let mut builder = PathBuilder::new();

    for i in 0..(points * 2) {
        let radius = if i % 2 == 0 { outer_r } else { inner_r };
        let angle = (i as f32) * std::f32::consts::PI / points as f32
                  - std::f32::consts::FRAC_PI_2;
        let x = cx + radius * angle.cos();
        let y = cy + radius * angle.sin();

        if i == 0 {
            builder.move_to(x, y);
        } else {
            builder.line_to(x, y);
        }
    }
    builder.close();
    builder.build()
}

fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 400).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::from_rgb(30, 30, 50));

    let star = create_star(200.0, 200.0, 150.0, 60.0, 5);

    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(255, 215, 0)); // Gold
    canvas.draw_path(&star, &paint);
}
```

### Bezier Curves

```rust
fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 300).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    // Quadratic bezier
    let mut builder = PathBuilder::new();
    builder.move_to(50.0, 250.0);
    builder.quad_to(200.0, 50.0, 350.0, 250.0);

    let mut paint = Paint::new();
    paint.set_style(Style::Stroke);
    paint.set_stroke_width(3.0);
    paint.set_color32(Color::from_rgb(0, 100, 200));
    canvas.draw_path(&builder.build(), &paint);

    // Cubic bezier
    let mut builder = PathBuilder::new();
    builder.move_to(50.0, 150.0);
    builder.cubic_to(100.0, 50.0, 300.0, 250.0, 350.0, 150.0);

    paint.set_color32(Color::from_rgb(200, 50, 50));
    canvas.draw_path(&builder.build(), &paint);
}
```

## Transformations

### Translate, Rotate, Scale

```rust
fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 400).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    let rect = Rect::from_xywh(-25.0, -25.0, 50.0, 50.0);
    let mut paint = Paint::new();

    // Original (translated to center)
    canvas.save();
    canvas.translate(100.0, 100.0);
    paint.set_color32(Color::from_rgb(255, 0, 0));
    canvas.draw_rect(&rect, &paint);
    canvas.restore();

    // Rotated
    canvas.save();
    canvas.translate(250.0, 100.0);
    canvas.rotate(45.0_f32.to_radians());
    paint.set_color32(Color::from_rgb(0, 255, 0));
    canvas.draw_rect(&rect, &paint);
    canvas.restore();

    // Scaled
    canvas.save();
    canvas.translate(100.0, 250.0);
    canvas.scale(2.0, 1.5);
    paint.set_color32(Color::from_rgb(0, 0, 255));
    canvas.draw_rect(&rect, &paint);
    canvas.restore();

    // Combined
    canvas.save();
    canvas.translate(250.0, 250.0);
    canvas.rotate(30.0_f32.to_radians());
    canvas.scale(1.5, 1.5);
    paint.set_color32(Color::from_rgb(255, 0, 255));
    canvas.draw_rect(&rect, &paint);
    canvas.restore();
}
```

### Matrix Transformations

```rust
use skia_rs_core::Matrix;

fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 400).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    // Create a skew matrix
    let skew = Matrix::skew(0.5, 0.0);

    canvas.save();
    canvas.translate(200.0, 200.0);
    canvas.concat(&skew);

    let rect = Rect::from_xywh(-50.0, -50.0, 100.0, 100.0);
    let mut paint = Paint::new();
    paint.set_color32(Color::from_rgb(100, 150, 200));
    canvas.draw_rect(&rect, &paint);

    canvas.restore();
}
```

## Gradients

### Linear Gradient

```rust
use skia_rs_paint::shader::{LinearGradient, TileMode};
use skia_rs_core::Color4f;

fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 200).unwrap();
    let mut canvas = surface.raster_canvas();

    let gradient = LinearGradient::new(
        Point::new(0.0, 0.0),
        Point::new(400.0, 0.0),
        &[
            Color4f::new(1.0, 0.0, 0.0, 1.0),  // Red
            Color4f::new(1.0, 1.0, 0.0, 1.0),  // Yellow
            Color4f::new(0.0, 1.0, 0.0, 1.0),  // Green
            Color4f::new(0.0, 0.0, 1.0, 1.0),  // Blue
        ],
        None,
        TileMode::Clamp,
    );

    let mut paint = Paint::new();
    paint.set_shader(Some(Box::new(gradient)));
    canvas.draw_rect(&Rect::from_wh(400.0, 200.0), &paint);
}
```

### Radial Gradient

```rust
use skia_rs_paint::shader::RadialGradient;

fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 400).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::BLACK);

    let gradient = RadialGradient::new(
        Point::new(200.0, 200.0),
        150.0,
        &[
            Color4f::new(1.0, 1.0, 1.0, 1.0),  // White center
            Color4f::new(0.0, 0.5, 1.0, 1.0),  // Blue
            Color4f::new(0.0, 0.0, 0.0, 0.0),  // Transparent
        ],
        Some(&[0.0, 0.5, 1.0]),
        TileMode::Clamp,
    );

    let mut paint = Paint::new();
    paint.set_shader(Some(Box::new(gradient)));
    canvas.draw_circle(Point::new(200.0, 200.0), 150.0, &paint);
}
```

## Clipping

### Rectangular Clip

```rust
fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 400).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    // Draw stripes (will be clipped)
    let mut paint = Paint::new();

    canvas.save();
    canvas.clip_rect(&Rect::from_xywh(100.0, 100.0, 200.0, 200.0), ClipOp::Intersect);

    for i in 0..20 {
        let color = if i % 2 == 0 { Color::RED } else { Color::BLUE };
        paint.set_color32(color);
        let x = i as f32 * 30.0;
        canvas.draw_rect(&Rect::from_xywh(x, 0.0, 30.0, 400.0), &paint);
    }

    canvas.restore();
}
```

### Path Clip

```rust
fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 400).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    // Create star clip path
    let star = create_star(200.0, 200.0, 180.0, 80.0, 5);

    canvas.save();
    canvas.clip_path(&star, ClipOp::Intersect);

    // Draw image or pattern inside clip
    let mut paint = Paint::new();
    for y in (0..400).step_by(20) {
        for x in (0..400).step_by(20) {
            let hue = ((x + y) % 360) as f32;
            paint.set_color32(hsl_to_rgb(hue, 0.7, 0.5));
            canvas.draw_rect(&Rect::from_xywh(x as f32, y as f32, 20.0, 20.0), &paint);
        }
    }

    canvas.restore();
}
```

## Text

### Basic Text

```rust
use skia_rs_text::{Font, Typeface, FontStyle};

fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 200).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    let typeface = Typeface::from_name("Arial", FontStyle::normal())
        .unwrap_or_else(Typeface::default);
    let font = Font::new(typeface, 32.0);

    let mut paint = Paint::new();
    paint.set_color32(Color::BLACK);
    paint.set_anti_alias(true);

    canvas.draw_string("Hello, skia-rs!", Point::new(50.0, 100.0), &font, &paint);
}
```

### Styled Text

```rust
fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 300).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    let mut paint = Paint::new();
    paint.set_anti_alias(true);

    // Regular
    let regular = Font::new(
        Typeface::from_name("Arial", FontStyle::normal()).unwrap(),
        24.0
    );
    paint.set_color32(Color::BLACK);
    canvas.draw_string("Regular Text", Point::new(50.0, 50.0), &regular, &paint);

    // Bold
    let bold = Font::new(
        Typeface::from_name("Arial", FontStyle::bold()).unwrap(),
        24.0
    );
    canvas.draw_string("Bold Text", Point::new(50.0, 100.0), &bold, &paint);

    // Italic
    let italic = Font::new(
        Typeface::from_name("Arial", FontStyle::italic()).unwrap(),
        24.0
    );
    canvas.draw_string("Italic Text", Point::new(50.0, 150.0), &italic, &paint);

    // Colored
    paint.set_color32(Color::from_rgb(200, 50, 50));
    canvas.draw_string("Colored Text", Point::new(50.0, 200.0), &regular, &paint);
}
```

## Alpha and Blending

### Transparency

```rust
fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 300).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    let mut paint = Paint::new();

    // Opaque red
    paint.set_color32(Color::from_argb(255, 255, 0, 0));
    canvas.draw_circle(Point::new(150.0, 150.0), 100.0, &paint);

    // Semi-transparent blue
    paint.set_color32(Color::from_argb(128, 0, 0, 255));
    canvas.draw_circle(Point::new(250.0, 150.0), 100.0, &paint);
}
```

### Blend Modes

```rust
use skia_rs_paint::BlendMode;

fn main() {
    let mut surface = Surface::new_raster_n32_premul(600, 400).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    let blend_modes = [
        (BlendMode::SrcOver, "SrcOver"),
        (BlendMode::Multiply, "Multiply"),
        (BlendMode::Screen, "Screen"),
        (BlendMode::Overlay, "Overlay"),
        (BlendMode::Darken, "Darken"),
        (BlendMode::Lighten, "Lighten"),
    ];

    let mut paint = Paint::new();

    for (i, (mode, name)) in blend_modes.iter().enumerate() {
        let x = (i % 3) as f32 * 200.0;
        let y = (i / 3) as f32 * 200.0;

        // Draw red square
        paint.set_color32(Color::from_rgb(255, 100, 100));
        paint.set_blend_mode(BlendMode::SrcOver);
        canvas.draw_rect(&Rect::from_xywh(x + 20.0, y + 20.0, 80.0, 80.0), &paint);

        // Draw blue circle with blend mode
        paint.set_color32(Color::from_rgb(100, 100, 255));
        paint.set_blend_mode(*mode);
        canvas.draw_circle(Point::new(x + 100.0, y + 100.0), 50.0, &paint);
    }
}
```

## Animation

### Simple Animation Loop

```rust
fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 400).unwrap();
    let mut paint = Paint::new();
    paint.set_color32(Color::RED);
    paint.set_anti_alias(true);

    for frame in 0..60 {
        let t = frame as f32 / 60.0;
        let angle = t * std::f32::consts::TAU;

        // Clear
        let mut canvas = surface.raster_canvas();
        canvas.clear(Color::WHITE);

        // Animate position
        let x = 200.0 + 100.0 * angle.cos();
        let y = 200.0 + 100.0 * angle.sin();

        canvas.draw_circle(Point::new(x, y), 30.0, &paint);

        // Save frame (in real app, display or encode)
        save_frame(&surface, frame);
    }
}
```

### Easing Functions

```rust
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn animate_with_easing() {
    for frame in 0..120 {
        let t = frame as f32 / 120.0;
        let eased = ease_in_out_cubic(t);

        let x = 50.0 + eased * 300.0;  // Smooth movement
        // Draw at position x
    }
}
```

## Image Operations

### Loading and Drawing Images

```rust
use skia_rs_codec::Image;

fn main() {
    let mut surface = Surface::new_raster_n32_premul(800, 600).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    // Load image
    let image_data = std::fs::read("photo.png").unwrap();
    let image = Image::decode(&image_data).unwrap();

    // Draw at position
    let paint = Paint::new();
    canvas.draw_image(&image, Point::new(100.0, 100.0), &paint);

    // Draw scaled
    canvas.save();
    canvas.translate(400.0, 100.0);
    canvas.scale(0.5, 0.5);
    canvas.draw_image(&image, Point::ZERO, &paint);
    canvas.restore();
}
```

### Image as Shader

```rust
use skia_rs_paint::shader::{ImageShader, TileMode};

fn main() {
    let mut surface = Surface::new_raster_n32_premul(400, 400).unwrap();

    // Load tile image
    let tile_data = std::fs::read("tile.png").unwrap();
    let tile = Image::decode(&tile_data).unwrap();

    // Create tiled shader
    let shader = ImageShader::new(
        tile,
        TileMode::Repeat,
        TileMode::Repeat,
        None,
    );

    let mut paint = Paint::new();
    paint.set_shader(Some(Box::new(shader)));

    let mut canvas = surface.raster_canvas();
    canvas.draw_rect(&Rect::from_wh(400.0, 400.0), &paint);
}
```

## SVG

### Rendering SVG

```rust
use skia_rs_svg::SvgDom;

fn main() {
    let mut surface = Surface::new_raster_n32_premul(800, 600).unwrap();
    let mut canvas = surface.raster_canvas();
    canvas.clear(Color::WHITE);

    // Load SVG
    let svg_data = std::fs::read_to_string("icon.svg").unwrap();
    let dom = SvgDom::parse(&svg_data).unwrap();

    // Render
    dom.render(&mut canvas);

    // Render scaled
    canvas.save();
    canvas.translate(400.0, 0.0);
    canvas.scale(2.0, 2.0);
    dom.render(&mut canvas);
    canvas.restore();
}
```

## PDF Generation

### Creating PDF

```rust
use skia_rs_pdf::{PdfDocument, PdfCanvas};

fn main() {
    let mut doc = PdfDocument::new();
    doc.set_title("My Document");
    doc.set_author("skia-rs");

    // Add a page
    let mut page = doc.add_page(612.0, 792.0); // Letter size
    let mut canvas = page.canvas();

    // Draw content
    let mut paint = Paint::new();
    paint.set_color32(Color::BLACK);

    let font = Font::new(Typeface::default(), 24.0);
    canvas.draw_string("Hello, PDF!", Point::new(72.0, 72.0), &font, &paint);

    paint.set_color32(Color::from_rgb(200, 50, 50));
    canvas.draw_rect(&Rect::from_xywh(72.0, 100.0, 200.0, 100.0), &paint);

    // Save
    let mut file = std::fs::File::create("output.pdf").unwrap();
    doc.write(&mut file).unwrap();
}
```

## Running Examples

All examples are in the `examples/` directory:

```bash
# Run a specific example
cargo run --example basic_drawing --release

# Run with features
cargo run --example gpu_rendering --release --features wgpu-backend

# List all examples
cargo run --example
```

## More Resources

- [API Documentation](https://docs.rs/skia-rs)
- [Migration Guide](./MIGRATION.md) - Coming from Skia C++
- [Architecture](./ARCHITECTURE.md) - Internal design
- [Performance Guide](./PERFORMANCE.md) - Optimization tips
