# Performance Tuning Guide

This guide covers optimization techniques for getting the best performance from skia-rs.

## Quick Tips

1. **Use release builds**: `cargo build --release`
2. **Enable LTO**: Add `lto = true` to `[profile.release]`
3. **Batch operations**: Draw similar items together
4. **Reuse objects**: Don't recreate Paint/Path every frame
5. **Use GPU**: Enable GPU backend for complex scenes
6. **Cache paths**: Store built paths instead of rebuilding

## Surface Management

### Choose the Right Size

```rust
// ❌ Too large - wastes memory and bandwidth
let surface = Surface::new_raster_n32_premul(4000, 4000)?;

// ✅ Match your actual needs
let surface = Surface::new_raster_n32_premul(1920, 1080)?;
```

### Memory Estimates

| Resolution | Memory (RGBA) |
|------------|---------------|
| 256×256 | 256 KB |
| 1024×1024 | 4 MB |
| 1920×1080 | 8 MB |
| 3840×2160 | 32 MB |

### Reuse Surfaces

```rust
// ❌ Creating new surface every frame
fn render() {
    let surface = Surface::new_raster_n32_premul(800, 600)?;
    // draw...
}

// ✅ Reuse surface, just clear
struct Renderer {
    surface: Surface,
}

impl Renderer {
    fn render(&mut self) {
        let mut canvas = self.surface.raster_canvas();
        canvas.clear(Color::WHITE);
        // draw...
    }
}
```

## Path Optimization

### Pre-build Paths

```rust
// ❌ Building path every frame
fn render(canvas: &mut impl Canvas) {
    let mut builder = PathBuilder::new();
    builder.add_circle(100.0, 100.0, 50.0);
    let path = builder.build();
    canvas.draw_path(&path, &paint);
}

// ✅ Build once, reuse
struct Scene {
    circle_path: Path,
}

impl Scene {
    fn new() -> Self {
        let mut builder = PathBuilder::new();
        builder.add_circle(100.0, 100.0, 50.0);
        Self {
            circle_path: builder.build(),
        }
    }

    fn render(&self, canvas: &mut impl Canvas) {
        canvas.draw_path(&self.circle_path, &paint);
    }
}
```

### Path Complexity

```rust
// Path segment count affects performance
// Simple paths (< 100 segments): Fast
// Medium paths (100-1000 segments): Moderate
// Complex paths (> 1000 segments): Consider simplification

// Simplify complex paths
let simplified = path.simplify(); // Reduce segment count
```

### Avoid Unnecessary Path Operations

```rust
// ❌ Expensive for simple shapes
let mut builder = PathBuilder::new();
builder.add_rect(&rect);
let path = builder.build();
canvas.draw_path(&path, &paint);

// ✅ Use direct draw methods
canvas.draw_rect(&rect, &paint);
```

## Paint Optimization

### Reuse Paint Objects

```rust
// ❌ Creating paint every draw call
for rect in &rects {
    let mut paint = Paint::new();
    paint.set_color32(Color::RED);
    canvas.draw_rect(rect, &paint);
}

// ✅ Reuse paint
let mut paint = Paint::new();
paint.set_color32(Color::RED);
for rect in &rects {
    canvas.draw_rect(rect, &paint);
}
```

### Simple vs Complex Paints

```rust
// Fast: Simple solid color
let mut paint = Paint::new();
paint.set_color32(Color::RED);

// Medium: With anti-aliasing
paint.set_anti_alias(true);

// Slower: With shader
paint.set_shader(Some(Box::new(gradient)));

// Slowest: With filter + shader + blend mode
paint.set_shader(Some(shader));
paint.set_color_filter(Some(filter));
paint.set_blend_mode(BlendMode::Multiply);
```

### Stroke vs Fill

```rust
// Fill is generally faster than stroke
paint.set_style(Style::Fill);    // Faster
paint.set_style(Style::Stroke);  // Slower (needs stroke expansion)
```

## Drawing Optimization

### Batch Similar Operations

```rust
// ❌ Alternating paints
for i in 0..100 {
    let paint = if i % 2 == 0 { &red_paint } else { &blue_paint };
    canvas.draw_rect(&rects[i], paint);
}

// ✅ Group by paint
for rect in &red_rects {
    canvas.draw_rect(rect, &red_paint);
}
for rect in &blue_rects {
    canvas.draw_rect(rect, &blue_paint);
}
```

### Use Appropriate Primitives

| Operation | Performance |
|-----------|-------------|
| `draw_point` | Fastest |
| `draw_line` | Fast |
| `draw_rect` | Fast |
| `draw_circle` | Medium |
| `draw_oval` | Medium |
| `draw_path` (simple) | Medium |
| `draw_path` (complex) | Slow |
| `draw_text_blob` | Slow |

### Clip Early

```rust
// ❌ Draw everything, let clipping handle it
for item in &all_items {
    canvas.draw_path(&item.path, &paint);
}

// ✅ Skip items outside viewport
let viewport = Rect::from_wh(800.0, 600.0);
for item in &all_items {
    if item.bounds.intersects(&viewport) {
        canvas.draw_path(&item.path, &paint);
    }
}
```

## Text Optimization

### Pre-build TextBlobs

```rust
// ❌ Shaping text every frame
fn render_text(canvas: &mut impl Canvas, text: &str, font: &Font) {
    canvas.draw_string(text, Point::new(0.0, 0.0), font, &paint);
}

// ✅ Build blob once
struct TextDisplay {
    blob: TextBlob,
}

impl TextDisplay {
    fn new(text: &str, font: &Font) -> Self {
        Self {
            blob: TextBlobBuilder::new()
                .add_text(text, font, Point::ZERO)
                .build(),
        }
    }

    fn render(&self, canvas: &mut impl Canvas, paint: &Paint) {
        canvas.draw_text_blob(&self.blob, Point::ZERO, paint);
    }
}
```

