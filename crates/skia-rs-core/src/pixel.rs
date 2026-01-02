//! Pixel formats and image storage.

use crate::color::{AlphaType, ColorSpace, ColorType};
use crate::geometry::{IRect, ISize};
use bitflags::bitflags;
use thiserror::Error;

// =============================================================================
// Errors
// =============================================================================

/// Errors related to pixel operations.
#[derive(Debug, Error)]
pub enum PixelError {
    /// Invalid image dimensions.
    #[error("invalid dimensions: {width}x{height}")]
    InvalidDimensions {
        /// Width.
        width: i32,
        /// Height.
        height: i32,
    },

    /// Row bytes too small for the width.
    #[error("row bytes {row_bytes} too small for width {width} with {bpp} bytes per pixel")]
    RowBytesTooSmall {
        /// Provided row bytes.
        row_bytes: usize,
        /// Image width.
        width: i32,
        /// Bytes per pixel.
        bpp: usize,
    },

    /// Buffer too small.
    #[error("buffer size {actual} too small, need {required}")]
    BufferTooSmall {
        /// Required size.
        required: usize,
        /// Actual size.
        actual: usize,
    },

    /// Unsupported color type.
    #[error("unsupported color type")]
    UnsupportedColorType,
}

// =============================================================================
// Image Info
// =============================================================================

/// Describes the properties of pixel data.
///
/// Equivalent to Skia's `SkImageInfo`.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageInfo {
    /// Image dimensions.
    pub dimensions: ISize,
    /// Pixel format.
    pub color_type: ColorType,
    /// Alpha interpretation.
    pub alpha_type: AlphaType,
    /// Color space (optional).
    pub color_space: Option<ColorSpace>,
}

impl ImageInfo {
    /// Creates a new image info with the specified properties.
    #[inline]
    pub fn new(
        width: i32,
        height: i32,
        color_type: ColorType,
        alpha_type: AlphaType,
    ) -> Result<Self, PixelError> {
        if width <= 0 || height <= 0 {
            return Err(PixelError::InvalidDimensions { width, height });
        }
        Ok(Self {
            dimensions: ISize::new(width, height),
            color_type,
            alpha_type,
            color_space: None,
        })
    }

    /// Creates image info with sRGB color space.
    #[inline]
    pub fn new_srgb(
        width: i32,
        height: i32,
        color_type: ColorType,
        alpha_type: AlphaType,
    ) -> Result<Self, PixelError> {
        let mut info = Self::new(width, height, color_type, alpha_type)?;
        info.color_space = Some(ColorSpace::srgb());
        Ok(info)
    }

    /// Creates RGBA 8888 image info.
    #[inline]
    pub fn new_rgba8888(
        width: i32,
        height: i32,
        alpha_type: AlphaType,
    ) -> Result<Self, PixelError> {
        Self::new(width, height, ColorType::Rgba8888, alpha_type)
    }

    /// Creates BGRA 8888 image info (native on little-endian).
    #[inline]
    pub fn new_bgra8888(
        width: i32,
        height: i32,
        alpha_type: AlphaType,
    ) -> Result<Self, PixelError> {
        Self::new(width, height, ColorType::Bgra8888, alpha_type)
    }

    /// Creates native 32-bit image info.
    #[inline]
    pub fn new_n32(width: i32, height: i32, alpha_type: AlphaType) -> Result<Self, PixelError> {
        Self::new(width, height, ColorType::n32(), alpha_type)
    }

    /// Creates opaque native 32-bit image info.
    #[inline]
    pub fn new_n32_opaque(width: i32, height: i32) -> Result<Self, PixelError> {
        Self::new_n32(width, height, AlphaType::Opaque)
    }

    /// Creates premultiplied native 32-bit image info.
    #[inline]
    pub fn new_n32_premul(width: i32, height: i32) -> Result<Self, PixelError> {
        Self::new_n32(width, height, AlphaType::Premul)
    }

    /// Creates alpha-only 8-bit image info.
    #[inline]
    pub fn new_alpha8(width: i32, height: i32) -> Result<Self, PixelError> {
        Self::new(width, height, ColorType::Alpha8, AlphaType::Premul)
    }

