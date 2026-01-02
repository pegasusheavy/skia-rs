//! Image generator trait for deferred image generation.
//!
//! `ImageGenerator` provides a base class for generating image data on-demand.
//! This is useful for:
//! - Lazy decoding of encoded image data
//! - Procedural image generation
//! - GPU texture generation
//!
//! Corresponds to Skia's `SkImageGenerator`.

use crate::ImageInfo;
use skia_rs_core::{AlphaType, ColorType};
use std::sync::Arc;

/// Result type for generator operations.
pub type GeneratorResult<T> = Result<T, GeneratorError>;

/// Error type for generator operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GeneratorError {
    /// Failed to generate pixels.
    #[error("Failed to generate pixels: {0}")]
    GenerateFailed(String),
    /// Invalid image info.
    #[error("Invalid image info: {0}")]
    InvalidInfo(String),
    /// Unsupported color type.
    #[error("Unsupported color type: {0:?}")]
    UnsupportedColorType(ColorType),
    /// Unsupported alpha type.
    #[error("Unsupported alpha type: {0:?}")]
    UnsupportedAlphaType(AlphaType),
    /// Decode error.
    #[error("Decode error: {0}")]
    DecodeError(String),
    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(String),
}

/// A generator for image data.
///
/// `ImageGenerator` provides a way to generate image pixels on demand.
/// This enables lazy image loading and procedural image generation.
///
/// # Implementing ImageGenerator
///
/// To implement a custom generator:
/// 1. Implement `info()` to return the image dimensions and format
/// 2. Implement `on_get_pixels()` to generate the actual pixel data
/// 3. Optionally override other methods for optimization
///
/// # Example
///
/// ```ignore
/// use skia_rs_codec::{ImageGenerator, ImageInfo, GeneratorResult};
/// use skia_rs_core::{ColorType, AlphaType};
///
/// struct SolidColorGenerator {
///     info: ImageInfo,
///     color: [u8; 4],
/// }
///
/// impl ImageGenerator for SolidColorGenerator {
///     fn info(&self) -> &ImageInfo {
///         &self.info
///     }
///
///     fn on_get_pixels(&self, pixels: &mut [u8], row_bytes: usize) -> GeneratorResult<()> {
///         for y in 0..self.info.height as usize {
///             for x in 0..self.info.width as usize {
///                 let offset = y * row_bytes + x * 4;
///                 pixels[offset..offset + 4].copy_from_slice(&self.color);
///             }
///         }
///         Ok(())
///     }
/// }
/// ```
///
/// Corresponds to Skia's `SkImageGenerator`.
pub trait ImageGenerator: Send + Sync {
    /// Get the image info describing the output.
    fn info(&self) -> &ImageInfo;

    /// Get the width of the generated image.
    #[inline]
    fn width(&self) -> i32 {
        self.info().width
    }

    /// Get the height of the generated image.
    #[inline]
    fn height(&self) -> i32 {
        self.info().height
    }

    /// Get the unique ID for this generator.
    ///
    /// Two generators with the same ID will produce identical images.
    fn unique_id(&self) -> u32 {
        // Default implementation uses address-based ID (data pointer only)
        (self as *const Self as *const () as usize) as u32
    }

    /// Get a reference to the original encoded data, if available.
    ///
    /// Returns `None` if the generator doesn't have encoded data
    /// (e.g., procedural generators).
    fn ref_encoded_data(&self) -> Option<Arc<[u8]>> {
        None
    }

    /// Check if generating to the requested info is supported.
    ///
    /// Returns `true` if `get_pixels()` can generate pixels
    /// for the given target info.
    fn query_supports_info(&self, info: &ImageInfo) -> bool {
        // Default: only exact match is supported
        info.width == self.info().width
            && info.height == self.info().height
            && info.color_type == self.info().color_type
            && info.alpha_type == self.info().alpha_type
    }

