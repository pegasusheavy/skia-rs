//! Font configuration for text rendering.

use crate::typeface::{Typeface, TypefaceRef};
use skia_rs_core::Scalar;
use std::sync::Arc;

/// Text baseline position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TextBaseline {
    /// Alphabetic baseline.
    #[default]
    Alphabetic = 0,
    /// Top of the em square.
    Top,
    /// Middle of the em square.
    Middle,
    /// Bottom of the em square.
    Bottom,
    /// Ideographic baseline.
    Ideographic,
    /// Hanging baseline.
    Hanging,
}

/// Font edging mode (how glyphs are rendered).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FontEdging {
    /// Alias (no anti-aliasing).
    Alias = 0,
    /// Anti-aliased.
    #[default]
    AntiAlias,
    /// Subpixel anti-aliased.
    SubpixelAntiAlias,
}

/// Font hinting level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FontHinting {
    /// No hinting.
    None = 0,
    /// Slight hinting.
    Slight,
    /// Normal hinting.
    #[default]
    Normal,
    /// Full hinting.
    Full,
}

/// Font metrics.
#[derive(Debug, Clone, Copy, Default)]
pub struct FontMetrics {
    /// Distance above baseline (negative for above).
    pub ascent: Scalar,
    /// Distance below baseline (positive for below).
    pub descent: Scalar,
    /// Distance between baselines.
    pub leading: Scalar,
    /// Top of bounding box (negative for above baseline).
    pub top: Scalar,
    /// Bottom of bounding box.
    pub bottom: Scalar,
    /// Average character width.
    pub avg_char_width: Scalar,
    /// Maximum character width.
    pub max_char_width: Scalar,
    /// X-height (height of lowercase 'x').
    pub x_height: Scalar,
    /// Cap height (height of uppercase letters).
    pub cap_height: Scalar,
    /// Underline position.
    pub underline_position: Scalar,
    /// Underline thickness.
    pub underline_thickness: Scalar,
    /// Strikeout position.
    pub strikeout_position: Scalar,
    /// Strikeout thickness.
    pub strikeout_thickness: Scalar,
}

impl FontMetrics {
    /// Calculate the line height.
    #[inline]
    pub fn line_height(&self) -> Scalar {
        -self.ascent + self.descent + self.leading
    }
}

/// A font configuration (typeface + size + options).
///
/// Corresponds to Skia's `SkFont`.
#[derive(Debug, Clone)]
pub struct Font {
    /// The typeface.
    typeface: TypefaceRef,
    /// Font size in points.
    size: Scalar,
    /// Horizontal scale factor.
    scale_x: Scalar,
    /// Skew factor for oblique simulation.
    skew_x: Scalar,
    /// Font edging mode.
    edging: FontEdging,
    /// Font hinting level.
    hinting: FontHinting,
    /// Enable subpixel positioning.
    subpixel: bool,
    /// Force auto-hinting.
    force_auto_hinting: bool,
    /// Embed bitmaps in outlines.
    embedded_bitmaps: bool,
    /// Enable linear metrics.
    linear_metrics: bool,
    /// Embolden the font.
    embolden: bool,
}

impl Default for Font {
    fn default() -> Self {
        Self::new(Arc::new(Typeface::default_typeface()), 12.0)
    }
}

impl Font {
    /// Create a new font with the given typeface and size.
    pub fn new(typeface: TypefaceRef, size: Scalar) -> Self {
        Self {
            typeface,
            size,
            scale_x: 1.0,
            skew_x: 0.0,
            edging: FontEdging::AntiAlias,
            hinting: FontHinting::Normal,
            subpixel: false,
            force_auto_hinting: false,
            embedded_bitmaps: true,
            linear_metrics: false,
            embolden: false,
        }
    }

    /// Create a font with default typeface.
    pub fn from_size(size: Scalar) -> Self {
        Self::new(Arc::new(Typeface::default_typeface()), size)
    }

    /// Get the typeface.
    #[inline]
    pub fn typeface(&self) -> Option<&Typeface> {
        Some(self.typeface.as_ref())
    }

    /// Get the typeface reference.
    #[inline]
    pub fn typeface_ref(&self) -> &TypefaceRef {
        &self.typeface
    }

    /// Set the typeface.
    #[inline]
    pub fn set_typeface(&mut self, typeface: TypefaceRef) -> &mut Self {
        self.typeface = typeface;
        self
    }

    /// Get the font size.
    #[inline]
    pub fn size(&self) -> Scalar {
        self.size
    }

    /// Set the font size.
    #[inline]
    pub fn set_size(&mut self, size: Scalar) -> &mut Self {
        self.size = size.max(0.0);
        self
    }

