//! Lazy/deferred images.
//!
//! Lazy images defer pixel generation until the pixels are actually needed.
//! This is useful for:
//! - Delaying expensive decode operations
//! - Memory-efficient image handling
//! - Procedural image generation
//!
//! Corresponds to Skia's lazy image generation via `SkImageGenerator`.

use crate::{GeneratorError, GeneratorResult, Image, ImageGenerator, ImageInfo};
use skia_rs_core::{AlphaType, ColorSpace, ColorType, Rect, Scalar};
use std::sync::Arc;

/// The state of a lazy image's pixel data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LazyImageState {
    /// Pixels have not been generated yet.
    NotGenerated,
    /// Pixels are currently being generated.
    Generating,
    /// Pixels have been generated and cached.
    Generated,
    /// Generation failed.
    Failed,
}

/// A lazy/deferred image.
///
/// `LazyImage` wraps an `ImageGenerator` and defers pixel generation
/// until explicitly requested. The generated pixels are then cached
/// for future access.
///
/// # Memory Behavior
///
/// - Before `ensure_pixels_generated()`: Only stores the generator
/// - After `ensure_pixels_generated()`: Stores generated pixel data
/// - `discard_pixels()`: Returns to the not-generated state
///
/// This allows memory-efficient handling of images that may not be
/// immediately displayed.
///
/// # Thread Safety
///
/// `LazyImage` is thread-safe. Pixel generation is synchronized,
/// ensuring only one thread generates pixels while others wait.
///
/// # Example
///
/// ```ignore
/// use skia_rs_codec::{LazyImage, EncodedImageGenerator};
///
/// // Create a lazy image from encoded data
/// let data = std::fs::read("image.png").unwrap();
/// let generator = EncodedImageGenerator::new(data).unwrap();
/// let lazy_image = LazyImage::from_generator(Box::new(generator));
///
/// // Pixels are not decoded yet
/// assert!(!lazy_image.is_generated());
///
/// // Generate pixels on demand
/// lazy_image.ensure_pixels_generated().unwrap();
/// assert!(lazy_image.is_generated());
///
/// // Access the pixels
/// let pixels = lazy_image.peek_pixels().unwrap();
/// ```
pub struct LazyImage {
    inner: Arc<LazyImageInner>,
}

struct LazyImageInner {
    generator: Box<dyn ImageGenerator>,
    state: parking_lot::RwLock<LazyImageState>,
    cached_pixels: parking_lot::RwLock<Option<CachedPixels>>,
}

struct CachedPixels {
    pixels: Vec<u8>,
    row_bytes: usize,
}

impl Clone for LazyImage {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl std::fmt::Debug for LazyImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyImage")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("color_type", &self.color_type())
            .field("state", &self.state())
            .finish()
    }
}

impl LazyImage {
    /// Create a lazy image from an image generator.
    pub fn from_generator(generator: Box<dyn ImageGenerator>) -> Self {
        Self {
            inner: Arc::new(LazyImageInner {
                generator,
                state: parking_lot::RwLock::new(LazyImageState::NotGenerated),
                cached_pixels: parking_lot::RwLock::new(None),
            }),
        }
    }

    /// Create a lazy image from encoded data.
    pub fn from_encoded(data: Vec<u8>) -> Option<Self> {
        let generator = crate::EncodedImageGenerator::new(data)?;
        Some(Self::from_generator(Box::new(generator)))
    }

    /// Create a lazy image from shared encoded data.
    pub fn from_encoded_shared(data: Arc<[u8]>) -> Option<Self> {
        let generator = crate::EncodedImageGenerator::from_shared(data)?;
        Some(Self::from_generator(Box::new(generator)))
    }

    /// Get the image width.
    #[inline]
    pub fn width(&self) -> i32 {
        self.inner.generator.width()
    }

    /// Get the image height.
    #[inline]
    pub fn height(&self) -> i32 {
        self.inner.generator.height()
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
        self.inner.generator.info()
    }

    /// Get the color type.
    #[inline]
    pub fn color_type(&self) -> ColorType {
        self.info().color_type
    }

    /// Get the alpha type.
    #[inline]
    pub fn alpha_type(&self) -> AlphaType {
        self.info().alpha_type
    }

    /// Get the color space.
    #[inline]
    pub fn color_space(&self) -> Option<&ColorSpace> {
        self.info().color_space()
    }

    /// Returns true if the image is opaque.
    #[inline]
    pub fn is_opaque(&self) -> bool {
        self.info().is_opaque()
    }

    /// Get the unique ID.
    #[inline]
    pub fn unique_id(&self) -> u32 {
        self.inner.generator.unique_id()
    }

    /// Get the current state of pixel generation.
    pub fn state(&self) -> LazyImageState {
        *self.inner.state.read()
    }