    /// Generate pixels into the provided buffer.
    ///
    /// # Arguments
    /// * `info` - The target image info (may differ from generator's native info)
    /// * `pixels` - Buffer to write pixels into
    /// * `row_bytes` - Number of bytes per row in the destination buffer
    ///
    /// # Returns
    /// `Ok(())` on success, `Err` on failure.
    fn get_pixels(
        &self,
        info: &ImageInfo,
        pixels: &mut [u8],
        row_bytes: usize,
    ) -> GeneratorResult<()> {
        // Validate buffer size
        let required_size = info.compute_byte_size(row_bytes);
        if pixels.len() < required_size {
            return Err(GeneratorError::GenerateFailed(format!(
                "Buffer too small: {} < {}",
                pixels.len(),
                required_size
            )));
        }

        // Check if conversion is needed
        if info.width == self.info().width
            && info.height == self.info().height
            && info.color_type == self.info().color_type
            && info.alpha_type == self.info().alpha_type
        {
            // Direct generation
            self.on_get_pixels(pixels, row_bytes)
        } else if self.query_supports_info(info) {
            // Generator supports conversion
            self.on_get_pixels_with_conversion(info, pixels, row_bytes)
        } else {
            Err(GeneratorError::GenerateFailed(
                "Requested format not supported".into(),
            ))
        }
    }

    /// Generate pixels into the provided buffer (implementation).
    ///
    /// This method should be implemented by concrete generators.
    /// The default implementation fails.
    fn on_get_pixels(&self, pixels: &mut [u8], row_bytes: usize) -> GeneratorResult<()>;

    /// Generate pixels with format conversion.
    ///
    /// Default implementation generates native format then converts.
    fn on_get_pixels_with_conversion(
        &self,
        info: &ImageInfo,
        pixels: &mut [u8],
        row_bytes: usize,
    ) -> GeneratorResult<()> {
        // Generate in native format first
        let native_info = self.info();
        let native_row_bytes = native_info.min_row_bytes();
        let native_size = native_info.compute_byte_size(native_row_bytes);
        let mut native_pixels = vec![0u8; native_size];

        self.on_get_pixels(&mut native_pixels, native_row_bytes)?;

        // Convert to target format
        convert_pixels(native_info, &native_pixels, native_row_bytes, info, pixels, row_bytes)
    }

    /// Check if the generator is valid and can produce pixels.
    fn is_valid(&self) -> bool {
        !self.info().is_empty()
    }

}

/// A boxed image generator.
pub type BoxedImageGenerator = Box<dyn ImageGenerator>;

/// A shared image generator.
pub type SharedImageGenerator = Arc<dyn ImageGenerator>;

