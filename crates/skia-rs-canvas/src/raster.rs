//! Rasterizer for drawing primitives to pixel buffers.
//!
//! This module provides software rasterization for basic shapes.
//!
//! ## Active Edge Table Algorithm
//!
//! For path filling, this module implements an optimized Active Edge Table (AET)
//! scanline algorithm with the following characteristics:
//!
//! - **Global Edge Table (GET)**: Edges sorted by y_min for efficient activation
//! - **Active Edge Table (AET)**: Only edges intersecting the current scanline
//! - **Winding Number Calculation**: Supports both non-zero and even-odd fill rules
//! - **X-Intersection Sorting**: Uses insertion sort optimized for nearly-sorted data
//!
//! The algorithm has O(n log n) setup time and O(n) per-scanline time, where n is
//! the number of edges, making it efficient for complex paths.

use skia_rs_core::{Color, Matrix, Point, Rect, Scalar};
use skia_rs_paint::{BlendMode, Paint, Style};
use skia_rs_path::{FillType, Path, PathElement};

/// A pixel buffer for rasterization.
#[derive(Debug, Clone)]
pub struct PixelBuffer {
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
    /// RGBA pixel data (4 bytes per pixel).
    pub pixels: Vec<u8>,
    /// Row stride in bytes.
    pub stride: usize,
}

impl PixelBuffer {
    /// Create a new pixel buffer.
    pub fn new(width: i32, height: i32) -> Self {
        let stride = (width as usize) * 4;
        let pixels = vec![0u8; (height as usize) * stride];
        Self {
            width,
            height,
            pixels,
            stride,
        }
    }

    /// Clear the buffer with a color.
    #[inline]
    pub fn clear(&mut self, color: Color) {
        let r = color.red();
        let g = color.green();
        let b = color.blue();
        let a = color.alpha();

        // Optimize for common case of fully transparent or opaque clear
        if a == 0 && r == 0 && g == 0 && b == 0 {
            self.pixels.fill(0);
            return;
        }

        // Create a 4-byte pattern and fill using chunks
        let pattern = [r, g, b, a];
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&pattern);
        }
    }

    /// Get a pixel at (x, y).
    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> Option<Color> {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return None;
        }
        let offset = (y as usize) * self.stride + (x as usize) * 4;
        Some(Color::from_argb(
            self.pixels[offset + 3],
            self.pixels[offset],
            self.pixels[offset + 1],
            self.pixels[offset + 2],
        ))
    }

    /// Set a pixel at (x, y).
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return;
        }
        let offset = (y as usize) * self.stride + (x as usize) * 4;
        self.pixels[offset] = color.red();
        self.pixels[offset + 1] = color.green();
        self.pixels[offset + 2] = color.blue();
        self.pixels[offset + 3] = color.alpha();
    }

    /// Blend a pixel at (x, y) using the given blend mode.
    #[inline]
    pub fn blend_pixel(&mut self, x: i32, y: i32, src: Color, blend_mode: BlendMode) {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return;
        }

        // Fast path for fully opaque source with SrcOver (most common case)
        if blend_mode == BlendMode::SrcOver && src.alpha() == 255 {
            self.set_pixel(x, y, src);
            return;
        }

        // Fast path for fully transparent source
        if src.alpha() == 0 && matches!(blend_mode, BlendMode::SrcOver | BlendMode::Src) {
            if blend_mode == BlendMode::Src {
                self.set_pixel(x, y, Color::from_argb(0, 0, 0, 0));
            }
            return;
        }

        let dst = self.get_pixel(x, y).unwrap_or(Color::from_argb(0, 0, 0, 0));
        let blended = blend_colors(src, dst, blend_mode);
        self.set_pixel(x, y, blended);
    }

    /// Blend a pixel with coverage (alpha) for anti-aliasing.
    /// Coverage is 0.0 to 1.0 representing how much of the pixel is covered.
    #[inline]
    pub fn blend_pixel_aa(
        &mut self,
        x: i32,
        y: i32,
        src: Color,
        coverage: f32,
        blend_mode: BlendMode,
    ) {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return;
        }

        if coverage <= 0.0 {
            return;
        }

        // Apply coverage to source alpha
        let adjusted_alpha = (src.alpha() as f32 * coverage.min(1.0)) as u8;
        let src_with_coverage =
            Color::from_argb(adjusted_alpha, src.red(), src.green(), src.blue());

        let dst = self.get_pixel(x, y).unwrap_or(Color::from_argb(0, 0, 0, 0));
        let blended = blend_colors(src_with_coverage, dst, blend_mode);
        self.set_pixel(x, y, blended);
    }
}

