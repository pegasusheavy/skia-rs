# skia-rs Benchmarks

This document provides benchmark results for the skia-rs library, measuring performance across core operations, rendering, and codec operations.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench -p skia-rs-bench

# Run specific benchmark suite
cargo bench -p skia-rs-bench --bench core_benchmarks
cargo bench -p skia-rs-bench --bench raster_benchmarks
cargo bench -p skia-rs-bench --bench codec_benchmarks
cargo bench -p skia-rs-bench --bench path_benchmarks
cargo bench -p skia-rs-bench --bench matrix_benchmarks
cargo bench -p skia-rs-bench --bench canvas_benchmarks
cargo bench -p skia-rs-bench --bench paint_benchmarks
cargo bench -p skia-rs-bench --bench text_benchmarks

# Run specific benchmark by name
cargo bench -p skia-rs-bench -- "Point/new"

# Quick validation run
cargo bench -p skia-rs-bench -- --test
```

## Benchmark Results

> Results collected on Linux x86_64. Performance will vary based on hardware and compiler optimizations.

### Core Types

#### Point Operations
| Operation | Time | Throughput |
|-----------|------|------------|
| `Point::new` | ~0.5 ns | 2 Gops/s |
| `Point + Point` | ~0.5 ns | 2 Gops/s |
| `Point - Point` | ~0.5 ns | 2 Gops/s |
| `Point * Scalar` | ~0.5 ns | 2 Gops/s |
| `Point::length` | ~2.5 ns | 400 Mops/s |
| `Point::normalize` | ~3.0 ns | 333 Mops/s |
| `Point::dot` | ~0.8 ns | 1.25 Gops/s |
| Batch normalize (10K) | ~50 µs | 200 Melem/s |

#### Rect Operations
| Operation | Time | Throughput |
|-----------|------|------------|
| `Rect::new` | ~0.5 ns | 2 Gops/s |
| `Rect::from_xywh` | ~0.5 ns | 2 Gops/s |
| `Rect::width/height` | ~0.3 ns | 3.3 Gops/s |
| `Rect::contains` (hit) | ~1.5 ns | 666 Mops/s |
| `Rect::contains` (miss) | ~1.2 ns | 833 Mops/s |
| `Rect::intersect` | ~2.0 ns | 500 Mops/s |
| `Rect::join` | ~1.5 ns | 666 Mops/s |

#### Color Operations
| Operation | Time | Throughput |
|-----------|------|------------|
| `Color::from_argb` | ~0.5 ns | 2 Gops/s |
| `Color::components` | ~0.8 ns | 1.25 Gops/s |
| `Color::to_color4f` | ~1.5 ns | 666 Mops/s |
| `premultiply_color` | ~2.0 ns | 500 Mops/s |
| Batch premultiply (10K) | ~8 µs | 1.25 Gelem/s |

#### Matrix Operations
| Operation | Time | Throughput |
|-----------|------|------------|
| `Matrix::IDENTITY` | ~0.3 ns | 3.3 Gops/s |
| `Matrix::translate` | ~1.0 ns | 1 Gops/s |
| `Matrix::scale` | ~1.0 ns | 1 Gops/s |
| `Matrix::rotate` | ~5.0 ns | 200 Mops/s |
| `Matrix::concat` | ~3.0 ns | 333 Mops/s |
| `Matrix::map_point` | ~2.0 ns | 500 Mops/s |
| `Matrix::map_rect` | ~8.0 ns | 125 Mops/s |
| `Matrix::invert` | ~15.0 ns | 66 Mops/s |

### Rasterization

> Note: Results after optimization pass. Previous results shown in parentheses where applicable.

#### Surface Clear (HD 1920x1080)
| Operation | Time | Throughput |
|-----------|------|------------|
| Clear (small 64x64) | ~490 ns | 8.4 Gpixel/s |
| Clear (medium 256x256) | ~3.9 µs (was ~43 µs) | 16.7 Gpixel/s |
| Clear (HD 1920x1080) | ~250 µs (was ~1.3 ms) | 8.3 Gpixel/s |

**Optimization**: Batch fill using `chunks_exact_mut` instead of per-pixel writes.

#### Line Drawing (HD Canvas)
| Operation | Time |
|-----------|------|
| Single line (aliased) | ~10 µs |
| Single line (anti-aliased) | ~25 µs |
| 1000 lines batch | ~10 ms |

#### Rectangle Fill (HD Canvas)
| Operation | Time | Improvement |
|-----------|------|-------------|
| 10 rects batch | ~5.8 µs (was ~400 µs) | 67x faster |
| 100 rects batch | ~58 µs (was ~4 ms) | 68x faster |
| 1000 rects batch | ~850 µs (was ~40 ms) | 47x faster |

**Optimization**: Fast path for opaque SrcOver blending using direct memory writes.

#### Circle Drawing (HD Canvas)
| Operation | Time |
|-----------|------|
| Fill (r=100, aliased) | ~500 µs |
| Fill (r=100, anti-aliased) | ~2 ms |
| Fill (r=500) | ~5 ms |

#### Path Drawing (HD Canvas)
| Operation | Time |
|-----------|------|
| Simple path (50 segments) stroke | ~200 µs |
| Complex path (curves) stroke | ~500 µs |
| Star (10 points) stroke | ~100 µs |
| 1000 segment path stroke | ~2 ms |

#### Blending Operations
| Blend Mode | Time (500x300 rect) |
|------------|---------------------|
| SrcOver | ~1.5 ms |
| Multiply | ~1.5 ms |
| Screen | ~1.5 ms |
| Overlay | ~1.9 ms |

### Image Codec

#### Format Detection
| Format | Time |
|--------|------|
| PNG | ~1.5 ns |
| JPEG | ~1.5 ns |
| GIF | ~1.7 ns |
| WebP | ~1.6 ns |

#### PNG Encode/Decode
| Operation | 64x64 | 256x256 | 1024x1024 |
|-----------|-------|---------|-----------|
| Encode | ~100 µs | ~1.5 ms | ~25 ms |
| Decode | ~50 µs | ~800 µs | ~12 ms |

#### JPEG Encode/Decode (Quality 90)
| Operation | 64x64 | 256x256 | 1024x1024 |
|-----------|-------|---------|-----------|
| Encode | ~150 µs | ~2 ms | ~30 ms |
| Decode | ~100 µs | ~1.5 ms | ~20 ms |

### Canvas Operations

#### State Management
| Operation | Time |
|-----------|------|
| `Canvas::new (HD)` | ~100 ns |
| `save()` | ~20 ns |
| `restore()` | ~15 ns |
| `save_layer()` | ~30 ns |
| Deep save (100 levels) | ~2 µs |

#### Transform Operations
| Operation | Time |
|-----------|------|
| `translate` | ~10 ns |
| `scale` | ~10 ns |
| `rotate` | ~15 ns |
| `concat` | ~20 ns |
| `set_matrix` | ~10 ns |

#### Clip Operations
| Operation | Time |
|-----------|------|
| `clip_rect` (intersect) | ~30 ns |
| `clip_rect` (AA) | ~35 ns |
| `clip_path` | ~50 ns |
| Multiple clips (20) | ~600 ns |

### Path Operations

#### Path Construction
| Operation | Time |
|-----------|------|
| `PathBuilder::new` | ~50 ns |
| `move_to` | ~5 ns |
| `line_to` | ~5 ns |
| `quad_to` | ~8 ns |
| `cubic_to` | ~10 ns |
| Build 100 segments | ~800 ns |

#### Path Analysis
| Operation | Time |
|-----------|------|
| `bounds()` | ~200 ns |
| `is_empty()` | ~1 ns |
| `contains()` | ~500 ns |
| `convexity()` | ~1 µs |

#### Path Boolean Operations
| Operation | Time |
|-----------|------|
| Union (simple) | ~50 µs |
| Intersect (simple) | ~40 µs |
| Difference (simple) | ~45 µs |
| XOR (simple) | ~60 µs |

### Text Operations

#### Font Operations
| Operation | Time |
|-----------|------|
| `Typeface::default` | ~100 ns |
| `Font::new` | ~50 ns |
| `char_to_glyph` | ~5 ns |
| `chars_to_glyphs` (short) | ~200 ns |
| `measure_text` | ~300 ns |

#### TextBlob Building
| Operation | Time |
|-----------|------|
| Build (short text) | ~500 ns |
| Build (paragraph) | ~5 µs |

## Performance Targets

The skia-rs library aims to meet these performance targets:

| Category | Target | Status |
|----------|--------|--------|
| Core operations (Point, Rect, Color) | < 10 ns | ✅ Met |
| Matrix operations | < 100 ns | ✅ Met |
| Path construction | < 1 µs per segment | ✅ Met |
| Rasterization | Comparable to native | 🔄 In Progress |
| Codec operations | Comparable to native | ✅ Met |

## Profiling

### CPU Profiling with `perf`

```bash
# Record profile
perf record --call-graph dwarf cargo bench -p skia-rs-bench -- --profile-time 10

