//! Gradient texture generation for GPU rendering.
//!
//! This module provides utilities for converting gradient definitions
//! into textures suitable for GPU sampling.

use skia_rs_core::{Color4f, Point, Scalar};

/// Gradient stop.
#[derive(Debug, Clone, Copy)]
pub struct GradientStop {
    /// Position (0.0 to 1.0).
    pub position: f32,
    /// Color at this position.
    pub color: Color4f,
}

impl GradientStop {
    /// Create a new gradient stop.
    pub fn new(position: f32, color: Color4f) -> Self {
        Self {
            position: position.clamp(0.0, 1.0),
            color,
        }
    }
}

/// Gradient type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientType {
    /// Linear gradient.
    Linear,
    /// Radial gradient.
    Radial,
    /// Sweep (angular) gradient.
    Sweep,
    /// Two-point conical gradient.
    TwoPointConical,
}

/// Tile mode for gradient edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradientTileMode {
    /// Clamp to edge colors.
    #[default]
    Clamp,
    /// Repeat the gradient.
    Repeat,
    /// Mirror the gradient.
    Mirror,
    /// Decal mode (transparent outside).
    Decal,
}

/// Configuration for gradient texture generation.
#[derive(Debug, Clone)]
pub struct GradientTextureConfig {
    /// Texture width.
    pub width: u32,
    /// Texture height (1 for 1D gradients).
    pub height: u32,
    /// Use sRGB color space.
    pub srgb: bool,
    /// Premultiply alpha.
    pub premultiply: bool,
    /// Generate mipmaps.
    pub mipmaps: bool,
}

impl Default for GradientTextureConfig {
    fn default() -> Self {
        Self {
            width: 256,
            height: 1,
            srgb: true,
            premultiply: true,
            mipmaps: false,
        }
    }
}

/// Generate a 1D gradient texture.
pub fn generate_gradient_texture_1d(
    stops: &[GradientStop],
    tile_mode: GradientTileMode,
    config: &GradientTextureConfig,
) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(config.width as usize * 4);

    // Ensure stops are sorted
    let mut sorted_stops = stops.to_vec();
    sorted_stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());

    // Ensure we have at least 2 stops
    if sorted_stops.is_empty() {
        sorted_stops.push(GradientStop::new(0.0, Color4f::black()));
        sorted_stops.push(GradientStop::new(1.0, Color4f::black()));
    } else if sorted_stops.len() == 1 {
        let color = sorted_stops[0].color;
        sorted_stops.clear();
        sorted_stops.push(GradientStop::new(0.0, color));
        sorted_stops.push(GradientStop::new(1.0, color));
    }

    // Add implicit stops at 0 and 1 if needed
    if sorted_stops[0].position > 0.0 {
        sorted_stops.insert(0, GradientStop::new(0.0, sorted_stops[0].color));
    }
    if sorted_stops.last().unwrap().position < 1.0 {
        let last_color = sorted_stops.last().unwrap().color;
        sorted_stops.push(GradientStop::new(1.0, last_color));
    }

    for x in 0..config.width {
        let mut t = x as f32 / (config.width - 1) as f32;

        // Apply tile mode
        t = apply_tile_mode(t, tile_mode);

        // Find surrounding stops
        let color = sample_gradient(&sorted_stops, t);

        // Apply premultiplication if requested
        let (r, g, b, a) = if config.premultiply {
            (
                color.r * color.a,
                color.g * color.a,
                color.b * color.a,
                color.a,
            )
        } else {
            (color.r, color.g, color.b, color.a)
        };

        // Apply sRGB conversion if requested
        let (r, g, b) = if config.srgb {
            (linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b))
        } else {
            (r, g, b)
        };

        // Convert to bytes
        pixels.push((r * 255.0).round() as u8);
        pixels.push((g * 255.0).round() as u8);
        pixels.push((b * 255.0).round() as u8);
        pixels.push((a * 255.0).round() as u8);
    }

    pixels
}

