//! Node.js bindings for skia-rs.
//!
//! This crate provides Node.js bindings using napi-rs.
//!
//! # Installation
//!
//! ```bash
//! npm install @skia-rs/node
//! # or build from source:
//! npm run build
//! ```
//!
//! # Example
//!
//! ```javascript
//! const skia = require('@skia-rs/node');
//!
//! // Create a surface
//! const surface = new skia.Surface(800, 600);
//!
//! // Create a paint
//! const paint = new skia.Paint();
//! paint.setColor(0xFFFF0000); // Red
//! paint.setAntiAlias(true);
//!
//! // Draw
//! surface.clear(0xFFFFFFFF); // White
//! surface.drawCircle(400, 300, 100, paint);
//!
//! // Get pixel data
//! const pixels = surface.getPixels(); // Buffer
//! ```

#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;

use skia_rs_canvas::Surface as RsSurface;
use skia_rs_core::cast::f32_to_u8_sat;
use skia_rs_core::{Color, Matrix as RsMatrix, Point as RsPoint, Rect as RsRect};
use skia_rs_paint::{Paint as RsPaint, Style as RsStyle};
use skia_rs_path::{Path as RsPath, PathBuilder as RsPathBuilder};

/// Narrow a JS `f64` argument (all JS numbers are `f64`) to Skia's `f32`
/// scalar type at the napi FFI boundary.
///
/// There is no lossless `f64` -> `f32` conversion in std, and every geometry
/// entry point in this crate narrows in exactly this way, so the cast is
/// centralized here instead of being repeated (and re-justified) at each
/// call site.
#[inline]
#[must_use]
const fn to_f32(x: f64) -> f32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "f64 (JS number) -> f32 (Skia scalar) narrowing is inherent to the napi <-> Skia boundary; no lossless std conversion exists"
    )]
    {
        x as f32
    }
}

/// Widen a Skia `f32` scalar to the `f64` JS number type returned across the
/// napi FFI boundary.
///
/// This is a lossless widening, but `From<f32> for f64` is not yet
/// const-stable, so the equivalent `as` cast (also lossless here) is
/// centralized in this helper instead of `f64::from`, so callers that need a
/// `const fn` keep compiling.
#[inline]
#[must_use]
const fn to_f64(x: f32) -> f64 {
    #[allow(
        clippy::cast_lossless,
        reason = "f32 -> f64 widening is lossless; `f64::from` is not yet const-stable, so this helper centralizes the equivalent `as` cast"
    )]
    {
        x as f64
    }
}

// =============================================================================
// Point
// =============================================================================

/// A 2D point with x and y coordinates.
#[napi]
pub struct Point {
    x: f64,
    y: f64,
}

#[napi]
impl Point {
    /// Create a new point.
    #[napi(constructor)]
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Get X coordinate.
    #[napi(getter)]
    #[must_use]
    pub const fn x(&self) -> f64 {
        self.x
    }

    /// Set X coordinate.
    #[napi(setter)]
    pub const fn set_x(&mut self, x: f64) {
        self.x = x;
    }

    /// Get Y coordinate.
    #[napi(getter)]
    #[must_use]
    pub const fn y(&self) -> f64 {
        self.y
    }

    /// Set Y coordinate.
    #[napi(setter)]
    pub const fn set_y(&mut self, y: f64) {
        self.y = y;
    }

    /// Calculate the length of the vector from origin.
    #[napi]
    #[must_use]
    pub fn length(&self) -> f64 {
        self.x.hypot(self.y)
    }

    /// Normalize the point (unit vector).
    #[napi]
    #[must_use]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len == 0.0 {
            Self::new(0.0, 0.0)
        } else {
            Self::new(self.x / len, self.y / len)
        }
    }

    /// Add two points.
    #[napi]
    #[must_use]
    #[allow(
        clippy::use_self,
        reason = "napi-derive's #[napi] macro cannot resolve `Self` in a by-reference parameter type in the expanded function signature; `Point` must be spelled out here"
    )]
    pub fn add(&self, other: &Point) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    /// Subtract two points.
    #[napi]
    #[must_use]
    #[allow(
        clippy::use_self,
        reason = "napi-derive's #[napi] macro cannot resolve `Self` in a by-reference parameter type in the expanded function signature; `Point` must be spelled out here"
    )]
    pub fn sub(&self, other: &Point) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    /// Multiply by scalar.
    #[napi]
    #[must_use]
    pub fn mul(&self, scalar: f64) -> Self {
        Self::new(self.x * scalar, self.y * scalar)
    }
}

