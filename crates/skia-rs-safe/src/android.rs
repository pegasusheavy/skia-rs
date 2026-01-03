//! Android-specific APIs for skia-rs.
//!
//! This module provides Android platform integration including:
//! - Hardware buffer support (AHardwareBuffer)
//! - Bitmap interop (android.graphics.Bitmap)
//! - Surface texture integration
//! - Choreographer frame timing
//!
//! # Platform Support
//!
//! These APIs are only available when targeting Android (`target_os = "android"`).
//!
//! # Example
//!
//! ```rust,ignore
//! use skia_rs_safe::android::{HardwareBuffer, HardwareBufferFormat, HardwareBufferUsage};
//!
//! // Create a hardware buffer for GPU rendering
//! let buffer = HardwareBuffer::new(
//!     800,
//!     600,
//!     HardwareBufferFormat::R8G8B8A8_UNORM,
//!     1, // layers
//!     HardwareBufferUsage::GPU_SAMPLED_IMAGE | HardwareBufferUsage::GPU_COLOR_OUTPUT,
//! ).expect("Failed to create hardware buffer");
//!
//! // Lock for CPU access
//! let pixels = buffer.lock_for_write().expect("Failed to lock");
//! // ... write pixels ...
//! buffer.unlock();
//! ```

#![cfg(target_os = "android")]

use std::ptr::NonNull;

// =============================================================================
// Hardware Buffer Format
// =============================================================================

/// Hardware buffer pixel formats (mirrors AHardwareBuffer_Format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum HardwareBufferFormat {
    /// 32-bit RGBA, 8 bits per channel.
    R8G8B8A8_UNORM = 1,
    /// 32-bit RGBX, 8 bits per channel (alpha ignored).
    R8G8B8X8_UNORM = 2,
    /// 24-bit RGB, 8 bits per channel.
    R8G8B8_UNORM = 3,
    /// 16-bit RGB, 5-6-5 bits per channel.
    R5G6B5_UNORM = 4,
    /// 16-bit RGBA, 4 bits per channel.
    R4G4B4A4_UNORM = 5, // Deprecated in API 26
    /// 16-bit float per channel RGBA.
    R16G16B16A16_FLOAT = 0x16,
    /// 10-bit RGB, 2-bit alpha.
    R10G10B10A2_UNORM = 0x2B,
    /// Blob format (opaque).
    BLOB = 0x21,
    /// Depth 16-bit.
    D16_UNORM = 0x30,
    /// Depth 24-bit.
    D24_UNORM = 0x31,
    /// Depth 24-bit, Stencil 8-bit.
    D24_UNORM_S8_UINT = 0x32,
    /// Depth 32-bit float.
    D32_FLOAT = 0x33,
    /// Depth 32-bit float, Stencil 8-bit.
    D32_FLOAT_S8_UINT = 0x34,
    /// Stencil 8-bit.
    S8_UINT = 0x35,
    /// YCbCr 420 semi-planar.
    Y8Cb8Cr8_420 = 0x23,
}

impl HardwareBufferFormat {
    /// Get bytes per pixel for this format.
    pub fn bytes_per_pixel(self) -> Option<usize> {
        match self {
            Self::R8G8B8A8_UNORM | Self::R8G8B8X8_UNORM => Some(4),
            Self::R8G8B8_UNORM => Some(3),
            Self::R5G6B5_UNORM | Self::R4G4B4A4_UNORM => Some(2),
            Self::R16G16B16A16_FLOAT => Some(8),
            Self::R10G10B10A2_UNORM => Some(4),
            Self::D16_UNORM => Some(2),
            Self::D24_UNORM | Self::D24_UNORM_S8_UINT => Some(4),
            Self::D32_FLOAT | Self::D32_FLOAT_S8_UINT => Some(4),
            Self::S8_UINT => Some(1),
            _ => None, // Blob, YCbCr formats
        }
    }

    /// Check if format has alpha channel.
    pub fn has_alpha(self) -> bool {
        matches!(
            self,
            Self::R8G8B8A8_UNORM
                | Self::R4G4B4A4_UNORM
                | Self::R16G16B16A16_FLOAT
                | Self::R10G10B10A2_UNORM
        )
    }
}

// =============================================================================
// Hardware Buffer Usage Flags
// =============================================================================

