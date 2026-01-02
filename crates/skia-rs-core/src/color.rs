//! Color types and color space handling.
//!
//! This module provides Skia-compatible color types.

use crate::Scalar;
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};

// =============================================================================
// Color (32-bit ARGB)
// =============================================================================

/// A 32-bit ARGB color.
///
/// Equivalent to Skia's `SkColor`. Format is 0xAARRGGBB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Pod, Zeroable)]
#[repr(transparent)]
pub struct Color(pub u32);

impl Color {
    // Standard colors (matching Skia's SK_Color* constants)
    /// Transparent black.
    pub const TRANSPARENT: Self = Self(0x00000000);
    /// Opaque black.
    pub const BLACK: Self = Self(0xFF000000);
    /// Dark gray.
    pub const DKGRAY: Self = Self(0xFF444444);
    /// Gray.
    pub const GRAY: Self = Self(0xFF888888);
    /// Light gray.
    pub const LTGRAY: Self = Self(0xFFCCCCCC);
    /// Opaque white.
    pub const WHITE: Self = Self(0xFFFFFFFF);
    /// Opaque red.
    pub const RED: Self = Self(0xFFFF0000);
    /// Opaque green.
    pub const GREEN: Self = Self(0xFF00FF00);
    /// Opaque blue.
    pub const BLUE: Self = Self(0xFF0000FF);
    /// Opaque yellow.
    pub const YELLOW: Self = Self(0xFFFFFF00);
    /// Opaque cyan.
    pub const CYAN: Self = Self(0xFF00FFFF);
    /// Opaque magenta.
    pub const MAGENTA: Self = Self(0xFFFF00FF);

