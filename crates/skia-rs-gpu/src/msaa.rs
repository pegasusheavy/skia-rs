//! MSAA (Multi-Sample Anti-Aliasing) support for GPU rendering.
//!
//! This module provides utilities for managing MSAA render targets
//! and resolving multisampled surfaces.

use crate::TextureFormat;
use skia_rs_core::Scalar;

/// MSAA sample count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SampleCount {
    /// No MSAA (1 sample).
    #[default]
    S1 = 1,
    /// 2x MSAA.
    S2 = 2,
    /// 4x MSAA.
    S4 = 4,
    /// 8x MSAA.
    S8 = 8,
    /// 16x MSAA (rarely supported).
    S16 = 16,
}

impl SampleCount {
    /// Get the numeric sample count.
    pub fn count(&self) -> u32 {
        *self as u32
    }

    /// Check if MSAA is enabled (sample count > 1).
    pub fn is_msaa(&self) -> bool {
        *self != SampleCount::S1
    }

    /// Get the next lower sample count.
    pub fn lower(&self) -> Self {
        match self {
            SampleCount::S16 => SampleCount::S8,
            SampleCount::S8 => SampleCount::S4,
            SampleCount::S4 => SampleCount::S2,
            SampleCount::S2 | SampleCount::S1 => SampleCount::S1,
        }
    }

    /// Get the next higher sample count.
    pub fn higher(&self) -> Self {
        match self {
            SampleCount::S1 => SampleCount::S2,
            SampleCount::S2 => SampleCount::S4,
            SampleCount::S4 => SampleCount::S8,
            SampleCount::S8 | SampleCount::S16 => SampleCount::S16,
        }
    }

    /// Create from a numeric value.
    pub fn from_count(count: u32) -> Self {
        match count {
            1 => SampleCount::S1,
            2 => SampleCount::S2,
            4 => SampleCount::S4,
            8 => SampleCount::S8,
            16 => SampleCount::S16,
            _ if count > 8 => SampleCount::S16,
            _ if count > 4 => SampleCount::S8,
            _ if count > 2 => SampleCount::S4,
            _ if count > 1 => SampleCount::S2,
            _ => SampleCount::S1,
        }
    }

    /// All sample counts from lowest to highest.
    pub const ALL: [SampleCount; 5] = [
        SampleCount::S1,
        SampleCount::S2,
        SampleCount::S4,
        SampleCount::S8,
        SampleCount::S16,
    ];
}

/// MSAA render target configuration.
#[derive(Debug, Clone)]
pub struct MsaaConfig {
    /// Sample count.
    pub sample_count: SampleCount,
    /// Color format.
    pub color_format: TextureFormat,
    /// Depth/stencil format (optional).
    pub depth_stencil_format: Option<TextureFormat>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl MsaaConfig {
    /// Create a new MSAA configuration.
    pub fn new(
        sample_count: SampleCount,
        color_format: TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            sample_count,
            color_format,
            depth_stencil_format: None,
            width,
            height,
        }
    }

    /// Add depth/stencil buffer.
    pub fn with_depth_stencil(mut self, format: TextureFormat) -> Self {
        self.depth_stencil_format = Some(format);
        self
    }

    /// Calculate memory usage estimate.
    pub fn memory_estimate(&self) -> u64 {
        let color_bytes = self.color_format.bytes_per_pixel() as u64;
        let samples = self.sample_count.count() as u64;
        let pixels = self.width as u64 * self.height as u64;

        let color_size = pixels * color_bytes * samples;

        let depth_size = self
            .depth_stencil_format
            .map_or(0, |fmt| pixels * fmt.bytes_per_pixel() as u64 * samples);

        color_size + depth_size
    }
}

/// MSAA resolve mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResolveMode {
    /// Average all samples (standard resolve).
    #[default]
    Average,
    /// Take minimum sample.
    Min,
    /// Take maximum sample.
    Max,
    /// Take sample 0 only.
    Sample0,
}

/// MSAA resolve configuration.
#[derive(Debug, Clone)]
pub struct ResolveConfig {
    /// Resolve mode.
    pub mode: ResolveMode,
    /// Source rectangle (multisampled).
    pub src_rect: Option<[u32; 4]>,
    /// Destination rectangle (resolved).
    pub dst_rect: Option<[u32; 4]>,
}

impl Default for ResolveConfig {
    fn default() -> Self {
        Self {
            mode: ResolveMode::Average,
            src_rect: None,
            dst_rect: None,
        }
    }
}

/// MSAA quality settings.
#[derive(Debug, Clone, Copy)]
pub struct MsaaQuality {
    /// Sample count.
    pub sample_count: SampleCount,
    /// Sample quality level (implementation-specific).
    pub quality_level: u32,
    /// Enable alpha-to-coverage.
    pub alpha_to_coverage: bool,
    /// Enable alpha-to-one.
    pub alpha_to_one: bool,
}

impl Default for MsaaQuality {
    fn default() -> Self {
        Self {
            sample_count: SampleCount::S4,
            quality_level: 0,
            alpha_to_coverage: false,
            alpha_to_one: false,
        }
    }
}

impl MsaaQuality {
    /// No MSAA.
    pub const OFF: Self = Self {
        sample_count: SampleCount::S1,
        quality_level: 0,
        alpha_to_coverage: false,
        alpha_to_one: false,
    };

