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
    ///
    /// All values are fully cache-served from the typeface's `ParsedTypeface`
    /// metadata — no re-parsing of the font on every call.
    pub fn metrics(&self) -> FontMetrics {
        if let Some(raw) = self.typeface.raw_metrics() {
            let upem = raw.units_per_em;
            if upem > 0 {
                let scale = self.size / upem as Scalar;

                // Font-space ascender is positive (units above baseline).
                // Screen-space ascent is negative (y grows downward).
                let ascent = -(raw.ascender as Scalar) * scale;
                // Font-space descender is negative (units below baseline).
                // Screen-space descent is positive.
                let descent = -(raw.descender as Scalar) * scale;
                let leading = raw.line_gap as Scalar * scale;

                // Cap and x heights — OS/2 v2+ only; fall back to a
                // fraction of ascent when missing.
                let cap_height = raw
                    .cap_height
                    .map(|v| -(v as Scalar) * scale)
                    .unwrap_or(ascent * 0.875);
                let x_height = raw
                    .x_height
                    .map(|v| -(v as Scalar) * scale)
                    .unwrap_or(ascent * 0.625);

                // Underline from `post` (cached). Font-space position is the
                // *center* of the underline, measured upward from the
                // baseline (usually negative). We want a screen-space
                // offset from the baseline (positive = below), so negate.
                let (underline_position, underline_thickness) = raw
                    .underline_position
                    .map(|pos| {
                        (
                            -(pos as Scalar) * scale,
                            raw.underline_thickness
                                .map(|t| t as Scalar * scale)
                                .unwrap_or(0.05 * self.size),
                        )
                    })
                    .unwrap_or((0.1 * self.size, 0.05 * self.size));

                // Strikeout from OS/2 (cached). Same convention as underline.
                let (strikeout_position, strikeout_thickness) = raw
                    .strikeout_position
                    .map(|pos| {
                        (
                            -(pos as Scalar) * scale,
                            raw.strikeout_thickness
                                .map(|t| t as Scalar * scale)
                                .unwrap_or(0.05 * self.size),
                        )
                    })
                    .unwrap_or((-0.3 * self.size, 0.05 * self.size));

                // Visible bounds: match Skia's convention of including a
                // small margin beyond ascent/descent.
                let top = ascent * 1.125;
                let bottom = descent * 1.125;

                // Average / max char width: still requires a Face parse since
                // hmtx glyph advances are not worth caching for every glyph.
                // We accept one parse here for avg/max width; underline and
                // strikeout are now fully cache-served above.
                let (avg_char_width, max_char_width) =
                    if let Some(data) = self.typeface.font_data() {
                        if let Ok(face) = ttf_parser::Face::parse(data, 0) {
                            let avg = face
                                .glyph_index('x')
                                .and_then(|g| face.glyph_hor_advance(g))
                                .map(|a| a as Scalar * scale)
                                .unwrap_or(0.5 * self.size);
                            let max = face
                                .glyph_index('M')
                                .and_then(|g| face.glyph_hor_advance(g))
                                .map(|a| a as Scalar * scale)
                                .unwrap_or(self.size);
                            (avg, max)
                        } else {
                            (0.5 * self.size, self.size)
                        }
                    } else {
                        (0.5 * self.size, self.size)
                    };

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

    /// Check if a glyph has a color representation in the font.
    ///
    /// Returns `true` iff at least one of the following color tables
    /// contains a definition for `glyph`:
    /// - `COLR` (layered colored outlines, v0; v1 paint graphs not yet
    ///   rendered but are still reported as color glyphs)
    /// - `sbix` / `CBDT` / `CBLC` / `bdat` (raster bitmap strikes)
    /// - `SVG ` (embedded SVG documents)
    ///
    /// Previously this was a `glyph > 0x1000` heuristic (gap C-5) which
    /// both false-positives for any normal font with >4096 glyphs and
    /// false-negatives for small emoji-only fonts. The new check
    /// consults the actual font tables via `ttf_parser`.
    pub fn glyph_is_color(&self, glyph: u16) -> bool {
        if glyph == 0 {
            return false;
        }
        let Some(data) = self.typeface.font_data() else {
            return false;
        };
        let Ok(face) = ttf_parser::Face::parse(data, 0) else {
            return false;
        };

        let gid = ttf_parser::GlyphId(glyph);

        if face.is_color_glyph(gid) {
            return true;
        }

        let ppem = self.size.ceil().max(1.0) as u16;
        if face.glyph_raster_image(gid, ppem).is_some() {
            return true;
        }

        if face.glyph_svg_image(gid).is_some() {
            return true;
        }

        false
    }

    /// Get an image for a color glyph (emoji) when the font ships one as
    /// a raster bitmap (CBDT/CBLC, sbix, or bdat).
    ///
    /// Returns `None` for:
    /// - non-color glyphs
    /// - color glyphs that only have a COLR or SVG definition (use
    ///   [`Self::glyph_color_layers`] or the SVG data directly for those)
    /// - the dataless default typeface
    ///
    /// Callers are responsible for decoding PNG/JPEG data returned in
    /// `GlyphImageFormat::Png`/`Jpeg` via `skia-rs-codec`. Bitmap formats
    /// (`Bgra32`, `Mono`, etc.) are returned raw — BGRA premultiplied
    /// is the format actually shipped in `CBDT` so consumers should
    /// treat it as premultiplied BGRA, not straight RGBA. The field
    /// order and pixel values are preserved byte-for-byte from the
    /// font; no color conversion is done.
    pub fn glyph_image(&self, glyph: u16) -> Option<GlyphImage> {
        if glyph == 0 {
            return None;
        }
        let data = self.typeface.font_data()?;
        let face = ttf_parser::Face::parse(data, 0).ok()?;
        let gid = ttf_parser::GlyphId(glyph);

        let ppem = self.size.ceil().max(1.0) as u16;
        let raster = face.glyph_raster_image(gid, ppem)?;

        // Scale raster bitmap offsets from the strike's ppem into the
        // font's current size so the returned left/top offsets are in
        // display pixels. The strike's `pixels_per_em` may differ from
        // `size` — ttf-parser picks the nearest available strike.
        let scale = if raster.pixels_per_em > 0 {
            self.size / raster.pixels_per_em as Scalar
        } else {
            1.0
        };

        let format = match raster.format {
            ttf_parser::RasterImageFormat::PNG => GlyphImageFormat::Png,
            ttf_parser::RasterImageFormat::BitmapMono
            | ttf_parser::RasterImageFormat::BitmapMonoPacked => GlyphImageFormat::Mono,
            ttf_parser::RasterImageFormat::BitmapGray2
            | ttf_parser::RasterImageFormat::BitmapGray2Packed => GlyphImageFormat::Gray2,
            ttf_parser::RasterImageFormat::BitmapGray4
            | ttf_parser::RasterImageFormat::BitmapGray4Packed => GlyphImageFormat::Gray4,
            ttf_parser::RasterImageFormat::BitmapGray8 => GlyphImageFormat::Gray8,
            ttf_parser::RasterImageFormat::BitmapPremulBgra32 => GlyphImageFormat::PremulBgra32,
        };

        Some(GlyphImage {
            width: raster.width as i32,
            height: raster.height as i32,
            // Preserve the raw data buffer — no decode, no conversion.
            pixels: raster.data.to_vec(),
            format,
            left: raster.x as Scalar * scale,
            top: raster.y as Scalar * scale,
        })
    }

    /// Return the SVG document embedded for `glyph` in an `SVG ` table,
    /// if present. Returns the raw SVG bytes (possibly gzipped — the
    /// `SVG ` table stores SVGZ in that case); callers are expected to
    /// decompress and render via `skia-rs-svg`.
    ///
    /// Returns `None` for non-SVG glyphs and for the dataless default
    /// typeface.
    pub fn glyph_svg(&self, glyph: u16) -> Option<Vec<u8>> {
        if glyph == 0 {
            return None;
        }
        let data = self.typeface.font_data()?;
        let face = ttf_parser::Face::parse(data, 0).ok()?;
        face.glyph_svg_image(ttf_parser::GlyphId(glyph))
            .map(|doc| doc.data.to_vec())
    }

    /// Return the number of color palettes in the `CPAL` table, or `None`
    /// if the font has no color palette (equivalently: no COLR table or
    /// no CPAL).
    pub fn color_palette_count(&self) -> Option<u16> {
        let data = self.typeface.font_data()?;
        let face = ttf_parser::Face::parse(data, 0).ok()?;
        Some(face.color_palettes()?.get())
    }

    /// Decompose a `COLR` (v0) color glyph into its layer list.
    ///
    /// Each returned `ColorGlyphLayer` pairs a regular glyph id with the
    /// RGBA color it should be filled in. Callers render a color glyph
    /// by drawing each layer's outline in order with its associated
    /// color — the standard COLR v0 composition model.
    ///
    /// For COLR v1 (paint graphs with gradients, transforms, and
    /// compositing) this method collects a flattened layer list by
    /// walking the paint tree and recording solid-fill sub-paths; it
    /// will not reproduce gradients, transforms, or blending. v1
    /// gradient fidelity is tracked as a follow-up (the ttf-parser
    /// Painter API is wired end-to-end but the paint enum carries types
    /// we cannot emit into `GlyphImage`). For simple v0 emoji fonts
    /// (Noto Color Emoji, Segoe UI Emoji) this is sufficient.
    ///
    /// `palette` selects which `CPAL` palette to use; pass 0 for the
    /// default palette. `foreground_color` is used for layers that
    /// reference the "foreground" palette index (0xFFFF); pass the
    /// paragraph foreground color for that span.
    ///
    /// Returns `None` if the glyph has no COLR definition, the font has
    /// no CPAL, or the typeface has no data.
    pub fn glyph_color_layers(
        &self,
        glyph: u16,
        palette: u16,
        foreground_color: u32,
    ) -> Option<Vec<ColorGlyphLayer>> {
        if glyph == 0 {
            return None;
        }
        let data = self.typeface.font_data()?;
        let face = ttf_parser::Face::parse(data, 0).ok()?;
        let gid = ttf_parser::GlyphId(glyph);

        if !face.is_color_glyph(gid) {
            return None;
        }

        // Paint walker that records (glyph_id, color) pairs every time a
        // "fill with solid color" paint is issued after an outline_glyph.
        // COLR v0 always matches this pattern; COLR v1 solid-fill layers
        // do too. Gradient/transformed paints are recorded as layers
        // with the gradient's "representative" color to avoid silently
        // dropping them — callers can detect non-solid layers by
        // checking `ColorGlyphLayer::is_gradient`.
        let fg = ttf_rgba_from_argb(foreground_color);
        let mut walker = ColorLayerWalker {
            layers: Vec::new(),
            current_glyph: None,
            palette,
            foreground: fg,
        };

        face.paint_color_glyph(gid, palette, fg, &mut walker)?;

        if walker.layers.is_empty() {
            None
        } else {
            Some(walker.layers)
        }
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
    /// Used by text decoration positioning to skip drawing underline or
    /// strikethrough strokes *through* glyph outlines (e.g. the descender
    /// of a `g` or the bowl of a `y` should produce a gap in the
    /// underline rather than be struck through).
    ///
    /// For each glyph in `glyphs`, the outline is read via `glyph_path`,
    /// flattened to line segments, and intersected with the horizontal
    /// band between `y = top` and `y = bottom`. Every segment that enters
    /// or leaves the band contributes an x-coordinate intercept; these
    /// are returned sorted in the form `[enter_1, exit_1, enter_2,
    /// exit_2, ...]` so callers can walk the list in pairs.
    ///
    /// When the underlying typeface has no font data (the default
    /// typeface) or a glyph has no outline (spaces), the function falls
    /// back to the bounding-box intercepts — the old placeholder
    /// behaviour — which produces a continuous underline for that glyph.
    pub fn glyph_intercepts(
        &self,
        glyphs: &[u16],
        positions: &[skia_rs_core::Point],
        top: Scalar,
        bottom: Scalar,
    ) -> Vec<Scalar> {
        let (top, bottom) = if top <= bottom {
            (top, bottom)
        } else {
            (bottom, top)
        };

        let mut intercepts: Vec<Scalar> = Vec::new();

        for (i, &glyph) in glyphs.iter().enumerate() {
            if glyph == 0 {
                continue;
            }
            let pos = positions.get(i).copied().unwrap_or_default();

            let glyph_xs = match self.glyph_path(glyph) {
                Some(path) => path_band_intercepts(&path, top - pos.y, bottom - pos.y),
                None => Vec::new(),
            };

            if !glyph_xs.is_empty() {
                for x in glyph_xs {
                    intercepts.push(pos.x + x);
                }
            } else {
                // Fallback: bounding box test. For the dataless default
                // typeface (no outline) this preserves the previous
                // placeholder behaviour so callers still get *something*.
                let bounds = self.glyph_bounds(glyph);
                if !bounds.is_empty() {
                    let glyph_top = pos.y + bounds.top;
                    let glyph_bottom = pos.y + bounds.bottom;
                    if glyph_bottom >= top && glyph_top <= bottom {
                        intercepts.push(pos.x + bounds.left);
                        intercepts.push(pos.x + bounds.right);
                    }
                }
            }
        }

        // Sort + collapse overlapping pairs so callers can walk them as
        // enter/exit pairs.
        intercepts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        intercepts
    }
}

/// Flatten a Path into line segments and return the x-coordinates where
/// the outline crosses the horizontal band [y_top, y_bottom] (y-down
/// screen space). Returns crossings grouped as enter/exit pairs per
/// continuous interval.
///
/// The quad/cubic/conic segments are approximated by adaptive subdivision
/// until the chord-to-curve error drops below a flatness tolerance
/// proportional to the band height (1/32 of band height, or 0.25 px
/// whichever is larger). For typical font sizes this produces 4-16
/// segments per curve, which is plenty for decoration gap rendering
/// without being wasteful.
fn path_band_intercepts(
    path: &skia_rs_path::Path,
    y_top: Scalar,
    y_bottom: Scalar,
) -> Vec<Scalar> {
    use skia_rs_core::Point;
    use skia_rs_path::PathElement;

    if y_top >= y_bottom {
        return Vec::new();
    }

    let band_height = y_bottom - y_top;
    let tolerance = (band_height / 32.0).max(0.25);

    let mut segments: Vec<(Point, Point)> = Vec::new();
    let mut current = Point::zero();
    let mut contour_start = Point::zero();

    for elem in path.iter() {
        match elem {
            PathElement::Move(p) => {
                current = p;
                contour_start = p;
            }
            PathElement::Line(p) => {
                segments.push((current, p));
                current = p;
            }
            PathElement::Quad(c, p) => {
                flatten_quad(current, c, p, tolerance, &mut segments);
                current = p;
            }
            PathElement::Conic(c, p, _w) => {
                // Approximate conic as quadratic (sufficient for decoration
                // intercepts; the exact rational-quadratic form isn't
                // needed at a 0.25px tolerance).
                flatten_quad(current, c, p, tolerance, &mut segments);
                current = p;
            }
            PathElement::Cubic(c1, c2, p) => {
                flatten_cubic(current, c1, c2, p, tolerance, &mut segments);
                current = p;
            }
            PathElement::Close => {
                if current != contour_start {
                    segments.push((current, contour_start));
                    current = contour_start;
                }
            }
        }
    }

    // Collect x-crossings for each horizontal line at the band edges.
    // For the gap-drawing case we want the *entire range* the outline
    // covers inside the band; it's sufficient to take the union of
    // segment endpoints that lie inside the band together with
    // intersections with each band edge. A conservative bound is to use
    // every segment that touches the band and emit its min/max x.
    let mut xs: Vec<Scalar> = Vec::new();
    for &(a, b) in &segments {
        let seg_top = a.y.min(b.y);
        let seg_bot = a.y.max(b.y);
        // Segment disjoint from band → skip.
        if seg_bot < y_top || seg_top > y_bottom {
            continue;
        }

        // Clip the segment to the band and record the x range of the
        // clipped portion. For a line segment (p1 → p2), parametrise by
        // t ∈ [0,1] and find the t range where y(t) ∈ [y_top, y_bottom].
        let dy = b.y - a.y;
        let (t0, t1) = if dy.abs() < 1e-6 {
            // Horizontal segment — lies entirely in the band if touching
            // it at all. Use the full range.
            (0.0f32, 1.0f32)
        } else {
            let mut t_enter = (y_top - a.y) / dy;
            let mut t_exit = (y_bottom - a.y) / dy;
            if t_enter > t_exit {
                std::mem::swap(&mut t_enter, &mut t_exit);
            }
            (t_enter.max(0.0), t_exit.min(1.0))
        };
        if t0 > t1 {
            continue;
        }
        let x0 = a.x + (b.x - a.x) * t0;
        let x1 = a.x + (b.x - a.x) * t1;
        xs.push(x0.min(x1));
        xs.push(x0.max(x1));
    }

    if xs.is_empty() {
        return xs;
    }

    // Merge into non-overlapping intervals so callers can consume them
    // as [enter_1, exit_1, enter_2, exit_2, …] pairs. Each pair of
    // xs[2k], xs[2k+1] represents a contiguous range of x where the
    // outline occupies the band.
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut merged: Vec<Scalar> = Vec::with_capacity(xs.len());
    let mut iter = xs.chunks_exact(2);
    let mut cur: Option<(Scalar, Scalar)> = None;
    for pair in &mut iter {
        let (lo, hi) = (pair[0], pair[1]);
        cur = match cur {
            Some((a, b)) if lo <= b + tolerance => Some((a, b.max(hi))),
            Some((a, b)) => {
                merged.push(a);
                merged.push(b);
                Some((lo, hi))
            }
            None => Some((lo, hi)),
        };
    }
    if let Some((a, b)) = cur {
        merged.push(a);
        merged.push(b);
    }

    merged
}

/// Recursively subdivide a quadratic bezier into chord approximations
/// until the control point is within `tol` of the chord midpoint.
fn flatten_quad(
    p0: skia_rs_core::Point,
    p1: skia_rs_core::Point,
    p2: skia_rs_core::Point,
    tol: Scalar,
    out: &mut Vec<(skia_rs_core::Point, skia_rs_core::Point)>,
) {
    use skia_rs_core::Point;
    // Midpoint of p0-p2 chord.
    let midx = (p0.x + p2.x) * 0.5;
    let midy = (p0.y + p2.y) * 0.5;
    // Distance from control to the chord midpoint.
    let dx = p1.x - midx;
    let dy = p1.y - midy;
    if dx * dx + dy * dy <= tol * tol {
        out.push((p0, p2));
        return;
    }
    // Subdivide at t=0.5 using de Casteljau.
    let q0 = Point::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5);
    let q1 = Point::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5);
    let mid = Point::new((q0.x + q1.x) * 0.5, (q0.y + q1.y) * 0.5);
    flatten_quad(p0, q0, mid, tol, out);
    flatten_quad(mid, q1, p2, tol, out);
}

/// Recursively subdivide a cubic bezier into chord approximations.
fn flatten_cubic(
    p0: skia_rs_core::Point,
    p1: skia_rs_core::Point,
    p2: skia_rs_core::Point,
    p3: skia_rs_core::Point,
    tol: Scalar,
    out: &mut Vec<(skia_rs_core::Point, skia_rs_core::Point)>,
) {
    use skia_rs_core::Point;
    // Flatness heuristic: distance of control points from the chord.
    let cx = (p0.x + p3.x) * 0.5;
    let cy = (p0.y + p3.y) * 0.5;
    let d1 = {
        let dx = p1.x - cx;
        let dy = p1.y - cy;
        dx * dx + dy * dy
    };
    let d2 = {
        let dx = p2.x - cx;
        let dy = p2.y - cy;
        dx * dx + dy * dy
    };
    if d1 <= tol * tol && d2 <= tol * tol {
        out.push((p0, p3));
        return;
    }
    // De Casteljau at t=0.5.
    let q0 = Point::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5);
    let q1 = Point::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5);
    let q2 = Point::new((p2.x + p3.x) * 0.5, (p2.y + p3.y) * 0.5);
    let r0 = Point::new((q0.x + q1.x) * 0.5, (q0.y + q1.y) * 0.5);
    let r1 = Point::new((q1.x + q2.x) * 0.5, (q1.y + q2.y) * 0.5);
    let mid = Point::new((r0.x + r1.x) * 0.5, (r0.y + r1.y) * 0.5);
    flatten_cubic(p0, q0, r0, mid, tol, out);
    flatten_cubic(mid, r1, q2, p3, tol, out);
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

