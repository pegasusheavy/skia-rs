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
    ///
    /// When the typeface has backing font data, this pulls real values from
    /// the font's `hhea`/`OS/2`/`post` tables via `ttf_parser` and scales them
    /// by `size / units_per_em`. Specifically:
    ///
    /// - `ascent` ← `Face::ascender()` (negated, screen-space is y-down)
    /// - `descent` ← `Face::descender()` (negated — ttf-parser returns a
    ///   negative font-space value; we store the positive screen-space
    ///   descent so that `line_height = -ascent + descent + leading` works)
    /// - `leading` ← `Face::line_gap()`
    /// - `x_height`, `cap_height` ← OS/2 v2+ when present
    /// - `underline_position` / `underline_thickness` ← `post` table
    /// - `strikeout_position` / `strikeout_thickness` ← OS/2
    ///
    /// Falls back to the previous hardcoded multiples of `size` only for the
    /// dataless default typeface, which has no font tables to read.
    pub fn metrics(&self) -> FontMetrics {
        if let Some(data) = self.typeface.font_data() {
            if let Ok(face) = ttf_parser::Face::parse(data, 0) {
                let upem = face.units_per_em();
                if upem > 0 {
                    let scale = self.size / upem as Scalar;

                    // Font-space ascender is positive (units above baseline).
                    // Screen-space ascent is negative (y grows downward).
                    let ascent = -(face.ascender() as Scalar) * scale;
                    // Font-space descender is negative (units below baseline).
                    // Screen-space descent is positive.
                    let descent = -(face.descender() as Scalar) * scale;
                    let leading = face.line_gap() as Scalar * scale;

                    // Cap and x heights — OS/2 v2+ only; fall back to a
                    // fraction of ascent when missing.
                    let cap_height = face
                        .capital_height()
                        .map(|v| -(v as Scalar) * scale)
                        .unwrap_or(ascent * 0.875);
                    let x_height = face
                        .x_height()
                        .map(|v| -(v as Scalar) * scale)
                        .unwrap_or(ascent * 0.625);

                    // Underline from `post`. Font-space position is the
                    // *center* of the underline, measured upward from the
                    // baseline (usually negative). We want a screen-space
                    // offset from the baseline (positive = below), so negate.
                    let (underline_position, underline_thickness) = face
                        .underline_metrics()
                        .map(|lm| {
                            (
                                -(lm.position as Scalar) * scale,
                                lm.thickness as Scalar * scale,
                            )
                        })
                        .unwrap_or((0.1 * self.size, 0.05 * self.size));

                    // Strikeout from OS/2. Same convention as underline.
                    let (strikeout_position, strikeout_thickness) = face
                        .strikeout_metrics()
                        .map(|lm| {
                            (
                                -(lm.position as Scalar) * scale,
                                lm.thickness as Scalar * scale,
                            )
                        })
                        .unwrap_or((-0.3 * self.size, 0.05 * self.size));

                    // Visible bounds: match Skia's convention of including a
                    // small margin beyond ascent/descent. A number of fonts
                    // do not carry usWinAscent/Descent in a usable form from
                    // ttf-parser, so we derive bottom/top from ascent/descent
                    // with a 10 % cushion — matching Skia's fallback.
                    let top = ascent * 1.125;
                    let bottom = descent * 1.125;

                    // Average / max char width: ttf-parser does not expose
                    // xAvgCharWidth directly via a dedicated accessor, so use
                    // a reasonable estimate from measured glyph advances.
                    // `glyph_hor_advance` on gid 1 is a good-enough fallback;
                    // prefer the measured advance of 'x' if present.
                    let avg_char_width = face
                        .glyph_index('x')
                        .and_then(|g| face.glyph_hor_advance(g))
                        .map(|a| a as Scalar * scale)
                        .unwrap_or(0.5 * self.size);
                    let max_char_width = face
                        .glyph_index('M')
                        .and_then(|g| face.glyph_hor_advance(g))
                        .map(|a| a as Scalar * scale)
                        .unwrap_or(self.size);

                    return FontMetrics {
                        ascent,
                        descent,
                        leading,
                        top,
                        bottom,
                        avg_char_width,
                        max_char_width,
                        x_height,
                        cap_height,
                        underline_position,
                        underline_thickness,
                        strikeout_position,
                        strikeout_thickness,
                    };
                }
            }
        }

        // Dataless default typeface: fall back to the hardcoded
        // approximation. The default typeface has no tables to read and all
        // callers have historically relied on these numbers.
        FontMetrics {
            ascent: -0.8 * self.size,
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
    ///
    /// Returns the per-character horizontal advance. Delegates to
    /// `glyph_advance`, which reads the font's `hmtx` table when present,
    /// so this function produces correct per-character positioning for real
    /// fonts rather than the old uniform `size / 2` approximation.
    pub fn get_widths(&self, text: &str) -> Vec<Scalar> {
        text.chars()
            .map(|c| self.glyph_advance(self.char_to_glyph(c)))
            .collect()
    }

    /// Get glyph bounds for text.
    ///
    /// Returns the bounding box of each character's glyph, positioned
    /// cumulatively along x by the per-glyph advance from `hmtx`. Each box's
    /// tight visual extent is taken from `ttf_parser::Face::glyph_bounding_box`
    /// when present; otherwise we fall back to the previous ascent/descent
    /// rectangle with the measured advance as its width.
    pub fn get_bounds(&self, text: &str) -> Vec<skia_rs_core::Rect> {
        let metrics = self.metrics();
        let mut out = Vec::with_capacity(text.chars().count());

        let parsed = self
            .typeface
            .font_data()
            .and_then(|data| ttf_parser::Face::parse(data, 0).ok());

        let scale = if let Some(face) = parsed.as_ref() {
            let upem = face.units_per_em();
            if upem > 0 {
                self.size / upem as Scalar
            } else {
                1.0
            }
        } else {
            1.0
        };

        let mut x_offset: Scalar = 0.0;
        for c in text.chars() {
            let glyph = self.char_to_glyph(c);
            let advance = self.glyph_advance(glyph);

            let rect = if let Some(face) = parsed.as_ref() {
                if glyph != 0 {
                    if let Some(bbox) = face.glyph_bounding_box(ttf_parser::GlyphId(glyph)) {
                        // Font-space bbox is y-up; flip to y-down screen
                        // space (negate y and swap top/bottom).
                        let left = bbox.x_min as Scalar * scale * self.scale_x;
                        let right = bbox.x_max as Scalar * scale * self.scale_x;
                        let top = -(bbox.y_max as Scalar) * scale;
                        let bottom = -(bbox.y_min as Scalar) * scale;
                        skia_rs_core::Rect::new(
                            x_offset + left,
                            top,
                            x_offset + right,
                            bottom,
                        )
                    } else {
                        // Glyph has no outline (e.g. space). Zero-sized box.
                        skia_rs_core::Rect::from_xywh(x_offset, 0.0, 0.0, 0.0)
                    }
                } else {
                    skia_rs_core::Rect::from_xywh(x_offset, 0.0, 0.0, 0.0)
                }
            } else {
                // Dataless typeface — fall back to advance×line-height box.
                skia_rs_core::Rect::from_xywh(
                    x_offset,
                    metrics.ascent,
                    advance,
                    -metrics.ascent + metrics.descent,
                )
            };

            out.push(rect);
            x_offset += advance;
        }

        out
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
    /// Returns the tight visual bounds around the glyph's outline, read
    /// from the font's glyph bbox (via `ttf_parser::Face::glyph_bounding_box`)
    /// and converted from font-space (y-up) into screen-space (y-down) with
    /// the same `size/upem` scale used for glyph paths.
    ///
    /// For the dataless default typeface or when a glyph has no outline,
    /// falls back to an ascent×descent rectangle sized by the measured
    /// advance.
    pub fn glyph_bounds(&self, glyph: u16) -> skia_rs_core::Rect {
        if glyph == 0 {
            return skia_rs_core::Rect::EMPTY;
        }

        if let Some(data) = self.typeface.font_data() {
            if let Ok(face) = ttf_parser::Face::parse(data, 0) {
                let upem = face.units_per_em();
                if upem > 0 {
                    if let Some(bbox) = face.glyph_bounding_box(ttf_parser::GlyphId(glyph)) {
                        let scale = self.size / upem as Scalar;
                        let left = bbox.x_min as Scalar * scale * self.scale_x;
                        let right = bbox.x_max as Scalar * scale * self.scale_x;
                        let top = -(bbox.y_max as Scalar) * scale;
                        let bottom = -(bbox.y_min as Scalar) * scale;
                        return skia_rs_core::Rect::new(left, top, right, bottom);
                    }
                    // No outline — still report a zero-sized rect rather
                    // than the old ascent×descent approximation.
                    return skia_rs_core::Rect::EMPTY;
                }
            }
        }

        // Dataless typeface fallback.
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
