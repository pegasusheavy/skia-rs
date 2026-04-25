//! Surface backing store for canvas.

use crate::Canvas;
use crate::raster::PixelBuffer;
#[cfg(feature = "codec")]
use skia_rs_codec::Image;
use skia_rs_core::pixel::{ImageInfo, SurfaceProps};
#[cfg(feature = "codec")]
use skia_rs_core::{AlphaType, ColorType, IRect};

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

    /// Get a canvas for drawing.
    ///
    /// The returned canvas renders directly into this surface's pixel
    /// buffer. Borrowing is mutable because every draw mutates pixels.
    pub fn canvas(&mut self) -> Canvas<'_> {
        Canvas::new_raster(&mut self.buffer)
    }

    /// Get a raster canvas that can actually draw pixels.
    ///
    /// Historically this was the only working canvas on `Surface`. It now
    /// returns the same [`Canvas`] as [`canvas`](Self::canvas); the alias is
    /// kept for source compatibility.
    #[allow(deprecated)]
    pub fn raster_canvas(&mut self) -> RasterCanvas<'_> {
        Canvas::new_raster(&mut self.buffer)
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

// =============================================================================
// Back-compat RasterCanvas alias.
//
// The old split between `Canvas` (stub) and `RasterCanvas` (real) has been
// unified into `Canvas<'a>` with a `Backing` enum. `RasterCanvas` is kept as
// an alias so existing callers keep building. Prefer `Canvas` in new code.
// =============================================================================

/// A raster-backed canvas.
///
/// Alias for [`Canvas`] with a raster [`Backing`](crate::Backing). Prefer
/// using [`Canvas`] directly.
///
/// [`Canvas`]: crate::Canvas
#[deprecated(
    since = "0.2.4",
    note = "Use `skia_rs_canvas::Canvas` directly; obtain one via `Surface::canvas` or `Canvas::new_raster`"
)]
pub type RasterCanvas<'a> = crate::Canvas<'a>;

#[cfg(test)]
mod tests {
    use super::*;
    use skia_rs_core::{AlphaType, Color, ColorType, Point, Rect};
    use skia_rs_paint::{Paint, Style};

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
            let mut canvas = surface.canvas();
            canvas.clear(Color::from_argb(255, 255, 0, 0));
        }

        let pixels = surface.pixels();
        assert_eq!(pixels[0], 255);
        assert_eq!(pixels[1], 0);
        assert_eq!(pixels[2], 0);
        assert_eq!(pixels[3], 255);
    }

    #[test]
    fn test_raster_canvas_draw_rect() {
        let mut surface = Surface::new_raster_n32_premul(100, 100).unwrap();
        {
            let mut canvas = surface.canvas();
            canvas.clear(Color::from_argb(255, 255, 255, 255));

            let mut paint = Paint::new();
            paint.set_color32(Color::from_argb(255, 0, 0, 255));
            paint.set_style(Style::Fill);

            canvas.draw_rect(&Rect::from_xywh(10.0, 10.0, 30.0, 30.0), &paint);
        }

        let buffer = surface.pixel_buffer();
        let pixel = buffer.get_pixel(20, 20).unwrap();
        assert_eq!(pixel.blue(), 255);
    }

    #[test]
    fn test_raster_canvas_draw_circle() {
        let mut surface = Surface::new_raster_n32_premul(100, 100).unwrap();
        {
            let mut canvas = surface.canvas();
            canvas.clear(Color::from_argb(255, 255, 255, 255));

            let mut paint = Paint::new();
            paint.set_color32(Color::from_argb(255, 0, 255, 0));
            paint.set_style(Style::Fill);

            canvas.draw_circle(Point::new(50.0, 50.0), 20.0, &paint);
        }

        let buffer = surface.pixel_buffer();
        let pixel = buffer.get_pixel(50, 50).unwrap();
        assert_eq!(pixel.green(), 255);
    }

    #[test]
    fn test_raster_canvas_transform() {
        let mut surface = Surface::new_raster_n32_premul(100, 100).unwrap();
        {
            let mut canvas = surface.canvas();
            canvas.clear(Color::from_argb(255, 0, 0, 0));

            canvas.translate(50.0, 50.0);

            let mut paint = Paint::new();
            paint.set_color32(Color::from_argb(255, 255, 0, 0));
            paint.set_style(Style::Fill);

            canvas.draw_rect(&Rect::from_xywh(0.0, 0.0, 10.0, 10.0), &paint);
        }

        let buffer = surface.pixel_buffer();
        let pixel = buffer.get_pixel(55, 55).unwrap();
        assert_eq!(pixel.red(), 255);
    }

    #[test]
    fn test_surface_canvas_returns_functional_raster() {
        // `Surface::canvas` must return a canvas that actually writes pixels.
        let mut surface = Surface::new_raster_n32_premul(10, 10).unwrap();
        {
            let mut canvas = surface.canvas();
            canvas.clear(Color::from_argb(255, 0, 255, 255));
        }
        let pixel = surface.pixel_buffer().get_pixel(5, 5).unwrap();
        assert_eq!(pixel.green(), 255);
        assert_eq!(pixel.blue(), 255);
    }
}