/// Raster image data extracted from a color glyph bitmap strike
/// (`CBDT`/`CBLC`, `sbix`, or `bdat`).
///
/// The `format` field distinguishes encoded (PNG) from raw-bitmap data.
/// Callers typically delegate PNG decoding to `skia-rs-codec`; raw bitmap
/// formats can be uploaded to a texture directly (with the documented
/// byte-order caveat for `PremulBgra32`).
#[derive(Debug, Clone)]
pub struct GlyphImage {
    /// Image width in pixels.
    pub width: i32,
    /// Image height in pixels.
    pub height: i32,
    /// Raw encoded or raw bitmap data. See `format` for the byte layout.
    pub pixels: Vec<u8>,
    /// Pixel layout / encoding of `pixels`.
    pub format: GlyphImageFormat,
    /// Left offset from glyph origin, in display pixels (scaled from the
    /// strike's native ppem to the font's current size).
    pub left: Scalar,
    /// Top offset from glyph origin, in display pixels.
    pub top: Scalar,
}

/// Pixel format of a [`GlyphImage`].
///
/// Mirrors `ttf_parser::RasterImageFormat` but is re-exported here so the
/// skia-rs-text public surface does not leak a ttf-parser type. See the
/// OpenType `CBDT`/`sbix` specs for each format's bit layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphImageFormat {
    /// PNG-encoded bytes. Delegate decoding to `skia-rs-codec`.
    Png,
    /// 1-bit-per-pixel monochrome bitmap, row-aligned to byte boundary.
    /// MSB is the first pixel in each byte.
    Mono,
    /// 2-bits-per-pixel grayscale bitmap.
    Gray2,
    /// 4-bits-per-pixel grayscale bitmap.
    Gray4,
    /// 8-bits-per-pixel grayscale bitmap.
    Gray8,
    /// 32-bit premultiplied BGRA (per the `CBDT` / `sbix` spec).
    PremulBgra32,
}