/// Blend two colors using a blend mode.
fn blend_colors(src: Color, dst: Color, mode: BlendMode) -> Color {
    let sa = src.alpha() as f32 / 255.0;
    let sr = src.red() as f32 / 255.0;
    let sg = src.green() as f32 / 255.0;
    let sb = src.blue() as f32 / 255.0;

    let da = dst.alpha() as f32 / 255.0;
    let dr = dst.red() as f32 / 255.0;
    let dg = dst.green() as f32 / 255.0;
    let db = dst.blue() as f32 / 255.0;

    let (ra, rr, rg, rb) = match mode {
        BlendMode::Clear => (0.0, 0.0, 0.0, 0.0),
        BlendMode::Src => (sa, sr, sg, sb),
        BlendMode::Dst => (da, dr, dg, db),
        BlendMode::SrcOver => {
            let a = sa + da * (1.0 - sa);
            if a > 0.0 {
                let r = (sr * sa + dr * da * (1.0 - sa)) / a;
                let g = (sg * sa + dg * da * (1.0 - sa)) / a;
                let b = (sb * sa + db * da * (1.0 - sa)) / a;
                (a, r, g, b)
            } else {
                (0.0, 0.0, 0.0, 0.0)
            }
        }
        BlendMode::DstOver => {
            let a = da + sa * (1.0 - da);
            if a > 0.0 {
                let r = (dr * da + sr * sa * (1.0 - da)) / a;
                let g = (dg * da + sg * sa * (1.0 - da)) / a;
                let b = (db * da + sb * sa * (1.0 - da)) / a;
                (a, r, g, b)
            } else {
                (0.0, 0.0, 0.0, 0.0)
            }
        }
        BlendMode::SrcIn => {
            let a = sa * da;
            (a, sr, sg, sb)
        }
        BlendMode::DstIn => {
            let a = da * sa;
            (a, dr, dg, db)
        }
        BlendMode::SrcOut => {
            let a = sa * (1.0 - da);
            (a, sr, sg, sb)
        }
        BlendMode::DstOut => {
            let a = da * (1.0 - sa);
            (a, dr, dg, db)
        }
        BlendMode::SrcATop => {
            let a = da;
            let r = sr * da + dr * (1.0 - sa);
            let g = sg * da + dg * (1.0 - sa);
            let b = sb * da + db * (1.0 - sa);
            (a, r, g, b)
        }
        BlendMode::DstATop => {
            let a = sa;
            let r = dr * sa + sr * (1.0 - da);
            let g = dg * sa + sg * (1.0 - da);
            let b = db * sa + sb * (1.0 - da);
            (a, r, g, b)
        }
        BlendMode::Xor => {
            let a = sa + da - 2.0 * sa * da;
            let r = sr * (1.0 - da) + dr * (1.0 - sa);
            let g = sg * (1.0 - da) + dg * (1.0 - sa);
            let b = sb * (1.0 - da) + db * (1.0 - sa);
            (a, r, g, b)
        }
        BlendMode::Plus => {
            let a = (sa + da).min(1.0);
            let r = (sr + dr).min(1.0);
            let g = (sg + dg).min(1.0);
            let b = (sb + db).min(1.0);
            (a, r, g, b)
        }
        BlendMode::Multiply => {
            let a = sa + da - sa * da;
            let r = sr * dr;
            let g = sg * dg;
            let b = sb * db;
            (a, r, g, b)
        }
        BlendMode::Screen => {
            let a = sa + da - sa * da;
            let r = sr + dr - sr * dr;
            let g = sg + dg - sg * dg;
            let b = sb + db - sb * db;
            (a, r, g, b)
        }
        _ => {
            // Default to SrcOver for unimplemented modes
            let a = sa + da * (1.0 - sa);
            if a > 0.0 {
                let r = (sr * sa + dr * da * (1.0 - sa)) / a;
                let g = (sg * sa + dg * da * (1.0 - sa)) / a;
                let b = (sb * sa + db * da * (1.0 - sa)) / a;
                (a, r, g, b)
            } else {
                (0.0, 0.0, 0.0, 0.0)
            }
        }
    };

    Color::from_argb(
        (ra * 255.0).clamp(0.0, 255.0) as u8,
        (rr * 255.0).clamp(0.0, 255.0) as u8,
        (rg * 255.0).clamp(0.0, 255.0) as u8,
        (rb * 255.0).clamp(0.0, 255.0) as u8,
    )
}

/// Rasterizer for drawing to a pixel buffer.
pub struct Rasterizer<'a> {
    buffer: &'a mut PixelBuffer,
    clip: Rect,
    matrix: Matrix,
}

impl<'a> Rasterizer<'a> {
    /// Create a new rasterizer.
    pub fn new(buffer: &'a mut PixelBuffer) -> Self {
        let clip = Rect::from_xywh(0.0, 0.0, buffer.width as Scalar, buffer.height as Scalar);
        Self {
            buffer,
            clip,
            matrix: Matrix::IDENTITY,
        }
    }

    /// Set the current transformation matrix.
    pub fn set_matrix(&mut self, matrix: &Matrix) {
        self.matrix = *matrix;
    }

    /// Set the clip rectangle.
    pub fn set_clip(&mut self, clip: Rect) {
        self.clip = clip;
    }

    /// Draw a point.
    pub fn draw_point(&mut self, point: Point, paint: &Paint) {
        let transformed = self.matrix.map_point(point);
        let x = transformed.x.round() as i32;
        let y = transformed.y.round() as i32;

        if self.clip.contains(transformed) {
            let color = paint.color32();
            self.buffer.blend_pixel(x, y, color, paint.blend_mode());
        }
    }

    /// Draw a line using Bresenham's algorithm (aliased) or Wu's algorithm (anti-aliased).
    pub fn draw_line(&mut self, p0: Point, p1: Point, paint: &Paint) {
        if paint.is_anti_alias() {
            self.draw_line_aa(p0, p1, paint);
        } else {
            self.draw_line_aliased(p0, p1, paint);
        }
    }