    /// Creates a color from alpha and RGB components (0-255 each).
    #[inline]
    pub const fn from_argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self(((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }

    /// Creates an opaque color from RGB components (0-255 each).
    #[inline]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::from_argb(255, r, g, b)
    }

    /// Extracts the alpha component (0-255).
    #[inline]
    pub const fn alpha(&self) -> u8 {
        ((self.0 >> 24) & 0xFF) as u8
    }

    /// Extracts the red component (0-255).
    #[inline]
    pub const fn red(&self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }

    /// Extracts the green component (0-255).
    #[inline]
    pub const fn green(&self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }

    /// Extracts the blue component (0-255).
    #[inline]
    pub const fn blue(&self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// Sets the alpha component.
    #[inline]
    pub const fn with_alpha(&self, a: u8) -> Self {
        Self((self.0 & 0x00FFFFFF) | ((a as u32) << 24))
    }

    /// Converts to Color4f.
    #[inline]
    pub fn to_color4f(&self) -> Color4f {
        Color4f::from_color(*self)
    }

    /// Returns the raw u32 value.
    #[inline]
    pub const fn as_u32(&self) -> u32 {
        self.0
    }
}

impl From<u32> for Color {
    #[inline]
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<Color> for u32 {
    #[inline]
    fn from(color: Color) -> Self {
        color.0
    }
}

/// Premultiply a color (multiply RGB by alpha).
///
/// Returns a color where R, G, B are scaled by alpha/255.
#[inline]
pub fn premultiply_color(color: Color) -> Color {
    let a = color.alpha() as u32;
    if a == 255 {
        return color;
    }
    if a == 0 {
        return Color::TRANSPARENT;
    }

    let r = (color.red() as u32 * a / 255) as u8;
    let g = (color.green() as u32 * a / 255) as u8;
    let b = (color.blue() as u32 * a / 255) as u8;

    Color::from_argb(color.alpha(), r, g, b)
}

/// Unpremultiply a color (divide RGB by alpha).
#[inline]
pub fn unpremultiply_color(color: Color) -> Color {
    let a = color.alpha() as u32;
    if a == 255 || a == 0 {
        return color;
    }

    let r = ((color.red() as u32 * 255 + a / 2) / a).min(255) as u8;
    let g = ((color.green() as u32 * 255 + a / 2) / a).min(255) as u8;
    let b = ((color.blue() as u32 * 255 + a / 2) / a).min(255) as u8;

    Color::from_argb(color.alpha(), r, g, b)
}

// Legacy function aliases for backwards compatibility
/// Creates a color from alpha and RGB components (0-255 each).
#[inline]
pub const fn color_argb(a: u8, r: u8, g: u8, b: u8) -> Color {
    Color::from_argb(a, r, g, b)
}

/// Creates an opaque color from RGB components (0-255 each).
#[inline]
pub const fn color_rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r, g, b)
}

/// Extracts the alpha component from a color.
#[inline]
pub const fn color_get_a(color: Color) -> u8 {
    color.alpha()
}

/// Extracts the red component from a color.
#[inline]
pub const fn color_get_r(color: Color) -> u8 {
    color.red()
}

/// Extracts the green component from a color.
#[inline]
pub const fn color_get_g(color: Color) -> u8 {
    color.green()
}

/// Extracts the blue component from a color.
#[inline]
pub const fn color_get_b(color: Color) -> u8 {
    color.blue()
}

// Legacy color constants
/// Transparent black.
pub const COLOR_TRANSPARENT: Color = Color::TRANSPARENT;
/// Opaque black.
pub const COLOR_BLACK: Color = Color::BLACK;
/// Dark gray.
pub const COLOR_DKGRAY: Color = Color::DKGRAY;
/// Gray.
pub const COLOR_GRAY: Color = Color::GRAY;
/// Light gray.
pub const COLOR_LTGRAY: Color = Color::LTGRAY;
/// Opaque white.
pub const COLOR_WHITE: Color = Color::WHITE;
/// Opaque red.
pub const COLOR_RED: Color = Color::RED;
/// Opaque green.
pub const COLOR_GREEN: Color = Color::GREEN;
/// Opaque blue.
pub const COLOR_BLUE: Color = Color::BLUE;
/// Opaque yellow.
pub const COLOR_YELLOW: Color = Color::YELLOW;
/// Opaque cyan.
pub const COLOR_CYAN: Color = Color::CYAN;
/// Opaque magenta.
pub const COLOR_MAGENTA: Color = Color::MAGENTA;

// =============================================================================
// Color4f (128-bit RGBA float)
// =============================================================================

/// A color with floating-point RGBA components.
///
/// Equivalent to Skia's `SkColor4f`. Components are typically in [0, 1] range
/// but can exceed this for HDR content.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Color4f {
    /// Red component.
    pub r: Scalar,
    /// Green component.
    pub g: Scalar,
    /// Blue component.
    pub b: Scalar,
    /// Alpha component.
    pub a: Scalar,
}

impl Color4f {
    /// Creates a new color.
    #[inline]
    pub const fn new(r: Scalar, g: Scalar, b: Scalar, a: Scalar) -> Self {
        Self { r, g, b, a }
    }

    /// Creates an opaque color.
    #[inline]
    pub const fn from_rgb(r: Scalar, g: Scalar, b: Scalar) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Transparent black.
    #[inline]
    pub const fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Opaque black.
    #[inline]
    pub const fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    /// Opaque white.
    #[inline]
    pub const fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    /// Converts from 32-bit ARGB color.
    #[inline]
    pub fn from_color(color: Color) -> Self {
        Self {
            r: color.red() as Scalar / 255.0,
            g: color.green() as Scalar / 255.0,
            b: color.blue() as Scalar / 255.0,
            a: color.alpha() as Scalar / 255.0,
        }
    }

