//! C FFI bindings for skia-rs.
//!
//! This crate provides C-compatible bindings for use from other languages.
//! It exposes a C API that mirrors the Skia C API for drop-in compatibility.
//!
//! # Safety
//!
//! All FFI functions are inherently unsafe. Callers must ensure:
//! - Pointers are valid and non-null (unless explicitly documented otherwise)
//! - Proper lifetime management (using the appropriate `_unref` functions)
//! - Thread safety when accessing shared objects

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_void, CStr};
use std::ptr;

// Re-export types for FFI
use skia_rs_canvas::{PixelBuffer, RasterCanvas, Surface};
use skia_rs_core::{
    AlphaType, Color, ColorType, IPoint, IRect, ISize, ImageInfo, Matrix, Point, Rect, Scalar,
    Size,
};
use skia_rs_paint::{BlendMode, Paint, Style};
use skia_rs_path::{FillType, Path, PathBuilder};

// =============================================================================
// Type Definitions
// =============================================================================

/// Opaque surface type.
pub type SkSurface = Surface;
/// Opaque paint type.
pub type SkPaint = Paint;
/// Opaque path type.
pub type SkPath = Path;
/// Opaque path builder type.
pub type SkPathBuilder = PathBuilder;
/// Opaque matrix type.
pub type SkMatrix = Matrix;

/// C-compatible point structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_point_t {
    /// X coordinate.
    pub x: f32,
    /// Y coordinate.
    pub y: f32,
}

/// C-compatible integer point structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_ipoint_t {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
}

/// C-compatible size structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_size_t {
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

/// C-compatible integer size structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_isize_t {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
}

/// C-compatible rectangle structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_rect_t {
    /// Left edge.
    pub left: f32,
    /// Top edge.
    pub top: f32,
    /// Right edge.
    pub right: f32,
    /// Bottom edge.
    pub bottom: f32,
}

/// C-compatible integer rectangle structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct sk_irect_t {
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge.
    pub right: i32,
    /// Bottom edge.
    pub bottom: i32,
}

/// C-compatible matrix structure.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct sk_matrix_t {
    /// Matrix values (row-major).
    pub values: [f32; 9],
}

impl Default for sk_matrix_t {
    fn default() -> Self {
        Self {
            values: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// C-compatible image info structure.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct sk_imageinfo_t {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
    /// Color type.
    pub color_type: u32,
    /// Alpha type.
    pub alpha_type: u32,
}

/// C-compatible color (ARGB).
pub type sk_color_t = u32;

// =============================================================================
// Conversion helpers
// =============================================================================

impl From<Point> for sk_point_t {
    fn from(p: Point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<sk_point_t> for Point {
    fn from(p: sk_point_t) -> Self {
        Point::new(p.x, p.y)
    }
}

impl From<Rect> for sk_rect_t {
    fn from(r: Rect) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

impl From<sk_rect_t> for Rect {
    fn from(r: sk_rect_t) -> Self {
        Rect::new(r.left, r.top, r.right, r.bottom)
    }
}

impl From<Matrix> for sk_matrix_t {
    fn from(m: Matrix) -> Self {
        Self { values: m.values }
    }
}

impl From<sk_matrix_t> for Matrix {
    fn from(m: sk_matrix_t) -> Self {
        Matrix { values: m.values }
    }
}

// =============================================================================
// Surface API
// =============================================================================

/// Create a new raster surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_new_raster(
    width: i32,
    height: i32,
) -> *mut SkSurface {
    match Surface::new_raster_n32_premul(width, height) {
        Some(surface) => Box::into_raw(Box::new(surface)),
        None => ptr::null_mut(),
    }
}

/// Create a raster surface with specific image info.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_new_raster_with_info(
    info: *const sk_imageinfo_t,
) -> *mut SkSurface {
    if info.is_null() {
        return ptr::null_mut();
    }

    let info = &*info;
    let color_type = match info.color_type {
        0 => ColorType::Unknown,
        1 => ColorType::Alpha8,
        2 => ColorType::Rgb565,
        3 => ColorType::Argb4444,
        4 => ColorType::Rgba8888,
        5 => ColorType::Bgra8888,
        _ => ColorType::Rgba8888,
    };

    let alpha_type = match info.alpha_type {
        0 => AlphaType::Unknown,
        1 => AlphaType::Opaque,
        2 => AlphaType::Premul,
        3 => AlphaType::Unpremul,
        _ => AlphaType::Premul,
    };

    let img_info = match ImageInfo::new(info.width, info.height, color_type, alpha_type) {
        Ok(i) => i,
        Err(_) => return ptr::null_mut(),
    };

    match Surface::new_raster(&img_info, None) {
        Some(surface) => Box::into_raw(Box::new(surface)),
        None => ptr::null_mut(),
    }
}

/// Delete a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_unref(surface: *mut SkSurface) {
    if !surface.is_null() {
        drop(Box::from_raw(surface));
    }
}

/// Get the width of a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_get_width(surface: *const SkSurface) -> i32 {
    if surface.is_null() {
        return 0;
    }
    (*surface).width()
}

/// Get the height of a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_get_height(surface: *const SkSurface) -> i32 {
    if surface.is_null() {
        return 0;
    }
    (*surface).height()
}

