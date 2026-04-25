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

use bitflags::bitflags;
use skia_rs_core::{Color4f, Matrix, Point, Rect, Scalar};
use std::sync::Arc;

// =============================================================================
// Shader Kind Discriminants (for serialization)
// =============================================================================

pub(crate) const SHADER_KIND_COLOR: u8 = 1;
pub(crate) const SHADER_KIND_LINEAR_GRADIENT: u8 = 2;
pub(crate) const SHADER_KIND_RADIAL_GRADIENT: u8 = 3;
pub(crate) const SHADER_KIND_SWEEP_GRADIENT: u8 = 4;
pub(crate) const SHADER_KIND_TWO_POINT_CONICAL: u8 = 5;
pub(crate) const SHADER_KIND_IMAGE: u8 = 6;
pub(crate) const SHADER_KIND_PERLIN_NOISE: u8 = 7;
pub(crate) const SHADER_KIND_BLEND: u8 = 8;
pub(crate) const SHADER_KIND_LOCAL_MATRIX: u8 = 9;
pub(crate) const SHADER_KIND_COMPOSE: u8 = 10;
pub(crate) const SHADER_KIND_EMPTY: u8 = 11;

// =============================================================================
// Helper Functions for Serialization
// =============================================================================

/// Serialize a child shader (recursively).
///
/// Format: [1 byte present] [4 bytes length] [data]
/// If the shader cannot be serialized, writes 0 for absent.
fn serialize_child_shader(buf: &mut Vec<u8>, shader: &ShaderRef) {
    if let Some(child_data) = shader.serialize() {
        buf.push(1);
        buf.extend_from_slice(&(child_data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&child_data);
    } else {
        buf.push(0);
    }
}

/// Serialize common gradient data (colors, positions, tile_mode, flags, local_matrix).
fn serialize_gradient_common(
    buf: &mut Vec<u8>,
    colors: &[Color4f],
    positions: Option<&[Scalar]>,
    tile_mode: TileMode,
    flags: GradientFlags,
    local_matrix: Option<&Matrix>,
) {
    // Tile mode (1 byte)
    buf.push(tile_mode as u8);

    // Flags (4 bytes)
    buf.extend_from_slice(&flags.bits().to_le_bytes());

    // Colors count (4 bytes)
    buf.extend_from_slice(&(colors.len() as u32).to_le_bytes());
    for color in colors {
        buf.extend_from_slice(&color.r.to_le_bytes());
        buf.extend_from_slice(&color.g.to_le_bytes());
        buf.extend_from_slice(&color.b.to_le_bytes());
        buf.extend_from_slice(&color.a.to_le_bytes());
    }

    // Positions (1 byte present flag + data if present)
    if let Some(positions) = positions {
        buf.push(1);
        for pos in positions {
            buf.extend_from_slice(&pos.to_le_bytes());
        }
    } else {
        buf.push(0);
    }

    // Local matrix (1 byte present flag + 36 bytes if present)
    if let Some(matrix) = local_matrix {
        buf.push(1);
        for i in 0..9 {
            buf.extend_from_slice(&matrix.values[i].to_le_bytes());
        }
    } else {
        buf.push(0);
    }
}

// =============================================================================
// Helper Functions for Gradient Sampling
// =============================================================================

/// Apply tile mode to a t value (typically 0-1 range).
#[inline]
fn apply_tile_mode(t: Scalar, mode: TileMode) -> Scalar {
    match mode {
        TileMode::Clamp => t.clamp(0.0, 1.0),
        TileMode::Repeat => t.rem_euclid(1.0),
        TileMode::Mirror => {
            let t = t.rem_euclid(2.0);
            if t > 1.0 { 2.0 - t } else { t }
        }
        TileMode::Decal => t, // Colors outside 0-1 handled separately
    }
}

/// Interpolate a gradient color at position t.
#[inline]
fn interpolate_gradient_color(
    colors: &[Color4f],
    positions: Option<&[Scalar]>,
    t: Scalar,
) -> Color4f {
    if colors.is_empty() {
        return Color4f::transparent();
    }
    if colors.len() == 1 {
        return colors[0];
    }

    // Handle out-of-bounds for decal mode
    if t < 0.0 || t > 1.0 {
        return Color4f::transparent();
    }

    // If no explicit positions, use uniform distribution
    if let Some(pos) = positions {
        // Find the segment containing t
        for i in 0..pos.len() - 1 {
            if t >= pos[i] && t <= pos[i + 1] {
                let segment_t = if pos[i + 1] > pos[i] {
                    (t - pos[i]) / (pos[i + 1] - pos[i])
                } else {
                    0.0
                };
                return colors[i].lerp(&colors[i + 1], segment_t);
            }
        }
        // Edge case: return last color
        return colors[colors.len() - 1];
    }

    // Uniform positions
    let scaled = t * (colors.len() - 1) as Scalar;
    let idx = scaled.floor() as usize;
    let frac = scaled - idx as Scalar;

    if idx >= colors.len() - 1 {
        colors[colors.len() - 1]
    } else {
        colors[idx].lerp(&colors[idx + 1], frac)
    }
}

/// Interpolate a gradient color at position t, honoring flags.
#[inline]
fn interpolate_gradient_color_with_flags(
    colors: &[Color4f],
    positions: Option<&[Scalar]>,
    t: Scalar,
    flags: GradientFlags,
) -> Color4f {
    if flags.contains(GradientFlags::INTERPOLATE_PREMUL) {
        // Convert to premul, interpolate, convert back
        if colors.is_empty() {
            return Color4f::transparent();
        }
        if colors.len() == 1 {
            return colors[0];
        }

        let premul: Vec<Color4f> = colors
            .iter()
            .map(|c| Color4f::new(c.r * c.a, c.g * c.a, c.b * c.a, c.a))
            .collect();
        let result = interpolate_gradient_color(&premul, positions, t);
        if result.a > 1e-6 {
            Color4f::new(
                result.r / result.a,
                result.g / result.a,
                result.b / result.a,
                result.a,
            )
        } else {
            Color4f::new(0.0, 0.0, 0.0, 0.0)
        }
    } else {
        interpolate_gradient_color(colors, positions, t)
    }
}

// =============================================================================
// Helper Functions for Image Sampling
// =============================================================================

/// Map a coordinate into the image range [0, size) using the given tile mode.
///
/// Returns NaN if the coordinate is outside the range under Decal mode —
/// callers should treat NaN results as transparent pixels.
fn apply_image_tile(coord: Scalar, size: Scalar, mode: TileMode) -> Scalar {
    if size <= 0.0 {
        return Scalar::NAN;
    }
    match mode {
        TileMode::Clamp => coord.clamp(0.0, size - 1e-4),
        TileMode::Repeat => {
            let m = coord.rem_euclid(size);
            if m < 0.0 {
                m + size
            } else {
                m
            }
        }
        TileMode::Mirror => {
            let n = coord.rem_euclid(2.0 * size);
            if n < size {
                n
            } else {
                2.0 * size - n - 1e-4
            }
        }
        TileMode::Decal => {
            if coord < 0.0 || coord >= size {
                Scalar::NAN
            } else {
                coord
            }
        }
    }
}

/// Read a single RGBA8 pixel at integer coordinates.
///
/// Returns premultiplied Color4f in [0, 1]. For non-RGBA8 color types,
/// the read is simplified (baseline support).
fn read_pixel_rgba8(
    pixels: &[u8],
    info: &skia_rs_core::pixel::ImageInfo,
    x: i32,
    y: i32,
) -> Color4f {
    use skia_rs_core::color::ColorType;
    let bpp = info.bytes_per_pixel();
    let row_bytes = info.min_row_bytes();
    let offset = (y as usize) * row_bytes + (x as usize) * bpp;
    if offset + bpp > pixels.len() {
        return Color4f::new(0.0, 0.0, 0.0, 0.0);
    }

    match info.color_type {
        ColorType::Rgba8888 => {
            let r = pixels[offset] as f32 / 255.0;
            let g = pixels[offset + 1] as f32 / 255.0;
            let b = pixels[offset + 2] as f32 / 255.0;
            let a = pixels[offset + 3] as f32 / 255.0;
            Color4f::new(r, g, b, a)
        }
        ColorType::Bgra8888 => {
            let b = pixels[offset] as f32 / 255.0;
            let g = pixels[offset + 1] as f32 / 255.0;
            let r = pixels[offset + 2] as f32 / 255.0;
            let a = pixels[offset + 3] as f32 / 255.0;
            Color4f::new(r, g, b, a)
        }
        ColorType::Rgb888x => {
            let r = pixels[offset] as f32 / 255.0;
            let g = pixels[offset + 1] as f32 / 255.0;
            let b = pixels[offset + 2] as f32 / 255.0;
            Color4f::new(r, g, b, 1.0)
        }
        ColorType::Alpha8 => {
            let a = pixels[offset] as f32 / 255.0;
            Color4f::new(a, a, a, a) // premultiplied white
        }
        ColorType::Gray8 => {
            let g = pixels[offset] as f32 / 255.0;
            Color4f::new(g, g, g, 1.0)
        }
        _ => Color4f::new(0.0, 0.0, 0.0, 0.0),
    }
}

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

bitflags! {
    /// Flags controlling gradient behavior.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct GradientFlags: u32 {
        /// Interpolate colors in premultiplied space.
        const INTERPOLATE_PREMUL = 1 << 0;
    }
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

    /// Sample the shader at a given point.
    ///
    /// The point is in the shader's local coordinate space (after applying
    /// any local matrix). Returns the color at that point.
    fn sample(&self, x: Scalar, y: Scalar) -> Color4f {
        // Default implementation returns transparent
        let _ = (x, y);
        Color4f::transparent()
    }

    /// Serialize this shader to bytes.
    ///
    /// Returns `None` for shader types that do not support serialization
    /// (e.g., runtime shaders with SkSL source). Built-in shaders override
    /// this to return `Some(bytes)`.
    fn serialize(&self) -> Option<Vec<u8>> {
        None
    }
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

    fn sample(&self, _x: Scalar, _y: Scalar) -> Color4f {
        self.color
    }

    fn serialize(&self) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        buf.push(SHADER_KIND_COLOR);
        buf.extend_from_slice(&self.color.r.to_le_bytes());
        buf.extend_from_slice(&self.color.g.to_le_bytes());
        buf.extend_from_slice(&self.color.b.to_le_bytes());
        buf.extend_from_slice(&self.color.a.to_le_bytes());
        Some(buf)
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
            flags: GradientFlags::empty(),
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

    fn sample(&self, x: Scalar, y: Scalar) -> Color4f {
        // Calculate the projection of the point onto the gradient line
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let len_sq = dx * dx + dy * dy;

        if len_sq < 1e-10 {
            // Degenerate gradient (start == end)
            return self
                .colors
                .first()
                .cloned()
                .unwrap_or(Color4f::transparent());
        }

        // Project point onto gradient line
        let px = x - self.start.x;
        let py = y - self.start.y;
        let mut t = (px * dx + py * dy) / len_sq;

        // Apply tile mode
        t = apply_tile_mode(t, self.tile_mode);

        // Interpolate color
        interpolate_gradient_color_with_flags(
            &self.colors,
            self.positions.as_deref(),
            t,
            self.flags,
        )
    }

    fn serialize(&self) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        buf.push(SHADER_KIND_LINEAR_GRADIENT);

        // Start point (8 bytes)
        buf.extend_from_slice(&self.start.x.to_le_bytes());
        buf.extend_from_slice(&self.start.y.to_le_bytes());

        // End point (8 bytes)
        buf.extend_from_slice(&self.end.x.to_le_bytes());
        buf.extend_from_slice(&self.end.y.to_le_bytes());

        // Common gradient data
        serialize_gradient_common(
            &mut buf,
            &self.colors,
            self.positions.as_deref(),
            self.tile_mode,
            self.flags,
            self.local_matrix.as_ref(),
        );

        Some(buf)
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
            flags: GradientFlags::empty(),
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

    fn sample(&self, x: Scalar, y: Scalar) -> Color4f {
        if self.radius <= 0.0 {
            return self
                .colors
                .first()
                .cloned()
                .unwrap_or(Color4f::transparent());
        }

        // Calculate distance from center
        let dx = x - self.center.x;
        let dy = y - self.center.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let mut t = dist / self.radius;

        // Apply tile mode
        t = apply_tile_mode(t, self.tile_mode);

        // Interpolate color
        interpolate_gradient_color_with_flags(
            &self.colors,
            self.positions.as_deref(),
            t,
            self.flags,
        )
    }

    fn serialize(&self) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        buf.push(SHADER_KIND_RADIAL_GRADIENT);

        // Center point (8 bytes)
        buf.extend_from_slice(&self.center.x.to_le_bytes());
        buf.extend_from_slice(&self.center.y.to_le_bytes());

        // Radius (4 bytes)
        buf.extend_from_slice(&self.radius.to_le_bytes());

        // Common gradient data
        serialize_gradient_common(
            &mut buf,
            &self.colors,
            self.positions.as_deref(),
            self.tile_mode,
            self.flags,
            self.local_matrix.as_ref(),
        );

        Some(buf)
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
            flags: GradientFlags::empty(),
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

    fn sample(&self, x: Scalar, y: Scalar) -> Color4f {
        // Calculate angle from center
        let dx = x - self.center.x;
        let dy = y - self.center.y;

        // atan2 returns [-PI, PI], convert to [0, 360]
        let mut angle = dy.atan2(dx).to_degrees();
        if angle < 0.0 {
            angle += 360.0;
        }

        // Map angle to t value
        let sweep = self.end_angle - self.start_angle;
        let mut t = if sweep.abs() < 1e-10 {
            0.0
        } else {
            (angle - self.start_angle) / sweep
        };

        // Apply tile mode
        t = apply_tile_mode(t, self.tile_mode);

        // Interpolate color
        interpolate_gradient_color_with_flags(
            &self.colors,
            self.positions.as_deref(),
            t,
            self.flags,
        )
    }

    fn serialize(&self) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        buf.push(SHADER_KIND_SWEEP_GRADIENT);

        // Center point (8 bytes)
        buf.extend_from_slice(&self.center.x.to_le_bytes());
        buf.extend_from_slice(&self.center.y.to_le_bytes());

        // Start and end angles (8 bytes)
        buf.extend_from_slice(&self.start_angle.to_le_bytes());
        buf.extend_from_slice(&self.end_angle.to_le_bytes());

        // Common gradient data
        serialize_gradient_common(
            &mut buf,
            &self.colors,
            self.positions.as_deref(),
            self.tile_mode,
            self.flags,
            self.local_matrix.as_ref(),
        );

        Some(buf)
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
            flags: GradientFlags::empty(),
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

    fn sample(&self, x: Scalar, y: Scalar) -> Color4f {
        // Two-point conical gradient: solve quadratic for parameter t.
        // Point P at (x, y). Circles C0 = start_center (radius r0) and
        // C1 = end_center (radius r1).
        //
        // We need t such that P lies on the interpolated circle:
        //   center(t) = C0 + t * (C1 - C0)
        //   radius(t) = r0 + t * (r1 - r0)
        //   |P - center(t)| = radius(t)
        //
        // Let d = C1 - C0, dr = r1 - r0, e = P - C0.
        // Expanding: |e - t*d|^2 = (r0 + t*dr)^2
        //   e.e - 2t(e.d) + t^2(d.d) = r0^2 + 2t*r0*dr + t^2*dr^2
        //   t^2(d.d - dr^2) - 2t(e.d + r0*dr) + (e.e - r0^2) = 0
        //
        // Quadratic: A*t^2 + B*t + C = 0 with
        //   A = d.d - dr^2
        //   B = -2*(e.d + r0*dr)
        //   C = e.e - r0^2
        //
        // Pick the larger root (Skia convention for non-degenerate case).

        let dx = self.end_center.x - self.start_center.x;
        let dy = self.end_center.y - self.start_center.y;
        let dr = self.end_radius - self.start_radius;

        let ex = x - self.start_center.x;
        let ey = y - self.start_center.y;

        let a = dx * dx + dy * dy - dr * dr;
        let b = -2.0 * (ex * dx + ey * dy + self.start_radius * dr);
        let c = ex * ex + ey * ey - self.start_radius * self.start_radius;

        let t_opt = if a.abs() < 1e-7 {
            // Linear case (d.d == dr^2): B*t + C = 0
            if b.abs() < 1e-7 {
                None
            } else {
                Some(-c / b)
            }
        } else {
            let disc = b * b - 4.0 * a * c;
            if disc < 0.0 {
                None
            } else {
                let sqrt_disc = disc.sqrt();
                // Two roots represent two circles the point could lie on.
                // We want the circle closest to the start (smallest valid t).
                let t0 = (-b + sqrt_disc) / (2.0 * a);
                let t1 = (-b - sqrt_disc) / (2.0 * a);
                // Prefer the root whose interpolated radius is non-negative
                let r_at = |t: Scalar| self.start_radius + t * dr;
                let t0_valid = r_at(t0) >= 0.0;
                let t1_valid = r_at(t1) >= 0.0;
                match (t0_valid, t1_valid) {
                    (true, true) => Some(t0.min(t1)),
                    (true, false) => Some(t0),
                    (false, true) => Some(t1),
                    (false, false) => None,
                }
            }
        };

        let t = match t_opt {
            Some(t) => t,
            None => return Color4f::transparent(),
        };

        // Apply tile mode to t, then interpolate colors.
        let t_tiled = apply_tile_mode(t, self.tile_mode);
        interpolate_gradient_color(&self.colors, self.positions.as_deref(), t_tiled)
    }

    fn serialize(&self) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        buf.push(SHADER_KIND_TWO_POINT_CONICAL);

        // Start center and radius (12 bytes)
        buf.extend_from_slice(&self.start_center.x.to_le_bytes());
        buf.extend_from_slice(&self.start_center.y.to_le_bytes());
        buf.extend_from_slice(&self.start_radius.to_le_bytes());

        // End center and radius (12 bytes)
        buf.extend_from_slice(&self.end_center.x.to_le_bytes());
        buf.extend_from_slice(&self.end_center.y.to_le_bytes());
        buf.extend_from_slice(&self.end_radius.to_le_bytes());

        // Common gradient data
        serialize_gradient_common(
            &mut buf,
            &self.colors,
            self.positions.as_deref(),
            self.tile_mode,
            self.flags,
            self.local_matrix.as_ref(),
        );

        Some(buf)
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
    /// Owned pixel data (RGBA8 premultiplied, row-major).
    /// None if the shader was created without pixels (metadata-only).
    pixels: Option<Arc<Vec<u8>>>,
    /// Image info describing pixel layout.
    image_info: Option<skia_rs_core::pixel::ImageInfo>,
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
            pixels: None,
            image_info: None,
        }
    }

    /// Create an image shader with the same tile mode for both axes.
    pub fn with_tile_mode(bounds: Rect, tile_mode: TileMode, sampling: SamplingOptions) -> Self {
        Self::new(bounds, tile_mode, tile_mode, sampling)
    }

    /// Create an ImageShader with owned pixel data.
    ///
    /// Pixels are treated as premultiplied RGBA8 in the sRGB color space.
    /// The image_info should describe the width, height, color type, and
    /// row bytes of the pixel buffer.
    pub fn with_pixels(
        pixels: Arc<Vec<u8>>,
        image_info: skia_rs_core::pixel::ImageInfo,
        tile_mode_x: TileMode,
        tile_mode_y: TileMode,
        sampling: SamplingOptions,
    ) -> Self {
        let bounds = Rect::from_xywh(
            0.0,
            0.0,
            image_info.width() as Scalar,
            image_info.height() as Scalar,
        );
        Self {
            bounds,
            tile_mode_x,
            tile_mode_y,
            sampling,
            local_matrix: None,
            pixels: Some(pixels),
            image_info: Some(image_info),
        }
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

    fn sample(&self, x: Scalar, y: Scalar) -> Color4f {
        let (pixels, info) = match (&self.pixels, &self.image_info) {
            (Some(p), Some(i)) => (p.as_ref(), i),
            _ => return Color4f::new(0.0, 0.0, 0.0, 0.0),
        };

        let width = info.width();
        let height = info.height();
        if width <= 0 || height <= 0 {
            return Color4f::new(0.0, 0.0, 0.0, 0.0);
        }

        // Apply tile mode to map arbitrary (x, y) into [0, width) x [0, height).
        let sx = apply_image_tile(x, width as Scalar, self.tile_mode_x);
        let sy = apply_image_tile(y, height as Scalar, self.tile_mode_y);

        if !sx.is_finite() || !sy.is_finite() || sx < 0.0 || sy < 0.0 {
            return Color4f::new(0.0, 0.0, 0.0, 0.0);
        }

        // Nearest-neighbor sampling: round down to integer pixel.
        let ix = (sx as i32).clamp(0, width - 1);
        let iy = (sy as i32).clamp(0, height - 1);

        read_pixel_rgba8(pixels, info, ix, iy)
    }

    fn is_opaque(&self) -> bool {
        // Image shaders are generally not assumed to be opaque
        // without analyzing the actual image data
        false
    }

    fn shader_kind(&self) -> ShaderKind {
        ShaderKind::Image
    }

    fn serialize(&self) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        buf.push(SHADER_KIND_IMAGE);

        // Bounds (16 bytes)
        buf.extend_from_slice(&self.bounds.left.to_le_bytes());
        buf.extend_from_slice(&self.bounds.top.to_le_bytes());
        buf.extend_from_slice(&self.bounds.right.to_le_bytes());
        buf.extend_from_slice(&self.bounds.bottom.to_le_bytes());

        // Tile modes (2 bytes)
        buf.push(self.tile_mode_x as u8);
        buf.push(self.tile_mode_y as u8);

        // Sampling options (2 bytes: filter + mipmap)
        buf.push(self.sampling.filter as u8);
        buf.push(self.sampling.mipmap as u8);

        // Local matrix (1 byte present flag + 36 bytes if present)
        if let Some(matrix) = &self.local_matrix {
            buf.push(1);
            for i in 0..9 {
                buf.extend_from_slice(&matrix.values[i].to_le_bytes());
            }
        } else {
            buf.push(0);
        }

        // Image info and pixel data (if present)
        if let (Some(pixels), Some(info)) = (&self.pixels, &self.image_info) {
            buf.push(1); // present

            // ImageInfo fields
            buf.extend_from_slice(&(info.width() as u32).to_le_bytes());
            buf.extend_from_slice(&(info.height() as u32).to_le_bytes());
            buf.push(info.color_type as u8);
            buf.push(info.alpha_type as u8);

            // Pixel data length + data
            buf.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
            buf.extend_from_slice(pixels);
        } else {
            buf.push(0); // absent
        }

        Some(buf)
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

    fn sample(&self, x: Scalar, y: Scalar) -> Color4f {
        let src = self.src.sample(x, y);
        let dst = self.dst.sample(x, y);
        self.blend_mode.apply(src, dst)
    }

    fn serialize(&self) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        buf.push(SHADER_KIND_BLEND);

        // Blend mode (1 byte)
        buf.push(self.blend_mode as u8);

        // Src shader (recursive)
        serialize_child_shader(&mut buf, &self.src);

        // Dst shader (recursive)
        serialize_child_shader(&mut buf, &self.dst);

        Some(buf)
    }
}

