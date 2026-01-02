---
description: Testing guidelines and patterns for skia-rs
globs: ["**/*.rs", "**/tests/**"]
alwaysApply: false
---

# Testing Guidelines

## Unit Tests

- Place tests in the same file as the code
- Test edge cases and error conditions
- Use property-based testing with `proptest` for numeric code

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_addition() {
        let p1 = Point::new(1.0, 2.0);
        let p2 = Point::new(3.0, 4.0);
        assert_eq!(p1 + p2, Point::new(4.0, 6.0));
    }
}
```

## Property Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn matrix_identity_preserves_point(x in -1000.0f32..1000.0, y in -1000.0f32..1000.0) {
        let p = Point::new(x, y);
        let result = Matrix::IDENTITY.map_point(p);
        prop_assert!((result.x - p.x).abs() < 1e-6);
        prop_assert!((result.y - p.y).abs() < 1e-6);
    }
}
```

## Conformance Testing

- Compare output against Skia reference implementation
- Use the `skia/` submodule for reference
- Document any intentional behavioral differences

## Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p skia-rs-core

# Run with output
cargo test --workspace -- --nocapture
```