/// Get the pixel data from a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_peek_pixels(
    surface: *const SkSurface,
    out_pixels: *mut *const u8,
    out_row_bytes: *mut usize,
) -> bool {
    if surface.is_null() || out_pixels.is_null() || out_row_bytes.is_null() {
        return false;
    }

    let surface = &*surface;
    *out_pixels = surface.pixels().as_ptr();
    *out_row_bytes = surface.row_bytes();
    true
}

// =============================================================================
// Paint API
// =============================================================================

/// Create a new paint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_new() -> *mut SkPaint {
    Box::into_raw(Box::new(Paint::new()))
}

/// Clone a paint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_clone(paint: *const SkPaint) -> *mut SkPaint {
    if paint.is_null() {
        return ptr::null_mut();
    }
    Box::into_raw(Box::new((*paint).clone()))
}

/// Delete a paint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_delete(paint: *mut SkPaint) {
    if !paint.is_null() {
        drop(Box::from_raw(paint));
    }
}

/// Set the paint color.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_color(paint: *mut SkPaint, color: sk_color_t) {
    if let Some(p) = paint.as_mut() {
        p.set_color32(Color(color));
    }
}

/// Get the paint color.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_get_color(paint: *const SkPaint) -> sk_color_t {
    if let Some(p) = paint.as_ref() {
        p.color32().0
    } else {
        0
    }
}

/// Set the paint style.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_style(paint: *mut SkPaint, style: u32) {
    if let Some(p) = paint.as_mut() {
        let style = match style {
            0 => Style::Fill,
            1 => Style::Stroke,
            2 => Style::StrokeAndFill,
            _ => Style::Fill,
        };
        p.set_style(style);
    }
}

/// Set the stroke width.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_stroke_width(paint: *mut SkPaint, width: f32) {
    if let Some(p) = paint.as_mut() {
        p.set_stroke_width(width);
    }
}

/// Get the stroke width.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_get_stroke_width(paint: *const SkPaint) -> f32 {
    if let Some(p) = paint.as_ref() {
        p.stroke_width()
    } else {
        0.0
    }
}

/// Set anti-alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_set_antialias(paint: *mut SkPaint, aa: bool) {
    if let Some(p) = paint.as_mut() {
        p.set_anti_alias(aa);
    }
}

/// Check if anti-alias is enabled.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_paint_is_antialias(paint: *const SkPaint) -> bool {
    if let Some(p) = paint.as_ref() {
        p.is_anti_alias()
    } else {
        false
    }
}

// =============================================================================
// Path API
// =============================================================================

/// Create a new path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_new() -> *mut SkPath {
    Box::into_raw(Box::new(Path::new()))
}

/// Clone a path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_clone(path: *const SkPath) -> *mut SkPath {
    if path.is_null() {
        return ptr::null_mut();
    }
    Box::into_raw(Box::new((*path).clone()))
}

/// Delete a path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_delete(path: *mut SkPath) {
    if !path.is_null() {
        drop(Box::from_raw(path));
    }
}

/// Get the path bounds.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_get_bounds(path: *const SkPath, bounds: *mut sk_rect_t) {
    if let (Some(p), Some(b)) = (path.as_ref(), bounds.as_mut()) {
        let rect = p.bounds();
        *b = rect.into();
    }
}

/// Check if path is empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_is_empty(path: *const SkPath) -> bool {
    if let Some(p) = path.as_ref() {
        p.is_empty()
    } else {
        true
    }
}

/// Get the fill type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_get_filltype(path: *const SkPath) -> u32 {
    if let Some(p) = path.as_ref() {
        match p.fill_type() {
            FillType::Winding => 0,
            FillType::EvenOdd => 1,
            FillType::InverseWinding => 2,
            FillType::InverseEvenOdd => 3,
        }
    } else {
        0
    }
}

/// Set the fill type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_set_filltype(path: *mut SkPath, fill_type: u32) {
    if let Some(p) = path.as_mut() {
        let ft = match fill_type {
            0 => FillType::Winding,
            1 => FillType::EvenOdd,
            2 => FillType::InverseWinding,
            3 => FillType::InverseEvenOdd,
            _ => FillType::Winding,
        };
        p.set_fill_type(ft);
    }
}