/// Perlin noise generator based on Ken Perlin's classic algorithm.
///
/// Produces smoothly-varying pseudo-random values in the range roughly [-1, 1]
/// for noise, or [0, 1] for turbulence (which uses abs()). The classic Perlin
/// noise uses a 256-entry permutation table and interpolates gradients at
/// integer lattice points.
struct PerlinNoiseGenerator {
    /// Permutation table seeded from the shader seed.
    perm: [u16; 512],
    /// Gradient lookup table. Each gradient is an 8-valued direction on a
    /// unit square (Skia style: simpler than classic Perlin gradients).
    grad_x: [f32; 256],
    grad_y: [f32; 256],
}

impl PerlinNoiseGenerator {
    fn new(seed: u32) -> Self {
        // Linear-congruential generator for reproducibility.
        let mut state = if seed == 0 { 0xdeadbeef } else { seed };
        let mut rand_u8 = || -> u8 {
            state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            ((state >> 16) & 0xff) as u8
        };

        let mut perm_table = [0u16; 256];
        for i in 0..256 {
            perm_table[i] = i as u16;
        }
        // Fisher-Yates shuffle
        for i in (1..256).rev() {
            let j = (rand_u8() as usize) % (i + 1);
            perm_table.swap(i, j);
        }

        let mut perm = [0u16; 512];
        for i in 0..512 {
            perm[i] = perm_table[i & 255];
        }

        // Gradient table: unit vectors at 8 directions
        let mut grad_x = [0.0f32; 256];
        let mut grad_y = [0.0f32; 256];
        for i in 0..256 {
            let angle = (i as f32) * std::f32::consts::TAU / 256.0;
            grad_x[i] = angle.cos();
            grad_y[i] = angle.sin();
        }

        Self { perm, grad_x, grad_y }
    }