/// Convert pixels between formats.
pub fn convert_pixels(
    src_info: &ImageInfo,
    src_pixels: &[u8],
    src_row_bytes: usize,
    dst_info: &ImageInfo,
    dst_pixels: &mut [u8],
    dst_row_bytes: usize,
) -> GeneratorResult<()> {
    // Validate dimensions
    if src_info.width != dst_info.width || src_info.height != dst_info.height {
        return Err(GeneratorError::GenerateFailed(
            "Dimension mismatch".into(),
        ));
    }

    let width = src_info.width as usize;
    let height = src_info.height as usize;

    // Same format - direct copy
    if src_info.color_type == dst_info.color_type && src_info.alpha_type == dst_info.alpha_type {
        let bytes_per_pixel = src_info.bytes_per_pixel();
        let copy_len = width * bytes_per_pixel;

        for y in 0..height {
            let src_offset = y * src_row_bytes;
            let dst_offset = y * dst_row_bytes;
            dst_pixels[dst_offset..dst_offset + copy_len]
                .copy_from_slice(&src_pixels[src_offset..src_offset + copy_len]);
        }
        return Ok(());
    }

    // Handle common conversions
    match (src_info.color_type, dst_info.color_type) {
        (ColorType::Rgba8888, ColorType::Bgra8888) | (ColorType::Bgra8888, ColorType::Rgba8888) => {
            // Swap R and B
            for y in 0..height {
                for x in 0..width {
                    let src_offset = y * src_row_bytes + x * 4;
                    let dst_offset = y * dst_row_bytes + x * 4;
                    dst_pixels[dst_offset] = src_pixels[src_offset + 2]; // R <-> B
                    dst_pixels[dst_offset + 1] = src_pixels[src_offset + 1]; // G
                    dst_pixels[dst_offset + 2] = src_pixels[src_offset]; // B <-> R
                    dst_pixels[dst_offset + 3] = src_pixels[src_offset + 3]; // A
                }
            }
            Ok(())
        }
        (ColorType::Gray8, ColorType::Rgba8888) => {
            for y in 0..height {
                for x in 0..width {
                    let src_offset = y * src_row_bytes + x;
                    let dst_offset = y * dst_row_bytes + x * 4;
                    let gray = src_pixels[src_offset];
                    dst_pixels[dst_offset] = gray;
                    dst_pixels[dst_offset + 1] = gray;
                    dst_pixels[dst_offset + 2] = gray;
                    dst_pixels[dst_offset + 3] = 255;
                }
            }
            Ok(())
        }
        (ColorType::Alpha8, ColorType::Rgba8888) => {
            for y in 0..height {
                for x in 0..width {
                    let src_offset = y * src_row_bytes + x;
                    let dst_offset = y * dst_row_bytes + x * 4;
                    let alpha = src_pixels[src_offset];
                    dst_pixels[dst_offset] = 0;
                    dst_pixels[dst_offset + 1] = 0;
                    dst_pixels[dst_offset + 2] = 0;
                    dst_pixels[dst_offset + 3] = alpha;
                }
            }
            Ok(())
        }
        (ColorType::Rgba8888, ColorType::Gray8) => {
            for y in 0..height {
                for x in 0..width {
                    let src_offset = y * src_row_bytes + x * 4;
                    let dst_offset = y * dst_row_bytes + x;
                    let r = src_pixels[src_offset] as u32;
                    let g = src_pixels[src_offset + 1] as u32;
                    let b = src_pixels[src_offset + 2] as u32;
                    // Luminance formula: 0.299*R + 0.587*G + 0.114*B
                    let gray = ((r * 77 + g * 150 + b * 29) >> 8) as u8;
                    dst_pixels[dst_offset] = gray;
                }
            }
            Ok(())
        }
        _ => Err(GeneratorError::UnsupportedColorType(dst_info.color_type)),
    }
}

/// A simple solid color generator.
pub struct SolidColorGenerator {
    info: ImageInfo,
    color: [u8; 4],
}

impl SolidColorGenerator {
    /// Create a new solid color generator.
    pub fn new(width: i32, height: i32, color: [u8; 4]) -> Self {
        Self {
            info: ImageInfo::new(width, height, ColorType::Rgba8888, AlphaType::Premul),
            color,
        }
    }
}

impl ImageGenerator for SolidColorGenerator {
    fn info(&self) -> &ImageInfo {
        &self.info
    }

    fn on_get_pixels(&self, pixels: &mut [u8], row_bytes: usize) -> GeneratorResult<()> {
        for y in 0..self.info.height as usize {
            for x in 0..self.info.width as usize {
                let offset = y * row_bytes + x * 4;
                pixels[offset..offset + 4].copy_from_slice(&self.color);
            }
        }
        Ok(())
    }
}

/// A generator that wraps encoded image data (lazy decoding).
pub struct EncodedImageGenerator {
    info: ImageInfo,
    encoded_data: Arc<[u8]>,
    unique_id: u32,
}

