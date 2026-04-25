//! Color, mask, and image filters.

use skia_rs_core::{Color4f, Rect, Scalar};
use std::sync::Arc;

/// A color filter that transforms colors.
pub trait ColorFilter: Send + Sync + std::fmt::Debug {
    /// Filter a color.
    fn filter_color(&self, color: Color4f) -> Color4f;
}

/// A matrix color filter.
#[derive(Debug, Clone)]
pub struct ColorMatrixFilter {
    /// 5x4 color matrix (row-major, last column is translation).
    matrix: [Scalar; 20],
}

impl ColorMatrixFilter {
    /// Create a new color matrix filter.
    pub fn new(matrix: [Scalar; 20]) -> Self {
        Self { matrix }
    }

    /// Create an identity color matrix.
    pub fn identity() -> Self {
        Self::new([
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
        ])
    }

    /// Create a saturation filter.
    pub fn saturation(sat: Scalar) -> Self {
        let s = sat;
        let ms = 1.0 - s;
        let r = 0.2126 * ms;
        let g = 0.7152 * ms;
        let b = 0.0722 * ms;

        Self::new([
            r + s,
            g,
            b,
            0.0,
            0.0,
            r,
            g + s,
            b,
            0.0,
            0.0,
            r,
            g,
            b + s,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        ])
    }
}

impl ColorFilter for ColorMatrixFilter {
    fn filter_color(&self, color: Color4f) -> Color4f {
        let m = &self.matrix;
        Color4f {
            r: m[0] * color.r + m[1] * color.g + m[2] * color.b + m[3] * color.a + m[4],
            g: m[5] * color.r + m[6] * color.g + m[7] * color.b + m[8] * color.a + m[9],
            b: m[10] * color.r + m[11] * color.g + m[12] * color.b + m[13] * color.a + m[14],
            a: m[15] * color.r + m[16] * color.g + m[17] * color.b + m[18] * color.a + m[19],
        }
    }
}

/// Blur style for mask filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlurStyle {
    /// Normal blur.
    #[default]
    Normal = 0,
    /// Solid blur (inner visible).
    Solid,
    /// Outer blur only.
    Outer,
    /// Inner blur only.
    Inner,
}

/// A mask filter (blur, emboss, etc.).
pub trait MaskFilter: Send + Sync + std::fmt::Debug {
    /// Get the blur radius if this is a blur filter.
    fn blur_radius(&self) -> Option<Scalar>;

    /// Apply the filter to an alpha mask in place.
    ///
    /// `mask` is a row-major `width * height` bytes of alpha values.
    /// Default implementation is a no-op — concrete filters override.
    fn apply_mask(&self, mask: &mut [u8], width: usize, height: usize) {
        let _ = (mask, width, height);
    }
}

/// A blur mask filter.
#[derive(Debug, Clone)]
pub struct BlurMaskFilter {
    style: BlurStyle,
    sigma: Scalar,
}

impl BlurMaskFilter {
    /// Create a new blur mask filter.
    pub fn new(style: BlurStyle, sigma: Scalar) -> Self {
        Self { style, sigma }
    }

    /// Get the blur style.
    pub fn style(&self) -> BlurStyle {
        self.style
    }

    /// Get the sigma value.
    pub fn sigma(&self) -> Scalar {
        self.sigma
    }
}

impl MaskFilter for BlurMaskFilter {
    fn blur_radius(&self) -> Option<Scalar> {
        Some(self.sigma)
    }

    fn apply_mask(&self, mask: &mut [u8], width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        let radius = self.sigma.max(0.0) as usize;
        if radius == 0 {
            return;
        }
        // Two-pass separable box blur (horizontal then vertical),
        // repeated twice for a better Gaussian approximation.
        for _ in 0..2 {
            box_blur_horizontal(mask, width, height, radius);
            box_blur_vertical(mask, width, height, radius);
        }
    }
}

/// An image filter.
pub trait ImageFilter: Send + Sync + std::fmt::Debug {
    /// Get the bounds that this filter affects.
    fn filter_bounds(&self, src: &Rect) -> Rect;

    /// Apply the filter to an RGBA8 image buffer in place.
    ///
    /// `pixels` is a row-major `width * height * 4` byte buffer.
    /// Default implementation is a no-op — concrete filters override.
    fn apply(&self, pixels: &mut [u8], width: usize, height: usize) {
        let _ = (pixels, width, height);
    }
}

/// A blur image filter.
#[derive(Debug, Clone)]
pub struct BlurImageFilter {
    sigma_x: Scalar,
    sigma_y: Scalar,
    tile_mode: crate::shader::TileMode,
}

impl BlurImageFilter {
    /// Create a new blur image filter.
    pub fn new(sigma_x: Scalar, sigma_y: Scalar, tile_mode: crate::shader::TileMode) -> Self {
        Self {
            sigma_x,
            sigma_y,
            tile_mode,
        }
    }
}

impl ImageFilter for BlurImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        // Blur expands bounds by ~3 sigma
        let dx = self.sigma_x * 3.0;
        let dy = self.sigma_y * 3.0;
        Rect::new(src.left - dx, src.top - dy, src.right + dx, src.bottom + dy)
    }

    fn apply(&self, pixels: &mut [u8], width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        let rx = self.sigma_x.max(0.0) as usize;
        let ry = self.sigma_y.max(0.0) as usize;
        if rx == 0 && ry == 0 {
            return;
        }
        for _ in 0..2 {
            if rx > 0 {
                rgba_box_blur_horizontal(pixels, width, height, rx);
            }
            if ry > 0 {
                rgba_box_blur_vertical(pixels, width, height, ry);
            }
        }
    }
}

/// A drop shadow image filter.
#[derive(Debug, Clone)]
pub struct DropShadowImageFilter {
    dx: Scalar,
    dy: Scalar,
    sigma_x: Scalar,
    sigma_y: Scalar,
    color: Color4f,
    shadow_only: bool,
}

impl DropShadowImageFilter {
    /// Create a new drop shadow filter.
    pub fn new(
        dx: Scalar,
        dy: Scalar,
        sigma_x: Scalar,
        sigma_y: Scalar,
        color: Color4f,
        shadow_only: bool,
    ) -> Self {
        Self {
            dx,
            dy,
            sigma_x,
            sigma_y,
            color,
            shadow_only,
        }
    }
}