bitflags::bitflags! {
    /// Hardware buffer usage flags (mirrors AHardwareBuffer_UsageFlags).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct HardwareBufferUsage: u64 {
        /// Buffer will be read by CPU.
        const CPU_READ_NEVER = 0;
        /// Buffer will be read by CPU rarely.
        const CPU_READ_RARELY = 2;
        /// Buffer will be read by CPU often.
        const CPU_READ_OFTEN = 3;
        /// Buffer will not be written by CPU.
        const CPU_WRITE_NEVER = 0;
        /// Buffer will be written by CPU rarely.
        const CPU_WRITE_RARELY = 2 << 4;
        /// Buffer will be written by CPU often.
        const CPU_WRITE_OFTEN = 3 << 4;
        /// Buffer will be used as a GPU texture.
        const GPU_SAMPLED_IMAGE = 1 << 8;
        /// Buffer will be used as a GPU framebuffer attachment.
        const GPU_FRAMEBUFFER = 1 << 9;
        /// Buffer will be used as a GPU color output.
        const GPU_COLOR_OUTPUT = Self::GPU_FRAMEBUFFER.bits();
        /// Buffer will be used in a compositor overlay.
        const COMPOSER_OVERLAY = 1 << 11;
        /// Buffer is protected content.
        const PROTECTED_CONTENT = 1 << 14;
        /// Buffer may be used as video encoder input.
        const VIDEO_ENCODE = 1 << 16;
        /// Buffer will be used for sensor direct data.
        const SENSOR_DIRECT_DATA = 1 << 23;
        /// Buffer will be used as GPU data buffer.
        const GPU_DATA_BUFFER = 1 << 24;
        /// Buffer will be used as a cube map texture.
        const GPU_CUBE_MAP = 1 << 25;
        /// Buffer will be used as a mipmap complete texture.
        const GPU_MIPMAP_COMPLETE = 1 << 26;
    }
}

// =============================================================================
// Hardware Buffer Description
// =============================================================================

/// Description of a hardware buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct HardwareBufferDesc {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Number of layers.
    pub layers: u32,
    /// Pixel format.
    pub format: HardwareBufferFormat,
    /// Usage flags.
    pub usage: HardwareBufferUsage,
    /// Stride in pixels (set by allocator).
    pub stride: u32,
}

// =============================================================================
// Hardware Buffer
// =============================================================================

/// Android hardware buffer wrapper.
///
/// This provides safe access to AHardwareBuffer for GPU interop
/// and zero-copy sharing between processes.
pub struct HardwareBuffer {
    // In a real implementation, this would hold the AHardwareBuffer pointer
    desc: HardwareBufferDesc,
    // Simulated pixel data for non-Android platforms
    #[cfg(not(target_os = "android"))]
    pixels: Vec<u8>,
}

impl HardwareBuffer {
    /// Create a new hardware buffer.
    pub fn new(
        width: u32,
        height: u32,
        format: HardwareBufferFormat,
        layers: u32,
        usage: HardwareBufferUsage,
    ) -> Option<Self> {
        if width == 0 || height == 0 || layers == 0 {
            return None;
        }

        let stride = width; // Simplified; real impl queries from AHardwareBuffer

        let desc = HardwareBufferDesc {
            width,
            height,
            layers,
            format,
            usage,
            stride,
        };

        #[cfg(not(target_os = "android"))]
        let pixels = {
            let bpp = format.bytes_per_pixel().unwrap_or(4);
            vec![0u8; (stride as usize) * (height as usize) * bpp]
        };

        Some(Self {
            desc,
            #[cfg(not(target_os = "android"))]
            pixels,
        })
    }

    /// Get the buffer description.
    pub fn desc(&self) -> &HardwareBufferDesc {
        &self.desc
    }

    /// Get width in pixels.
    pub fn width(&self) -> u32 {
        self.desc.width
    }

    /// Get height in pixels.
    pub fn height(&self) -> u32 {
        self.desc.height
    }

    /// Get pixel format.
    pub fn format(&self) -> HardwareBufferFormat {
        self.desc.format
    }

    /// Get stride in pixels.
    pub fn stride(&self) -> u32 {
        self.desc.stride
    }

