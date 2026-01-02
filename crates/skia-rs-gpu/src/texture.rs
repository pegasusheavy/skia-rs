//! GPU texture abstraction.

use skia_rs_core::{AlphaType, ColorType};

/// Texture format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    /// RGBA8 unorm.
    Rgba8Unorm,
    /// RGBA8 sRGB.
    Rgba8UnormSrgb,
    /// BGRA8 unorm.
    Bgra8Unorm,
    /// BGRA8 sRGB.
    Bgra8UnormSrgb,
    /// R8 unorm.
    R8Unorm,
    /// RG8 unorm.
    Rg8Unorm,
    /// RGBA16 float.
    Rgba16Float,
    /// RGBA32 float.
    Rgba32Float,
    /// Depth24 stencil8.
    Depth24Stencil8,
    /// Depth32 float.
    Depth32Float,
}

impl TextureFormat {
    /// Get bytes per pixel.
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8Unorm => 1,
            Self::Rg8Unorm => 2,
            Self::Rgba8Unorm | Self::Rgba8UnormSrgb | Self::Bgra8Unorm | Self::Bgra8UnormSrgb => 4,
            Self::Rgba16Float => 8,
            Self::Rgba32Float => 16,
            Self::Depth24Stencil8 | Self::Depth32Float => 4,
        }
    }

    /// Convert from ColorType.
    pub fn from_color_type(color_type: ColorType) -> Option<Self> {
        match color_type {
            ColorType::Rgba8888 => Some(Self::Rgba8Unorm),
            ColorType::Bgra8888 => Some(Self::Bgra8Unorm),
            ColorType::Alpha8 => Some(Self::R8Unorm),
            ColorType::Gray8 => Some(Self::R8Unorm),
            ColorType::RgbaF16 => Some(Self::Rgba16Float),
            ColorType::RgbaF32 => Some(Self::Rgba32Float),
            _ => None,
        }
    }

    /// Check if this is a depth format.
    pub fn is_depth(&self) -> bool {
        matches!(self, Self::Depth24Stencil8 | Self::Depth32Float)
    }

    /// Check if this is an sRGB format.
    pub fn is_srgb(&self) -> bool {
        matches!(self, Self::Rgba8UnormSrgb | Self::Bgra8UnormSrgb)
    }
}

/// Texture usage flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextureUsage(u32);

impl TextureUsage {
    /// No usage.
    pub const NONE: Self = Self(0);
    /// Can be copied from.
    pub const COPY_SRC: Self = Self(1 << 0);
    /// Can be copied to.
    pub const COPY_DST: Self = Self(1 << 1);
    /// Can be sampled in a shader.
    pub const SAMPLED: Self = Self(1 << 2);
    /// Can be used as a storage texture.
    pub const STORAGE: Self = Self(1 << 3);
    /// Can be used as a render target.
    pub const RENDER_TARGET: Self = Self(1 << 4);

    /// Check if this usage includes another.
    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combine usages.
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl std::ops::BitOr for TextureUsage {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Texture descriptor.
#[derive(Debug, Clone)]
pub struct TextureDescriptor {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Depth (for 3D textures) or array layers.
    pub depth_or_layers: u32,
    /// Mip level count.
    pub mip_level_count: u32,
    /// Sample count (for MSAA).
    pub sample_count: u32,
    /// Format.
    pub format: TextureFormat,
    /// Usage.
    pub usage: TextureUsage,
    /// Label for debugging.
    pub label: Option<String>,
}

impl TextureDescriptor {
    /// Create a simple 2D texture descriptor.
    pub fn new_2d(width: u32, height: u32, format: TextureFormat, usage: TextureUsage) -> Self {
        Self {
            width,
            height,
            depth_or_layers: 1,
            mip_level_count: 1,
            sample_count: 1,
            format,
            usage,
            label: None,
        }
    }

    /// Set the label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set mip level count.
    pub fn with_mip_levels(mut self, count: u32) -> Self {
        self.mip_level_count = count;
        self
    }
}

/// Backend texture handle (opaque).
#[derive(Debug)]
pub struct BackendTexture {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Format.
    pub format: TextureFormat,
    /// Backend-specific handle.
    pub handle: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_format_bytes() {
        assert_eq!(TextureFormat::Rgba8Unorm.bytes_per_pixel(), 4);
        assert_eq!(TextureFormat::R8Unorm.bytes_per_pixel(), 1);
        assert_eq!(TextureFormat::Rgba16Float.bytes_per_pixel(), 8);
    }

    #[test]
    fn test_texture_usage() {
        let usage = TextureUsage::SAMPLED | TextureUsage::RENDER_TARGET;
        assert!(usage.contains(TextureUsage::SAMPLED));
        assert!(usage.contains(TextureUsage::RENDER_TARGET));
        assert!(!usage.contains(TextureUsage::STORAGE));
    }
}
