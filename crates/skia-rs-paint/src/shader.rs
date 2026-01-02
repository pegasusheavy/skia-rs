//! Shader types for gradients, images, and patterns.
//!
//! This module provides Skia-compatible shader implementations for:
//! - Solid colors
//! - Linear gradients
//! - Radial gradients
//! - Sweep (angular) gradients
//! - Two-point conical gradients
//! - Image shaders
//! - Blend shaders

use skia_rs_core::{Color4f, Matrix, Point, Rect, Scalar};
use std::sync::Arc;

/// Tile mode for shaders.
///
/// Determines how a shader handles coordinates outside its bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TileMode {
    /// Clamp to edge color.
    #[default]
    Clamp = 0,
    /// Repeat the pattern.
    Repeat,
    /// Mirror the pattern.
    Mirror,
    /// Extend with transparent (coordinates outside bounds are transparent).
    Decal,
}

/// Gradient flags for controlling interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GradientFlags(u32);

impl GradientFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Interpolate colors in premultiplied space.
    pub const INTERPOLATE_PREMUL: Self = Self(1 << 0);
}

/// A shader that generates colors for drawing.
///
/// Corresponds to Skia's `SkShader`.
pub trait Shader: Send + Sync + std::fmt::Debug {
    /// Get the local matrix.
    fn local_matrix(&self) -> Option<&Matrix>;

    /// Check if this shader is opaque.
    fn is_opaque(&self) -> bool;

    /// Returns the kind of shader for debugging.
    fn shader_kind(&self) -> ShaderKind;
}

/// Kind of shader (for debugging/inspection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderKind {
    /// Solid color shader.
    Color,
    /// Linear gradient shader.
    LinearGradient,
    /// Radial gradient shader.
    RadialGradient,
    /// Sweep/angular gradient shader.
    SweepGradient,
    /// Two-point conical gradient shader.
    TwoPointConicalGradient,
    /// Image shader.
    Image,
    /// Blend shader (combines two shaders).
    Blend,
    /// Perlin noise shader.
    PerlinNoise,
    /// Local matrix wrapper shader.
    LocalMatrix,
    /// Composed shader (chain of shaders).
    Compose,
    /// Empty/null shader.
    Empty,
}

/// A solid color shader.
///
/// Corresponds to Skia's `SkColorShader`.
#[derive(Debug, Clone)]
pub struct ColorShader {
    color: Color4f,
}

impl ColorShader {
    /// Create a new solid color shader.
    #[inline]
    pub fn new(color: Color4f) -> Self {
        Self { color }
    }

    /// Get the color.
    #[inline]
    pub fn color(&self) -> Color4f {
        self.color
    }
}

impl Shader for ColorShader {
    fn local_matrix(&self) -> Option<&Matrix> {
        None
    }

    fn is_opaque(&self) -> bool {
        self.color.a >= 1.0
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::Color
    }
}

/// Linear gradient shader.
///
/// Corresponds to Skia's `SkGradientShader::MakeLinear`.
#[derive(Debug, Clone)]
pub struct LinearGradient {
    start: Point,
    end: Point,
    colors: Vec<Color4f>,
    positions: Option<Vec<Scalar>>,
    tile_mode: TileMode,
    flags: GradientFlags,
    local_matrix: Option<Matrix>,
}

impl LinearGradient {
    /// Create a new linear gradient.
    pub fn new(
        start: Point,
        end: Point,
        colors: Vec<Color4f>,
        positions: Option<Vec<Scalar>>,
        tile_mode: TileMode,
    ) -> Self {
        Self {
            start,
            end,
            colors,
            positions,
            tile_mode,
            flags: GradientFlags::NONE,
            local_matrix: None,
        }
    }

    /// Set the local matrix.
    pub fn with_local_matrix(mut self, matrix: Matrix) -> Self {
        self.local_matrix = Some(matrix);
        self
    }

    /// Set gradient flags.
    pub fn with_flags(mut self, flags: GradientFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Get the start point.
    #[inline]
    pub fn start(&self) -> Point {
        self.start
    }

    /// Get the end point.
    #[inline]
    pub fn end(&self) -> Point {
        self.end
    }

    /// Get the colors.
    #[inline]
    pub fn colors(&self) -> &[Color4f] {
        &self.colors
    }

    /// Get the positions.
    #[inline]
    pub fn positions(&self) -> Option<&[Scalar]> {
        self.positions.as_deref()
    }

    /// Get the tile mode.
    #[inline]
    pub fn tile_mode(&self) -> TileMode {
        self.tile_mode
    }
}

impl Shader for LinearGradient {
    fn local_matrix(&self) -> Option<&Matrix> {
        self.local_matrix.as_ref()
    }