    /// Evaluate classic 2D Perlin noise at (x, y). Returns approximately [-1, 1].
    fn noise_2d(&self, x: f32, y: f32) -> f32 {
        let xi = x.floor() as i32;
        let yi = y.floor() as i32;
        let xf = x - (xi as f32);
        let yf = y - (yi as f32);

        let u = fade(xf);
        let v = fade(yf);

        let xi0 = (xi & 255) as usize;
        let yi0 = (yi & 255) as usize;
        let xi1 = ((xi + 1) & 255) as usize;
        let yi1 = ((yi + 1) & 255) as usize;

        let g00 = self.perm[(self.perm[xi0] as usize + yi0) & 511] as usize;
        let g10 = self.perm[(self.perm[xi1] as usize + yi0) & 511] as usize;
        let g01 = self.perm[(self.perm[xi0] as usize + yi1) & 511] as usize;
        let g11 = self.perm[(self.perm[xi1] as usize + yi1) & 511] as usize;

        let d00 = self.grad_x[g00] * xf + self.grad_y[g00] * yf;
        let d10 = self.grad_x[g10] * (xf - 1.0) + self.grad_y[g10] * yf;
        let d01 = self.grad_x[g01] * xf + self.grad_y[g01] * (yf - 1.0);
        let d11 = self.grad_x[g11] * (xf - 1.0) + self.grad_y[g11] * (yf - 1.0);

        let ix0 = lerp(d00, d10, u);
        let ix1 = lerp(d01, d11, u);
        lerp(ix0, ix1, v)
    }

