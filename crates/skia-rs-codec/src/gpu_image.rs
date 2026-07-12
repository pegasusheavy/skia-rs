//! GPU-backed images.
//!
//! GPU images represent images that can be efficiently rendered on the GPU.
//! This module provides the data structures and interfaces; actual GPU operations
//! are handled by the `skia-rs-gpu` crate.

use crate::ImageInfo;
use skia_rs_core::{AlphaType, ColorType, Rect};
use std::sync::Arc;

/// Origin of a GPU texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpuSurfaceOrigin {
    /// Top-left origin (standard).
    #[default]
    TopLeft,
    /// Bottom-left origin (OpenGL style).
    BottomLeft,
}

/// Caching hint for GPU images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpuImageCachingHint {
    /// Allow the image to be cached.
    #[default]
    Allow,
    /// Disallow caching.
    Disallow,
}

/// Texture format for GPU images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpuTextureFormat {
    /// RGBA 8-bit unsigned normalized.
    #[default]
    Rgba8Unorm,
    /// RGBA 8-bit unsigned normalized sRGB.
    Rgba8UnormSrgb,
    /// BGRA 8-bit unsigned normalized.
    Bgra8Unorm,
    /// BGRA 8-bit unsigned normalized sRGB.
    Bgra8UnormSrgb,
    /// RGB 10-bit, A 2-bit unsigned normalized.
    Rgb10a2Unorm,
    /// RGBA 16-bit float.
    Rgba16Float,
}

impl GpuTextureFormat {
    /// Get bytes per pixel for this format.
    #[inline]
    #[must_use] 
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Rgba8Unorm
            | Self::Rgba8UnormSrgb
            | Self::Bgra8Unorm
            | Self::Bgra8UnormSrgb
            | Self::Rgb10a2Unorm => 4,
            Self::Rgba16Float => 8,
        }
    }

    /// Convert from `ColorType`.
    #[must_use] 
    pub const fn from_color_type(color_type: ColorType) -> Option<Self> {
        match color_type {
            ColorType::Rgba8888 => Some(Self::Rgba8Unorm),
            ColorType::Bgra8888 => Some(Self::Bgra8Unorm),
            _ => None,
        }
    }

    /// Convert to `ColorType`.
    #[must_use] 
    pub const fn to_color_type(&self) -> ColorType {
        match self {
            // `Rgb10a2Unorm`/`Rgba16Float` have no exact `ColorType` match;
            // approximate as `Rgba8888`.
            Self::Rgba8Unorm | Self::Rgba8UnormSrgb | Self::Rgb10a2Unorm | Self::Rgba16Float => {
                ColorType::Rgba8888
            }
            Self::Bgra8Unorm | Self::Bgra8UnormSrgb => ColorType::Bgra8888,
        }
    }
}

/// Errors from GPU image backend operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GpuImageError {
    /// Upload failed.
    #[error("GPU upload failed: {0}")]
    UploadFailed(String),
    /// Read-back failed.
    #[error("GPU read-back failed: {0}")]
    ReadBackFailed(String),
    /// Backend unavailable (no texture handle, no context).
    #[error("GPU backend unavailable: {0}")]
    BackendUnavailable(String),
    /// Color format not supported by backend.
    #[error("Unsupported format: {0:?}")]
    UnsupportedFormat(GpuTextureFormat),
}

/// Backend contract for wiring raster pixels to/from GPU textures.
///
/// Implementors live in the `skia-rs-gpu` crate (or a user-provided
/// shim) and install themselves on a `GpuImage` via
/// [`GpuImage::set_backend`]. When installed, `GpuImage::upload` and
/// `GpuImage::read_pixels_from_gpu` delegate to the backend; when no
/// backend is installed, both methods fall back to the raster cache
/// (if present) so offline construction and testing remain possible
/// without a GPU context.
///
/// # Thread safety
///
/// Implementations must be `Send + Sync` because `GpuImage` is `Clone`
/// and may be passed between threads. Backends that hold a GPU device
/// handle typically already satisfy this contract.
pub trait GpuImageBackend: Send + Sync + std::fmt::Debug {
    /// Upload raster pixels to a new GPU texture, returning the backend
    /// handle. Called at most once per `GpuImage` when the raster cache
    /// is pushed to GPU.
    ///
    /// # Errors
    /// Returns [`GpuImageError`] if the backend rejects the upload (e.g.
    /// unsupported format or device-side failure).
    fn upload(
        &self,
        info: &ImageInfo,
        format: GpuTextureFormat,
        pixels: &[u8],
        row_bytes: usize,
    ) -> Result<GpuTextureHandle, GpuImageError>;