// =============================================================================
// Rect
// =============================================================================

/// A rectangle defined by left, top, right, bottom edges.
#[napi]
pub struct Rect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

#[napi]
impl Rect {
    /// Create a new rectangle from edges.
    #[napi(constructor)]
    #[must_use]
    pub const fn new(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Create a rectangle from position and size.
    #[napi(factory)]
    #[must_use]
    pub fn from_xywh(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }

    /// Create a rectangle from size (at origin).
    #[napi(factory)]
    #[must_use]
    pub const fn from_wh(width: f64, height: f64) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: width,
            bottom: height,
        }
    }

    #[napi(getter)]
    #[must_use]
    pub const fn left(&self) -> f64 {
        self.left
    }

    #[napi(getter)]
    #[must_use]
    pub const fn top(&self) -> f64 {
        self.top
    }

    #[napi(getter)]
    #[must_use]
    pub const fn right(&self) -> f64 {
        self.right
    }

    #[napi(getter)]
    #[must_use]
    pub const fn bottom(&self) -> f64 {
        self.bottom
    }

    #[napi(getter)]
    #[must_use]
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    #[napi(getter)]
    #[must_use]
    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }

    /// Check if the rectangle is empty.
    #[napi]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    /// Check if a point is inside the rectangle.
    #[napi]
    #[must_use]
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    /// Get the center of the rectangle.
    #[napi]
    #[must_use]
    pub const fn center(&self) -> Point {
        Point::new(
            f64::midpoint(self.left, self.right),
            f64::midpoint(self.top, self.bottom),
        )
    }
}

// =============================================================================
// Matrix
// =============================================================================

/// A 3x3 transformation matrix.
#[napi]
pub struct Matrix {
    inner: RsMatrix,
}

impl Default for Matrix {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl Matrix {
    /// Create an identity matrix.
    #[napi(constructor)]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: RsMatrix::IDENTITY,
        }
    }

    /// Create a translation matrix.
    #[napi(factory)]
    #[must_use]
    pub const fn translate(dx: f64, dy: f64) -> Self {
        Self {
            inner: RsMatrix::translate(to_f32(dx), to_f32(dy)),
        }
    }

    /// Create a scale matrix.
    #[napi(factory)]
    #[must_use]
    pub const fn scale(sx: f64, sy: f64) -> Self {
        Self {
            inner: RsMatrix::scale(to_f32(sx), to_f32(sy)),
        }
    }

    /// Create a rotation matrix (radians).
    #[napi(factory)]
    #[must_use]
    pub fn rotate(radians: f64) -> Self {
        Self {
            inner: RsMatrix::rotate(to_f32(radians)),
        }
    }

    /// Create a rotation matrix (degrees).
    #[napi(factory)]
    #[must_use]
    pub fn rotate_deg(degrees: f64) -> Self {
        let radians = degrees.to_radians();
        Self {
            inner: RsMatrix::rotate(to_f32(radians)),
        }
    }

    /// Concatenate with another matrix.
    #[napi]
    #[must_use]
    #[allow(
        clippy::use_self,
        reason = "napi-derive's #[napi] macro cannot resolve `Self` in a by-reference parameter type in the expanded function signature; `Matrix` must be spelled out here"
    )]
    pub fn concat(&self, other: &Matrix) -> Self {
        Self {
            inner: self.inner.concat(&other.inner),
        }
    }

    /// Invert the matrix.
    #[napi]
    #[must_use]
    #[allow(
        clippy::use_self,
        reason = "napi-derive's #[napi] macro cannot resolve `Self` inside a generic return type (`Option<Self>`) in the expanded function signature; `Matrix` must be spelled out here"
    )]
    pub fn invert(&self) -> Option<Matrix> {
        self.inner.invert().map(|inner| Matrix { inner })
    }

    /// Transform a point.
    #[napi]
    #[must_use]
    pub fn map_point(&self, x: f64, y: f64) -> Point {
        let p = self.inner.map_point(RsPoint::new(to_f32(x), to_f32(y)));
        Point::new(to_f64(p.x), to_f64(p.y))
    }

    /// Get matrix values as array.
    #[napi]
    #[must_use]
    pub fn get_values(&self) -> Vec<f64> {
        self.inner.values.iter().map(|&v| to_f64(v)).collect()
    }
}

