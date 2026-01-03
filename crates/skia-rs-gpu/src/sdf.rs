//! Signed Distance Field (SDF) rendering for GPU text and shapes.
//!
//! This module provides utilities for generating and rendering SDFs,
//! which enable resolution-independent rendering of text and vector shapes.

use skia_rs_core::{Point, Rect, Scalar};

/// SDF generation configuration.
#[derive(Debug, Clone)]
pub struct SdfConfig {
    /// Output texture size.
    pub size: u32,
    /// Padding around the shape.
    pub padding: u32,
    /// Spread (distance field radius in pixels).
    pub spread: f32,
    /// Scale factor for generating the SDF.
    pub scale: f32,
}

impl Default for SdfConfig {
    fn default() -> Self {
        Self {
            size: 64,
            padding: 4,
            spread: 8.0,
            scale: 1.0,
        }
    }
}

impl SdfConfig {
    /// Create a configuration for high-resolution SDF.
    pub fn high_res() -> Self {
        Self {
            size: 128,
            padding: 8,
            spread: 16.0,
            scale: 1.0,
        }
    }

    /// Create a configuration for compact SDF.
    pub fn compact() -> Self {
        Self {
            size: 32,
            padding: 2,
            spread: 4.0,
            scale: 1.0,
        }
    }
}

/// SDF render parameters for shader use.
#[derive(Debug, Clone, Copy)]
pub struct SdfRenderParams {
    /// Smoothing factor (typically 0.25 / spread).
    pub smoothing: f32,
    /// Outline width (0 = no outline).
    pub outline_width: f32,
    /// Soft edge factor.
    pub soft_edge: f32,
    /// Distance threshold for rendering.
    pub threshold: f32,
}

impl Default for SdfRenderParams {
    fn default() -> Self {
        Self {
            smoothing: 0.25 / 8.0,
            outline_width: 0.0,
            soft_edge: 0.0,
            threshold: 0.5,
        }
    }
}

impl SdfRenderParams {
    /// Create parameters for crisp rendering.
    pub fn crisp(spread: f32) -> Self {
        Self {
            smoothing: 0.1 / spread,
            outline_width: 0.0,
            soft_edge: 0.0,
            threshold: 0.5,
        }
    }

    /// Create parameters for soft rendering.
    pub fn soft(spread: f32) -> Self {
        Self {
            smoothing: 0.5 / spread,
            outline_width: 0.0,
            soft_edge: 0.2,
            threshold: 0.5,
        }
    }

    /// Create parameters with outline.
    pub fn with_outline(spread: f32, outline_width: f32) -> Self {
        Self {
            smoothing: 0.25 / spread,
            outline_width,
            soft_edge: 0.0,
            threshold: 0.5,
        }
    }
}

/// Generate a signed distance field from a binary mask.
pub fn generate_sdf_from_mask(
    mask: &[u8],
    width: u32,
    height: u32,
    spread: f32,
) -> Vec<f32> {
    let mut sdf = vec![0.0f32; (width * height) as usize];

    // Two-pass algorithm: compute distance transform
    // This is a simplified version; production would use a better algorithm

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let is_inside = mask[idx] > 127;

            // Find nearest opposite pixel
            let mut min_dist_sq = f32::MAX;

            let search_radius = (spread * 2.0).ceil() as i32;
            for dy in -search_radius..=search_radius {
                for dx in -search_radius..=search_radius {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;

                    if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                        let nidx = (ny as u32 * width + nx as u32) as usize;
                        let neighbor_inside = mask[nidx] > 127;

                        if is_inside != neighbor_inside {
                            let dist_sq = (dx * dx + dy * dy) as f32;
                            min_dist_sq = min_dist_sq.min(dist_sq);
                        }
                    }
                }
            }

            let dist = if min_dist_sq == f32::MAX {
                spread
            } else {
                min_dist_sq.sqrt()
            };

            // Signed distance: negative inside, positive outside
            sdf[idx] = if is_inside { -dist } else { dist };
        }
    }

    sdf
}

/// Convert SDF to normalized texture data (0-255).
pub fn sdf_to_texture(sdf: &[f32], spread: f32) -> Vec<u8> {
    sdf.iter()
        .map(|&d| {
            // Map [-spread, spread] to [0, 1], then to [0, 255]
            let normalized = (d / spread + 1.0) * 0.5;
            (normalized.clamp(0.0, 1.0) * 255.0).round() as u8
        })
        .collect()
}