/// Check if path contains a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_path_contains(path: *const SkPath, x: f32, y: f32) -> bool {
    if let Some(p) = path.as_ref() {
        p.contains(Point::new(x, y))
    } else {
        false
    }
}

// =============================================================================
// Path Builder API
// =============================================================================

/// Create a new path builder.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_new() -> *mut SkPathBuilder {
    Box::into_raw(Box::new(PathBuilder::new()))
}

/// Delete a path builder.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_delete(builder: *mut SkPathBuilder) {
    if !builder.is_null() {
        drop(Box::from_raw(builder));
    }
}

/// Move to a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_move_to(builder: *mut SkPathBuilder, x: f32, y: f32) {
    if let Some(b) = builder.as_mut() {
        b.move_to(x, y);
    }
}

/// Line to a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_line_to(builder: *mut SkPathBuilder, x: f32, y: f32) {
    if let Some(b) = builder.as_mut() {
        b.line_to(x, y);
    }
}

/// Quadratic bezier to a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_quad_to(
    builder: *mut SkPathBuilder,
    cx: f32,
    cy: f32,
    x: f32,
    y: f32,
) {
    if let Some(b) = builder.as_mut() {
        b.quad_to(cx, cy, x, y);
    }
}

/// Cubic bezier to a point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_cubic_to(
    builder: *mut SkPathBuilder,
    c1x: f32,
    c1y: f32,
    c2x: f32,
    c2y: f32,
    x: f32,
    y: f32,
) {
    if let Some(b) = builder.as_mut() {
        b.cubic_to(c1x, c1y, c2x, c2y, x, y);
    }
}

/// Close the path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_close(builder: *mut SkPathBuilder) {
    if let Some(b) = builder.as_mut() {
        b.close();
    }
}

/// Add a rectangle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_add_rect(builder: *mut SkPathBuilder, rect: *const sk_rect_t) {
    if let (Some(b), Some(r)) = (builder.as_mut(), rect.as_ref()) {
        b.add_rect(&Rect::from(*r));
    }
}

/// Add an oval.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_add_oval(builder: *mut SkPathBuilder, rect: *const sk_rect_t) {
    if let (Some(b), Some(r)) = (builder.as_mut(), rect.as_ref()) {
        b.add_oval(&Rect::from(*r));
    }
}

/// Add a circle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_add_circle(
    builder: *mut SkPathBuilder,
    cx: f32,
    cy: f32,
    radius: f32,
) {
    if let Some(b) = builder.as_mut() {
        b.add_circle(cx, cy, radius);
    }
}

/// Build the path (consumes the builder).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_pathbuilder_detach(builder: *mut SkPathBuilder) -> *mut SkPath {
    if builder.is_null() {
        return ptr::null_mut();
    }
    let builder = Box::from_raw(builder);
    Box::into_raw(Box::new(builder.build()))
}

// =============================================================================
// Matrix API
// =============================================================================

/// Set matrix to identity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_identity(matrix: *mut sk_matrix_t) {
    if let Some(m) = matrix.as_mut() {
        *m = sk_matrix_t::default();
    }
}

/// Set matrix to translate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_translate(matrix: *mut sk_matrix_t, dx: f32, dy: f32) {
    if let Some(m) = matrix.as_mut() {
        *m = Matrix::translate(dx, dy).into();
    }
}

/// Set matrix to scale.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_scale(matrix: *mut sk_matrix_t, sx: f32, sy: f32) {
    if let Some(m) = matrix.as_mut() {
        *m = Matrix::scale(sx, sy).into();
    }
}

/// Set matrix to rotate (degrees).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_set_rotate(matrix: *mut sk_matrix_t, degrees: f32) {
    if let Some(m) = matrix.as_mut() {
        let radians = degrees * std::f32::consts::PI / 180.0;
        *m = Matrix::rotate(radians).into();
    }
}

/// Concatenate two matrices.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_concat(
    result: *mut sk_matrix_t,
    a: *const sk_matrix_t,
    b: *const sk_matrix_t,
) {
    if let (Some(r), Some(a), Some(b)) = (result.as_mut(), a.as_ref(), b.as_ref()) {
        let ma: Matrix = (*a).into();
        let mb: Matrix = (*b).into();
        *r = ma.concat(&mb).into();
    }
}

/// Map a point through a matrix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_matrix_map_point(
    matrix: *const sk_matrix_t,
    point: *const sk_point_t,
    result: *mut sk_point_t,
) {
    if let (Some(m), Some(p), Some(r)) = (matrix.as_ref(), point.as_ref(), result.as_mut()) {
        let mat: Matrix = (*m).into();
        let pt: Point = (*p).into();
        *r = mat.map_point(pt).into();
    }
}

// =============================================================================
// Utility functions
// =============================================================================