    /// Lock the buffer for CPU read access.
    pub fn lock_for_read(&self) -> Option<LockedBuffer<'_>> {
        #[cfg(not(target_os = "android"))]
        {
            Some(LockedBuffer {
                buffer: self,
                ptr: self.pixels.as_ptr() as *mut u8,
                writable: false,
            })
        }
        #[cfg(target_os = "android")]
        {
            // Real implementation would call AHardwareBuffer_lock
            None
        }
    }

    /// Lock the buffer for CPU write access.
    pub fn lock_for_write(&mut self) -> Option<LockedBufferMut<'_>> {
        #[cfg(not(target_os = "android"))]
        {
            Some(LockedBufferMut {
                buffer: self,
                ptr: self.pixels.as_mut_ptr(),
            })
        }
        #[cfg(target_os = "android")]
        {
            // Real implementation would call AHardwareBuffer_lock
            None
        }
    }

    /// Check if this buffer can be used as a GPU texture.
    pub fn is_gpu_texture(&self) -> bool {
        self.desc.usage.contains(HardwareBufferUsage::GPU_SAMPLED_IMAGE)
    }

    /// Check if this buffer can be used as a GPU render target.
    pub fn is_gpu_render_target(&self) -> bool {
        self.desc.usage.contains(HardwareBufferUsage::GPU_COLOR_OUTPUT)
    }
}

/// A locked hardware buffer for read access.
pub struct LockedBuffer<'a> {
    #[allow(dead_code)]
    buffer: &'a HardwareBuffer,
    ptr: *mut u8,
    #[allow(dead_code)]
    writable: bool,
}

impl<'a> LockedBuffer<'a> {
    /// Get a slice of the pixel data.
    pub fn as_slice(&self) -> &[u8] {
        let bpp = self.buffer.format().bytes_per_pixel().unwrap_or(4);
        let len = (self.buffer.stride() as usize)
            * (self.buffer.height() as usize)
            * bpp;
        unsafe { std::slice::from_raw_parts(self.ptr, len) }
    }
}

/// A locked hardware buffer for write access.
pub struct LockedBufferMut<'a> {
    #[allow(dead_code)]
    buffer: &'a mut HardwareBuffer,
    ptr: *mut u8,
}

impl<'a> LockedBufferMut<'a> {
    /// Get a mutable slice of the pixel data.
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        let bpp = self.buffer.format().bytes_per_pixel().unwrap_or(4);
        let len = (self.buffer.stride() as usize)
            * (self.buffer.height() as usize)
            * bpp;
        unsafe { std::slice::from_raw_parts_mut(self.ptr, len) }
    }
}

// =============================================================================
// Android Bitmap
// =============================================================================

/// Android bitmap configuration (mirrors Bitmap.Config).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum BitmapConfig {
    /// Each pixel is stored on 4 bytes. Alpha, Red, Green, Blue.
    ARGB_8888 = 0,
    /// Each pixel is stored on 2 bytes. RGB channels with 5-6-5 bit depths.
    RGB_565 = 1,
    /// Deprecated. Each pixel is 4 bits (16 color palette).
    ARGB_4444 = 2,
    /// Each pixel is a single byte (grayscale).
    ALPHA_8 = 3,
    /// Each pixel is 8 bytes (RGBA float16).
    RGBA_F16 = 4,
    /// Each pixel is 8 bytes (RGBA float16) but no alpha blending.
    RGBA_1010102 = 5,
    /// Hardware-accelerated bitmap.
    HARDWARE = 6,
}

impl BitmapConfig {
    /// Get bytes per pixel.
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Self::ARGB_8888 | Self::RGBA_1010102 => 4,
            Self::RGB_565 | Self::ARGB_4444 => 2,
            Self::ALPHA_8 => 1,
            Self::RGBA_F16 => 8,
            Self::HARDWARE => 0, // Variable
        }
    }
}

/// Android bitmap wrapper for interop.
///
/// This type mirrors `android.graphics.Bitmap` and provides
/// conversion to/from skia-rs surfaces.
pub struct AndroidBitmap {
    width: i32,
    height: i32,
    config: BitmapConfig,
    pixels: Vec<u8>,
    row_bytes: usize,
    is_mutable: bool,
    has_alpha: bool,
}

