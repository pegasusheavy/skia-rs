# skia-rs Architecture

This document describes the internal architecture of skia-rs, a 100% Rust implementation of Google's Skia 2D graphics library.

## High-Level Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                          skia-rs-safe                                │
│                    (High-level ergonomic API)                        │
└─────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌───────────────┐          ┌───────────────┐          ┌───────────────┐
│  skia-rs-svg  │          │  skia-rs-pdf  │          │skia-rs-skottie│
│ (SVG support) │          │(PDF generation)│          │(Lottie anim)  │
└───────────────┘          └───────────────┘          └───────────────┘
        │                           │                           │
        └───────────────────────────┼───────────────────────────┘
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         skia-rs-canvas                               │
│              (Canvas, Surface, Rasterizer, Clipping)                 │
└─────────────────────────────────────────────────────────────────────┘
        │                           │                           │
        ▼                           ▼                           ▼
┌───────────────┐          ┌───────────────┐          ┌───────────────┐
│ skia-rs-text  │          │ skia-rs-paint │          │ skia-rs-codec │
│(Text & fonts) │          │(Paint/Shaders)│          │(Image codecs) │
└───────────────┘          └───────────────┘          └───────────────┘
        │                           │                           │
        └───────────────────────────┼───────────────────────────┘
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          skia-rs-path                                │
│              (Path, PathBuilder, PathOps, PathEffects)               │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          skia-rs-core                                │
│         (Scalar, Point, Rect, Matrix, Color, ImageInfo)              │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌───────────────┐          ┌───────────────┐
│ skia-rs-gpu   │          │ skia-rs-ffi   │
│(GPU backends) │          │ (C bindings)  │
└───────────────┘          └───────────────┘
```

## Crate Responsibilities

### skia-rs-core

The foundation crate providing primitive types used throughout the library.

**Key Types:**
- `Scalar` - Floating-point type alias (f32)
- `Point`, `IPoint` - 2D coordinate types
- `Rect`, `IRect` - Rectangle types
- `Size`, `ISize` - Dimension types
- `Matrix` - 3x3 affine transformation matrix
- `Matrix44` - 4x4 transformation matrix
- `Color`, `Color4f` - Color representations
- `ColorSpace` - Color space management
- `ImageInfo` - Pixel format description
- `Region` - Complex clip regions

**Design Principles:**
- All types are `Copy` where possible
- `#[repr(C)]` for FFI compatibility
- Zero-cost abstractions
- SIMD-friendly memory layouts

### skia-rs-path

Path geometry and manipulation.

**Key Types:**
- `Path` - Immutable path data
- `PathBuilder` - Fluent path construction
- `PathElement` - Move, Line, Quad, Conic, Cubic, Close
- `FillType` - Winding, EvenOdd, InverseWinding, InverseEvenOdd
- `PathMeasure` - Path length and position queries
- `PathOps` - Boolean path operations

**Path Storage:**
```rust
struct Path {
    elements: Vec<PathElement>,
    bounds: Rect,           // Cached bounds
    fill_type: FillType,
    is_convex: Option<bool>, // Lazily computed
}
```

**PathBuilder Pattern:**
```rust
let path = PathBuilder::new()
    .move_to(0.0, 0.0)
    .line_to(100.0, 0.0)
    .quad_to(150.0, 50.0, 100.0, 100.0)
    .close()
    .build();
```

### skia-rs-paint

Drawing style and effects.

**Key Types:**
- `Paint` - Complete drawing style
- `Style` - Fill, Stroke, StrokeAndFill
- `StrokeCap` - Butt, Round, Square
- `StrokeJoin` - Miter, Round, Bevel
- `BlendMode` - Porter-Duff and advanced blend modes
- `Shader` trait - Color/pattern sources
- `ColorFilter` - Color transformations
- `MaskFilter` - Blur effects
- `PathEffect` - Path modifications

**Shader Hierarchy:**
```
Shader (trait)
├── ColorShader         - Solid color
├── LinearGradient      - Linear gradient
├── RadialGradient      - Radial gradient
├── SweepGradient       - Angular gradient
├── ImageShader         - Image pattern
├── BlendShader         - Blend two shaders
├── PerlinNoiseShader   - Procedural noise
└── RuntimeShader       - SkSL custom shader
```

### skia-rs-canvas

Drawing operations and surface management.

**Key Types:**
- `Canvas` trait - Drawing interface
- `Surface` - Pixel backing store
- `RasterCanvas` - Software rasterizer
- `PixelBuffer` - Raw pixel storage
- `Picture` - Recorded draw commands
- `PictureRecorder` - Command recording

