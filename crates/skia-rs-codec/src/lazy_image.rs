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
    /// Generation state and the cached pixels guarded together under one
    /// mutex, with a condvar so late arrivals block on the generating thread
    /// instead of racing or erroring.
    gen_state: parking_lot::Mutex<GenState>,
    cv: parking_lot::Condvar,
}

struct GenState {
    state: LazyImageState,
    cached: Option<CachedPixels>,
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
                gen_state: parking_lot::Mutex::new(GenState {
                    state: LazyImageState::NotGenerated,
                    cached: None,
                }),
                cv: parking_lot::Condvar::new(),
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
        self.inner.gen_state.lock().state
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
    /// If pixels have already been generated, this is a no-op. If another
    /// thread is currently generating, this **blocks** on a condition
    /// variable until that thread publishes its result, then returns it —
    /// concurrent callers never see a spurious "generation in progress"
    /// error. If generation fails, the failure is cached and returned on
    /// subsequent calls.
    pub fn ensure_pixels_generated(&self) -> GeneratorResult<()> {
        let mut guard = self.inner.gen_state.lock();
        loop {
            match guard.state {
                LazyImageState::Generated => return Ok(()),
                LazyImageState::Failed => {
                    return Err(GeneratorError::GenerateFailed(
                        "Previous generation failed".into(),
                    ));
                }
                LazyImageState::Generating => {
                    // Another thread is decoding; wait for it to finish and
                    // re-check the state it published.
                    self.inner.cv.wait(&mut guard);
                }
                LazyImageState::NotGenerated => {
                    // Claim the generation slot, then release the lock so
                    // the (potentially expensive) decode does not block
                    // waiters from parking on the condvar.
                    guard.state = LazyImageState::Generating;
                    drop(guard);

                    let info = self.inner.generator.info();
                    let row_bytes = info.min_row_bytes();
                    let size = info.compute_byte_size(row_bytes);
                    let mut pixels = vec![0u8; size];

                    // If the generator panics we must not leave the state at
                    // `Generating` — parking_lot mutexes do not poison, so
                    // condvar waiters would block forever. Catch the unwind,
                    // publish `Failed` + notify, then resume the panic.
                    let result =
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            self.inner
                                .generator
                                .get_pixels(info, &mut pixels, row_bytes)
                        })) {
                            Ok(result) => result,
                            Err(payload) => {
                                let mut guard = self.inner.gen_state.lock();
                                guard.state = LazyImageState::Failed;
                                drop(guard);
                                self.inner.cv.notify_all();
                                std::panic::resume_unwind(payload);
                            }
                        };

                    let mut guard = self.inner.gen_state.lock();
                    let ret = match result {
                        Ok(()) => {
                            guard.cached = Some(CachedPixels { pixels, row_bytes });
                            guard.state = LazyImageState::Generated;
                            Ok(())
                        }
                        Err(e) => {
                            guard.state = LazyImageState::Failed;
                            Err(e)
                        }
                    };
                    // Wake every waiter so they observe Generated/Failed.
                    self.inner.cv.notify_all();
                    return ret;
                }
            }
        }
    }

    /// Discard cached pixels to free memory.
    ///
    /// After calling this, `is_generated()` returns false and
    /// the next `ensure_pixels_generated()` or `peek_pixels()` call
    /// will regenerate the pixels.
    pub fn discard_pixels(&self) {
        let mut guard = self.inner.gen_state.lock();
        if guard.state == LazyImageState::Generated {
            guard.cached = None;
            guard.state = LazyImageState::NotGenerated;
        }
    }

    /// Get direct access to the already-generated pixel data.
    ///
    /// Returns `Some(pixmap)` only when pixels have **already** been
    /// generated; it never triggers a decode (that would be a surprising
    /// side effect for a "peek"). Call [`ensure_pixels_generated`] first if
    /// you need the pixels produced.
    ///
    /// [`ensure_pixels_generated`]: LazyImage::ensure_pixels_generated
    ///
    /// # Locking
    ///
    /// The returned [`PeekedPixels`] guard holds the image's internal lock
    /// for as long as it is alive, so the pixels cannot be discarded or
    /// regenerated out from under the borrow — not even through another
    /// clone of this `LazyImage` on another thread (such a call blocks
    /// until the guard drops). Consequently, do **not** call
    /// [`discard_pixels`], [`ensure_pixels_generated`], or any other
    /// `LazyImage` method from the same thread while holding the guard;
    /// that would self-deadlock. Copy what you need and drop the guard.
    ///
    /// [`discard_pixels`]: LazyImage::discard_pixels
    pub fn peek_pixels(&self) -> Option<PeekedPixels<'_>> {
        let guard = self.inner.gen_state.lock();
        if guard.state != LazyImageState::Generated || guard.cached.is_none() {
            return None;
        }
        Some(PeekedPixels { guard })
    }

    /// Read pixels into a provided buffer.
    ///
    /// Returns `false` (copying nothing) if generation fails or if the
    /// destination is too small to hold every row — no silent partial copy.
    pub fn read_pixels(&self, dst: &mut [u8], dst_row_bytes: usize) -> bool {
        // Ensure generated
        if self.ensure_pixels_generated().is_err() {
            return false;
        }

        let guard = self.inner.gen_state.lock();
        let Some(ref cached) = guard.cached else {
            return false;
        };
        let info = self.info();
        let bytes_per_pixel = info.bytes_per_pixel();
        let width = info.width as usize;
        let height = info.height as usize;
        let copy_len = width * bytes_per_pixel;

        // Validate that the whole image fits in both buffers before copying
        // anything, so a too-small destination reports failure rather than a
        // partially-filled buffer.
        if height == 0 {
            return true;
        }
        // `dst_row_bytes` is caller-supplied and may be a hostile
        // near-`usize::MAX` value; use checked arithmetic so a wrapping
        // multiply can't slip a too-small destination past this precheck.
        let Some(last_dst) = (height - 1)
            .checked_mul(dst_row_bytes)
            .and_then(|v| v.checked_add(copy_len))
        else {
            return false;
        };
        let Some(last_src) = (height - 1)
            .checked_mul(cached.row_bytes)
            .and_then(|v| v.checked_add(copy_len))
        else {
            return false;
        };
        if last_dst > dst.len() || last_src > cached.pixels.len() {
            return false;
        }

        for y in 0..height {
            let src_offset = y * cached.row_bytes;
            let dst_offset = y * dst_row_bytes;
            dst[dst_offset..dst_offset + copy_len]
                .copy_from_slice(&cached.pixels[src_offset..src_offset + copy_len]);
        }
        true
    }

    /// Convert to an immutable `Image`.
    ///
    /// This generates pixels if needed and creates a copy.
    pub fn to_image(&self) -> Option<Image> {
        // Ensure pixels are generated
        self.ensure_pixels_generated().ok()?;

        let guard = self.inner.gen_state.lock();
        if let Some(ref cached) = guard.cached {
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

/// A borrowed view of a [`LazyImage`]'s generated pixels.
///
/// Returned by [`LazyImage::peek_pixels`]. Dereferences to `&[u8]`. The
/// guard holds the image's internal lock, which is what makes the borrow
/// sound: `discard_pixels` (from this or any clone, on any thread) cannot
/// free the buffer while a `PeekedPixels` is alive — it blocks until the
/// guard is dropped. Do not call other `LazyImage` methods on the same
/// thread while holding the guard (self-deadlock).
pub struct PeekedPixels<'a> {
    guard: parking_lot::MutexGuard<'a, GenState>,
}

impl PeekedPixels<'_> {
    /// The row stride, in bytes, of the pixel buffer.
    pub fn row_bytes(&self) -> usize {
        // Invariant: peek_pixels only constructs this guard when `cached`
        // is Some, and the lock is held, so it cannot become None.
        self.guard.cached.as_ref().expect("cached pixels").row_bytes
    }
}

