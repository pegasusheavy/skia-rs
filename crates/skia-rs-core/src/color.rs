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

    /// Parse a CSS color string (CSS Color Level 3 & 4).
    ///
    /// Supports:
    /// - Hex: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`
    /// - Functional: `rgb(r, g, b)`, `rgba(r, g, b, a)`,
    ///   `hsl(h, s%, l%)`, `hsla(h, s%, l%, a)`
    /// - Named: all CSS Color Level 3 names (transparent, red, aliceblue, …)
    /// - CSS Color Level 4: `lab()`, `lch()`, `oklab()`, `oklch()`, `hwb()`,
    ///   `color(srgb|srgb-linear|display-p3 ...)`
    ///
    /// Level 4 colors are converted to sRGB; out-of-gamut values are clamped.
    /// Returns `None` if the string does not match any supported form.
    pub fn from_css(s: &str) -> Option<Self> {
        let s = s.trim();

        if let Some(hex) = s.strip_prefix('#') {
            return Self::parse_hex_color(hex);
        }

        if let Some(inner) = s.strip_prefix("rgba(").and_then(|r| r.strip_suffix(')')) {
            return Self::parse_rgba_inner(inner);
        }
        if let Some(inner) = s.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
            return Self::parse_rgb_inner(inner);
        }
        if let Some(inner) = s.strip_prefix("hsla(").and_then(|r| r.strip_suffix(')')) {
            return Self::parse_hsl_inner(inner, true);
        }
        if let Some(inner) = s.strip_prefix("hsl(").and_then(|r| r.strip_suffix(')')) {
            return Self::parse_hsl_inner(inner, false);
        }

        // CSS Color Level 4
        if let Some(inner) = s.strip_prefix("lab(").and_then(|r| r.strip_suffix(')')) {
            return Self::parse_lab(inner);
        }
        if let Some(inner) = s.strip_prefix("lch(").and_then(|r| r.strip_suffix(')')) {
            return Self::parse_lch(inner);
        }
        if let Some(inner) = s.strip_prefix("oklab(").and_then(|r| r.strip_suffix(')')) {
            return Self::parse_oklab(inner);
        }
        if let Some(inner) = s.strip_prefix("oklch(").and_then(|r| r.strip_suffix(')')) {
            return Self::parse_oklch(inner);
        }
        if let Some(inner) = s.strip_prefix("hwb(").and_then(|r| r.strip_suffix(')')) {
            return Self::parse_hwb(inner);
        }
        if let Some(inner) = s.strip_prefix("color(").and_then(|r| r.strip_suffix(')')) {
            return Self::parse_color_function(inner);
        }

        Self::named_color(s)
    }

    fn parse_hex_color(hex: &str) -> Option<Self> {
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                Some(Self::from_rgb(r, g, b))
            }
            4 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                let a = u8::from_str_radix(&hex[3..4], 16).ok()? * 17;
                Some(Self::from_argb(a, r, g, b))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::from_rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::from_argb(a, r, g, b))
            }
            _ => None,
        }
    }

    fn parse_rgb_inner(inner: &str) -> Option<Self> {
        let mut it = inner.split(',');
        let r: u8 = it.next()?.trim().parse().ok()?;
        let g: u8 = it.next()?.trim().parse().ok()?;
        let b: u8 = it.next()?.trim().parse().ok()?;
        if it.next().is_some() {
            return None;
        }
        Some(Self::from_rgb(r, g, b))
    }

    fn parse_rgba_inner(inner: &str) -> Option<Self> {
        let mut it = inner.split(',');
        let r: u8 = it.next()?.trim().parse().ok()?;
        let g: u8 = it.next()?.trim().parse().ok()?;
        let b: u8 = it.next()?.trim().parse().ok()?;
        let a: f32 = it.next()?.trim().parse().ok()?;
        if it.next().is_some() {
            return None;
        }
        Some(Self::from_argb(
            (a * 255.0).round().clamp(0.0, 255.0) as u8,
            r,
            g,
            b,
        ))
    }

    fn parse_hsl_inner(inner: &str, with_alpha: bool) -> Option<Self> {
        let mut it = inner.split(',');
        let h: f32 = it.next()?.trim().trim_end_matches("deg").parse().ok()?;
        let s: f32 = it.next()?.trim().trim_end_matches('%').parse::<f32>().ok()? / 100.0;
        let l: f32 = it.next()?.trim().trim_end_matches('%').parse::<f32>().ok()? / 100.0;
        let a: f32 = if with_alpha {
            it.next()?.trim().parse().ok()?
        } else {
            if it.next().is_some() {
                return None;
            }
            1.0
        };

        let (rf, gf, bf) = hsl_to_rgb(h, s, l);
        let to_u8 = |x: f32| (x * 255.0).round().clamp(0.0, 255.0) as u8;
        Some(Self::from_argb(
            (a * 255.0).round().clamp(0.0, 255.0) as u8,
            to_u8(rf),
            to_u8(gf),
            to_u8(bf),
        ))
    }

    // -------------------------------------------------------------------------
    // CSS Color Level 4 parsers
    // -------------------------------------------------------------------------

    /// Split color function components on whitespace/comma, handling slash-separated alpha.
    fn split_components(s: &str) -> (Vec<&str>, Option<&str>) {
        if let Some((main, alpha)) = s.split_once('/') {
            let parts: Vec<&str> = main
                .split(|c| c == ' ' || c == ',')
                .filter(|p| !p.is_empty())
                .collect();
            (parts, Some(alpha.trim()))
        } else {
            let parts: Vec<&str> = s
                .split(|c| c == ' ' || c == ',')
                .filter(|p| !p.is_empty())
                .collect();
            (parts, None)
        }
    }

    /// Parse alpha component (number 0..1 or percentage 0..100%).
    fn parse_alpha(s: &str) -> Option<u8> {
        let s = s.trim();
        if let Some(pct) = s.strip_suffix('%') {
            let v: f32 = pct.parse().ok()?;
            Some((v * 2.55).round().clamp(0.0, 255.0) as u8)
        } else {
            let v: f32 = s.parse().ok()?;
            Some((v * 255.0).round().clamp(0.0, 255.0) as u8)
        }
    }

    /// Convert linear sRGB component to sRGB u8.
    fn linear_to_srgb_u8(x: f32) -> u8 {
        let c = if x <= 0.0 {
            0.0
        } else if x >= 1.0 {
            1.0
        } else if x <= 0.0031308 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        (c * 255.0).round() as u8
    }

    /// Convert Oklab to linear sRGB.
    /// Reference: https://bottosson.github.io/posts/oklab/
    fn oklab_to_linear_srgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
        let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
        let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
        let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

        let l_cubed = l_ * l_ * l_;
        let m_cubed = m_ * m_ * m_;
        let s_cubed = s_ * s_ * s_;

        let r = 4.0767245293 * l_cubed - 3.3072168827 * m_cubed + 0.2307590544 * s_cubed;
        let g = -1.2681437731 * l_cubed + 2.6093323231 * m_cubed - 0.3411344290 * s_cubed;
        let b = -0.0041119885 * l_cubed - 0.7034763098 * m_cubed + 1.7068625689 * s_cubed;

        (r, g, b)
    }

    /// Convert CIE Lab (D50) to linear sRGB.
    fn lab_to_linear_srgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
        // Lab → XYZ (D50 whitepoint)
        let fy = (l + 16.0) / 116.0;
        let fx = a / 500.0 + fy;
        let fz = fy - b / 200.0;

        let lab_f_inverse = |t: f32| {
            let k = (6.0_f32 / 29.0_f32).powi(3);
            if t.powi(3) > k {
                t.powi(3)
            } else {
                (t - 16.0 / 116.0) * 3.0 * (6.0_f32 / 29.0_f32).powi(2)
            }
        };

        // D50 whitepoint
        let wx = 0.96422;
        let wy = 1.0;
        let wz = 0.82521;
        let x_d50 = wx * lab_f_inverse(fx);
        let y_d50 = wy * lab_f_inverse(fy);
        let z_d50 = wz * lab_f_inverse(fz);

        // Bradford-adapt D50 XYZ → D65 XYZ
        let x_d65 = 0.9555766 * x_d50 + -0.0230393 * y_d50 + 0.0631636 * z_d50;
        let y_d65 = -0.0282895 * x_d50 + 1.0099416 * y_d50 + 0.0210077 * z_d50;
        let z_d65 = 0.0122982 * x_d50 + -0.0204830 * y_d50 + 1.3299098 * z_d50;

        // XYZ (D65) → linear sRGB
        let r = 3.2404542 * x_d65 + -1.5371385 * y_d65 + -0.4985314 * z_d65;
        let g = -0.9692660 * x_d65 + 1.8760108 * y_d65 + 0.0415560 * z_d65;
        let b = 0.0556434 * x_d65 + -0.2040259 * y_d65 + 1.0572252 * z_d65;

        (r, g, b)
    }

    /// Convert cylindrical LCH to Cartesian Lab.
    fn lch_to_lab(l: f32, c: f32, h_deg: f32) -> (f32, f32, f32) {
        let h_rad = h_deg.to_radians();
        (l, c * h_rad.cos(), c * h_rad.sin())
    }

    fn parse_oklab(s: &str) -> Option<Self> {
        let (parts, alpha) = Self::split_components(s);
        if parts.len() != 3 {
            return None;
        }
        let l = Self::parse_oklab_l(parts[0])?;
        let a: f32 = parts[1].trim().parse().ok()?;
        let b: f32 = parts[2].trim().parse().ok()?;
        let alpha_u8 = alpha.and_then(Self::parse_alpha).unwrap_or(255);

        let (lin_r, lin_g, lin_b) = Self::oklab_to_linear_srgb(l, a, b);
        let r = Self::linear_to_srgb_u8(lin_r);
        let g = Self::linear_to_srgb_u8(lin_g);
        let b_ = Self::linear_to_srgb_u8(lin_b);
        Some(Self::from_argb(alpha_u8, r, g, b_))
    }

    fn parse_oklab_l(s: &str) -> Option<f32> {
        let s = s.trim();
        if let Some(pct) = s.strip_suffix('%') {
            Some(pct.parse::<f32>().ok()? / 100.0)
        } else {
            s.parse::<f32>().ok()
        }
    }

    fn parse_oklch(s: &str) -> Option<Self> {
        let (parts, alpha) = Self::split_components(s);
        if parts.len() != 3 {
            return None;
        }
        let l = Self::parse_oklab_l(parts[0])?;
        let c: f32 = if let Some(pct) = parts[1].trim().strip_suffix('%') {
            pct.parse::<f32>().ok()? / 100.0
        } else {
            parts[1].trim().parse().ok()?
        };
        let h: f32 = parts[2].trim().trim_end_matches("deg").parse().ok()?;
        let alpha_u8 = alpha.and_then(Self::parse_alpha).unwrap_or(255);

        let (l, a, b) = Self::lch_to_lab(l, c, h);
        let (lin_r, lin_g, lin_b) = Self::oklab_to_linear_srgb(l, a, b);
        let r = Self::linear_to_srgb_u8(lin_r);
        let g = Self::linear_to_srgb_u8(lin_g);
        let b_ = Self::linear_to_srgb_u8(lin_b);
        Some(Self::from_argb(alpha_u8, r, g, b_))
    }

    fn parse_lab(s: &str) -> Option<Self> {
        let (parts, alpha) = Self::split_components(s);
        if parts.len() != 3 {
            return None;
        }
        let l = Self::parse_lab_l(parts[0])?;
        let a: f32 = parts[1].trim().parse().ok()?;
        let b: f32 = parts[2].trim().parse().ok()?;
        let alpha_u8 = alpha.and_then(Self::parse_alpha).unwrap_or(255);

        let (lin_r, lin_g, lin_b) = Self::lab_to_linear_srgb(l, a, b);
        let r = Self::linear_to_srgb_u8(lin_r);
        let g = Self::linear_to_srgb_u8(lin_g);
        let b_ = Self::linear_to_srgb_u8(lin_b);
        Some(Self::from_argb(alpha_u8, r, g, b_))
    }

    fn parse_lab_l(s: &str) -> Option<f32> {
        let s = s.trim();
        if let Some(pct) = s.strip_suffix('%') {
            Some(pct.parse::<f32>().ok()?)
        } else {
            s.parse::<f32>().ok()
        }
    }

    fn parse_lch(s: &str) -> Option<Self> {
        let (parts, alpha) = Self::split_components(s);
        if parts.len() != 3 {
            return None;
        }
        let l = Self::parse_lab_l(parts[0])?;
        let c: f32 = parts[1].trim().parse().ok()?;
        let h: f32 = parts[2].trim().trim_end_matches("deg").parse().ok()?;
        let alpha_u8 = alpha.and_then(Self::parse_alpha).unwrap_or(255);

        let (l, a, b) = Self::lch_to_lab(l, c, h);
        let (lin_r, lin_g, lin_b) = Self::lab_to_linear_srgb(l, a, b);
        let r = Self::linear_to_srgb_u8(lin_r);
        let g = Self::linear_to_srgb_u8(lin_g);
        let b_ = Self::linear_to_srgb_u8(lin_b);
        Some(Self::from_argb(alpha_u8, r, g, b_))
    }

    fn parse_hwb(s: &str) -> Option<Self> {
        let (parts, alpha) = Self::split_components(s);
        if parts.len() != 3 {
            return None;
        }
        let h: f32 = parts[0].trim().trim_end_matches("deg").parse().ok()?;
        let w: f32 = parts[1].trim().trim_end_matches('%').parse::<f32>().ok()? / 100.0;
        let b: f32 = parts[2].trim().trim_end_matches('%').parse::<f32>().ok()? / 100.0;
        let alpha_u8 = alpha.and_then(Self::parse_alpha).unwrap_or(255);

        // If w + b >= 1, return gray
        let sum = w + b;
        let (w, b) = if sum >= 1.0 {
            (w / sum, b / sum)
        } else {
            (w, b)
        };

        // Get pure hue via HSL
        let (hr, hg, hb) = hsl_to_rgb(h, 1.0, 0.5);
        let blend = |ch: f32| ch * (1.0 - w - b) + w;
        let r = blend(hr);
        let g = blend(hg);
        let b_ch = blend(hb);

        let to_u8 = |x: f32| (x * 255.0).round().clamp(0.0, 255.0) as u8;
        Some(Self::from_argb(alpha_u8, to_u8(r), to_u8(g), to_u8(b_ch)))
    }

    fn parse_color_function(s: &str) -> Option<Self> {
        let (parts, alpha) = Self::split_components(s);
        if parts.len() != 4 {
            return None;
        }
        let space = parts[0];
        let r: f32 = parts[1].trim().parse().ok()?;
        let g: f32 = parts[2].trim().parse().ok()?;
        let b: f32 = parts[3].trim().parse().ok()?;
        let alpha_u8 = alpha.and_then(Self::parse_alpha).unwrap_or(255);

        let (lin_r, lin_g, lin_b) = match space {
            "srgb" => {
                // sRGB gamma space → linear
                let to_linear = |x: f32| {
                    if x <= 0.04045 {
                        x / 12.92
                    } else {
                        ((x + 0.055) / 1.055).powf(2.4)
                    }
                };
                (to_linear(r), to_linear(g), to_linear(b))
            }
            "srgb-linear" => (r, g, b),
            "display-p3" => {
                // P3 → linear sRGB matrix (approximate)
                let lr = 1.2249 * r + -0.2247 * g + 0.0000 * b;
                let lg = -0.0420 * r + 1.0419 * g + 0.0000 * b;
                let lb = -0.0197 * r + -0.0786 * g + 1.0980 * b;
                (lr, lg, lb)
            }
            "a98-rgb" => {
                // Adobe RGB → linear sRGB (approximate)
                let lr = 1.04788 * r + -0.02908 * g + -0.00923 * b;
                let lg = -0.01726 * r + 0.99835 * g + 0.00754 * b;
                let lb = 0.00612 * r + -0.03608 * g + 1.03988 * b;
                (lr, lg, lb)
            }
            // rec2020 and prophoto-rgb not supported — no matrix
            _ => return None,
        };

        Some(Self::from_argb(
            alpha_u8,
            Self::linear_to_srgb_u8(lin_r),
            Self::linear_to_srgb_u8(lin_g),
            Self::linear_to_srgb_u8(lin_b),
        ))
    }

    fn named_color(s: &str) -> Option<Self> {
        // CSS Color Level 3 named colors (subset of the X11 set).
        // Table is scanned with eq_ignore_ascii_case so callers do not need
        // to pre-lowercase the input — no heap allocation on the hot path.
        const TABLE: &[(&str, (u8, u8, u8))] = &[
            ("aliceblue", (240, 248, 255)),
            ("antiquewhite", (250, 235, 215)),
            ("aqua", (0, 255, 255)),
            ("aquamarine", (127, 255, 212)),
            ("azure", (240, 255, 255)),
            ("beige", (245, 245, 220)),
            ("bisque", (255, 228, 196)),
            ("blanchedalmond", (255, 235, 205)),
            ("blue", (0, 0, 255)),
            ("blueviolet", (138, 43, 226)),
            ("brown", (165, 42, 42)),
            ("burlywood", (222, 184, 135)),
            ("cadetblue", (95, 158, 160)),
            ("chartreuse", (127, 255, 0)),
            ("chocolate", (210, 105, 30)),
            ("coral", (255, 127, 80)),
            ("cornflowerblue", (100, 149, 237)),
            ("cornsilk", (255, 248, 220)),
            ("crimson", (220, 20, 60)),
            ("cyan", (0, 255, 255)),
            ("darkblue", (0, 0, 139)),
            ("darkcyan", (0, 139, 139)),
            ("darkgoldenrod", (184, 134, 11)),
            ("darkgray", (169, 169, 169)),
            ("darkgreen", (0, 100, 0)),
            ("darkgrey", (169, 169, 169)),
            ("darkkhaki", (189, 183, 107)),
            ("darkmagenta", (139, 0, 139)),
            ("darkolivegreen", (85, 107, 47)),
            ("darkorange", (255, 140, 0)),
            ("darkorchid", (153, 50, 204)),
            ("darkred", (139, 0, 0)),
            ("darksalmon", (233, 150, 122)),
            ("darkseagreen", (143, 188, 143)),
            ("darkslateblue", (72, 61, 139)),
            ("darkslategray", (47, 79, 79)),
            ("darkslategrey", (47, 79, 79)),
            ("darkturquoise", (0, 206, 209)),
            ("darkviolet", (148, 0, 211)),
            ("deeppink", (255, 20, 147)),
            ("deepskyblue", (0, 191, 255)),
            ("dimgray", (105, 105, 105)),
            ("dimgrey", (105, 105, 105)),
            ("dodgerblue", (30, 144, 255)),
            ("firebrick", (178, 34, 34)),
            ("floralwhite", (255, 250, 240)),
            ("forestgreen", (34, 139, 34)),
            ("fuchsia", (255, 0, 255)),
            ("gainsboro", (220, 220, 220)),
            ("ghostwhite", (248, 248, 255)),
            ("gold", (255, 215, 0)),
            ("goldenrod", (218, 165, 32)),
            ("gray", (128, 128, 128)),
            ("green", (0, 128, 0)),
            ("greenyellow", (173, 255, 47)),
            ("grey", (128, 128, 128)),
            ("honeydew", (240, 255, 240)),
            ("hotpink", (255, 105, 180)),
            ("indianred", (205, 92, 92)),
            ("indigo", (75, 0, 130)),
            ("ivory", (255, 255, 240)),
            ("khaki", (240, 230, 140)),
            ("lavender", (230, 230, 250)),
            ("lavenderblush", (255, 240, 245)),
            ("lawngreen", (124, 252, 0)),
            ("lemonchiffon", (255, 250, 205)),
            ("lightblue", (173, 216, 230)),
            ("lightcoral", (240, 128, 128)),
            ("lightcyan", (224, 255, 255)),
            ("lightgoldenrodyellow", (250, 250, 210)),
            ("lightgray", (211, 211, 211)),
            ("lightgreen", (144, 238, 144)),
            ("lightgrey", (211, 211, 211)),
            ("lightpink", (255, 182, 193)),
            ("lightsalmon", (255, 160, 122)),
            ("lightseagreen", (32, 178, 170)),
            ("lightskyblue", (135, 206, 250)),
            ("lightslategray", (119, 136, 153)),
            ("lightslategrey", (119, 136, 153)),
            ("lightsteelblue", (176, 196, 222)),
            ("lightyellow", (255, 255, 224)),
            ("lime", (0, 255, 0)),
            ("limegreen", (50, 205, 50)),
            ("linen", (250, 240, 230)),
            ("magenta", (255, 0, 255)),
            ("maroon", (128, 0, 0)),
            ("mediumaquamarine", (102, 205, 170)),
            ("mediumblue", (0, 0, 205)),
            ("mediumorchid", (186, 85, 211)),
            ("mediumpurple", (147, 112, 219)),
            ("mediumseagreen", (60, 179, 113)),
            ("mediumslateblue", (123, 104, 238)),
            ("mediumspringgreen", (0, 250, 154)),
            ("mediumturquoise", (72, 209, 204)),
            ("mediumvioletred", (199, 21, 133)),
            ("midnightblue", (25, 25, 112)),
            ("mintcream", (245, 255, 250)),
            ("mistyrose", (255, 228, 225)),
            ("moccasin", (255, 228, 181)),
            ("navajowhite", (255, 222, 173)),
            ("navy", (0, 0, 128)),
            ("oldlace", (253, 245, 230)),
            ("olive", (128, 128, 0)),
            ("olivedrab", (107, 142, 35)),
            ("orange", (255, 165, 0)),
            ("orangered", (255, 69, 0)),
            ("orchid", (218, 112, 214)),
            ("palegoldenrod", (238, 232, 170)),
            ("palegreen", (152, 251, 152)),
            ("paleturquoise", (175, 238, 238)),
            ("palevioletred", (219, 112, 147)),
            ("papayawhip", (255, 239, 213)),
            ("peachpuff", (255, 218, 185)),
            ("peru", (205, 133, 63)),
            ("pink", (255, 192, 203)),
            ("plum", (221, 160, 221)),
            ("powderblue", (176, 224, 230)),
            ("purple", (128, 0, 128)),
            ("rebeccapurple", (102, 51, 153)),
            ("red", (255, 0, 0)),
            ("rosybrown", (188, 143, 143)),
            ("royalblue", (65, 105, 225)),
            ("saddlebrown", (139, 69, 19)),
            ("salmon", (250, 128, 114)),
            ("sandybrown", (244, 164, 96)),
            ("seagreen", (46, 139, 87)),
            ("seashell", (255, 245, 238)),
            ("sienna", (160, 82, 45)),
            ("silver", (192, 192, 192)),
            ("skyblue", (135, 206, 235)),
            ("slateblue", (106, 90, 205)),
            ("slategray", (112, 128, 144)),
            ("slategrey", (112, 128, 144)),
            ("snow", (255, 250, 250)),
            ("springgreen", (0, 255, 127)),
            ("steelblue", (70, 130, 180)),
            ("tan", (210, 180, 140)),
            ("teal", (0, 128, 128)),
            ("thistle", (216, 191, 216)),
            ("tomato", (255, 99, 71)),
            ("turquoise", (64, 224, 208)),
            ("violet", (238, 130, 238)),
            ("wheat", (245, 222, 179)),
            ("whitesmoke", (245, 245, 245)),
            ("yellow", (255, 255, 0)),
            ("yellowgreen", (154, 205, 50)),
        ];

        // Special-case the two colors that do not decode to from_rgb(…) alone.
        if s.eq_ignore_ascii_case("black") {
            return Some(Self::BLACK);
        }
        if s.eq_ignore_ascii_case("white") {
            return Some(Self::WHITE);
        }
        if s.eq_ignore_ascii_case("transparent") {
            return Some(Self::TRANSPARENT);
        }

        for &(name, (r, g, b)) in TABLE {
            if s.eq_ignore_ascii_case(name) {
                return Some(Self::from_rgb(r, g, b));
            }
        }
        None
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

    /// Parse an ICC profile from a byte buffer.
    ///
    /// Reads the 128-byte ICC header and the tag table to recover the
    /// profile's transfer function (from the `rTRC` tag) and gamut
    /// primaries (from the `rXYZ`/`gXYZ`/`bXYZ` tags). The extracted
    /// transfer function and gamut are used to populate
    /// `embedded_color_space`.
    ///
    /// Supported transfer-curve tag types:
    /// - `curv` with count 0 (identity), count 1 (pure gamma via u8.8
    ///   fixed-point), or tabulated (approximated as sRGB if it spans
    ///   `[0, 1]`, else gamma 2.2).
    /// - `para` function types 0, 3, and 4 (pure gamma, 5-parameter, and
    ///   7-parameter parametric curves respectively). The sRGB form is
    ///   detected by parameter inspection.
    ///
    /// Primaries are matched against known named gamuts (sRGB/Rec.709,
    /// Display P3, Adobe RGB, Rec.2020) with a small tolerance; anything
    /// else falls back to `ColorGamut::Custom`.
    ///
    /// Returns `None` if the bytes are malformed, too short, or fail the
    /// `acsp` magic-number check. Profiles whose tag table cannot be
    /// resolved fall back to sRGB for `embedded_color_space` rather than
    /// returning `None`.
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

        // Parse the tag table and try to rebuild a ColorSpace from the
        // rTRC + rXYZ/gXYZ/bXYZ tags. The red TRC is used for all three
        // channels (ColorSpace currently carries a single TransferFunction
        // and virtually all real-world RGB profiles use identical R/G/B
        // curves). If any required tag is missing or uses an unsupported
        // encoding, fall back to sRGB instead of failing the whole parse.
        let embedded_color_space = icc::parse_tag_table(data)
            .and_then(|tags| {
                let rx = icc::read_xyz_tag(&tags, data, b"rXYZ")?;
                let gx = icc::read_xyz_tag(&tags, data, b"gXYZ")?;
                let bx = icc::read_xyz_tag(&tags, data, b"bXYZ")?;
                let rtrc = icc::read_trc_tag(&tags, data, b"rTRC")?;
                let gamut = icc::classify_gamut(rx, gx, bx);
                Some(ColorSpace {
                    transfer_fn: rtrc,
                    gamut,
                })
            })
            .unwrap_or_else(ColorSpace::srgb);

        Some(Self {
            profile_class,
            color_space,
            pcs,
            description: "ICC Profile".to_string(),
            embedded_color_space,
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
// ICC Tag Table Parsing (internal)
// =============================================================================

/// Private helpers for parsing the ICC v2/v4 tag table referenced by
/// `IccProfile::from_bytes`. Kept in its own module so the parsing code
/// cannot accidentally reach into `IccProfile` internals — this module's
/// only interface with the outer type is returning a `ColorSpace`.
mod icc {
    use super::{ColorGamut, TransferFunction};

    /// A single entry from the ICC tag table.
    #[derive(Debug)]
    pub(super) struct IccTag {
        pub signature: [u8; 4],
        pub offset: u32,
        pub size: u32,
    }

    #[inline]
    fn read_u32_be(bytes: &[u8], off: usize) -> Option<u32> {
        bytes
            .get(off..off + 4)?
            .try_into()
            .ok()
            .map(u32::from_be_bytes)
    }

    #[inline]
    fn read_u16_be(bytes: &[u8], off: usize) -> Option<u16> {
        bytes
            .get(off..off + 2)?
            .try_into()
            .ok()
            .map(u16::from_be_bytes)
    }

    /// Read a signed 15.16 fixed-point number (s15Fixed16Number) at `off`.
    #[inline]
    fn read_s15_16(bytes: &[u8], off: usize) -> Option<f32> {
        let raw = read_u32_be(bytes, off)? as i32;
        Some(raw as f32 / 65536.0)
    }

    /// Parse the tag table at byte 128. Performs bounds and sanity checks
    /// so a corrupted count or offset cannot cause downstream panics.
    pub(super) fn parse_tag_table(bytes: &[u8]) -> Option<Vec<IccTag>> {
        if bytes.len() < 132 {
            return None;
        }
        let count = read_u32_be(bytes, 128)? as usize;
        // Each tag entry is 12 bytes. Reject pathological counts and
        // counts that would overflow the buffer.
        if count > 1024 {
            return None;
        }
        let table_end = 132_usize.checked_add(count.checked_mul(12)?)?;
        if table_end > bytes.len() {
            return None;
        }
        let mut tags = Vec::with_capacity(count);
        for i in 0..count {
            let base = 132 + i * 12;
            let signature: [u8; 4] = bytes.get(base..base + 4)?.try_into().ok()?;
            let offset = read_u32_be(bytes, base + 4)?;
            let size = read_u32_be(bytes, base + 8)?;
            // The tag's data range must lie within the buffer.
            let end = (offset as usize).checked_add(size as usize)?;
            if end > bytes.len() {
                return None;
            }
            tags.push(IccTag {
                signature,
                offset,
                size,
            });
        }
        Some(tags)
    }

    fn find_tag<'a>(tags: &'a [IccTag], sig: &[u8; 4]) -> Option<&'a IccTag> {
        tags.iter().find(|t| &t.signature == sig)
    }

    /// Read an `XYZ ` tag and return its three s15.16 components.
    pub(super) fn read_xyz_tag(
        tags: &[IccTag],
        bytes: &[u8],
        sig: &[u8; 4],
    ) -> Option<[f32; 3]> {
        let tag = find_tag(tags, sig)?;
        let start = tag.offset as usize;
        let end = start.checked_add(tag.size as usize)?;
        let data = bytes.get(start..end)?;
        // 4 bytes type sig + 4 bytes reserved + 3 × 4 bytes s15Fixed16
        if data.len() < 20 || &data[0..4] != b"XYZ " {
            return None;
        }
        let x = read_s15_16(data, 8)?;
        let y = read_s15_16(data, 12)?;
        let z = read_s15_16(data, 16)?;
        Some([x, y, z])
    }

    /// Read a TRC tag (`rTRC`, `gTRC`, or `bTRC`) and decode its transfer
    /// function. Supports `curv` and `para` type signatures.
    pub(super) fn read_trc_tag(
        tags: &[IccTag],
        bytes: &[u8],
        sig: &[u8; 4],
    ) -> Option<TransferFunction> {
        let tag = find_tag(tags, sig)?;
        let start = tag.offset as usize;
        let end = start.checked_add(tag.size as usize)?;
        let data = bytes.get(start..end)?;
        if data.len() < 12 {
            return None;
        }
        match &data[0..4] {
            b"curv" => parse_curv(data),
            b"para" => parse_para(data),
            _ => None,
        }
    }

    /// Parse a `curv` tag body.
    ///
    /// Layout: 4 bytes sig + 4 bytes reserved + 4 bytes count + count × u16.
    /// - count == 0: identity curve (gamma 1.0) → Linear
    /// - count == 1: u8.8 fixed-point gamma
    /// - count >  1: tabulated curve (approximated; see below)
    fn parse_curv(data: &[u8]) -> Option<TransferFunction> {
        let count = read_u32_be(data, 8)? as usize;
        if count == 0 {
            return Some(TransferFunction::Linear);
        }
        if count == 1 {
            let raw = read_u16_be(data, 12)?;
            let gamma = raw as f32 / 256.0;
            return Some(gamma_to_transfer(gamma));
        }
        // Tabulated curve. We don't store arbitrary LUTs in
        // TransferFunction, so approximate: if the curve spans roughly
        // [0, 1] we treat it as sRGB (the overwhelmingly common case for
        // tabulated TRCs on display profiles); otherwise fall back to a
        // gamma 2.2 approximation. Callers needing exact evaluation can
        // parse the raw bytes from raw_data().
        if data.len() < 12 + count * 2 {
            return None;
        }
        let first = read_u16_be(data, 12)?;
        let last = read_u16_be(data, 12 + (count - 1) * 2)?;
        if first <= 8 && last >= 0xFF00 {
            Some(TransferFunction::Srgb)
        } else {
            Some(gamma_to_transfer(2.2))
        }
    }

    /// Parse a `para` tag body.
    ///
    /// Layout: 4 bytes sig + 4 bytes reserved + 2 bytes function type +
    /// 2 bytes reserved + parameters (each an s15Fixed16).
    ///
    /// Function types:
    /// - 0: Y = X^g                             (1 param)
    /// - 1: Y = (aX+b)^g if X >= -b/a else 0    (3 params)
    /// - 2: Y = (aX+b)^g + c if X >= -b/a else c (4 params)
    /// - 3: Y = (aX+b)^g if X >= d else cX       (5 params)
    /// - 4: Y = (aX+b)^g + e if X >= d else cX + f (7 params; sRGB-shaped)
    fn parse_para(data: &[u8]) -> Option<TransferFunction> {
        let function_type = read_u16_be(data, 8)?;
        let p = 12; // parameters start offset within the tag data
        match function_type {
            0 => {
                if data.len() < p + 4 {
                    return None;
                }
                let g = read_s15_16(data, p)?;
                Some(gamma_to_transfer(g))
            }
            1 => {
                if data.len() < p + 12 {
                    return None;
                }
                let g = read_s15_16(data, p)?;
                let a = read_s15_16(data, p + 4)?;
                let b = read_s15_16(data, p + 8)?;
                // Expressed as the 7-param form with d = -b/a, c/e/f = 0.
                let d = if a != 0.0 { -b / a } else { 0.0 };
                classify_parametric(g, a, b, 0.0, d, 0.0, 0.0)
            }
            2 => {
                if data.len() < p + 16 {
                    return None;
                }
                let g = read_s15_16(data, p)?;
                let a = read_s15_16(data, p + 4)?;
                let b = read_s15_16(data, p + 8)?;
                let c = read_s15_16(data, p + 12)?;
                let d = if a != 0.0 { -b / a } else { 0.0 };
                classify_parametric(g, a, b, 0.0, d, c, c)
            }
            3 => {
                if data.len() < p + 20 {
                    return None;
                }
                let g = read_s15_16(data, p)?;
                let a = read_s15_16(data, p + 4)?;
                let b = read_s15_16(data, p + 8)?;
                let c = read_s15_16(data, p + 12)?;
                let d = read_s15_16(data, p + 16)?;
                classify_parametric(g, a, b, c, d, 0.0, 0.0)
            }
            4 => {
                if data.len() < p + 28 {
                    return None;
                }
                let g = read_s15_16(data, p)?;
                let a = read_s15_16(data, p + 4)?;
                let b = read_s15_16(data, p + 8)?;
                let c = read_s15_16(data, p + 12)?;
                let d = read_s15_16(data, p + 16)?;
                let e = read_s15_16(data, p + 20)?;
                let f = read_s15_16(data, p + 24)?;
                classify_parametric(g, a, b, c, d, e, f)
            }
            _ => None,
        }
    }

    /// Collapse a pure power curve to a named variant where possible.
    fn gamma_to_transfer(g: f32) -> TransferFunction {
        if (g - 1.0).abs() < 1e-3 {
            TransferFunction::Linear
        } else {
            // Store as a parametric pure power: Y = X^g (a=1, b=0, rest 0).
            TransferFunction::Parametric {
                g,
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 0.0,
                e: 0.0,
                f: 0.0,
            }
        }
    }

    /// Identify common named transfer functions from their 7-parameter
    /// form. Falls back to `TransferFunction::Parametric` for anything we
    /// don't recognise.
    fn classify_parametric(
        g: f32,
        a: f32,
        b: f32,
        c: f32,
        d: f32,
        e: f32,
        f: f32,
    ) -> Option<TransferFunction> {
        // sRGB: g=2.4, a=1/1.055, b=0.055/1.055, c=1/12.92, d=0.04045,
        // e=0, f=0. Tolerances are loose enough to accept the rounded
        // s15Fixed16 representations that real profiles emit.
        let near = |x: f32, y: f32| (x - y).abs() < 1e-3;
        if near(g, 2.4)
            && near(a, 1.0 / 1.055)
            && near(b, 0.055 / 1.055)
            && near(c, 1.0 / 12.92)
            && near(d, 0.04045)
            && near(e, 0.0)
            && near(f, 0.0)
        {
            return Some(TransferFunction::Srgb);
        }
        // Rec.2020 / Rec.709: g=1/0.45 ≈ 2.222, a=1/1.099, b=0.099/1.099,
        // c=1/4.5, d=0.081, e=0, f=0.
        if near(g, 1.0 / 0.45)
            && near(a, 1.0 / 1.099)
            && near(b, 0.099 / 1.099)
            && near(c, 1.0 / 4.5)
            && near(d, 0.081)
            && near(e, 0.0)
            && near(f, 0.0)
        {
            return Some(TransferFunction::Rec2020);
        }
        Some(TransferFunction::Parametric {
            g,
            a,
            b,
            c,
            d,
            e,
            f,
        })
    }

    /// Map D50-adapted primaries (rXYZ, gXYZ, bXYZ) to a named
    /// `ColorGamut`. Tolerance is ~0.01 in XYZ units, which is wide
    /// enough to swallow the precision loss of s15Fixed16 but tight
    /// enough to discriminate sRGB, P3, Adobe RGB, and Rec.2020.
    pub(super) fn classify_gamut(r: [f32; 3], g: [f32; 3], b: [f32; 3]) -> ColorGamut {
        // Reference primaries, D50-adapted (via Bradford), as emitted by
        // well-known profile generators (lcms2, ArgyllCMS, Apple):
        let sets: &[(ColorGamut, [[f32; 3]; 3])] = &[
            (
                ColorGamut::Srgb,
                [
                    [0.4361, 0.2225, 0.0139],
                    [0.3851, 0.7169, 0.0971],
                    [0.1431, 0.0606, 0.7141],
                ],
            ),
            (
                ColorGamut::DisplayP3,
                [
                    [0.5151, 0.2412, -0.0010],
                    [0.2920, 0.6922, 0.0419],
                    [0.1571, 0.0666, 0.7841],
                ],
            ),
            (
                ColorGamut::AdobeRgb,
                [
                    [0.6097, 0.3111, 0.0195],
                    [0.2053, 0.6257, 0.0609],
                    [0.1492, 0.0632, 0.7446],
                ],
            ),
            (
                ColorGamut::Rec2020,
                [
                    [0.6734, 0.2790, -0.0019],
                    [0.1656, 0.6753, 0.0299],
                    [0.1251, 0.0457, 0.7969],
                ],
            ),
        ];
        const TOL: f32 = 0.01;
        for (gamut, p) in sets {
            let dr = (r[0] - p[0][0]).abs() + (r[1] - p[0][1]).abs() + (r[2] - p[0][2]).abs();
            let dg = (g[0] - p[1][0]).abs() + (g[1] - p[1][1]).abs() + (g[2] - p[1][2]).abs();
            let db = (b[0] - p[2][0]).abs() + (b[1] - p[2][1]).abs() + (b[2] - p[2][2]).abs();
            if dr < 3.0 * TOL && dg < 3.0 * TOL && db < 3.0 * TOL {
                return *gamut;
            }
        }
        ColorGamut::Custom
    }
}

// =============================================================================
// Color Space Conversion Utilities
// =============================================================================

/// Convert a single sRGB component to linear.
///
/// The sRGB transfer function is piecewise: linear for small values,
/// then a power curve for larger values.
#[inline]
pub fn srgb_to_linear(s: Scalar) -> Scalar {
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert a single linear component to sRGB.
#[inline]
pub fn linear_to_srgb(l: Scalar) -> Scalar {
    if l <= 0.0031308 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert an sRGB Color4f to linear RGB.
#[inline]
pub fn color4f_srgb_to_linear(color: &Color4f) -> Color4f {
    Color4f {
        r: srgb_to_linear(color.r),
        g: srgb_to_linear(color.g),
        b: srgb_to_linear(color.b),
        a: color.a, // Alpha is always linear
    }
}

/// Convert a linear RGB Color4f to sRGB.
#[inline]
pub fn color4f_linear_to_srgb(color: &Color4f) -> Color4f {
    Color4f {
        r: linear_to_srgb(color.r),
        g: linear_to_srgb(color.g),
        b: linear_to_srgb(color.b),
        a: color.a,
    }
}

/// Convert sRGB Color to linear RGB Color4f.
#[inline]
pub fn color_to_linear(color: Color) -> Color4f {
    color4f_srgb_to_linear(&Color4f::from_color(color))
}

/// Convert linear RGB Color4f to sRGB Color.
#[inline]
pub fn linear_to_color(color: &Color4f) -> Color {
    color4f_linear_to_srgb(color).to_color()
}

/// HSL to RGB conversion.
///
/// H is in [0, 360), S and L are in [0, 1].
/// Returns (R, G, B) in [0, 1].
pub fn hsl_to_rgb(h: Scalar, s: Scalar, l: Scalar) -> (Scalar, Scalar, Scalar) {
    if s == 0.0 {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let h = h / 360.0;

    let hue_to_rgb = |p: Scalar, q: Scalar, mut t: Scalar| -> Scalar {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 0.5 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    };

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    (r, g, b)
}

/// RGB to HSL conversion.
///
/// R, G, B are in [0, 1].
/// Returns (H, S, L) where H is in [0, 360), S and L are in [0, 1].
pub fn rgb_to_hsl(r: Scalar, g: Scalar, b: Scalar) -> (Scalar, Scalar, Scalar) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if max == min {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    (h * 60.0, s, l)
}

/// HSV to RGB conversion.
///
/// H is in [0, 360), S and V are in [0, 1].
/// Returns (R, G, B) in [0, 1].
pub fn hsv_to_rgb(h: Scalar, s: Scalar, v: Scalar) -> (Scalar, Scalar, Scalar) {
    if s == 0.0 {
        return (v, v, v);
    }

    let h = h / 60.0;
    let i = h.floor();
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    match i as i32 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

/// RGB to HSV conversion.
///
/// R, G, B are in [0, 1].
/// Returns (H, S, V) where H is in [0, 360), S and V are in [0, 1].
pub fn rgb_to_hsv(r: Scalar, g: Scalar, b: Scalar) -> (Scalar, Scalar, Scalar) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let v = max;
    let d = max - min;

    let s = if max == 0.0 { 0.0 } else { d / max };

    if max == min {
        return (0.0, s, v);
    }

    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    (h * 60.0, s, v)
}

/// RGB to XYZ conversion (using sRGB primaries).
///
/// R, G, B are linear values in [0, 1].
/// Returns (X, Y, Z) where Y is luminance.
pub fn rgb_to_xyz(r: Scalar, g: Scalar, b: Scalar) -> (Scalar, Scalar, Scalar) {
    // sRGB to XYZ matrix (D65 white point)
    let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
    let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
    let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;
    (x, y, z)
}

/// XYZ to RGB conversion (to sRGB primaries).
///
/// Returns (R, G, B) linear values.
pub fn xyz_to_rgb(x: Scalar, y: Scalar, z: Scalar) -> (Scalar, Scalar, Scalar) {
    // XYZ to sRGB matrix (D65 white point)
    let r = x * 3.2404542 + y * -1.5371385 + z * -0.4985314;
    let g = x * -0.9692660 + y * 1.8760108 + z * 0.0415560;
    let b = x * 0.0556434 + y * -0.2040259 + z * 1.0572252;
    (r, g, b)
}

/// RGB to Lab conversion.
///
/// R, G, B are linear values in [0, 1].
/// Returns (L, a, b) where L is in [0, 100], a and b are approximately [-128, 128].
pub fn rgb_to_lab(r: Scalar, g: Scalar, b: Scalar) -> (Scalar, Scalar, Scalar) {
    let (x, y, z) = rgb_to_xyz(r, g, b);

    // D65 reference white
    let ref_x = 0.95047;
    let ref_y = 1.00000;
    let ref_z = 1.08883;

    let f = |t: Scalar| -> Scalar {
        if t > 0.008856 {
            t.cbrt()
        } else {
            (7.787 * t) + (16.0 / 116.0)
        }
    };

    let fx = f(x / ref_x);
    let fy = f(y / ref_y);
    let fz = f(z / ref_z);

    let l = (116.0 * fy) - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);

    (l, a, b)
}

/// Lab to RGB conversion.
///
/// Returns linear (R, G, B) values.
pub fn lab_to_rgb(l: Scalar, a: Scalar, b: Scalar) -> (Scalar, Scalar, Scalar) {
    // D65 reference white
    let ref_x = 0.95047;
    let ref_y = 1.00000;
    let ref_z = 1.08883;

    let fy = (l + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;

    let f_inv = |t: Scalar| -> Scalar {
        let t3 = t * t * t;
        if t3 > 0.008856 {
            t3
        } else {
            (t - 16.0 / 116.0) / 7.787
        }
    };

    let x = ref_x * f_inv(fx);
    let y = ref_y * f_inv(fy);
    let z = ref_z * f_inv(fz);

    xyz_to_rgb(x, y, z)
}

/// Calculate the perceived luminance of an sRGB color.
///
/// Returns a value in [0, 1] representing the relative luminance.
/// This is useful for contrast calculations.
pub fn luminance(color: Color) -> Scalar {
    let r = srgb_to_linear(color.red() as Scalar / 255.0);
    let g = srgb_to_linear(color.green() as Scalar / 255.0);
    let b = srgb_to_linear(color.blue() as Scalar / 255.0);

    // Rec. 709 luminance coefficients
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Calculate contrast ratio between two colors (WCAG).
///
/// Returns a value >= 1.0. Higher values indicate more contrast.
/// WCAG requires 4.5:1 for normal text, 3:1 for large text.
pub fn contrast_ratio(color1: Color, color2: Color) -> Scalar {
    let l1 = luminance(color1);
    let l2 = luminance(color2);

    let lighter = l1.max(l2);
    let darker = l1.min(l2);

    (lighter + 0.05) / (darker + 0.05)
}

/// Mix two colors in linear space with a given ratio.
///
/// `t` of 0.0 returns `color1`, `t` of 1.0 returns `color2`.
pub fn mix_colors(color1: Color, color2: Color, t: Scalar) -> Color {
    let c1 = color_to_linear(color1);
    let c2 = color_to_linear(color2);

    let mixed = Color4f {
        r: c1.r + (c2.r - c1.r) * t,
        g: c1.g + (c2.g - c1.g) * t,
        b: c1.b + (c2.b - c1.b) * t,
        a: c1.a + (c2.a - c1.a) * t,
    };

    linear_to_color(&mixed)
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
    fn test_color_from_css_hex() {
        assert_eq!(Color::from_css("#f00"), Some(Color::from_rgb(255, 0, 0)));
        assert_eq!(
            Color::from_css("#ff0000"),
            Some(Color::from_rgb(255, 0, 0))
        );
        assert_eq!(
            Color::from_css("#ff0000ff"),
            Some(Color::from_argb(255, 255, 0, 0))
        );
        // 4-digit hex with alpha: #fabc = ff aa bb cc
        assert_eq!(
            Color::from_css("#fabc"),
            Some(Color::from_argb(0xcc, 0xff, 0xaa, 0xbb))
        );
    }

    #[test]
    fn test_color_from_css_named() {
        assert_eq!(Color::from_css("red"), Some(Color::from_rgb(255, 0, 0)));
        assert_eq!(
            Color::from_css("transparent"),
            Some(Color::from_argb(0, 0, 0, 0))
        );
        assert_eq!(
            Color::from_css("aliceblue"),
            Some(Color::from_rgb(240, 248, 255))
        );
        assert_eq!(Color::from_css("lime"), Some(Color::from_rgb(0, 255, 0)));
        // Test aliases
        assert_eq!(Color::from_css("cyan"), Some(Color::from_rgb(0, 255, 255)));
        assert_eq!(Color::from_css("aqua"), Some(Color::from_rgb(0, 255, 255)));
    }

    #[test]
    fn test_color_from_css_rgb_function() {
        assert_eq!(
            Color::from_css("rgb(255, 0, 0)"),
            Some(Color::from_rgb(255, 0, 0))
        );
        assert_eq!(
            Color::from_css("rgba(255, 0, 0, 0.5)"),
            Some(Color::from_argb(128, 255, 0, 0))
        );
    }

    #[test]
    fn test_color_from_css_hsl_function() {
        // hsl(0, 100%, 50%) = red
        assert_eq!(
            Color::from_css("hsl(0, 100%, 50%)"),
            Some(Color::from_rgb(255, 0, 0))
        );
        // hsl(120, 100%, 50%) = green
        assert_eq!(
            Color::from_css("hsl(120, 100%, 50%)"),
            Some(Color::from_rgb(0, 255, 0))
        );
        // hsl(240, 100%, 50%) = blue
        assert_eq!(
            Color::from_css("hsl(240, 100%, 50%)"),
            Some(Color::from_rgb(0, 0, 255))
        );
    }

    #[test]
    fn test_color_from_css_returns_none_for_invalid() {
        assert!(Color::from_css("banana").is_none());
        assert!(Color::from_css("#gg0000").is_none());
        assert!(Color::from_css("").is_none());
        assert!(Color::from_css("rgb(999, 0, 0)").is_none());
    }

    #[test]
    fn test_color_from_css_named_case_insensitive() {
        // Named-color lookup must be case-insensitive without allocating.
        assert_eq!(Color::from_css("RED"), Color::from_css("red"));
        assert_eq!(Color::from_css("Red"), Color::from_css("red"));
        assert_eq!(Color::from_css("AliceBlue"), Color::from_css("aliceblue"));
        assert_eq!(Color::from_css("TRANSPARENT"), Color::from_css("transparent"));
        assert_eq!(Color::from_css("WHITE"), Color::from_css("white"));
        assert_eq!(Color::from_css("BLACK"), Color::from_css("black"));
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

    #[test]
    fn test_srgb_linear_roundtrip() {
        // Test roundtrip conversion
        for i in 0..=100 {
            let s = i as f32 / 100.0;
            let linear = srgb_to_linear(s);
            let back = linear_to_srgb(linear);
            assert!((s - back).abs() < 0.0001, "Roundtrip failed for {}", s);
        }
    }

    #[test]
    fn test_srgb_linear_values() {
        // Black stays black
        assert_eq!(srgb_to_linear(0.0), 0.0);
        // White stays white
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 0.0001);
        // Middle gray is darker in linear
        let mid = srgb_to_linear(0.5);
        assert!(mid < 0.5);
        assert!(mid > 0.1);
    }

    #[test]
    fn test_hsl_roundtrip() {
        let test_cases = [
            (1.0, 0.0, 0.0), // Red
            (0.0, 1.0, 0.0), // Green
            (0.0, 0.0, 1.0), // Blue
            (0.5, 0.5, 0.5), // Gray
        ];

        for (r, g, b) in test_cases {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let (r2, g2, b2) = hsl_to_rgb(h, s, l);
            assert!((r - r2).abs() < 0.001);
            assert!((g - g2).abs() < 0.001);
            assert!((b - b2).abs() < 0.001);
        }
    }

    #[test]
    fn test_hsv_roundtrip() {
        let test_cases = [
            (1.0, 0.0, 0.0), // Red
            (0.0, 1.0, 0.0), // Green
            (0.0, 0.0, 1.0), // Blue
            (0.5, 0.5, 0.5), // Gray
        ];

        for (r, g, b) in test_cases {
            let (h, s, v) = rgb_to_hsv(r, g, b);
            let (r2, g2, b2) = hsv_to_rgb(h, s, v);
            assert!((r - r2).abs() < 0.001);
            assert!((g - g2).abs() < 0.001);
            assert!((b - b2).abs() < 0.001);
        }
    }

    #[test]
    fn test_luminance() {
        // White has luminance 1.0
        assert!((luminance(Color::WHITE) - 1.0).abs() < 0.001);
        // Black has luminance 0.0
        assert!(luminance(Color::BLACK).abs() < 0.001);
        // Green is brighter than red (for same intensity)
        assert!(luminance(Color::GREEN) > luminance(Color::RED));
    }

    #[test]
    fn test_contrast_ratio() {
        // White and black have maximum contrast
        let ratio = contrast_ratio(Color::WHITE, Color::BLACK);
        assert!(ratio > 20.0);

        // Same color has ratio of 1.0
        let same = contrast_ratio(Color::RED, Color::RED);
        assert!((same - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_mix_colors() {
        // Mix black and white at 50%
        let mixed = mix_colors(Color::BLACK, Color::WHITE, 0.5);
        // Should be approximately middle gray
        let gray = mixed.red();
        assert!(gray > 100 && gray < 200);
    }

    #[test]
    fn test_icc_profile_rejects_short_buffer() {
        // Anything shorter than the 128-byte header must be rejected.
        assert!(IccProfile::from_bytes(&[]).is_none());
        assert!(IccProfile::from_bytes(&[0u8; 64]).is_none());
        assert!(IccProfile::from_bytes(&[0u8; 127]).is_none());
    }

    #[test]
    fn test_icc_profile_rejects_missing_magic() {
        // 128 zero bytes has no "acsp" signature at offset 36.
        let bytes = [0u8; 128];
        assert!(IccProfile::from_bytes(&bytes).is_none());
    }

    #[test]
    fn test_icc_profile_parses_srgb_profile_correctly() {
        // Fixture is Artifex Software's sRGB ICC profile (tabulated curv
        // tag spanning [0, 0xFFFF]). The tag-table parser should recover
        // sRGB primaries and classify the curv as TransferFunction::Srgb.
        let bytes = match std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/srgb.icc"
        )) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("tests/fixtures/srgb.icc not available; skipping");
                return;
            }
        };
        let profile = IccProfile::from_bytes(&bytes).expect("should parse sRGB profile");
        assert_eq!(profile.color_space, IccColorSpace::Rgb);
        assert_eq!(profile.pcs, IccPcs::Xyz);
        assert!(
            profile.color_space().is_srgb(),
            "parsed sRGB profile should report is_srgb=true; got {:?}",
            profile.color_space(),
        );
    }

    #[test]
    fn test_icc_profile_parses_display_p3_profile() {
        // Fixture is Blender's srgb_p3d65_display.icc — P3 primaries
        // with sRGB-shaped parametric TRC (para type 3). The parser
        // should report DisplayP3 gamut and a non-sRGB color space.
        let bytes = match std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/display_p3.icc"
        )) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("tests/fixtures/display_p3.icc not available; skipping");
                return;
            }
        };
        let profile = IccProfile::from_bytes(&bytes).expect("should parse P3 profile");
        assert_eq!(profile.color_space, IccColorSpace::Rgb);
        assert!(
            !profile.color_space().is_srgb(),
            "Display P3 must not report is_srgb; got {:?}",
            profile.color_space(),
        );
        assert_eq!(
            profile.color_space().gamut,
            ColorGamut::DisplayP3,
            "Display P3 primaries must be classified as ColorGamut::DisplayP3",
        );
    }

    // CSS Color Level 4 tests

    #[test]
    fn test_color_from_css_oklab_white() {
        // oklab(1 0 0) = white
        let c = Color::from_css("oklab(1 0 0)").unwrap();
        assert!(c.red() >= 250, "r={}", c.red());
        assert!(c.green() >= 250, "g={}", c.green());
        assert!(c.blue() >= 250, "b={}", c.blue());
    }

    #[test]
    fn test_color_from_css_oklab_black() {
        // oklab(0 0 0) = black
        let c = Color::from_css("oklab(0 0 0)").unwrap();
        assert!(c.red() < 10, "r={}", c.red());
        assert!(c.green() < 10, "g={}", c.green());
        assert!(c.blue() < 10, "b={}", c.blue());
    }

    #[test]
    fn test_color_from_css_oklab_red_is_red_ish() {
        // oklab(0.628 0.2256 0.1257) ≈ sRGB red
        let c = Color::from_css("oklab(0.628 0.2256 0.1257)").unwrap();
        assert!(c.red() > 200, "r={}", c.red());
        assert!(c.green() < 50, "g={}", c.green());
        assert!(c.blue() < 50, "b={}", c.blue());
    }

    #[test]
    fn test_color_from_css_oklch_equivalent_to_oklab() {
        // oklch(0.5 0.1 0) = oklab(0.5 0.1 0)
        let c1 = Color::from_css("oklch(0.5 0.1 0)").unwrap();
        let c2 = Color::from_css("oklab(0.5 0.1 0)").unwrap();
        assert!((c1.red() as i16 - c2.red() as i16).abs() <= 1);
        assert!((c1.green() as i16 - c2.green() as i16).abs() <= 1);
        assert!((c1.blue() as i16 - c2.blue() as i16).abs() <= 1);
    }

    #[test]
    fn test_color_from_css_lab_black() {
        let c = Color::from_css("lab(0 0 0)").unwrap();
        assert!(c.red() < 10, "r={}", c.red());
        assert!(c.green() < 10, "g={}", c.green());
        assert!(c.blue() < 10, "b={}", c.blue());
    }

    #[test]
    fn test_color_from_css_lab_white() {
        let c = Color::from_css("lab(100 0 0)").unwrap();
        assert!(c.red() >= 250, "r={}", c.red());
        assert!(c.green() >= 250, "g={}", c.green());
        assert!(c.blue() >= 250, "b={}", c.blue());
    }

    #[test]
    fn test_color_from_css_lch_equivalent_to_lab() {
        // lch(50 50 0) = lab(50 50 0)
        let c1 = Color::from_css("lch(50 50 0)").unwrap();
        let c2 = Color::from_css("lab(50 50 0)").unwrap();
        assert!((c1.red() as i16 - c2.red() as i16).abs() <= 1);
        assert!((c1.green() as i16 - c2.green() as i16).abs() <= 1);
        assert!((c1.blue() as i16 - c2.blue() as i16).abs() <= 1);
    }

    #[test]
    fn test_color_from_css_hwb_red() {
        // hwb(0 0% 0%) = pure red
        let c = Color::from_css("hwb(0 0% 0%)").unwrap();
        assert!(c.red() >= 250, "r={}", c.red());
        assert!(c.green() < 10, "g={}", c.green());
        assert!(c.blue() < 10, "b={}", c.blue());
    }

    #[test]
    fn test_color_from_css_hwb_white() {
        // hwb(0 100% 0%) = white
        let c = Color::from_css("hwb(0 100% 0%)").unwrap();
        assert!(c.red() >= 250, "r={}", c.red());
        assert!(c.green() >= 250, "g={}", c.green());
        assert!(c.blue() >= 250, "b={}", c.blue());
    }

    #[test]
    fn test_color_from_css_hwb_black() {
        // hwb(0 0% 100%) = black
        let c = Color::from_css("hwb(0 0% 100%)").unwrap();
        assert!(c.red() < 10, "r={}", c.red());
        assert!(c.green() < 10, "g={}", c.green());
        assert!(c.blue() < 10, "b={}", c.blue());
    }

    #[test]
    fn test_color_from_css_color_function_srgb() {
        // color(srgb 1 0 0) = red
        let c = Color::from_css("color(srgb 1 0 0)").unwrap();
        assert!(c.red() >= 250, "r={}", c.red());
        assert!(c.green() < 10, "g={}", c.green());
        assert!(c.blue() < 10, "b={}", c.blue());
    }

    #[test]
    fn test_color_from_css_color_function_srgb_linear() {
        // color(srgb-linear 1 0 0) = red in linear space
        let c = Color::from_css("color(srgb-linear 1 0 0)").unwrap();
        assert!(c.red() >= 250, "r={}", c.red());
        assert!(c.green() < 10, "g={}", c.green());
        assert!(c.blue() < 10, "b={}", c.blue());
    }

    #[test]
    fn test_color_from_css_color_function_display_p3() {
        // color(display-p3 1 0 0) = P3 red (slightly outside sRGB)
        let c = Color::from_css("color(display-p3 1 0 0)").unwrap();
        // Should clamp to sRGB red
        assert!(c.red() >= 250, "r={}", c.red());
    }

    #[test]
    fn test_color_from_css_alpha_slash_syntax() {
        // oklab(1 0 0 / 0.5) = half-alpha white
        let c = Color::from_css("oklab(1 0 0 / 0.5)").unwrap();
        assert!(c.alpha() > 120 && c.alpha() < 135, "a={}", c.alpha());
    }

    #[test]
    fn test_color_from_css_alpha_percentage() {
        // oklab(1 0 0 / 50%) = half-alpha white
        let c = Color::from_css("oklab(1 0 0 / 50%)").unwrap();
        assert!(c.alpha() > 120 && c.alpha() < 135, "a={}", c.alpha());
    }

    #[test]
    fn test_color_from_css_oklab_percentage_lightness() {
        // oklab(100% 0 0) = white
        let c = Color::from_css("oklab(100% 0 0)").unwrap();
        assert!(c.red() >= 250, "r={}", c.red());
        assert!(c.green() >= 250, "g={}", c.green());
        assert!(c.blue() >= 250, "b={}", c.blue());
    }

    #[test]
    fn test_color_from_css_level4_comma_syntax() {
        // Support legacy comma syntax: oklab(1, 0, 0)
        let c = Color::from_css("oklab(1, 0, 0)").unwrap();
        assert!(c.red() >= 250, "r={}", c.red());
    }

    #[test]
    fn test_color_from_css_unsupported_color_space_returns_none() {
        // rec2020 and prophoto-rgb not supported
        assert!(Color::from_css("color(rec2020 1 0 0)").is_none());
        assert!(Color::from_css("color(prophoto-rgb 1 0 0)").is_none());
    }
}