    /// Read back pixels from the GPU texture into the destination
    /// buffer in the specified info's layout.
    ///
    /// # Errors
    /// Returns [`GpuImageError`] if the read-back fails (e.g. an invalid
    /// handle or a device-side failure).
    fn read_back(
        &self,
        handle: &GpuTextureHandle,
        info: &ImageInfo,
        format: GpuTextureFormat,
        dst: &mut [u8],
        dst_row_bytes: usize,
    ) -> Result<(), GpuImageError>;

    /// Release a GPU texture. Called when the `GpuImage` is dropped
    /// or when `clear_texture_handle` is invoked. Backends should
    /// free device memory here.
    fn release(&self, handle: &GpuTextureHandle);
}

/// A shared GPU image backend.
pub type GpuImageBackendRef = Arc<dyn GpuImageBackend>;

/// Allocate the next process-wide unique GPU image ID from a monotonic
/// atomic counter, mirroring [`crate::image`]'s `next_image_id`.
fn next_gpu_image_id() -> u64 {
    static ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// A GPU-backed image.
///
/// Unlike raster `Image`, `GpuImage` is designed to store pixels in GPU memory.
/// This enables:
/// - Faster rendering when drawing to GPU surfaces
/// - No CPU-GPU transfer on each draw
/// - GPU-side filtering and transformation
///
/// # Creating GPU Images
///
/// GPU images can be created from:
/// - Raster pixel data (to be uploaded to GPU)
/// - Backend-specific texture handles
///
/// GPU upload/read-back is delegated to a [`GpuImageBackend`] installed
/// via [`GpuImage::set_backend`]. The `skia-rs-gpu` crate provides
/// concrete backends for Vulkan, Metal, OpenGL, D3D12, and WebGPU; the
/// wiring between those backends and this trait is scheduled as part
/// of the skia-rs-gpu completion work (Phase 6F). Until then,
/// `GpuImage` still exposes a fully functional raster-cache fast path,
/// sufficient for rendering tests and non-GPU consumers.
///
/// # Memory Model
///
/// `GpuImage` maintains both:
/// - A reference to GPU texture (when uploaded)
/// - An optional raster cache for CPU readback
#[derive(Clone)]
pub struct GpuImage {
    inner: Arc<GpuImageInner>,
}

struct GpuImageInner {
    info: ImageInfo,
    texture_format: GpuTextureFormat,
    /// Unique identifier for this GPU image.
    unique_id: u64,
    origin: GpuSurfaceOrigin,
    /// GPU texture handle (backend-specific).
    texture_handle: parking_lot::RwLock<Option<GpuTextureHandle>>,
    /// Cached raster copy for upload and read-back.
    raster_cache: parking_lot::RwLock<Option<Vec<u8>>>,
    row_bytes: usize,
    /// Backend driver, if installed.
    backend: parking_lot::RwLock<Option<GpuImageBackendRef>>,
}

/// A backend-agnostic GPU texture handle.
#[derive(Debug, Clone)]
pub struct GpuTextureHandle {
    /// Backend-specific texture ID or pointer.
    pub id: u64,
    /// Backend type identifier.
    pub backend: GpuBackend,
}

/// GPU backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackend {
    /// Vulkan backend.
    Vulkan,
    /// Metal backend.
    Metal,
    /// Direct3D 12 backend.
    D3D12,
    /// OpenGL backend.
    OpenGL,
    /// WebGPU backend.
    WebGpu,
    /// Software/CPU fallback.
    Software,
}

impl std::fmt::Debug for GpuImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuImage")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("color_type", &self.color_type())
            .field("format", &self.inner.texture_format)
            .field("origin", &self.inner.origin)
            .field("has_texture", &self.has_texture())
            .finish()
    }
}