// =============================================================================
// Paint
// =============================================================================

/// Paint controls styling for drawing operations.
#[napi]
pub struct Paint {
    inner: RsPaint,
}

impl Default for Paint {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl Paint {
    /// Create a new paint with default settings.
    #[napi(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RsPaint::new(),
        }
    }

    /// Get color as ARGB integer.
    #[napi]
    #[must_use]
    pub fn get_color(&self) -> u32 {
        self.inner.color32().0
    }

    /// Set color as ARGB integer.
    #[napi]
    pub fn set_color(&mut self, color: u32) {
        self.inner.set_color32(Color(color));
    }

    /// Set color from ARGB components (0-255).
    #[napi]
    pub fn set_argb(&mut self, a: u8, r: u8, g: u8, b: u8) {
        self.inner.set_color32(Color::from_argb(a, r, g, b));
    }

    /// Get style: 0=fill, 1=stroke, `2=stroke_and_fill`.
    #[napi]
    #[must_use]
    pub const fn get_style(&self) -> u32 {
        match self.inner.style() {
            RsStyle::Fill => 0,
            RsStyle::Stroke => 1,
            RsStyle::StrokeAndFill => 2,
        }
    }

    /// Set style: 0=fill, 1=stroke, `2=stroke_and_fill`.
    #[napi]
    pub const fn set_style(&mut self, style: u32) {
        let s = match style {
            0 => RsStyle::Fill,
            1 => RsStyle::Stroke,
            _ => RsStyle::StrokeAndFill,
        };
        self.inner.set_style(s);
    }

    /// Get stroke width.
    #[napi]
    #[must_use]
    pub const fn get_stroke_width(&self) -> f64 {
        to_f64(self.inner.stroke_width())
    }

    /// Set stroke width.
    #[napi]
    pub const fn set_stroke_width(&mut self, width: f64) {
        self.inner.set_stroke_width(to_f32(width));
    }

    /// Get anti-aliasing state.
    #[napi]
    #[must_use]
    pub const fn get_anti_alias(&self) -> bool {
        self.inner.is_anti_alias()
    }

    /// Set anti-aliasing.
    #[napi]
    pub const fn set_anti_alias(&mut self, aa: bool) {
        self.inner.set_anti_alias(aa);
    }

    /// Get alpha (0-255).
    #[napi]
    #[must_use]
    pub fn get_alpha(&self) -> u32 {
        u32::from(f32_to_u8_sat(self.inner.alpha() * 255.0))
    }

    /// Set alpha (0-255).
    #[napi]
    pub fn set_alpha(&mut self, alpha: u32) {
        let clamped = u8::try_from(alpha.min(255)).unwrap_or(u8::MAX);
        self.inner.set_alpha(f32::from(clamped) / 255.0);
    }
}

#[cfg(test)]
mod tests {
    use super::f32_to_u8_sat;

    /// Mirrors the arithmetic in `Paint::set_alpha` / `Paint::get_alpha`
    /// without requiring a napi runtime.
    fn set_alpha_arith(alpha: u32) -> f32 {
        let clamped = u8::try_from(alpha.min(255)).unwrap_or(u8::MAX);
        f32::from(clamped) / 255.0
    }

    fn get_alpha_arith(a: f32) -> u32 {
        u32::from(f32_to_u8_sat(a * 255.0))
    }

    #[test]
    fn set_alpha_round_trip() {
        assert_eq!(get_alpha_arith(set_alpha_arith(128)), 128);
        assert_eq!(get_alpha_arith(set_alpha_arith(0)), 0);
        assert_eq!(get_alpha_arith(set_alpha_arith(255)), 255);
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "exact round-trip through a saturating u8 byte is guaranteed to reproduce exactly 1.0; this is the behavior under test, not a tolerance-worthy comparison"
    )]
    fn set_alpha_clamps_above_255() {
        // Without the clamp, 300 / 255.0 ~= 1.176, which is > 1.0.
        let clamped = set_alpha_arith(300);
        assert_eq!(clamped, 1.0);
        assert_eq!(get_alpha_arith(clamped), 255);
    }
}

// =============================================================================
// PathBuilder
// =============================================================================

/// Builder for constructing paths.
#[napi]
pub struct PathBuilder {
    inner: RsPathBuilder,
}