impl EncodedImageGenerator {
    /// Create a generator from encoded data.
    ///
    /// Returns `None` if the data cannot be decoded.
    pub fn new(data: Vec<u8>) -> Option<Self> {
        // Probe the encoded data to get dimensions
        let (width, height) = crate::get_image_dimensions(&data).ok()?;

        static ID_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        let unique_id = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Use RGBA8888/Premul as default - actual format determined at decode time
        Some(Self {
            info: ImageInfo::new(width, height, ColorType::Rgba8888, AlphaType::Premul),
            encoded_data: data.into(),
            unique_id,
        })
    }

    /// Create a generator from shared encoded data.
    pub fn from_shared(data: Arc<[u8]>) -> Option<Self> {
        let (width, height) = crate::get_image_dimensions(&data).ok()?;

        static ID_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        let unique_id = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Some(Self {
            info: ImageInfo::new(width, height, ColorType::Rgba8888, AlphaType::Premul),
            encoded_data: data,
            unique_id,
        })
    }
}

impl ImageGenerator for EncodedImageGenerator {
    fn info(&self) -> &ImageInfo {
        &self.info
    }

    fn unique_id(&self) -> u32 {
        self.unique_id
    }

    fn ref_encoded_data(&self) -> Option<Arc<[u8]>> {
        Some(self.encoded_data.clone())
    }

    fn on_get_pixels(&self, pixels: &mut [u8], row_bytes: usize) -> GeneratorResult<()> {
        let image = crate::decode_image(&self.encoded_data)
            .map_err(|e| GeneratorError::DecodeError(e.to_string()))?;

        // Read pixels from decoded image
        let src_row_bytes = image.row_bytes();
        let bytes_per_pixel = self.info.bytes_per_pixel();
        let width = self.info.width as usize;
        let height = self.info.height as usize;

        if let Some(src_pixels) = image.peek_pixels() {
            for y in 0..height {
                let src_offset = y * src_row_bytes;
                let dst_offset = y * row_bytes;
                let copy_len = width * bytes_per_pixel;
                pixels[dst_offset..dst_offset + copy_len]
                    .copy_from_slice(&src_pixels[src_offset..src_offset + copy_len]);
            }
            Ok(())
        } else {
            Err(GeneratorError::GenerateFailed(
                "Failed to access decoded pixels".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solid_color_generator() {
        let generator = SolidColorGenerator::new(10, 10, [255, 0, 0, 255]);
        assert_eq!(generator.width(), 10);
        assert_eq!(generator.height(), 10);

        let mut pixels = vec![0u8; 10 * 10 * 4];
        generator.get_pixels(generator.info(), &mut pixels, 10 * 4).unwrap();

        // Check first pixel is red
        assert_eq!(pixels[0], 255); // R
        assert_eq!(pixels[1], 0); // G
        assert_eq!(pixels[2], 0); // B
        assert_eq!(pixels[3], 255); // A
    }

    #[test]
    fn test_convert_rgba_to_bgra() {
        let src_info = ImageInfo::new(2, 2, ColorType::Rgba8888, AlphaType::Premul);
        let dst_info = ImageInfo::new(2, 2, ColorType::Bgra8888, AlphaType::Premul);

        let src_pixels = [
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 255, 255, // White
        ];
        let mut dst_pixels = vec![0u8; 16];

        convert_pixels(&src_info, &src_pixels, 8, &dst_info, &mut dst_pixels, 8).unwrap();

        // Check that R and B are swapped
        assert_eq!(dst_pixels[0..4], [0, 0, 255, 255]); // Blue channel first (was Red)
        assert_eq!(dst_pixels[4..8], [0, 255, 0, 255]); // Green unchanged
    }

    #[test]
    fn test_generator_unique_id() {
        let generator1 = SolidColorGenerator::new(10, 10, [0, 0, 0, 255]);
        let generator2 = SolidColorGenerator::new(10, 10, [0, 0, 0, 255]);

        // Different generators should have different IDs
        assert_ne!(generator1.unique_id(), generator2.unique_id());
    }
}