impl GpuImage {
    /// Create a GPU image from raster pixel data.
    ///
    /// The pixels are stored for later upload to GPU.
    #[must_use]
    pub fn from_raster_data(info: &ImageInfo, pixels: &[u8], row_bytes: usize) -> Option<Self> {
        if info.is_empty() {
            return None;
        }

        let expected_size = info.compute_byte_size(row_bytes);
        if pixels.len() < expected_size {
            return None;
        }

        let texture_format = GpuTextureFormat::from_color_type(info.color_type).unwrap_or_default();

        let unique_id = next_gpu_image_id();

        Some(Self {
            inner: Arc::new(GpuImageInner {
                info: info.clone(),
                texture_format,
                unique_id,
                origin: GpuSurfaceOrigin::TopLeft,
                texture_handle: parking_lot::RwLock::new(None),
                raster_cache: parking_lot::RwLock::new(Some(pixels[..expected_size].to_vec())),
                row_bytes,
                backend: parking_lot::RwLock::new(None),
            }),
        })
    }

    /// Create a GPU image from owned pixel data.
    #[must_use]
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

        let texture_format = GpuTextureFormat::from_color_type(info.color_type).unwrap_or_default();

        let unique_id = next_gpu_image_id();

        Some(Self {
            inner: Arc::new(GpuImageInner {
                info,
                texture_format,
                unique_id,
                origin: GpuSurfaceOrigin::TopLeft,
                texture_handle: parking_lot::RwLock::new(None),
                raster_cache: parking_lot::RwLock::new(Some(pixels)),
                row_bytes,
                backend: parking_lot::RwLock::new(None),
            }),
        })
    }

    /// Create a GPU image from an existing texture handle.
    ///
    /// This creates a `GpuImage` that references an already-uploaded texture.
    #[must_use]
    pub fn from_texture(
        info: ImageInfo,
        handle: GpuTextureHandle,
        origin: GpuSurfaceOrigin,
    ) -> Option<Self> {
        if info.is_empty() {
            return None;
        }

        let texture_format = GpuTextureFormat::from_color_type(info.color_type).unwrap_or_default();
        let row_bytes = info.min_row_bytes();

        let unique_id = next_gpu_image_id();

        Some(Self {
            inner: Arc::new(GpuImageInner {
                info,
                texture_format,
                unique_id,
                origin,
                texture_handle: parking_lot::RwLock::new(Some(handle)),
                raster_cache: parking_lot::RwLock::new(None),
                row_bytes,
                backend: parking_lot::RwLock::new(None),
            }),
        })
    }

    /// Get the image width.
    #[inline]
    #[must_use] 
    pub fn width(&self) -> i32 {
        self.inner.info.width
    }

    /// Get the image height.
    #[inline]
    #[must_use] 
    pub fn height(&self) -> i32 {
        self.inner.info.height
    }

    /// Get the image dimensions as (width, height).
    #[inline]
    #[must_use] 
    pub fn dimensions(&self) -> (i32, i32) {
        (self.width(), self.height())
    }

    /// Get the image bounds as a rectangle.
    #[inline]
    #[must_use] 
    pub fn bounds(&self) -> Rect {
        Rect::from_xywh(
            0.0,
            0.0,
            skia_rs_core::cast::scalar_from_i32(self.width()),
            skia_rs_core::cast::scalar_from_i32(self.height()),
        )
    }

    /// Get the image info.
    #[inline]
    #[must_use] 
    pub fn info(&self) -> &ImageInfo {
        &self.inner.info
    }

    /// Get the color type.
    #[inline]
    #[must_use] 
    pub fn color_type(&self) -> ColorType {
        self.inner.info.color_type
    }

    /// Get the alpha type.
    #[inline]
    #[must_use] 
    pub fn alpha_type(&self) -> AlphaType {
        self.inner.info.alpha_type
    }

    /// Get the texture format.
    #[inline]
    #[must_use] 
    pub fn texture_format(&self) -> GpuTextureFormat {
        self.inner.texture_format
    }

    /// Get the surface origin.
    #[inline]
    #[must_use] 
    pub fn origin(&self) -> GpuSurfaceOrigin {
        self.inner.origin
    }

    /// Get the unique ID.
    #[inline]
    #[must_use] 
    pub fn unique_id(&self) -> u64 {
        self.inner.unique_id
    }

    /// Returns true if the image is opaque.
    #[inline]
    #[must_use] 
    pub fn is_opaque(&self) -> bool {
        self.inner.info.is_opaque()
    }

    /// Check if this GPU image is still valid.
    #[must_use] 
    pub fn is_valid(&self) -> bool {
        !self.inner.info.is_empty()
    }

    /// Check if a GPU texture has been created.
    #[must_use] 
    pub fn has_texture(&self) -> bool {
        self.inner.texture_handle.read().is_some()
    }

    /// Get the texture handle, if available.
    #[must_use] 
    pub fn texture_handle(&self) -> Option<GpuTextureHandle> {
        self.inner.texture_handle.read().clone()
    }

    /// Set the texture handle (called by GPU backend after upload).
    pub fn set_texture_handle(&self, handle: GpuTextureHandle) {
        *self.inner.texture_handle.write() = Some(handle);
    }

    /// Clear the texture handle, calling the backend's `release` if a
    /// backend is installed. Safe to call multiple times.
    pub fn clear_texture_handle(&self) {
        let handle = self.inner.texture_handle.write().take();
        if let Some(handle) = handle {
            let backend = self.inner.backend.read().clone();
            if let Some(backend) = backend {
                backend.release(&handle);
            }
        }
    }

    /// Check if raster data is available.
    #[must_use] 
    pub fn has_raster_data(&self) -> bool {
        self.inner.raster_cache.read().is_some()
    }

    /// Get a reference to the raster data for upload.
    #[must_use] 
    pub fn peek_raster_pixels(&self) -> Option<Vec<u8>> {
        self.inner.raster_cache.read().clone()
    }

    /// Get the row bytes for the raster data.
    #[must_use] 
    pub fn row_bytes(&self) -> usize {
        self.inner.row_bytes
    }

    /// Install a GPU backend driver. When set, `upload` and
    /// `read_pixels` will delegate to the backend; otherwise both use
    /// the raster cache.
    pub fn set_backend(&self, backend: GpuImageBackendRef) {
        *self.inner.backend.write() = Some(backend);
    }

    /// Get the installed backend, if any.
    #[must_use] 
    pub fn backend(&self) -> Option<GpuImageBackendRef> {
        self.inner.backend.read().clone()
    }

    /// Upload raster pixels to GPU via the installed backend.
    ///
    /// Returns `Ok(())` on success. Fails with `BackendUnavailable` if
    /// no backend is installed; fails with `UploadFailed` if the
    /// backend rejects the upload. On success the texture handle is
    /// stored and `has_texture()` returns true.
    ///
    /// # Errors
    /// Returns [`GpuImageError::BackendUnavailable`] if no backend is
    /// installed or there is no cached raster data to upload, and
    /// propagates any backend-reported upload failure.
    pub fn upload(&self) -> Result<(), GpuImageError> {
        let backend = self
            .inner
            .backend
            .read()
            .clone()
            .ok_or_else(|| GpuImageError::BackendUnavailable("no backend set".into()))?;

        let cache = self.inner.raster_cache.read();
        let cached = cache
            .as_ref()
            .ok_or_else(|| GpuImageError::UploadFailed("no raster data to upload".into()))?;

        let handle = backend.upload(
            &self.inner.info,
            self.inner.texture_format,
            cached,
            self.inner.row_bytes,
        )?;
        drop(cache);
        *self.inner.texture_handle.write() = Some(handle);
        Ok(())
    }

    /// Read pixels from the GPU texture via the installed backend.
    ///
    /// Prefer this over `read_pixels` when the image is GPU-resident;
    /// `read_pixels` favors the raster cache which may be stale after
    /// a GPU-side modification.
    ///
    /// # Errors
    /// Returns [`GpuImageError::BackendUnavailable`] if no backend or no
    /// texture handle is set, and propagates any backend-reported
    /// read-back failure.
    pub fn read_pixels_from_gpu(
        &self,
        dst: &mut [u8],
        dst_row_bytes: usize,
    ) -> Result<(), GpuImageError> {
        let backend = self
            .inner
            .backend
            .read()
            .clone()
            .ok_or_else(|| GpuImageError::BackendUnavailable("no backend set".into()))?;

        let handle = self
            .inner
            .texture_handle
            .read()
            .clone()
            .ok_or_else(|| GpuImageError::BackendUnavailable("no texture handle".into()))?;

        backend.read_back(
            &handle,
            &self.inner.info,
            self.inner.texture_format,
            dst,
            dst_row_bytes,
        )
    }

    /// Read pixels from the GPU image back to CPU memory.
    ///
    /// Resolution order:
    /// 1. If the raster cache is populated, copy from it (fast path).
    /// 2. If a backend is installed and a texture handle is set, call
    ///    the backend's `read_back`.
    /// 3. Otherwise return `false`.
    pub fn read_pixels(&self, dst: &mut [u8], dst_row_bytes: usize) -> bool {
        let cache = self.inner.raster_cache.read();
        if let Some(ref cached) = *cache {
            let bytes_per_pixel = self.color_type().bytes_per_pixel();
            let width = usize::try_from(self.width()).unwrap_or(0);
            let height = usize::try_from(self.height()).unwrap_or(0);
            let src_row_bytes = self.inner.row_bytes;
            let copy_len = width * bytes_per_pixel;

            // Validate the destination can hold every row before copying, so
            // a too-small buffer reports failure instead of a partially
            // filled result.
            if height > 0 {
                let last_dst = (height - 1) * dst_row_bytes + copy_len;
                let last_src = (height - 1) * src_row_bytes + copy_len;
                if last_dst > dst.len() || last_src > cached.len() {
                    return false;
                }
            }

            for y in 0..height {
                let src_offset = y * src_row_bytes;
                let dst_offset = y * dst_row_bytes;
                dst[dst_offset..dst_offset + copy_len]
                    .copy_from_slice(&cached[src_offset..src_offset + copy_len]);
            }
            return true;
        }
        drop(cache);

        // Fall back to backend read-back.
        self.read_pixels_from_gpu(dst, dst_row_bytes).is_ok()
    }

    /// Store raster data read back from GPU.
    pub fn cache_raster_pixels(&self, pixels: Vec<u8>) {
        *self.inner.raster_cache.write() = Some(pixels);
    }

    /// Discard cached raster data to free memory.
    ///
    /// The GPU texture (if any) is retained.
    pub fn discard_raster_cache(&self) {
        *self.inner.raster_cache.write() = None;
    }

    /// Convert to a raster image.
    ///
    /// Returns `None` if no raster data is available.
    #[must_use] 
    pub fn to_raster(&self) -> Option<crate::Image> {
        let cache = self.inner.raster_cache.read();
        cache.as_ref().and_then(|cached| {
            crate::Image::from_raster_data(&self.inner.info, cached, self.inner.row_bytes)
        })
    }

    /// Create a subset of this GPU image.
    #[must_use] 
    pub fn make_subset(&self, subset: &Rect) -> Option<Self> {
        let x = skia_rs_core::cast::saturate_to_i32(subset.left);
        let y = skia_rs_core::cast::saturate_to_i32(subset.top);
        let w = skia_rs_core::cast::saturate_to_i32(subset.width());
        let h = skia_rs_core::cast::saturate_to_i32(subset.height());

        if x < 0 || y < 0 || w <= 0 || h <= 0 {
            return None;
        }
        if x + w > self.width() || y + h > self.height() {
            return None;
        }

        let (Ok(x), Ok(y), Ok(w_usize), Ok(h_usize)) = (
            usize::try_from(x),
            usize::try_from(y),
            usize::try_from(w),
            usize::try_from(h),
        ) else {
            return None;
        };

        let cache = self.inner.raster_cache.read();
        cache.as_ref().and_then(|cached| {
            let new_info = ImageInfo::new(w, h, self.color_type(), self.alpha_type());
            let bytes_per_pixel = self.color_type().bytes_per_pixel();
            let src_row_bytes = self.inner.row_bytes;
            let new_row_bytes = w_usize * bytes_per_pixel;
            let mut new_pixels = vec![0u8; h_usize * new_row_bytes];

            for row in 0..h_usize {
                let src_offset = (y + row) * src_row_bytes + x * bytes_per_pixel;
                let dst_offset = row * new_row_bytes;

                new_pixels[dst_offset..dst_offset + new_row_bytes]
                    .copy_from_slice(&cached[src_offset..src_offset + new_row_bytes]);
            }

            Self::from_raster_data_owned(new_info, new_pixels, new_row_bytes)
        })
    }
}