/// Sample SDF at a point with bilinear filtering.
pub fn sample_sdf_bilinear(sdf: &[f32], width: u32, height: u32, x: f32, y: f32) -> f32 {
    let x0 = (x.floor() as i32).clamp(0, width as i32 - 1) as u32;
    let y0 = (y.floor() as i32).clamp(0, height as i32 - 1) as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);

    let fx = x - x.floor();
    let fy = y - y.floor();

    let d00 = sdf[(y0 * width + x0) as usize];
    let d10 = sdf[(y0 * width + x1) as usize];
    let d01 = sdf[(y1 * width + x0) as usize];
    let d11 = sdf[(y1 * width + x1) as usize];

    let d0 = d00 * (1.0 - fx) + d10 * fx;
    let d1 = d01 * (1.0 - fx) + d11 * fx;

    d0 * (1.0 - fy) + d1 * fy
}

/// Generate SDF for a circle.
pub fn generate_circle_sdf(size: u32, radius: f32) -> Vec<f32> {
    let mut sdf = Vec::with_capacity((size * size) as usize);
    let center = size as f32 * 0.5;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            let dist = (dx * dx + dy * dy).sqrt() - radius;
            sdf.push(dist);
        }
    }

    sdf
}

/// Generate SDF for a rounded rectangle.
pub fn generate_rounded_rect_sdf(size: u32, rect: Rect, radius: f32) -> Vec<f32> {
    let mut sdf = Vec::with_capacity((size * size) as usize);

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            // Distance to rounded rectangle
            let dist = sdf_rounded_rect(px, py, &rect, radius);
            sdf.push(dist);
        }
    }

    sdf
}

/// Calculate SDF for a rounded rectangle at a point.
fn sdf_rounded_rect(x: f32, y: f32, rect: &Rect, radius: f32) -> f32 {
    let center = rect.center();
    let cx = center.x;
    let cy = center.y;
    let hw = rect.width() * 0.5 - radius;
    let hh = rect.height() * 0.5 - radius;

    let dx = (x - cx).abs() - hw;
    let dy = (y - cy).abs() - hh;

    let outside_dist = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    let inside_dist = dx.max(dy).min(0.0);

    outside_dist + inside_dist - radius
}

/// Multi-channel SDF data (for improved quality).
#[derive(Debug, Clone)]
pub struct MsdfData {
    /// Red channel (SDF).
    pub r: Vec<f32>,
    /// Green channel (SDF).
    pub g: Vec<f32>,
    /// Blue channel (SDF).
    pub b: Vec<f32>,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl MsdfData {
    /// Create empty MSDF data.
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Self {
            r: vec![0.0; size],
            g: vec![0.0; size],
            b: vec![0.0; size],
            width,
            height,
        }
    }

    /// Convert to RGB texture data.
    pub fn to_texture(&self, spread: f32) -> Vec<u8> {
        let size = (self.width * self.height) as usize;
        let mut data = Vec::with_capacity(size * 3);

        for i in 0..size {
            let r = ((self.r[i] / spread + 1.0) * 0.5).clamp(0.0, 1.0);
            let g = ((self.g[i] / spread + 1.0) * 0.5).clamp(0.0, 1.0);
            let b = ((self.b[i] / spread + 1.0) * 0.5).clamp(0.0, 1.0);

            data.push((r * 255.0).round() as u8);
            data.push((g * 255.0).round() as u8);
            data.push((b * 255.0).round() as u8);
        }

        data
    }

    /// Sample median value for MSDF rendering.
    pub fn sample_median(&self, x: f32, y: f32) -> f32 {
        let r = sample_sdf_bilinear(&self.r, self.width, self.height, x, y);
        let g = sample_sdf_bilinear(&self.g, self.width, self.height, x, y);
        let b = sample_sdf_bilinear(&self.b, self.width, self.height, x, y);

        // Median of three
        r.max(g.min(b)).min(g.max(b))
    }
}

/// SDF glyph metrics.
#[derive(Debug, Clone, Copy)]
pub struct SdfGlyphMetrics {
    /// UV coordinates in atlas [u0, v0, u1, v1].
    pub uv: [f32; 4],
    /// Offset from baseline.
    pub offset: Point,
    /// Size in pixels.
    pub size: [f32; 2],
    /// Advance width.
    pub advance: f32,
    /// SDF spread used.
    pub spread: f32,
}

