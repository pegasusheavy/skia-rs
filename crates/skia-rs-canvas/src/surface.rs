//! Surface backing store for canvas.

use crate::Canvas;
use crate::raster::PixelBuffer;
#[cfg(feature = "codec")]
use skia_rs_codec::Image;
use skia_rs_core::pixel::{ImageInfo, SurfaceProps};
use skia_rs_core::{AlphaType, ColorType, Color, IRect, Matrix, Point, Rect, Region, Scalar};
use skia_rs_paint::{BlendMode, Paint};
use skia_rs_path::Path;

/// A surface is a backing store for a canvas.
pub struct Surface {
    info: ImageInfo,
    #[allow(dead_code)]
    props: SurfaceProps,
    buffer: PixelBuffer,
}

impl Surface {
    /// Create a raster surface.
    pub fn new_raster(info: &ImageInfo, props: Option<&SurfaceProps>) -> Option<Self> {
        if info.is_empty() {
            return None;
        }

        let buffer = PixelBuffer::new(info.width(), info.height());

        Some(Self {
            info: info.clone(),
            props: props.copied().unwrap_or_default(),
            buffer,
        })
    }

    /// Create a raster surface with specified dimensions using RGBA8888 format.
    pub fn new_raster_n32_premul(width: i32, height: i32) -> Option<Self> {
        use skia_rs_core::{AlphaType, ColorType};

        let info = ImageInfo::new(width, height, ColorType::Rgba8888, AlphaType::Premul).ok()?;
        Self::new_raster(&info, None)
    }

    /// Get the image info.
    #[inline]
    pub fn info(&self) -> &ImageInfo {
        &self.info
    }

    /// Get the width.
    #[inline]
    pub fn width(&self) -> i32 {
        self.info.width()
    }

    /// Get the height.
    #[inline]
    pub fn height(&self) -> i32 {
        self.info.height()
    }

    /// Get a canvas for drawing (without pixel access).
    pub fn canvas(&self) -> Canvas {
        Canvas::new(self.info.width(), self.info.height())
    }

    /// Get a raster canvas that can actually draw pixels.
    pub fn raster_canvas(&mut self) -> RasterCanvas<'_> {
        RasterCanvas::new(&mut self.buffer)
    }

    /// Get access to the pixel data.
    pub fn pixels(&self) -> &[u8] {
        &self.buffer.pixels
    }

    /// Get mutable access to the pixel data.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.buffer.pixels
    }

    /// Get the row bytes.
    pub fn row_bytes(&self) -> usize {
        self.buffer.stride
    }

    /// Get the pixel buffer.
    pub fn pixel_buffer(&self) -> &PixelBuffer {
        &self.buffer
    }

    /// Get mutable pixel buffer.
    pub fn pixel_buffer_mut(&mut self) -> &mut PixelBuffer {
        &mut self.buffer
    }

    /// Create a snapshot of the surface as an immutable image.
    ///
    /// The returned image shares pixel data with the surface when possible.
    #[cfg(feature = "codec")]
    pub fn make_image_snapshot(&self) -> Option<Image> {
        let pixels = self.buffer.pixels.clone();
        let row_bytes = self.buffer.stride;

        // Convert core ImageInfo to codec ImageInfo
        let codec_info = skia_rs_codec::ImageInfo::new(
            self.info.width(),
            self.info.height(),
            self.info.color_type,
            self.info.alpha_type,
        );

        Image::from_raster_data_owned(codec_info, pixels, row_bytes)
    }

    /// Create a snapshot of a subset of the surface.
    #[cfg(feature = "codec")]
    pub fn make_image_snapshot_subset(&self, subset: &IRect) -> Option<Image> {
        // Validate subset bounds
        if subset.left < 0
            || subset.top < 0
            || subset.right > self.width()
            || subset.bottom > self.height()
        {
            return None;
        }

        let width = subset.width();
        let height = subset.height();
        let row_bytes = (width as usize) * 4;
        let mut pixels = vec![0u8; (height as usize) * row_bytes];

        // Copy subset pixels
        for y in 0..height {
            for x in 0..width {
                let src_x = subset.left + x;
                let src_y = subset.top + y;

                if let Some(color) = self.buffer.get_pixel(src_x, src_y) {
                    let dst_offset = ((y * width + x) * 4) as usize;
                    pixels[dst_offset] = color.red();
                    pixels[dst_offset + 1] = color.green();
                    pixels[dst_offset + 2] = color.blue();
                    pixels[dst_offset + 3] = color.alpha();
                }
            }
        }

        let info =
            skia_rs_codec::ImageInfo::new(width, height, ColorType::Rgba8888, AlphaType::Premul);
        Image::from_raster_data_owned(info, pixels, row_bytes)
    }
}