    /// Returns the image width.
    #[inline]
    pub fn width(&self) -> i32 {
        self.dimensions.width
    }

    /// Returns the image height.
    #[inline]
    pub fn height(&self) -> i32 {
        self.dimensions.height
    }

    /// Returns true if the image has zero area.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dimensions.is_empty()
    }

    /// Returns bytes per pixel.
    #[inline]
    pub fn bytes_per_pixel(&self) -> usize {
        self.color_type.bytes_per_pixel()
    }

    /// Returns the minimum row bytes for this image.
    #[inline]
    pub fn min_row_bytes(&self) -> usize {
        self.width() as usize * self.bytes_per_pixel()
    }

    /// Computes the byte size for the given row bytes.
    #[inline]
    pub fn compute_byte_size(&self, row_bytes: usize) -> usize {
        if self.height() <= 0 {
            return 0;
        }
        // Last row doesn't need full row_bytes, just the actual pixels
        let last_row = self.min_row_bytes();
        let other_rows = row_bytes * (self.height() as usize - 1);
        other_rows + last_row
    }

    /// Returns the byte size using minimum row bytes.
    #[inline]
    pub fn min_byte_size(&self) -> usize {
        self.compute_byte_size(self.min_row_bytes())
    }

    /// Returns the bounds as an IRect.
    #[inline]
    pub fn bounds(&self) -> IRect {
        IRect::from_size(self.dimensions)
    }

    /// Returns true if the alpha type is opaque.
    #[inline]
    pub fn is_opaque(&self) -> bool {
        self.alpha_type.is_opaque()
    }

    /// Returns a new image info with the specified alpha type.
    #[inline]
    pub fn with_alpha_type(&self, alpha_type: AlphaType) -> Self {
        Self {
            alpha_type,
            ..self.clone()
        }
    }

    /// Returns a new image info with the specified color type.
    #[inline]
    pub fn with_color_type(&self, color_type: ColorType) -> Self {
        Self {
            color_type,
            ..self.clone()
        }
    }

    /// Returns a new image info with the specified color space.
    #[inline]
    pub fn with_color_space(&self, color_space: Option<ColorSpace>) -> Self {
        Self {
            color_space,
            ..self.clone()
        }
    }

    /// Returns a new image info with the specified dimensions.
    #[inline]
    pub fn with_dimensions(&self, width: i32, height: i32) -> Result<Self, PixelError> {
        if width <= 0 || height <= 0 {
            return Err(PixelError::InvalidDimensions { width, height });
        }
        Ok(Self {
            dimensions: ISize::new(width, height),
            ..self.clone()
        })
    }

    /// Validates that row_bytes is sufficient for this image.
    #[inline]
    pub fn validate_row_bytes(&self, row_bytes: usize) -> Result<(), PixelError> {
        let min = self.min_row_bytes();
        if row_bytes < min {
            Err(PixelError::RowBytesTooSmall {
                row_bytes,
                width: self.width(),
                bpp: self.bytes_per_pixel(),
            })
        } else {
            Ok(())
        }
    }
}

impl Default for ImageInfo {
    fn default() -> Self {
        Self {
            dimensions: ISize::empty(),
            color_type: ColorType::Unknown,
            alpha_type: AlphaType::Unknown,
            color_space: None,
        }
    }
}

// =============================================================================
// Pixmap (read-only pixel access)
// =============================================================================

/// Read-only access to pixel data.
///
/// Equivalent to Skia's `SkPixmap`.
#[derive(Debug)]
pub struct Pixmap<'a> {
    /// Image properties.
    pub info: ImageInfo,
    /// Pixel data.
    pixels: &'a [u8],
    /// Bytes per row (may include padding).
    row_bytes: usize,
}