/// Generate a 2D radial gradient texture.
pub fn generate_radial_gradient_texture(
    stops: &[GradientStop],
    tile_mode: GradientTileMode,
    config: &GradientTextureConfig,
) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((config.width * config.height) as usize * 4);

    // Ensure stops are sorted
    let mut sorted_stops = stops.to_vec();
    sorted_stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());

    if sorted_stops.is_empty() {
        sorted_stops.push(GradientStop::new(0.0, Color4f::black()));
        sorted_stops.push(GradientStop::new(1.0, Color4f::black()));
    }

    let center_x = config.width as f32 * 0.5;
    let center_y = config.height as f32 * 0.5;
    let max_radius = (center_x * center_x + center_y * center_y).sqrt();

    for y in 0..config.height {
        for x in 0..config.width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let mut t = (dx * dx + dy * dy).sqrt() / max_radius;

            t = apply_tile_mode(t, tile_mode);

            let color = sample_gradient(&sorted_stops, t);

            let (r, g, b, a) = if config.premultiply {
                (
                    color.r * color.a,
                    color.g * color.a,
                    color.b * color.a,
                    color.a,
                )
            } else {
                (color.r, color.g, color.b, color.a)
            };

            let (r, g, b) = if config.srgb {
                (linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b))
            } else {
                (r, g, b)
            };

            pixels.push((r * 255.0).round() as u8);
            pixels.push((g * 255.0).round() as u8);
            pixels.push((b * 255.0).round() as u8);
            pixels.push((a * 255.0).round() as u8);
        }
    }

    pixels
}

/// Generate a sweep gradient texture.
pub fn generate_sweep_gradient_texture(
    stops: &[GradientStop],
    tile_mode: GradientTileMode,
    config: &GradientTextureConfig,
) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((config.width * config.height) as usize * 4);

    let mut sorted_stops = stops.to_vec();
    sorted_stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());

    if sorted_stops.is_empty() {
        sorted_stops.push(GradientStop::new(0.0, Color4f::black()));
        sorted_stops.push(GradientStop::new(1.0, Color4f::black()));
    }

    let center_x = config.width as f32 * 0.5;
    let center_y = config.height as f32 * 0.5;

    for y in 0..config.height {
        for x in 0..config.width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let mut angle = dy.atan2(dx);

            // Normalize to 0-1 range
            let mut t = (angle + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);
            t = apply_tile_mode(t, tile_mode);

            let color = sample_gradient(&sorted_stops, t);

            let (r, g, b, a) = if config.premultiply {
                (
                    color.r * color.a,
                    color.g * color.a,
                    color.b * color.a,
                    color.a,
                )
            } else {
                (color.r, color.g, color.b, color.a)
            };

            let (r, g, b) = if config.srgb {
                (linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b))
            } else {
                (r, g, b)
            };

            pixels.push((r * 255.0).round() as u8);
            pixels.push((g * 255.0).round() as u8);
            pixels.push((b * 255.0).round() as u8);
            pixels.push((a * 255.0).round() as u8);
        }
    }

    pixels
}

/// Apply tile mode to a gradient position.
fn apply_tile_mode(t: f32, mode: GradientTileMode) -> f32 {
    match mode {
        GradientTileMode::Clamp => t.clamp(0.0, 1.0),
        GradientTileMode::Repeat => t.rem_euclid(1.0),
        GradientTileMode::Mirror => {
            let cycle = t.rem_euclid(2.0);
            if cycle > 1.0 { 2.0 - cycle } else { cycle }
        }
        GradientTileMode::Decal => {
            if t < 0.0 || t > 1.0 {
                -1.0 // Signal for transparent
            } else {
                t
            }
        }
    }
}

/// Sample gradient at position t.
fn sample_gradient(stops: &[GradientStop], t: f32) -> Color4f {
    if t < 0.0 {
        return Color4f::transparent();
    }

    // Find surrounding stops
    let mut lower_idx = 0;
    for (i, stop) in stops.iter().enumerate() {
        if stop.position <= t {
            lower_idx = i;
        } else {
            break;
        }
    }

    let upper_idx = (lower_idx + 1).min(stops.len() - 1);

    if lower_idx == upper_idx {
        return stops[lower_idx].color;
    }

    let lower = &stops[lower_idx];
    let upper = &stops[upper_idx];

    // Interpolate
    let range = upper.position - lower.position;
    let blend = if range > 0.0 {
        (t - lower.position) / range
    } else {
        0.0
    };

    Color4f::new(
        lower.color.r + (upper.color.r - lower.color.r) * blend,
        lower.color.g + (upper.color.g - lower.color.g) * blend,
        lower.color.b + (upper.color.b - lower.color.b) * blend,
        lower.color.a + (upper.color.a - lower.color.a) * blend,
    )
}

