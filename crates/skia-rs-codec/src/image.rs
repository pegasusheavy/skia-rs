//! Image type for immutable pixel data.
//!
//! Images represent immutable pixel data that can be drawn to a canvas.

use skia_rs_core::{AlphaType, ColorSpace, ColorType, Rect, Scalar};
use std::sync::Arc;

/// Simplified image info for codec use (avoids Result-based construction).
#[derive(Debug, Clone, PartialEq)]
pub struct ImageInfo {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
    /// Color type.
    pub color_type: ColorType,
    /// Alpha type.
    pub alpha_type: AlphaType,
    /// Color space.
    pub color_space: Option<ColorSpace>,
}

impl ImageInfo {
    /// Create a new image info.
    pub fn new(width: i32, height: i32, color_type: ColorType, alpha_type: AlphaType) -> Self {
        Self {
            width,
            height,
            color_type,
            alpha_type,
            color_space: None,
        }
    }

    /// Get the width.
    #[inline]
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Get the height.
    #[inline]
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Get the color type.
    #[inline]
    pub fn color_type(&self) -> ColorType {
        self.color_type
    }

    /// Get the alpha type.
    #[inline]
    pub fn alpha_type(&self) -> AlphaType {
        self.alpha_type
    }

    /// Get the color space.
    #[inline]
    pub fn color_space(&self) -> Option<&ColorSpace> {
        self.color_space.as_ref()
    }

    /// Returns true if dimensions are zero or negative.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    /// Returns true if alpha is opaque.
    #[inline]
    pub fn is_opaque(&self) -> bool {
        self.alpha_type == AlphaType::Opaque
    }

    /// Bytes per pixel.
    #[inline]
    pub fn bytes_per_pixel(&self) -> usize {
        self.color_type.bytes_per_pixel()
    }

    /// Minimum row bytes.
    #[inline]
    pub fn min_row_bytes(&self) -> usize {
        self.width as usize * self.bytes_per_pixel()
    }

    /// Compute byte size for given row bytes.
    #[inline]
    pub fn compute_byte_size(&self, row_bytes: usize) -> usize {
        if self.height <= 0 {
            return 0;
        }
        let last_row = self.min_row_bytes();
        let other_rows = row_bytes * (self.height as usize - 1);
        other_rows + last_row
    }
}

/// An immutable image.
///
/// Images are the immutable counterpart to Bitmap. Once created, an Image's
/// pixels cannot be changed. Images can be created from:
/// - Pixel data (raster images)
/// - GPU textures
/// - Other images (subsets, scaling)
/// - Encoded data (PNG, JPEG, etc.)
///
/// Corresponds to Skia's `SkImage`.
#[derive(Clone)]
pub struct Image {
    inner: Arc<ImageData>,
}

struct ImageData {
    info: ImageInfo,
    pixels: Vec<u8>,
    row_bytes: usize,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("color_type", &self.color_type())
            .field("alpha_type", &self.alpha_type())
            .finish()
    }
}

impl Image {
    /// Create an image from raw pixel data.
    ///
    /// The pixels are copied into the image.
    pub fn from_raster_data(info: &ImageInfo, pixels: &[u8], row_bytes: usize) -> Option<Self> {
        if info.is_empty() {
            return None;
        }

        let expected_size = info.compute_byte_size(row_bytes);
        if pixels.len() < expected_size {
            return None;
        }

        Some(Self {
            inner: Arc::new(ImageData {
                info: info.clone(),
                pixels: pixels[..expected_size].to_vec(),
                row_bytes,
            }),
        })
    }

    /// Create an image from owned pixel data.
    pub fn from_raster_data_owned(
        info: ImageInfo,
        pixels: Vec<u8>,
        row_bytes: usize,
    ) -> Option<Self> {
        if info.is_empty() {
            return None;
        }

        let expected_size = info.compute_byte_size(row_bytes);
        if pixels.len() < expected_size {
            return None;
        }

        Some(Self {
            inner: Arc::new(ImageData {
                info,
                pixels,
                row_bytes,
            }),
        })
    }