impl<'a> Pixmap<'a> {
    /// Creates a new pixmap wrapping existing pixel data.
    pub fn new(info: ImageInfo, pixels: &'a [u8], row_bytes: usize) -> Result<Self, PixelError> {
        info.validate_row_bytes(row_bytes)?;

        let required = info.compute_byte_size(row_bytes);
        if pixels.len() < required {
            return Err(PixelError::BufferTooSmall {
                required,
                actual: pixels.len(),
            });
        }

        Ok(Self {
            info,
            pixels,
            row_bytes,
        })
    }

    /// Returns the image info.
    #[inline]
    pub fn info(&self) -> &ImageInfo {
        &self.info
    }

    /// Returns the raw pixel data.
    #[inline]
    pub fn pixels(&self) -> &[u8] {
        self.pixels
    }

    /// Returns the row bytes.
    #[inline]
    pub fn row_bytes(&self) -> usize {
        self.row_bytes
    }

    /// Returns the image width.
    #[inline]
    pub fn width(&self) -> i32 {
        self.info.width()
    }

    /// Returns the image height.
    #[inline]
    pub fn height(&self) -> i32 {
        self.info.height()
    }

    /// Returns a pointer to the start of a row.
    #[inline]
    pub fn row(&self, y: i32) -> Option<&[u8]> {
        if y < 0 || y >= self.height() {
            return None;
        }
        let offset = y as usize * self.row_bytes;
        let end = offset + self.info.min_row_bytes();
        Some(&self.pixels[offset..end])
    }

    /// Returns the address of a specific pixel.
    #[inline]
    pub fn pixel_addr(&self, x: i32, y: i32) -> Option<&[u8]> {
        if x < 0 || x >= self.width() || y < 0 || y >= self.height() {
            return None;
        }
        let bpp = self.info.bytes_per_pixel();
        let offset = y as usize * self.row_bytes + x as usize * bpp;
        Some(&self.pixels[offset..offset + bpp])
    }
}

// =============================================================================
// Bitmap (mutable pixel storage)
// =============================================================================

/// Mutable pixel storage.
///
/// Equivalent to Skia's `SkBitmap`.
#[derive(Debug, Clone)]
pub struct Bitmap {
    /// Image properties.
    info: ImageInfo,
    /// Owned pixel data.
    pixels: Vec<u8>,
    /// Bytes per row.
    row_bytes: usize,
}

impl Bitmap {
    /// Creates a new empty bitmap.
    pub fn new() -> Self {
        Self {
            info: ImageInfo::default(),
            pixels: Vec::new(),
            row_bytes: 0,
        }
    }

    /// Allocates a bitmap with the specified properties.
    pub fn allocate(info: ImageInfo) -> Result<Self, PixelError> {
        let row_bytes = info.min_row_bytes();
        let size = info.compute_byte_size(row_bytes);
        let pixels = vec![0u8; size];

        Ok(Self {
            info,
            pixels,
            row_bytes,
        })
    }

    /// Allocates a bitmap with custom row bytes.
    pub fn allocate_with_row_bytes(info: ImageInfo, row_bytes: usize) -> Result<Self, PixelError> {
        info.validate_row_bytes(row_bytes)?;
        let size = info.compute_byte_size(row_bytes);
        let pixels = vec![0u8; size];

        Ok(Self {
            info,
            pixels,
            row_bytes,
        })
    }

    /// Returns the image info.
    #[inline]
    pub fn info(&self) -> &ImageInfo {
        &self.info
    }

    /// Returns the raw pixel data.
    #[inline]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Returns mutable pixel data.
    #[inline]
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Returns the row bytes.
    #[inline]
    pub fn row_bytes(&self) -> usize {
        self.row_bytes
    }

    /// Returns the image width.
    #[inline]
    pub fn width(&self) -> i32 {
        self.info.width()
    }

    /// Returns the image height.
    #[inline]
    pub fn height(&self) -> i32 {
        self.info.height()
    }

    /// Returns true if the bitmap has no pixels.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }

    /// Returns a pointer to the start of a row.
    #[inline]
    pub fn row(&self, y: i32) -> Option<&[u8]> {
        if y < 0 || y >= self.height() {
            return None;
        }
        let offset = y as usize * self.row_bytes;
        let end = offset + self.info.min_row_bytes();
        Some(&self.pixels[offset..end])
    }