**Canvas Architecture:**
```rust
trait Canvas {
    fn save(&mut self);
    fn restore(&mut self);
    fn translate(&mut self, dx: Scalar, dy: Scalar);
    fn rotate(&mut self, radians: Scalar);
    fn scale(&mut self, sx: Scalar, sy: Scalar);
    fn concat(&mut self, matrix: &Matrix);
    
    fn clip_rect(&mut self, rect: &Rect, op: ClipOp);
    fn clip_path(&mut self, path: &Path, op: ClipOp);
    
    fn draw_rect(&mut self, rect: &Rect, paint: &Paint);
    fn draw_circle(&mut self, center: Point, radius: Scalar, paint: &Paint);
    fn draw_path(&mut self, path: &Path, paint: &Paint);
    // ... more draw methods
}
```

**Rasterizer Pipeline:**
```
draw_path(path, paint)
       │
       ▼
┌──────────────────┐
│ Transform Path   │  ← Apply canvas matrix
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Apply PathEffect │  ← Dash, corner, etc.
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Stroke → Fill    │  ← If stroke style
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Clip to Region   │  ← Apply clip stack
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Rasterize        │  ← Scanline fill
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Apply Shader     │  ← Sample colors
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Blend to Surface │  ← Porter-Duff
└──────────────────┘
```

### skia-rs-text

Text rendering and font management.

**Key Types:**
- `Typeface` - Font data and metrics
- `Font` - Typeface + size + style
- `FontStyle` - Weight, width, slant
- `TextBlob` - Positioned glyphs
- `TextBlobBuilder` - Glyph positioning
- `GlyphRun` - Run of same-styled glyphs

**Text Pipeline:**
```
"Hello World"
      │
      ▼
┌──────────────────┐
│ Text Shaping     │  ← rustybuzz (HarfBuzz)
│ (Unicode → Glyphs)│
└──────────────────┘
      │
      ▼
┌──────────────────┐
│ Layout/Position  │  ← Glyph positioning
└──────────────────┘
      │
      ▼
┌──────────────────┐
│ Rasterize Glyphs │  ← Glyph outlines to paths
└──────────────────┘
      │
      ▼
┌──────────────────┐
│ Cache & Render   │  ← Glyph cache
└──────────────────┘
```

### skia-rs-codec

Image encoding and decoding.

**Key Types:**
- `Image` - Immutable image data
- `ImageDecoder` trait - Decode from bytes
- `ImageEncoder` trait - Encode to bytes
- `ImageFormat` - PNG, JPEG, WebP, etc.
- `ImageGenerator` - Deferred image loading

**Supported Formats:**
| Format | Decode | Encode |
|--------|--------|--------|
| PNG | ✅ | ✅ |
| JPEG | ✅ | ✅ |
| WebP | ✅ | ✅ |
| GIF | ✅ | ❌ |
| BMP | ✅ | ✅ |
| ICO | ✅ | ❌ |
| AVIF | ✅ | ✅ |
| RAW | ✅ | ❌ |

### skia-rs-gpu

GPU rendering backends.

**Backend Abstraction:**
```rust
trait GpuBackend {
    fn create_surface(&self, info: &ImageInfo) -> Result<GpuSurface>;
    fn flush(&mut self);
    fn submit(&mut self);
}

enum GpuBackendType {
    Wgpu,    // Cross-platform via wgpu
    Vulkan,  // Direct Vulkan via ash
    OpenGL,  // Direct OpenGL via glow
    Metal,   // Direct Metal via metal-rs
}
```

**GPU Pipeline:**
```
Draw Commands
      │
      ▼
┌──────────────────┐
│ Tessellation     │  ← Paths to triangles
└──────────────────┘
      │
      ▼
┌──────────────────┐
│ Stencil-Cover    │  ← Complex path filling
└──────────────────┘
      │
      ▼
┌──────────────────┐
│ Shader Programs  │  ← WGSL/GLSL/MSL
└──────────────────┘
      │
      ▼
┌──────────────────┐
│ GPU Execution    │  ← Backend-specific
└──────────────────┘
```

### skia-rs-svg

SVG parsing and rendering.

**Key Types:**
- `SvgDom` - Parsed SVG document
- `SvgNode` - DOM node types
- `Stylesheet` - CSS styling
- `SvgRenderer` - Render to canvas

### skia-rs-pdf

PDF document generation.

**Key Types:**
- `PdfDocument` - PDF file builder
- `PdfPage` - Single page
- `PdfCanvas` - Drawing to PDF
- `PdfFontManager` - Font embedding

### skia-rs-skottie

Lottie animation playback.

**Key Types:**
- `Animation` - Loaded animation
- `Layer` - Animation layer
- `Shape` - Animated shapes
- `Transform` - Animated transforms

### skia-rs-ffi

C API bindings.

**Design:**
- Opaque pointer types for all objects
- Reference counting via `RefCounted<T>`
- Panic catching at FFI boundary
- C header generation via cbindgen

## Memory Layout

### Core Types