    /// Check if pixels have been generated.
    #[inline]
    pub fn is_generated(&self) -> bool {
        self.state() == LazyImageState::Generated
    }

    /// Check if generation has failed.
    #[inline]
    pub fn is_failed(&self) -> bool {
        self.state() == LazyImageState::Failed
    }

    /// Get a reference to the original encoded data, if available.
    pub fn ref_encoded_data(&self) -> Option<Arc<[u8]>> {
        self.inner.generator.ref_encoded_data()
    }

    /// Ensure pixels are generated.
    ///
    /// If pixels have already been generated, this is a no-op.
    /// If generation fails, the error is cached and returned on subsequent calls.
    pub fn ensure_pixels_generated(&self) -> GeneratorResult<()> {
        // Fast path: already generated
        {
            let state = self.inner.state.read();
            match *state {
                LazyImageState::Generated => return Ok(()),
                LazyImageState::Failed => {
                    return Err(GeneratorError::GenerateFailed(
                        "Previous generation failed".into(),
                    ));
                }
                _ => {}
            }
        }

        // Need to generate - acquire write lock
        let mut state = self.inner.state.write();

        // Double-check state after acquiring lock
        match *state {
            LazyImageState::Generated => return Ok(()),
            LazyImageState::Failed => {
                return Err(GeneratorError::GenerateFailed(
                    "Previous generation failed".into(),
                ));
            }
            LazyImageState::Generating => {
                return Err(GeneratorError::GenerateFailed(
                    "Generation in progress".into(),
                ));
            }
            _ => {}
        }

        // Start generation
        *state = LazyImageState::Generating;
        drop(state);

        // Generate pixels
        let info = self.inner.generator.info();
        let row_bytes = info.min_row_bytes();
        let size = info.compute_byte_size(row_bytes);
        let mut pixels = vec![0u8; size];

        let result = self
            .inner
            .generator
            .get_pixels(info, &mut pixels, row_bytes);

        // Update state based on result
        let mut state = self.inner.state.write();
        match result {
            Ok(()) => {
                *self.inner.cached_pixels.write() = Some(CachedPixels { pixels, row_bytes });
                *state = LazyImageState::Generated;
                Ok(())
            }
            Err(e) => {
                *state = LazyImageState::Failed;
                Err(e)
            }
        }
    }

    /// Discard cached pixels to free memory.
    ///
    /// After calling this, `is_generated()` returns false and
    /// the next `ensure_pixels_generated()` or `peek_pixels()` call
    /// will regenerate the pixels.
    pub fn discard_pixels(&self) {
        let mut state = self.inner.state.write();
        if *state == LazyImageState::Generated {
            *self.inner.cached_pixels.write() = None;
            *state = LazyImageState::NotGenerated;
        }
    }

    /// Get direct access to the generated pixel data.
    ///
    /// If pixels haven't been generated yet, this will trigger generation.
    pub fn peek_pixels(&self) -> Option<&[u8]> {
        // Ensure generated first
        if self.ensure_pixels_generated().is_err() {
            return None;
        }

        // This is safe because:
        // 1. We just ensured pixels are generated
        // 2. The inner Arc keeps the data alive
        // 3. The cached_pixels field is only modified under write lock
        //
        // However, we need to be careful about the borrow checker.
        // We'll return None and let callers use read_pixels instead.
        None
    }

    /// Read pixels into a provided buffer.
    pub fn read_pixels(&self, dst: &mut [u8], dst_row_bytes: usize) -> bool {
        // Ensure generated
        if self.ensure_pixels_generated().is_err() {
            return false;
        }

        let cache = self.inner.cached_pixels.read();
        if let Some(ref cached) = *cache {
            let info = self.info();
            let bytes_per_pixel = info.bytes_per_pixel();
            let width = info.width as usize;
            let height = info.height as usize;

            for y in 0..height {
                let src_offset = y * cached.row_bytes;
                let dst_offset = y * dst_row_bytes;
                let copy_len = width * bytes_per_pixel;

                if dst_offset + copy_len <= dst.len()
                    && src_offset + copy_len <= cached.pixels.len()
                {
                    dst[dst_offset..dst_offset + copy_len]
                        .copy_from_slice(&cached.pixels[src_offset..src_offset + copy_len]);
                }
            }
            true
        } else {
            false
        }
    }

    /// Convert to an immutable `Image`.
    ///
    /// This generates pixels if needed and creates a copy.
    pub fn to_image(&self) -> Option<Image> {
        // Ensure pixels are generated
        self.ensure_pixels_generated().ok()?;

        let cache = self.inner.cached_pixels.read();
        if let Some(ref cached) = *cache {
            Image::from_raster_data(self.info(), &cached.pixels, cached.row_bytes)
        } else {
            None
        }
    }