    /// 2x MSAA.
    pub const X2: Self = Self {
        sample_count: SampleCount::S2,
        quality_level: 0,
        alpha_to_coverage: false,
        alpha_to_one: false,
    };

    /// 4x MSAA.
    pub const X4: Self = Self {
        sample_count: SampleCount::S4,
        quality_level: 0,
        alpha_to_coverage: false,
        alpha_to_one: false,
    };

    /// 8x MSAA.
    pub const X8: Self = Self {
        sample_count: SampleCount::S8,
        quality_level: 0,
        alpha_to_coverage: false,
        alpha_to_one: false,
    };
}

/// Sample positions for various MSAA modes.
pub mod sample_positions {
    use super::*;

    /// Standard 2x MSAA sample positions.
    pub const MSAA_2X: [(f32, f32); 2] = [(0.25, 0.25), (0.75, 0.75)];

    /// Standard 4x MSAA sample positions (rotated grid).
    pub const MSAA_4X: [(f32, f32); 4] = [
        (0.375, 0.125),
        (0.875, 0.375),
        (0.125, 0.625),
        (0.625, 0.875),
    ];

    /// Standard 8x MSAA sample positions.
    pub const MSAA_8X: [(f32, f32); 8] = [
        (0.5625, 0.3125),
        (0.4375, 0.6875),
        (0.8125, 0.5625),
        (0.3125, 0.1875),
        (0.1875, 0.8125),
        (0.0625, 0.4375),
        (0.6875, 0.9375),
        (0.9375, 0.0625),
    ];

    /// Get sample positions for a sample count.
    pub fn get_positions(sample_count: SampleCount) -> &'static [(f32, f32)] {
        match sample_count {
            SampleCount::S1 => &[(0.5, 0.5)],
            SampleCount::S2 => &MSAA_2X,
            SampleCount::S4 => &MSAA_4X,
            SampleCount::S8 => &MSAA_8X,
            SampleCount::S16 => &MSAA_8X, // Use 8x positions, implementation varies
        }
    }
}

/// Coverage mask for MSAA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageMask(pub u32);

impl CoverageMask {
    /// No samples covered.
    pub const NONE: Self = Self(0);

    /// All samples covered (for given sample count).
    pub fn all(sample_count: SampleCount) -> Self {
        Self((1 << sample_count.count()) - 1)
    }

    /// Check if a specific sample is covered.
    pub fn is_covered(&self, sample: u32) -> bool {
        (self.0 & (1 << sample)) != 0
    }

    /// Count covered samples.
    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }

    /// Calculate coverage percentage.
    pub fn coverage(&self, sample_count: SampleCount) -> f32 {
        self.count() as f32 / sample_count.count() as f32
    }
}

impl std::ops::BitOr for CoverageMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for CoverageMask {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::Not for CoverageMask {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_count() {
        assert_eq!(SampleCount::S4.count(), 4);
        assert!(SampleCount::S4.is_msaa());
        assert!(!SampleCount::S1.is_msaa());
    }

    #[test]
    fn test_sample_count_navigation() {
        assert_eq!(SampleCount::S4.lower(), SampleCount::S2);
        assert_eq!(SampleCount::S4.higher(), SampleCount::S8);
        assert_eq!(SampleCount::S1.lower(), SampleCount::S1);
        assert_eq!(SampleCount::S16.higher(), SampleCount::S16);
    }

    #[test]
    fn test_sample_count_from_count() {
        assert_eq!(SampleCount::from_count(4), SampleCount::S4);
        assert_eq!(SampleCount::from_count(5), SampleCount::S8);
        assert_eq!(SampleCount::from_count(0), SampleCount::S1);
    }

    #[test]
    fn test_msaa_config_memory() {
        let config = MsaaConfig::new(SampleCount::S4, TextureFormat::Rgba8Unorm, 1920, 1080);

        let memory = config.memory_estimate();
        // 1920 * 1080 * 4 bytes * 4 samples = ~31.6 MB
        assert!(memory > 30_000_000);
        assert!(memory < 35_000_000);
    }

    #[test]
    fn test_msaa_quality_presets() {
        assert_eq!(MsaaQuality::OFF.sample_count, SampleCount::S1);
        assert_eq!(MsaaQuality::X4.sample_count, SampleCount::S4);
        assert_eq!(MsaaQuality::X8.sample_count, SampleCount::S8);
    }

    #[test]
    fn test_coverage_mask() {
        let mask = CoverageMask::all(SampleCount::S4);
        assert_eq!(mask.0, 0b1111);
        assert_eq!(mask.count(), 4);
        assert!((mask.coverage(SampleCount::S4) - 1.0).abs() < 0.001);

        assert!(mask.is_covered(0));
        assert!(mask.is_covered(3));
        assert!(!mask.is_covered(4));
    }

    #[test]
    fn test_coverage_mask_ops() {
        let a = CoverageMask(0b1010);
        let b = CoverageMask(0b1100);

        assert_eq!((a | b).0, 0b1110);
        assert_eq!((a & b).0, 0b1000);
    }

    #[test]
    fn test_sample_positions() {
        let pos_4x = sample_positions::get_positions(SampleCount::S4);
        assert_eq!(pos_4x.len(), 4);

        // All positions should be in [0, 1]
        for (x, y) in pos_4x {
            assert!(*x >= 0.0 && *x <= 1.0);
            assert!(*y >= 0.0 && *y <= 1.0);
        }
    }
}
