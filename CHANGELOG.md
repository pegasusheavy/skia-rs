# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.5] - 2026-04-25

Phase 6 complete: audited and completed the remaining six crates
(text, codec, ffi, svg, pdf, gpu). 80 gaps across six GAPS.md files;
all addressed with explicit resolution or documented deferrals.

### Added (skia-rs-text)
- Real Font::metrics from hhea/OS/2/post tables (replaced hardcoded approximations)
- Paragraph::layout routes through functional rustybuzz Shaper (was ad-hoc glyph mapping)
- Per-span style preservation via LineFragment
- Real glyph intercepts with de Casteljau flattening
- Color font table parsing (COLR v0/v1 layers, CBDT/CBLC/sbix/bdat, SVG-in-OpenType bytes, palette access)
- FontMgr::make_from_data, make_from_file, character-coverage matching
- GlyphRun bounds from real advances

### Added (skia-rs-codec)
- Image::read_pixels format conversion via skia_rs_core::convert_pixels
- Image::make_with_filter applies real ImageFilter from skia-rs-paint
- GpuImageBackend trait defining upload/read_back/release contract
- EncodedImageGenerator caches decoded image and reports real info
- SamplingOptions::{Nearest, Linear} and Image::make_scaled_with bilinear
- Fixed WebP RGB→RGBA decode bug (surfaced by new round-trip tests)

### Added (skia-rs-ffi)
- Panic catching wraps every exported sk_* function (soundness fix)
- Tag-validated RefCounted<T> rejects untagged pointers
- sk_init(major, minor) + sk_is_initialized() for runtime ABI check
- Expanded API: sk_canvas_t, sk_image_t, sk_typeface_t/sk_font_t,
  sk_shader_t, sk_colorfilter_t, sk_maskfilter_t, sk_imagefilter_t,
  gradient constructors, blur/saturation/matrix filters, image I/O,
  PNG encode, surface read_pixels, path iteration, matrix helpers
- cbindgen-driven include/skia_rs.h header and examples/draw_rect.c

