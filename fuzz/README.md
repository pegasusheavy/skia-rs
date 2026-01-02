# Skia-RS Fuzzing

This directory contains fuzz targets for testing skia-rs with `cargo-fuzz` and libFuzzer.

## Prerequisites

1. Install the nightly Rust toolchain:
   ```bash
   rustup install nightly
   ```

2. Install cargo-fuzz:
   ```bash
   cargo install cargo-fuzz
   ```

## Available Fuzz Targets

| Target | Description |
|--------|-------------|
| `fuzz_point` | Point operations (add, sub, length, normalize, dot, cross) |
| `fuzz_rect` | Rectangle operations (contains, intersect, join, offset, inset) |
| `fuzz_matrix` | Matrix transformations (translate, scale, rotate, invert, map_point) |
| `fuzz_color` | Color operations (ARGB, Color4f, premultiply, lerp) |
| `fuzz_path` | Path construction with arbitrary commands |
| `fuzz_path_builder` | PathBuilder shape methods (rect, oval, circle, round_rect) |
| `fuzz_paint` | Paint configuration (colors, styles, stroke settings) |
| `fuzz_canvas` | Canvas operations (transforms, clipping, drawing) |

## Running Fuzz Tests

### Run a specific fuzz target:
```bash
cd fuzz
cargo +nightly fuzz run fuzz_point
```

### Run with a time limit (e.g., 60 seconds):
```bash
cargo +nightly fuzz run fuzz_point -- -max_total_time=60
```

### Run with multiple jobs:
```bash
cargo +nightly fuzz run fuzz_point -- -jobs=4 -workers=4
```

### List all available targets:
```bash
cargo +nightly fuzz list
```

## Corpus Management

Fuzz inputs are stored in `fuzz/corpus/<target_name>/`. These are automatically generated and should be committed to preserve interesting test cases.

### Minimize corpus:
```bash
cargo +nightly fuzz cmin fuzz_point
```

## Reproducing Crashes

When a crash is found, it's saved to `fuzz/artifacts/<target_name>/`. To reproduce:

```bash
cargo +nightly fuzz run fuzz_point fuzz/artifacts/fuzz_point/<crash_file>
```

## Coverage

To generate coverage reports:

```bash
cargo +nightly fuzz coverage fuzz_point
```

## Writing New Fuzz Targets

1. Create a new file in `fuzz/fuzz_targets/`:
   ```rust
   #![no_main]

   use arbitrary::Arbitrary;
   use libfuzzer_sys::fuzz_target;

   #[derive(Debug, Arbitrary)]
   struct MyInput {
       // Define structured input
   }

   fuzz_target!(|input: MyInput| {
       // Test code here
   });
   ```

2. Add the target to `fuzz/Cargo.toml`:
   ```toml
   [[bin]]
   name = "fuzz_my_target"
   path = "fuzz_targets/fuzz_my_target.rs"
   test = false
   doc = false
   bench = false
   ```

## Best Practices

1. **Use `Arbitrary` derive** for structured fuzzing input
2. **Validate inputs early** - skip invalid/uninteresting inputs
3. **Limit resource usage** - cap iteration counts and sizes
4. **Assert invariants** - verify expected properties hold
5. **Don't assert on expected failures** - only assert on unexpected behavior

## Continuous Fuzzing

For continuous fuzzing in CI, consider using:
- [OSS-Fuzz](https://google.github.io/oss-fuzz/)
- [cargo-fuzz with GitHub Actions](https://rust-fuzz.github.io/book/cargo-fuzz/continuous-fuzzing.html)