    /// Converts to 32-bit ARGB color.
    #[inline]
    pub fn to_color(&self) -> Color {
        let a = (self.a.clamp(0.0, 1.0) * 255.0).round() as u8;
        let r = (self.r.clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (self.g.clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (self.b.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color::from_argb(a, r, g, b)
    }

    /// Returns true if the color is opaque (alpha >= 1.0).
    #[inline]
    pub fn is_opaque(&self) -> bool {
        self.a >= 1.0
    }

    /// Returns true if all components are finite.
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.r.is_finite() && self.g.is_finite() && self.b.is_finite() && self.a.is_finite()
    }

    /// Returns a premultiplied version (RGB multiplied by alpha).
    #[inline]
    pub fn premul(&self) -> Self {
        Self {
            r: self.r * self.a,
            g: self.g * self.a,
            b: self.b * self.a,
            a: self.a,
        }
    }

    /// Returns an unpremultiplied version (RGB divided by alpha).
    #[inline]
    pub fn unpremul(&self) -> Self {
        if self.a == 0.0 {
            Self::transparent()
        } else {
            Self {
                r: self.r / self.a,
                g: self.g / self.a,
                b: self.b / self.a,
                a: self.a,
            }
        }
    }

    /// Linearly interpolates between two colors.
    #[inline]
    pub fn lerp(&self, other: &Self, t: Scalar) -> Self {
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// Returns the color as an array [r, g, b, a].
    #[inline]
    pub fn as_array(&self) -> [Scalar; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl From<Color> for Color4f {
    #[inline]
    fn from(color: Color) -> Self {
        Self::from_color(color)
    }
}

impl From<Color4f> for Color {
    #[inline]
    fn from(color: Color4f) -> Self {
        color.to_color()
    }
}

// =============================================================================
// Alpha Type
// =============================================================================

/// Describes how alpha is encoded in pixel data.
///
/// Equivalent to Skia's `SkAlphaType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum AlphaType {
    /// Alpha is unknown or unspecified.
    #[default]
    Unknown = 0,
    /// All pixels are opaque (alpha = 1).
    Opaque = 1,
    /// RGB is premultiplied by alpha.
    Premul = 2,
    /// RGB is not premultiplied (straight alpha).
    Unpremul = 3,
}

impl AlphaType {
    /// Returns true if the alpha type is opaque.
    #[inline]
    pub fn is_opaque(self) -> bool {
        matches!(self, Self::Opaque)
    }
}

// =============================================================================
// Color Type
// =============================================================================

/// Describes the pixel format for color storage.
///
/// Equivalent to Skia's `SkColorType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ColorType {
    /// Unknown or invalid.
    #[default]
    Unknown = 0,
    /// 8-bit alpha only.
    Alpha8 = 1,
    /// 16-bit RGB (5-6-5).
    Rgb565 = 2,
    /// 16-bit ARGB (4-4-4-4).
    Argb4444 = 3,
    /// 32-bit RGBA (8-8-8-8).
    Rgba8888 = 4,
    /// 32-bit RGB (8-8-8-x), alpha ignored.
    Rgb888x = 5,
    /// 32-bit BGRA (8-8-8-8).
    Bgra8888 = 6,
    /// 32-bit RGBA (10-10-10-2).
    Rgba1010102 = 7,
    /// 32-bit BGRA (10-10-10-2).
    Bgra1010102 = 8,
    /// 32-bit RGB (10-10-10-x), alpha ignored.
    Rgb101010x = 9,
    /// 32-bit BGR (10-10-10-x), alpha ignored.
    Bgr101010x = 10,
    /// 8-bit gray.
    Gray8 = 11,
    /// 64-bit RGBA (16-16-16-16 float).
    RgbaF16 = 12,
    /// 64-bit RGBA (16-16-16-16 float), normalized.
    RgbaF16Norm = 13,
    /// 128-bit RGBA (32-32-32-32 float).
    RgbaF32 = 14,
    /// 8-bit palette index (requires color table).
    R8Unorm = 15,
    /// 16-bit alpha only.
    A16Float = 16,
    /// 32-bit red (16 float) + green (16 float).
    R16G16Float = 17,
    /// 16-bit alpha.
    A16Unorm = 18,
    /// 32-bit RG (16-16 unorm).
    R16G16Unorm = 19,
    /// 64-bit RG (16-16 float) per component.
    R16G16B16A16Unorm = 20,
    /// sRGB 32-bit RGBA (8-8-8-8).
    Srgba8888 = 21,
    /// 8-bit red.
    R8Unorm2 = 22,
    /// 24-bit RGB (8-8-8), no alpha.
    Rgb888 = 23,
}

impl ColorType {
    /// Returns the number of bytes per pixel, or 0 if unknown.
    #[inline]
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Unknown => 0,
            Self::Alpha8 | Self::Gray8 | Self::R8Unorm | Self::R8Unorm2 => 1,
            Self::Rgb565 | Self::Argb4444 | Self::A16Float | Self::A16Unorm => 2,
            Self::Rgb888 => 3,
            Self::Rgba8888
            | Self::Rgb888x
            | Self::Bgra8888
            | Self::Rgba1010102
            | Self::Bgra1010102
            | Self::Rgb101010x
            | Self::Bgr101010x
            | Self::Srgba8888
            | Self::R16G16Float
            | Self::R16G16Unorm => 4,
            Self::RgbaF16 | Self::RgbaF16Norm | Self::R16G16B16A16Unorm => 8,
            Self::RgbaF32 => 16,
        }
    }

    /// Returns true if the format has an alpha channel.
    #[inline]
    pub const fn has_alpha(self) -> bool {
        !matches!(
            self,
            Self::Rgb565
                | Self::Rgb888
                | Self::Rgb888x
                | Self::Rgb101010x
                | Self::Bgr101010x
                | Self::Gray8
                | Self::R8Unorm
                | Self::R8Unorm2
        )
    }

    /// Returns the native 32-bit RGBA format for the current platform.
    #[inline]
    pub const fn n32() -> Self {
        // On little-endian (most platforms), BGRA is native.
        // This matches how Skia determines kN32_SkColorType.
        #[cfg(target_endian = "little")]
        {
            Self::Bgra8888
        }
        #[cfg(target_endian = "big")]
        {
            Self::Rgba8888
        }
    }
}

// =============================================================================
// Color Space
// =============================================================================

/// Describes the color space for interpreting colors.
///
/// Equivalent to Skia's `SkColorSpace`.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorSpace {
    /// The transfer function (gamma curve).
    pub transfer_fn: TransferFunction,
    /// The gamut (color primaries and white point).
    pub gamut: ColorGamut,
}