### Added (skia-rs-svg)
- <text> element renders real glyph outlines via skia-rs-text
- Gradient url(#id) references resolved through Defs symbol table
- Gradient <stop> parsing from attrs or style; spread + transform
- CSS Stylesheet applied during render tree-walk
- Replaced hand-rolled XML parser with roxmltree (entities, CDATA,
  namespaces, text content all fixed)
- <image> data: base64 decode via skia-rs-codec
- <clipPath> dispatch via canvas.clip_path
- Extended color parsing (3/4/6/8-digit hex, rgb/a, hsl/a, ~140 CSS3 names)
- Extended length units (px/pt/pc/em/rem/ex/ch/vw/vh/vmin/vmax/cm/mm/in/%)

### Added (skia-rs-pdf)
- PdfDocument wires PdfFontManager, PdfImageManager, TransparencyManager
  so the Resources dict actually contains /Font, /XObject, /ExtGState
- Per-page used_fonts/used_images/used_ext_gstates tracking
- Real TrueType metrics from ttf_parser::Face (ascender/descender/
  italic_angle/cap_height/x_height/bbox/per-code widths)
- FNV-1a subset-prefix tag on BaseFont; FontFile2 stream emission
- PDF/A validation wired into write_to with XMP metadata + OutputIntent
- ExtGState registration for alpha < 1 or non-Normal blend mode
- set_alpha/set_blend_mode/draw_image/draw_text_with_font methods
- Conic flattening via skia_rs_path::flatten::flatten_conic_adaptive
- ToUnicode CMap from recorded used_chars (surrogate-pair aware)

### Added (skia-rs-gpu)
- WgpuExecutor + WgpuPipelineCache consume CommandBuffer + RenderPipelineDescriptor
- Paint→pipeline bridge (paint_bridge.rs): PipelineSelection, BlendMode→BlendState, uniform packing
- Ear-clipping triangulator replaces fan (handles concave correctly)
- Adaptive conic flattening in tessellation
- WgpuStencilSurface with full depth/stencil translation (wires stencil-cover algorithm)
- Naga WGSL validation in shader compilation
- Felzenszwalb-Huttenlocher O(n) SDF distance transform
- Atlas compact() + free()
- MSAA resolve path in read_pixels

### Fixed
- All six crates: GAPS.md annotated with per-gap Status lines

### Test counts
- skia-rs-text: 19 → 57
- skia-rs-codec: 26 → 47
- skia-rs-ffi: 13 → 28
- skia-rs-svg: 21 → 39
- skia-rs-pdf: 27 → 62
- skia-rs-gpu: 77 → 104 (default), 78 → 115 (wgpu)
- **Workspace total: 695 → 718+ tests, 0 failures**

### Deferred (documented in per-crate GAPS.md)
- Text: COLR v1 gradients/transforms, SVG-in-OpenType rasterization
- Codec: RAW demosaic (needs color-pipeline library), ICC profile chain
- FFI: full API surface expansion (text blobs, codec streaming, runtime effects, matrix44, picture, GPU FFI, SVG/PDF FFI)
- SVG: CIE-lab/oklch color spaces (upstream color-management)
- PDF: byte-level TTF subsetting, external veraPDF validation in CI
- GPU: OpenGL/Vulkan/Metal executors (wgpu path is fully functional), real-GPU CI setup

## [0.2.4] - 2026-04-25

### Added (skia-rs-canvas)
- **Canvas unified via Backing enum**: `Canvas<'a>` now carries
  `Backing::Raster(&mut PixelBuffer) | Recording(&mut Vec<DrawCommand>) | Null`.
  All 11 critical draw methods now dispatch via match and render correctly.
  Picture playback works end-to-end with pixel-identical round-trip (C-1..C-7)
- **ClipStack-based clip**: Canvas clip is now full `ClipStack` with
  Difference ops, AA, and path clipping (C-8, C-9, N-1)
- **draw_points with PointMode**: Points/Lines/Polygon modes (C-4)
- **save_layer offscreen composition**: allocates per-layer buffer,
  composites back on restore with paint alpha + blend mode, nests (C-10)
- **Barycentric color interpolation in draw_vertices**: per-vertex
  Gouraud shading for Triangles/Strip/Fan (N-5)
- **draw_image_lattice + draw_atlas + draw_patch**: real implementations
  dispatching to draw_image_rect / draw_vertices
- **Coons patch draw_patch**: full bicubic surface with boundary-curve
  evaluation and optional corner color interpolation
- **Real glyph outlines in text rendering** via ttf-parser (N-6):
  - New `Font::glyph_path()` returns a Path with screen-space scaling
  - Real `cmap` and `hmtx` parsing in skia-rs-text Typeface
  - `Canvas::draw_string`, `draw_text_blob`, `draw_glyphs` now render
    actual character shapes, not rectangles

### Changed (skia-rs-canvas)
- `Surface::canvas()` now returns a functional raster Canvas (was a stub)
- `RasterCanvas` is a deprecated type alias for `Canvas<'a>` (migration path)
- SSE4.1 `fill_span_blend` dispatch disabled — fell back to scalar due
  to incorrect per-channel alpha handling. AVX2 and NEON paths unchanged.

### Fixed (skia-rs-canvas)
- `ClipStack::clip_rect_aa` now intersects both region and mask in the
  `RegionAndMask` branch (N-4)
- `draw_vertices` flat-shade fallback preserved when no colors provided

### Tests (skia-rs-canvas)
- Test count grew from 39 to 103 in skia-rs-canvas
- New pixel-level round-trip tests for Picture playback (Circle, Path,
  Oval, Arc, RoundRect, Line, ClipRect, Scale)
- Matrix stack, SaveLayerRec, RSXform, FilterMode, PointMode coverage

## [0.2.3] - 2026-04-25

### Added (skia-rs-paint)

- **Full filter pipeline**: Paint now carries `mask_filter`, `color_filter`,
  and `image_filter` fields with getter/setter pairs (GAP-C1)
- **BlendMode::apply** for all 29 blend modes — Porter-Duff (Clear/Src/Dst/
  SrcOver/DstOver/SrcIn/DstIn/SrcOut/DstOut/SrcATop/DstATop/Xor/Plus/Modulate),
  separable (Screen/Overlay/Darken/Lighten/ColorDodge/ColorBurn/HardLight/
  SoftLight/Difference/Exclusion/Multiply), and non-separable HSL
  (Hue/Saturation/Color/Luminosity) (GAP-C6)
- **All shader sample() implementations**: LocalMatrixShader, BlendShader,
  ComposeShader, TwoPointConicalGradient, PerlinNoiseShader, ImageShader
  (with pixel data) (GAP-C3/C4/C5/C7/C8)
- **MaskFilter::apply_mask** for separable box-blur approximation of
  Gaussian blur (GAP-N3)
- **ImageFilter::apply** for all 14 concrete filter types: Blur, DropShadow,
  Morphology, ColorFilter, DisplacementMap, Lighting, Compose, Merge,
  Offset, MatrixConvolution, Tile (and three multi-input types with
  documented limitations) (GAP-N4/N5/N6)
- **SkSL tree-walking interpreter**: RuntimeShader::sample and
  RuntimeColorFilter::filter_color now evaluate SkSL at runtime with
  full Stmt/Expr coverage and approximately 35 built-in functions (GAP-C9/C10)
- **Full WGSL code generation**: every Stmt and Expr variant now emits
  valid WGSL (GAP-C11)
- **Dedicated MSL code generation**: separate emitters using MSL type
  syntax (float4, discard_fragment, etc.) (GAP-C12)
- **SPIR-V compilation**: compile_to(SpirV) and compile_to_spirv() via
  naga WGSL to SPIR-V path (GAP-N9)
- **SkSL semantic validation**: type checking, scope resolution, arity
  checks, return-type validation, builtin signatures (GAP-N10)
- **SkSL preprocessor**: #define, #undef, #ifdef, #ifndef, #else, #endif
  with whole-identifier text substitution (GAP-N11)
- **SkSL layout qualifier parsing**: layout(color) uniform and similar
  annotations now parse cleanly (GAP-N11)
- **ColorMatrixFilter convenience constructors**: brightness, contrast,
  hue_rotate, invert, sepia, grayscale (GAP-N1)
- **EffectKind entry-point validation**: main(vec2)->vec4 for Shader,
  main(vec4)->vec4 for ColorFilter, main(vec4,vec4)->vec4 for Blender
  (GAP-N8)
- **RuntimeEffect compilation caches**: GLSL/WGSL/MSL/SPIR-V results
  cached via OnceLock (GAP-N7)
- **Paint serialization** now round-trips shader, mask_filter, color_filter,
  and image_filter (GAP-C2)

### Changed (skia-rs-paint)

- GradientFlags now uses the bitflags crate (GAP-N2)
- GradientFlags::INTERPOLATE_PREMUL flag is honored in gradient
  interpolation (GAP-N2)
- RuntimeEffectError now uses thiserror derive (GAP-N12)

### Tests (skia-rs-paint)

- Test count grew from 17 to 145+ across skia-rs-paint
- Added property-based tests via proptest for blend mode finiteness
  and serialization round-trips
- All 31 audit gaps have test coverage

## [0.2.2] - 2026-04-25

### Added (skia-rs-path)

- **PathMeasure fully implemented**: `compute_lengths`, `get_point_at`,
  `get_tangent_at`, `get_matrix_at`, `get_segment`, `contour_count`,
  `contour_length` now functional (GAP-C1)
- Adaptive curve flattening utilities (`flatten_quad_adaptive`,
  `flatten_cubic_adaptive`, `flatten_conic_adaptive`) in internal
  `flatten` module
- `StrokeJoin::Round` now generates actual arc segments instead of a
  straight-line midpoint approximation (GAP-N7)
- `Path::tight_bounds()` now computes from quadratic and cubic curve
  extrema, not control points (GAP-N1)
- `Path::is_oval()` verifies cardinal-point endpoint geometry rather
  than only counting verb types (GAP-N4)
- `Path::convexity()` result is now cached via `AtomicU8` for repeat
  calls — Send+Sync compatible (GAP-N5)

### Fixed (skia-rs-path)

- `TrimEffect::apply` now extracts the actual sub-segment using
  `PathMeasure::get_segment` (GAP-C2). `Path1DEffect` (GAP-C3)
  auto-fixed by the same PathMeasure implementation.
- `DashEffect` now flattens curves before applying dash intervals,
  allowing dash transitions mid-curve (GAP-C6)
- `stroke_to_fill` tracks `is_closed` per contour, fixing multi-contour
  paths with mixed open/closed states (GAP-N8)
- Conic weight is now honored in `path_to_polygons` via the
  rational-quadratic evaluator (GAP-N6)
- Removed unused dependencies (`thiserror`, `arrayvec`, `proptest`)
- Cleaned up compiler warnings (unused `Verb` import, unnecessary `mut`)

### Known Limitations (skia-rs-path)

- Boolean operations (`PathOp::Difference`, `Intersect`, `Xor`) still
  have known correctness issues for non-convex inputs and partial
  overlaps. Tracked as GAP-C4 and GAP-C5. Limitations documented in
  the public rustdoc. Planned for a future release with a proper
  polygon-clipping algorithm (Weiler-Atherton or sweep-line).
- `Path::length()` and `Path::contains()` use control-polygon
  approximations for curves. Acceptable for typical use; tracked for
  incremental improvement.

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
