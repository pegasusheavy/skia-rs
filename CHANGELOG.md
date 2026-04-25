# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-04-25

### Fixed - skia-rs-core

- `Matrix::map_point` now guards against divide-by-zero in perspective transforms (GAP-C4)
- `Region::contains_rect` correctly handles rects spanning multiple components (GAP-C2)
- `Region::union` merges overlapping rectangles to prevent unbounded growth (GAP-C3)
- `Matrix::skew` uses raw skew factors matching Skia's API (GAP-N1)
- `Matrix::invert` uses f32-appropriate singularity threshold (GAP-N5)
- `RRect::from_rect_xy` clamps radii to half-dimensions (GAP-N2)

### Added - skia-rs-core

- `Matrix44::ortho_checked` and `perspective_checked` variants returning `Option` (GAP-N4)
- `Matrix44::get`/`set` now have explicit bounds-check messages (GAP-N3)
- `convert_pixels` handles alpha type conversion (Premul ↔ Unpremul) (GAP-N6)
- Documented limitation in `IccProfile::from_bytes` regarding non-sRGB profiles (GAP-C1)

### Removed - skia-rs-core

- Orphan source files `matrix.rs` and `scalar.rs` (dead code) (GAP-C5, GAP-N7)

### Known Limitations - skia-rs-core

- `IccProfile::from_bytes` does not yet parse tag tables; non-sRGB profiles will
  report incorrect color space metadata. Tracked as GAP-C1, deferred to a
  future release for full ICC support.

## [0.1.0] - 2026-01-02

### 🎉 Initial Release

The first public release of skia-rs, a pure Rust implementation of the Skia 2D graphics library.

### Added

#### Core Types (`skia-rs-core`)
- `Point`, `IPoint` - 2D point types with arithmetic operations
- `Rect`, `IRect` - Rectangle types with intersection, union, and containment
- `RRect` - Rounded rectangle with per-corner radii
- `Matrix` - 3x3 transformation matrix with all standard operations
- `Matrix44` - 4x4 transformation matrix for 3D graphics
- `Color`, `Color4f` - 32-bit and floating-point color types
- `ColorSpace` - sRGB and linear color space support
- `ImageInfo`, `Pixmap`, `Bitmap` - Pixel storage and metadata
- `Region` - Complex clipping region with boolean operations
- ICC profile support for color management

#### Path System (`skia-rs-path`)
- `Path` - Complete path representation with all verb types
- `PathBuilder` - Fluent API for path construction
- `PathMeasure` - Path length and position calculations
- `PathOps` - Boolean operations (union, intersect, difference, xor)
- `PathEffect` - Dash, corner, discrete, trim effects
- SVG path parsing with full command support
- Arc approximation using cubic Bézier curves

#### Paint & Effects (`skia-rs-paint`)
- `Paint` - Full paint properties (color, stroke, anti-alias, etc.)
- `BlendMode` - All Porter-Duff and advanced blend modes
- `Style` - Fill, stroke, and stroke-and-fill styles
- Shaders:
  - `ColorShader` - Solid color
  - `LinearGradient`, `RadialGradient`, `SweepGradient` - Gradient fills
  - `TwoPointConicalGradient` - Two-point conical gradients
  - `ImageShader` - Image-based shaders
  - `BlendShader`, `ComposeShader` - Shader composition
  - `PerlinNoiseShader` - Procedural noise
- Color filters: matrix, lighting, blend mode
- Mask filters: blur, shader-based, table/gamma
- Image filters: blur, drop shadow, morphology, displacement, lighting, convolution

#### Canvas & Drawing (`skia-rs-canvas`)
- `Surface` - Drawing target with pixel storage
- `Canvas` - Full drawing API with save/restore stack
- Software rasterizer with anti-aliased rendering
- Drawing operations:
  - Shapes: rect, round rect, oval, circle, arc, path
  - Lines and points
  - Images with various sampling options
  - Text (via text crate integration)
- Clipping: rect and path-based
- Transformations: translate, rotate, scale, skew, concat
- `Picture` and `PictureRecorder` for recording/playback

#### Text (`skia-rs-text`)
- `Typeface` - Font face abstraction
- `Font` - Font with size and style properties
- `FontMetrics` - Font measurement data
- `TextBlob`, `TextBlobBuilder` - Positioned glyph runs
- `FontMgr` - Font enumeration and matching
- `Paragraph`, `ParagraphBuilder` - Rich text layout
- Text shaping via rustybuzz integration

#### Image Codecs (`skia-rs-codec`)
- `Image` - Immutable image with pixel access
- PNG encoding and decoding
- JPEG encoding and decoding
- GIF encoding and decoding
- WebP encoding and decoding
- Automatic format detection

#### GPU (`skia-rs-gpu`)
- wgpu backend foundation (in progress)
- `WgpuContext` - GPU context management
- `WgpuSurface` - GPU-backed surfaces
- `WgpuTexture` - Texture management

#### SVG (`skia-rs-svg`)
- SVG DOM parsing
- Basic element support (rect, circle, ellipse, path, etc.)
- Style attribute parsing

#### PDF (`skia-rs-pdf`)
- `PdfDocument` - PDF document creation
- `PdfPage` - Page management
- `PdfCanvas` - Drawing to PDF

#### FFI (`skia-rs-ffi`)
- C-compatible bindings for core types
- Opaque pointer-based API
- Static and dynamic library outputs

#### Safe API (`skia-rs-safe`)
- Unified re-export of all crates
- Feature flags for optional components
- High-level ergonomic API

### Performance

- Optimized software rasterizer achieving 68x speedup for rectangle fills
- SIMD-friendly memory layouts
- Minimal allocations in hot paths
- Comprehensive benchmark suite

### Testing

- Unit tests for all public APIs
- Property-based testing with proptest
- 17 fuzz targets covering major subsystems
- CI/CD with GitHub Actions

### Documentation

- Rustdoc for all public items
- Example code in documentation
- Benchmark documentation (`BENCHMARK.md`)

---

## [Unreleased]

### Planned for v0.2.0
- Complete wgpu GPU backend
- Vulkan backend
- SVG export
- Extended codec support (BMP, ICO, HEIF)
- Performance optimizations with SIMD

[0.1.0]: https://github.com/pegasusheavy/skia-rs/releases/tag/v0.1.0
[Unreleased]: https://github.com/pegasusheavy/skia-rs/compare/v0.1.0...HEAD