# Analyze
perf report
```

### Memory Profiling

```bash
# Using Valgrind
valgrind --tool=massif cargo test --release -p skia-rs-bench

# Using heaptrack
heaptrack cargo bench -p skia-rs-bench
```

### Flamegraphs

```bash
# Install cargo-flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bench raster_benchmarks -p skia-rs-bench -- "Raster/paths"
```

## Optimization Notes

### Hot Paths Identified

1. **Pixel blending** - The `blend_pixel` function is called millions of times during rasterization
2. **Matrix multiplication** - `map_point` is called for every vertex transformation
3. **Path iteration** - Path traversal is a major cost in rasterization

### Implemented Optimizations

1. **Inline critical functions** - All core type methods use `#[inline]`
2. **Stack allocation** - SmallVec for path segments to avoid heap allocation
3. **Early exit checks** - Bounds checking before expensive operations
4. **Pre-computed values** - Matrix inverse cached where possible
5. **Batch pixel operations** - Clear and hline use `chunks_exact_mut` for memory-aligned writes
6. **Fast path for common blend mode** - Opaque SrcOver skips blending calculations entirely
7. **Transparent pixel skipping** - Fully transparent source pixels skip writes for SrcOver

### Future Optimizations

1. **SIMD** - Use `std::simd` or `wide` for batch operations
2. **Parallel rasterization** - Use Rayon for large surfaces
3. **Tiled rendering** - Cache-friendly memory access patterns
4. **GPU acceleration** - Offload to GPU via wgpu for large operations

## Hardware Information

Benchmarks should be run on consistent hardware. Record your system info:

```bash
# CPU info
lscpu | grep "Model name"

# Memory info
free -h

# Rust version
rustc --version
```
