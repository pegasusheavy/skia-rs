//! Rasterizer for drawing primitives to pixel buffers.
//!
//! This module provides software rasterization for basic shapes.

use skia_rs_core::{Color, Matrix, Point, Rect, Scalar};
use skia_rs_paint::{BlendMode, Paint, Style};
use skia_rs_path::{Path, PathElement};

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
            use skia_rs_paint::shader::Shader;
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

        let r_sq = r * r;

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

    /// Fill a path using scanline algorithm.
    fn fill_path(&mut self, path: &Path, paint: &Paint) {
        // Get transformed bounds
        let bounds = self.matrix.map_rect(&path.bounds());

        let y_min = bounds.top.floor() as i32;
        let y_max = bounds.bottom.ceil() as i32;

        let color = paint.color32();
        let blend_mode = paint.blend_mode();

        // Collect edges
        let edges = collect_edges(path, &self.matrix);

        // Scanline fill
        for y in y_min..y_max {
            let scanline = y as f32 + 0.5;

            // Find intersections
            let mut intersections: Vec<f32> = Vec::new();

            for edge in &edges {
                if let Some(x) = edge.intersect_scanline(scanline) {
                    intersections.push(x);
                }
            }

            // Sort intersections
            intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            // Fill between pairs
            let mut i = 0;
            while i + 1 < intersections.len() {
                let x0 = intersections[i].round() as i32;
                let x1 = intersections[i + 1].round() as i32;
                self.draw_hline(x0, x1, y, color, blend_mode);
                i += 2;
            }
        }
    }
}

/// An edge for scanline rasterization.
#[derive(Debug, Clone)]
struct Edge {
    y_min: f32,
    y_max: f32,
    x_at_y_min: f32,
    inv_slope: f32, // dx/dy
}

impl Edge {
    fn new(p0: Point, p1: Point) -> Option<Self> {
        let (top, bottom) = if p0.y < p1.y { (p0, p1) } else { (p1, p0) };

        let dy = bottom.y - top.y;
        if dy.abs() < 0.001 {
            return None; // Horizontal edge
        }

        let dx = bottom.x - top.x;

        Some(Self {
            y_min: top.y,
            y_max: bottom.y,
            x_at_y_min: top.x,
            inv_slope: dx / dy,
        })
    }

    fn intersect_scanline(&self, y: f32) -> Option<f32> {
        if y < self.y_min || y >= self.y_max {
            return None;
        }
        let x = self.x_at_y_min + (y - self.y_min) * self.inv_slope;
        Some(x)
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
}