    /// Create a new RGBA image filled with a color.
    pub fn from_color(width: i32, height: i32, color: u32) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }

        let info = ImageInfo::new(width, height, ColorType::Rgba8888, AlphaType::Premul);
        let row_bytes = (width as usize) * 4;
        let mut pixels = vec![0u8; (height as usize) * row_bytes];

        // Split color into RGBA bytes
        let r = ((color >> 16) & 0xFF) as u8;
        let g = ((color >> 8) & 0xFF) as u8;
        let b = (color & 0xFF) as u8;
        let a = ((color >> 24) & 0xFF) as u8;

        for y in 0..height as usize {
            for x in 0..width as usize {
                let offset = y * row_bytes + x * 4;
                pixels[offset] = r;
                pixels[offset + 1] = g;
                pixels[offset + 2] = b;
                pixels[offset + 3] = a;
            }
        }

        Self::from_raster_data_owned(info, pixels, row_bytes)
    }

    /// Get the image width.
    #[inline]
    pub fn width(&self) -> i32 {
        self.inner.info.width()
    }

    /// Get the image height.
    #[inline]
    pub fn height(&self) -> i32 {
        self.inner.info.height()
    }

    /// Get the image dimensions as (width, height).
    #[inline]
    pub fn dimensions(&self) -> (i32, i32) {
        (self.width(), self.height())
    }

    /// Get the image bounds as a rectangle.
    #[inline]
    pub fn bounds(&self) -> Rect {
        Rect::from_xywh(0.0, 0.0, self.width() as Scalar, self.height() as Scalar)
    }

    /// Get the image info.
    #[inline]
    pub fn info(&self) -> &ImageInfo {
        &self.inner.info
    }

    /// Get the color type.
    #[inline]
    pub fn color_type(&self) -> ColorType {
        self.inner.info.color_type()
    }

    /// Get the alpha type.
    #[inline]
    pub fn alpha_type(&self) -> AlphaType {
        self.inner.info.alpha_type()
    }

    /// Get the color space.
    #[inline]
    pub fn color_space(&self) -> Option<&ColorSpace> {
        self.inner.info.color_space()
    }

    /// Returns true if the image is opaque.
    #[inline]
    pub fn is_opaque(&self) -> bool {
        self.inner.info.is_opaque()
    }

    /// Get the row bytes (stride).
    #[inline]
    pub fn row_bytes(&self) -> usize {
        self.inner.row_bytes
    }

    /// Get the unique ID for this image.
    #[inline]
    pub fn unique_id(&self) -> usize {
        Arc::as_ptr(&self.inner) as usize
    }

    /// Read pixels from the image into a buffer.
    pub fn read_pixels(
        &self,
        dst_info: &ImageInfo,
        dst_pixels: &mut [u8],
        dst_row_bytes: usize,
        src_x: i32,
        src_y: i32,
    ) -> bool {
        // Validate bounds
        if src_x < 0 || src_y < 0 {
            return false;
        }
        if src_x + dst_info.width() > self.width() {
            return false;
        }
        if src_y + dst_info.height() > self.height() {
            return false;
        }

        let dst_size = dst_info.compute_byte_size(dst_row_bytes);
        if dst_pixels.len() < dst_size {
            return false;
        }

        // Simple copy for matching formats
        if dst_info.color_type() == self.color_type() && dst_info.alpha_type() == self.alpha_type()
        {
            let bytes_per_pixel = self.color_type().bytes_per_pixel();
            let src_row_bytes = self.inner.row_bytes;

            for y in 0..dst_info.height() as usize {
                let src_offset =
                    (src_y as usize + y) * src_row_bytes + src_x as usize * bytes_per_pixel;
                let dst_offset = y * dst_row_bytes;
                let copy_len = dst_info.width() as usize * bytes_per_pixel;

                dst_pixels[dst_offset..dst_offset + copy_len]
                    .copy_from_slice(&self.inner.pixels[src_offset..src_offset + copy_len]);
            }

            return true;
        }

        // TODO: Format conversion
        false
    }

    /// Read a single pixel at (x, y).
    pub fn read_pixel(&self, x: i32, y: i32) -> Option<skia_rs_core::Color4f> {
        if x < 0 || x >= self.width() || y < 0 || y >= self.height() {
            return None;
        }

        let bytes_per_pixel = self.color_type().bytes_per_pixel();
        let offset = (y as usize) * self.inner.row_bytes + (x as usize) * bytes_per_pixel;

        match self.color_type() {
            ColorType::Rgba8888 => {
                let r = self.inner.pixels[offset] as f32 / 255.0;
                let g = self.inner.pixels[offset + 1] as f32 / 255.0;
                let b = self.inner.pixels[offset + 2] as f32 / 255.0;
                let a = self.inner.pixels[offset + 3] as f32 / 255.0;
                Some(skia_rs_core::Color4f::new(r, g, b, a))
            }
            ColorType::Bgra8888 => {
                let b = self.inner.pixels[offset] as f32 / 255.0;
                let g = self.inner.pixels[offset + 1] as f32 / 255.0;
                let r = self.inner.pixels[offset + 2] as f32 / 255.0;
                let a = self.inner.pixels[offset + 3] as f32 / 255.0;
                Some(skia_rs_core::Color4f::new(r, g, b, a))
            }
            ColorType::Alpha8 => {
                let a = self.inner.pixels[offset] as f32 / 255.0;
                Some(skia_rs_core::Color4f::new(0.0, 0.0, 0.0, a))
            }
            ColorType::Gray8 => {
                let v = self.inner.pixels[offset] as f32 / 255.0;
                Some(skia_rs_core::Color4f::new(v, v, v, 1.0))
            }
            _ => None,
        }
    }

    /// Get direct access to the pixel data (if available).
    pub fn peek_pixels(&self) -> Option<&[u8]> {
        Some(&self.inner.pixels)
    }

    /// Create a subset of this image.
    pub fn make_subset(&self, subset: &Rect) -> Option<Self> {
        let x = subset.left as i32;
        let y = subset.top as i32;
        let w = subset.width() as i32;
        let h = subset.height() as i32;

        if x < 0 || y < 0 || w <= 0 || h <= 0 {
            return None;
        }
        if x + w > self.width() || y + h > self.height() {
            return None;
        }

        let new_info = ImageInfo::new(w, h, self.color_type(), self.alpha_type());
        let bytes_per_pixel = self.color_type().bytes_per_pixel();
        let new_row_bytes = w as usize * bytes_per_pixel;
        let mut new_pixels = vec![0u8; (h as usize) * new_row_bytes];

        for row in 0..h as usize {
            let src_offset =
                (y as usize + row) * self.inner.row_bytes + x as usize * bytes_per_pixel;
            let dst_offset = row * new_row_bytes;

            new_pixels[dst_offset..dst_offset + new_row_bytes]
                .copy_from_slice(&self.inner.pixels[src_offset..src_offset + new_row_bytes]);
        }

        Self::from_raster_data_owned(new_info, new_pixels, new_row_bytes)
    }

    /// Create a scaled version of this image.
    pub fn make_scaled(&self, width: i32, height: i32) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }

        // Simple nearest-neighbor scaling
        let new_info = ImageInfo::new(width, height, self.color_type(), self.alpha_type());
        let bytes_per_pixel = self.color_type().bytes_per_pixel();
        let new_row_bytes = width as usize * bytes_per_pixel;
        let mut new_pixels = vec![0u8; (height as usize) * new_row_bytes];

        let x_scale = self.width() as f32 / width as f32;
        let y_scale = self.height() as f32 / height as f32;

        for dst_y in 0..height as usize {
            let src_y = ((dst_y as f32 * y_scale) as usize).min(self.height() as usize - 1);
            for dst_x in 0..width as usize {
                let src_x = ((dst_x as f32 * x_scale) as usize).min(self.width() as usize - 1);

                let src_offset = src_y * self.inner.row_bytes + src_x * bytes_per_pixel;
                let dst_offset = dst_y * new_row_bytes + dst_x * bytes_per_pixel;

                for i in 0..bytes_per_pixel {
                    new_pixels[dst_offset + i] = self.inner.pixels[src_offset + i];
                }
            }
        }

        Self::from_raster_data_owned(new_info, new_pixels, new_row_bytes)
    }

    /// Create a transformed version of this image.
    pub fn make_with_filter(&self) -> Option<Self> {
        // TODO: Implement matrix transformation
        Some(self.clone())
    }
}