impl Default for PathBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl PathBuilder {
    /// Create a new path builder.
    #[napi(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RsPathBuilder::new(),
        }
    }

    /// Move to a point.
    #[napi]
    pub fn move_to(&mut self, x: f64, y: f64) -> &Self {
        self.inner.move_to(to_f32(x), to_f32(y));
        self
    }

    /// Line to a point.
    #[napi]
    pub fn line_to(&mut self, x: f64, y: f64) -> &Self {
        self.inner.line_to(to_f32(x), to_f32(y));
        self
    }

    /// Quadratic bezier curve.
    #[napi]
    pub fn quad_to(&mut self, cx: f64, cy: f64, x: f64, y: f64) -> &Self {
        self.inner
            .quad_to(to_f32(cx), to_f32(cy), to_f32(x), to_f32(y));
        self
    }

    /// Cubic bezier curve.
    #[napi]
    pub fn cubic_to(&mut self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64) -> &Self {
        self.inner.cubic_to(
            to_f32(c1x),
            to_f32(c1y),
            to_f32(c2x),
            to_f32(c2y),
            to_f32(x),
            to_f32(y),
        );
        self
    }

    /// Close the current contour.
    #[napi]
    pub fn close(&mut self) -> &Self {
        self.inner.close();
        self
    }

    /// Add a rectangle.
    #[napi]
    pub fn add_rect(&mut self, left: f64, top: f64, right: f64, bottom: f64) -> &Self {
        self.inner.add_rect(&RsRect::new(
            to_f32(left),
            to_f32(top),
            to_f32(right),
            to_f32(bottom),
        ));
        self
    }

    /// Add an oval inscribed in a rectangle.
    #[napi]
    pub fn add_oval(&mut self, left: f64, top: f64, right: f64, bottom: f64) -> &Self {
        self.inner.add_oval(&RsRect::new(
            to_f32(left),
            to_f32(top),
            to_f32(right),
            to_f32(bottom),
        ));
        self
    }

    /// Add a circle.
    #[napi]
    pub fn add_circle(&mut self, cx: f64, cy: f64, radius: f64) -> &Self {
        self.inner
            .add_circle(to_f32(cx), to_f32(cy), to_f32(radius));
        self
    }

    /// Add a rounded rectangle.
    #[napi]
    pub fn add_round_rect(
        &mut self,
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        rx: f64,
        ry: f64,
    ) -> &Self {
        self.inner.add_round_rect(
            &RsRect::new(to_f32(left), to_f32(top), to_f32(right), to_f32(bottom)),
            to_f32(rx),
            to_f32(ry),
        );
        self
    }

    /// Build the path.
    #[napi]
    #[must_use]
    pub fn build(&self) -> Path {
        Path {
            inner: self.inner.clone().build(),
        }
    }

    /// Reset the builder.
    #[napi]
    pub fn reset(&mut self) {
        self.inner = RsPathBuilder::new();
    }
}

// =============================================================================
// Path
// =============================================================================

/// An immutable path containing geometry.
#[napi]
pub struct Path {
    inner: RsPath,
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl Path {
    /// Create an empty path.
    #[napi(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RsPath::new(),
        }
    }

    /// Check if the path is empty.
    #[napi]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the bounding box.
    #[napi]
    #[must_use]
    pub fn get_bounds(&self) -> Rect {
        let b = self.inner.bounds();
        Rect::new(to_f64(b.left), to_f64(b.top), to_f64(b.right), to_f64(b.bottom))
    }

    /// Check if a point is inside the path.
    #[napi]
    #[must_use]
    pub fn contains(&self, x: f64, y: f64) -> bool {
        self.inner.contains(RsPoint::new(to_f32(x), to_f32(y)))
    }
}

// =============================================================================
// Surface
// =============================================================================

/// A drawing surface backed by pixels.
#[napi]
pub struct Surface {
    inner: RsSurface,
}

#[napi]
impl Surface {
    /// Create a new raster surface.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying raster surface could not be
    /// allocated (e.g. invalid or excessive dimensions).
    #[napi(constructor)]
    pub fn new(width: i32, height: i32) -> Result<Self> {
        RsSurface::new_raster_n32_premul(width, height)
            .map(|s| Self { inner: s })
            .ok_or_else(|| Error::from_reason("Failed to create surface"))
    }

