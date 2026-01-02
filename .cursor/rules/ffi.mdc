---
description: FFI (Foreign Function Interface) guidelines for skia-rs-ffi
globs: ["crates/skia-rs-ffi/**"]
alwaysApply: false
---

# FFI Guidelines

## C API Design

- All FFI functions in `skia-rs-ffi`
- Use opaque pointers for complex types
- Follow Skia's C API naming: `sk_{type}_{method}`
- Always check for null pointers

```rust
// skia-rs-ffi/src/lib.rs

/// Create a new paint object.
///
/// # Safety
/// Returns a valid pointer that must be freed with `sk_paint_delete`.
#[no_mangle]
pub unsafe extern "C" fn sk_paint_new() -> *mut Paint {
    Box::into_raw(Box::new(Paint::new()))
}

/// Delete a paint object.
///
/// # Safety
/// `paint` must be a valid pointer returned by `sk_paint_new`.
#[no_mangle]
pub unsafe extern "C" fn sk_paint_delete(paint: *mut Paint) {
    if !paint.is_null() {
        drop(Box::from_raw(paint));
    }
}

/// Set the paint color.
///
/// # Safety
/// `paint` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn sk_paint_set_color(paint: *mut Paint, color: u32) {
    if let Some(p) = paint.as_mut() {
        p.set_color32(Color(color));
    }
}
```

## Type Mappings

| Skia C++ | Rust | C FFI |
|----------|------|-------|
| `SkScalar` | `Scalar` (f32) | `float` |
| `SkPoint` | `Point` | `sk_point_t` |
| `SkRect` | `Rect` | `sk_rect_t` |
| `SkColor` | `Color` | `uint32_t` |
| `SkMatrix` | `Matrix` | `sk_matrix_t` |
| `SkPath*` | `*mut Path` | `sk_path_t*` |
| `SkPaint*` | `*mut Paint` | `sk_paint_t*` |
| `SkCanvas*` | `*mut Canvas` | `sk_canvas_t*` |

## Safety Requirements

- All FFI functions must be marked `unsafe`
- Document safety requirements in doc comments
- Check for null pointers before dereferencing
- Use `#[repr(C)]` for all FFI-visible structs