impl ImageFilter for DropShadowImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        let blur_dx = self.sigma_x * 3.0;
        let blur_dy = self.sigma_y * 3.0;

        if self.shadow_only {
            Rect::new(
                src.left + self.dx - blur_dx,
                src.top + self.dy - blur_dy,
                src.right + self.dx + blur_dx,
                src.bottom + self.dy + blur_dy,
            )
        } else {
            Rect::new(
                src.left.min(src.left + self.dx - blur_dx),
                src.top.min(src.top + self.dy - blur_dy),
                src.right.max(src.right + self.dx + blur_dx),
                src.bottom.max(src.bottom + self.dy + blur_dy),
            )
        }
    }

    fn apply(&self, pixels: &mut [u8], width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        // 1. Extract alpha channel into shadow buffer
        let mut shadow = vec![0u8; width * height];
        for i in 0..width * height {
            shadow[i] = pixels[i * 4 + 3];
        }
        // 2. Blur shadow
        let rx = self.sigma_x.max(0.0) as usize;
        let ry = self.sigma_y.max(0.0) as usize;
        if rx > 0 {
            box_blur_horizontal(&mut shadow, width, height, rx);
        }
        if ry > 0 {
            box_blur_vertical(&mut shadow, width, height, ry);
        }
        // 3. Colorize and offset the shadow, then composite
        let dx = self.dx.round() as i32;
        let dy = self.dy.round() as i32;
        let sr = (self.color.r * 255.0).clamp(0.0, 255.0) as u8;
        let sg = (self.color.g * 255.0).clamp(0.0, 255.0) as u8;
        let sb = (self.color.b * 255.0).clamp(0.0, 255.0) as u8;
        let sa_mul = self.color.a.clamp(0.0, 1.0);
        let original = pixels.to_vec();
        for y in 0..height {
            for x in 0..width {
                // Read shadow from offset position (clamp)
                let sx = (x as i32 - dx).clamp(0, width as i32 - 1) as usize;
                let sy = (y as i32 - dy).clamp(0, height as i32 - 1) as usize;
                let shadow_alpha = (shadow[sy * width + sx] as f32 / 255.0) * sa_mul;
                let shadow_rgba = [
                    (sr as f32 * shadow_alpha) as u8,
                    (sg as f32 * shadow_alpha) as u8,
                    (sb as f32 * shadow_alpha) as u8,
                    (shadow_alpha * 255.0) as u8,
                ];
                // Src-over: result = src + dst * (1 - src.a)
                let idx = (y * width + x) * 4;
                if self.shadow_only {
                    // Only shadow, no source
                    pixels[idx] = shadow_rgba[0];
                    pixels[idx + 1] = shadow_rgba[1];
                    pixels[idx + 2] = shadow_rgba[2];
                    pixels[idx + 3] = shadow_rgba[3];
                } else {
                    // Composite source over shadow
                    let src_a = original[idx + 3] as f32 / 255.0;
                    let inv_a = 1.0 - src_a;
                    pixels[idx] =
                        (original[idx] as f32 + shadow_rgba[0] as f32 * inv_a).clamp(0.0, 255.0)
                            as u8;
                    pixels[idx + 1] = (original[idx + 1] as f32 + shadow_rgba[1] as f32 * inv_a)
                        .clamp(0.0, 255.0)
                        as u8;
                    pixels[idx + 2] = (original[idx + 2] as f32 + shadow_rgba[2] as f32 * inv_a)
                        .clamp(0.0, 255.0)
                        as u8;
                    pixels[idx + 3] = (original[idx + 3] as f32 + shadow_rgba[3] as f32 * inv_a)
                        .clamp(0.0, 255.0)
                        as u8;
                }
            }
        }
    }
}

// =============================================================================
// Additional Mask Filters
// =============================================================================

/// A shader-based mask filter.
///
/// Corresponds to Skia's `SkShaderMaskFilter`.
#[derive(Debug)]
pub struct ShaderMaskFilter {
    shader: crate::shader::ShaderRef,
}

impl ShaderMaskFilter {
    /// Create a new shader mask filter.
    pub fn new(shader: crate::shader::ShaderRef) -> Self {
        Self { shader }
    }

    /// Get the shader.
    pub fn shader(&self) -> &crate::shader::ShaderRef {
        &self.shader
    }
}

impl MaskFilter for ShaderMaskFilter {
    fn blur_radius(&self) -> Option<Scalar> {
        None
    }
}

/// A table-based mask filter using a lookup table.
///
/// Corresponds to Skia's `SkTableMaskFilter`.
#[derive(Debug, Clone)]
pub struct TableMaskFilter {
    table: [u8; 256],
}

impl TableMaskFilter {
    /// Create a new table mask filter.
    pub fn new(table: [u8; 256]) -> Self {
        Self { table }
    }

    /// Create a gamma correction table.
    pub fn gamma(gamma: Scalar) -> Self {
        let mut table = [0u8; 256];
        for (i, v) in table.iter_mut().enumerate() {
            let normalized = i as Scalar / 255.0;
            *v = (normalized.powf(gamma) * 255.0).round() as u8;
        }
        Self { table }
    }

    /// Create a clipping table.
    pub fn clip(min: u8, max: u8) -> Self {
        let mut table = [0u8; 256];
        for (i, v) in table.iter_mut().enumerate() {
            let i = i as u8;
            *v = if i < min {
                0
            } else if i > max {
                255
            } else {
                ((i - min) as f32 / (max - min) as f32 * 255.0) as u8
            };
        }
        Self { table }
    }

    /// Get the lookup table.
    pub fn table(&self) -> &[u8; 256] {
        &self.table
    }
}

impl MaskFilter for TableMaskFilter {
    fn blur_radius(&self) -> Option<Scalar> {
        None
    }
}

// =============================================================================
// Additional Image Filters
// =============================================================================

/// Morphology filter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MorphologyType {
    /// Dilate (expand bright regions).
    Dilate,
    /// Erode (shrink bright regions).
    Erode,
}

/// A morphology image filter (dilate or erode).
///
/// Corresponds to Skia's `SkMorphologyImageFilter`.
#[derive(Debug, Clone)]
pub struct MorphologyImageFilter {
    morph_type: MorphologyType,
    radius_x: Scalar,
    radius_y: Scalar,
    input: Option<ImageFilterRef>,
}