    fn is_opaque(&self) -> bool {
        self.colors.iter().all(|c| c.a >= 1.0)
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::LinearGradient
    }
}

/// Radial gradient shader.
///
/// Corresponds to Skia's `SkGradientShader::MakeRadial`.
#[derive(Debug, Clone)]
pub struct RadialGradient {
    center: Point,
    radius: Scalar,
    colors: Vec<Color4f>,
    positions: Option<Vec<Scalar>>,
    tile_mode: TileMode,
    flags: GradientFlags,
    local_matrix: Option<Matrix>,
}

impl RadialGradient {
    /// Create a new radial gradient.
    pub fn new(
        center: Point,
        radius: Scalar,
        colors: Vec<Color4f>,
        positions: Option<Vec<Scalar>>,
        tile_mode: TileMode,
    ) -> Self {
        Self {
            center,
            radius,
            colors,
            positions,
            tile_mode,
            flags: GradientFlags::NONE,
            local_matrix: None,
        }
    }

    /// Set the local matrix.
    pub fn with_local_matrix(mut self, matrix: Matrix) -> Self {
        self.local_matrix = Some(matrix);
        self
    }

    /// Set gradient flags.
    pub fn with_flags(mut self, flags: GradientFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Get the center point.
    #[inline]
    pub fn center(&self) -> Point {
        self.center
    }

    /// Get the radius.
    #[inline]
    pub fn radius(&self) -> Scalar {
        self.radius
    }

    /// Get the colors.
    #[inline]
    pub fn colors(&self) -> &[Color4f] {
        &self.colors
    }

    /// Get the positions.
    #[inline]
    pub fn positions(&self) -> Option<&[Scalar]> {
        self.positions.as_deref()
    }

    /// Get the tile mode.
    #[inline]
    pub fn tile_mode(&self) -> TileMode {
        self.tile_mode
    }
}

impl Shader for RadialGradient {
    fn local_matrix(&self) -> Option<&Matrix> {
        self.local_matrix.as_ref()
    }

    fn is_opaque(&self) -> bool {
        self.colors.iter().all(|c| c.a >= 1.0)
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::RadialGradient
    }
}

/// Sweep (angular) gradient shader.
///
/// Corresponds to Skia's `SkGradientShader::MakeSweep`.
#[derive(Debug, Clone)]
pub struct SweepGradient {
    center: Point,
    start_angle: Scalar,
    end_angle: Scalar,
    colors: Vec<Color4f>,
    positions: Option<Vec<Scalar>>,
    tile_mode: TileMode,
    flags: GradientFlags,
    local_matrix: Option<Matrix>,
}

impl SweepGradient {
    /// Create a new sweep gradient.
    ///
    /// Angles are in degrees, with 0 pointing right and increasing clockwise.
    pub fn new(
        center: Point,
        start_angle: Scalar,
        end_angle: Scalar,
        colors: Vec<Color4f>,
        positions: Option<Vec<Scalar>>,
        tile_mode: TileMode,
    ) -> Self {
        Self {
            center,
            start_angle,
            end_angle,
            colors,
            positions,
            tile_mode,
            flags: GradientFlags::NONE,
            local_matrix: None,
        }
    }

    /// Create a full sweep gradient (0-360 degrees).
    pub fn new_full(center: Point, colors: Vec<Color4f>, positions: Option<Vec<Scalar>>) -> Self {
        Self::new(center, 0.0, 360.0, colors, positions, TileMode::Clamp)
    }

    /// Set the local matrix.
    pub fn with_local_matrix(mut self, matrix: Matrix) -> Self {
        self.local_matrix = Some(matrix);
        self
    }

    /// Set gradient flags.
    pub fn with_flags(mut self, flags: GradientFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Get the center point.
    #[inline]
    pub fn center(&self) -> Point {
        self.center
    }

    /// Get the start angle in degrees.
    #[inline]
    pub fn start_angle(&self) -> Scalar {
        self.start_angle
    }

    /// Get the end angle in degrees.
    #[inline]
    pub fn end_angle(&self) -> Scalar {
        self.end_angle
    }

    /// Get the colors.
    #[inline]
    pub fn colors(&self) -> &[Color4f] {
        &self.colors
    }

    /// Get the positions.
    #[inline]
    pub fn positions(&self) -> Option<&[Scalar]> {
        self.positions.as_deref()
    }
}

impl Shader for SweepGradient {
    fn local_matrix(&self) -> Option<&Matrix> {
        self.local_matrix.as_ref()
    }