    /// Fractal (summed octaves with decreasing amplitude).
    fn fractal_2d(&self, x: f32, y: f32, octaves: u32) -> f32 {
        let mut total = 0.0_f32;
        let mut freq = 1.0_f32;
        let mut amp = 1.0_f32;
        let mut max_val = 0.0_f32;
        for _ in 0..octaves {
            total += self.noise_2d(x * freq, y * freq) * amp;
            max_val += amp;
            freq *= 2.0;
            amp *= 0.5;
        }
        if max_val > 0.0 {
            total / max_val
        } else {
            0.0
        }
    }

    /// Turbulence (absolute value of octave sum).
    fn turbulence_2d(&self, x: f32, y: f32, octaves: u32) -> f32 {
        let mut total = 0.0_f32;
        let mut freq = 1.0_f32;
        let mut amp = 1.0_f32;
        let mut max_val = 0.0_f32;
        for _ in 0..octaves {
            total += self.noise_2d(x * freq, y * freq).abs() * amp;
            max_val += amp;
            freq *= 2.0;
            amp *= 0.5;
        }
        if max_val > 0.0 {
            (total / max_val).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

#[inline]
fn fade(t: f32) -> f32 {
    // Ken Perlin's improved fade: 6t^5 - 15t^4 + 10t^3
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
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

    fn sample(&self, x: Scalar, y: Scalar) -> Color4f {
        let generator = PerlinNoiseGenerator::new(self.seed as u32);
        let fx = x * self.base_frequency_x;
        let fy = y * self.base_frequency_y;
        let octaves = self.num_octaves.max(1) as u32;

        let value = match self.noise_type {
            NoiseType::FractalNoise => {
                // Map fractal noise from [-1, 1] to [0, 1]
                (generator.fractal_2d(fx, fy, octaves) * 0.5 + 0.5).clamp(0.0, 1.0)
            }
            NoiseType::Turbulence => generator.turbulence_2d(fx, fy, octaves),
        };

        // Return grayscale with full alpha. Skia uses separate per-channel noise
        // in production, but grayscale is an acceptable baseline.
        Color4f::new(value, value, value, 1.0)
    }

    fn serialize(&self) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        buf.push(SHADER_KIND_PERLIN_NOISE);

        // Noise type (1 byte)
        buf.push(match self.noise_type {
            NoiseType::FractalNoise => 0,
            NoiseType::Turbulence => 1,
        });

        // Base frequencies (8 bytes)
        buf.extend_from_slice(&self.base_frequency_x.to_le_bytes());
        buf.extend_from_slice(&self.base_frequency_y.to_le_bytes());

        // Number of octaves (4 bytes)
        buf.extend_from_slice(&self.num_octaves.to_le_bytes());

        // Seed (4 bytes)
        buf.extend_from_slice(&self.seed.to_le_bytes());

        // Tile size (1 byte present flag + 8 bytes if present)
        if let Some((width, height)) = self.tile_size {
            buf.push(1);
            buf.extend_from_slice(&width.to_le_bytes());
            buf.extend_from_slice(&height.to_le_bytes());
        } else {
            buf.push(0);
        }

        Some(buf)
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

    fn sample(&self, x: Scalar, y: Scalar) -> Color4f {
        // Apply the inverse of the local matrix to map destination coords
        // back into the inner shader's coordinate space. If the matrix is
        // not invertible, fall back to sampling the inner shader at the
        // untransformed coordinates.
        let inv = self.matrix.invert();
        let (sx, sy) = match inv {
            Some(m) => {
                let p = m.map_point(Point::new(x, y));
                (p.x, p.y)
            }
            None => (x, y),
        };
        self.inner.sample(sx, sy)
    }

    fn serialize(&self) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        buf.push(SHADER_KIND_LOCAL_MATRIX);

        // Matrix (36 bytes)
        for i in 0..9 {
            buf.extend_from_slice(&self.matrix.values[i].to_le_bytes());
        }

        // Inner shader (recursive)
        serialize_child_shader(&mut buf, &self.inner);

        Some(buf)
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

    fn sample(&self, x: Scalar, y: Scalar) -> Color4f {
        let dst = self.inner.sample(x, y);
        let src = self.outer.sample(x, y);
        self.blend_mode.apply(src, dst)
    }

    fn serialize(&self) -> Option<Vec<u8>> {
        let mut buf = Vec::new();
        buf.push(SHADER_KIND_COMPOSE);

        // Blend mode (1 byte)
        buf.push(self.blend_mode as u8);

        // Outer shader (recursive)
        serialize_child_shader(&mut buf, &self.outer);

        // Inner shader (recursive)
        serialize_child_shader(&mut buf, &self.inner);

        Some(buf)
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

    fn serialize(&self) -> Option<Vec<u8>> {
        Some(vec![SHADER_KIND_EMPTY])
    }
}

// =============================================================================
// Deserialization
// =============================================================================

/// Deserialize a shader from bytes.
///
/// Returns `None` if the data is invalid or the shader type is unsupported.
pub(crate) fn deserialize_shader(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    if *offset >= bytes.len() {
        return None;
    }
    let kind = bytes[*offset];
    *offset += 1;

    match kind {
        SHADER_KIND_COLOR => deserialize_color_shader(bytes, offset),
        SHADER_KIND_LINEAR_GRADIENT => deserialize_linear_gradient(bytes, offset),
        SHADER_KIND_RADIAL_GRADIENT => deserialize_radial_gradient(bytes, offset),
        SHADER_KIND_SWEEP_GRADIENT => deserialize_sweep_gradient(bytes, offset),
        SHADER_KIND_TWO_POINT_CONICAL => deserialize_two_point_conical(bytes, offset),
        SHADER_KIND_IMAGE => deserialize_image_shader(bytes, offset),
        SHADER_KIND_PERLIN_NOISE => deserialize_perlin_noise(bytes, offset),
        SHADER_KIND_BLEND => deserialize_blend_shader(bytes, offset),
        SHADER_KIND_LOCAL_MATRIX => deserialize_local_matrix_shader(bytes, offset),
        SHADER_KIND_COMPOSE => deserialize_compose_shader(bytes, offset),
        SHADER_KIND_EMPTY => Some(Arc::new(EmptyShader::new())),
        _ => None, // Unknown shader kind
    }
}

/// Helper to read a child shader (recursively).
fn deserialize_child_shader(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    if *offset >= bytes.len() {
        return None;
    }
    let present = bytes[*offset];
    *offset += 1;

    if present == 0 {
        return None;
    }

    // Read length
    if *offset + 4 > bytes.len() {
        return None;
    }
    let len = u32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]) as usize;
    *offset += 4;

    if *offset + len > bytes.len() {
        return None;
    }

    let child_bytes = &bytes[*offset..*offset + len];
    *offset += len;

    let mut child_offset = 0;
    deserialize_shader(child_bytes, &mut child_offset)
}

/// Helper to deserialize common gradient data.
fn deserialize_gradient_common(
    bytes: &[u8],
    offset: &mut usize,
) -> Option<(Vec<Color4f>, Option<Vec<Scalar>>, TileMode, GradientFlags, Option<Matrix>)> {
    // Tile mode (1 byte)
    if *offset >= bytes.len() {
        return None;
    }
    let tile_mode = match bytes[*offset] {
        0 => TileMode::Clamp,
        1 => TileMode::Repeat,
        2 => TileMode::Mirror,
        3 => TileMode::Decal,
        _ => return None,
    };
    *offset += 1;

    // Flags (4 bytes)
    if *offset + 4 > bytes.len() {
        return None;
    }
    let flags = GradientFlags::from_bits_truncate(u32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]));
    *offset += 4;