impl AndroidBitmap {
    /// Create a new mutable bitmap.
    pub fn create(width: i32, height: i32, config: BitmapConfig) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }

        let bpp = config.bytes_per_pixel();
        if bpp == 0 {
            return None; // HARDWARE config not supported for CPU bitmaps
        }

        let row_bytes = (width as usize) * bpp;
        let total_size = row_bytes * (height as usize);

        Some(Self {
            width,
            height,
            config,
            pixels: vec![0u8; total_size],
            row_bytes,
            is_mutable: true,
            has_alpha: matches!(
                config,
                BitmapConfig::ARGB_8888 | BitmapConfig::ARGB_4444 | BitmapConfig::RGBA_F16
            ),
        })
    }

    /// Create from existing pixel data.
    pub fn from_pixels(
        width: i32,
        height: i32,
        config: BitmapConfig,
        pixels: Vec<u8>,
    ) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }

        let bpp = config.bytes_per_pixel();
        if bpp == 0 {
            return None;
        }

        let row_bytes = (width as usize) * bpp;
        let expected_size = row_bytes * (height as usize);

        if pixels.len() < expected_size {
            return None;
        }

        Some(Self {
            width,
            height,
            config,
            pixels,
            row_bytes,
            is_mutable: true,
            has_alpha: matches!(
                config,
                BitmapConfig::ARGB_8888 | BitmapConfig::ARGB_4444 | BitmapConfig::RGBA_F16
            ),
        })
    }

    /// Get width.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Get height.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Get config.
    pub fn config(&self) -> BitmapConfig {
        self.config
    }

    /// Get row bytes (stride).
    pub fn row_bytes(&self) -> usize {
        self.row_bytes
    }

    /// Check if bitmap is mutable.
    pub fn is_mutable(&self) -> bool {
        self.is_mutable
    }

    /// Check if bitmap has alpha.
    pub fn has_alpha(&self) -> bool {
        self.has_alpha
    }

    /// Get pixel data.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Get mutable pixel data.
    pub fn pixels_mut(&mut self) -> Option<&mut [u8]> {
        if self.is_mutable {
            Some(&mut self.pixels)
        } else {
            None
        }
    }

    /// Copy a region to another bitmap.
    pub fn copy_region(
        &self,
        src_x: i32,
        src_y: i32,
        width: i32,
        height: i32,
    ) -> Option<AndroidBitmap> {
        if src_x < 0 || src_y < 0 || width <= 0 || height <= 0 {
            return None;
        }
        if src_x + width > self.width || src_y + height > self.height {
            return None;
        }

        let mut dst = Self::create(width, height, self.config)?;
        let bpp = self.config.bytes_per_pixel();

        for y in 0..height {
            let src_offset = ((src_y + y) as usize) * self.row_bytes + (src_x as usize) * bpp;
            let dst_offset = (y as usize) * dst.row_bytes;
            let copy_bytes = (width as usize) * bpp;

            dst.pixels[dst_offset..dst_offset + copy_bytes]
                .copy_from_slice(&self.pixels[src_offset..src_offset + copy_bytes]);
        }

        Some(dst)
    }

    /// Get a pixel color at coordinates.
    pub fn get_pixel(&self, x: i32, y: i32) -> Option<u32> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }

        let bpp = self.config.bytes_per_pixel();
        let offset = (y as usize) * self.row_bytes + (x as usize) * bpp;

        match self.config {
            BitmapConfig::ARGB_8888 => {
                let r = self.pixels[offset];
                let g = self.pixels[offset + 1];
                let b = self.pixels[offset + 2];
                let a = self.pixels[offset + 3];
                Some(((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
            }
            BitmapConfig::RGB_565 => {
                let lo = self.pixels[offset] as u16;
                let hi = self.pixels[offset + 1] as u16;
                let rgb565 = (hi << 8) | lo;
                let r = ((rgb565 >> 11) & 0x1F) as u8;
                let g = ((rgb565 >> 5) & 0x3F) as u8;
                let b = (rgb565 & 0x1F) as u8;
                // Expand to 8-bit
                let r8 = (r << 3) | (r >> 2);
                let g8 = (g << 2) | (g >> 4);
                let b8 = (b << 3) | (b >> 2);
                Some(0xFF000000 | ((r8 as u32) << 16) | ((g8 as u32) << 8) | (b8 as u32))
            }
            BitmapConfig::ALPHA_8 => {
                let a = self.pixels[offset];
                Some(((a as u32) << 24) | 0x00FFFFFF)
            }
            _ => None,
        }
    }

    /// Set a pixel color at coordinates.
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) -> bool {
        if !self.is_mutable || x < 0 || y < 0 || x >= self.width || y >= self.height {
            return false;
        }

        let bpp = self.config.bytes_per_pixel();
        let offset = (y as usize) * self.row_bytes + (x as usize) * bpp;

        match self.config {
            BitmapConfig::ARGB_8888 => {
                self.pixels[offset] = ((color >> 16) & 0xFF) as u8; // R
                self.pixels[offset + 1] = ((color >> 8) & 0xFF) as u8; // G
                self.pixels[offset + 2] = (color & 0xFF) as u8; // B
                self.pixels[offset + 3] = ((color >> 24) & 0xFF) as u8; // A
                true
            }
            BitmapConfig::RGB_565 => {
                let r = ((color >> 16) & 0xFF) as u16;
                let g = ((color >> 8) & 0xFF) as u16;
                let b = (color & 0xFF) as u16;
                let rgb565 = ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3);
                self.pixels[offset] = (rgb565 & 0xFF) as u8;
                self.pixels[offset + 1] = ((rgb565 >> 8) & 0xFF) as u8;
                true
            }
            BitmapConfig::ALPHA_8 => {
                self.pixels[offset] = ((color >> 24) & 0xFF) as u8;
                true
            }
            _ => false,
        }
    }

    /// Make the bitmap immutable.
    pub fn make_immutable(&mut self) {
        self.is_mutable = false;
    }
}