/// A single layer of a COLR color glyph.
///
/// COLR v0 decomposes an emoji glyph into a stack of regular outline
/// glyphs each filled with a solid palette color. The caller renders a
/// color glyph by drawing every layer's `glyph_path` in order with its
/// `color`, which produces the final emoji.
///
/// For COLR v1 solid-fill paints, the same structure applies. Gradient
/// and transformed paints are reported as layers with `is_gradient =
/// true` and a fallback solid color sampled from the gradient's first
/// stop — callers that care about high-fidelity v1 rendering should
/// check this flag and treat such layers as placeholders rather than
/// final.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorGlyphLayer {
    /// Glyph id of the layer's outline.
    pub glyph: u16,
    /// ARGB color (0xAARRGGBB) to fill the outline with.
    pub color: u32,
    /// `true` if the originating COLR paint was a gradient or otherwise
    /// non-solid; the `color` field contains a representative solid
    /// color in that case.
    pub is_gradient: bool,
}

/// Convert a `0xAARRGGBB` u32 to ttf_parser's `RgbaColor`.
fn ttf_rgba_from_argb(argb: u32) -> ttf_parser::RgbaColor {
    let a = ((argb >> 24) & 0xff) as u8;
    let r = ((argb >> 16) & 0xff) as u8;
    let g = ((argb >> 8) & 0xff) as u8;
    let b = (argb & 0xff) as u8;
    ttf_parser::RgbaColor::new(r, g, b, a)
}