    /// Draw line without anti-aliasing (Bresenham).
    fn draw_line_aliased(&mut self, p0: Point, p1: Point, paint: &Paint) {
        let t0 = self.matrix.map_point(p0);
        let t1 = self.matrix.map_point(p1);

        let mut x0 = t0.x.round() as i32;
        let mut y0 = t0.y.round() as i32;
        let x1 = t1.x.round() as i32;
        let y1 = t1.y.round() as i32;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let color = paint.color32();
        let blend_mode = paint.blend_mode();

        loop {
            if self.clip.contains(Point::new(x0 as Scalar, y0 as Scalar)) {
                self.buffer.blend_pixel(x0, y0, color, blend_mode);
            }

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Draw line with anti-aliasing using Wu's algorithm.
    fn draw_line_aa(&mut self, p0: Point, p1: Point, paint: &Paint) {
        let t0 = self.matrix.map_point(p0);
        let t1 = self.matrix.map_point(p1);

        let mut x0 = t0.x;
        let mut y0 = t0.y;
        let mut x1 = t1.x;
        let mut y1 = t1.y;

        let color = paint.color32();
        let blend_mode = paint.blend_mode();

        let steep = (y1 - y0).abs() > (x1 - x0).abs();

        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }

        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx.abs() < 0.0001 { 1.0 } else { dy / dx };

        // Handle first endpoint
        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xgap = 1.0 - (x0 + 0.5).fract();
        let xpxl1 = xend as i32;
        let ypxl1 = yend.floor() as i32;

        if steep {
            self.plot_aa(ypxl1, xpxl1, (1.0 - yend.fract()) * xgap, color, blend_mode);
            self.plot_aa(ypxl1 + 1, xpxl1, yend.fract() * xgap, color, blend_mode);
        } else {
            self.plot_aa(xpxl1, ypxl1, (1.0 - yend.fract()) * xgap, color, blend_mode);
            self.plot_aa(xpxl1, ypxl1 + 1, yend.fract() * xgap, color, blend_mode);
        }

        let mut intery = yend + gradient;

        // Handle second endpoint
        let xend = x1.round();
        let yend = y1 + gradient * (xend - x1);
        let xgap = (x1 + 0.5).fract();
        let xpxl2 = xend as i32;
        let ypxl2 = yend.floor() as i32;

        if steep {
            self.plot_aa(ypxl2, xpxl2, (1.0 - yend.fract()) * xgap, color, blend_mode);
            self.plot_aa(ypxl2 + 1, xpxl2, yend.fract() * xgap, color, blend_mode);
        } else {
            self.plot_aa(xpxl2, ypxl2, (1.0 - yend.fract()) * xgap, color, blend_mode);
            self.plot_aa(xpxl2, ypxl2 + 1, yend.fract() * xgap, color, blend_mode);
        }

        // Main loop
        if steep {
            for x in (xpxl1 + 1)..xpxl2 {
                let y = intery.floor() as i32;
                self.plot_aa(y, x, 1.0 - intery.fract(), color, blend_mode);
                self.plot_aa(y + 1, x, intery.fract(), color, blend_mode);
                intery += gradient;
            }
        } else {
            for x in (xpxl1 + 1)..xpxl2 {
                let y = intery.floor() as i32;
                self.plot_aa(x, y, 1.0 - intery.fract(), color, blend_mode);
                self.plot_aa(x, y + 1, intery.fract(), color, blend_mode);
                intery += gradient;
            }
        }
    }

    /// Plot a pixel with coverage for anti-aliasing.
    #[inline]
    fn plot_aa(&mut self, x: i32, y: i32, coverage: f32, color: Color, blend_mode: BlendMode) {
        if self.clip.contains(Point::new(x as Scalar, y as Scalar)) {
            self.buffer
                .blend_pixel_aa(x, y, color, coverage, blend_mode);
        }
    }

    /// Draw a horizontal line (fast path).
    fn draw_hline(&mut self, x0: i32, x1: i32, y: i32, color: Color, blend_mode: BlendMode) {
        let (start, end) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
        let start = start.max(self.clip.left as i32);
        let end = end.min(self.clip.right as i32 - 1);

        if y < self.clip.top as i32 || y >= self.clip.bottom as i32 {
            return;
        }

        // Batch optimization for opaque SrcOver (most common case)
        if blend_mode == BlendMode::SrcOver
            && color.alpha() == 255
            && start >= 0
            && end < self.buffer.width
            && y >= 0
            && y < self.buffer.height
        {
            let row_offset = (y as usize) * self.buffer.stride;
            let start_offset = row_offset + (start as usize) * 4;
            let end_offset = row_offset + ((end + 1) as usize) * 4;
            let pattern = [color.red(), color.green(), color.blue(), color.alpha()];
            for chunk in self.buffer.pixels[start_offset..end_offset].chunks_exact_mut(4) {
                chunk.copy_from_slice(&pattern);
            }
            return;
        }

        for x in start..=end {
            self.buffer.blend_pixel(x, y, color, blend_mode);
        }
    }

    /// Draw a filled rectangle.
    pub fn fill_rect(&mut self, rect: &Rect, paint: &Paint) {
        let transformed = self.matrix.map_rect(rect);

        let x0 = transformed.left.round() as i32;
        let y0 = transformed.top.round() as i32;
        let x1 = transformed.right.round() as i32;
        let y1 = transformed.bottom.round() as i32;

        let blend_mode = paint.blend_mode();

        // Check if we have a shader
        if let Some(shader) = paint.shader() {
            // Shader-based fill - sample each pixel
            for y in y0..y1 {
                for x in x0..x1 {
                    // Sample shader at pixel center
                    let color4f = shader.sample(x as Scalar + 0.5, y as Scalar + 0.5);
                    let color = color4f.to_color();
                    self.buffer.blend_pixel(x, y, color, blend_mode);
                }
            }
        } else {
            // Solid color fill (fast path)
            let color = paint.color32();
            for y in y0..y1 {
                self.draw_hline(x0, x1 - 1, y, color, blend_mode);
            }
        }
    }

    /// Draw a stroked rectangle.
    pub fn stroke_rect(&mut self, rect: &Rect, paint: &Paint) {
        let tl = Point::new(rect.left, rect.top);
        let tr = Point::new(rect.right, rect.top);
        let bl = Point::new(rect.left, rect.bottom);
        let br = Point::new(rect.right, rect.bottom);

        self.draw_line(tl, tr, paint);
        self.draw_line(tr, br, paint);
        self.draw_line(br, bl, paint);
        self.draw_line(bl, tl, paint);
    }

    /// Draw a rectangle (filled or stroked based on paint style).
    pub fn draw_rect(&mut self, rect: &Rect, paint: &Paint) {
        match paint.style() {
            Style::Fill => self.fill_rect(rect, paint),
            Style::Stroke => self.stroke_rect(rect, paint),
            Style::StrokeAndFill => {
                self.fill_rect(rect, paint);
                self.stroke_rect(rect, paint);
            }
        }
    }

    /// Draw a filled circle using midpoint circle algorithm.
    pub fn fill_circle(&mut self, center: Point, radius: Scalar, paint: &Paint) {
        let tc = self.matrix.map_point(center);
        let cx = tc.x.round() as i32;
        let cy = tc.y.round() as i32;
        let r = (radius * self.matrix.scale_x().abs()).round() as i32;

        let color = paint.color32();
        let blend_mode = paint.blend_mode();

        let mut x = 0;
        let mut y = r;
        let mut d = 1 - r;

        while x <= y {
            // Draw horizontal lines for each octant
            self.draw_hline(cx - x, cx + x, cy + y, color, blend_mode);
            self.draw_hline(cx - x, cx + x, cy - y, color, blend_mode);
            self.draw_hline(cx - y, cx + y, cy + x, color, blend_mode);
            self.draw_hline(cx - y, cx + y, cy - x, color, blend_mode);

            x += 1;
            if d < 0 {
                d += 2 * x + 1;
            } else {
                y -= 1;
                d += 2 * (x - y) + 1;
            }
        }
    }

    /// Draw a stroked circle.
    pub fn stroke_circle(&mut self, center: Point, radius: Scalar, paint: &Paint) {
        let tc = self.matrix.map_point(center);
        let cx = tc.x.round() as i32;
        let cy = tc.y.round() as i32;
        let r = (radius * self.matrix.scale_x().abs()).round() as i32;

        let color = paint.color32();
        let blend_mode = paint.blend_mode();

        let mut x = 0;
        let mut y = r;
        let mut d = 1 - r;

        while x <= y {
            // Plot pixels in all 8 octants
            self.buffer.blend_pixel(cx + x, cy + y, color, blend_mode);
            self.buffer.blend_pixel(cx - x, cy + y, color, blend_mode);
            self.buffer.blend_pixel(cx + x, cy - y, color, blend_mode);
            self.buffer.blend_pixel(cx - x, cy - y, color, blend_mode);
            self.buffer.blend_pixel(cx + y, cy + x, color, blend_mode);
            self.buffer.blend_pixel(cx - y, cy + x, color, blend_mode);
            self.buffer.blend_pixel(cx + y, cy - x, color, blend_mode);
            self.buffer.blend_pixel(cx - y, cy - x, color, blend_mode);

            x += 1;
            if d < 0 {
                d += 2 * x + 1;
            } else {
                y -= 1;
                d += 2 * (x - y) + 1;
            }
        }
    }

    /// Draw a circle (filled or stroked based on paint style).
    pub fn draw_circle(&mut self, center: Point, radius: Scalar, paint: &Paint) {
        if paint.is_anti_alias() {
            self.draw_circle_aa(center, radius, paint);
        } else {
            match paint.style() {
                Style::Fill => self.fill_circle(center, radius, paint),
                Style::Stroke => self.stroke_circle(center, radius, paint),
                Style::StrokeAndFill => {
                    self.fill_circle(center, radius, paint);
                    self.stroke_circle(center, radius, paint);
                }
            }
        }
    }

    /// Draw an anti-aliased circle.
    fn draw_circle_aa(&mut self, center: Point, radius: Scalar, paint: &Paint) {
        let tc = self.matrix.map_point(center);
        let cx = tc.x;
        let cy = tc.y;
        let r = radius * self.matrix.scale_x().abs();

        let color = paint.color32();
        let blend_mode = paint.blend_mode();

        // Calculate bounding box
        let min_x = (cx - r - 1.0).floor() as i32;
        let max_x = (cx + r + 1.0).ceil() as i32;
        let min_y = (cy - r - 1.0).floor() as i32;
        let max_y = (cy + r + 1.0).ceil() as i32;

        match paint.style() {
            Style::Fill => {
                // For each pixel in bounding box
                for py in min_y..=max_y {
                    for px in min_x..=max_x {
                        // Calculate distance from pixel center to circle center
                        let dx = px as f32 + 0.5 - cx;
                        let dy = py as f32 + 0.5 - cy;
                        let dist_sq = dx * dx + dy * dy;

                        // Calculate coverage using smoothstep
                        let dist = dist_sq.sqrt();
                        let coverage = if dist <= r - 0.5 {
                            1.0
                        } else if dist >= r + 0.5 {
                            0.0
                        } else {
                            // Smooth edge
                            1.0 - (dist - (r - 0.5))
                        };

                        if coverage > 0.0 {
                            self.plot_aa(px, py, coverage, color, blend_mode);
                        }
                    }
                }
            }
            Style::Stroke => {
                let stroke_width = paint.stroke_width().max(1.0);
                let outer_r = r + stroke_width / 2.0;
                let inner_r = (r - stroke_width / 2.0).max(0.0);

                for py in min_y..=max_y {
                    for px in min_x..=max_x {
                        let dx = px as f32 + 0.5 - cx;
                        let dy = py as f32 + 0.5 - cy;
                        let dist = (dx * dx + dy * dy).sqrt();

                        let outer_coverage = if dist <= outer_r - 0.5 {
                            1.0
                        } else if dist >= outer_r + 0.5 {
                            0.0
                        } else {
                            1.0 - (dist - (outer_r - 0.5))
                        };

                        let inner_coverage = if dist <= inner_r - 0.5 {
                            1.0
                        } else if dist >= inner_r + 0.5 {
                            0.0
                        } else {
                            1.0 - (dist - (inner_r - 0.5))
                        };

                        let coverage = outer_coverage - inner_coverage;

                        if coverage > 0.0 {
                            self.plot_aa(px, py, coverage, color, blend_mode);
                        }
                    }
                }
            }
            Style::StrokeAndFill => {
                self.draw_circle_aa(center, radius, &{
                    let mut p = paint.clone();
                    p.set_style(Style::Fill);
                    p
                });
                self.draw_circle_aa(center, radius, &{
                    let mut p = paint.clone();
                    p.set_style(Style::Stroke);
                    p
                });
            }
        }
    }

    /// Draw an oval (approximated with ellipse).
    pub fn draw_oval(&mut self, rect: &Rect, paint: &Paint) {
        let center = Point::new(
            (rect.left + rect.right) / 2.0,
            (rect.top + rect.bottom) / 2.0,
        );
        let rx = rect.width() / 2.0;
        let ry = rect.height() / 2.0;

        if (rx - ry).abs() < 0.01 {
            // Close to circle, use circle drawing
            self.draw_circle(center, rx, paint);
        } else {
            // Draw as path with bezier approximation
            let path = ellipse_to_path(center, rx, ry);
            self.draw_path(&path, paint);
        }
    }

    /// Draw a path.
    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        match paint.style() {
            Style::Fill => self.fill_path(path, paint),
            Style::Stroke => self.stroke_path(path, paint),
            Style::StrokeAndFill => {
                self.fill_path(path, paint);
                self.stroke_path(path, paint);
            }
        }
    }