// =============================================================================
// GPU Surface Abstraction
// =============================================================================

/// Backend type for GPU surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuBackendType {
    /// OpenGL backend.
    OpenGL,
    /// Vulkan backend.
    Vulkan,
    /// Metal backend (macOS/iOS).
    Metal,
    /// Direct3D 11 backend (Windows).
    Direct3D11,
    /// Direct3D 12 backend (Windows).
    Direct3D12,
    /// Dawn/WebGPU backend.
    Dawn,
}

/// GPU surface capabilities.
#[derive(Debug, Clone, Default)]
pub struct GpuSurfaceCapabilities {
    /// Maximum texture size.
    pub max_texture_size: u32,
    /// Whether MSAA is supported.
    pub msaa_supported: bool,
    /// Maximum MSAA sample count.
    pub max_msaa_samples: u32,
    /// Whether sRGB framebuffers are supported.
    pub srgb_framebuffer: bool,
    /// Whether floating-point framebuffers are supported.
    pub float_framebuffer: bool,
}

/// GPU context for creating GPU surfaces.
pub trait GpuContext: Send + Sync {
    /// Get the backend type.
    fn backend_type(&self) -> GpuBackendType;

    /// Get surface capabilities.
    fn capabilities(&self) -> GpuSurfaceCapabilities;

    /// Create a GPU surface with the given dimensions.
    fn create_surface(
        &self,
        width: i32,
        height: i32,
        info: &ImageInfo,
    ) -> Option<Box<dyn GpuSurface>>;

    /// Flush all pending GPU operations.
    fn flush(&self);

    /// Finish all pending GPU operations (blocking).
    fn finish(&self);

    /// Reset the context state.
    fn reset(&self);
}

/// A GPU-backed surface.
pub trait GpuSurface: Send + Sync {
    /// Get the image info.
    fn info(&self) -> &ImageInfo;

    /// Get the width.
    fn width(&self) -> i32;

    /// Get the height.
    fn height(&self) -> i32;

    /// Read pixels from the GPU surface into a buffer.
    fn read_pixels(&self, dst: &mut [u8], dst_row_bytes: usize, src_x: i32, src_y: i32) -> bool;

    /// Write pixels to the GPU surface from a buffer.
    fn write_pixels(&mut self, src: &[u8], src_row_bytes: usize, dst_x: i32, dst_y: i32) -> bool;

    /// Flush pending drawing operations.
    fn flush(&mut self);

    /// Create an image snapshot.
    #[cfg(feature = "codec")]
    fn make_image_snapshot(&self) -> Option<Image>;
}

/// A canvas that draws directly to a pixel buffer.
pub struct RasterCanvas<'a> {
    buffer: &'a mut PixelBuffer,
    matrix_stack: Vec<Matrix>,
    clip_stack: Vec<Rect>,
    save_count: usize,
}

impl<'a> RasterCanvas<'a> {
    /// Create a new raster canvas.
    pub fn new(buffer: &'a mut PixelBuffer) -> Self {
        let clip = Rect::from_xywh(0.0, 0.0, buffer.width as Scalar, buffer.height as Scalar);
        Self {
            buffer,
            matrix_stack: vec![Matrix::IDENTITY],
            clip_stack: vec![clip],
            save_count: 1,
        }
    }

    /// Get the width.
    #[inline]
    pub fn width(&self) -> i32 {
        self.buffer.width
    }

    /// Get the height.
    #[inline]
    pub fn height(&self) -> i32 {
        self.buffer.height
    }

    /// Get the current transformation matrix.
    #[inline]
    pub fn total_matrix(&self) -> &Matrix {
        self.matrix_stack.last().unwrap()
    }

    /// Get the current clip bounds.
    #[inline]
    pub fn clip_bounds(&self) -> Rect {
        self.clip_stack.last().copied().unwrap_or(Rect::EMPTY)
    }

