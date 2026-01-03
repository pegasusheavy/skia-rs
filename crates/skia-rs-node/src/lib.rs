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
use skia_rs_core::{Color, Matrix as RsMatrix, Point as RsPoint, Rect as RsRect};
use skia_rs_paint::{Paint as RsPaint, Style as RsStyle};
use skia_rs_path::{Path as RsPath, PathBuilder as RsPathBuilder};

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
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Get X coordinate.
    #[napi(getter)]
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Set X coordinate.
    #[napi(setter)]
    pub fn set_x(&mut self, x: f64) {
        self.x = x;
    }

    /// Get Y coordinate.
    #[napi(getter)]
    pub fn y(&self) -> f64 {
        self.y
    }

    /// Set Y coordinate.
    #[napi(setter)]
    pub fn set_y(&mut self, y: f64) {
        self.y = y;
    }

    /// Calculate the length of the vector from origin.
    #[napi]
    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Normalize the point (unit vector).
    #[napi]
    pub fn normalize(&self) -> Point {
        let len = self.length();
        if len == 0.0 {
            Point::new(0.0, 0.0)
        } else {
            Point::new(self.x / len, self.y / len)
        }
    }

    /// Add two points.
    #[napi]
    pub fn add(&self, other: &Point) -> Point {
        Point::new(self.x + other.x, self.y + other.y)
    }

    /// Subtract two points.
    #[napi]
    pub fn sub(&self, other: &Point) -> Point {
        Point::new(self.x - other.x, self.y - other.y)
    }

    /// Multiply by scalar.
    #[napi]
    pub fn mul(&self, scalar: f64) -> Point {
        Point::new(self.x * scalar, self.y * scalar)
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
    pub fn new(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Create a rectangle from position and size.
    #[napi(factory)]
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
    pub fn from_wh(width: f64, height: f64) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: width,
            bottom: height,
        }
    }

    #[napi(getter)]
    pub fn left(&self) -> f64 {
        self.left
    }

    #[napi(getter)]
    pub fn top(&self) -> f64 {
        self.top
    }

    #[napi(getter)]
    pub fn right(&self) -> f64 {
        self.right
    }

    #[napi(getter)]
    pub fn bottom(&self) -> f64 {
        self.bottom
    }

    #[napi(getter)]
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    #[napi(getter)]
    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }

    /// Check if the rectangle is empty.
    #[napi]
    pub fn is_empty(&self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    /// Check if a point is inside the rectangle.
    #[napi]
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    /// Get the center of the rectangle.
    #[napi]
    pub fn center(&self) -> Point {
        Point::new(
            (self.left + self.right) / 2.0,
            (self.top + self.bottom) / 2.0,
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

#[napi]
impl Matrix {
    /// Create an identity matrix.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RsMatrix::IDENTITY,
        }
    }

    /// Create a translation matrix.
    #[napi(factory)]
    pub fn translate(dx: f64, dy: f64) -> Self {
        Self {
            inner: RsMatrix::translate(dx as f32, dy as f32),
        }
    }

    /// Create a scale matrix.
    #[napi(factory)]
    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            inner: RsMatrix::scale(sx as f32, sy as f32),
        }
    }

    /// Create a rotation matrix (radians).
    #[napi(factory)]
    pub fn rotate(radians: f64) -> Self {
        Self {
            inner: RsMatrix::rotate(radians as f32),
        }
    }

    /// Create a rotation matrix (degrees).
    #[napi(factory)]
    pub fn rotate_deg(degrees: f64) -> Self {
        let radians = degrees * std::f64::consts::PI / 180.0;
        Self {
            inner: RsMatrix::rotate(radians as f32),
        }
    }

    /// Concatenate with another matrix.
    #[napi]
    pub fn concat(&self, other: &Matrix) -> Matrix {
        Matrix {
            inner: self.inner.concat(&other.inner),
        }
    }

    /// Invert the matrix.
    #[napi]
    pub fn invert(&self) -> Option<Matrix> {
        self.inner.invert().map(|m| Matrix { inner: m })
    }

    /// Transform a point.
    #[napi]
    pub fn map_point(&self, x: f64, y: f64) -> Point {
        let p = self.inner.map_point(RsPoint::new(x as f32, y as f32));
        Point::new(p.x as f64, p.y as f64)
    }

    /// Get matrix values as array.
    #[napi]
    pub fn get_values(&self) -> Vec<f64> {
        self.inner.values.iter().map(|&v| v as f64).collect()
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

#[napi]
impl Paint {
    /// Create a new paint with default settings.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RsPaint::new(),
        }
    }

    /// Get color as ARGB integer.
    #[napi]
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
    pub fn set_argb(&mut self, a: u32, r: u32, g: u32, b: u32) {
        self.inner
            .set_color32(Color::from_argb(a as u8, r as u8, g as u8, b as u8));
    }

    /// Get style: 0=fill, 1=stroke, 2=stroke_and_fill.
    #[napi]
    pub fn get_style(&self) -> u32 {
        match self.inner.style() {
            RsStyle::Fill => 0,
            RsStyle::Stroke => 1,
            RsStyle::StrokeAndFill => 2,
        }
    }

    /// Set style: 0=fill, 1=stroke, 2=stroke_and_fill.
    #[napi]
    pub fn set_style(&mut self, style: u32) {
        let s = match style {
            0 => RsStyle::Fill,
            1 => RsStyle::Stroke,
            _ => RsStyle::StrokeAndFill,
        };
        self.inner.set_style(s);
    }

    /// Get stroke width.
    #[napi]
    pub fn get_stroke_width(&self) -> f64 {
        self.inner.stroke_width() as f64
    }

    /// Set stroke width.
    #[napi]
    pub fn set_stroke_width(&mut self, width: f64) {
        self.inner.set_stroke_width(width as f32);
    }

    /// Get anti-aliasing state.
    #[napi]
    pub fn get_anti_alias(&self) -> bool {
        self.inner.is_anti_alias()
    }

    /// Set anti-aliasing.
    #[napi]
    pub fn set_anti_alias(&mut self, aa: bool) {
        self.inner.set_anti_alias(aa);
    }

    /// Get alpha (0-255).
    #[napi]
    pub fn get_alpha(&self) -> u32 {
        self.inner.alpha() as u32
    }

    /// Set alpha (0-255).
    #[napi]
    pub fn set_alpha(&mut self, alpha: u32) {
        self.inner.set_alpha(alpha as u8);
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

#[napi]
impl PathBuilder {
    /// Create a new path builder.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RsPathBuilder::new(),
        }
    }

    /// Move to a point.
    #[napi]
    pub fn move_to(&mut self, x: f64, y: f64) -> &Self {
        self.inner.move_to(x as f32, y as f32);
        self
    }

    /// Line to a point.
    #[napi]
    pub fn line_to(&mut self, x: f64, y: f64) -> &Self {
        self.inner.line_to(x as f32, y as f32);
        self
    }

    /// Quadratic bezier curve.
    #[napi]
    pub fn quad_to(&mut self, cx: f64, cy: f64, x: f64, y: f64) -> &Self {
        self.inner.quad_to(cx as f32, cy as f32, x as f32, y as f32);
        self
    }

    /// Cubic bezier curve.
    #[napi]
    pub fn cubic_to(
        &mut self,
        c1x: f64,
        c1y: f64,
        c2x: f64,
        c2y: f64,
        x: f64,
        y: f64,
    ) -> &Self {
        self.inner.cubic_to(
            c1x as f32,
            c1y as f32,
            c2x as f32,
            c2y as f32,
            x as f32,
            y as f32,
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
            left as f32,
            top as f32,
            right as f32,
            bottom as f32,
        ));
        self
    }

    /// Add an oval inscribed in a rectangle.
    #[napi]
    pub fn add_oval(&mut self, left: f64, top: f64, right: f64, bottom: f64) -> &Self {
        self.inner.add_oval(&RsRect::new(
            left as f32,
            top as f32,
            right as f32,
            bottom as f32,
        ));
        self
    }

    /// Add a circle.
    #[napi]
    pub fn add_circle(&mut self, cx: f64, cy: f64, radius: f64) -> &Self {
        self.inner.add_circle(cx as f32, cy as f32, radius as f32);
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
            &RsRect::new(left as f32, top as f32, right as f32, bottom as f32),
            rx as f32,
            ry as f32,
        );
        self
    }

    /// Build the path.
    #[napi]
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