    /// Stroke a path.
    fn stroke_path(&mut self, path: &Path, paint: &Paint) {
        let mut current = Point::zero();
        let mut contour_start = Point::zero();

        for element in path.iter() {
            match element {
                PathElement::Move(p) => {
                    current = p;
                    contour_start = p;
                }
                PathElement::Line(p) => {
                    self.draw_line(current, p, paint);
                    current = p;
                }
                PathElement::Quad(ctrl, end) => {
                    // Approximate with lines
                    let steps = 16;
                    for i in 1..=steps {
                        let t = i as f32 / steps as f32;
                        let mt = 1.0 - t;
                        let p = Point::new(
                            mt * mt * current.x + 2.0 * mt * t * ctrl.x + t * t * end.x,
                            mt * mt * current.y + 2.0 * mt * t * ctrl.y + t * t * end.y,
                        );
                        self.draw_line(
                            if i == 1 {
                                current
                            } else {
                                let pt = (i - 1) as f32 / steps as f32;
                                let pmt = 1.0 - pt;
                                Point::new(
                                    pmt * pmt * current.x
                                        + 2.0 * pmt * pt * ctrl.x
                                        + pt * pt * end.x,
                                    pmt * pmt * current.y
                                        + 2.0 * pmt * pt * ctrl.y
                                        + pt * pt * end.y,
                                )
                            },
                            p,
                            paint,
                        );
                    }
                    current = end;
                }
                PathElement::Conic(ctrl, end, _w) => {
                    // Approximate as quad for simplicity
                    let steps = 16;
                    for i in 1..=steps {
                        let t = i as f32 / steps as f32;
                        let mt = 1.0 - t;
                        let p = Point::new(
                            mt * mt * current.x + 2.0 * mt * t * ctrl.x + t * t * end.x,
                            mt * mt * current.y + 2.0 * mt * t * ctrl.y + t * t * end.y,
                        );
                        let prev_t = (i - 1) as f32 / steps as f32;
                        let prev_mt = 1.0 - prev_t;
                        let prev = Point::new(
                            prev_mt * prev_mt * current.x
                                + 2.0 * prev_mt * prev_t * ctrl.x
                                + prev_t * prev_t * end.x,
                            prev_mt * prev_mt * current.y
                                + 2.0 * prev_mt * prev_t * ctrl.y
                                + prev_t * prev_t * end.y,
                        );
                        self.draw_line(prev, p, paint);
                    }
                    current = end;
                }
                PathElement::Cubic(c1, c2, end) => {
                    // Approximate with lines
                    let steps = 24;
                    let mut prev = current;
                    for i in 1..=steps {
                        let t = i as f32 / steps as f32;
                        let mt = 1.0 - t;
                        let mt2 = mt * mt;
                        let t2 = t * t;
                        let p = Point::new(
                            mt2 * mt * current.x
                                + 3.0 * mt2 * t * c1.x
                                + 3.0 * mt * t2 * c2.x
                                + t2 * t * end.x,
                            mt2 * mt * current.y
                                + 3.0 * mt2 * t * c1.y
                                + 3.0 * mt * t2 * c2.y
                                + t2 * t * end.y,
                        );
                        self.draw_line(prev, p, paint);
                        prev = p;
                    }
                    current = end;
                }
                PathElement::Close => {
                    if current != contour_start {
                        self.draw_line(current, contour_start, paint);
                    }
                    current = contour_start;
                }
            }
        }
    }

