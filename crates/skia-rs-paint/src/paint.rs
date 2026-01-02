//! Paint structure for drawing configuration.

use crate::blend::BlendMode;
use skia_rs_core::{Color, Color4f, Scalar};

/// Paint style (fill, stroke, or both).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Style {
    /// Fill the shape.
    #[default]
    Fill = 0,
    /// Stroke the outline.
    Stroke,
    /// Both fill and stroke.
    StrokeAndFill,
}

/// Stroke cap style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum StrokeCap {
    /// Flat cap.
    #[default]
    Butt = 0,
    /// Round cap.
    Round,
    /// Square cap (extends by half stroke width).
    Square,
}

/// Stroke join style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum StrokeJoin {
    /// Miter join.
    #[default]
    Miter = 0,
    /// Round join.
    Round,
    /// Bevel join.
    Bevel,
}

/// Paint configuration for drawing operations.
#[derive(Debug, Clone)]
pub struct Paint {
    /// Fill color.
    color: Color4f,
    /// Blend mode.
    blend_mode: BlendMode,
    /// Style (fill/stroke).
    style: Style,
    /// Stroke width.
    stroke_width: Scalar,
    /// Stroke miter limit.
    stroke_miter: Scalar,
    /// Stroke cap.
    stroke_cap: StrokeCap,
    /// Stroke join.
    stroke_join: StrokeJoin,
    /// Anti-aliasing enabled.
    anti_alias: bool,
    /// Dithering enabled.
    dither: bool,
}

impl Default for Paint {
    fn default() -> Self {
        Self {
            color: Color4f::new(0.0, 0.0, 0.0, 1.0),
            blend_mode: BlendMode::SrcOver,
            style: Style::Fill,
            stroke_width: 1.0,
            stroke_miter: 4.0,
            stroke_cap: StrokeCap::Butt,
            stroke_join: StrokeJoin::Miter,
            anti_alias: true,
            dither: false,
        }
    }
}

impl Paint {
    /// Create a new paint with default settings.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the color as Color4f.
    #[inline]
    pub fn color(&self) -> Color4f {
        self.color
    }

    /// Get the color as 32-bit Color.
    #[inline]
    pub fn color32(&self) -> Color {
        Color::from_argb(
            (self.color.a * 255.0).clamp(0.0, 255.0) as u8,
            (self.color.r * 255.0).clamp(0.0, 255.0) as u8,
            (self.color.g * 255.0).clamp(0.0, 255.0) as u8,
            (self.color.b * 255.0).clamp(0.0, 255.0) as u8,
        )
    }

    /// Set the color from Color4f.
    #[inline]
    pub fn set_color(&mut self, color: Color4f) -> &mut Self {
        self.color = color;
        self
    }

    /// Set the color from 32-bit Color.
    #[inline]
    pub fn set_color32(&mut self, color: Color) -> &mut Self {
        self.color = color.to_color4f();
        self
    }

    /// Set ARGB components.
    #[inline]
    pub fn set_argb(&mut self, a: u8, r: u8, g: u8, b: u8) -> &mut Self {
        self.set_color32(Color::from_argb(a, r, g, b))
    }

    /// Get the alpha value (0.0-1.0).
    #[inline]
    pub fn alpha(&self) -> Scalar {
        self.color.a
    }

    /// Set the alpha value (0.0-1.0).
    #[inline]
    pub fn set_alpha(&mut self, alpha: Scalar) -> &mut Self {
        self.color.a = alpha;
        self
    }

    /// Get the blend mode.
    #[inline]
    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    /// Set the blend mode.
    #[inline]
    pub fn set_blend_mode(&mut self, mode: BlendMode) -> &mut Self {
        self.blend_mode = mode;
        self
    }

    /// Get the style.
    #[inline]
    pub fn style(&self) -> Style {
        self.style
    }

    /// Set the style.
    #[inline]
    pub fn set_style(&mut self, style: Style) -> &mut Self {
        self.style = style;
        self
    }

    /// Get the stroke width.
    #[inline]
    pub fn stroke_width(&self) -> Scalar {
        self.stroke_width
    }

    /// Set the stroke width.
    #[inline]
    pub fn set_stroke_width(&mut self, width: Scalar) -> &mut Self {
        self.stroke_width = width.max(0.0);
        self
    }

    /// Get the stroke miter limit.
    #[inline]
    pub fn stroke_miter(&self) -> Scalar {
        self.stroke_miter
    }

    /// Set the stroke miter limit.
    #[inline]
    pub fn set_stroke_miter(&mut self, miter: Scalar) -> &mut Self {
        self.stroke_miter = miter.max(0.0);
        self
    }

    /// Get the stroke cap.
    #[inline]
    pub fn stroke_cap(&self) -> StrokeCap {
        self.stroke_cap
    }

    /// Set the stroke cap.
    #[inline]
    pub fn set_stroke_cap(&mut self, cap: StrokeCap) -> &mut Self {
        self.stroke_cap = cap;
        self
    }

    /// Get the stroke join.
    #[inline]
    pub fn stroke_join(&self) -> StrokeJoin {
        self.stroke_join
    }

    /// Set the stroke join.
    #[inline]
    pub fn set_stroke_join(&mut self, join: StrokeJoin) -> &mut Self {
        self.stroke_join = join;
        self
    }

    /// Check if anti-aliasing is enabled.
    #[inline]
    pub fn is_anti_alias(&self) -> bool {
        self.anti_alias
    }

    /// Set anti-aliasing.
    #[inline]
    pub fn set_anti_alias(&mut self, aa: bool) -> &mut Self {
        self.anti_alias = aa;
        self
    }

