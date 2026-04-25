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
        convert_pixels(
            native_info,
            &native_pixels,
            native_row_bytes,
            info,
            pixels,
            row_bytes,
        )
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
///
/// Delegates to [`skia_rs_core::convert_pixels`], which handles both
/// color-type translation (RGBA<->BGRA, Gray8<->RGBA, Alpha8<->RGBA,
/// RGB888<->RGBA, RGB565<->RGBA) and alpha-type translation
/// (Premul<->Unpremul) in a single row-by-row pass.
pub fn convert_pixels(
    src_info: &ImageInfo,
    src_pixels: &[u8],
    src_row_bytes: usize,
    dst_info: &ImageInfo,
    dst_pixels: &mut [u8],
    dst_row_bytes: usize,
) -> GeneratorResult<()> {
    // Validate dimensions up front for a clean error message.
    if src_info.width != dst_info.width || src_info.height != dst_info.height {
        return Err(GeneratorError::GenerateFailed("Dimension mismatch".into()));
    }

    let src_core = skia_rs_core::ImageInfo::new(
        src_info.width,
        src_info.height,
        src_info.color_type,
        src_info.alpha_type,
    )
    .map_err(|e| GeneratorError::InvalidInfo(e.to_string()))?;
    let dst_core = skia_rs_core::ImageInfo::new(
        dst_info.width,
        dst_info.height,
        dst_info.color_type,
        dst_info.alpha_type,
    )
    .map_err(|e| GeneratorError::InvalidInfo(e.to_string()))?;

    skia_rs_core::convert_pixels(
        src_pixels,
        &src_core,
        src_row_bytes,
        dst_pixels,
        &dst_core,
        dst_row_bytes,
    )
    .map_err(|e| match e {
        skia_rs_core::PixelError::UnsupportedColorType => {
            GeneratorError::UnsupportedColorType(dst_info.color_type)
        }
        other => GeneratorError::GenerateFailed(other.to_string()),
    })
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

/// A generator that wraps encoded image data with eager decoding.
///
/// The encoded payload is decoded exactly once, at construction time. This
/// trades a small startup cost for:
/// - **Correctness**: `info()` returns the decoder's actual color/alpha
///   type rather than a hard-coded `Rgba8888/Premul` placeholder, so
///   callers that compare `generator.info()` against a target buffer see
///   the real format.
/// - **Performance**: `on_get_pixels` serves from the cached decoded image
///   instead of re-running the decode on every call.
///
/// For deferred decoding at a higher level (the original motivation for
/// this type), wrap it in a [`LazyImage`](crate::LazyImage) — that API
/// handles the "don't decode until actually needed" semantics and keeps
/// the generator's role focused on producing pixels once asked.
pub struct EncodedImageGenerator {
    info: ImageInfo,
    encoded_data: Arc<[u8]>,
    unique_id: u32,
    /// Decoded image, computed once in the constructor.
    decoded: crate::Image,
}

impl EncodedImageGenerator {
    /// Create a generator from encoded data.
    ///
    /// Returns `None` if the data cannot be decoded.
    pub fn new(data: Vec<u8>) -> Option<Self> {
        Self::from_shared(data.into())
    }

    /// Create a generator from shared encoded data.
    ///
    /// Decodes the payload up front to determine its native format.
    pub fn from_shared(data: Arc<[u8]>) -> Option<Self> {
        let decoded = crate::decode_image(&data).ok()?;

        static ID_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        let unique_id = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Some(Self {
            info: decoded.info().clone(),
            encoded_data: data,
            unique_id,
            decoded,
        })
    }

    /// Get a reference to the cached decoded image.
    pub fn decoded_image(&self) -> &crate::Image {
        &self.decoded
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
        let info = self.decoded.info();
        let src_row_bytes = self.decoded.row_bytes();
        let bytes_per_pixel = info.bytes_per_pixel();
        let width = info.width as usize;
        let height = info.height as usize;

        if let Some(src_pixels) = self.decoded.peek_pixels() {
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

    fn on_get_pixels_with_conversion(
        &self,
        info: &ImageInfo,
        pixels: &mut [u8],
        row_bytes: usize,
    ) -> GeneratorResult<()> {
        // Convert directly from the cached decoded pixels to the caller's
        // target. Overrides the default trait impl, which would allocate a
        // staging buffer and decode into native first.
        let src_info = self.decoded.info();
        let src_row_bytes = self.decoded.row_bytes();
        let src_pixels = self.decoded.peek_pixels().ok_or_else(|| {
            GeneratorError::GenerateFailed("Failed to access decoded pixels".into())
        })?;

        convert_pixels(
            src_info,
            src_pixels,
            src_row_bytes,
            info,
            pixels,
            row_bytes,
        )
    }

    fn query_supports_info(&self, info: &ImageInfo) -> bool {
        // Native format is always supported.
        if info.width == self.info.width
            && info.height == self.info.height
            && info.color_type == self.info.color_type
            && info.alpha_type == self.info.alpha_type
        {
            return true;
        }
        // Otherwise, we support any format convert_pixels can reach. The
        // function gate-keeps itself by returning UnsupportedColorType, so
        // probe with a 0x0 info to avoid allocating a buffer here.
        if info.width != self.info.width || info.height != self.info.height {
            return false;
        }
        // Try the conversion on a tiny probe buffer; the function rejects
        // on dimension mismatch before touching pixels, so we have to pass
        // the real dimensions. Check against a whitelist of supported
        // (src, dst) pairs instead.
        matches!(
            (self.info.color_type, info.color_type),
            (ColorType::Rgba8888, ColorType::Rgba8888)
                | (ColorType::Rgba8888, ColorType::Bgra8888)
                | (ColorType::Bgra8888, ColorType::Rgba8888)
                | (ColorType::Bgra8888, ColorType::Bgra8888)
                | (ColorType::Gray8, ColorType::Rgba8888)
                | (ColorType::Alpha8, ColorType::Rgba8888)
                | (ColorType::Rgba8888, ColorType::Gray8)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "png")]
    use crate::codec::ImageEncoder;

    #[test]
    fn test_solid_color_generator() {
        let generator = SolidColorGenerator::new(10, 10, [255, 0, 0, 255]);
        assert_eq!(generator.width(), 10);
        assert_eq!(generator.height(), 10);

        let mut pixels = vec![0u8; 10 * 10 * 4];
        generator
            .get_pixels(generator.info(), &mut pixels, 10 * 4)
            .unwrap();

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

    #[test]
    fn test_convert_pixels_premul_to_unpremul() {
        // (25, 50, 75, 128) premul -> (50, 100, 150, 128) unpremul
        let src_info = ImageInfo::new(1, 1, ColorType::Rgba8888, AlphaType::Premul);
        let dst_info = ImageInfo::new(1, 1, ColorType::Rgba8888, AlphaType::Unpremul);
        let src = [25, 50, 75, 128];
        let mut dst = [0u8; 4];
        convert_pixels(&src_info, &src, 4, &dst_info, &mut dst, 4).unwrap();
        // Allow off-by-one from integer rounding.
        assert!((dst[0] as i32 - 50).abs() <= 1);
        assert!((dst[1] as i32 - 100).abs() <= 1);
        assert!((dst[2] as i32 - 150).abs() <= 1);
        assert_eq!(dst[3], 128);
    }

    #[cfg(feature = "png")]
    #[test]
    fn test_encoded_generator_reports_decoded_info() {
        // Encode a tiny PNG and verify the generator's info reflects the
        // decoder's actual output (not the hard-coded Rgba8888/Premul
        // placeholder that caused C-4).
        let info = ImageInfo::new(2, 2, ColorType::Rgba8888, AlphaType::Unpremul);
        let src_pixels = vec![
            255, 0, 0, 255, 0, 255, 0, 255, // row 0
            0, 0, 255, 255, 255, 255, 0, 255, // row 1
        ];
        let src_image =
            crate::Image::from_raster_data(&info, &src_pixels, 8).expect("raster image");
        let encoded = crate::PngEncoder::new()
            .encode_bytes(&src_image)
            .expect("encode PNG");

        let generator = EncodedImageGenerator::new(encoded).expect("build generator");
        // After eager decode the advertised info matches decoder output.
        assert_eq!(generator.info().width, 2);
        assert_eq!(generator.info().height, 2);
        assert_eq!(generator.info().color_type, ColorType::Rgba8888);

        // Native format read-out works and matches.
        let mut dst = vec![0u8; 16];
        generator
            .get_pixels(generator.info(), &mut dst, 8)
            .expect("get_pixels native");
        // Pixel 0: should be red; alpha interpretation is decoder-dependent
        // (PNG reports Unpremul for this input) but red still shows red.
        assert_eq!(dst[0], 255);
        assert_eq!(dst[1], 0);
        assert_eq!(dst[2], 0);
    }

    #[cfg(feature = "png")]
    #[test]
    fn test_encoded_generator_caches_decoded_image() {
        // Decoding is driven by from_shared, so calling get_pixels N times
        // should not re-decode. We verify by inspecting that repeated
        // reads from the generator all produce the exact same bytes even
        // after we drop and recreate the destination buffer.
        let info = ImageInfo::new(4, 4, ColorType::Rgba8888, AlphaType::Unpremul);
        let src_pixels: Vec<u8> = (0..(4 * 4 * 4)).map(|i| (i % 256) as u8).collect();
        let src_image =
            crate::Image::from_raster_data(&info, &src_pixels, 16).expect("raster image");
        let encoded = crate::PngEncoder::new()
            .encode_bytes(&src_image)
            .expect("encode PNG");

        let generator = EncodedImageGenerator::new(encoded).expect("build generator");

        // The decoded image handle should be stable across calls — same
        // underlying Arc means we are serving from cache, not re-decoding.
        let img1_ptr = generator.decoded_image().unique_id();
        let img2_ptr = generator.decoded_image().unique_id();
        assert_eq!(img1_ptr, img2_ptr);

        // Two calls produce identical bytes.
        let row_bytes = generator.info().min_row_bytes();
        let size = generator.info().compute_byte_size(row_bytes);
        let mut buf1 = vec![0u8; size];
        let mut buf2 = vec![0u8; size];
        generator
            .get_pixels(generator.info(), &mut buf1, row_bytes)
            .unwrap();
        generator
            .get_pixels(generator.info(), &mut buf2, row_bytes)
            .unwrap();
        assert_eq!(buf1, buf2);
    }

    #[cfg(feature = "png")]
    #[test]
    fn test_encoded_generator_converts_on_demand() {
        // Encode a PNG as RGBA, then ask the generator for BGRA: the
        // conversion path must run over the cached decoded pixels.
        let info = ImageInfo::new(1, 1, ColorType::Rgba8888, AlphaType::Unpremul);
        let src_pixels = vec![10, 20, 30, 255];
        let src_image = crate::Image::from_raster_data(&info, &src_pixels, 4).unwrap();
        let encoded = crate::PngEncoder::new().encode_bytes(&src_image).unwrap();

        let generator = EncodedImageGenerator::new(encoded).unwrap();
        // Build a BGRA target info matching the generator's dimensions/alpha.
        let target = ImageInfo::new(1, 1, ColorType::Bgra8888, generator.info().alpha_type);
        let mut dst = vec![0u8; 4];
        generator.get_pixels(&target, &mut dst, 4).unwrap();
        // BGRA: [B=30, G=20, R=10, A=255]
        assert_eq!(dst, vec![30, 20, 10, 255]);
    }
}