    /// Width in pixels.
    #[napi(getter)]
    #[must_use]
    pub const fn width(&self) -> i32 {
        self.inner.width()
    }

    /// Height in pixels.
    #[napi(getter)]
    #[must_use]
    pub const fn height(&self) -> i32 {
        self.inner.height()
    }

    /// Clear the surface with a color.
    #[napi]
    pub fn clear(&mut self, color: u32) {
        let mut canvas = self.inner.raster_canvas();
        canvas.clear(Color(color));
    }

    /// Draw a rectangle.
    #[napi]
    pub fn draw_rect(&mut self, left: f64, top: f64, right: f64, bottom: f64, paint: &Paint) {
        let mut canvas = self.inner.raster_canvas();
        canvas.draw_rect(
            &RsRect::new(to_f32(left), to_f32(top), to_f32(right), to_f32(bottom)),
            &paint.inner,
        );
    }

    /// Draw a circle.
    #[napi]
    pub fn draw_circle(&mut self, cx: f64, cy: f64, radius: f64, paint: &Paint) {
        let mut canvas = self.inner.raster_canvas();
        canvas.draw_circle(
            RsPoint::new(to_f32(cx), to_f32(cy)),
            to_f32(radius),
            &paint.inner,
        );
    }

    /// Draw an oval inscribed in a rectangle.
    #[napi]
    pub fn draw_oval(&mut self, left: f64, top: f64, right: f64, bottom: f64, paint: &Paint) {
        let mut canvas = self.inner.raster_canvas();
        canvas.draw_oval(
            &RsRect::new(to_f32(left), to_f32(top), to_f32(right), to_f32(bottom)),
            &paint.inner,
        );
    }

    /// Draw a line.
    #[napi]
    pub fn draw_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, paint: &Paint) {
        let mut canvas = self.inner.raster_canvas();
        canvas.draw_line(
            RsPoint::new(to_f32(x0), to_f32(y0)),
            RsPoint::new(to_f32(x1), to_f32(y1)),
            &paint.inner,
        );
    }

    /// Draw a path.
    #[napi]
    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        let mut canvas = self.inner.raster_canvas();
        canvas.draw_path(&path.inner, &paint.inner);
    }

    /// Draw a point.
    #[napi]
    pub fn draw_point(&mut self, x: f64, y: f64, paint: &Paint) {
        let mut canvas = self.inner.raster_canvas();
        canvas.draw_point(RsPoint::new(to_f32(x), to_f32(y)), &paint.inner);
    }

    /// Get pixel data as Buffer (RGBA).
    ///
    /// The surface stores premultiplied RGBA8888 pixels internally; this
    /// unpremultiplies before returning so callers see straight-alpha RGBA,
    /// as documented.
    #[napi]
    #[must_use]
    pub fn get_pixels(&self) -> Buffer {
        let mut pixels = self.inner.pixels().to_vec();
        skia_rs_canvas::simd::unpremultiply_span(&mut pixels);
        Buffer::from(pixels)
    }

    /// Get row bytes.
    #[napi]
    #[must_use]
    pub fn get_row_bytes(&self) -> u32 {
        u32::try_from(self.inner.row_bytes()).unwrap_or(u32::MAX)
    }
}

// =============================================================================
// Color utilities
// =============================================================================

/// Create an ARGB color value.
#[napi]
#[must_use]
pub const fn argb(a: u8, r: u8, g: u8, b: u8) -> u32 {
    Color::from_argb(a, r, g, b).0
}

/// Create an RGB color value (fully opaque).
#[napi]
#[must_use]
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    Color::from_rgb(r, g, b).0
}

/// Predefined colors.
pub mod colors {
    use super::napi;

    #[napi]
    pub const BLACK: u32 = 0xFF00_0000;
    #[napi]
    pub const WHITE: u32 = 0xFFFF_FFFF;
    #[napi]
    pub const RED: u32 = 0xFFFF_0000;
    #[napi]
    pub const GREEN: u32 = 0xFF00_FF00;
    #[napi]
    pub const BLUE: u32 = 0xFF00_00FF;
    #[napi]
    pub const YELLOW: u32 = 0xFFFF_FF00;
    #[napi]
    pub const CYAN: u32 = 0xFF00_FFFF;
    #[napi]
    pub const MAGENTA: u32 = 0xFFFF_00FF;
    #[napi]
    pub const TRANSPARENT: u32 = 0x0000_0000;
}