### Glyph Caching

The text system automatically caches rendered glyphs. To optimize:

```rust
// Use consistent fonts to maximize cache hits
let font = Font::new(typeface, 16.0);  // Same size = same cache entry

// Avoid too many font sizes
// ❌ font sizes: 10, 11, 12, 13, 14, ...
// ✅ font sizes: 12, 16, 24, 32
```

## GPU Backend

### When to Use GPU

```rust
// CPU (Raster) is better for:
// - Simple scenes (< 100 draw calls)
// - Small surfaces (< 512×512)
// - Static images
// - CPU-bound workflows

// GPU is better for:
// - Complex scenes (> 1000 draw calls)
// - Large surfaces (> 1080p)
// - Many overlapping transparent layers
// - Real-time animation
// - Video/game rendering
```

### GPU Setup

```rust
#[cfg(feature = "wgpu-backend")]
fn create_gpu_surface(width: u32, height: u32) -> GpuSurface {
    let context = WgpuContext::new()?;
    context.create_surface(width, height)
}
```

### GPU Best Practices

```rust
// Minimize GPU state changes
// Group draws by: shader → blend mode → texture

// Batch vertex data
// Build mesh once, draw many times

// Use texture atlases for small images
let atlas = TextureAtlas::new(2048, 2048);
atlas.add_image(&small_image_1);
atlas.add_image(&small_image_2);
```

## Memory Optimization

### Reduce Allocations

```rust
// ❌ Allocating in hot loop
for _ in 0..1000 {
    let points: Vec<Point> = compute_points();
    // use points
}

// ✅ Reuse allocation
let mut points = Vec::with_capacity(estimated_size);
for _ in 0..1000 {
    points.clear();
    compute_points_into(&mut points);
    // use points
}
```

### Use Stack Allocation

```rust
use smallvec::SmallVec;

// ❌ Always heap allocated
let points: Vec<Point> = vec![p1, p2, p3, p4];

// ✅ Stack for small arrays
let points: SmallVec<[Point; 8]> = smallvec![p1, p2, p3, p4];
```

### Image Memory

```rust
// Lazy loading
let lazy_image = LazyImage::new(|| {
    Image::decode(&load_file("large.png"))
});
// Only decoded when first used

// Release when not needed
drop(large_image);  // Frees pixel buffer immediately
```

## Profiling

### Built-in Profiling

```rust
use skia_rs_bench::memory::{measure_memory, MemoryProfile};

// Measure single operation
let (result, stats) = measure_memory("my_operation", || {
    // operation to measure
});
println!("Allocated: {} bytes", stats.allocated);

// Profile multiple operations
let mut profile = MemoryProfile::new();
profile.measure("create_surface", || Surface::new_raster_n32_premul(1000, 1000));
profile.measure("build_path", || build_complex_path());
println!("{}", profile.report());
```

### External Profilers

```bash
# CPU profiling (Linux)
perf record --call-graph dwarf cargo run --release
perf report

# Memory profiling
valgrind --tool=massif cargo run --release
ms_print massif.out.*

# Flamegraph
cargo install flamegraph
cargo flamegraph --release
```

## Benchmark Targets

Based on optimization work, target these performance levels:

| Operation | Target | Optimized |
|-----------|--------|-----------|
| Point operations | < 5 ns | ✅ |
| Matrix multiply | < 10 ns | ✅ |
| Path bounds | < 100 ns | ✅ |
| Rect fill (100×100) | < 1 µs | ✅ |
| Circle fill (r=50) | < 5 µs | ✅ |
| Complex path (100 seg) | < 50 µs | ✅ |
| Text blob (short) | < 100 µs | ✅ |
| 1080p clear | < 1 ms | ✅ |

## Common Pitfalls

### 1. Ignoring Anti-Aliasing Cost

```rust
// AA adds ~2-4x overhead
paint.set_anti_alias(true);  // Use only when needed
```

### 2. Unnecessary Transform Updates

```rust
// ❌ Recalculating matrix every frame
canvas.save();
canvas.translate(x, y);
canvas.rotate(angle);
canvas.scale(s, s);
// draw
canvas.restore();

// ✅ Pre-compute combined matrix
let transform = Matrix::translate(x, y)
    .pre_rotate(angle)
    .pre_scale(s, s);
canvas.save();
canvas.concat(&transform);
// draw
canvas.restore();
```

### 3. Drawing Invisible Content

```rust
// ❌ Drawing fully transparent
paint.set_alpha(0);
canvas.draw_rect(&rect, &paint);  // Wasted work!

// ✅ Skip if invisible
if paint.alpha() > 0 {
    canvas.draw_rect(&rect, &paint);
}
```

### 4. Overdraw

```rust
// ❌ Drawing multiple opaque layers
canvas.draw_rect(&background, &opaque_paint);
canvas.draw_rect(&background, &another_opaque);  // Completely overwrites!

// ✅ Only draw final layer
canvas.draw_rect(&background, &another_opaque);
```

## Release Build Settings

```toml
# Cargo.toml

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"  # Smaller binary, faster unwinding

# For maximum performance at cost of compile time
[profile.release-max]
inherits = "release"
lto = "fat"
```

## Summary Checklist

- [ ] Using release builds with LTO
- [ ] Pre-building and caching Path objects
- [ ] Reusing Paint objects
- [ ] Batching similar draw operations
- [ ] Using direct draw methods for simple shapes
- [ ] Culling off-screen content early
- [ ] Pre-building TextBlobs for repeated text
- [ ] Using GPU backend for complex scenes
- [ ] Minimizing allocations in hot paths
- [ ] Profiling to identify bottlenecks