    fn is_opaque(&self) -> bool {
        self.colors.iter().all(|c| c.a >= 1.0)
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::SweepGradient
    }
}

/// Two-point conical gradient shader.
///
/// Creates a gradient between two circles defined by center and radius.
/// Corresponds to Skia's `SkGradientShader::MakeTwoPointConical`.
#[derive(Debug, Clone)]
pub struct TwoPointConicalGradient {
    start_center: Point,
    start_radius: Scalar,
    end_center: Point,
    end_radius: Scalar,
    colors: Vec<Color4f>,
    positions: Option<Vec<Scalar>>,
    tile_mode: TileMode,
    flags: GradientFlags,
    local_matrix: Option<Matrix>,
}

impl TwoPointConicalGradient {
    /// Create a new two-point conical gradient.
    pub fn new(
        start_center: Point,
        start_radius: Scalar,
        end_center: Point,
        end_radius: Scalar,
        colors: Vec<Color4f>,
        positions: Option<Vec<Scalar>>,
        tile_mode: TileMode,
    ) -> Self {
        Self {
            start_center,
            start_radius,
            end_center,
            end_radius,
            colors,
            positions,
            tile_mode,
            flags: GradientFlags::NONE,
            local_matrix: None,
        }
    }

    /// Set the local matrix.
    pub fn with_local_matrix(mut self, matrix: Matrix) -> Self {
        self.local_matrix = Some(matrix);
        self
    }

    /// Set gradient flags.
    pub fn with_flags(mut self, flags: GradientFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Get the start center.
    #[inline]
    pub fn start_center(&self) -> Point {
        self.start_center
    }

    /// Get the start radius.
    #[inline]
    pub fn start_radius(&self) -> Scalar {
        self.start_radius
    }

    /// Get the end center.
    #[inline]
    pub fn end_center(&self) -> Point {
        self.end_center
    }

    /// Get the end radius.
    #[inline]
    pub fn end_radius(&self) -> Scalar {
        self.end_radius
    }

    /// Get the colors.
    #[inline]
    pub fn colors(&self) -> &[Color4f] {
        &self.colors
    }

    /// Get the positions.
    #[inline]
    pub fn positions(&self) -> Option<&[Scalar]> {
        self.positions.as_deref()
    }

    /// Get the tile mode.
    #[inline]
    pub fn tile_mode(&self) -> TileMode {
        self.tile_mode
    }
}

impl Shader for TwoPointConicalGradient {
    fn local_matrix(&self) -> Option<&Matrix> {
        self.local_matrix.as_ref()
    }

    fn is_opaque(&self) -> bool {
        self.colors.iter().all(|c| c.a >= 1.0)
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::TwoPointConicalGradient
    }
}

/// Image shader that tiles an image.
///
/// Corresponds to Skia's `SkImageShader`.
#[derive(Debug, Clone)]
pub struct ImageShader {
    /// Image bounds (width, height).
    bounds: Rect,
    /// Tile mode for X axis.
    tile_mode_x: TileMode,
    /// Tile mode for Y axis.
    tile_mode_y: TileMode,
    /// Sampling options.
    sampling: SamplingOptions,
    /// Local matrix.
    local_matrix: Option<Matrix>,
}

/// Sampling options for image shaders.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SamplingOptions {
    /// Filter mode.
    pub filter: FilterMode,
    /// Mipmap mode.
    pub mipmap: MipmapMode,
}

/// Filter mode for image sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FilterMode {
    /// Nearest neighbor sampling.
    #[default]
    Nearest = 0,
    /// Bilinear interpolation.
    Linear,
}

/// Mipmap mode for image sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum MipmapMode {
    /// No mipmapping.
    #[default]
    None = 0,
    /// Nearest mipmap level.
    Nearest,
    /// Linear interpolation between mipmap levels.
    Linear,
}

impl SamplingOptions {
    /// Nearest neighbor sampling (no filtering).
    pub const NEAREST: Self = Self {
        filter: FilterMode::Nearest,
        mipmap: MipmapMode::None,
    };