#[napi]
impl Path {
    /// Create an empty path.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RsPath::new(),
        }
    }

    /// Check if the path is empty.
    #[napi]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the bounding box.
    #[napi]
    pub fn get_bounds(&self) -> Rect {
        let b = self.inner.bounds();
        Rect::new(b.left as f64, b.top as f64, b.right as f64, b.bottom as f64)
    }

    /// Check if a point is inside the path.
    #[napi]
    pub fn contains(&self, x: f64, y: f64) -> bool {
        self.inner.contains(RsPoint::new(x as f32, y as f32))
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
    #[napi(constructor)]
    pub fn new(width: i32, height: i32) -> Result<Self> {
        RsSurface::new_raster_n32_premul(width, height)
            .map(|s| Self { inner: s })
            .ok_or_else(|| Error::from_reason("Failed to create surface"))
    }

    /// Width in pixels.
    #[napi(getter)]
    pub fn width(&self) -> i32 {
        self.inner.width()
    }

    /// Height in pixels.
    #[napi(getter)]
    pub fn height(&self) -> i32 {
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
            &RsRect::new(left as f32, top as f32, right as f32, bottom as f32),
            &paint.inner,
        );
    }

    /// Draw a circle.
    #[napi]
    pub fn draw_circle(&mut self, cx: f64, cy: f64, radius: f64, paint: &Paint) {
        let mut canvas = self.inner.raster_canvas();
        canvas.draw_circle(RsPoint::new(cx as f32, cy as f32), radius as f32, &paint.inner);
    }

    /// Draw an oval inscribed in a rectangle.
    #[napi]
    pub fn draw_oval(&mut self, left: f64, top: f64, right: f64, bottom: f64, paint: &Paint) {
        let mut canvas = self.inner.raster_canvas();
        canvas.draw_oval(
            &RsRect::new(left as f32, top as f32, right as f32, bottom as f32),
            &paint.inner,
        );
    }

    /// Draw a line.
    #[napi]
    pub fn draw_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, paint: &Paint) {
        let mut canvas = self.inner.raster_canvas();
        canvas.draw_line(
            RsPoint::new(x0 as f32, y0 as f32),
            RsPoint::new(x1 as f32, y1 as f32),
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
        canvas.draw_point(RsPoint::new(x as f32, y as f32), &paint.inner);
    }

    /// Get pixel data as Buffer (RGBA).
    #[napi]
    pub fn get_pixels(&self) -> Buffer {
        Buffer::from(self.inner.pixels())
    }

    /// Get row bytes.
    #[napi]
    pub fn get_row_bytes(&self) -> u32 {
        self.inner.row_bytes() as u32
    }
}

// =============================================================================
// Color utilities
// =============================================================================

/// Create an ARGB color value.
#[napi]
pub fn argb(a: u32, r: u32, g: u32, b: u32) -> u32 {
    Color::from_argb(a as u8, r as u8, g as u8, b as u8).0
}

/// Create an RGB color value (fully opaque).
#[napi]
pub fn rgb(r: u32, g: u32, b: u32) -> u32 {
    Color::from_rgb(r as u8, g as u8, b as u8).0
}

/// Predefined colors.
pub mod colors {
    use super::*;

    #[napi]
    pub const BLACK: u32 = 0xFF000000;
    #[napi]
    pub const WHITE: u32 = 0xFFFFFFFF;
    #[napi]
    pub const RED: u32 = 0xFFFF0000;
    #[napi]
    pub const GREEN: u32 = 0xFF00FF00;
    #[napi]
    pub const BLUE: u32 = 0xFF0000FF;
    #[napi]
    pub const YELLOW: u32 = 0xFFFFFF00;
    #[napi]
    pub const CYAN: u32 = 0xFF00FFFF;
    #[napi]
    pub const MAGENTA: u32 = 0xFFFF00FF;
    #[napi]
    pub const TRANSPARENT: u32 = 0x00000000;
}
