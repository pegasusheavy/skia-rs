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
            1.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0,
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
            r + s, g, b, 0.0, 0.0,
            r, g + s, b, 0.0, 0.0,
            r, g, b + s, 0.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0,
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
}

/// An image filter.
pub trait ImageFilter: Send + Sync + std::fmt::Debug {
    /// Get the bounds that this filter affects.
    fn filter_bounds(&self, src: &Rect) -> Rect;
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
        Self { sigma_x, sigma_y, tile_mode }
    }
}

impl ImageFilter for BlurImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        // Blur expands bounds by ~3 sigma
        let dx = self.sigma_x * 3.0;
        let dy = self.sigma_y * 3.0;
        Rect::new(
            src.left - dx,
            src.top - dy,
            src.right + dx,
            src.bottom + dy,
        )
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
        Self { dx, dy, sigma_x, sigma_y, color, shadow_only }
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
            *v = if i < min { 0 } else if i > max { 255 } else {
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
        Self { color_filter, input }
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
        Self { src_rect, dst_rect, input }
    }
}

impl ImageFilter for TileImageFilter {
    fn filter_bounds(&self, _src: &Rect) -> Rect {
        self.dst_rect
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
        Self { mode, background, foreground }
    }
}

impl ImageFilter for BlendImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        let bg = self.background.as_ref().map(|f| f.filter_bounds(src)).unwrap_or(*src);
        let fg = self.foreground.as_ref().map(|f| f.filter_bounds(src)).unwrap_or(*src);
        bg.union(&fg)
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
        Self { k1, k2, k3, k4, enforce_pm_color, background, foreground }
    }
}

impl ImageFilter for ArithmeticImageFilter {
    fn filter_bounds(&self, src: &Rect) -> Rect {
        let bg = self.background.as_ref().map(|f| f.filter_bounds(src)).unwrap_or(*src);
        let fg = self.foreground.as_ref().map(|f| f.filter_bounds(src)).unwrap_or(*src);
        bg.union(&fg)
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
