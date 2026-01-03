//! WebAssembly support for skia-rs.
//!
//! This module provides WASM-specific functionality for running
//! skia-rs in web browsers.
//!
//! # Example
//!
//! ```javascript
//! import init, { WasmSurface } from 'skia-rs';
//!
//! async function main() {
//!     await init();
//!     
//!     const surface = new WasmSurface(800, 600);
//!     surface.clear(0xFFFFFFFF);
//!     surface.draw_circle(400, 300, 100, 0xFFFF0000);
//!     
//!     // Get ImageData for canvas
//!     const imageData = surface.get_image_data();
//!     ctx.putImageData(imageData, 0, 0);
//! }
//! ```

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;
use web_sys::{ImageData, HtmlCanvasElement, CanvasRenderingContext2d};

use crate::prelude::*;

/// WASM-friendly surface wrapper.
#[wasm_bindgen]
pub struct WasmSurface {
    inner: Surface,
}

#[wasm_bindgen]
impl WasmSurface {
    /// Create a new WASM surface.
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Result<WasmSurface, JsValue> {
        let surface = Surface::new_raster_n32_premul(width as i32, height as i32)
            .ok_or_else(|| JsValue::from_str("Failed to create surface"))?;
        Ok(Self { inner: surface })
    }

    /// Get the width.
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.inner.width() as u32
    }

    /// Get the height.
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.inner.height() as u32
    }

    /// Clear the surface with a color (ARGB).
    pub fn clear(&mut self, color: u32) {
        let mut canvas = self.inner.raster_canvas();
        canvas.clear(Color(color));
    }

    /// Draw a rectangle.
    pub fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: u32) {
        let mut canvas = self.inner.raster_canvas();
        let mut paint = Paint::new();
        paint.set_color32(Color(color));
        canvas.draw_rect(&Rect::from_xywh(x, y, width, height), &paint);
    }

    /// Draw a circle.
    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: u32) {
        let mut canvas = self.inner.raster_canvas();
        let mut paint = Paint::new();
        paint.set_color32(Color(color));
        canvas.draw_circle(Point::new(cx, cy), radius, &paint);
    }

    /// Draw a line.
    pub fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: u32, width: f32) {
        let mut canvas = self.inner.raster_canvas();
        let mut paint = Paint::new();
        paint.set_color32(Color(color));
        paint.set_style(Style::Stroke);
        paint.set_stroke_width(width);
        canvas.draw_line(Point::new(x0, y0), Point::new(x1, y1), &paint);
    }

    /// Get pixel data as a Uint8ClampedArray for ImageData.
    pub fn get_pixels(&self) -> Vec<u8> {
        self.inner.pixels().to_vec()
    }

    /// Get as ImageData for direct canvas rendering.
    pub fn get_image_data(&self) -> Result<ImageData, JsValue> {
        let pixels = self.inner.pixels();
        let width = self.inner.width() as u32;
        let height = self.inner.height() as u32;

        // Convert BGRA to RGBA for web
        let mut rgba = pixels.to_vec();
        for chunk in rgba.chunks_exact_mut(4) {
            chunk.swap(0, 2); // Swap B and R
        }

        ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(&rgba),
            width,
            height,
        )
    }

    /// Draw to an HTML canvas element.
    pub fn draw_to_canvas(&self, canvas_id: &str) -> Result<(), JsValue> {
        let document = web_sys::window()
            .ok_or_else(|| JsValue::from_str("No window"))?
            .document()
            .ok_or_else(|| JsValue::from_str("No document"))?;

        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("Canvas not found"))?
            .dyn_into::<HtmlCanvasElement>()?;

        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("No 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;

        let image_data = self.get_image_data()?;
        ctx.put_image_data(&image_data, 0.0, 0.0)?;

        Ok(())
    }
}

/// WASM-friendly paint wrapper.
#[wasm_bindgen]
pub struct WasmPaint {
    inner: Paint,
}

#[wasm_bindgen]
impl WasmPaint {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { inner: Paint::new() }
    }

    pub fn set_color(&mut self, color: u32) {
        self.inner.set_color32(Color(color));
    }

    pub fn set_stroke_width(&mut self, width: f32) {
        self.inner.set_stroke_width(width);
    }

    pub fn set_anti_alias(&mut self, aa: bool) {
        self.inner.set_anti_alias(aa);
    }

    pub fn set_style_fill(&mut self) {
        self.inner.set_style(Style::Fill);
    }

    pub fn set_style_stroke(&mut self) {
        self.inner.set_style(Style::Stroke);
    }
}

/// WASM-friendly path builder.
#[wasm_bindgen]
pub struct WasmPathBuilder {
    inner: PathBuilder,
}

#[wasm_bindgen]
impl WasmPathBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { inner: PathBuilder::new() }
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        self.inner.move_to(x, y);
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        self.inner.line_to(x, y);
    }

    pub fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.inner.quad_to(cx, cy, x, y);
    }

    pub fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        self.inner.cubic_to(c1x, c1y, c2x, c2y, x, y);
    }

    pub fn close(&mut self) {
        self.inner.close();
    }

    pub fn add_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.inner.add_rect(&Rect::from_xywh(x, y, width, height));
    }

    pub fn add_circle(&mut self, cx: f32, cy: f32, radius: f32) {
        self.inner.add_circle(cx, cy, radius);
    }
}

/// Initialize panic hook for better error messages in browser console.
#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Color utilities for WASM.
#[wasm_bindgen]
pub fn argb(a: u8, r: u8, g: u8, b: u8) -> u32 {
    Color::from_argb(a, r, g, b).0
}

#[wasm_bindgen]
pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    Color::from_rgb(r, g, b).0
}

// Re-export common colors
#[wasm_bindgen]
pub fn color_black() -> u32 { 0xFF000000 }
#[wasm_bindgen]
pub fn color_white() -> u32 { 0xFFFFFFFF }
#[wasm_bindgen]
pub fn color_red() -> u32 { 0xFFFF0000 }
#[wasm_bindgen]
pub fn color_green() -> u32 { 0xFF00FF00 }
#[wasm_bindgen]
pub fn color_blue() -> u32 { 0xFF0000FF }
#[wasm_bindgen]
pub fn color_transparent() -> u32 { 0x00000000 }