    /// Get the horizontal scale.
    #[inline]
    pub fn scale_x(&self) -> Scalar {
        self.scale_x
    }

    /// Set the horizontal scale.
    #[inline]
    pub fn set_scale_x(&mut self, scale: Scalar) -> &mut Self {
        self.scale_x = scale;
        self
    }

    /// Get the skew factor.
    #[inline]
    pub fn skew_x(&self) -> Scalar {
        self.skew_x
    }

    /// Set the skew factor.
    #[inline]
    pub fn set_skew_x(&mut self, skew: Scalar) -> &mut Self {
        self.skew_x = skew;
        self
    }

    /// Get the edging mode.
    #[inline]
    pub fn edging(&self) -> FontEdging {
        self.edging
    }

    /// Set the edging mode.
    #[inline]
    pub fn set_edging(&mut self, edging: FontEdging) -> &mut Self {
        self.edging = edging;
        self
    }

    /// Get the hinting level.
    #[inline]
    pub fn hinting(&self) -> FontHinting {
        self.hinting
    }

    /// Set the hinting level.
    #[inline]
    pub fn set_hinting(&mut self, hinting: FontHinting) -> &mut Self {
        self.hinting = hinting;
        self
    }

    /// Check if subpixel positioning is enabled.
    #[inline]
    pub fn is_subpixel(&self) -> bool {
        self.subpixel
    }

    /// Set subpixel positioning.
    #[inline]
    pub fn set_subpixel(&mut self, subpixel: bool) -> &mut Self {
        self.subpixel = subpixel;
        self
    }

    /// Check if emboldening is enabled.
    #[inline]
    pub fn is_embolden(&self) -> bool {
        self.embolden
    }

    /// Set emboldening.
    #[inline]
    pub fn set_embolden(&mut self, embolden: bool) -> &mut Self {
        self.embolden = embolden;
        self
    }

    /// Get the font metrics.
    pub fn metrics(&self) -> FontMetrics {
        // Calculate metrics based on size and typeface
        let scale = self.size / self.typeface.units_per_em() as Scalar;

        FontMetrics {
            ascent: -0.8 * self.size, // Approximate
            descent: 0.2 * self.size,
            leading: 0.0,
            top: -0.9 * self.size,
            bottom: 0.3 * self.size,
            avg_char_width: 0.5 * self.size,
            max_char_width: self.size,
            x_height: 0.5 * self.size,
            cap_height: 0.7 * self.size,
            underline_position: 0.1 * self.size,
            underline_thickness: 0.05 * self.size,
            strikeout_position: -0.3 * self.size,
            strikeout_thickness: 0.05 * self.size,
        }
    }

    /// Get spacing between baselines.
    #[inline]
    pub fn spacing(&self) -> Scalar {
        let m = self.metrics();
        m.line_height()
    }

    /// Get the ascent (negative value, distance from baseline to top).
    #[inline]
    pub fn ascent(&self) -> Scalar {
        self.metrics().ascent
    }

    /// Get the descent (positive value, distance from baseline to bottom).
    #[inline]
    pub fn descent(&self) -> Scalar {
        self.metrics().descent
    }

    /// Measure the width of text.
    ///
    /// Sums `glyph_advance` across each character's glyph so that fonts with
    /// real `hmtx` data measure correctly. Falls back to the crude
    /// `size * 0.5 * char_count` estimate for the dataless default typeface.
    pub fn measure_text(&self, text: &str) -> Scalar {
        text.chars()
            .map(|c| self.glyph_advance(self.char_to_glyph(c)))
            .sum()
    }

    /// Get glyph widths for text.
    pub fn get_widths(&self, text: &str) -> Vec<Scalar> {
        // Simple approximation
        let width = self.size * 0.5 * self.scale_x;
        text.chars().map(|_| width).collect()
    }

    /// Get glyph bounds for text.
    pub fn get_bounds(&self, text: &str) -> Vec<skia_rs_core::Rect> {
        let width = self.size * 0.5 * self.scale_x;
        let metrics = self.metrics();

        text.chars()
            .enumerate()
            .map(|(i, _)| {
                skia_rs_core::Rect::from_xywh(
                    i as Scalar * width,
                    metrics.ascent,
                    width,
                    -metrics.ascent + metrics.descent,
                )
            })
            .collect()
    }

    /// Convert character to glyph ID.
    #[inline]
    pub fn char_to_glyph(&self, c: char) -> u16 {
        self.typeface.char_to_glyph(c)
    }

    /// Convert string to glyph IDs.
    #[inline]
    pub fn text_to_glyphs(&self, text: &str) -> Vec<u16> {
        self.typeface.chars_to_glyphs(text)
    }