    // Colors count (4 bytes)
    if *offset + 4 > bytes.len() {
        return None;
    }
    let color_count = u32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]) as usize;
    *offset += 4;

    // Colors (16 bytes each)
    if *offset + color_count * 16 > bytes.len() {
        return None;
    }
    let mut colors = Vec::with_capacity(color_count);
    for _ in 0..color_count {
        let r = f32::from_le_bytes([
            bytes[*offset],
            bytes[*offset + 1],
            bytes[*offset + 2],
            bytes[*offset + 3],
        ]);
        let g = f32::from_le_bytes([
            bytes[*offset + 4],
            bytes[*offset + 5],
            bytes[*offset + 6],
            bytes[*offset + 7],
        ]);
        let b = f32::from_le_bytes([
            bytes[*offset + 8],
            bytes[*offset + 9],
            bytes[*offset + 10],
            bytes[*offset + 11],
        ]);
        let a = f32::from_le_bytes([
            bytes[*offset + 12],
            bytes[*offset + 13],
            bytes[*offset + 14],
            bytes[*offset + 15],
        ]);
        colors.push(Color4f::new(r, g, b, a));
        *offset += 16;
    }

    // Positions (1 byte present flag + data)
    if *offset >= bytes.len() {
        return None;
    }
    let positions_present = bytes[*offset];
    *offset += 1;
    let positions = if positions_present == 1 {
        if *offset + color_count * 4 > bytes.len() {
            return None;
        }
        let mut pos_vec = Vec::with_capacity(color_count);
        for _ in 0..color_count {
            let pos = f32::from_le_bytes([
                bytes[*offset],
                bytes[*offset + 1],
                bytes[*offset + 2],
                bytes[*offset + 3],
            ]);
            pos_vec.push(pos);
            *offset += 4;
        }
        Some(pos_vec)
    } else {
        None
    };

    // Local matrix (1 byte present flag + 36 bytes)
    if *offset >= bytes.len() {
        return None;
    }
    let matrix_present = bytes[*offset];
    *offset += 1;
    let local_matrix = if matrix_present == 1 {
        if *offset + 36 > bytes.len() {
            return None;
        }
        let mut m = Matrix::identity();
        for i in 0..9 {
            m.values[i] = f32::from_le_bytes([
                bytes[*offset],
                bytes[*offset + 1],
                bytes[*offset + 2],
                bytes[*offset + 3],
            ]);
            *offset += 4;
        }
        Some(m)
    } else {
        None
    };

    Some((colors, positions, tile_mode, flags, local_matrix))
}

fn deserialize_color_shader(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    if *offset + 16 > bytes.len() {
        return None;
    }
    let r = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    let g = f32::from_le_bytes([
        bytes[*offset + 4],
        bytes[*offset + 5],
        bytes[*offset + 6],
        bytes[*offset + 7],
    ]);
    let b = f32::from_le_bytes([
        bytes[*offset + 8],
        bytes[*offset + 9],
        bytes[*offset + 10],
        bytes[*offset + 11],
    ]);
    let a = f32::from_le_bytes([
        bytes[*offset + 12],
        bytes[*offset + 13],
        bytes[*offset + 14],
        bytes[*offset + 15],
    ]);
    *offset += 16;
    Some(Arc::new(ColorShader::new(Color4f::new(r, g, b, a))))
}

fn deserialize_linear_gradient(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    // Start point (8 bytes)
    if *offset + 8 > bytes.len() {
        return None;
    }
    let start_x = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    let start_y = f32::from_le_bytes([
        bytes[*offset + 4],
        bytes[*offset + 5],
        bytes[*offset + 6],
        bytes[*offset + 7],
    ]);
    *offset += 8;

    // End point (8 bytes)
    if *offset + 8 > bytes.len() {
        return None;
    }
    let end_x = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    let end_y = f32::from_le_bytes([
        bytes[*offset + 4],
        bytes[*offset + 5],
        bytes[*offset + 6],
        bytes[*offset + 7],
    ]);
    *offset += 8;

    // Common gradient data
    let (colors, positions, tile_mode, flags, local_matrix) =
        deserialize_gradient_common(bytes, offset)?;

    let mut gradient = LinearGradient::new(
        Point::new(start_x, start_y),
        Point::new(end_x, end_y),
        colors,
        positions,
        tile_mode,
    )
    .with_flags(flags);

    if let Some(matrix) = local_matrix {
        gradient = gradient.with_local_matrix(matrix);
    }

    Some(Arc::new(gradient))
}

fn deserialize_radial_gradient(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    // Center point (8 bytes)
    if *offset + 8 > bytes.len() {
        return None;
    }
    let center_x = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    let center_y = f32::from_le_bytes([
        bytes[*offset + 4],
        bytes[*offset + 5],
        bytes[*offset + 6],
        bytes[*offset + 7],
    ]);
    *offset += 8;

    // Radius (4 bytes)
    if *offset + 4 > bytes.len() {
        return None;
    }
    let radius = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    *offset += 4;

    // Common gradient data
    let (colors, positions, tile_mode, flags, local_matrix) =
        deserialize_gradient_common(bytes, offset)?;

    let mut gradient = RadialGradient::new(
        Point::new(center_x, center_y),
        radius,
        colors,
        positions,
        tile_mode,
    )
    .with_flags(flags);

    if let Some(matrix) = local_matrix {
        gradient = gradient.with_local_matrix(matrix);
    }

    Some(Arc::new(gradient))
}