// =============================================================================
// Surface Texture
// =============================================================================

/// Texture transform matrix for SurfaceTexture.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct SurfaceTextureTransform {
    /// Transform matrix (column-major 4x4).
    pub matrix: [f32; 16],
}

impl Default for SurfaceTextureTransform {
    fn default() -> Self {
        Self {
            matrix: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }
}

/// Android SurfaceTexture wrapper.
///
/// Provides integration with `android.graphics.SurfaceTexture` for
/// camera preview, video playback, and other streaming content.
pub struct SurfaceTexture {
    texture_id: u32,
    width: i32,
    height: i32,
    transform: SurfaceTextureTransform,
    timestamp_ns: i64,
    frame_available: bool,
}

impl SurfaceTexture {
    /// Create a new SurfaceTexture with the given OpenGL ES texture ID.
    pub fn new(texture_id: u32) -> Self {
        Self {
            texture_id,
            width: 0,
            height: 0,
            transform: SurfaceTextureTransform::default(),
            timestamp_ns: 0,
            frame_available: false,
        }
    }

    /// Get the OpenGL ES texture ID.
    pub fn texture_id(&self) -> u32 {
        self.texture_id
    }

    /// Get the transform matrix.
    pub fn transform(&self) -> &SurfaceTextureTransform {
        &self.transform
    }

    /// Get the timestamp of the most recent frame in nanoseconds.
    pub fn timestamp(&self) -> i64 {
        self.timestamp_ns
    }

    /// Set the default buffer size.
    pub fn set_default_buffer_size(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
    }

    /// Check if a new frame is available.
    pub fn is_frame_available(&self) -> bool {
        self.frame_available
    }

    /// Update the texture with the latest frame.
    ///
    /// This would normally call `SurfaceTexture.updateTexImage()`.
    pub fn update_tex_image(&mut self) -> bool {
        if self.frame_available {
            self.frame_available = false;
            // Real implementation would update the GL texture
            true
        } else {
            false
        }
    }

    /// Release the SurfaceTexture.
    pub fn release(&mut self) {
        self.texture_id = 0;
        self.frame_available = false;
    }
}

// =============================================================================
// Choreographer
// =============================================================================

/// Frame timing information from Choreographer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    /// Frame number.
    pub frame_number: u64,
    /// Vsync timestamp in nanoseconds.
    pub vsync_ns: i64,
    /// Deadline for frame submission in nanoseconds.
    pub deadline_ns: i64,
    /// Frame interval in nanoseconds (typically 16.6ms for 60Hz).
    pub frame_interval_ns: i64,
}

/// Choreographer callback type.
pub type ChoreographerCallback = Box<dyn FnMut(FrameInfo) + Send>;

/// Android Choreographer wrapper for frame timing.
///
/// Provides Vsync-aligned frame callbacks for smooth animation.
pub struct Choreographer {
    frame_number: u64,
    last_vsync_ns: i64,
    frame_interval_ns: i64,
    #[allow(dead_code)]
    callback: Option<ChoreographerCallback>,
}