impl std::ops::Deref for PeekedPixels<'_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.guard.cached.as_ref().expect("cached pixels").pixels
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

    /// A generator that sleeps during decode and counts how many times its
    /// pixel-producing path actually ran. Used to prove waiters block on the
    /// generating thread instead of racing into a second decode or erroring.
    struct CountingSlowGenerator {
        info: ImageInfo,
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl ImageGenerator for CountingSlowGenerator {
        fn info(&self) -> &ImageInfo {
            &self.info
        }
        fn on_get_pixels(&self, pixels: &mut [u8], _row_bytes: usize) -> GeneratorResult<()> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(50));
            for b in pixels.iter_mut() {
                *b = 77;
            }
            Ok(())
        }
    }

    /// Concurrent `ensure_pixels_generated` calls must all succeed while the
    /// generator's decode runs exactly once — late arrivals block on the
    /// generating thread rather than erroring with "generation in progress".
    #[test]
    fn test_concurrent_generation_blocks_and_runs_once() {
        use std::thread;

        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let generator = CountingSlowGenerator {
            info: ImageInfo::new(8, 8, ColorType::Rgba8888, AlphaType::Premul),
            calls: Arc::clone(&calls),
        };
        let lazy = Arc::new(LazyImage::from_generator(Box::new(generator)));

        let handles: Vec<_> = (0..6)
            .map(|_| {
                let lazy = Arc::clone(&lazy);
                thread::spawn(move || {
                    // Every caller must observe success, never an error.
                    lazy.ensure_pixels_generated()
                        .expect("waiters must not error");
                    assert!(lazy.is_generated());
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "decode must run exactly once despite concurrent callers"
        );
    }

    /// `peek_pixels` returns `None` before generation (no decode side
    /// effect) and `Some` after the pixels exist.
    #[test]
    fn test_peek_pixels_only_after_generation() {
        let generator = SolidColorGenerator::new(3, 3, [9, 8, 7, 255]);
        let lazy = LazyImage::from_generator(Box::new(generator));

        // Peeking must not trigger a decode.
        assert!(lazy.peek_pixels().is_none());
        assert_eq!(lazy.state(), LazyImageState::NotGenerated);

        lazy.ensure_pixels_generated().unwrap();
        let px = lazy
            .peek_pixels()
            .expect("pixels available after generation");
        assert_eq!(&px[0..4], &[9, 8, 7, 255]);
    }

    /// The `PeekedPixels` guard holds the internal lock, so a concurrent
    /// `discard_pixels` through a *clone* blocks until the guard drops —
    /// the peeked bytes can never dangle.
    #[test]
    fn test_peek_pixels_guard_blocks_concurrent_discard() {
        use std::thread;

        let generator = SolidColorGenerator::new(2, 2, [5, 6, 7, 255]);
        let lazy = LazyImage::from_generator(Box::new(generator));
        lazy.ensure_pixels_generated().unwrap();

        let px = lazy.peek_pixels().expect("generated");
        let clone = lazy.clone();
        let discarder = thread::spawn(move || {
            // Must block until the guard below is dropped.
            clone.discard_pixels();
        });

        // Give the discarder ample time to run; the buffer must still be
        // intact because the guard holds the lock.
        thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(&px[0..4], &[5, 6, 7, 255], "pixels valid while guard held");
        drop(px);

        discarder.join().unwrap();
        assert!(!lazy.is_generated(), "discard proceeded after guard drop");
    }

    /// A generator that panics during decode.
    struct PanickingGenerator {
        info: ImageInfo,
    }

    impl ImageGenerator for PanickingGenerator {
        fn info(&self) -> &ImageInfo {
            &self.info
        }
        fn on_get_pixels(&self, _pixels: &mut [u8], _row_bytes: usize) -> GeneratorResult<()> {
            panic!("generator blew up");
        }
    }

    /// If the generator panics mid-decode, the state must flip to `Failed`
    /// and condvar waiters must be woken — never left blocking forever on a
    /// state stuck at `Generating` (parking_lot does not poison).
    #[test]
    fn test_generator_panic_fails_waiters_instead_of_hanging() {
        use std::thread;

        let generator = PanickingGenerator {
            info: ImageInfo::new(2, 2, ColorType::Rgba8888, AlphaType::Premul),
        };
        let lazy = Arc::new(LazyImage::from_generator(Box::new(generator)));

        // First caller panics (the unwind guard re-raises the panic after
        // publishing Failed).
        let l1 = Arc::clone(&lazy);
        let panicker = thread::spawn(move || {
            let _ = l1.ensure_pixels_generated();
        });
        assert!(panicker.join().is_err(), "generator panic propagates");

        // Late caller must observe Failed and get an Err — not hang.
        assert_eq!(lazy.state(), LazyImageState::Failed);
        assert!(lazy.ensure_pixels_generated().is_err());
        assert!(lazy.peek_pixels().is_none());
    }

    /// `read_pixels` must report failure (no partial copy) when the
    /// destination cannot hold the whole image.
    #[test]
    fn test_read_pixels_rejects_small_destination() {
        let generator = SolidColorGenerator::new(4, 4, [1, 2, 3, 255]);
        let lazy = LazyImage::from_generator(Box::new(generator));

        let mut too_small = vec![0u8; 4 * 4 * 4 - 1];
        assert!(!lazy.read_pixels(&mut too_small, 4 * 4));
        // Nothing should have been written.
        assert!(too_small.iter().all(|&b| b == 0));

        let mut ok = vec![0u8; 4 * 4 * 4];
        assert!(lazy.read_pixels(&mut ok, 4 * 4));
    }

    /// A hostile near-`usize::MAX` `dst_row_bytes` must not wrap the
    /// destination-fits precheck's multiply; `read_pixels` must return
    /// `false` instead of panicking (overflow) or copying out of bounds.
    #[test]
    fn test_read_pixels_rejects_overflowing_row_bytes() {
        let generator = SolidColorGenerator::new(4, 4, [1, 2, 3, 255]);
        let lazy = LazyImage::from_generator(Box::new(generator));

        let mut dst = vec![0u8; 4 * 4 * 4];
        assert!(!lazy.read_pixels(&mut dst, usize::MAX - 1));
    }
}
