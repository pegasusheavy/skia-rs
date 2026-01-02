//! GPU-backed images.
//!
//! GPU images represent images that can be efficiently rendered on the GPU.
//! This module provides the data structures and interfaces; actual GPU operations
//! are handled by the `skia-rs-gpu` crate.

use crate::ImageInfo;
use skia_rs_core::{AlphaType, ColorType, Rect, Scalar};
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
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Rgba8Unorm | Self::Rgba8UnormSrgb | Self::Bgra8Unorm | Self::Bgra8UnormSrgb => 4,
            Self::Rgb10a2Unorm => 4,
            Self::Rgba16Float => 8,
        }
    }

    /// Convert from ColorType.
    pub fn from_color_type(color_type: ColorType) -> Option<Self> {
        match color_type {
            ColorType::Rgba8888 => Some(Self::Rgba8Unorm),
            ColorType::Bgra8888 => Some(Self::Bgra8Unorm),
            _ => None,
        }
    }

    /// Convert to ColorType.
    pub fn to_color_type(&self) -> ColorType {
        match self {
            Self::Rgba8Unorm | Self::Rgba8UnormSrgb => ColorType::Rgba8888,
            Self::Bgra8Unorm | Self::Bgra8UnormSrgb => ColorType::Bgra8888,
            Self::Rgb10a2Unorm | Self::Rgba16Float => ColorType::Rgba8888, // Approximation
        }
    }
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
/// The actual GPU upload is handled by the `skia-rs-gpu` crate.
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
    pub fn from_raster_data(info: &ImageInfo, pixels: &[u8], row_bytes: usize) -> Option<Self> {
        if info.is_empty() {
            return None;
        }

        let expected_size = info.compute_byte_size(row_bytes);
        if pixels.len() < expected_size {
            return None;
        }

        let texture_format =
            GpuTextureFormat::from_color_type(info.color_type).unwrap_or_default();

        static ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let unique_id = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Some(Self {
            inner: Arc::new(GpuImageInner {
                info: info.clone(),
                texture_format,
                unique_id,
                origin: GpuSurfaceOrigin::TopLeft,
                texture_handle: parking_lot::RwLock::new(None),
                raster_cache: parking_lot::RwLock::new(Some(pixels[..expected_size].to_vec())),
                row_bytes,
            }),
        })
    }

    /// Create a GPU image from owned pixel data.
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

        let texture_format =
            GpuTextureFormat::from_color_type(info.color_type).unwrap_or_default();

        static ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let unique_id = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Some(Self {
            inner: Arc::new(GpuImageInner {
                info,
                texture_format,
                unique_id,
                origin: GpuSurfaceOrigin::TopLeft,
                texture_handle: parking_lot::RwLock::new(None),
                raster_cache: parking_lot::RwLock::new(Some(pixels)),
                row_bytes,
            }),
        })
    }

    /// Create a GPU image from an existing texture handle.
    ///
    /// This creates a GpuImage that references an already-uploaded texture.
    pub fn from_texture(
        info: ImageInfo,
        handle: GpuTextureHandle,
        origin: GpuSurfaceOrigin,
    ) -> Option<Self> {
        if info.is_empty() {
            return None;
        }

        let texture_format =
            GpuTextureFormat::from_color_type(info.color_type).unwrap_or_default();
        let row_bytes = info.min_row_bytes();

        static ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let unique_id = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Some(Self {
            inner: Arc::new(GpuImageInner {
                info,
                texture_format,
                unique_id,
                origin,
                texture_handle: parking_lot::RwLock::new(Some(handle)),
                raster_cache: parking_lot::RwLock::new(None),
                row_bytes,
            }),
        })
    }

    /// Get the image width.
    #[inline]
    pub fn width(&self) -> i32 {
        self.inner.info.width
    }

    /// Get the image height.
    #[inline]
    pub fn height(&self) -> i32 {
        self.inner.info.height
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
        self.inner.info.color_type
    }

    /// Get the alpha type.
    #[inline]
    pub fn alpha_type(&self) -> AlphaType {
        self.inner.info.alpha_type
    }

    /// Get the texture format.
    #[inline]
    pub fn texture_format(&self) -> GpuTextureFormat {
        self.inner.texture_format
    }

    /// Get the surface origin.
    #[inline]
    pub fn origin(&self) -> GpuSurfaceOrigin {
        self.inner.origin
    }

    /// Get the unique ID.
    #[inline]
    pub fn unique_id(&self) -> u64 {
        self.inner.unique_id
    }

    /// Returns true if the image is opaque.
    #[inline]
    pub fn is_opaque(&self) -> bool {
        self.inner.info.is_opaque()
    }

    /// Check if this GPU image is still valid.
    pub fn is_valid(&self) -> bool {
        !self.inner.info.is_empty()
    }

    /// Check if a GPU texture has been created.
    pub fn has_texture(&self) -> bool {
        self.inner.texture_handle.read().is_some()
    }

    /// Get the texture handle, if available.
    pub fn texture_handle(&self) -> Option<GpuTextureHandle> {
        self.inner.texture_handle.read().clone()
    }

    /// Set the texture handle (called by GPU backend after upload).
    pub fn set_texture_handle(&self, handle: GpuTextureHandle) {
        *self.inner.texture_handle.write() = Some(handle);
    }

    /// Clear the texture handle (texture was destroyed).
    pub fn clear_texture_handle(&self) {
        *self.inner.texture_handle.write() = None;
    }

    /// Check if raster data is available.
    pub fn has_raster_data(&self) -> bool {
        self.inner.raster_cache.read().is_some()
    }

    /// Get a reference to the raster data for upload.
    pub fn peek_raster_pixels(&self) -> Option<Vec<u8>> {
        self.inner.raster_cache.read().clone()
    }

    /// Get the row bytes for the raster data.
    pub fn row_bytes(&self) -> usize {
        self.inner.row_bytes
    }

    /// Read pixels from the GPU image back to CPU memory.
    ///
    /// Uses cached raster data if available.
    pub fn read_pixels(&self, dst: &mut [u8], dst_row_bytes: usize) -> bool {
        let cache = self.inner.raster_cache.read();
        if let Some(ref cached) = *cache {
            let bytes_per_pixel = self.color_type().bytes_per_pixel();
            let width = self.width() as usize;
            let height = self.height() as usize;
            let src_row_bytes = self.inner.row_bytes;

            for y in 0..height {
                let src_offset = y * src_row_bytes;
                let dst_offset = y * dst_row_bytes;
                let copy_len = width * bytes_per_pixel;

                if dst_offset + copy_len <= dst.len() && src_offset + copy_len <= cached.len() {
                    dst[dst_offset..dst_offset + copy_len]
                        .copy_from_slice(&cached[src_offset..src_offset + copy_len]);
                }
            }
            return true;
        }

        // GPU read-back would be triggered here by GPU backend
        false
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
    pub fn to_raster(&self) -> Option<crate::Image> {
        let cache = self.inner.raster_cache.read();
        if let Some(ref cached) = *cache {
            crate::Image::from_raster_data(&self.inner.info, cached, self.inner.row_bytes)
        } else {
            None
        }
    }

    /// Create a subset of this GPU image.
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

        let cache = self.inner.raster_cache.read();
        if let Some(ref cached) = *cache {
            let new_info = ImageInfo::new(w, h, self.color_type(), self.alpha_type());
            let bytes_per_pixel = self.color_type().bytes_per_pixel();
            let src_row_bytes = self.inner.row_bytes;
            let new_row_bytes = w as usize * bytes_per_pixel;
            let mut new_pixels = vec![0u8; (h as usize) * new_row_bytes];

            for row in 0..h as usize {
                let src_offset = (y as usize + row) * src_row_bytes + x as usize * bytes_per_pixel;
                let dst_offset = row * new_row_bytes;

                new_pixels[dst_offset..dst_offset + new_row_bytes]
                    .copy_from_slice(&cached[src_offset..src_offset + new_row_bytes]);
            }

            Self::from_raster_data_owned(new_info, new_pixels, new_row_bytes)
        } else {
            None
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
}