    // =========================================================================
    // Glyph Operations
    // =========================================================================

    /// Get the advance width for a glyph.
    ///
    /// The advance is the horizontal distance to move after drawing this
    /// glyph. When font data is available the value comes from the font's
    /// `hmtx` table scaled by `size / units_per_em`; otherwise a crude
    /// `size * 0.5` approximation is used for the dataless default
    /// typeface.
    pub fn glyph_advance(&self, glyph: u16) -> Scalar {
        if glyph == 0 {
            return 0.0;
        }
        if let Some(data) = self.typeface.font_data() {
            if let Ok(face) = ttf_parser::Face::parse(data, 0) {
                let upem = face.units_per_em();
                if upem > 0 {
                    if let Some(adv) = face.glyph_hor_advance(ttf_parser::GlyphId(glyph)) {
                        return adv as Scalar * self.size / upem as Scalar * self.scale_x;
                    }
                }
            }
        }
        self.size * 0.5 * self.scale_x
    }

    /// Get advance widths for multiple glyphs.
    pub fn glyph_advances(&self, glyphs: &[u16]) -> Vec<Scalar> {
        glyphs.iter().map(|&g| self.glyph_advance(g)).collect()
    }

    /// Get the bounding box for a glyph.
    ///
    /// Returns the tight bounds around the glyph's visible pixels.
    pub fn glyph_bounds(&self, glyph: u16) -> skia_rs_core::Rect {
        if glyph == 0 {
            return skia_rs_core::Rect::EMPTY;
        }

        let advance = self.glyph_advance(glyph);
        let metrics = self.metrics();

        skia_rs_core::Rect::from_xywh(
            0.0,
            metrics.ascent,
            advance,
            -metrics.ascent + metrics.descent,
        )
    }

    /// Get bounding boxes for multiple glyphs.
    pub fn glyph_bounds_batch(&self, glyphs: &[u16]) -> Vec<skia_rs_core::Rect> {
        glyphs.iter().map(|&g| self.glyph_bounds(g)).collect()
    }

    /// Get the path outline for a glyph.
    ///
    /// Returns a path that can be filled to render the glyph. The path is in
    /// local coordinates with the glyph origin at (0, 0), scaled by
    /// `size / units_per_em`, and y-flipped from font-space (y-up) into
    /// screen-space (y-down) so that drawing it at the baseline position
    /// produces correctly oriented glyphs.
    ///
    /// Returns `None` if:
    /// - `glyph` is 0 (.notdef)
    /// - the underlying typeface has no font data (e.g. the default typeface)
    /// - the font data fails to parse
    /// - the glyph has no outline (e.g. space, non-printing characters)
    pub fn glyph_path(&self, glyph: u16) -> Option<skia_rs_path::Path> {
        if glyph == 0 {
            return None;
        }

        let data = self.typeface.font_data()?;
        let face = ttf_parser::Face::parse(data, 0).ok()?;

        let upem = face.units_per_em();
        if upem == 0 {
            return None;
        }
        let scale = self.size / upem as Scalar;

        let mut builder = GlyphOutlineBuilder {
            builder: skia_rs_path::PathBuilder::new(),
            scale,
        };
        face.outline_glyph(ttf_parser::GlyphId(glyph), &mut builder)?;
        Some(builder.builder.build())
    }

    /// Get paths for multiple glyphs.
    pub fn glyph_paths(&self, glyphs: &[u16]) -> Vec<Option<skia_rs_path::Path>> {
        glyphs.iter().map(|&g| self.glyph_path(g)).collect()
    }

    /// Get the path for a string of text.
    ///
    /// The returned path contains all glyph outlines positioned correctly.
    pub fn text_path(&self, text: &str) -> skia_rs_path::Path {
        let mut builder = skia_rs_path::PathBuilder::new();
        let glyphs = self.text_to_glyphs(text);
        let mut x_offset: Scalar = 0.0;

        for glyph in glyphs {
            if let Some(glyph_path) = self.glyph_path(glyph) {
                // Transform and add glyph path
                let transform = skia_rs_core::Matrix::translate(x_offset, 0.0);
                let transformed = glyph_path.transformed(&transform);
                builder.add_path(&transformed);
            }
            x_offset += self.glyph_advance(glyph);
        }

        builder.build()
    }

    /// Check if a glyph is a color/emoji glyph.
    ///
    /// Color glyphs require special rendering (as images rather than outlines).
    pub fn glyph_is_color(&self, glyph: u16) -> bool {
        // Placeholder - real implementation would check font tables (COLR/CPAL or CBDT/CBLC)
        // For now, assume high glyph IDs might be emoji
        glyph > 0x1000
    }