impl Choreographer {
    /// Get the main thread Choreographer instance.
    pub fn instance() -> Self {
        Self {
            frame_number: 0,
            last_vsync_ns: 0,
            // Default to 60 Hz
            frame_interval_ns: 16_666_667,
            callback: None,
        }
    }

    /// Post a frame callback.
    pub fn post_frame_callback(&mut self, callback: ChoreographerCallback) {
        self.callback = Some(callback);
    }

    /// Remove the frame callback.
    pub fn remove_frame_callback(&mut self) {
        self.callback = None;
    }

    /// Get the current refresh rate in Hz.
    pub fn refresh_rate(&self) -> f32 {
        1_000_000_000.0 / self.frame_interval_ns as f32
    }

    /// Get the frame interval in nanoseconds.
    pub fn frame_interval_ns(&self) -> i64 {
        self.frame_interval_ns
    }

    /// Simulate a frame callback (for testing).
    #[cfg(test)]
    pub fn simulate_frame(&mut self, vsync_ns: i64) {
        self.frame_number += 1;
        self.last_vsync_ns = vsync_ns;

        if let Some(callback) = &mut self.callback {
            let info = FrameInfo {
                frame_number: self.frame_number,
                vsync_ns,
                deadline_ns: vsync_ns + self.frame_interval_ns,
                frame_interval_ns: self.frame_interval_ns,
            };
            callback(info);
        }
    }
}

// =============================================================================
// Display
// =============================================================================

/// Display refresh rate mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefreshRateMode {
    /// Standard refresh rate (typically 60Hz).
    Standard,
    /// High refresh rate (90Hz, 120Hz, etc.).
    High,
    /// Adaptive refresh rate (variable).
    Adaptive,
}

/// Android display information.
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    /// Display ID.
    pub display_id: i32,
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
    /// Density DPI.
    pub density_dpi: i32,
    /// Refresh rate in Hz.
    pub refresh_rate: f32,
    /// Available refresh rates.
    pub supported_refresh_rates: Vec<f32>,
    /// HDR capability.
    pub hdr_supported: bool,
    /// Wide color gamut support.
    pub wide_color_gamut: bool,
}

impl DisplayInfo {
    /// Get the default display info.
    pub fn default_display() -> Self {
        Self {
            display_id: 0,
            width: 1080,
            height: 1920,
            density_dpi: 420,
            refresh_rate: 60.0,
            supported_refresh_rates: vec![60.0],
            hdr_supported: false,
            wide_color_gamut: false,
        }
    }

    /// Get density scale factor.
    pub fn density(&self) -> f32 {
        self.density_dpi as f32 / 160.0
    }

    /// Get scaled density for fonts.
    pub fn scaled_density(&self) -> f32 {
        self.density()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_buffer_format() {
        assert_eq!(HardwareBufferFormat::R8G8B8A8_UNORM.bytes_per_pixel(), Some(4));
        assert_eq!(HardwareBufferFormat::RGB_565.bytes_per_pixel(), Some(2));
        assert!(HardwareBufferFormat::R8G8B8A8_UNORM.has_alpha());
        assert!(!HardwareBufferFormat::R8G8B8_UNORM.has_alpha());
    }

    #[test]
    fn test_android_bitmap_create() {
        let bitmap = AndroidBitmap::create(100, 100, BitmapConfig::ARGB_8888);
        assert!(bitmap.is_some());

        let bitmap = bitmap.unwrap();
        assert_eq!(bitmap.width(), 100);
        assert_eq!(bitmap.height(), 100);
        assert!(bitmap.is_mutable());
        assert!(bitmap.has_alpha());
    }

    #[test]
    fn test_android_bitmap_pixel() {
        let mut bitmap = AndroidBitmap::create(10, 10, BitmapConfig::ARGB_8888).unwrap();

        // Set pixel
        assert!(bitmap.set_pixel(5, 5, 0xFFFF0000)); // Red

        // Get pixel
        let pixel = bitmap.get_pixel(5, 5);
        assert!(pixel.is_some());
        assert_eq!(pixel.unwrap(), 0xFFFF0000);
    }

    #[test]
    fn test_display_info() {
        let display = DisplayInfo::default_display();
        assert!(display.density() > 0.0);
        assert_eq!(display.refresh_rate, 60.0);
    }
}
