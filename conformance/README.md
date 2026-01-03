# Backend Conformance Testing

This directory contains reference images and tooling for cross-backend conformance testing.

## Structure

```
conformance/
├── README.md           # This file
├── reference/          # Reference images (generated from raster backend)
└── test_cases.json     # Test case definitions
```

## Supported Backends

| Backend | Platforms | CI Support |
|---------|-----------|------------|
| **Raster** | All | ✅ Full |
| **WebGPU/WGPU** | All | ✅ Software (GL fallback) |
| **Vulkan** | Linux, Windows | ✅ With SDK |
| **OpenGL** | Linux, Windows | ✅ Mesa software |
| **Metal** | macOS | ✅ Native |

## Running Conformance Tests

### Generate Reference Images

```bash
# Generate raster reference images
cargo run --example conformance_gen -- --backend raster --output-dir conformance/reference
```

### Test a Specific Backend

```bash
# Test WebGPU backend
cargo run --example conformance_gen --features wgpu-backend -- --backend wgpu --output-dir test_output

# Compare with reference
compare -metric RMSE test_output/wgpu_solid_rect.png conformance/reference/raster_solid_rect.png null:
```

### CI Integration

The CI workflow automatically:
1. Builds each backend on supported platforms
2. Generates test images using `conformance_gen`
3. Compares output with reference images using ImageMagick
4. Reports RMSE (Root Mean Square Error) differences
5. Uploads artifacts for manual inspection

## Test Cases

### Basic Shapes
- `solid_rect` - Filled rectangle
- `stroked_rect` - Stroked rectangle
- `circle_fill` - Filled circle
- `circle_stroke` - Stroked circle
- `oval` - Filled oval/ellipse

### Lines
- `lines` - Multiple colored lines

### Paths
- `path_triangle` - Simple triangle path
- `path_star` - Star shape
- `path_bezier` - Cubic bezier curve
- `nested_paths` - Rectangle with hole (even-odd fill)
- `rounded_rect` - Rounded rectangle

### Compositing
- `overlapping_shapes` - Overlapping shapes
- `alpha_blending` - Semi-transparent shapes
- `antialiased_shapes` - Anti-aliased rendering

### Stroke Properties
- `stroke_widths` - Various stroke widths

## Tolerance Settings

Due to differences in anti-aliasing and floating-point precision:

| Comparison | RMSE Threshold |
|------------|----------------|
| Same backend | 0 |
| Cross-backend (no AA) | < 0.01 |
| Cross-backend (with AA) | < 0.05 |
| GPU vs CPU | < 0.10 |

## Adding New Test Cases

1. Add a new drawing function in `examples/conformance_gen.rs`
2. Add it to the `test_cases` vector
3. Regenerate reference images
4. Commit the new reference images

## Troubleshooting

### CI Failures

1. Check if the RMSE exceeds thresholds
2. Download artifacts to compare visually
3. Determine if difference is:
   - Platform-specific (acceptable)
   - Bug in backend (needs fix)
   - Reference image needs update

### Software Rendering

In CI, GPU backends fall back to software rendering:
- `LIBGL_ALWAYS_SOFTWARE=1` for OpenGL
- `WGPU_BACKEND=gl` for WebGPU
- Mesa drivers for Vulkan (lavapipe)

This may cause minor visual differences but ensures consistent testing.