    /// Get the image for a color glyph (emoji).
    ///
    /// Returns the pixel data and size for rendering emoji and other color glyphs.
    pub fn glyph_image(&self, glyph: u16) -> Option<GlyphImage> {
        if !self.glyph_is_color(glyph) {
            return None;
        }

        // Placeholder - returns a simple colored rectangle
        // Real implementation would extract the actual glyph image from CBDT/CBLC or SVG tables
        let size = (self.size * 2.0).ceil() as i32;
        let mut pixels = vec![0u8; (size * size * 4) as usize];

        // Fill with a placeholder color
        for y in 0..size {
            for x in 0..size {
                let offset = ((y * size + x) * 4) as usize;
                pixels[offset] = 255; // R
                pixels[offset + 1] = 200; // G
                pixels[offset + 2] = 0; // B (yellow-ish for emoji placeholder)
                pixels[offset + 3] = 255; // A
            }
        }

        Some(GlyphImage {
            width: size,
            height: size,
            pixels,
            left: 0.0,
            top: -self.size * 0.8,
        })
    }

    /// Get positioning information for a run of glyphs.
    pub fn glyph_positions(
        &self,
        glyphs: &[u16],
        start: skia_rs_core::Point,
    ) -> Vec<skia_rs_core::Point> {
        let mut positions = Vec::with_capacity(glyphs.len());
        let mut x = start.x;
        let y = start.y;

        for &glyph in glyphs {
            positions.push(skia_rs_core::Point::new(x, y));
            x += self.glyph_advance(glyph);
        }

        positions
    }

    /// Get the intercepts (horizontal line intersections) for glyph outlines.
    ///
    /// Used for text decoration positioning (underline, strikethrough).
    pub fn glyph_intercepts(
        &self,
        glyphs: &[u16],
        positions: &[skia_rs_core::Point],
        top: Scalar,
        bottom: Scalar,
    ) -> Vec<Scalar> {
        // Placeholder - returns approximated intercepts
        // Real implementation would intersect glyph paths with the horizontal band
        let mut intercepts = Vec::new();

        for (i, &glyph) in glyphs.iter().enumerate() {
            if glyph == 0 {
                continue;
            }

            let pos = positions.get(i).copied().unwrap_or_default();
            let bounds = self.glyph_bounds(glyph);

            // Check if glyph intersects the band
            let glyph_top = pos.y + bounds.top;
            let glyph_bottom = pos.y + bounds.bottom;

            if glyph_bottom >= top && glyph_top <= bottom {
                intercepts.push(pos.x);
                intercepts.push(pos.x + bounds.width());
            }
        }

        intercepts
    }
}

/// Adapts `ttf_parser::OutlineBuilder` onto `skia_rs_path::PathBuilder`.
///
/// Fonts store outlines with y-axis pointing up; our Path lives in screen
/// space (y-down). The builder flips `y` and scales both axes by
/// `size / units_per_em` so the produced path is already in pixel units
/// relative to the glyph origin.
struct GlyphOutlineBuilder {
    builder: skia_rs_path::PathBuilder,
    scale: Scalar,
}

impl ttf_parser::OutlineBuilder for GlyphOutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(x * self.scale, -y * self.scale);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(x * self.scale, -y * self.scale);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quad_to(
            x1 * self.scale,
            -y1 * self.scale,
            x * self.scale,
            -y * self.scale,
        );
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_to(
            x1 * self.scale,
            -y1 * self.scale,
            x2 * self.scale,
            -y2 * self.scale,
            x * self.scale,
            -y * self.scale,
        );
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

/// Image data for a color glyph (emoji).
#[derive(Debug, Clone)]
pub struct GlyphImage {
    /// Image width in pixels.
    pub width: i32,
    /// Image height in pixels.
    pub height: i32,
    /// RGBA pixel data.
    pub pixels: Vec<u8>,
    /// Left offset from glyph origin.
    pub left: Scalar,
    /// Top offset from glyph origin (typically negative).
    pub top: Scalar,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_default() {
        let font = Font::default();
        assert_eq!(font.size(), 12.0);
        assert_eq!(font.scale_x(), 1.0);
    }

    #[test]
    fn test_font_from_size() {
        let font = Font::from_size(24.0);
        assert_eq!(font.size(), 24.0);
    }

    #[test]
    fn test_font_measure_text() {
        let font = Font::from_size(20.0);
        let width = font.measure_text("Hello");
        assert!(width > 0.0);
    }

    #[test]
    fn test_font_metrics() {
        let font = Font::from_size(16.0);
        let metrics = font.metrics();
        assert!(metrics.ascent < 0.0); // Above baseline
        assert!(metrics.descent > 0.0); // Below baseline
    }
}