impl Drop for GpuImageInner {
    fn drop(&mut self) {
        // Release the GPU handle through the backend, if both are set.
        // The inner values are dropped exclusively when the last Arc
        // drops, so no locking race is possible here.
        if let (Some(handle), Some(backend)) = (
            self.texture_handle.get_mut().take(),
            self.backend.get_mut().clone(),
        ) {
            backend.release(&handle);
        }
    }
}

/// A reference to a GPU image.
pub type GpuImageRef = Arc<GpuImage>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_image_creation() {
        let info = ImageInfo::new(100, 100, ColorType::Rgba8888, AlphaType::Premul);
        let pixels = vec![255u8; 100 * 100 * 4];

        let image = GpuImage::from_raster_data(&info, &pixels, 100 * 4).unwrap();
        assert_eq!(image.width(), 100);
        assert_eq!(image.height(), 100);
        assert!(!image.has_texture());
        assert!(image.has_raster_data());
    }

    #[test]
    fn test_gpu_image_read_pixels() {
        let info = ImageInfo::new(10, 10, ColorType::Rgba8888, AlphaType::Premul);
        let pixels = vec![128u8; 10 * 10 * 4];

        let image = GpuImage::from_raster_data(&info, &pixels, 10 * 4).unwrap();

        let mut dst = vec![0u8; 10 * 10 * 4];
        assert!(image.read_pixels(&mut dst, 10 * 4));
        assert_eq!(dst[0], 128);
    }

    #[test]
    fn test_gpu_image_read_pixels_rejects_small_destination() {
        let info = ImageInfo::new(10, 10, ColorType::Rgba8888, AlphaType::Premul);
        let pixels = vec![128u8; 10 * 10 * 4];
        let image = GpuImage::from_raster_data(&info, &pixels, 10 * 4).unwrap();

        // Destination one byte short of the full image: must fail cleanly
        // without a partial copy.
        let mut dst = vec![0u8; 10 * 10 * 4 - 1];
        assert!(!image.read_pixels(&mut dst, 10 * 4));
        assert!(dst.iter().all(|&b| b == 0), "no partial copy on failure");
    }

    #[test]
    fn test_gpu_image_subset() {
        let info = ImageInfo::new(100, 100, ColorType::Rgba8888, AlphaType::Premul);
        let pixels = vec![255u8; 100 * 100 * 4];

        let image = GpuImage::from_raster_data(&info, &pixels, 100 * 4).unwrap();
        let subset = image
            .make_subset(&Rect::from_xywh(25.0, 25.0, 50.0, 50.0))
            .unwrap();

        assert_eq!(subset.dimensions(), (50, 50));
    }

    #[test]
    fn test_gpu_image_texture_handle() {
        let info = ImageInfo::new(64, 64, ColorType::Rgba8888, AlphaType::Premul);
        let pixels = vec![0u8; 64 * 64 * 4];

        let image = GpuImage::from_raster_data(&info, &pixels, 64 * 4).unwrap();
        assert!(!image.has_texture());

        // Simulate GPU upload
        let handle = GpuTextureHandle {
            id: 12345,
            backend: GpuBackend::WebGpu,
        };
        image.set_texture_handle(handle);

        assert!(image.has_texture());
        let retrieved = image.texture_handle().unwrap();
        assert_eq!(retrieved.id, 12345);
    }

    #[test]
    fn test_texture_format() {
        assert_eq!(GpuTextureFormat::Rgba8Unorm.bytes_per_pixel(), 4);
        assert_eq!(GpuTextureFormat::Rgba16Float.bytes_per_pixel(), 8);

        let format = GpuTextureFormat::from_color_type(ColorType::Rgba8888).unwrap();
        assert_eq!(format, GpuTextureFormat::Rgba8Unorm);
    }

    /// An in-memory mock backend that round-trips pixels through a
    /// virtual "GPU store" indexed by handle id. Used to verify that
    /// `GpuImage` correctly delegates to the backend trait for upload,
    /// read-back, and release.
    #[derive(Debug)]
    struct MockBackend {
        store: parking_lot::Mutex<std::collections::HashMap<u64, Vec<u8>>>,
        next_id: std::sync::atomic::AtomicU64,
        release_count: std::sync::atomic::AtomicU64,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                store: parking_lot::Mutex::new(std::collections::HashMap::new()),
                next_id: std::sync::atomic::AtomicU64::new(1),
                release_count: std::sync::atomic::AtomicU64::new(0),
            }
        }

        fn release_count(&self) -> u64 {
            self.release_count
                .load(std::sync::atomic::Ordering::Relaxed)
        }
    }

    impl GpuImageBackend for MockBackend {
        fn upload(
            &self,
            _info: &ImageInfo,
            _format: GpuTextureFormat,
            pixels: &[u8],
            _row_bytes: usize,
        ) -> Result<GpuTextureHandle, GpuImageError> {
            let id = self
                .next_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.store.lock().insert(id, pixels.to_vec());
            Ok(GpuTextureHandle {
                id,
                backend: GpuBackend::Software,
            })
        }

        fn read_back(
            &self,
            handle: &GpuTextureHandle,
            _info: &ImageInfo,
            _format: GpuTextureFormat,
            dst: &mut [u8],
            _dst_row_bytes: usize,
        ) -> Result<(), GpuImageError> {
            let store = self.store.lock();
            let src = store.get(&handle.id).ok_or_else(|| {
                GpuImageError::ReadBackFailed(format!("unknown handle {}", handle.id))
            })?;
            let len = src.len().min(dst.len());
            dst[..len].copy_from_slice(&src[..len]);
            drop(store);
            Ok(())
        }

        fn release(&self, handle: &GpuTextureHandle) {
            self.store.lock().remove(&handle.id);
            self.release_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    #[test]
    fn test_backend_upload_and_readback_roundtrip() {
        let info = ImageInfo::new(2, 2, ColorType::Rgba8888, AlphaType::Premul);
        let src_pixels = vec![
            10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160,
        ];
        let image = GpuImage::from_raster_data(&info, &src_pixels, 8).unwrap();

        let backend: Arc<MockBackend> = Arc::new(MockBackend::new());
        image.set_backend(backend as Arc<dyn GpuImageBackend>);

        // No texture before upload.
        assert!(!image.has_texture());
        image.upload().expect("upload should succeed");
        assert!(image.has_texture());

        // Read back via GPU path: drop the raster cache first so we
        // truly exercise the backend.
        image.discard_raster_cache();
        assert!(!image.has_raster_data());

        let mut dst = vec![0u8; 16];
        image
            .read_pixels_from_gpu(&mut dst, 8)
            .expect("read_back should succeed");
        assert_eq!(dst, src_pixels);
    }

    #[test]
    fn test_backend_release_on_clear_handle() {
        let info = ImageInfo::new(1, 1, ColorType::Rgba8888, AlphaType::Premul);
        let pixels = vec![0, 0, 0, 0];
        let image = GpuImage::from_raster_data(&info, &pixels, 4).unwrap();

        let backend: Arc<MockBackend> = Arc::new(MockBackend::new());
        image.set_backend(backend.clone() as Arc<dyn GpuImageBackend>);
        image.upload().unwrap();
        assert_eq!(backend.release_count(), 0);

        image.clear_texture_handle();
        assert_eq!(backend.release_count(), 1);
        assert!(!image.has_texture());
    }

    #[test]
    fn test_backend_release_on_drop() {
        let info = ImageInfo::new(1, 1, ColorType::Rgba8888, AlphaType::Premul);
        let pixels = vec![0, 0, 0, 0];
        let backend: Arc<MockBackend> = Arc::new(MockBackend::new());

        {
            let image = GpuImage::from_raster_data(&info, &pixels, 4).unwrap();
            image.set_backend(backend.clone() as Arc<dyn GpuImageBackend>);
            image.upload().unwrap();
            assert_eq!(backend.release_count(), 0);
            // image drops here
        }
        assert_eq!(backend.release_count(), 1);
    }

    #[test]
    fn test_upload_without_backend_errors() {
        let info = ImageInfo::new(1, 1, ColorType::Rgba8888, AlphaType::Premul);
        let pixels = vec![0, 0, 0, 0];
        let image = GpuImage::from_raster_data(&info, &pixels, 4).unwrap();

        let err = image.upload().expect_err("must fail without backend");
        assert!(matches!(err, GpuImageError::BackendUnavailable(_)));
    }

    #[test]
    fn test_read_pixels_falls_back_to_backend() {
        // With no raster cache and a backend + texture handle,
        // read_pixels should serve via the backend.
        let info = ImageInfo::new(1, 1, ColorType::Rgba8888, AlphaType::Premul);
        let pixels = vec![42, 43, 44, 45];
        let image = GpuImage::from_raster_data(&info, &pixels, 4).unwrap();
        let backend: Arc<MockBackend> = Arc::new(MockBackend::new());
        image.set_backend(backend as Arc<dyn GpuImageBackend>);
        image.upload().unwrap();
        image.discard_raster_cache();

        let mut dst = vec![0u8; 4];
        assert!(image.read_pixels(&mut dst, 4));
        assert_eq!(dst, pixels);
    }
}