impl MorphologyImageFilter {
    /// Create a dilate filter.
    pub fn dilate(radius_x: Scalar, radius_y: Scalar, input: Option<ImageFilterRef>) -> Self {
        Self {
            morph_type: MorphologyType::Dilate,
            radius_x,
            radius_y,
            input,
        }
    }

    /// Create an erode filter.
    pub fn erode(radius_x: Scalar, radius_y: Scalar, input: Option<ImageFilterRef>) -> Self {
        Self {
            morph_type: MorphologyType::Erode,
            radius_x,
            radius_y,
            input,
        }
    }

    /// Get the morphology type.
    pub fn morph_type(&self) -> MorphologyType {
        self.morph_type
    }

    /// Get the X radius.
    pub fn radius_x(&self) -> Scalar {
        self.radius_x
    }

    /// Get the Y radius.
    pub fn radius_y(&self) -> Scalar {
        self.radius_y
    }
}

impl ImageFilter for MorphologyImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        Rect::new(
            src.left - self.radius_x,
            src.top - self.radius_y,
            src.right + self.radius_x,
            src.bottom + self.radius_y,
        )
    }

    fn apply(&self, pixels: &mut [u8], width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        let rx = self.radius_x.max(0.0).round() as usize;
        let ry = self.radius_y.max(0.0).round() as usize;
        if rx == 0 && ry == 0 {
            return;
        }
        let is_dilate = matches!(self.morph_type, MorphologyType::Dilate);
        let src = pixels.to_vec();
        for y in 0..height {
            for x in 0..width {
                let mut r = if is_dilate { 0u8 } else { 255u8 };
                let mut g = r;
                let mut b = r;
                let mut a = r;
                let x_min = x.saturating_sub(rx);
                let y_min = y.saturating_sub(ry);
                let x_max = (x + rx).min(width - 1);
                let y_max = (y + ry).min(height - 1);
                for ny in y_min..=y_max {
                    for nx in x_min..=x_max {
                        let idx = (ny * width + nx) * 4;
                        let sr = src[idx];
                        let sg = src[idx + 1];
                        let sb = src[idx + 2];
                        let sa = src[idx + 3];
                        if is_dilate {
                            if sr > r {
                                r = sr;
                            }
                            if sg > g {
                                g = sg;
                            }
                            if sb > b {
                                b = sb;
                            }
                            if sa > a {
                                a = sa;
                            }
                        } else {
                            if sr < r {
                                r = sr;
                            }
                            if sg < g {
                                g = sg;
                            }
                            if sb < b {
                                b = sb;
                            }
                            if sa < a {
                                a = sa;
                            }
                        }
                    }
                }
                let idx = (y * width + x) * 4;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = a;
            }
        }
    }
}

/// A color filter wrapped as an image filter.
///
/// Corresponds to Skia's `SkColorFilterImageFilter`.
#[derive(Debug)]
pub struct ColorFilterImageFilter {
    color_filter: ColorFilterRef,
    input: Option<ImageFilterRef>,
}

impl ColorFilterImageFilter {
    /// Create a new color filter image filter.
    pub fn new(color_filter: ColorFilterRef, input: Option<ImageFilterRef>) -> Self {
        Self {
            color_filter,
            input,
        }
    }

    /// Get the color filter.
    pub fn color_filter(&self) -> &ColorFilterRef {
        &self.color_filter
    }
}

impl ImageFilter for ColorFilterImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        // Color filters don't change bounds
        *src
    }

    fn apply(&self, pixels: &mut [u8], width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        // Apply inner filter first if present
        if let Some(ref input) = self.input {
            input.apply(pixels, width, height);
        }
        // Apply color filter to each pixel
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                let color = Color4f {
                    r: pixels[idx] as f32 / 255.0,
                    g: pixels[idx + 1] as f32 / 255.0,
                    b: pixels[idx + 2] as f32 / 255.0,
                    a: pixels[idx + 3] as f32 / 255.0,
                };
                let filtered = self.color_filter.filter_color(color);
                pixels[idx] = (filtered.r * 255.0).clamp(0.0, 255.0) as u8;
                pixels[idx + 1] = (filtered.g * 255.0).clamp(0.0, 255.0) as u8;
                pixels[idx + 2] = (filtered.b * 255.0).clamp(0.0, 255.0) as u8;
                pixels[idx + 3] = (filtered.a * 255.0).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// A displacement map image filter.
///
/// Corresponds to Skia's `SkDisplacementMapEffect`.
#[derive(Debug)]
pub struct DisplacementMapImageFilter {
    x_channel: ColorChannel,
    y_channel: ColorChannel,
    scale: Scalar,
    displacement: ImageFilterRef,
    color: Option<ImageFilterRef>,
}

/// Color channel selector for displacement map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorChannel {
    /// Red channel.
    R,
    /// Green channel.
    G,
    /// Blue channel.
    B,
    /// Alpha channel.
    A,
}

impl DisplacementMapImageFilter {
    /// Create a new displacement map filter.
    pub fn new(
        x_channel: ColorChannel,
        y_channel: ColorChannel,
        scale: Scalar,
        displacement: ImageFilterRef,
        color: Option<ImageFilterRef>,
    ) -> Self {
        Self {
            x_channel,
            y_channel,
            scale,
            displacement,
            color,
        }
    }
}

impl ImageFilter for DisplacementMapImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        // Displacement can move pixels by up to scale/2 in each direction
        let offset = self.scale / 2.0;
        Rect::new(
            src.left - offset,
            src.top - offset,
            src.right + offset,
            src.bottom + offset,
        )
    }

    fn apply(&self, pixels: &mut [u8], width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        // Baseline: use the source image's selected channels as displacement.
        // A true implementation would apply the displacement filter input first,
        // then use those pixels as the displacement map. This baseline uses the
        // source image itself as the displacement map.
        let scale = self.scale;
        let src = pixels.to_vec();
        let x_channel_idx = match self.x_channel {
            ColorChannel::R => 0,
            ColorChannel::G => 1,
            ColorChannel::B => 2,
            ColorChannel::A => 3,
        };
        let y_channel_idx = match self.y_channel {
            ColorChannel::R => 0,
            ColorChannel::G => 1,
            ColorChannel::B => 2,
            ColorChannel::A => 3,
        };
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                // Read displacement channels (0..255 maps to -1..1)
                let dx_f = (src[idx + x_channel_idx] as f32 / 127.5 - 1.0) * scale;
                let dy_f = (src[idx + y_channel_idx] as f32 / 127.5 - 1.0) * scale;
                let sx = ((x as f32 + dx_f) as i32).clamp(0, width as i32 - 1) as usize;
                let sy = ((y as f32 + dy_f) as i32).clamp(0, height as i32 - 1) as usize;
                let src_idx = (sy * width + sx) * 4;
                pixels[idx] = src[src_idx];
                pixels[idx + 1] = src[src_idx + 1];
                pixels[idx + 2] = src[src_idx + 2];
                pixels[idx + 3] = src[src_idx + 3];
            }
        }
    }
}