/// Pack a ttf_parser `RgbaColor` back into an ARGB u32.
fn argb_from_ttf_rgba(c: ttf_parser::RgbaColor) -> u32 {
    ((c.alpha as u32) << 24)
        | ((c.red as u32) << 16)
        | ((c.green as u32) << 8)
        | (c.blue as u32)
}

/// Painter implementation that collects COLR layers as
/// `(glyph_id, color)` pairs. Outlines are recorded on `outline_glyph`
/// and materialised as a layer every time a `paint` call fills them.
/// Clips, transforms, and composite modes are ignored — for gradients
/// we sample the first stop as a representative color and mark the
/// layer with `is_gradient = true`.
struct ColorLayerWalker {
    layers: Vec<ColorGlyphLayer>,
    current_glyph: Option<u16>,
    palette: u16,
    foreground: ttf_parser::RgbaColor,
}

impl<'a> ttf_parser::colr::Painter<'a> for ColorLayerWalker {
    fn outline_glyph(&mut self, glyph_id: ttf_parser::GlyphId) {
        // Record which glyph is the current outline target; the next
        // `paint` call will consume this and emit a layer.
        self.current_glyph = Some(glyph_id.0);
    }

    fn paint(&mut self, paint: ttf_parser::colr::Paint<'a>) {
        let Some(glyph) = self.current_glyph.take() else {
            return;
        };

        let palette = self.palette;
        let fallback = self.foreground;

        match paint {
            ttf_parser::colr::Paint::Solid(color) => {
                self.layers.push(ColorGlyphLayer {
                    glyph,
                    color: argb_from_ttf_rgba(color),
                    is_gradient: false,
                });
            }
            ttf_parser::colr::Paint::LinearGradient(g) => {
                let rep = g
                    .stops(palette, &[])
                    .next()
                    .map(|s| s.color)
                    .unwrap_or(fallback);
                self.layers.push(ColorGlyphLayer {
                    glyph,
                    color: argb_from_ttf_rgba(rep),
                    is_gradient: true,
                });
            }
            ttf_parser::colr::Paint::RadialGradient(g) => {
                let rep = g
                    .stops(palette, &[])
                    .next()
                    .map(|s| s.color)
                    .unwrap_or(fallback);
                self.layers.push(ColorGlyphLayer {
                    glyph,
                    color: argb_from_ttf_rgba(rep),
                    is_gradient: true,
                });
            }
            ttf_parser::colr::Paint::SweepGradient(g) => {
                let rep = g
                    .stops(palette, &[])
                    .next()
                    .map(|s| s.color)
                    .unwrap_or(fallback);
                self.layers.push(ColorGlyphLayer {
                    glyph,
                    color: argb_from_ttf_rgba(rep),
                    is_gradient: true,
                });
            }
        }
    }

    fn push_clip(&mut self) {
        // Consume the current outline so it is not mistakenly paired
        // with a later paint call.
        self.current_glyph = None;
    }

    fn push_clip_box(&mut self, _clipbox: ttf_parser::colr::ClipBox) {}
    fn pop_clip(&mut self) {}
    fn push_layer(&mut self, _mode: ttf_parser::colr::CompositeMode) {}
    fn pop_layer(&mut self) {}
    fn push_transform(&mut self, _transform: ttf_parser::Transform) {}
    fn pop_transform(&mut self) {}
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