    /// Returns a mutable pointer to the start of a row.
    #[inline]
    pub fn row_mut(&mut self, y: i32) -> Option<&mut [u8]> {
        if y < 0 || y >= self.height() {
            return None;
        }
        let offset = y as usize * self.row_bytes;
        let min_row = self.info.min_row_bytes();
        let end = offset + min_row;
        Some(&mut self.pixels[offset..end])
    }

    /// Returns a read-only pixmap view.
    #[inline]
    pub fn as_pixmap(&self) -> Pixmap<'_> {
        Pixmap {
            info: self.info.clone(),
            pixels: &self.pixels,
            row_bytes: self.row_bytes,
        }
    }

    /// Fills the bitmap with zeros.
    #[inline]
    pub fn erase(&mut self) {
        self.pixels.fill(0);
    }

    /// Fills the bitmap with a specific byte value.
    #[inline]
    pub fn fill(&mut self, value: u8) {
        self.pixels.fill(value);
    }
}

impl Default for Bitmap {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Surface Properties
// =============================================================================

bitflags! {
    /// Surface properties flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct SurfacePropsFlags: u32 {
        /// Use device-independent fonts.
        const USE_DEVICE_INDEPENDENT_FONTS = 1 << 0;
    }
}

/// Pixel geometry for LCD text rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PixelGeometry {
    /// Unknown pixel geometry.
    #[default]
    Unknown = 0,
    /// Horizontal RGB subpixels.
    RgbH,
    /// Horizontal BGR subpixels.
    BgrH,
    /// Vertical RGB subpixels.
    RgbV,
    /// Vertical BGR subpixels.
    BgrV,
}

/// Properties of a surface (for text rendering hints).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SurfaceProps {
    /// Flags.
    pub flags: SurfacePropsFlags,
    /// Pixel geometry.
    pub pixel_geometry: PixelGeometry,
}

impl SurfaceProps {
    /// Create new surface properties.
    #[inline]
    pub const fn new(flags: SurfacePropsFlags, pixel_geometry: PixelGeometry) -> Self {
        Self {
            flags,
            pixel_geometry,
        }
    }
}

// =============================================================================
// Pixel Format Conversion
// =============================================================================

/// Convert pixels between color types.
///
/// This handles common pixel format conversions used in graphics applications.
pub fn convert_pixels(
    src: &[u8],
    src_info: &ImageInfo,
    src_row_bytes: usize,
    dst: &mut [u8],
    dst_info: &ImageInfo,
    dst_row_bytes: usize,
) -> Result<(), PixelError> {
    // Validate dimensions match
    if src_info.width() != dst_info.width() || src_info.height() != dst_info.height() {
        return Err(PixelError::InvalidDimensions {
            width: dst_info.width(),
            height: dst_info.height(),
        });
    }

    src_info.validate_row_bytes(src_row_bytes)?;
    dst_info.validate_row_bytes(dst_row_bytes)?;

    let required_src = src_info.compute_byte_size(src_row_bytes);
    if src.len() < required_src {
        return Err(PixelError::BufferTooSmall {
            required: required_src,
            actual: src.len(),
        });
    }

    let required_dst = dst_info.compute_byte_size(dst_row_bytes);
    if dst.len() < required_dst {
        return Err(PixelError::BufferTooSmall {
            required: required_dst,
            actual: dst.len(),
        });
    }

    let width = src_info.width() as usize;
    let height = src_info.height() as usize;

    for y in 0..height {
        let src_row_start = y * src_row_bytes;
        let dst_row_start = y * dst_row_bytes;

        convert_row(
            &src[src_row_start..],
            src_info.color_type,
            &mut dst[dst_row_start..],
            dst_info.color_type,
            width,
        )?;
    }

    Ok(())
}