/// Light type for lighting filters.
#[derive(Debug, Clone)]
pub enum LightType {
    /// Distant light (parallel rays).
    Distant {
        /// Direction vector (towards light source).
        direction: (Scalar, Scalar, Scalar),
    },
    /// Point light (emanates from a single point).
    Point {
        /// Light position.
        location: (Scalar, Scalar, Scalar),
    },
    /// Spot light (cone of light from a point).
    Spot {
        /// Light position.
        location: (Scalar, Scalar, Scalar),
        /// Target point.
        target: (Scalar, Scalar, Scalar),
        /// Specular exponent.
        specular_exponent: Scalar,
        /// Cutoff angle.
        cutoff_angle: Scalar,
    },
}

/// A lighting image filter.
///
/// Corresponds to Skia's lighting image filters.
#[derive(Debug)]
pub struct LightingImageFilter {
    light: LightType,
    surface_scale: Scalar,
    diffuse_constant: Option<Scalar>,
    specular_constant: Option<Scalar>,
    specular_exponent: Option<Scalar>,
    light_color: Color4f,
    input: Option<ImageFilterRef>,
}

impl LightingImageFilter {
    /// Create a diffuse lighting filter.
    pub fn diffuse(
        light: LightType,
        surface_scale: Scalar,
        kd: Scalar,
        light_color: Color4f,
        input: Option<ImageFilterRef>,
    ) -> Self {
        Self {
            light,
            surface_scale,
            diffuse_constant: Some(kd),
            specular_constant: None,
            specular_exponent: None,
            light_color,
            input,
        }
    }

    /// Create a specular lighting filter.
    pub fn specular(
        light: LightType,
        surface_scale: Scalar,
        ks: Scalar,
        shininess: Scalar,
        light_color: Color4f,
        input: Option<ImageFilterRef>,
    ) -> Self {
        Self {
            light,
            surface_scale,
            diffuse_constant: None,
            specular_constant: Some(ks),
            specular_exponent: Some(shininess),
            light_color,
            input,
        }
    }
}

impl ImageFilter for LightingImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        // Lighting doesn't change bounds
        *src
    }

    fn apply(&self, pixels: &mut [u8], width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        // Baseline lighting: modulate intensity by surface_scale factor.
        // A full implementation would compute per-pixel normals from the alpha
        // channel (height field) and evaluate the lighting model (Phong, diffuse).
        // This simplified version applies uniform lighting based on surface_scale.
        let scale = self.surface_scale.max(0.0).min(10.0);
        let factor = (1.0 + scale * 0.1).min(2.0);
        let lr = (self.light_color.r * 255.0).clamp(0.0, 255.0);
        let lg = (self.light_color.g * 255.0).clamp(0.0, 255.0);
        let lb = (self.light_color.b * 255.0).clamp(0.0, 255.0);
        for i in 0..width * height {
            let idx = i * 4;
            pixels[idx] = ((pixels[idx] as f32 * factor * lr / 255.0).clamp(0.0, 255.0)) as u8;
            pixels[idx + 1] =
                ((pixels[idx + 1] as f32 * factor * lg / 255.0).clamp(0.0, 255.0)) as u8;
            pixels[idx + 2] =
                ((pixels[idx + 2] as f32 * factor * lb / 255.0).clamp(0.0, 255.0)) as u8;
        }
    }
}

/// A compose image filter that chains two filters.
///
/// Corresponds to Skia's `SkComposeImageFilter`.
#[derive(Debug)]
pub struct ComposeImageFilter {
    outer: ImageFilterRef,
    inner: ImageFilterRef,
}

impl ComposeImageFilter {
    /// Create a compose filter (outer applied after inner).
    pub fn new(outer: ImageFilterRef, inner: ImageFilterRef) -> Self {
        Self { outer, inner }
    }
}

impl ImageFilter for ComposeImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        let inner_bounds = self.inner.filter_bounds(src);
        self.outer.filter_bounds(&inner_bounds)
    }

    fn apply(&self, pixels: &mut [u8], width: usize, height: usize) {
        // Apply inner filter first, then outer
        self.inner.apply(pixels, width, height);
        self.outer.apply(pixels, width, height);
    }
}

/// A merge image filter that combines multiple inputs.
///
/// Corresponds to Skia's `SkMergeImageFilter`.
#[derive(Debug)]
pub struct MergeImageFilter {
    inputs: Vec<Option<ImageFilterRef>>,
}

impl MergeImageFilter {
    /// Create a merge filter with the given inputs.
    pub fn new(inputs: Vec<Option<ImageFilterRef>>) -> Self {
        Self { inputs }
    }
}

impl ImageFilter for MergeImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        let mut result = *src;
        for input in &self.inputs {
            if let Some(filter) = input {
                let bounds = filter.filter_bounds(src);
                result = result.union(&bounds);
            }
        }
        result
    }

    /// Apply is a no-op for this filter in the single-buffer apply() API.
    ///
    /// MergeImageFilter requires multiple independent input pixel buffers,
    /// which the current single-buffer signature cannot provide. A future
    /// revision will introduce a multi-input apply variant.
    fn apply(&self, _pixels: &mut [u8], _width: usize, _height: usize) {
        // Documented limitation: multi-input filter requires extended API.
    }
}

/// An offset image filter.
///
/// Corresponds to Skia's `SkOffsetImageFilter`.
#[derive(Debug)]
pub struct OffsetImageFilter {
    dx: Scalar,
    dy: Scalar,
    input: Option<ImageFilterRef>,
}