/// A reference to an image (shared ownership).
pub type ImageRef = Arc<Image>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_from_color() {
        let image = Image::from_color(100, 100, 0xFF_FF0000).unwrap();
        assert_eq!(image.width(), 100);
        assert_eq!(image.height(), 100);
        assert_eq!(image.color_type(), ColorType::Rgba8888);
    }

    #[test]
    fn test_image_from_raster() {
        let info = ImageInfo::new(10, 10, ColorType::Rgba8888, AlphaType::Premul);
        let pixels = vec![0u8; 10 * 10 * 4];
        let image = Image::from_raster_data(&info, &pixels, 10 * 4).unwrap();
        assert_eq!(image.dimensions(), (10, 10));
    }

    #[test]
    fn test_image_subset() {
        let image = Image::from_color(100, 100, 0xFF_FF0000).unwrap();
        let subset = image
            .make_subset(&Rect::from_xywh(25.0, 25.0, 50.0, 50.0))
            .unwrap();
        assert_eq!(subset.dimensions(), (50, 50));
    }

    #[test]
    fn test_image_scaled() {
        let image = Image::from_color(100, 100, 0xFF_FF0000).unwrap();
        let scaled = image.make_scaled(50, 50).unwrap();
        assert_eq!(scaled.dimensions(), (50, 50));
    }

    #[test]
    fn test_image_bounds() {
        let image = Image::from_color(100, 200, 0xFF_000000).unwrap();
        let bounds = image.bounds();
        assert_eq!(bounds.width(), 100.0);
        assert_eq!(bounds.height(), 200.0);
    }
}