```
Point (8 bytes)
┌─────────┬─────────┐
│ x: f32  │ y: f32  │
└─────────┴─────────┘

Rect (16 bytes)
┌─────────┬─────────┬─────────┬─────────┐
│left: f32│top: f32 │right:f32│bot: f32 │
└─────────┴─────────┴─────────┴─────────┘

Matrix (36 bytes)
┌─────────────────────────────────────┐
│ values: [f32; 9]                    │
│ [sx, kx, tx, ky, sy, ty, p0, p1, p2]│
└─────────────────────────────────────┘

Color (4 bytes)
┌────────────────────────────────────┐
│ ARGB: u32 (0xAARRGGBB)             │
└────────────────────────────────────┘

Color4f (16 bytes)
┌─────────┬─────────┬─────────┬─────────┐
│ r: f32  │ g: f32  │ b: f32  │ a: f32  │
└─────────┴─────────┴─────────┴─────────┘
```

### Surface Memory

```
Surface (N bytes)
┌─────────────────────────────────────────────┐
│ info: ImageInfo                             │
│ pixels: Vec<u8>  ← width * height * 4 bytes │
│ row_bytes: usize                            │
└─────────────────────────────────────────────┘

Example: 1920x1080 RGBA surface
Memory = 1920 × 1080 × 4 = 8,294,400 bytes ≈ 8 MB
```

## Threading Model

### Thread Safety Categories

| Type | Safety | Notes |
|------|--------|-------|
| `Point`, `Rect`, `Color` | Send + Sync | Immutable value types |
| `Matrix` | Send + Sync | Copy type |
| `Path` | Send + Sync | Immutable after build |
| `Paint` | Send + !Sync | Clone for thread use |
| `Surface` | !Send + !Sync | Single-thread access |
| `Image` | Send + Sync | Immutable pixel data |

### Recommended Patterns

```rust
// Parallel path building
let paths: Vec<Path> = data
    .par_iter()  // rayon
    .map(|d| build_path(d))
    .collect();

// Shared image across threads
let image = Arc::new(Image::decode(&data)?);
let image_clone = Arc::clone(&image);
thread::spawn(move || {
    // use image_clone
});

// Thread-local surfaces
thread_local! {
    static SURFACE: RefCell<Surface> = RefCell::new(
        Surface::new_raster_n32_premul(256, 256).unwrap()
    );
}
```

## Performance Optimizations

### SIMD

```rust
// Automatic SIMD detection
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

// Runtime feature detection
if is_x86_feature_detected!("avx2") {
    blend_avx2(dst, src, count);
} else if is_x86_feature_detected!("sse4.1") {
    blend_sse41(dst, src, count);
} else {
    blend_scalar(dst, src, count);
}
```

### Caching

```rust
// Glyph cache
struct GlyphCache {
    cache: HashMap<GlyphKey, CachedGlyph>,
    atlas: TextureAtlas,
    lru: VecDeque<GlyphKey>,
}

// Path bounds caching
struct Path {
    elements: Vec<PathElement>,
    bounds: OnceCell<Rect>,  // Lazy computation
}
```

### Memory Pooling

```rust
// Reuse allocations
struct Rasterizer {
    edge_pool: Vec<Edge>,
    span_pool: Vec<Span>,
    // Cleared but not deallocated between uses
}
```

## Extension Points

### Custom Shaders

```rust
impl Shader for MyShader {
    fn sample(&self, x: Scalar, y: Scalar) -> Color4f {
        // Custom color computation
    }
    
    fn is_opaque(&self) -> bool { false }
    fn shader_kind(&self) -> ShaderKind { ShaderKind::Color }
}
```

### Custom Path Effects

```rust
impl PathEffect for MyEffect {
    fn apply(&self, path: &Path) -> Option<Path> {
        // Transform the path
    }
}
```

### Custom Image Generators

```rust
impl ImageGenerator for MyGenerator {
    fn info(&self) -> ImageInfo { ... }
    fn generate(&self, pixels: &mut [u8]) -> bool { ... }
}
```

## Build Configuration

### Feature Flags

```toml
[features]
default = ["wgpu-backend"]
vulkan = ["dep:ash"]
opengl = ["dep:glow"]
metal = ["dep:metal"]
wgpu-backend = ["dep:wgpu"]
avif = ["dep:avif-decode", "dep:ravif"]
raw = ["dep:rawloader"]
```

### Platform Support

| Platform | Raster | WGPU | Vulkan | OpenGL | Metal |
|----------|--------|------|--------|--------|-------|
| Linux | ✅ | ✅ | ✅ | ✅ | ❌ |
| macOS | ✅ | ✅ | ❌ | ❌ | ✅ |
| Windows | ✅ | ✅ | ✅ | ✅ | ❌ |
| iOS | ✅ | ✅ | ❌ | ❌ | ✅ |
| Android | ✅ | ✅ | ✅ | ✅ | ❌ |
| WASM | ✅ | ✅ | ❌ | ❌ | ❌ |