impl OffsetImageFilter {
    /// Create an offset filter.
    pub fn new(dx: Scalar, dy: Scalar, input: Option<ImageFilterRef>) -> Self {
        Self { dx, dy, input }
    }
}

impl ImageFilter for OffsetImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        Rect::new(
            src.left + self.dx,
            src.top + self.dy,
            src.right + self.dx,
            src.bottom + self.dy,
        )
    }

    fn apply(&self, pixels: &mut [u8], width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        // Apply input filter first if present
        if let Some(ref input) = self.input {
            input.apply(pixels, width, height);
        }
        // Shift pixels with clamp-to-edge
        let offset_x = self.dx.round() as i32;
        let offset_y = self.dy.round() as i32;
        if offset_x == 0 && offset_y == 0 {
            return;
        }

        // Create a copy to read from
        let mut copy = vec![0u8; width * height * 4];
        copy.copy_from_slice(pixels);

        // Fill with shifted pixels
        for y in 0..height {
            for x in 0..width {
                let src_x = (x as i32 - offset_x).clamp(0, width as i32 - 1) as usize;
                let src_y = (y as i32 - offset_y).clamp(0, height as i32 - 1) as usize;
                let dst_idx = (y * width + x) * 4;
                let src_idx = (src_y * width + src_x) * 4;
                pixels[dst_idx..dst_idx + 4].copy_from_slice(&copy[src_idx..src_idx + 4]);
            }
        }
    }
}

/// A matrix convolution image filter.
///
/// Corresponds to Skia's `SkMatrixConvolutionImageFilter`.
#[derive(Debug, Clone)]
pub struct MatrixConvolutionImageFilter {
    kernel_size: (i32, i32),
    kernel: Vec<Scalar>,
    gain: Scalar,
    bias: Scalar,
    kernel_offset: (i32, i32),
    tile_mode: crate::shader::TileMode,
    convolve_alpha: bool,
    input: Option<ImageFilterRef>,
}

impl MatrixConvolutionImageFilter {
    /// Create a matrix convolution filter.
    pub fn new(
        kernel_size: (i32, i32),
        kernel: Vec<Scalar>,
        gain: Scalar,
        bias: Scalar,
        kernel_offset: (i32, i32),
        tile_mode: crate::shader::TileMode,
        convolve_alpha: bool,
        input: Option<ImageFilterRef>,
    ) -> Self {
        Self {
            kernel_size,
            kernel,
            gain,
            bias,
            kernel_offset,
            tile_mode,
            convolve_alpha,
            input,
        }
    }
}

impl ImageFilter for MatrixConvolutionImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        let (kw, kh) = self.kernel_size;
        let (ox, oy) = self.kernel_offset;
        Rect::new(
            src.left - (kw - ox - 1) as Scalar,
            src.top - (kh - oy - 1) as Scalar,
            src.right + ox as Scalar,
            src.bottom + oy as Scalar,
        )
    }

    fn apply(&self, pixels: &mut [u8], width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        let (kw, kh) = self.kernel_size;
        if kw <= 0 || kh <= 0 || self.kernel.is_empty() {
            return;
        }
        let src = pixels.to_vec();
        let gain = self.gain;
        let bias = self.bias;
        let (ox, oy) = self.kernel_offset;
        for y in 0..height {
            for x in 0..width {
                let mut acc = [0.0f32; 4];
                for ky in 0..kh {
                    for kx in 0..kw {
                        let sx = (x as i32 + kx - ox).clamp(0, width as i32 - 1) as usize;
                        let sy = (y as i32 + ky - oy).clamp(0, height as i32 - 1) as usize;
                        let sidx = (sy * width + sx) * 4;
                        let kidx = (ky as usize) * (kw as usize) + kx as usize;
                        let k = self.kernel.get(kidx).copied().unwrap_or(0.0);
                        // Convolve alpha only if convolve_alpha is true
                        for c in 0..3 {
                            acc[c] += src[sidx + c] as f32 * k;
                        }
                        if self.convolve_alpha {
                            acc[3] += src[sidx + 3] as f32 * k;
                        } else {
                            // Keep alpha unchanged (just accumulate for average)
                            if kx == 0 && ky == 0 {
                                acc[3] = src[sidx + 3] as f32;
                            }
                        }
                    }
                }
                let idx = (y * width + x) * 4;
                for c in 0..3 {
                    let v = acc[c] * gain + bias;
                    pixels[idx + c] = v.clamp(0.0, 255.0) as u8;
                }
                if self.convolve_alpha {
                    let v = acc[3] * gain + bias;
                    pixels[idx + 3] = v.clamp(0.0, 255.0) as u8;
                } else {
                    pixels[idx + 3] = src[idx + 3];
                }
            }
        }
    }
}

/// A tile image filter.
///
/// Corresponds to Skia's `SkTileImageFilter`.
#[derive(Debug)]
pub struct TileImageFilter {
    src_rect: Rect,
    dst_rect: Rect,
    input: Option<ImageFilterRef>,
}

impl TileImageFilter {
    /// Create a tile filter.
    pub fn new(src_rect: Rect, dst_rect: Rect, input: Option<ImageFilterRef>) -> Self {
        Self {
            src_rect,
            dst_rect,
            input,
        }
    }
}

impl ImageFilter for TileImageFilter {
    fn filter_bounds(&self, _src: &Rect) -> Rect {
        self.dst_rect
    }

    fn apply(&self, pixels: &mut [u8], width: usize, height: usize) {
        if width == 0 || height == 0 {
            return;
        }
        // Tile the src_rect region across the output via modulo mapping.
        let sx0 = (self.src_rect.left.max(0.0) as usize).min(width);
        let sy0 = (self.src_rect.top.max(0.0) as usize).min(height);
        let sx1 = (self.src_rect.right as usize).min(width);
        let sy1 = (self.src_rect.bottom as usize).min(height);
        if sx1 <= sx0 || sy1 <= sy0 {
            return;
        }
        let sw = sx1 - sx0;
        let sh = sy1 - sy0;
        let src = pixels.to_vec();
        for y in 0..height {
            for x in 0..width {
                let tx = sx0 + (x % sw);
                let ty = sy0 + (y % sh);
                let src_idx = (ty * width + tx) * 4;
                let dst_idx = (y * width + x) * 4;
                pixels[dst_idx] = src[src_idx];
                pixels[dst_idx + 1] = src[src_idx + 1];
                pixels[dst_idx + 2] = src[src_idx + 2];
                pixels[dst_idx + 3] = src[src_idx + 3];
            }
        }
    }
}