/// Get the library version.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_version() -> *const c_char {
    static VERSION: &[u8] = b"skia-rs 0.1.0\0";
    VERSION.as_ptr() as *const c_char
}

/// Check if the library is available.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_is_available() -> bool {
    true
}

// =============================================================================
// Drawing helpers (simplified)
// =============================================================================

/// Clear a surface with a color.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_clear(surface: *mut SkSurface, color: sk_color_t) {
    if let Some(s) = surface.as_mut() {
        let mut canvas = s.raster_canvas();
        canvas.clear(Color(color));
    }
}

/// Draw a rect on a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_draw_rect(
    surface: *mut SkSurface,
    rect: *const sk_rect_t,
    paint: *const SkPaint,
) {
    if let (Some(s), Some(r), Some(p)) = (surface.as_mut(), rect.as_ref(), paint.as_ref()) {
        let mut canvas = s.raster_canvas();
        canvas.draw_rect(&Rect::from(*r), p);
    }
}

/// Draw a circle on a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_draw_circle(
    surface: *mut SkSurface,
    cx: f32,
    cy: f32,
    radius: f32,
    paint: *const SkPaint,
) {
    if let (Some(s), Some(p)) = (surface.as_mut(), paint.as_ref()) {
        let mut canvas = s.raster_canvas();
        canvas.draw_circle(Point::new(cx, cy), radius, p);
    }
}

/// Draw a path on a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_draw_path(
    surface: *mut SkSurface,
    path: *const SkPath,
    paint: *const SkPaint,
) {
    if let (Some(s), Some(path), Some(p)) = (surface.as_mut(), path.as_ref(), paint.as_ref()) {
        let mut canvas = s.raster_canvas();
        canvas.draw_path(path, p);
    }
}

/// Draw a line on a surface.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sk_surface_draw_line(
    surface: *mut SkSurface,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    paint: *const SkPaint,
) {
    if let (Some(s), Some(p)) = (surface.as_mut(), paint.as_ref()) {
        let mut canvas = s.raster_canvas();
        canvas.draw_line(Point::new(x0, y0), Point::new(x1, y1), p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_creation() {
        unsafe {
            let surface = sk_surface_new_raster(100, 100);
            assert!(!surface.is_null());
            assert_eq!(sk_surface_get_width(surface), 100);
            assert_eq!(sk_surface_get_height(surface), 100);
            sk_surface_unref(surface);
        }
    }

    #[test]
    fn test_paint_operations() {
        unsafe {
            let paint = sk_paint_new();
            assert!(!paint.is_null());

            sk_paint_set_color(paint, 0xFF0000FF); // Blue
            assert_eq!(sk_paint_get_color(paint), 0xFF0000FF);

            sk_paint_set_stroke_width(paint, 2.0);
            assert_eq!(sk_paint_get_stroke_width(paint), 2.0);

            sk_paint_delete(paint);
        }
    }

    #[test]
    fn test_path_builder() {
        unsafe {
            let builder = sk_pathbuilder_new();
            assert!(!builder.is_null());

            sk_pathbuilder_move_to(builder, 0.0, 0.0);
            sk_pathbuilder_line_to(builder, 100.0, 0.0);
            sk_pathbuilder_line_to(builder, 100.0, 100.0);
            sk_pathbuilder_close(builder);

            let path = sk_pathbuilder_detach(builder);
            assert!(!path.is_null());
            assert!(!sk_path_is_empty(path));

            let mut bounds = sk_rect_t::default();
            sk_path_get_bounds(path, &mut bounds);
            assert_eq!(bounds.left, 0.0);
            assert_eq!(bounds.right, 100.0);

            sk_path_delete(path);
        }
    }

    #[test]
    fn test_matrix_operations() {
        unsafe {
            let mut matrix = sk_matrix_t::default();
            sk_matrix_set_translate(&mut matrix, 10.0, 20.0);

            let point = sk_point_t { x: 0.0, y: 0.0 };
            let mut result = sk_point_t::default();
            sk_matrix_map_point(&matrix, &point, &mut result);

            assert_eq!(result.x, 10.0);
            assert_eq!(result.y, 20.0);
        }
    }

    #[test]
    fn test_draw_rect() {
        unsafe {
            let surface = sk_surface_new_raster(100, 100);
            let paint = sk_paint_new();
            sk_paint_set_color(paint, 0xFFFF0000); // Red

            sk_surface_clear(surface, 0xFFFFFFFF); // White

            let rect = sk_rect_t {
                left: 10.0,
                top: 10.0,
                right: 50.0,
                bottom: 50.0,
            };
            sk_surface_draw_rect(surface, &rect, paint);

            sk_paint_delete(paint);
            sk_surface_unref(surface);
        }
    }
}