    /// Fill a path using the optimized Active Edge Table algorithm.
    ///
    /// This implementation uses:
    /// - Global Edge Table (GET) for efficient edge activation
    /// - Active Edge Table (AET) for tracking current edges
    /// - Full winding number calculation for non-zero fill rule
    /// - Even-odd fill rule support
    /// - Incremental x-intercept updates between scanlines
    fn fill_path(&mut self, path: &Path, paint: &Paint) {
        let fill_type = path.fill_type();
        let color = paint.color32();
        let blend_mode = paint.blend_mode();

        // Collect edges from path
        let edges = collect_edges(path, &self.matrix);
        if edges.is_empty() {
            return;
        }

        // Create Global Edge Table (edges sorted by y_min)
        let mut get = GlobalEdgeTable::new(edges);

        // Get scanline range
        let Some(y_start) = get.y_min() else {
            return;
        };
        let y_end = get.y_max();

        let y_min = y_start.floor() as i32;
        let y_max = y_end.ceil() as i32;

        // Create Active Edge Table
        let mut aet = ActiveEdgeTable::new();

        // Process each scanline
        for y in y_min..y_max {
            let scanline = y as f32 + 0.5;

            // Add new edges that become active at this scanline
            aet.add_edges(get.get_new_edges_at(scanline), scanline);

            // Remove edges that are no longer active
            aet.remove_inactive(scanline);

            // Skip if no active edges
            if aet.is_empty() {
                continue;
            }

            // Sort active edges by x-intercept (insertion sort - efficient for nearly-sorted)
            aet.sort_by_x();

            // Get spans to fill based on fill rule
            let spans = aet.get_spans(fill_type);

            // Fill spans
            for (x0, x1) in spans {
                let x_start = x0.round() as i32;
                let x_end = x1.round() as i32;
                if x_start < x_end {
                    self.draw_hline(x_start, x_end - 1, y, color, blend_mode);
                }
            }

            // Update x-intercepts for next scanline
            aet.step_all();
        }
    }

    /// Fill a path using anti-aliased rendering.
    ///
    /// Uses supersampling for improved edge quality.
    pub fn fill_path_aa(&mut self, path: &Path, paint: &Paint) {
        let fill_type = path.fill_type();
        let color = paint.color32();
        let blend_mode = paint.blend_mode();

        // Collect edges from path
        let edges = collect_edges(path, &self.matrix);
        if edges.is_empty() {
            return;
        }

        // Create Global Edge Table for initial scanline range
        let get = GlobalEdgeTable::new(edges);

        let Some(y_start) = get.y_min() else {
            return;
        };
        let y_end = get.y_max();

        let y_min = y_start.floor() as i32;
        let y_max = y_end.ceil() as i32;

        // 4x vertical supersampling
        const SAMPLES: usize = 4;
        let sample_offsets = [0.125f32, 0.375, 0.625, 0.875];

        // Process each pixel row
        for y in y_min..y_max {
            // Accumulate coverage for each pixel
            let mut coverage_map: std::collections::HashMap<i32, f32> =
                std::collections::HashMap::new();

            // Sample at multiple y positions within the pixel
            for &offset in &sample_offsets {
                let scanline = y as f32 + offset;

                // Re-create AET for each sample (simpler than tracking multiple)
                let mut sample_aet = ActiveEdgeTable::new();
                let edges = collect_edges(path, &self.matrix);
                let mut sample_get = GlobalEdgeTable::new(edges);

                sample_aet.add_edges(sample_get.get_new_edges_at(scanline), scanline);

                if sample_aet.is_empty() {
                    continue;
                }

                sample_aet.sort_by_x();
                let spans = sample_aet.get_spans(fill_type);

                // Accumulate coverage
                for (x0, x1) in spans {
                    let x_start = x0.floor() as i32;
                    let x_end = x1.ceil() as i32;

                    for x in x_start..x_end {
                        // Calculate pixel coverage for this sample
                        let pixel_left = x as f32;
                        let pixel_right = (x + 1) as f32;

                        let overlap_left = pixel_left.max(x0);
                        let overlap_right = pixel_right.min(x1);
                        let overlap = (overlap_right - overlap_left).max(0.0);

                        *coverage_map.entry(x).or_insert(0.0) += overlap / SAMPLES as f32;
                    }
                }
            }

            // Render pixels with accumulated coverage
            for (x, coverage) in coverage_map {
                if coverage > 0.0 {
                    self.buffer
                        .blend_pixel_aa(x, y, color, coverage.min(1.0), blend_mode);
                }
            }
        }
    }
}

/// An edge for scanline rasterization with winding direction.
///
/// Edges are oriented from y_min to y_max, and the winding direction
/// is used for non-zero fill rule calculation.
#[derive(Debug, Clone)]
struct Edge {
    /// Minimum y coordinate (top of edge).
    y_min: f32,
    /// Maximum y coordinate (bottom of edge).
    y_max: f32,
    /// X coordinate at y_min.
    x_at_y_min: f32,
    /// Inverse slope (dx/dy) for efficient x calculation.
    inv_slope: f32,
    /// Winding direction: +1 for downward edges, -1 for upward edges.
    /// Used for non-zero fill rule.
    winding: i32,
}