/// Convert linear color component to sRGB.
fn linear_to_srgb(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert sRGB color component to linear.
pub fn srgb_to_linear(srgb: f32) -> f32 {
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

/// Gradient lookup table for shader use.
#[derive(Debug, Clone)]
pub struct GradientLUT {
    /// RGBA data.
    pub data: Vec<u8>,
    /// Width of the lookup table.
    pub width: u32,
}

impl GradientLUT {
    /// Create a new gradient LUT from stops.
    pub fn from_stops(stops: &[GradientStop], width: u32, tile_mode: GradientTileMode) -> Self {
        let config = GradientTextureConfig {
            width,
            height: 1,
            srgb: false, // Keep linear for shader use
            premultiply: true,
            mipmaps: false,
        };

        let data = generate_gradient_texture_1d(stops, tile_mode, &config);
        Self { data, width }
    }

    /// Sample the LUT at position t.
    pub fn sample(&self, t: f32) -> Color4f {
        let t = t.clamp(0.0, 1.0);
        let x = (t * (self.width - 1) as f32).round() as usize;
        let idx = x * 4;

        if idx + 3 < self.data.len() {
            Color4f::new(
                self.data[idx] as f32 / 255.0,
                self.data[idx + 1] as f32 / 255.0,
                self.data[idx + 2] as f32 / 255.0,
                self.data[idx + 3] as f32 / 255.0,
            )
        } else {
            Color4f::transparent()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_stop() {
        let stop = GradientStop::new(0.5, Color4f::from_rgb(1.0, 0.0, 0.0));
        assert_eq!(stop.position, 0.5);
        assert_eq!(stop.color.r, 1.0);
    }

    #[test]
    fn test_gradient_stop_clamping() {
        let stop = GradientStop::new(1.5, Color4f::from_rgb(1.0, 0.0, 0.0));
        assert_eq!(stop.position, 1.0);

        let stop = GradientStop::new(-0.5, Color4f::from_rgb(1.0, 0.0, 0.0));
        assert_eq!(stop.position, 0.0);
    }

    #[test]
    fn test_generate_gradient_1d() {
        let stops = vec![
            GradientStop::new(0.0, Color4f::from_rgb(1.0, 0.0, 0.0)),
            GradientStop::new(1.0, Color4f::from_rgb(0.0, 0.0, 1.0)),
        ];

        let config = GradientTextureConfig {
            width: 256,
            height: 1,
            srgb: false,
            premultiply: false,
            mipmaps: false,
        };

        let pixels = generate_gradient_texture_1d(&stops, GradientTileMode::Clamp, &config);
        assert_eq!(pixels.len(), 256 * 4);

        // First pixel should be red
        assert!(pixels[0] > 200); // R
        assert!(pixels[2] < 50); // B

        // Last pixel should be blue
        let last = (255 * 4) as usize;
        assert!(pixels[last] < 50); // R
        assert!(pixels[last + 2] > 200); // B
    }

    #[test]
    fn test_tile_mode_repeat() {
        let t = apply_tile_mode(1.5, GradientTileMode::Repeat);
        assert!((t - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_tile_mode_mirror() {
        let t = apply_tile_mode(1.5, GradientTileMode::Mirror);
        assert!((t - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_linear_srgb_conversion() {
        let linear = 0.5;
        let srgb = linear_to_srgb(linear);
        let back = srgb_to_linear(srgb);
        assert!((linear - back).abs() < 0.001);
    }

    #[test]
    fn test_gradient_lut() {
        let stops = vec![
            GradientStop::new(0.0, Color4f::from_rgb(1.0, 0.0, 0.0)),
            GradientStop::new(1.0, Color4f::from_rgb(0.0, 0.0, 1.0)),
        ];

        let lut = GradientLUT::from_stops(&stops, 256, GradientTileMode::Clamp);

        let start = lut.sample(0.0);
        assert!(start.r > 0.9);

        let end = lut.sample(1.0);
        assert!(end.b > 0.9);

        let mid = lut.sample(0.5);
        assert!(mid.r > 0.3 && mid.r < 0.7);
        assert!(mid.b > 0.3 && mid.b < 0.7);
    }
}
