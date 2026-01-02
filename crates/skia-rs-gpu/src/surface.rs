//! GPU surface abstraction.

use crate::{GpuError, GpuResult, TextureDescriptor, TextureFormat, TextureUsage};
use skia_rs_core::{Color, Rect, Scalar};

/// GPU surface properties.
#[derive(Debug, Clone)]
pub struct GpuSurfaceProps {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Sample count for MSAA.
    pub sample_count: u32,
    /// Surface format.
    pub format: TextureFormat,
    /// Use sRGB color space.
    pub srgb: bool,
}

impl Default for GpuSurfaceProps {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            sample_count: 1,
            format: TextureFormat::Rgba8Unorm,
            srgb: false,
        }
    }
}

impl GpuSurfaceProps {
    /// Create new surface properties.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }

    /// Set format.
    pub fn with_format(mut self, format: TextureFormat) -> Self {
        self.format = format;
        self
    }

    /// Set sample count.
    pub fn with_sample_count(mut self, count: u32) -> Self {
        self.sample_count = count;
        self
    }

    /// Enable sRGB.
    pub fn with_srgb(mut self, srgb: bool) -> Self {
        self.srgb = srgb;
        self
    }
}

/// Trait for GPU surfaces.
pub trait GpuSurface: Send + Sync {
    /// Get width.
    fn width(&self) -> u32;

    /// Get height.
    fn height(&self) -> u32;

    /// Get format.
    fn format(&self) -> TextureFormat;

    /// Get sample count.
    fn sample_count(&self) -> u32;

    /// Clear the surface.
    fn clear(&mut self, color: Color);

    /// Present the surface (for display surfaces).
    fn present(&mut self);

    /// Read pixels from the surface.
    fn read_pixels(&self, dst: &mut [u8], dst_row_bytes: usize) -> bool;

    /// Flush pending operations.
    fn flush(&mut self);
}

/// Render pass descriptor.
#[derive(Debug, Clone)]
pub struct RenderPassDescriptor {
    /// Clear color (if any).
    pub clear_color: Option<[f32; 4]>,
    /// Clear depth (if any).
    pub clear_depth: Option<f32>,
    /// Clear stencil (if any).
    pub clear_stencil: Option<u32>,
}

impl Default for RenderPassDescriptor {
    fn default() -> Self {
        Self {
            clear_color: Some([0.0, 0.0, 0.0, 1.0]),
            clear_depth: Some(1.0),
            clear_stencil: Some(0),
        }
    }
}

impl RenderPassDescriptor {
    /// Create with no clearing.
    pub fn no_clear() -> Self {
        Self {
            clear_color: None,
            clear_depth: None,
            clear_stencil: None,
        }
    }

    /// Create with color clear only.
    pub fn color_clear(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            clear_color: Some([r, g, b, a]),
            clear_depth: None,
            clear_stencil: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_props() {
        let props = GpuSurfaceProps::new(800, 600)
            .with_format(TextureFormat::Bgra8Unorm)
            .with_sample_count(4);

        assert_eq!(props.width, 800);
        assert_eq!(props.height, 600);
        assert_eq!(props.sample_count, 4);
    }

    #[test]
    fn test_render_pass_descriptor() {
        let desc = RenderPassDescriptor::color_clear(1.0, 0.0, 0.0, 1.0);
        assert!(desc.clear_color.is_some());
        assert!(desc.clear_depth.is_none());
    }
}