    /// Save the current state.
    pub fn save(&mut self) -> usize {
        let matrix = *self.matrix_stack.last().unwrap();
        let clip = *self.clip_stack.last().unwrap();
        self.matrix_stack.push(matrix);
        self.clip_stack.push(clip);
        self.save_count += 1;
        self.save_count
    }

    /// Restore to the previous state.
    pub fn restore(&mut self) {
        if self.save_count > 1 {
            self.matrix_stack.pop();
            self.clip_stack.pop();
            self.save_count -= 1;
        }
    }

    /// Restore to a specific save count.
    pub fn restore_to_count(&mut self, count: usize) {
        while self.save_count > count {
            self.restore();
        }
    }

    /// Translate the canvas.
    pub fn translate(&mut self, dx: Scalar, dy: Scalar) {
        let matrix = Matrix::translate(dx, dy);
        self.concat(&matrix);
    }

    /// Scale the canvas.
    pub fn scale(&mut self, sx: Scalar, sy: Scalar) {
        let matrix = Matrix::scale(sx, sy);
        self.concat(&matrix);
    }

    /// Rotate the canvas (angle in degrees).
    pub fn rotate(&mut self, degrees: Scalar) {
        let radians = degrees * std::f32::consts::PI / 180.0;
        let matrix = Matrix::rotate(radians);
        self.concat(&matrix);
    }

    /// Concatenate a matrix.
    pub fn concat(&mut self, matrix: &Matrix) {
        if let Some(current) = self.matrix_stack.last_mut() {
            *current = current.concat(matrix);
        }
    }

    /// Set the matrix.
    pub fn set_matrix(&mut self, matrix: &Matrix) {
        if let Some(current) = self.matrix_stack.last_mut() {
            *current = *matrix;
        }
    }

    /// Clip to a rectangle.
    pub fn clip_rect(&mut self, rect: &Rect) {
        let transformed = self.total_matrix().map_rect(rect);
        if let Some(current) = self.clip_stack.last_mut() {
            if let Some(intersection) = current.intersect(&transformed) {
                *current = intersection;
            } else {
                *current = Rect::EMPTY;
            }
        }
    }

    /// Clear the canvas with a color.
    pub fn clear(&mut self, color: Color) {
        self.buffer.clear(color);
    }

    /// Draw a color over the entire canvas.
    pub fn draw_color(&mut self, color: Color, blend_mode: BlendMode) {
        let matrix = *self.total_matrix();
        let clip = self.clip_bounds();
        let width = self.width();
        let height = self.height();

        let mut rasterizer = crate::raster::Rasterizer::new(self.buffer);
        rasterizer.set_matrix(&matrix);
        rasterizer.set_clip(clip);

        let mut paint = Paint::new();
        paint.set_color32(color);
        paint.set_blend_mode(blend_mode);

        let rect = Rect::from_xywh(0.0, 0.0, width as Scalar, height as Scalar);
        rasterizer.fill_rect(&rect, &paint);
    }

    /// Draw a point.
    pub fn draw_point(&mut self, point: Point, paint: &Paint) {
        let matrix = *self.total_matrix();
        let clip = self.clip_bounds();

        let mut rasterizer = crate::raster::Rasterizer::new(self.buffer);
        rasterizer.set_matrix(&matrix);
        rasterizer.set_clip(clip);
        rasterizer.draw_point(point, paint);
    }

    /// Draw a line.
    pub fn draw_line(&mut self, p0: Point, p1: Point, paint: &Paint) {
        let matrix = *self.total_matrix();
        let clip = self.clip_bounds();

        let mut rasterizer = crate::raster::Rasterizer::new(self.buffer);
        rasterizer.set_matrix(&matrix);
        rasterizer.set_clip(clip);
        rasterizer.draw_line(p0, p1, paint);
    }

    /// Draw a rectangle.
    pub fn draw_rect(&mut self, rect: &Rect, paint: &Paint) {
        let matrix = *self.total_matrix();
        let clip = self.clip_bounds();

        let mut rasterizer = crate::raster::Rasterizer::new(self.buffer);
        rasterizer.set_matrix(&matrix);
        rasterizer.set_clip(clip);
        rasterizer.draw_rect(rect, paint);
    }