fn deserialize_sweep_gradient(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    // Center point (8 bytes)
    if *offset + 8 > bytes.len() {
        return None;
    }
    let center_x = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    let center_y = f32::from_le_bytes([
        bytes[*offset + 4],
        bytes[*offset + 5],
        bytes[*offset + 6],
        bytes[*offset + 7],
    ]);
    *offset += 8;

    // Start and end angles (8 bytes)
    if *offset + 8 > bytes.len() {
        return None;
    }
    let start_angle = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    let end_angle = f32::from_le_bytes([
        bytes[*offset + 4],
        bytes[*offset + 5],
        bytes[*offset + 6],
        bytes[*offset + 7],
    ]);
    *offset += 8;

    // Common gradient data
    let (colors, positions, tile_mode, flags, local_matrix) =
        deserialize_gradient_common(bytes, offset)?;

    let mut gradient = SweepGradient::new(
        Point::new(center_x, center_y),
        start_angle,
        end_angle,
        colors,
        positions,
        tile_mode,
    )
    .with_flags(flags);

    if let Some(matrix) = local_matrix {
        gradient = gradient.with_local_matrix(matrix);
    }

    Some(Arc::new(gradient))
}

fn deserialize_two_point_conical(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    // Start center and radius (12 bytes)
    if *offset + 12 > bytes.len() {
        return None;
    }
    let start_x = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    let start_y = f32::from_le_bytes([
        bytes[*offset + 4],
        bytes[*offset + 5],
        bytes[*offset + 6],
        bytes[*offset + 7],
    ]);
    let start_radius = f32::from_le_bytes([
        bytes[*offset + 8],
        bytes[*offset + 9],
        bytes[*offset + 10],
        bytes[*offset + 11],
    ]);
    *offset += 12;

    // End center and radius (12 bytes)
    if *offset + 12 > bytes.len() {
        return None;
    }
    let end_x = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    let end_y = f32::from_le_bytes([
        bytes[*offset + 4],
        bytes[*offset + 5],
        bytes[*offset + 6],
        bytes[*offset + 7],
    ]);
    let end_radius = f32::from_le_bytes([
        bytes[*offset + 8],
        bytes[*offset + 9],
        bytes[*offset + 10],
        bytes[*offset + 11],
    ]);
    *offset += 12;

    // Common gradient data
    let (colors, positions, tile_mode, flags, local_matrix) =
        deserialize_gradient_common(bytes, offset)?;

    let mut gradient = TwoPointConicalGradient::new(
        Point::new(start_x, start_y),
        start_radius,
        Point::new(end_x, end_y),
        end_radius,
        colors,
        positions,
        tile_mode,
    )
    .with_flags(flags);

    if let Some(matrix) = local_matrix {
        gradient = gradient.with_local_matrix(matrix);
    }

    Some(Arc::new(gradient))
}

fn deserialize_image_shader(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    // Bounds (16 bytes)
    if *offset + 16 > bytes.len() {
        return None;
    }
    let left = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    let top = f32::from_le_bytes([
        bytes[*offset + 4],
        bytes[*offset + 5],
        bytes[*offset + 6],
        bytes[*offset + 7],
    ]);
    let right = f32::from_le_bytes([
        bytes[*offset + 8],
        bytes[*offset + 9],
        bytes[*offset + 10],
        bytes[*offset + 11],
    ]);
    let bottom = f32::from_le_bytes([
        bytes[*offset + 12],
        bytes[*offset + 13],
        bytes[*offset + 14],
        bytes[*offset + 15],
    ]);
    *offset += 16;

    // Tile modes (2 bytes)
    if *offset + 2 > bytes.len() {
        return None;
    }
    let tile_mode_x = match bytes[*offset] {
        0 => TileMode::Clamp,
        1 => TileMode::Repeat,
        2 => TileMode::Mirror,
        3 => TileMode::Decal,
        _ => return None,
    };
    let tile_mode_y = match bytes[*offset + 1] {
        0 => TileMode::Clamp,
        1 => TileMode::Repeat,
        2 => TileMode::Mirror,
        3 => TileMode::Decal,
        _ => return None,
    };
    *offset += 2;

    // Sampling options (2 bytes)
    if *offset + 2 > bytes.len() {
        return None;
    }
    let filter = match bytes[*offset] {
        0 => FilterMode::Nearest,
        1 => FilterMode::Linear,
        _ => return None,
    };
    let mipmap = match bytes[*offset + 1] {
        0 => MipmapMode::None,
        1 => MipmapMode::Nearest,
        2 => MipmapMode::Linear,
        _ => return None,
    };
    *offset += 2;

    // Local matrix (1 byte present flag + 36 bytes)
    if *offset >= bytes.len() {
        return None;
    }
    let matrix_present = bytes[*offset];
    *offset += 1;
    let local_matrix = if matrix_present == 1 {
        if *offset + 36 > bytes.len() {
            return None;
        }
        let mut m = Matrix::identity();
        for i in 0..9 {
            m.values[i] = f32::from_le_bytes([
                bytes[*offset],
                bytes[*offset + 1],
                bytes[*offset + 2],
                bytes[*offset + 3],
            ]);
            *offset += 4;
        }
        Some(m)
    } else {
        None
    };

    // Image info and pixel data (if present)
    if *offset >= bytes.len() {
        return None;
    }
    let image_present = bytes[*offset];
    *offset += 1;

    let (pixels, image_info) = if image_present == 1 {
        // ImageInfo fields
        if *offset + 6 > bytes.len() {
            return None;
        }
        let width = i32::from_le_bytes([
            bytes[*offset],
            bytes[*offset + 1],
            bytes[*offset + 2],
            bytes[*offset + 3],
        ]);
        let height = i32::from_le_bytes([
            bytes[*offset + 4],
            bytes[*offset + 5],
            bytes[*offset + 6],
            bytes[*offset + 7],
        ]);
        let color_type = match bytes[*offset + 8] {
            0 => skia_rs_core::color::ColorType::Unknown,
            1 => skia_rs_core::color::ColorType::Alpha8,
            2 => skia_rs_core::color::ColorType::Rgb565,
            3 => skia_rs_core::color::ColorType::Argb4444,
            4 => skia_rs_core::color::ColorType::Rgba8888,
            5 => skia_rs_core::color::ColorType::Rgb888x,
            6 => skia_rs_core::color::ColorType::Bgra8888,
            7 => skia_rs_core::color::ColorType::Rgba1010102,
            8 => skia_rs_core::color::ColorType::Bgra1010102,
            9 => skia_rs_core::color::ColorType::Rgb101010x,
            10 => skia_rs_core::color::ColorType::Bgr101010x,
            11 => skia_rs_core::color::ColorType::Gray8,
            12 => skia_rs_core::color::ColorType::RgbaF16,
            13 => skia_rs_core::color::ColorType::RgbaF16Norm,
            14 => skia_rs_core::color::ColorType::RgbaF32,
            15 => skia_rs_core::color::ColorType::R8Unorm,
            16 => skia_rs_core::color::ColorType::A16Float,
            17 => skia_rs_core::color::ColorType::R16G16Float,
            18 => skia_rs_core::color::ColorType::A16Unorm,
            19 => skia_rs_core::color::ColorType::R16G16Unorm,
            20 => skia_rs_core::color::ColorType::R16G16B16A16Unorm,
            21 => skia_rs_core::color::ColorType::Srgba8888,
            22 => skia_rs_core::color::ColorType::R8Unorm2,
            _ => return None,
        };
        let alpha_type = match bytes[*offset + 9] {
            0 => skia_rs_core::color::AlphaType::Unknown,
            1 => skia_rs_core::color::AlphaType::Opaque,
            2 => skia_rs_core::color::AlphaType::Premul,
            3 => skia_rs_core::color::AlphaType::Unpremul,
            _ => return None,
        };
        *offset += 10;

        // Pixel data length + data
        if *offset + 4 > bytes.len() {
            return None;
        }
        let pixel_len = u32::from_le_bytes([
            bytes[*offset],
            bytes[*offset + 1],
            bytes[*offset + 2],
            bytes[*offset + 3],
        ]) as usize;
        *offset += 4;

        if *offset + pixel_len > bytes.len() {
            return None;
        }
        let pixel_data = bytes[*offset..*offset + pixel_len].to_vec();
        *offset += pixel_len;

        let info = skia_rs_core::pixel::ImageInfo::new(
            width,
            height,
            color_type,
            alpha_type,
        ).ok()?;

        (Some(Arc::new(pixel_data)), Some(info))
    } else {
        (None, None)
    };

    Some(Arc::new(ImageShader {
        bounds: Rect::from_xywh(left, top, right - left, bottom - top),
        tile_mode_x,
        tile_mode_y,
        sampling: SamplingOptions { filter, mipmap },
        local_matrix,
        pixels,
        image_info,
    }))
}