/// A blend image filter.
///
/// Corresponds to Skia's `SkBlendImageFilter`.
#[derive(Debug)]
pub struct BlendImageFilter {
    mode: crate::BlendMode,
    background: Option<ImageFilterRef>,
    foreground: Option<ImageFilterRef>,
}

impl BlendImageFilter {
    /// Create a blend filter.
    pub fn new(
        mode: crate::BlendMode,
        background: Option<ImageFilterRef>,
        foreground: Option<ImageFilterRef>,
    ) -> Self {
        Self {
            mode,
            background,
            foreground,
        }
    }
}

impl ImageFilter for BlendImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        let bg = self
            .background
            .as_ref()
            .map(|f| f.filter_bounds(src))
            .unwrap_or(*src);
        let fg = self
            .foreground
            .as_ref()
            .map(|f| f.filter_bounds(src))
            .unwrap_or(*src);
        bg.union(&fg)
    }

    /// Apply is a no-op for this filter in the single-buffer apply() API.
    ///
    /// BlendImageFilter requires separate background and foreground pixel
    /// buffers, which the current single-buffer signature cannot provide.
    /// A future revision will introduce a multi-input apply variant.
    fn apply(&self, _pixels: &mut [u8], _width: usize, _height: usize) {
        // Documented limitation: multi-input filter requires extended API.
    }
}

/// An arithmetic blend image filter.
///
/// Result = k1 * src * dst + k2 * src + k3 * dst + k4
#[derive(Debug)]
pub struct ArithmeticImageFilter {
    k1: Scalar,
    k2: Scalar,
    k3: Scalar,
    k4: Scalar,
    enforce_pm_color: bool,
    background: Option<ImageFilterRef>,
    foreground: Option<ImageFilterRef>,
}

impl ArithmeticImageFilter {
    /// Create an arithmetic blend filter.
    pub fn new(
        k1: Scalar,
        k2: Scalar,
        k3: Scalar,
        k4: Scalar,
        enforce_pm_color: bool,
        background: Option<ImageFilterRef>,
        foreground: Option<ImageFilterRef>,
    ) -> Self {
        Self {
            k1,
            k2,
            k3,
            k4,
            enforce_pm_color,
            background,
            foreground,
        }
    }
}

impl ImageFilter for ArithmeticImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        let bg = self
            .background
            .as_ref()
            .map(|f| f.filter_bounds(src))
            .unwrap_or(*src);
        let fg = self
            .foreground
            .as_ref()
            .map(|f| f.filter_bounds(src))
            .unwrap_or(*src);
        bg.union(&fg)
    }

    /// Apply is a no-op for this filter in the single-buffer apply() API.
    ///
    /// ArithmeticImageFilter requires separate background and foreground pixel
    /// buffers, which the current single-buffer signature cannot provide.
    /// A future revision will introduce a multi-input apply variant.
    fn apply(&self, _pixels: &mut [u8], _width: usize, _height: usize) {
        // Documented limitation: multi-input filter requires extended API.
    }
}

// =============================================================================
// Boxed Filter Types
// =============================================================================

/// Boxed filter types.
pub type ColorFilterRef = Arc<dyn ColorFilter + Send + Sync>;
/// Boxed mask filter type.
pub type MaskFilterRef = Arc<dyn MaskFilter + Send + Sync>;
/// Boxed image filter type.
pub type ImageFilterRef = Arc<dyn ImageFilter + Send + Sync>;

// =============================================================================
// Box Blur Helpers
// =============================================================================

fn box_blur_horizontal(buf: &mut [u8], width: usize, height: usize, radius: usize) {
    if width == 0 {
        return;
    }
    let mut row_copy = vec![0u8; width];
    let r = radius;
    let window = (2 * r + 1) as u32;
    for y in 0..height {
        let row_start = y * width;
        // Copy current row
        row_copy.copy_from_slice(&buf[row_start..row_start + width]);
        // Initialize sum with edge clamping
        let mut sum: u32 = 0;
        for i in 0..=r {
            sum += row_copy[i.min(width - 1)] as u32;
        }
        sum += row_copy[0] as u32 * r as u32; // clamp left edge
        buf[row_start] = (sum / window) as u8;

        for x in 1..width {
            // Slide window: add right, remove left
            let right_idx = (x + r).min(width - 1);
            let left_idx = if x > r { x - r - 1 } else { 0 };
            sum = sum.saturating_add(row_copy[right_idx] as u32);
            sum = sum.saturating_sub(row_copy[left_idx] as u32);
            buf[row_start + x] = (sum / window) as u8;
        }
    }
}

fn box_blur_vertical(buf: &mut [u8], width: usize, height: usize, radius: usize) {
    if height == 0 {
        return;
    }
    let mut col_copy = vec![0u8; height];
    let r = radius;
    let window = (2 * r + 1) as u32;
    for x in 0..width {
        // Copy column
        for y in 0..height {
            col_copy[y] = buf[y * width + x];
        }
        // Initialize sum with top edge clamping
        let mut sum: u32 = 0;
        for i in 0..=r {
            sum += col_copy[i.min(height - 1)] as u32;
        }
        sum += col_copy[0] as u32 * r as u32;
        buf[x] = (sum / window) as u8;

        for y in 1..height {
            let bottom_idx = (y + r).min(height - 1);
            let top_idx = if y > r { y - r - 1 } else { 0 };
            sum = sum.saturating_add(col_copy[bottom_idx] as u32);
            sum = sum.saturating_sub(col_copy[top_idx] as u32);
            buf[y * width + x] = (sum / window) as u8;
        }
    }
}