    /// Bilinear filtering.
    pub const LINEAR: Self = Self {
        filter: FilterMode::Linear,
        mipmap: MipmapMode::None,
    };

    /// Trilinear filtering (linear with linear mipmap).
    pub const TRILINEAR: Self = Self {
        filter: FilterMode::Linear,
        mipmap: MipmapMode::Linear,
    };
}

impl ImageShader {
    /// Create a new image shader.
    pub fn new(
        bounds: Rect,
        tile_mode_x: TileMode,
        tile_mode_y: TileMode,
        sampling: SamplingOptions,
    ) -> Self {
        Self {
            bounds,
            tile_mode_x,
            tile_mode_y,
            sampling,
            local_matrix: None,
        }
    }

    /// Create an image shader with the same tile mode for both axes.
    pub fn with_tile_mode(bounds: Rect, tile_mode: TileMode, sampling: SamplingOptions) -> Self {
        Self::new(bounds, tile_mode, tile_mode, sampling)
    }

    /// Set the local matrix.
    pub fn with_local_matrix(mut self, matrix: Matrix) -> Self {
        self.local_matrix = Some(matrix);
        self
    }

    /// Get the image bounds.
    #[inline]
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Get the X tile mode.
    #[inline]
    pub fn tile_mode_x(&self) -> TileMode {
        self.tile_mode_x
    }

    /// Get the Y tile mode.
    #[inline]
    pub fn tile_mode_y(&self) -> TileMode {
        self.tile_mode_y
    }

    /// Get the sampling options.
    #[inline]
    pub fn sampling(&self) -> SamplingOptions {
        self.sampling
    }
}

impl Shader for ImageShader {
    fn local_matrix(&self) -> Option<&Matrix> {
        self.local_matrix.as_ref()
    }

    fn is_opaque(&self) -> bool {
        // Image shaders are generally not assumed to be opaque
        // without analyzing the actual image data
        false
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::Image
    }
}

/// Blend shader that combines two shaders.
///
/// Corresponds to Skia's `SkShaders::Blend`.
#[derive(Debug)]
pub struct BlendShader {
    blend_mode: crate::BlendMode,
    dst: ShaderRef,
    src: ShaderRef,
}

impl BlendShader {
    /// Create a new blend shader.
    pub fn new(blend_mode: crate::BlendMode, dst: ShaderRef, src: ShaderRef) -> Self {
        Self {
            blend_mode,
            dst,
            src,
        }
    }

    /// Get the blend mode.
    #[inline]
    pub fn blend_mode(&self) -> crate::BlendMode {
        self.blend_mode
    }

    /// Get the destination shader.
    #[inline]
    pub fn dst(&self) -> &ShaderRef {
        &self.dst
    }

    /// Get the source shader.
    #[inline]
    pub fn src(&self) -> &ShaderRef {
        &self.src
    }
}

impl Shader for BlendShader {
    fn local_matrix(&self) -> Option<&Matrix> {
        None
    }

    fn is_opaque(&self) -> bool {
        // Blend shader opacity depends on the blend mode and child shaders
        false
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::Blend
    }
}

/// Perlin noise shader.
///
/// Corresponds to Skia's `SkPerlinNoiseShader`.
#[derive(Debug, Clone)]
pub struct PerlinNoiseShader {
    noise_type: NoiseType,
    base_frequency_x: Scalar,
    base_frequency_y: Scalar,
    num_octaves: i32,
    seed: Scalar,
    tile_size: Option<(i32, i32)>,
}

/// Type of Perlin noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoiseType {
    /// Fractal noise (smoother).
    FractalNoise,
    /// Turbulence (more chaotic).
    Turbulence,
}

impl PerlinNoiseShader {
    /// Create a fractal noise shader.
    pub fn fractal_noise(
        base_frequency_x: Scalar,
        base_frequency_y: Scalar,
        num_octaves: i32,
        seed: Scalar,
    ) -> Self {
        Self {
            noise_type: NoiseType::FractalNoise,
            base_frequency_x,
            base_frequency_y,
            num_octaves,
            seed,
            tile_size: None,
        }
    }

    /// Create a turbulence shader.
    pub fn turbulence(
        base_frequency_x: Scalar,
        base_frequency_y: Scalar,
        num_octaves: i32,
        seed: Scalar,
    ) -> Self {
        Self {
            noise_type: NoiseType::Turbulence,
            base_frequency_x,
            base_frequency_y,
            num_octaves,
            seed,
            tile_size: None,
        }
    }