impl Default for ColorSpace {
    fn default() -> Self {
        Self::srgb()
    }
}

impl ColorSpace {
    /// Creates the sRGB color space.
    #[inline]
    pub fn srgb() -> Self {
        Self {
            transfer_fn: TransferFunction::Srgb,
            gamut: ColorGamut::Srgb,
        }
    }

    /// Creates a linear sRGB color space.
    #[inline]
    pub fn srgb_linear() -> Self {
        Self {
            transfer_fn: TransferFunction::Linear,
            gamut: ColorGamut::Srgb,
        }
    }

    /// Creates the Display P3 color space.
    #[inline]
    pub fn display_p3() -> Self {
        Self {
            transfer_fn: TransferFunction::Srgb,
            gamut: ColorGamut::DisplayP3,
        }
    }

    /// Returns true if this is the sRGB color space.
    #[inline]
    pub fn is_srgb(&self) -> bool {
        matches!(
            (&self.transfer_fn, &self.gamut),
            (TransferFunction::Srgb, ColorGamut::Srgb)
        )
    }

    /// Returns true if this has a linear transfer function.
    #[inline]
    pub fn is_linear(&self) -> bool {
        matches!(self.transfer_fn, TransferFunction::Linear)
    }
}

/// Transfer function (gamma curve) for a color space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransferFunction {
    /// Linear (gamma = 1.0).
    Linear,
    /// sRGB transfer function.
    Srgb,
    /// Rec. 2020 / Rec. 709 transfer function.
    Rec2020,
    /// PQ (Perceptual Quantizer) for HDR.
    Pq,
    /// HLG (Hybrid Log-Gamma) for HDR.
    Hlg,
    /// Custom parametric transfer function.
    Parametric {
        /// g parameter.
        g: Scalar,
        /// a parameter.
        a: Scalar,
        /// b parameter.
        b: Scalar,
        /// c parameter.
        c: Scalar,
        /// d parameter.
        d: Scalar,
        /// e parameter.
        e: Scalar,
        /// f parameter.
        f: Scalar,
    },
}