fn deserialize_perlin_noise(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    // Noise type (1 byte)
    if *offset >= bytes.len() {
        return None;
    }
    let noise_type = match bytes[*offset] {
        0 => NoiseType::FractalNoise,
        1 => NoiseType::Turbulence,
        _ => return None,
    };
    *offset += 1;

    // Base frequencies (8 bytes)
    if *offset + 8 > bytes.len() {
        return None;
    }
    let base_frequency_x = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    let base_frequency_y = f32::from_le_bytes([
        bytes[*offset + 4],
        bytes[*offset + 5],
        bytes[*offset + 6],
        bytes[*offset + 7],
    ]);
    *offset += 8;

    // Number of octaves (4 bytes)
    if *offset + 4 > bytes.len() {
        return None;
    }
    let num_octaves = i32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    *offset += 4;

    // Seed (4 bytes)
    if *offset + 4 > bytes.len() {
        return None;
    }
    let seed = f32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]);
    *offset += 4;

    // Tile size (1 byte present flag + 8 bytes)
    if *offset >= bytes.len() {
        return None;
    }
    let tile_present = bytes[*offset];
    *offset += 1;
    let tile_size = if tile_present == 1 {
        if *offset + 8 > bytes.len() {
            return None;
        }
        let width = i32::from_le_bytes([
            bytes[*offset],
            bytes[*offset + 1],
            bytes[*offset + 2],
            bytes[*offset + 3],
        ]);
        let height = i32::from_le_bytes([
            bytes[*offset + 4],
            bytes[*offset + 5],
            bytes[*offset + 6],
            bytes[*offset + 7],
        ]);
        *offset += 8;
        Some((width, height))
    } else {
        None
    };

    let mut shader = PerlinNoiseShader {
        noise_type,
        base_frequency_x,
        base_frequency_y,
        num_octaves,
        seed,
        tile_size: None,
    };

    if let Some((w, h)) = tile_size {
        shader = shader.with_tile_size(w, h);
    }

    Some(Arc::new(shader))
}

fn deserialize_blend_shader(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    // Blend mode (1 byte)
    if *offset >= bytes.len() {
        return None;
    }
    let blend_mode = crate::BlendMode::from_u8(bytes[*offset])?;
    *offset += 1;

    // Src shader (recursive)
    let src = deserialize_child_shader(bytes, offset)?;

    // Dst shader (recursive)
    let dst = deserialize_child_shader(bytes, offset)?;

    Some(Arc::new(BlendShader::new(blend_mode, dst, src)))
}

fn deserialize_local_matrix_shader(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    // Matrix (36 bytes)
    if *offset + 36 > bytes.len() {
        return None;
    }
    let mut matrix = Matrix::identity();
    for i in 0..9 {
        matrix.values[i] = f32::from_le_bytes([
            bytes[*offset],
            bytes[*offset + 1],
            bytes[*offset + 2],
            bytes[*offset + 3],
        ]);
        *offset += 4;
    }

    // Inner shader (recursive)
    let inner = deserialize_child_shader(bytes, offset)?;

    Some(Arc::new(LocalMatrixShader::new(inner, matrix)))
}