    /// Set the tile size for seamless tiling.
    pub fn with_tile_size(mut self, width: i32, height: i32) -> Self {
        self.tile_size = Some((width, height));
        self
    }

    /// Get the noise type.
    #[inline]
    pub fn noise_type(&self) -> NoiseType {
        self.noise_type
    }

    /// Get the base frequency X.
    #[inline]
    pub fn base_frequency_x(&self) -> Scalar {
        self.base_frequency_x
    }

    /// Get the base frequency Y.
    #[inline]
    pub fn base_frequency_y(&self) -> Scalar {
        self.base_frequency_y
    }

    /// Get the number of octaves.
    #[inline]
    pub fn num_octaves(&self) -> i32 {
        self.num_octaves
    }

    /// Get the seed.
    #[inline]
    pub fn seed(&self) -> Scalar {
        self.seed
    }
}

impl Shader for PerlinNoiseShader {
    fn local_matrix(&self) -> Option<&Matrix> {
        None
    }

    fn is_opaque(&self) -> bool {
        true
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::PerlinNoise
    }
}

/// A boxed shader type.
pub type ShaderRef = Arc<dyn Shader>;

/// Local matrix wrapper shader.
///
/// Wraps another shader with a local transformation matrix.
/// Corresponds to Skia's `SkLocalMatrixShader`.
#[derive(Debug)]
pub struct LocalMatrixShader {
    inner: ShaderRef,
    matrix: Matrix,
}

impl LocalMatrixShader {
    /// Create a new local matrix shader.
    pub fn new(inner: ShaderRef, matrix: Matrix) -> Self {
        Self { inner, matrix }
    }

    /// Get the inner shader.
    #[inline]
    pub fn inner(&self) -> &ShaderRef {
        &self.inner
    }

    /// Get the matrix.
    #[inline]
    pub fn matrix(&self) -> &Matrix {
        &self.matrix
    }
}

impl Shader for LocalMatrixShader {
    fn local_matrix(&self) -> Option<&Matrix> {
        Some(&self.matrix)
    }

    fn is_opaque(&self) -> bool {
        self.inner.is_opaque()
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::LocalMatrix
    }
}

/// Compose shader that chains two shaders together.
///
/// The compose shader applies the inner shader, then the outer shader.
#[derive(Debug)]
pub struct ComposeShader {
    outer: ShaderRef,
    inner: ShaderRef,
    blend_mode: crate::BlendMode,
}

impl ComposeShader {
    /// Create a new compose shader.
    pub fn new(outer: ShaderRef, inner: ShaderRef, blend_mode: crate::BlendMode) -> Self {
        Self {
            outer,
            inner,
            blend_mode,
        }
    }

    /// Get the outer shader.
    #[inline]
    pub fn outer(&self) -> &ShaderRef {
        &self.outer
    }

    /// Get the inner shader.
    #[inline]
    pub fn inner(&self) -> &ShaderRef {
        &self.inner
    }

    /// Get the blend mode.
    #[inline]
    pub fn blend_mode(&self) -> crate::BlendMode {
        self.blend_mode
    }
}

impl Shader for ComposeShader {
    fn local_matrix(&self) -> Option<&Matrix> {
        None
    }

    fn is_opaque(&self) -> bool {
        self.outer.is_opaque() && self.inner.is_opaque()
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::Compose
    }
}

/// Empty shader that produces transparent pixels.
#[derive(Debug, Clone, Default)]
pub struct EmptyShader;

impl EmptyShader {
    /// Create an empty shader.
    pub fn new() -> Self {
        Self
    }
}

impl Shader for EmptyShader {
    fn local_matrix(&self) -> Option<&Matrix> {
        None
    }

    fn is_opaque(&self) -> bool {
        false
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::Empty
    }
}

/// Convenience functions for creating shaders.
pub mod shaders {
    use super::*;

    /// Create a solid color shader.
    pub fn color(color: Color4f) -> ShaderRef {
        Arc::new(ColorShader::new(color))
    }

    /// Create a linear gradient shader.
    pub fn linear_gradient(
        start: Point,
        end: Point,
        colors: Vec<Color4f>,
        positions: Option<Vec<Scalar>>,
        tile_mode: TileMode,
    ) -> ShaderRef {
        Arc::new(LinearGradient::new(
            start, end, colors, positions, tile_mode,
        ))
    }