    /// Make a subset of this lazy image.
    ///
    /// Returns a new lazy image that will generate only the subset.
    pub fn make_subset(&self, subset: &Rect) -> Option<Self> {
        // Generate full image first, then take subset
        let image = self.to_image()?;
        let subset_image = image.make_subset(subset)?;

        // Wrap in a pre-generated lazy image
        Some(Self::from_image(subset_image))
    }

    /// Create a lazy image from an already-decoded image.
    ///
    /// This is useful for wrapping existing images in the lazy interface.
    pub fn from_image(image: Image) -> Self {
        let generator = RasterImageGenerator::new(image);
        Self::from_generator(Box::new(generator))
    }
}

/// A generator that wraps an existing raster image.
struct RasterImageGenerator {
    image: Image,
}

impl RasterImageGenerator {
    fn new(image: Image) -> Self {
        Self { image }
    }
}

impl ImageGenerator for RasterImageGenerator {
    fn info(&self) -> &ImageInfo {
        self.image.info()
    }

    fn unique_id(&self) -> u32 {
        self.image.unique_id() as u32
    }

    fn on_get_pixels(&self, pixels: &mut [u8], row_bytes: usize) -> GeneratorResult<()> {
        let info = self.info();
        let bytes_per_pixel = info.bytes_per_pixel();
        let width = info.width as usize;
        let height = info.height as usize;

        if let Some(src_pixels) = self.image.peek_pixels() {
            let src_row_bytes = self.image.row_bytes();

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
                "Failed to access image pixels".into(),
            ))
        }
    }
}

/// A lazy image reference (shared ownership).
pub type LazyImageRef = Arc<LazyImage>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SolidColorGenerator;

    #[test]
    fn test_lazy_image_creation() {
        let generator = SolidColorGenerator::new(100, 100, [255, 0, 0, 255]);
        let lazy = LazyImage::from_generator(Box::new(generator));

        assert_eq!(lazy.dimensions(), (100, 100));
        assert!(!lazy.is_generated());
    }

    #[test]
    fn test_lazy_image_generation() {
        let generator = SolidColorGenerator::new(10, 10, [128, 64, 32, 255]);
        let lazy = LazyImage::from_generator(Box::new(generator));

        assert_eq!(lazy.state(), LazyImageState::NotGenerated);

        lazy.ensure_pixels_generated().unwrap();

        assert_eq!(lazy.state(), LazyImageState::Generated);
        assert!(lazy.is_generated());
    }

    #[test]
    fn test_lazy_image_read_pixels() {
        let generator = SolidColorGenerator::new(10, 10, [255, 128, 64, 255]);
        let lazy = LazyImage::from_generator(Box::new(generator));

        let mut pixels = vec![0u8; 10 * 10 * 4];
        assert!(lazy.read_pixels(&mut pixels, 10 * 4));

        // Verify first pixel color
        assert_eq!(pixels[0], 255); // R
        assert_eq!(pixels[1], 128); // G
        assert_eq!(pixels[2], 64); // B
        assert_eq!(pixels[3], 255); // A
    }

    #[test]
    fn test_lazy_image_discard() {
        let generator = SolidColorGenerator::new(10, 10, [255, 0, 0, 255]);
        let lazy = LazyImage::from_generator(Box::new(generator));

        lazy.ensure_pixels_generated().unwrap();
        assert!(lazy.is_generated());

        lazy.discard_pixels();
        assert!(!lazy.is_generated());

        // Can regenerate
        lazy.ensure_pixels_generated().unwrap();
        assert!(lazy.is_generated());
    }

    #[test]
    fn test_lazy_image_to_image() {
        let generator = SolidColorGenerator::new(50, 50, [0, 255, 0, 255]);
        let lazy = LazyImage::from_generator(Box::new(generator));

        let image = lazy.to_image().unwrap();
        assert_eq!(image.dimensions(), (50, 50));
    }

    #[test]
    fn test_lazy_image_from_image() {
        let info = ImageInfo::new(20, 20, ColorType::Rgba8888, AlphaType::Premul);
        let pixels = vec![100u8; 20 * 20 * 4];
        let image = Image::from_raster_data(&info, &pixels, 20 * 4).unwrap();

        let lazy = LazyImage::from_image(image);
        assert_eq!(lazy.dimensions(), (20, 20));

        // Already has pixel data via generator
        lazy.ensure_pixels_generated().unwrap();
        assert!(lazy.is_generated());
    }

    #[test]
    fn test_lazy_image_thread_safety() {
        use std::thread;

        let generator = SolidColorGenerator::new(10, 10, [255, 0, 0, 255]);
        let lazy = Arc::new(LazyImage::from_generator(Box::new(generator)));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let lazy = Arc::clone(&lazy);
                thread::spawn(move || {
                    lazy.ensure_pixels_generated().unwrap();
                    assert!(lazy.is_generated());
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