/// SDF text rendering batch.
#[derive(Debug, Clone)]
pub struct SdfTextBatch {
    /// Instances to render.
    pub instances: Vec<SdfTextInstance>,
    /// Render parameters.
    pub params: SdfRenderParams,
}

/// SDF text instance.
#[derive(Debug, Clone, Copy)]
pub struct SdfTextInstance {
    /// Position on screen.
    pub position: Point,
    /// UV coordinates.
    pub uv: [f32; 4],
    /// Size.
    pub size: [f32; 2],
    /// Color (RGBA).
    pub color: [f32; 4],
    /// Scale factor.
    pub scale: f32,
}

impl SdfTextBatch {
    /// Create a new batch.
    pub fn new(params: SdfRenderParams) -> Self {
        Self {
            instances: Vec::new(),
            params,
        }
    }

    /// Add a glyph instance.
    pub fn add_glyph(
        &mut self,
        metrics: &SdfGlyphMetrics,
        position: Point,
        scale: f32,
        color: [f32; 4],
    ) {
        self.instances.push(SdfTextInstance {
            position: Point::new(
                position.x + metrics.offset.x * scale,
                position.y + metrics.offset.y * scale,
            ),
            uv: metrics.uv,
            size: [metrics.size[0] * scale, metrics.size[1] * scale],
            color,
            scale,
        });
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Get instance count.
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Clear the batch.
    pub fn clear(&mut self) {
        self.instances.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdf_config() {
        let config = SdfConfig::default();
        assert_eq!(config.size, 64);
        assert_eq!(config.spread, 8.0);

        let high = SdfConfig::high_res();
        assert!(high.size > config.size);
    }

    #[test]
    fn test_sdf_render_params() {
        let params = SdfRenderParams::default();
        assert!(params.smoothing > 0.0);
        assert_eq!(params.outline_width, 0.0);

        let outline = SdfRenderParams::with_outline(8.0, 2.0);
        assert_eq!(outline.outline_width, 2.0);
    }

    #[test]
    fn test_generate_circle_sdf() {
        let sdf = generate_circle_sdf(32, 10.0);
        assert_eq!(sdf.len(), 32 * 32);

        // Center should be inside (negative)
        let center_idx = 16 * 32 + 16;
        assert!(sdf[center_idx] < 0.0);

        // Corner should be outside (positive)
        assert!(sdf[0] > 0.0);
    }

    #[test]
    fn test_generate_rounded_rect_sdf() {
        let rect = Rect::from_xywh(4.0, 4.0, 24.0, 24.0);
        let sdf = generate_rounded_rect_sdf(32, rect, 4.0);
        assert_eq!(sdf.len(), 32 * 32);

        // Center should be inside
        let center_idx = 16 * 32 + 16;
        assert!(sdf[center_idx] < 0.0);
    }

    #[test]
    fn test_sdf_to_texture() {
        let sdf = vec![-8.0, 0.0, 8.0];
        let texture = sdf_to_texture(&sdf, 8.0);

        assert_eq!(texture.len(), 3);
        assert_eq!(texture[0], 0);   // -spread -> 0
        assert_eq!(texture[1], 128); // 0 -> 0.5 -> 128
        assert_eq!(texture[2], 255); // +spread -> 255
    }

    #[test]
    fn test_sample_sdf_bilinear() {
        let sdf = vec![0.0, 1.0, 2.0, 3.0];
        let val = sample_sdf_bilinear(&sdf, 2, 2, 0.5, 0.5);
        // Should be average of all four corners
        assert!((val - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_msdf_data() {
        let mut msdf = MsdfData::new(32, 32);
        assert_eq!(msdf.r.len(), 32 * 32);

        // Set some values
        msdf.r[0] = 1.0;
        msdf.g[0] = 2.0;
        msdf.b[0] = 3.0;

        let median = msdf.sample_median(0.0, 0.0);
        assert!((median - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_sdf_text_batch() {
        let params = SdfRenderParams::default();
        let mut batch = SdfTextBatch::new(params);

        assert!(batch.is_empty());

        let metrics = SdfGlyphMetrics {
            uv: [0.0, 0.0, 0.1, 0.1],
            offset: Point::new(0.0, -10.0),
            size: [16.0, 20.0],
            advance: 10.0,
            spread: 8.0,
        };

        batch.add_glyph(&metrics, Point::new(100.0, 100.0), 2.0, [1.0, 1.0, 1.0, 1.0]);

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.instances[0].size, [32.0, 40.0]); // Scaled by 2
    }
}