    /// Draw an oval.
    pub fn draw_oval(&mut self, rect: &Rect, paint: &Paint) {
        let matrix = *self.total_matrix();
        let clip = self.clip_bounds();

        let mut rasterizer = crate::raster::Rasterizer::new(self.buffer);
        rasterizer.set_matrix(&matrix);
        rasterizer.set_clip(clip);
        rasterizer.draw_oval(rect, paint);
    }

    /// Draw a circle.
    pub fn draw_circle(&mut self, center: Point, radius: Scalar, paint: &Paint) {
        let matrix = *self.total_matrix();
        let clip = self.clip_bounds();

        let mut rasterizer = crate::raster::Rasterizer::new(self.buffer);
        rasterizer.set_matrix(&matrix);
        rasterizer.set_clip(clip);
        rasterizer.draw_circle(center, radius, paint);
    }

    /// Draw a rounded rectangle.
    pub fn draw_round_rect(&mut self, rect: &Rect, rx: Scalar, ry: Scalar, paint: &Paint) {
        // Approximate with path
        use skia_rs_path::PathBuilder;

        let mut builder = PathBuilder::new();

        // Start at top-left after corner
        builder.move_to(rect.left + rx, rect.top);

        // Top edge
        builder.line_to(rect.right - rx, rect.top);
        // Top-right corner
        builder.quad_to(rect.right, rect.top, rect.right, rect.top + ry);

        // Right edge
        builder.line_to(rect.right, rect.bottom - ry);
        // Bottom-right corner
        builder.quad_to(rect.right, rect.bottom, rect.right - rx, rect.bottom);

        // Bottom edge
        builder.line_to(rect.left + rx, rect.bottom);
        // Bottom-left corner
        builder.quad_to(rect.left, rect.bottom, rect.left, rect.bottom - ry);

        // Left edge
        builder.line_to(rect.left, rect.top + ry);
        // Top-left corner
        builder.quad_to(rect.left, rect.top, rect.left + rx, rect.top);

        builder.close();
        let path = builder.build();

        self.draw_path(&path, paint);
    }

    /// Draw a path.
    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        let matrix = *self.total_matrix();
        let clip = self.clip_bounds();