/// Convert a single row of pixels.
fn convert_row(
    src: &[u8],
    src_type: ColorType,
    dst: &mut [u8],
    dst_type: ColorType,
    width: usize,
) -> Result<(), PixelError> {
    use ColorType::*;

    // Same format - just copy
    if src_type == dst_type {
        let bpp = src_type.bytes_per_pixel();
        dst[..width * bpp].copy_from_slice(&src[..width * bpp]);
        return Ok(());
    }

    match (src_type, dst_type) {
        // RGBA8888 <-> BGRA8888
        (Rgba8888, Bgra8888) | (Bgra8888, Rgba8888) => {
            for i in 0..width {
                let si = i * 4;
                dst[si] = src[si + 2]; // R <-> B
                dst[si + 1] = src[si + 1]; // G
                dst[si + 2] = src[si]; // B <-> R
                dst[si + 3] = src[si + 3]; // A
            }
        }

        // RGB888 -> RGBA8888
        (Rgb888, Rgba8888) => {
            for i in 0..width {
                let si = i * 3;
                let di = i * 4;
                dst[di] = src[si];
                dst[di + 1] = src[si + 1];
                dst[di + 2] = src[si + 2];
                dst[di + 3] = 255;
            }
        }

        // RGBA8888 -> RGB888 (drop alpha)
        (Rgba8888, Rgb888) => {
            for i in 0..width {
                let si = i * 4;
                let di = i * 3;
                dst[di] = src[si];
                dst[di + 1] = src[si + 1];
                dst[di + 2] = src[si + 2];
            }
        }

        // RGB565 -> RGBA8888
        (Rgb565, Rgba8888) => {
            for i in 0..width {
                let si = i * 2;
                let di = i * 4;
                let pixel = u16::from_le_bytes([src[si], src[si + 1]]);
                dst[di] = ((pixel >> 11) as u8) << 3; // R (5 bits -> 8 bits)
                dst[di + 1] = ((pixel >> 5) as u8 & 0x3F) << 2; // G (6 bits -> 8 bits)
                dst[di + 2] = (pixel as u8 & 0x1F) << 3; // B (5 bits -> 8 bits)
                dst[di + 3] = 255;
            }
        }

        // RGBA8888 -> RGB565
        (Rgba8888, Rgb565) => {
            for i in 0..width {
                let si = i * 4;
                let di = i * 2;
                let r = (src[si] >> 3) as u16;
                let g = (src[si + 1] >> 2) as u16;
                let b = (src[si + 2] >> 3) as u16;
                let pixel = (r << 11) | (g << 5) | b;
                let bytes = pixel.to_le_bytes();
                dst[di] = bytes[0];
                dst[di + 1] = bytes[1];
            }
        }

        // Gray8 -> RGBA8888
        (Gray8, Rgba8888) => {
            for i in 0..width {
                let gray = src[i];
                let di = i * 4;
                dst[di] = gray;
                dst[di + 1] = gray;
                dst[di + 2] = gray;
                dst[di + 3] = 255;
            }
        }

        // RGBA8888 -> Gray8 (luminance)
        (Rgba8888, Gray8) => {
            for i in 0..width {
                let si = i * 4;
                let r = src[si] as f32;
                let g = src[si + 1] as f32;
                let b = src[si + 2] as f32;
                // ITU-R BT.601 luma coefficients
                dst[i] = (0.299 * r + 0.587 * g + 0.114 * b).round() as u8;
            }
        }

        // Alpha8 -> RGBA8888 (white with alpha)
        (Alpha8, Rgba8888) => {
            for i in 0..width {
                let di = i * 4;
                dst[di] = 255;
                dst[di + 1] = 255;
                dst[di + 2] = 255;
                dst[di + 3] = src[i];
            }
        }

        // RGBA8888 -> Alpha8 (extract alpha)
        (Rgba8888, Alpha8) => {
            for i in 0..width {
                dst[i] = src[i * 4 + 3];
            }
        }

        // BGRA8888 -> RGB888
        (Bgra8888, Rgb888) => {
            for i in 0..width {
                let si = i * 4;
                let di = i * 3;
                dst[di] = src[si + 2]; // R
                dst[di + 1] = src[si + 1]; // G
                dst[di + 2] = src[si]; // B
            }
        }

        // RGB888 -> BGRA8888
        (Rgb888, Bgra8888) => {
            for i in 0..width {
                let si = i * 3;
                let di = i * 4;
                dst[di] = src[si + 2]; // B
                dst[di + 1] = src[si + 1]; // G
                dst[di + 2] = src[si]; // R
                dst[di + 3] = 255;
            }
        }

        _ => return Err(PixelError::UnsupportedColorType),
    }

    Ok(())
}