    /// Create a radial gradient shader.
    pub fn radial_gradient(
        center: Point,
        radius: Scalar,
        colors: Vec<Color4f>,
        positions: Option<Vec<Scalar>>,
        tile_mode: TileMode,
    ) -> ShaderRef {
        Arc::new(RadialGradient::new(
            center, radius, colors, positions, tile_mode,
        ))
    }

    /// Create a sweep gradient shader.
    pub fn sweep_gradient(
        center: Point,
        start_angle: Scalar,
        end_angle: Scalar,
        colors: Vec<Color4f>,
        positions: Option<Vec<Scalar>>,
        tile_mode: TileMode,
    ) -> ShaderRef {
        Arc::new(SweepGradient::new(
            center,
            start_angle,
            end_angle,
            colors,
            positions,
            tile_mode,
        ))
    }

    /// Create a two-point conical gradient shader.
    pub fn two_point_conical_gradient(
        start_center: Point,
        start_radius: Scalar,
        end_center: Point,
        end_radius: Scalar,
        colors: Vec<Color4f>,
        positions: Option<Vec<Scalar>>,
        tile_mode: TileMode,
    ) -> ShaderRef {
        Arc::new(TwoPointConicalGradient::new(
            start_center,
            start_radius,
            end_center,
            end_radius,
            colors,
            positions,
            tile_mode,
        ))
    }

    /// Create a blend shader.
    pub fn blend(blend_mode: crate::BlendMode, dst: ShaderRef, src: ShaderRef) -> ShaderRef {
        Arc::new(BlendShader::new(blend_mode, dst, src))
    }

    /// Create a fractal noise shader.
    pub fn fractal_noise(
        base_frequency_x: Scalar,
        base_frequency_y: Scalar,
        num_octaves: i32,
        seed: Scalar,
    ) -> ShaderRef {
        Arc::new(PerlinNoiseShader::fractal_noise(
            base_frequency_x,
            base_frequency_y,
            num_octaves,
            seed,
        ))
    }

    /// Create a turbulence shader.
    pub fn turbulence(
        base_frequency_x: Scalar,
        base_frequency_y: Scalar,
        num_octaves: i32,
        seed: Scalar,
    ) -> ShaderRef {
        Arc::new(PerlinNoiseShader::turbulence(
            base_frequency_x,
            base_frequency_y,
            num_octaves,
            seed,
        ))
    }

    /// Wrap a shader with a local matrix transformation.
    pub fn with_local_matrix(shader: ShaderRef, matrix: Matrix) -> ShaderRef {
        Arc::new(LocalMatrixShader::new(shader, matrix))
    }

    /// Compose two shaders together.
    pub fn compose(outer: ShaderRef, inner: ShaderRef, blend_mode: crate::BlendMode) -> ShaderRef {
        Arc::new(ComposeShader::new(outer, inner, blend_mode))
    }

    /// Create an empty (transparent) shader.
    pub fn empty() -> ShaderRef {
        Arc::new(EmptyShader::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_shader() {
        let shader = ColorShader::new(Color4f::new(1.0, 0.0, 0.0, 1.0));
        assert!(shader.is_opaque());
        assert_eq!(shader.shader_kind(), ShaderKind::Color);
    }

    #[test]
    fn test_linear_gradient() {
        let colors = vec![
            Color4f::new(1.0, 0.0, 0.0, 1.0),
            Color4f::new(0.0, 0.0, 1.0, 1.0),
        ];
        let shader = LinearGradient::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            colors,
            None,
            TileMode::Clamp,
        );
        assert!(shader.is_opaque());
        assert_eq!(shader.shader_kind(), ShaderKind::LinearGradient);
    }

    #[test]
    fn test_gradient_with_transparency() {
        let colors = vec![
            Color4f::new(1.0, 0.0, 0.0, 0.5),
            Color4f::new(0.0, 0.0, 1.0, 1.0),
        ];
        let shader = LinearGradient::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            colors,
            None,
            TileMode::Clamp,
        );
        assert!(!shader.is_opaque());
    }

    #[test]
    fn test_shader_convenience_functions() {
        let color = shaders::color(Color4f::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(color.shader_kind(), ShaderKind::Color);

        let linear = shaders::linear_gradient(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            vec![Color4f::new(1.0, 0.0, 0.0, 1.0)],
            None,
            TileMode::Clamp,
        );
        assert_eq!(linear.shader_kind(), ShaderKind::LinearGradient);
    }
}