/// Color gamut (primaries and white point).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorGamut {
    /// sRGB / Rec. 709.
    Srgb,
    /// Adobe RGB.
    AdobeRgb,
    /// Display P3.
    DisplayP3,
    /// Rec. 2020.
    Rec2020,
    /// XYZ (device-independent).
    Xyz,
    /// Custom gamut defined by primaries and white point.
    Custom,
}

// =============================================================================
// ICC Profile Support
// =============================================================================

/// An ICC color profile for accurate color management.
///
/// This provides a simplified representation of ICC profiles.
/// For full ICC support, consider using a dedicated ICC library.
#[derive(Debug, Clone)]
pub struct IccProfile {
    /// Profile class (display, input, output, etc.).
    pub profile_class: IccProfileClass,
    /// Color space of the profile.
    pub color_space: IccColorSpace,
    /// Profile connection space.
    pub pcs: IccPcs,
    /// Profile description.
    pub description: String,
    /// Embedded color space.
    pub embedded_color_space: ColorSpace,
    /// Raw ICC profile data (optional).
    raw_data: Option<Vec<u8>>,
}

impl Default for IccProfile {
    fn default() -> Self {
        Self::srgb()
    }
}

impl IccProfile {
    /// Create a standard sRGB profile.
    pub fn srgb() -> Self {
        Self {
            profile_class: IccProfileClass::Display,
            color_space: IccColorSpace::Rgb,
            pcs: IccPcs::Xyz,
            description: "sRGB IEC61966-2.1".to_string(),
            embedded_color_space: ColorSpace::srgb(),
            raw_data: None,
        }
    }

    /// Create a Display P3 profile.
    pub fn display_p3() -> Self {
        Self {
            profile_class: IccProfileClass::Display,
            color_space: IccColorSpace::Rgb,
            pcs: IccPcs::Xyz,
            description: "Display P3".to_string(),
            embedded_color_space: ColorSpace::display_p3(),
            raw_data: None,
        }
    }

    /// Parse an ICC profile from raw bytes.
    ///
    /// This performs basic validation and extracts key information.
    /// Returns `None` if the data is not a valid ICC profile.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        // ICC profile header is 128 bytes
        if data.len() < 128 {
            return None;
        }

        // Check signature "acsp" at bytes 36-39
        if &data[36..40] != b"acsp" {
            return None;
        }

        // Extract profile class (bytes 12-15)
        let profile_class = match &data[12..16] {
            b"scnr" => IccProfileClass::Input,
            b"mntr" => IccProfileClass::Display,
            b"prtr" => IccProfileClass::Output,
            b"link" => IccProfileClass::DeviceLink,
            b"spac" => IccProfileClass::ColorSpace,
            b"abst" => IccProfileClass::Abstract,
            b"nmcl" => IccProfileClass::NamedColor,
            _ => IccProfileClass::Unknown,
        };

        // Extract color space (bytes 16-19)
        let color_space = match &data[16..20] {
            b"RGB " => IccColorSpace::Rgb,
            b"CMYK" => IccColorSpace::Cmyk,
            b"GRAY" => IccColorSpace::Gray,
            b"Lab " => IccColorSpace::Lab,
            b"XYZ " => IccColorSpace::Xyz,
            _ => IccColorSpace::Unknown,
        };

        // Extract PCS (bytes 20-23)
        let pcs = match &data[20..24] {
            b"XYZ " => IccPcs::Xyz,
            b"Lab " => IccPcs::Lab,
            _ => IccPcs::Xyz,
        };