/// Swizzle RGBA to BGRA (or vice versa) in place.
///
/// This is a fast path for the common case of converting between
/// RGBA and BGRA formats.
#[inline]
pub fn swizzle_rb_in_place(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }
}

/// Convert premultiplied alpha to unpremultiplied in place.
pub fn unpremultiply_in_place(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let a = chunk[3];
        if a == 0 {
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
        } else if a < 255 {
            let scale = 255.0 / a as f32;
            chunk[0] = (chunk[0] as f32 * scale).min(255.0) as u8;
            chunk[1] = (chunk[1] as f32 * scale).min(255.0) as u8;
            chunk[2] = (chunk[2] as f32 * scale).min(255.0) as u8;
        }
    }
}

/// Convert unpremultiplied alpha to premultiplied in place.
pub fn premultiply_in_place(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let a = chunk[3];
        if a == 0 {
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
        } else if a < 255 {
            let scale = a as f32 / 255.0;
            chunk[0] = (chunk[0] as f32 * scale) as u8;
            chunk[1] = (chunk[1] as f32 * scale) as u8;
            chunk[2] = (chunk[2] as f32 * scale) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_info() {
        let info = ImageInfo::new_n32_premul(100, 200).unwrap();
        assert_eq!(info.width(), 100);
        assert_eq!(info.height(), 200);
        assert_eq!(info.bytes_per_pixel(), 4);
        assert_eq!(info.min_row_bytes(), 400);
    }

    #[test]
    fn test_bitmap_allocate() {
        let info = ImageInfo::new_rgba8888(10, 10, AlphaType::Premul).unwrap();
        let bitmap = Bitmap::allocate(info).unwrap();
        assert_eq!(bitmap.width(), 10);
        assert_eq!(bitmap.height(), 10);
        assert_eq!(bitmap.pixels().len(), 400);
    }

    #[test]
    fn test_pixmap() {
        let data = vec![0u8; 400];
        let info = ImageInfo::new_rgba8888(10, 10, AlphaType::Premul).unwrap();
        let pixmap = Pixmap::new(info, &data, 40).unwrap();
        assert_eq!(pixmap.width(), 10);
        assert_eq!(pixmap.height(), 10);
    }

    #[test]
    fn test_rgba_bgra_conversion() {
        let src_info = ImageInfo::new_rgba8888(2, 1, AlphaType::Premul).unwrap();
        let dst_info = ImageInfo::new_bgra8888(2, 1, AlphaType::Premul).unwrap();

        let src = [255, 128, 64, 255, 100, 150, 200, 128];
        let mut dst = [0u8; 8];

        convert_pixels(&src, &src_info, 8, &mut dst, &dst_info, 8).unwrap();

        assert_eq!(dst[0], 64); // B from R position
        assert_eq!(dst[1], 128); // G
        assert_eq!(dst[2], 255); // R from B position
        assert_eq!(dst[3], 255); // A
    }

    #[test]
    fn test_swizzle_in_place() {
        let mut pixels = [255, 128, 64, 255, 100, 150, 200, 128];
        swizzle_rb_in_place(&mut pixels);

        assert_eq!(pixels[0], 64); // R <-> B swapped
        assert_eq!(pixels[2], 255); // R <-> B swapped
    }

    #[test]
    fn test_premultiply_round_trip() {
        let mut pixels = [200, 100, 50, 128];
        premultiply_in_place(&mut pixels);

        // After premultiply: r = 200 * 128/255 ≈ 100
        assert!(pixels[0] > 90 && pixels[0] < 110);

        unpremultiply_in_place(&mut pixels);

        // Should be close to original (with some precision loss)
        assert!(pixels[0] > 190 && pixels[0] < 210);
    }
}
