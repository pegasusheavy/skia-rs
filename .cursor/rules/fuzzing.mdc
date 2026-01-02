---
description: Fuzzing setup and guidelines for skia-rs
globs: ["fuzz/**"]
alwaysApply: false
---

# Fuzzing

## Setup

```bash
# Install nightly toolchain and cargo-fuzz
rustup install nightly
cargo install cargo-fuzz
```

## Running Fuzz Tests

```bash
# Run a fuzz target
cd fuzz
cargo +nightly fuzz run fuzz_point

# Run with time limit
cargo +nightly fuzz run fuzz_matrix -- -max_total_time=60

# List all targets
cargo +nightly fuzz list
```

## Available Fuzz Targets

- `fuzz_point` - Point operations
- `fuzz_rect` - Rectangle operations
- `fuzz_matrix` - Matrix transformations
- `fuzz_color` - Color conversions
- `fuzz_path` - Path construction
- `fuzz_path_builder` - PathBuilder shapes
- `fuzz_paint` - Paint configuration
- `fuzz_canvas` - Canvas operations

## Writing Fuzz Targets

```rust
#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
struct MyInput {
    value: f32,
    flag: bool,
}

fuzz_target!(|input: MyInput| {
    // Skip invalid inputs early
    if !input.value.is_finite() {
        return;
    }

    // Test code that should not panic
    let result = my_function(input.value);

    // Assert invariants
    assert!(result.is_valid());
});
```

## Best Practices

- Use `Arbitrary` derive for structured input
- Validate and skip invalid inputs early
- Limit resource usage (iteration counts, sizes)
- Assert invariants, not expected failures
- Keep fuzz targets focused on specific functionality
- The `fuzz` crate uses `edition = "2021"` due to `libfuzzer-sys` requirements