        let mut rasterizer = crate::raster::Rasterizer::new(self.buffer);
        rasterizer.set_matrix(&matrix);
        rasterizer.set_clip(clip);
        rasterizer.draw_path(path, paint);
    }

    /// Draw an arc.
    pub fn draw_arc(
        &mut self,
        oval: &Rect,
        start_angle: Scalar,
        sweep_angle: Scalar,
        use_center: bool,
        paint: &Paint,
    ) {
        use skia_rs_path::PathBuilder;

        let center = Point::new(
            (oval.left + oval.right) / 2.0,
            (oval.top + oval.bottom) / 2.0,
        );
        let rx = oval.width() / 2.0;
        let ry = oval.height() / 2.0;

        let start_rad = start_angle.to_radians();
        let end_rad = (start_angle + sweep_angle).to_radians();

        let start_x = center.x + rx * start_rad.cos();
        let start_y = center.y + ry * start_rad.sin();

        let mut builder = PathBuilder::new();

        if use_center {
            builder.move_to(center.x, center.y);
            builder.line_to(start_x, start_y);
        } else {
            builder.move_to(start_x, start_y);
        }

        // Approximate arc with line segments
        let steps = ((sweep_angle.abs() / 10.0).ceil() as usize).max(4);
        for i in 1..=steps {
            let t = i as Scalar / steps as Scalar;
            let angle = start_rad + (end_rad - start_rad) * t;
            let x = center.x + rx * angle.cos();
            let y = center.y + ry * angle.sin();
            builder.line_to(x, y);
        }

        if use_center {
            builder.close();
        }

        let path = builder.build();
        self.draw_path(&path, paint);
    }

    /// Draw an image at the specified position.
    #[cfg(feature = "codec")]
    pub fn draw_image(&mut self, image: &Image, left: Scalar, top: Scalar, paint: Option<&Paint>) {
        let src_rect = IRect::new(0, 0, image.width(), image.height());
        let dst_rect =
            Rect::from_xywh(left, top, image.width() as Scalar, image.height() as Scalar);
        self.draw_image_rect(image, Some(&src_rect), &dst_rect, paint);
    }

    /// Draw an image with source and destination rectangles.
    #[cfg(feature = "codec")]
    pub fn draw_image_rect(
        &mut self,
        image: &Image,
        src: Option<&IRect>,
        dst: &Rect,
        paint: Option<&Paint>,
    ) {
        let src_rect = src
            .cloned()
            .unwrap_or_else(|| IRect::new(0, 0, image.width(), image.height()));

        // Apply current transformation to destination
        let matrix = *self.total_matrix();
        let transformed_dst = matrix.map_rect(dst);

        // Get the clip bounds
        let clip = self.clip_bounds();

        // Calculate intersection with clip
        let visible_dst = match transformed_dst.intersect(&clip) {
            Some(r) => r,
            None => return, // Completely clipped
        };

        // Calculate scale factors
        let scale_x = (src_rect.width() as Scalar) / dst.width();
        let scale_y = (src_rect.height() as Scalar) / dst.height();

        // Blend mode from paint
        let blend_mode = paint
            .map(|p| p.blend_mode())
            .unwrap_or(skia_rs_paint::BlendMode::SrcOver);
        let alpha = paint.map(|p| p.alpha()).unwrap_or(1.0);

        // Iterate over destination pixels
        let dst_x_start = visible_dst.left.floor() as i32;
        let dst_x_end = visible_dst.right.ceil() as i32;
        let dst_y_start = visible_dst.top.floor() as i32;
        let dst_y_end = visible_dst.bottom.ceil() as i32;

        for dst_y in dst_y_start..dst_y_end {
            for dst_x in dst_x_start..dst_x_end {
                // Calculate source coordinates
                let rel_x = (dst_x as Scalar - transformed_dst.left) * scale_x;
                let rel_y = (dst_y as Scalar - transformed_dst.top) * scale_y;

                let src_x = (src_rect.left as Scalar + rel_x) as i32;
                let src_y = (src_rect.top as Scalar + rel_y) as i32;

                // Bounds check
                if src_x < 0 || src_x >= image.width() || src_y < 0 || src_y >= image.height() {
                    continue;
                }

                // Get source pixel
                if let Some(src_color) = image.read_pixel(src_x, src_y) {
                    let mut color = Color::from_argb(
                        (src_color.a * alpha * 255.0) as u8,
                        (src_color.r * 255.0) as u8,
                        (src_color.g * 255.0) as u8,
                        (src_color.b * 255.0) as u8,
                    );

                    // Apply alpha
                    if alpha < 1.0 {
                        let a = (color.alpha() as f32 * alpha) as u8;
                        color = Color::from_argb(a, color.red(), color.green(), color.blue());
                    }

                    self.buffer.blend_pixel(dst_x, dst_y, color, blend_mode);
                }
            }
        }
    }

    /// Draw an image with nine-patch stretching.
    #[cfg(feature = "codec")]
    pub fn draw_image_nine(
        &mut self,
        image: &Image,
        center: &IRect,
        dst: &Rect,
        paint: Option<&Paint>,
    ) {
        let img_w = image.width();
        let img_h = image.height();

        // Calculate the nine regions
        let left_w = center.left as Scalar;
        let right_w = (img_w - center.right) as Scalar;
        let top_h = center.top as Scalar;
        let bottom_h = (img_h - center.bottom) as Scalar;

        let center_w = dst.width() - left_w - right_w;
        let center_h = dst.height() - top_h - bottom_h;

        // Top-left corner
        self.draw_image_rect(
            image,
            Some(&IRect::new(0, 0, center.left, center.top)),
            &Rect::from_xywh(dst.left, dst.top, left_w, top_h),
            paint,
        );

        // Top edge (stretched)
        self.draw_image_rect(
            image,
            Some(&IRect::new(center.left, 0, center.right, center.top)),
            &Rect::from_xywh(dst.left + left_w, dst.top, center_w, top_h),
            paint,
        );

        // Top-right corner
        self.draw_image_rect(
            image,
            Some(&IRect::new(center.right, 0, img_w, center.top)),
            &Rect::from_xywh(dst.right - right_w, dst.top, right_w, top_h),
            paint,
        );

        // Left edge (stretched)
        self.draw_image_rect(
            image,
            Some(&IRect::new(0, center.top, center.left, center.bottom)),
            &Rect::from_xywh(dst.left, dst.top + top_h, left_w, center_h),
            paint,
        );

        // Center (stretched both ways)
        self.draw_image_rect(
            image,
            Some(&IRect::new(
                center.left,
                center.top,
                center.right,
                center.bottom,
            )),
            &Rect::from_xywh(dst.left + left_w, dst.top + top_h, center_w, center_h),
            paint,
        );

        // Right edge (stretched)
        self.draw_image_rect(
            image,
            Some(&IRect::new(center.right, center.top, img_w, center.bottom)),
            &Rect::from_xywh(dst.right - right_w, dst.top + top_h, right_w, center_h),
            paint,
        );

        // Bottom-left corner
        self.draw_image_rect(
            image,
            Some(&IRect::new(0, center.bottom, center.left, img_h)),
            &Rect::from_xywh(dst.left, dst.bottom - bottom_h, left_w, bottom_h),
            paint,
        );

        // Bottom edge (stretched)
        self.draw_image_rect(
            image,
            Some(&IRect::new(center.left, center.bottom, center.right, img_h)),
            &Rect::from_xywh(dst.left + left_w, dst.bottom - bottom_h, center_w, bottom_h),
            paint,
        );

        // Bottom-right corner
        self.draw_image_rect(
            image,
            Some(&IRect::new(center.right, center.bottom, img_w, img_h)),
            &Rect::from_xywh(
                dst.right - right_w,
                dst.bottom - bottom_h,
                right_w,
                bottom_h,
            ),
            paint,
        );
    }

    /// Draw a region.
    pub fn draw_region(&mut self, region: &Region, paint: &Paint) {
        // Draw each rectangle in the region
        for rect in region.iter() {
            let rect_f = rect.to_rect();
            self.draw_rect(&rect_f, paint);
        }
    }

    /// Draw vertices (triangles).
    pub fn draw_vertices(
        &mut self,
        mode: VertexMode,
        positions: &[Point],
        colors: Option<&[Color]>,
        paint: &Paint,
    ) {
        if positions.len() < 3 {
            return;
        }

        let matrix = *self.total_matrix();

        match mode {
            VertexMode::Triangles => {
                // Draw triangles (every 3 vertices)
                for chunk in positions.chunks(3) {
                    if chunk.len() == 3 {
                        self.draw_triangle(
                            matrix.map_point(chunk[0]),
                            matrix.map_point(chunk[1]),
                            matrix.map_point(chunk[2]),
                            colors.and_then(|c| c.first().copied()),
                            paint,
                        );
                    }
                }
            }
            VertexMode::TriangleStrip => {
                // Triangle strip
                for i in 0..positions.len().saturating_sub(2) {
                    let (p0, p1, p2) = if i % 2 == 0 {
                        (positions[i], positions[i + 1], positions[i + 2])
                    } else {
                        (positions[i + 1], positions[i], positions[i + 2])
                    };
                    self.draw_triangle(
                        matrix.map_point(p0),
                        matrix.map_point(p1),
                        matrix.map_point(p2),
                        colors.and_then(|c| c.get(i).copied()),
                        paint,
                    );
                }
            }
            VertexMode::TriangleFan => {
                // Triangle fan
                let center = positions[0];
                for i in 1..positions.len().saturating_sub(1) {
                    self.draw_triangle(
                        matrix.map_point(center),
                        matrix.map_point(positions[i]),
                        matrix.map_point(positions[i + 1]),
                        colors.and_then(|c| c.get(i).copied()),
                        paint,
                    );
                }
            }
        }
    }

    /// Draw a single filled triangle.
    fn draw_triangle(
        &mut self,
        p0: Point,
        p1: Point,
        p2: Point,
        color: Option<Color>,
        paint: &Paint,
    ) {
        // Use scanline algorithm for triangle fill
        let color = color.unwrap_or_else(|| paint.color32());
        let blend_mode = paint.blend_mode();

        // Sort vertices by y coordinate
        let mut verts = [(p0.x, p0.y), (p1.x, p1.y), (p2.x, p2.y)];
        verts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let (x0, y0) = verts[0];
        let (x1, y1) = verts[1];
        let (x2, y2) = verts[2];

        // Calculate edge slopes
        let inv_slope_02 = if (y2 - y0).abs() > 0.001 {
            (x2 - x0) / (y2 - y0)
        } else {
            0.0
        };

        let inv_slope_01 = if (y1 - y0).abs() > 0.001 {
            (x1 - x0) / (y1 - y0)
        } else {
            0.0
        };

        let inv_slope_12 = if (y2 - y1).abs() > 0.001 {
            (x2 - x1) / (y2 - y1)
        } else {
            0.0
        };

        // Scan from y0 to y1
        let y_start = y0.ceil() as i32;
        let y_mid = y1.ceil() as i32;
        let y_end = y2.ceil() as i32;

        for y in y_start..y_mid {
            let t = (y as Scalar - y0) / (y1 - y0).max(0.001);
            let x_left = x0 + (y as Scalar - y0) * inv_slope_02;
            let x_right = x0 + (y as Scalar - y0) * inv_slope_01;

            let (xa, xb) = if x_left < x_right {
                (x_left, x_right)
            } else {
                (x_right, x_left)
            };

            for x in (xa.ceil() as i32)..(xb.floor() as i32) {
                self.buffer.blend_pixel(x, y, color, blend_mode);
            }
        }

        // Scan from y1 to y2
        for y in y_mid..y_end {
            let x_left = x0 + (y as Scalar - y0) * inv_slope_02;
            let x_right = x1 + (y as Scalar - y1) * inv_slope_12;

            let (xa, xb) = if x_left < x_right {
                (x_left, x_right)
            } else {
                (x_right, x_left)
            };

            for x in (xa.ceil() as i32)..(xb.floor() as i32) {
                self.buffer.blend_pixel(x, y, color, blend_mode);
            }
        }
    }

    /// Draw text at the specified position.
    #[cfg(feature = "text")]
    pub fn draw_string(
        &mut self,
        text: &str,
        x: Scalar,
        y: Scalar,
        font: &skia_rs_text::Font,
        paint: &Paint,
    ) {
        // Simple text rendering - just draw each character as a rectangle placeholder
        // A real implementation would use glyph outlines from the font
        let color = paint.color32();
        let blend_mode = paint.blend_mode();
        let matrix = *self.total_matrix();

        let char_width = font.size() * 0.5;
        let char_height = font.size();
        let mut current_x = x;

        for _ch in text.chars() {
            // Transform position
            let pos = matrix.map_point(Point::new(current_x, y - char_height * 0.8));

            // Draw a simple rectangle for each character (placeholder)
            let rect = Rect::from_xywh(
                pos.x,
                pos.y,
                char_width * matrix.scale_x().abs(),
                char_height * matrix.scale_y().abs(),
            );

            if let Some(clipped) = rect.intersect(&self.clip_bounds()) {
                let r = clipped.round_out();
                for py in r.top..r.bottom {
                    for px in r.left..r.right {
                        self.buffer.blend_pixel(px, py, color, blend_mode);
                    }
                }
            }

            current_x += char_width;
        }
    }

    /// Draw a text blob.
    #[cfg(feature = "text")]
    pub fn draw_text_blob(
        &mut self,
        blob: &skia_rs_text::TextBlob,
        x: Scalar,
        y: Scalar,
        paint: &Paint,
    ) {
        let color = paint.color32();
        let blend_mode = paint.blend_mode();
        let matrix = *self.total_matrix();

        for run in blob.runs() {
            let font = &run.font;
            let char_width = font.size() * 0.5;
            let char_height = font.size();

            for (i, &glyph) in run.glyphs.iter().enumerate() {
                if glyph == 0 {
                    continue; // Skip .notdef glyph
                }

                let pos = if i < run.positions.len() {
                    run.positions[i]
                } else {
                    Point::new(i as Scalar * char_width, 0.0)
                };

                let world_pos = matrix.map_point(Point::new(
                    x + run.origin.x + pos.x,
                    y + run.origin.y + pos.y - char_height * 0.8,
                ));

                // Draw glyph as rectangle (placeholder)
                let rect = Rect::from_xywh(
                    world_pos.x,
                    world_pos.y,
                    char_width * matrix.scale_x().abs(),
                    char_height * matrix.scale_y().abs(),
                );

                if let Some(clipped) = rect.intersect(&self.clip_bounds()) {
                    let r = clipped.round_out();
                    for py in r.top..r.bottom {
                        for px in r.left..r.right {
                            self.buffer.blend_pixel(px, py, color, blend_mode);
                        }
                    }
                }
            }
        }
    }
}