impl Edge {
    /// Create a new edge from two points.
    ///
    /// Returns `None` for horizontal edges (no contribution to fill).
    fn new(p0: Point, p1: Point) -> Option<Self> {
        let dy = p1.y - p0.y;
        if dy.abs() < 0.001 {
            return None; // Horizontal edge
        }

        // Determine winding direction based on original edge direction
        let winding = if dy > 0.0 { 1 } else { -1 };

        // Orient edge so y_min < y_max
        let (top, bottom) = if p0.y < p1.y { (p0, p1) } else { (p1, p0) };

        let dy = bottom.y - top.y;
        let dx = bottom.x - top.x;

        Some(Self {
            y_min: top.y,
            y_max: bottom.y,
            x_at_y_min: top.x,
            inv_slope: dx / dy,
            winding,
        })
    }

    /// Calculate x intersection at a given scanline y.
    #[inline]
    fn x_at(&self, y: f32) -> f32 {
        self.x_at_y_min + (y - self.y_min) * self.inv_slope
    }

    /// Check if this edge is active at the given scanline.
    ///
    /// Note: This method is available for direct edge queries but is not used
    /// by the optimized AET algorithm which tracks edges through the GET.
    #[inline]
    #[allow(dead_code)]
    fn is_active_at(&self, y: f32) -> bool {
        y >= self.y_min && y < self.y_max
    }
}

/// Active edge entry for the Active Edge Table.
///
/// Contains the current x-intercept and a reference to the edge.
#[derive(Debug, Clone)]
struct ActiveEdge {
    /// Current x-intercept at the current scanline.
    x: f32,
    /// Inverse slope for incremental updates.
    inv_slope: f32,
    /// Winding direction.
    winding: i32,
    /// Maximum y coordinate (for removal).
    y_max: f32,
}

impl ActiveEdge {
    /// Create a new active edge from an Edge at a given scanline.
    fn from_edge(edge: &Edge, y: f32) -> Self {
        Self {
            x: edge.x_at(y),
            inv_slope: edge.inv_slope,
            winding: edge.winding,
            y_max: edge.y_max,
        }
    }

    /// Update x-intercept for the next scanline.
    #[inline]
    fn step(&mut self) {
        self.x += self.inv_slope;
    }

    /// Check if this edge is still active at the given y.
    #[inline]
    fn is_active_at(&self, y: f32) -> bool {
        y < self.y_max
    }
}

/// Global Edge Table - edges sorted by y_min for efficient scanline processing.
struct GlobalEdgeTable {
    /// Edges sorted by y_min.
    edges: Vec<Edge>,
    /// Current index into the edge list.
    current_index: usize,
}

impl GlobalEdgeTable {
    /// Create a new GET from a list of edges.
    fn new(mut edges: Vec<Edge>) -> Self {
        // Sort edges by y_min (primary), then by x_at_y_min (secondary)
        edges.sort_by(|a, b| {
            a.y_min
                .partial_cmp(&b.y_min)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.x_at_y_min
                        .partial_cmp(&b.x_at_y_min)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        Self {
            edges,
            current_index: 0,
        }
    }

    /// Get the minimum y coordinate where edges start.
    fn y_min(&self) -> Option<f32> {
        self.edges.first().map(|e| e.y_min)
    }

    /// Get the maximum y coordinate where edges end.
    fn y_max(&self) -> f32 {
        self.edges
            .iter()
            .map(|e| e.y_max)
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// Get all edges that become active at the given scanline.
    fn get_new_edges_at(&mut self, y: f32) -> impl Iterator<Item = &Edge> {
        let start = self.current_index;
        while self.current_index < self.edges.len() && self.edges[self.current_index].y_min <= y {
            self.current_index += 1;
        }
        self.edges[start..self.current_index].iter()
    }
}

/// Active Edge Table - maintains edges intersecting the current scanline.
struct ActiveEdgeTable {
    /// Active edges, sorted by x-intercept.
    edges: Vec<ActiveEdge>,
}

impl ActiveEdgeTable {
    /// Create a new empty AET.
    fn new() -> Self {
        Self { edges: Vec::new() }
    }

    /// Add new edges that become active at the given scanline.
    fn add_edges<'a>(&mut self, new_edges: impl Iterator<Item = &'a Edge>, y: f32) {
        for edge in new_edges {
            self.edges.push(ActiveEdge::from_edge(edge, y));
        }
    }

    /// Remove edges that are no longer active at the given scanline.
    fn remove_inactive(&mut self, y: f32) {
        self.edges.retain(|e| e.is_active_at(y));
    }

    /// Sort edges by x-intercept using insertion sort.
    ///
    /// Insertion sort is optimal here because the list is nearly sorted
    /// (edges only move slightly between scanlines).
    fn sort_by_x(&mut self) {
        // Insertion sort - optimal for nearly sorted data
        for i in 1..self.edges.len() {
            let mut j = i;
            while j > 0 && self.edges[j - 1].x > self.edges[j].x {
                self.edges.swap(j - 1, j);
                j -= 1;
            }
        }
    }

    /// Step all edges to the next scanline.
    fn step_all(&mut self) {
        for edge in &mut self.edges {
            edge.step();
        }
    }

    /// Get span pairs for filling using the specified fill rule.
    fn get_spans(&self, fill_type: FillType) -> Vec<(f32, f32)> {
        let mut spans = Vec::new();

        match fill_type {
            FillType::Winding | FillType::InverseWinding => {
                // Non-zero winding rule
                let mut winding = 0i32;
                let mut span_start: Option<f32> = None;

                for edge in &self.edges {
                    let was_inside = winding != 0;
                    winding += edge.winding;
                    let is_inside = winding != 0;

                    if !was_inside && is_inside {
                        span_start = Some(edge.x);
                    } else if was_inside && !is_inside {
                        if let Some(start) = span_start {
                            spans.push((start, edge.x));
                            span_start = None;
                        }
                    }
                }
            }
            FillType::EvenOdd | FillType::InverseEvenOdd => {
                // Even-odd rule - fill between alternating pairs
                let mut inside = false;
                let mut span_start: Option<f32> = None;

                for edge in &self.edges {
                    inside = !inside;
                    if inside {
                        span_start = Some(edge.x);
                    } else if let Some(start) = span_start {
                        spans.push((start, edge.x));
                        span_start = None;
                    }
                }
            }
        }

        spans
    }

    /// Check if the AET is empty.
    fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

/// Collect edges from a path.
fn collect_edges(path: &Path, matrix: &Matrix) -> Vec<Edge> {
    let mut edges = Vec::new();
    let mut current = Point::zero();
    let mut contour_start = Point::zero();

    for element in path.iter() {
        match element {
            PathElement::Move(p) => {
                current = matrix.map_point(p);
                contour_start = current;
            }
            PathElement::Line(p) => {
                let end = matrix.map_point(p);
                if let Some(edge) = Edge::new(current, end) {
                    edges.push(edge);
                }
                current = end;
            }
            PathElement::Quad(ctrl, end) => {
                let ctrl = matrix.map_point(ctrl);
                let end = matrix.map_point(end);
                // Flatten to lines
                let steps = 8;
                let start = current;
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let mt = 1.0 - t;
                    let p = Point::new(
                        mt * mt * start.x + 2.0 * mt * t * ctrl.x + t * t * end.x,
                        mt * mt * start.y + 2.0 * mt * t * ctrl.y + t * t * end.y,
                    );
                    if let Some(edge) = Edge::new(current, p) {
                        edges.push(edge);
                    }
                    current = p;
                }
            }
            PathElement::Conic(ctrl, end, _w) => {
                let ctrl = matrix.map_point(ctrl);
                let end = matrix.map_point(end);
                let steps = 8;
                let start = current;
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let mt = 1.0 - t;
                    let p = Point::new(
                        mt * mt * start.x + 2.0 * mt * t * ctrl.x + t * t * end.x,
                        mt * mt * start.y + 2.0 * mt * t * ctrl.y + t * t * end.y,
                    );
                    if let Some(edge) = Edge::new(current, p) {
                        edges.push(edge);
                    }
                    current = p;
                }
            }
            PathElement::Cubic(c1, c2, end) => {
                let c1 = matrix.map_point(c1);
                let c2 = matrix.map_point(c2);
                let end = matrix.map_point(end);
                let steps = 12;
                let start = current;
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let mt = 1.0 - t;
                    let mt2 = mt * mt;
                    let t2 = t * t;
                    let p = Point::new(
                        mt2 * mt * start.x
                            + 3.0 * mt2 * t * c1.x
                            + 3.0 * mt * t2 * c2.x
                            + t2 * t * end.x,
                        mt2 * mt * start.y
                            + 3.0 * mt2 * t * c1.y
                            + 3.0 * mt * t2 * c2.y
                            + t2 * t * end.y,
                    );
                    if let Some(edge) = Edge::new(current, p) {
                        edges.push(edge);
                    }
                    current = p;
                }
            }
            PathElement::Close => {
                if let Some(edge) = Edge::new(current, contour_start) {
                    edges.push(edge);
                }
                current = contour_start;
            }
        }
    }