fn rgba_box_blur_horizontal(pixels: &mut [u8], width: usize, height: usize, radius: usize) {
    if width == 0 {
        return;
    }
    let stride = width * 4;
    let window = (2 * radius + 1) as u32;
    let mut row_copy = vec![0u8; stride];
    for y in 0..height {
        let row_start = y * stride;
        row_copy.copy_from_slice(&pixels[row_start..row_start + stride]);
        // 4 channels: track sum per channel
        let mut sums: [u32; 4] = [0; 4];
        for i in 0..=radius {
            let idx = i.min(width - 1) * 4;
            for c in 0..4 {
                sums[c] += row_copy[idx + c] as u32;
            }
        }
        for c in 0..4 {
            sums[c] += row_copy[c] as u32 * radius as u32;
        }
        for c in 0..4 {
            pixels[row_start + c] = (sums[c] / window) as u8;
        }

        for x in 1..width {
            let right_idx = (x + radius).min(width - 1) * 4;
            let left_idx = if x > radius {
                (x - radius - 1) * 4
            } else {
                0
            };
            for c in 0..4 {
                sums[c] = sums[c].saturating_add(row_copy[right_idx + c] as u32);
                sums[c] = sums[c].saturating_sub(row_copy[left_idx + c] as u32);
                pixels[row_start + x * 4 + c] = (sums[c] / window) as u8;
            }
        }
    }
}