/// Vertex drawing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum VertexMode {
    /// Separate triangles (every 3 vertices).
    #[default]
    Triangles = 0,
    /// Triangle strip (shared edges).
    TriangleStrip,
    /// Triangle fan (shared center vertex).
    TriangleFan,
}

#[cfg(test)]
mod tests {
    use super::*;
    use skia_rs_core::{AlphaType, ColorType};
    use skia_rs_paint::Style;

    #[test]
    fn test_surface_new_raster() {
        let info = ImageInfo::new(100, 100, ColorType::Rgba8888, AlphaType::Premul).unwrap();
        let surface = Surface::new_raster(&info, None).unwrap();
        assert_eq!(surface.width(), 100);
        assert_eq!(surface.height(), 100);
    }

    #[test]
    fn test_surface_new_raster_n32() {
        let surface = Surface::new_raster_n32_premul(200, 150).unwrap();
        assert_eq!(surface.width(), 200);
        assert_eq!(surface.height(), 150);
    }

    #[test]
    fn test_raster_canvas_clear() {
        let mut surface = Surface::new_raster_n32_premul(100, 100).unwrap();
        {
            let mut canvas = surface.raster_canvas();
            canvas.clear(Color::from_argb(255, 255, 0, 0));
        }

        // Check that pixels are red
        let pixels = surface.pixels();
        assert_eq!(pixels[0], 255); // R
        assert_eq!(pixels[1], 0); // G
        assert_eq!(pixels[2], 0); // B
        assert_eq!(pixels[3], 255); // A
    }