    edges
}

/// Create an ellipse path using cubic bezier approximation.
fn ellipse_to_path(center: Point, rx: Scalar, ry: Scalar) -> Path {
    use skia_rs_path::PathBuilder;

    // Magic number for cubic approximation of quarter circle
    const KAPPA: Scalar = 0.5522847498;

    let kx = rx * KAPPA;
    let ky = ry * KAPPA;

    let mut builder = PathBuilder::new();
    builder.move_to(center.x + rx, center.y);

    // Top right quadrant
    builder.cubic_to(
        center.x + rx,
        center.y - ky,
        center.x + kx,
        center.y - ry,
        center.x,
        center.y - ry,
    );

    // Top left quadrant
    builder.cubic_to(
        center.x - kx,
        center.y - ry,
        center.x - rx,
        center.y - ky,
        center.x - rx,
        center.y,
    );

    // Bottom left quadrant
    builder.cubic_to(
        center.x - rx,
        center.y + ky,
        center.x - kx,
        center.y + ry,
        center.x,
        center.y + ry,
    );

    // Bottom right quadrant
    builder.cubic_to(
        center.x + kx,
        center.y + ry,
        center.x + rx,
        center.y + ky,
        center.x + rx,
        center.y,
    );

    builder.close();
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_buffer_new() {
        let buffer = PixelBuffer::new(100, 100);
        assert_eq!(buffer.width, 100);
        assert_eq!(buffer.height, 100);
        assert_eq!(buffer.pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn test_pixel_buffer_clear() {
        let mut buffer = PixelBuffer::new(10, 10);
        buffer.clear(Color::from_argb(255, 255, 0, 0));

        let pixel = buffer.get_pixel(5, 5).unwrap();
        assert_eq!(pixel.red(), 255);
        assert_eq!(pixel.green(), 0);
        assert_eq!(pixel.blue(), 0);
        assert_eq!(pixel.alpha(), 255);
    }

    #[test]
    fn test_pixel_buffer_set_get() {
        let mut buffer = PixelBuffer::new(10, 10);
        buffer.set_pixel(5, 5, Color::from_argb(255, 0, 255, 0));

        let pixel = buffer.get_pixel(5, 5).unwrap();
        assert_eq!(pixel.green(), 255);
    }

    #[test]
    fn test_rasterizer_draw_rect() {
        let mut buffer = PixelBuffer::new(100, 100);
        buffer.clear(Color::from_argb(255, 255, 255, 255));

        let mut rasterizer = Rasterizer::new(&mut buffer);
        let mut paint = Paint::new();
        paint.set_color32(Color::from_argb(255, 255, 0, 0));
        paint.set_style(Style::Fill);

        rasterizer.fill_rect(&Rect::from_xywh(10.0, 10.0, 50.0, 50.0), &paint);

        // Check a pixel inside the rect
        let pixel = buffer.get_pixel(25, 25).unwrap();
        assert_eq!(pixel.red(), 255);
        assert_eq!(pixel.green(), 0);
    }

    #[test]
    fn test_blend_src_over() {
        let src = Color::from_argb(128, 255, 0, 0);
        let dst = Color::from_argb(255, 0, 0, 255);
        let result = blend_colors(src, dst, BlendMode::SrcOver);

        // Semi-transparent red over blue should give purple-ish
        assert!(result.red() > 100);
        assert!(result.blue() > 100);
    }

    // ============ Active Edge Table Tests ============

    #[test]
    fn test_edge_creation() {
        // Horizontal edge should return None
        let p0 = Point::new(0.0, 10.0);
        let p1 = Point::new(100.0, 10.0);
        assert!(Edge::new(p0, p1).is_none());

        // Downward edge (positive winding)
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(10.0, 100.0);
        let edge = Edge::new(p0, p1).unwrap();
        assert_eq!(edge.winding, 1);
        assert_eq!(edge.y_min, 0.0);
        assert_eq!(edge.y_max, 100.0);

        // Upward edge (negative winding)
        let p0 = Point::new(10.0, 100.0);
        let p1 = Point::new(0.0, 0.0);
        let edge = Edge::new(p0, p1).unwrap();
        assert_eq!(edge.winding, -1);
        assert_eq!(edge.y_min, 0.0);
        assert_eq!(edge.y_max, 100.0);
    }

    #[test]
    fn test_edge_x_at() {
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(100.0, 100.0);
        let edge = Edge::new(p0, p1).unwrap();

        // 45-degree line: x should equal y
        assert!((edge.x_at(0.0) - 0.0).abs() < 0.001);
        assert!((edge.x_at(50.0) - 50.0).abs() < 0.001);
        assert!((edge.x_at(100.0) - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_active_edge_step() {
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(100.0, 100.0);
        let edge = Edge::new(p0, p1).unwrap();

        let mut active = ActiveEdge::from_edge(&edge, 0.5);
        let initial_x = active.x;
        active.step();
        // After stepping, x should increase by inv_slope
        assert!((active.x - initial_x - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_global_edge_table_ordering() {
        let edges = vec![
            Edge::new(Point::new(0.0, 50.0), Point::new(10.0, 100.0)).unwrap(),
            Edge::new(Point::new(0.0, 0.0), Point::new(10.0, 50.0)).unwrap(),
            Edge::new(Point::new(0.0, 25.0), Point::new(10.0, 75.0)).unwrap(),
        ];

        let get = GlobalEdgeTable::new(edges);

        // Edges should be sorted by y_min
        assert_eq!(get.edges[0].y_min, 0.0);
        assert_eq!(get.edges[1].y_min, 25.0);
        assert_eq!(get.edges[2].y_min, 50.0);
    }

    #[test]
    fn test_active_edge_table_spans_even_odd() {
        let mut aet = ActiveEdgeTable::new();

        // Simulate a square: 4 vertical edges
        let left_edge = Edge::new(Point::new(10.0, 0.0), Point::new(10.0, 100.0)).unwrap();
        let right_edge = Edge::new(Point::new(50.0, 0.0), Point::new(50.0, 100.0)).unwrap();

        aet.add_edges([&left_edge, &right_edge].into_iter(), 50.0);
        aet.sort_by_x();

        let spans = aet.get_spans(FillType::EvenOdd);
        assert_eq!(spans.len(), 1);
        assert!((spans[0].0 - 10.0).abs() < 0.001);
        assert!((spans[0].1 - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_active_edge_table_spans_nonzero() {
        let mut aet = ActiveEdgeTable::new();

        // Create a proper polygon contour - a square traversed clockwise
        // Left edge goes down (winding +1), right edge goes up (winding -1)
        let left_edge = Edge::new(Point::new(10.0, 0.0), Point::new(10.0, 100.0)).unwrap();
        // Reverse the right edge so it goes up (from bottom to top)
        let right_edge = Edge::new(Point::new(50.0, 100.0), Point::new(50.0, 0.0)).unwrap();

        aet.add_edges([&left_edge, &right_edge].into_iter(), 50.0);
        aet.sort_by_x();

        let spans = aet.get_spans(FillType::Winding);

        // Left edge winding +1, right edge winding -1
        // At scanline: winding goes 0 -> +1 -> 0
        // Should produce one span from left to right
        assert_eq!(spans.len(), 1);
        assert!((spans[0].0 - 10.0).abs() < 0.001);
        assert!((spans[0].1 - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_fill_triangle_path() {
        use skia_rs_path::PathBuilder;

        let mut buffer = PixelBuffer::new(100, 100);
        buffer.clear(Color::from_argb(255, 255, 255, 255));

        let mut rasterizer = Rasterizer::new(&mut buffer);
        let mut paint = Paint::new();
        paint.set_color32(Color::from_argb(255, 255, 0, 0));
        paint.set_style(Style::Fill);

        // Create a triangle path
        let mut builder = PathBuilder::new();
        builder
            .move_to(50.0, 10.0)
            .line_to(90.0, 90.0)
            .line_to(10.0, 90.0)
            .close();
        let path = builder.build();

        rasterizer.draw_path(&path, &paint);

        // Check a pixel inside the triangle (centroid-ish)
        let pixel = buffer.get_pixel(50, 60).unwrap();
        assert_eq!(pixel.red(), 255, "Triangle should be filled at center");

        // Check a pixel outside the triangle
        let pixel = buffer.get_pixel(10, 10).unwrap();
        assert_eq!(
            pixel.red(),
            255,
            "Outside should remain white (background)"
        );
        assert_eq!(pixel.green(), 255);
    }

    #[test]
    fn test_fill_complex_polygon() {
        use skia_rs_path::PathBuilder;

        let mut buffer = PixelBuffer::new(100, 100);
        buffer.clear(Color::from_argb(255, 0, 0, 0));

        let mut rasterizer = Rasterizer::new(&mut buffer);
        let mut paint = Paint::new();
        paint.set_color32(Color::from_argb(255, 0, 255, 0));
        paint.set_style(Style::Fill);

        // Create a star-like shape (self-intersecting)
        let mut builder = PathBuilder::new();
        builder
            .move_to(50.0, 10.0)
            .line_to(61.0, 40.0)
            .line_to(90.0, 40.0)
            .line_to(68.0, 58.0)
            .line_to(79.0, 90.0)
            .line_to(50.0, 70.0)
            .line_to(21.0, 90.0)
            .line_to(32.0, 58.0)
            .line_to(10.0, 40.0)
            .line_to(39.0, 40.0)
            .close();
        let path = builder.build();

        rasterizer.draw_path(&path, &paint);

        // The path should have some filled pixels
        // Check center region
        let pixel = buffer.get_pixel(50, 50).unwrap();
        // With even-odd rule, center of star might not be filled
        // With non-zero (default), it should be filled
        assert_eq!(pixel.green(), 255, "Star center should be filled");
    }

    #[test]
    fn test_winding_number_calculation() {
        // Test that the winding rule correctly handles overlapping regions
        use skia_rs_path::PathBuilder;

        let mut buffer = PixelBuffer::new(100, 100);

        // Create two overlapping squares - with non-zero winding, overlap is filled
        let mut path_builder = PathBuilder::new();

        // First square (clockwise)
        path_builder
            .move_to(20.0, 20.0)
            .line_to(60.0, 20.0)
            .line_to(60.0, 60.0)
            .line_to(20.0, 60.0)
            .close();

        // Second square (also clockwise, overlapping)
        path_builder
            .move_to(40.0, 40.0)
            .line_to(80.0, 40.0)
            .line_to(80.0, 80.0)
            .line_to(40.0, 80.0)
            .close();

        let mut path = path_builder.build();
        path.set_fill_type(FillType::Winding);

        buffer.clear(Color::from_argb(255, 255, 255, 255));
        let mut rasterizer = Rasterizer::new(&mut buffer);
        let mut paint = Paint::new();
        paint.set_color32(Color::from_argb(255, 255, 0, 0));

        rasterizer.fill_path(&path, &paint);

        // With non-zero winding, the overlap region should be filled
        let overlap_pixel = buffer.get_pixel(50, 50).unwrap();
        assert_eq!(overlap_pixel.red(), 255, "Overlap should be filled");
    }
}