    /// Check if dithering is enabled.
    #[inline]
    pub fn is_dither(&self) -> bool {
        self.dither
    }

    /// Set dithering.
    #[inline]
    pub fn set_dither(&mut self, dither: bool) -> &mut Self {
        self.dither = dither;
        self
    }

    /// Alias for is_anti_alias (Skia compatibility).
    #[inline]
    pub fn anti_alias(&self) -> bool {
        self.anti_alias
    }

    // =========================================================================
    // Serialization
    // =========================================================================

    /// Serialize the paint to bytes.
    ///
    /// Format:
    /// - 4 bytes: RGBA color (each component 0-255)
    /// - 1 byte: blend mode
    /// - 1 byte: style
    /// - 4 bytes: stroke width (f32 little-endian)
    /// - 4 bytes: stroke miter (f32 little-endian)
    /// - 1 byte: stroke cap
    /// - 1 byte: stroke join
    /// - 1 byte: flags (bit 0: anti_alias, bit 1: dither)
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(17);

        // Color (4 bytes)
        data.push((self.color.r * 255.0).clamp(0.0, 255.0) as u8);
        data.push((self.color.g * 255.0).clamp(0.0, 255.0) as u8);
        data.push((self.color.b * 255.0).clamp(0.0, 255.0) as u8);
        data.push((self.color.a * 255.0).clamp(0.0, 255.0) as u8);

        // Blend mode (1 byte)
        data.push(self.blend_mode as u8);

        // Style (1 byte)
        data.push(self.style as u8);

        // Stroke width (4 bytes)
        data.extend_from_slice(&self.stroke_width.to_le_bytes());

        // Stroke miter (4 bytes)
        data.extend_from_slice(&self.stroke_miter.to_le_bytes());

        // Stroke cap (1 byte)
        data.push(self.stroke_cap as u8);

        // Stroke join (1 byte)
        data.push(self.stroke_join as u8);

        // Flags (1 byte)
        let mut flags = 0u8;
        if self.anti_alias {
            flags |= 0x01;
        }
        if self.dither {
            flags |= 0x02;
        }
        data.push(flags);

        data
    }

    /// Deserialize a paint from bytes.
    ///
    /// Returns `None` if the data is invalid or too short.
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 17 {
            return None;
        }

        // Color
        let r = data[0] as f32 / 255.0;
        let g = data[1] as f32 / 255.0;
        let b = data[2] as f32 / 255.0;
        let a = data[3] as f32 / 255.0;
        let color = Color4f::new(r, g, b, a);

        // Blend mode
        let blend_mode = BlendMode::from_u8(data[4])?;

        // Style
        let style = match data[5] {
            0 => Style::Fill,
            1 => Style::Stroke,
            2 => Style::StrokeAndFill,
            _ => return None,
        };

        // Stroke width
        let stroke_width = f32::from_le_bytes([data[6], data[7], data[8], data[9]]);

        // Stroke miter
        let stroke_miter = f32::from_le_bytes([data[10], data[11], data[12], data[13]]);

        // Stroke cap
        let stroke_cap = match data[14] {
            0 => StrokeCap::Butt,
            1 => StrokeCap::Round,
            2 => StrokeCap::Square,
            _ => return None,
        };

        // Stroke join
        let stroke_join = match data[15] {
            0 => StrokeJoin::Miter,
            1 => StrokeJoin::Round,
            2 => StrokeJoin::Bevel,
            _ => return None,
        };

        // Flags
        let flags = data[16];
        let anti_alias = (flags & 0x01) != 0;
        let dither = (flags & 0x02) != 0;

        Some(Self {
            color,
            blend_mode,
            style,
            stroke_width,
            stroke_miter,
            stroke_cap,
            stroke_join,
            anti_alias,
            dither,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paint_serialization() {
        let mut paint = Paint::new();
        paint
            .set_color(Color4f::new(1.0, 0.5, 0.25, 0.75))
            .set_blend_mode(BlendMode::Multiply)
            .set_style(Style::Stroke)
            .set_stroke_width(2.5)
            .set_stroke_miter(8.0)
            .set_stroke_cap(StrokeCap::Round)
            .set_stroke_join(StrokeJoin::Bevel)
            .set_anti_alias(false)
            .set_dither(true);

        let serialized = paint.serialize();
        let deserialized = Paint::deserialize(&serialized).unwrap();

        // Check round-trip
        assert!((deserialized.color().r - paint.color().r).abs() < 0.01);
        assert!((deserialized.color().g - paint.color().g).abs() < 0.01);
        assert!((deserialized.color().b - paint.color().b).abs() < 0.01);
        assert!((deserialized.color().a - paint.color().a).abs() < 0.01);
        assert_eq!(deserialized.blend_mode(), paint.blend_mode());
        assert_eq!(deserialized.style(), paint.style());
        assert!((deserialized.stroke_width() - paint.stroke_width()).abs() < 0.001);
        assert!((deserialized.stroke_miter() - paint.stroke_miter()).abs() < 0.001);
        assert_eq!(deserialized.stroke_cap(), paint.stroke_cap());
        assert_eq!(deserialized.stroke_join(), paint.stroke_join());
        assert_eq!(deserialized.is_anti_alias(), paint.is_anti_alias());
        assert_eq!(deserialized.is_dither(), paint.is_dither());
    }

    #[test]
    fn test_paint_deserialize_invalid() {
        // Too short
        assert!(Paint::deserialize(&[0; 10]).is_none());

        // Invalid blend mode
        let mut data = Paint::new().serialize();
        data[4] = 255;
        assert!(Paint::deserialize(&data).is_none());
    }
}