    #[test]
    fn test_raster_canvas_draw_rect() {
        let mut surface = Surface::new_raster_n32_premul(100, 100).unwrap();
        {
            let mut canvas = surface.raster_canvas();
            canvas.clear(Color::from_argb(255, 255, 255, 255));

            let mut paint = Paint::new();
            paint.set_color32(Color::from_argb(255, 0, 0, 255));
            paint.set_style(Style::Fill);

            canvas.draw_rect(&Rect::from_xywh(10.0, 10.0, 30.0, 30.0), &paint);
        }

        // Check a pixel inside the rect (at 20, 20)
        let buffer = surface.pixel_buffer();
        let pixel = buffer.get_pixel(20, 20).unwrap();
        assert_eq!(pixel.blue(), 255);
    }

    #[test]
    fn test_raster_canvas_draw_circle() {
        let mut surface = Surface::new_raster_n32_premul(100, 100).unwrap();
        {
            let mut canvas = surface.raster_canvas();
            canvas.clear(Color::from_argb(255, 255, 255, 255));

            let mut paint = Paint::new();
            paint.set_color32(Color::from_argb(255, 0, 255, 0));
            paint.set_style(Style::Fill);

            canvas.draw_circle(Point::new(50.0, 50.0), 20.0, &paint);
        }

        // Check center pixel
        let buffer = surface.pixel_buffer();
        let pixel = buffer.get_pixel(50, 50).unwrap();
        assert_eq!(pixel.green(), 255);
    }

    #[test]
    fn test_raster_canvas_transform() {
        let mut surface = Surface::new_raster_n32_premul(100, 100).unwrap();
        {
            let mut canvas = surface.raster_canvas();
            canvas.clear(Color::from_argb(255, 0, 0, 0));

            canvas.translate(50.0, 50.0);

            let mut paint = Paint::new();
            paint.set_color32(Color::from_argb(255, 255, 0, 0));
            paint.set_style(Style::Fill);

            // This rect at (0,0) with size 10x10 should appear at (50,50)
            canvas.draw_rect(&Rect::from_xywh(0.0, 0.0, 10.0, 10.0), &paint);
        }

        // Check pixel at 55, 55 (inside transformed rect)
        let buffer = surface.pixel_buffer();
        let pixel = buffer.get_pixel(55, 55).unwrap();
        assert_eq!(pixel.red(), 255);
    }
}