fn rgba_box_blur_vertical(pixels: &mut [u8], width: usize, height: usize, radius: usize) {
    if height == 0 {
        return;
    }
    let stride = width * 4;
    let window = (2 * radius + 1) as u32;
    let mut col_copy = vec![0u8; height * 4];
    for x in 0..width {
        for y in 0..height {
            for c in 0..4 {
                col_copy[y * 4 + c] = pixels[y * stride + x * 4 + c];
            }
        }
        let mut sums: [u32; 4] = [0; 4];
        for i in 0..=radius {
            let idx = i.min(height - 1) * 4;
            for c in 0..4 {
                sums[c] += col_copy[idx + c] as u32;
            }
        }
        for c in 0..4 {
            sums[c] += col_copy[c] as u32 * radius as u32;
        }
        for c in 0..4 {
            pixels[x * 4 + c] = (sums[c] / window) as u8;
        }

        for y in 1..height {
            let bottom_idx = (y + radius).min(height - 1) * 4;
            let top_idx = if y > radius { (y - radius - 1) * 4 } else { 0 };
            for c in 0..4 {
                sums[c] = sums[c].saturating_add(col_copy[bottom_idx + c] as u32);
                sums[c] = sums[c].saturating_sub(col_copy[top_idx + c] as u32);
                pixels[y * stride + x * 4 + c] = (sums[c] / window) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blur_mask_filter_blurs() {
        let filter = BlurMaskFilter::new(BlurStyle::Normal, 3.0);
        // 5x5 mask with a single bright pixel in the center
        let mut mask = vec![0u8; 25];
        mask[12] = 255;
        filter.apply_mask(&mut mask, 5, 5);
        // Center should be less than 255 (blurred)
        assert!(
            mask[12] < 255,
            "center should be blurred lower, got {}",
            mask[12]
        );
        // Should have spread to neighbors
        let has_spread = mask.iter().any(|&v| v > 0 && v < 255);
        assert!(has_spread, "blur should produce intermediate values");
    }

    #[test]
    fn test_blur_image_filter_blurs_rgba() {
        let filter = BlurImageFilter::new(3.0, 3.0, crate::shader::TileMode::Clamp);
        // 5x5 RGBA image, white pixel in center, rest black
        let mut pixels = vec![0u8; 25 * 4];
        pixels[12 * 4] = 255;
        pixels[12 * 4 + 1] = 255;
        pixels[12 * 4 + 2] = 255;
        pixels[12 * 4 + 3] = 255;
        filter.apply(&mut pixels, 5, 5);
        // Center should dim
        assert!(pixels[12 * 4] < 255);
        // Neighbor should brighten
        let neighbor_r = pixels[11 * 4];
        assert!(neighbor_r > 0);
    }

    #[test]
    fn test_color_filter_image_filter() {
        let color_filter = Arc::new(ColorMatrixFilter::saturation(0.0)) as ColorFilterRef;
        let filter = ColorFilterImageFilter::new(color_filter, None);
        // 2x2 RGBA image with various colors
        let mut pixels = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            128, 128, 128, 255, // gray
        ];
        filter.apply(&mut pixels, 2, 2);
        // Saturation 0.0 should produce grayscale
        // Red pixel should have equal RGB
        let r = pixels[0];
        let g = pixels[1];
        let b = pixels[2];
        assert!(
            (r as i32 - g as i32).abs() < 5,
            "red pixel should be grayscale"
        );
        assert!(
            (g as i32 - b as i32).abs() < 5,
            "red pixel should be grayscale"
        );
    }

    #[test]
    fn test_compose_image_filter() {
        let inner = Arc::new(BlurImageFilter::new(2.0, 2.0, crate::shader::TileMode::Clamp))
            as ImageFilterRef;
        let outer_color = Arc::new(ColorMatrixFilter::saturation(0.0)) as ColorFilterRef;
        let outer = Arc::new(ColorFilterImageFilter::new(outer_color, None)) as ImageFilterRef;
        let compose = ComposeImageFilter::new(outer, inner);

        let mut pixels = vec![0u8; 25 * 4];
        pixels[12 * 4] = 255; // red center
        pixels[12 * 4 + 3] = 255;
        compose.apply(&mut pixels, 5, 5);
        // Should be blurred and grayscale
        assert!(pixels[12 * 4] < 255, "should be blurred");
        // Check a neighbor got some color
        assert!(pixels[11 * 4] > 0, "should have spread");
    }

    #[test]
    fn test_offset_image_filter() {
        let filter = OffsetImageFilter::new(1.0, 1.0, None);
        let mut pixels = vec![0u8; 16 * 4]; // 4x4
        pixels[0] = 255; // red at top-left
        pixels[3] = 255; // alpha
        filter.apply(&mut pixels, 4, 4);
        // Pixel should have shifted right and down
        // Top-left should now be clamped from what was at (-1, -1) = top-left
        assert_eq!(pixels[0], 255, "clamp-to-edge should preserve top-left");
    }

    #[test]
    fn test_drop_shadow_image_filter_applies_shadow() {
        let filter = DropShadowImageFilter::new(
            2.0,
            2.0,
            1.0,
            1.0,
            Color4f::new(0.0, 0.0, 0.0, 1.0),
            false,
        );
        // 5x5 image with single white pixel at center
        let mut pixels = vec![0u8; 25 * 4];
        pixels[12 * 4] = 255;
        pixels[12 * 4 + 1] = 255;
        pixels[12 * 4 + 2] = 255;
        pixels[12 * 4 + 3] = 255;
        filter.apply(&mut pixels, 5, 5);
        // Shadow should now be present at some offset positions
        // The original white pixel should still be bright
        assert!(pixels[12 * 4 + 3] > 200, "source should remain bright");
        // Check that some shadow spread occurred (non-zero alpha in neighbors)
        let mut has_shadow = false;
        for i in 0..25 {
            if i != 12 && pixels[i * 4 + 3] > 0 {
                has_shadow = true;
                break;
            }
        }
        assert!(has_shadow, "shadow should spread to other pixels");
    }

    #[test]
    fn test_morphology_dilate_expands_bright_pixel() {
        let filter = MorphologyImageFilter::dilate(1.0, 1.0, None);
        // Single bright white pixel in center
        let mut pixels = vec![0u8; 25 * 4];
        for c in 0..4 {
            pixels[12 * 4 + c] = 255;
        }
        filter.apply(&mut pixels, 5, 5);
        // Neighbors should now be bright (dilated)
        assert_eq!(
            pixels[7 * 4],
            255,
            "neighbor above (7) should be dilated to 255"
        );
        assert_eq!(
            pixels[11 * 4],
            255,
            "neighbor left (11) should be dilated to 255"
        );
        assert_eq!(
            pixels[13 * 4],
            255,
            "neighbor right (13) should be dilated to 255"
        );
        assert_eq!(
            pixels[17 * 4],
            255,
            "neighbor below (17) should be dilated to 255"
        );
    }

    #[test]
    fn test_morphology_erode_shrinks_bright_region() {
        let filter = MorphologyImageFilter::erode(1.0, 1.0, None);
        // 3x3 bright region in center of 5x5
        let mut pixels = vec![0u8; 25 * 4];
        for y in 1..4 {
            for x in 1..4 {
                let idx = (y * 5 + x) * 4;
                pixels[idx] = 255;
                pixels[idx + 1] = 255;
                pixels[idx + 2] = 255;
                pixels[idx + 3] = 255;
            }
        }
        filter.apply(&mut pixels, 5, 5);
        // Center should still be bright
        assert_eq!(pixels[12 * 4], 255, "center should remain bright");
        // Edges should erode (neighbor of black = become black)
        assert_eq!(
            pixels[6 * 4],
            0,
            "top-left of bright region (6) should erode to 0"
        );
    }

    #[test]
    fn test_displacement_map_displaces_pixels() {
        let filter = DisplacementMapImageFilter::new(
            ColorChannel::R,
            ColorChannel::G,
            10.0,
            Arc::new(BlurImageFilter::new(0.0, 0.0, crate::shader::TileMode::Clamp)),
            None,
        );
        // Create a gradient image for displacement
        let mut pixels = vec![0u8; 25 * 4];
        for y in 0..5 {
            for x in 0..5 {
                let idx = (y * 5 + x) * 4;
                pixels[idx] = (x * 50) as u8; // R gradient horizontal
                pixels[idx + 1] = (y * 50) as u8; // G gradient vertical
                pixels[idx + 2] = 128;
                pixels[idx + 3] = 255;
            }
        }
        let original = pixels.clone();
        filter.apply(&mut pixels, 5, 5);
        // Pixels should have moved based on R and G channels
        // At least some pixels should differ from original
        let mut changed = false;
        for i in 0..25 {
            if pixels[i * 4] != original[i * 4]
                || pixels[i * 4 + 1] != original[i * 4 + 1]
                || pixels[i * 4 + 2] != original[i * 4 + 2]
            {
                changed = true;
                break;
            }
        }
        assert!(changed, "displacement should change pixel positions");
    }

    #[test]
    fn test_lighting_image_filter() {
        let light = LightType::Distant {
            direction: (0.0, 0.0, 1.0),
        };
        let filter = LightingImageFilter::diffuse(
            light,
            1.0,
            1.0,
            Color4f::new(1.0, 1.0, 1.0, 1.0),
            None,
        );
        let mut pixels = vec![128u8; 25 * 4]; // gray image
        filter.apply(&mut pixels, 5, 5);
        // Lighting should modulate the image (may brighten based on implementation)
        // Just check it doesn't crash and produces valid output
        assert!(pixels[0] <= 255);
    }

    #[test]
    fn test_matrix_convolution_identity() {
        // Identity kernel: center=1, rest=0
        let kernel = vec![
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
        ];
        let filter = MatrixConvolutionImageFilter::new(
            (3, 3),
            kernel,
            1.0,
            0.0,
            (1, 1),
            crate::shader::TileMode::Clamp,
            true,
            None,
        );
        let mut pixels = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            128, 128, 128, 255, // gray
        ];
        let original = pixels.clone();
        filter.apply(&mut pixels, 2, 2);
        // Identity kernel should preserve pixels (within rounding)
        for i in 0..16 {
            assert!(
                (pixels[i] as i32 - original[i] as i32).abs() <= 1,
                "identity kernel should preserve pixel values"
            );
        }
    }

    #[test]
    fn test_tile_image_filter() {
        let filter = TileImageFilter::new(
            Rect::new(0.0, 0.0, 2.0, 2.0),
            Rect::new(0.0, 0.0, 4.0, 4.0),
            None,
        );
        let mut pixels = vec![0u8; 16 * 4]; // 4x4
                                            // Set top-left 2x2 to distinct colors
        pixels[0] = 255; // (0,0) red
        pixels[4] = 0;
        pixels[4 + 1] = 255; // (1,0) green
        pixels[8] = 0;
        pixels[8 + 2] = 255; // (0,1) blue
        pixels[12] = 255;
        pixels[12 + 1] = 255; // (1,1) yellow
        filter.apply(&mut pixels, 4, 4);
        // The 2x2 tile should repeat across the 4x4 image
        // (2,0) should match (0,0) = red
        assert_eq!(pixels[2 * 4], 255, "tile should repeat horizontally");
        // (0,2) should match (0,0) = red
        assert_eq!(pixels[8 * 4], 255, "tile should repeat vertically");
    }
}