        // For now, default to sRGB for all parsed profiles
        // A full implementation would parse the tag table to get
        // the actual transfer function and primaries
        Some(Self {
            profile_class,
            color_space,
            pcs,
            description: "ICC Profile".to_string(),
            embedded_color_space: ColorSpace::srgb(),
            raw_data: Some(data.to_vec()),
        })
    }

    /// Get the raw ICC profile data if available.
    pub fn raw_data(&self) -> Option<&[u8]> {
        self.raw_data.as_deref()
    }

    /// Get the color space associated with this profile.
    pub fn color_space(&self) -> &ColorSpace {
        &self.embedded_color_space
    }

    /// Check if this is an sRGB profile.
    pub fn is_srgb(&self) -> bool {
        self.embedded_color_space.is_srgb()
    }
}

/// ICC profile class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IccProfileClass {
    /// Input device (scanner, camera).
    Input,
    /// Display device (monitor).
    Display,
    /// Output device (printer).
    Output,
    /// Device link profile.
    DeviceLink,
    /// Color space conversion profile.
    ColorSpace,
    /// Abstract profile.
    Abstract,
    /// Named color profile.
    NamedColor,
    /// Unknown profile class.
    Unknown,
}

/// ICC color space type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IccColorSpace {
    /// RGB color space.
    Rgb,
    /// CMYK color space.
    Cmyk,
    /// Grayscale.
    Gray,
    /// CIE Lab.
    Lab,
    /// CIE XYZ.
    Xyz,
    /// Unknown color space.
    Unknown,
}

/// ICC Profile Connection Space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IccPcs {
    /// CIE XYZ.
    Xyz,
    /// CIE Lab.
    Lab,
}

// =============================================================================
// Color Filter Flags
// =============================================================================

bitflags! {
    /// Flags describing color filter properties.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ColorFilterFlags: u32 {
        /// Filter produces only alpha output.
        const ALPHA_UNCHANGED = 1 << 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_components() {
        let c = Color::from_argb(128, 255, 127, 64);
        assert_eq!(c.alpha(), 128);
        assert_eq!(c.red(), 255);
        assert_eq!(c.green(), 127);
        assert_eq!(c.blue(), 64);
    }

    #[test]
    fn test_color4f_conversion() {
        let c = Color::WHITE;
        let c4 = Color4f::from_color(c);
        assert_eq!(c4.r, 1.0);
        assert_eq!(c4.g, 1.0);
        assert_eq!(c4.b, 1.0);
        assert_eq!(c4.a, 1.0);
        assert_eq!(c4.to_color(), c);
    }

    #[test]
    fn test_color4f_premul() {
        let c = Color4f::new(1.0, 0.5, 0.25, 0.5);
        let premul = c.premul();
        assert_eq!(premul.r, 0.5);
        assert_eq!(premul.g, 0.25);
        assert_eq!(premul.b, 0.125);
        assert_eq!(premul.a, 0.5);
    }

    #[test]
    fn test_color_type_bytes() {
        assert_eq!(ColorType::Rgba8888.bytes_per_pixel(), 4);
        assert_eq!(ColorType::Alpha8.bytes_per_pixel(), 1);
        assert_eq!(ColorType::RgbaF32.bytes_per_pixel(), 16);
    }

    #[test]
    fn test_premultiply() {
        let c = Color::from_argb(128, 200, 100, 50);
        let premul = premultiply_color(c);
        // 200 * 128 / 255 ≈ 100
        assert!(premul.red() > 90 && premul.red() < 110);
    }

    #[test]
    fn test_color_with_alpha() {
        let c = Color::RED;
        let transparent = c.with_alpha(128);
        assert_eq!(transparent.alpha(), 128);
        assert_eq!(transparent.red(), 255);
        assert_eq!(transparent.green(), 0);
        assert_eq!(transparent.blue(), 0);
    }
}