fn deserialize_compose_shader(bytes: &[u8], offset: &mut usize) -> Option<ShaderRef> {
    // Blend mode (1 byte)
    if *offset >= bytes.len() {
        return None;
    }
    let blend_mode = crate::BlendMode::from_u8(bytes[*offset])?;
    *offset += 1;

    // Outer shader (recursive)
    let outer = deserialize_child_shader(bytes, offset)?;

    // Inner shader (recursive)
    let inner = deserialize_child_shader(bytes, offset)?;

    Some(Arc::new(ComposeShader::new(outer, inner, blend_mode)))
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

    #[test]
    fn test_local_matrix_shader_translates() {
        let base: Arc<dyn Shader> = Arc::new(ColorShader::new(Color4f::new(1.0, 0.0, 0.0, 1.0)));
        // LocalMatrixShader with translate(50, 0) should still return red
        // regardless of query point (ColorShader ignores coords).
        let matrix = Matrix::translate(50.0, 0.0);
        let wrapped = LocalMatrixShader::new(base, matrix);
        let c = wrapped.sample(0.0, 0.0);
        assert!((c.r - 1.0).abs() < 1e-5);
        assert!((c.g - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_blend_shader_uses_blend_mode() {
        let red: Arc<dyn Shader> = Arc::new(ColorShader::new(Color4f::new(1.0, 0.0, 0.0, 1.0)));
        let green: Arc<dyn Shader> = Arc::new(ColorShader::new(Color4f::new(0.0, 1.0, 0.0, 1.0)));
        let blend = BlendShader::new(crate::BlendMode::Src, red.clone(), green.clone());
        let c = blend.sample(0.0, 0.0);
        // Src mode returns src; with BlendShader(mode, dst, src), src is green
        assert!((c.g - 1.0).abs() < 1e-5, "Expected green component = 1.0");
        assert!((c.r - 0.0).abs() < 1e-5, "Expected red component = 0.0");
    }

    #[test]
    fn test_compose_shader_blends_children() {
        let solid: Arc<dyn Shader> = Arc::new(ColorShader::new(Color4f::new(1.0, 1.0, 1.0, 1.0)));
        let half: Arc<dyn Shader> = Arc::new(ColorShader::new(Color4f::new(0.5, 0.5, 0.5, 1.0)));
        // ComposeShader(outer, inner, mode) blends them
        let compose = ComposeShader::new(solid.clone(), half.clone(), crate::BlendMode::SrcOver);
        let c = compose.sample(0.0, 0.0);
        assert!(c.a > 0.5, "ComposeShader should not produce transparent output");
    }

    #[test]
    fn test_two_point_conical_gradient_interpolates() {
        // Two circles: inner at origin (r=10), outer at (100, 0) (r=30).
        // Interpolate red -> blue.
        let gradient = TwoPointConicalGradient::new(
            Point::new(0.0, 0.0),
            10.0,
            Point::new(100.0, 0.0),
            30.0,
            vec![
                Color4f::new(1.0, 0.0, 0.0, 1.0), // red at t=0 (inner circle)
                Color4f::new(0.0, 0.0, 1.0, 1.0), // blue at t=1 (outer circle)
            ],
            None, // positions default
            TileMode::Clamp,
        );

        // A point on the inner circle (at start_center + start_radius in x direction)
        // should be red (t=0).
        let c_inner = gradient.sample(10.0, 0.0);
        assert!(c_inner.r > 0.9, "inner circle point should be red, got r={}", c_inner.r);

        // A point on the outer circle (at end_center + end_radius in x direction)
        // should be blue (t=1).
        let c_outer = gradient.sample(130.0, 0.0);
        assert!(c_outer.b > 0.9, "outer circle point should be blue, got b={}", c_outer.b);

        // Midpoint should show a mix (purple-ish)
        let c_mid = gradient.sample(50.0, 0.0);
        assert!(c_mid.r > 0.1 && c_mid.b > 0.1,
                "midpoint should mix red and blue, got r={} b={}", c_mid.r, c_mid.b);
    }

    #[test]
    fn test_two_point_conical_gradient_outside_returns_clamped() {
        // Outside both circles with TileMode::Clamp should clamp to edge color
        let gradient = TwoPointConicalGradient::new(
            Point::new(0.0, 0.0),
            10.0,
            Point::new(50.0, 0.0),
            20.0,
            vec![
                Color4f::new(1.0, 0.0, 0.0, 1.0),
                Color4f::new(0.0, 1.0, 0.0, 1.0),
            ],
            None,
            TileMode::Clamp,
        );

        // Far off-axis
        let c = gradient.sample(200.0, 200.0);
        // Should be bounded, not NaN
        assert!(c.r.is_finite() && c.g.is_finite() && c.b.is_finite() && c.a.is_finite());
    }

    #[test]
    fn test_perlin_fractal_noise_in_range() {
        let shader = PerlinNoiseShader::fractal_noise(0.1, 0.1, 4, 42.0);
        for x in 0..10 {
            for y in 0..10 {
                let c = shader.sample(x as f32, y as f32);
                assert!(c.r >= 0.0 && c.r <= 1.0, "r out of range: {}", c.r);
                assert!(c.a >= 0.99, "alpha should be 1: {}", c.a);
            }
        }
    }

    #[test]
    fn test_perlin_turbulence_in_range() {
        let shader = PerlinNoiseShader::turbulence(0.1, 0.1, 4, 42.0);
        for x in 0..10 {
            for y in 0..10 {
                let c = shader.sample(x as f32, y as f32);
                assert!(c.r >= 0.0 && c.r <= 1.0);
            }
        }
    }

    #[test]
    fn test_perlin_different_seeds_produce_different_noise() {
        let a = PerlinNoiseShader::fractal_noise(0.5, 0.5, 2, 1.0);
        let b = PerlinNoiseShader::fractal_noise(0.5, 0.5, 2, 999.0);
        // At least one of a few sample points should differ
        let mut differ = false;
        for k in 0..10 {
            let ra = a.sample(k as f32, k as f32).r;
            let rb = b.sample(k as f32, k as f32).r;
            if (ra - rb).abs() > 1e-4 {
                differ = true;
                break;
            }
        }
        assert!(differ, "Different seeds should produce different output");
    }

    #[test]
    fn test_perlin_deterministic_per_seed() {
        let shader = PerlinNoiseShader::fractal_noise(0.3, 0.3, 3, 12345.0);
        let c1 = shader.sample(10.0, 10.0);
        let c2 = shader.sample(10.0, 10.0);
        assert!((c1.r - c2.r).abs() < 1e-5);
    }

    #[test]
    fn test_image_shader_samples_rgba8_pixel() {
        use skia_rs_core::color::AlphaType;
        use skia_rs_core::color::ColorType;
        use skia_rs_core::pixel::ImageInfo;
        // 2x2 image: red, green, blue, white
        let pixels: Arc<Vec<u8>> = Arc::new(vec![
            255, 0, 0, 255,    // (0,0) red
            0, 255, 0, 255,    // (1,0) green
            0, 0, 255, 255,    // (0,1) blue
            255, 255, 255, 255, // (1,1) white
        ]);
        let info = ImageInfo::new(2, 2, ColorType::Rgba8888, AlphaType::Premul).unwrap();
        let shader = ImageShader::with_pixels(
            pixels,
            info,
            TileMode::Clamp,
            TileMode::Clamp,
            SamplingOptions::default(),
        );

        // Sample exact pixel positions
        let c_00 = shader.sample(0.5, 0.5);
        assert!((c_00.r - 1.0).abs() < 1e-3, "expected red at (0.5, 0.5), got {:?}", c_00);

        let c_11 = shader.sample(1.5, 1.5);
        assert!((c_11.r - 1.0).abs() < 1e-3 && (c_11.g - 1.0).abs() < 1e-3, "expected white at (1.5, 1.5), got {:?}", c_11);
    }

    #[test]
    fn test_image_shader_without_pixels_returns_transparent() {
        // Using the existing constructor (no pixels), sample should return transparent
        let shader = ImageShader::new(
            Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
            TileMode::Clamp,
            TileMode::Clamp,
            SamplingOptions::default(),
        );
        let c = shader.sample(50.0, 50.0);
        assert_eq!(c.a, 0.0, "expected transparent for shader without pixels");
    }

    #[test]
    fn test_image_shader_clamp_tile_mode() {
        use skia_rs_core::color::AlphaType;
        use skia_rs_core::color::ColorType;
        use skia_rs_core::pixel::ImageInfo;
        let pixels: Arc<Vec<u8>> = Arc::new(vec![
            0, 0, 0, 255,
            255, 255, 255, 255,
        ]);
        let info = ImageInfo::new(2, 1, ColorType::Rgba8888, AlphaType::Premul).unwrap();
        let shader = ImageShader::with_pixels(
            pixels,
            info,
            TileMode::Clamp,
            TileMode::Clamp,
            SamplingOptions::default(),
        );

        // Sampling far outside should clamp to nearest edge pixel
        let c_right = shader.sample(100.0, 0.5);
        assert!((c_right.r - 1.0).abs() < 1e-3, "clamp should return white from right edge, got {:?}", c_right);
    }

    #[test]
    fn test_image_shader_repeat_tile_mode() {
        use skia_rs_core::color::AlphaType;
        use skia_rs_core::color::ColorType;
        use skia_rs_core::pixel::ImageInfo;
        let pixels: Arc<Vec<u8>> = Arc::new(vec![
            255, 0, 0, 255,    // red
            0, 255, 0, 255,    // green
        ]);
        let info = ImageInfo::new(2, 1, ColorType::Rgba8888, AlphaType::Premul).unwrap();
        let shader = ImageShader::with_pixels(
            pixels,
            info,
            TileMode::Repeat,
            TileMode::Repeat,
            SamplingOptions::default(),
        );

        // Sampling beyond width should wrap
        let c = shader.sample(2.5, 0.5); // Should wrap to 0.5
        assert!((c.r - 1.0).abs() < 1e-3, "repeat should wrap to red pixel, got {:?}", c);
    }

    #[test]
    fn test_image_shader_decal_tile_mode() {
        use skia_rs_core::color::AlphaType;
        use skia_rs_core::color::ColorType;
        use skia_rs_core::pixel::ImageInfo;
        let pixels: Arc<Vec<u8>> = Arc::new(vec![
            255, 0, 0, 255,
        ]);
        let info = ImageInfo::new(1, 1, ColorType::Rgba8888, AlphaType::Premul).unwrap();
        let shader = ImageShader::with_pixels(
            pixels,
            info,
            TileMode::Decal,
            TileMode::Decal,
            SamplingOptions::default(),
        );

        // Inside bounds should return color
        let c_in = shader.sample(0.5, 0.5);
        assert!((c_in.r - 1.0).abs() < 1e-3, "inside should return red");

        // Outside bounds should return transparent
        let c_out = shader.sample(2.0, 0.5);
        assert_eq!(c_out.a, 0.0, "decal outside bounds should be transparent");
    }

    #[test]
    fn test_gradient_flags_interpolate_premul() {
        // Create a gradient from opaque red to transparent
        let colors = vec![
            Color4f::new(1.0, 0.0, 0.0, 1.0),
            Color4f::new(0.0, 0.0, 0.0, 0.0),
        ];

        // Without premul flag
        let grad_normal = LinearGradient::new(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            colors.clone(),
            None,
            TileMode::Clamp,
        );

        // With premul flag
        let grad_premul = LinearGradient::new(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            colors.clone(),
            None,
            TileMode::Clamp,
        )
        .with_flags(GradientFlags::INTERPOLATE_PREMUL);

        // Sample at midpoint (t=0.5)
        let c_normal = grad_normal.sample(5.0, 0.0);
        let c_premul = grad_premul.sample(5.0, 0.0);

        // Without premul, RGB lerps directly: (1.0 + 0.0)/2 = 0.5
        // With premul, RGB*alpha lerps: (1.0*1.0 + 0.0*0.0)/2 = 0.5, then unpremul
        // The difference should be visible
        assert!(
            c_normal.r > 0.4 && c_normal.r < 0.6,
            "normal interpolation at midpoint"
        );
        // With premul, the color at midpoint should be darker (more transparent)
        assert!(c_premul.a < 1.0, "premul should affect alpha");
    }
}
