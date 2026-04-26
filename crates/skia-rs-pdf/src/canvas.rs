//! PDF canvas for drawing.

use crate::font::{PdfFontManager, StandardFont};
use crate::image::PdfImageManager;
use crate::transparency::{PdfBlendMode, TransparencyManager};
use skia_rs_core::{Color, Matrix, Point, Rect, Scalar};
use skia_rs_paint::{BlendMode, Paint, Style};
use skia_rs_path::{Path, PathElement};
use std::collections::BTreeSet;

/// A canvas that generates PDF content streams.
///
/// The canvas holds `&mut` references to the document-level
/// [`PdfFontManager`], [`PdfImageManager`] and [`TransparencyManager`].
/// Every time the canvas emits a `Tf`, `Do` or `gs` operator it registers
/// the corresponding resource with the manager and remembers its index in
/// one of the `used_*` sets so that `PdfDocument::write_to` can build a
/// correct per-page `/Resources` dictionary.
pub struct PdfCanvas<'a> {
    /// Page width.
    width: Scalar,
    /// Page height.
    height: Scalar,
    /// Object ID (reserved at begin_page; filled in when the page is written).
    object_id: u32,
    /// Content stream.
    content: Vec<u8>,
    /// Graphics state stack.
    state_stack: Vec<GraphicsState>,
    /// Shared font manager.
    fonts: &'a mut PdfFontManager,
    /// Shared image manager.
    images: &'a mut PdfImageManager,
    /// Shared transparency manager.
    transparency: &'a mut TransparencyManager,
    /// Set of font indices referenced by this page.
    used_fonts: BTreeSet<usize>,
    /// Set of image indices referenced by this page.
    used_images: BTreeSet<usize>,
    /// Set of ExtGState indices referenced by this page.
    used_ext_gstates: BTreeSet<usize>,
    /// Currently selected font index (for draw_text without an explicit font).
    current_font: Option<usize>,
}

/// Graphics state.
#[derive(Clone)]
struct GraphicsState {
    /// Current transformation matrix.
    matrix: Matrix,
    /// Current color.
    color: Color,
    /// Line width.
    line_width: Scalar,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            matrix: Matrix::IDENTITY,
            color: Color::BLACK,
            line_width: 1.0,
        }
    }
}

/// Emitted content and per-page resources produced by a [`PdfCanvas`].
///
/// Returned by [`PdfCanvas::finish`]; consumed by
/// [`crate::PdfDocument::end_page`].
pub struct PageContent {
    /// Page width.
    pub width: Scalar,
    /// Page height.
    pub height: Scalar,
    /// Reserved object id for the page object.
    pub object_id: u32,
    /// Raw content-stream bytes.
    pub content: Vec<u8>,
    /// Font manager indices referenced from this page.
    pub used_fonts: Vec<usize>,
    /// Image manager indices referenced from this page.
    pub used_images: Vec<usize>,
    /// ExtGState manager indices referenced from this page.
    pub used_ext_gstates: Vec<usize>,
}

impl<'a> PdfCanvas<'a> {
    /// Create a new PDF canvas.
    ///
    /// Called by [`crate::PdfDocument::begin_page`]; external callers should
    /// prefer that entry point.
    pub fn new(
        width: Scalar,
        height: Scalar,
        object_id: u32,
        fonts: &'a mut PdfFontManager,
        images: &'a mut PdfImageManager,
        transparency: &'a mut TransparencyManager,
    ) -> Self {
        let mut canvas = Self {
            width,
            height,
            object_id,
            content: Vec::new(),
            state_stack: vec![GraphicsState::default()],
            fonts,
            images,
            transparency,
            used_fonts: BTreeSet::new(),
            used_images: BTreeSet::new(),
            used_ext_gstates: BTreeSet::new(),
            current_font: None,
        };

        // Set up coordinate system (PDF has origin at bottom-left)
        canvas.write_op(&format!("1 0 0 -1 0 {} cm\n", height));

        canvas
    }

    /// Get the width.
    pub fn width(&self) -> Scalar {
        self.width
    }

    /// Get the height.
    pub fn height(&self) -> Scalar {
        self.height
    }

    /// Get the object ID.
    pub fn object_id(&self) -> u32 {
        self.object_id
    }

    /// Select a font for subsequent [`draw_text`](Self::draw_text) calls.
    ///
    /// `index` must be a valid index into the document's font manager.
    pub fn set_font(&mut self, index: usize) {
        assert!(
            self.fonts.get(index).is_some(),
            "font index {} out of range",
            index
        );
        self.used_fonts.insert(index);
        self.current_font = Some(index);
    }

    /// Register a standard font and select it as the current font.
    ///
    /// Returns the manager index — reuse it with [`set_font`](Self::set_font)
    /// or pass it to [`draw_text_with_font`](Self::draw_text_with_font).
    pub fn use_standard_font(&mut self, font: StandardFont) -> usize {
        let idx = self.fonts.register_standard(font);
        self.set_font(idx);
        idx
    }

    /// Finish this canvas, returning the page content and used-resource sets.
    pub fn finish(self) -> PageContent {
        PageContent {
            width: self.width,
            height: self.height,
            object_id: self.object_id,
            content: self.content,
            used_fonts: self.used_fonts.into_iter().collect(),
            used_images: self.used_images.into_iter().collect(),
            used_ext_gstates: self.used_ext_gstates.into_iter().collect(),
        }
    }

    /// Write a PDF operation.
    fn write_op(&mut self, op: &str) {
        self.content.extend_from_slice(op.as_bytes());
    }

    /// Get current state.
    fn state(&self) -> &GraphicsState {
        self.state_stack.last().unwrap()
    }

    /// Get mutable current state.
    fn state_mut(&mut self) -> &mut GraphicsState {
        self.state_stack.last_mut().unwrap()
    }

    /// Save graphics state.
    pub fn save(&mut self) {
        let state = self.state().clone();
        self.state_stack.push(state);
        self.write_op("q\n");
    }

    /// Restore graphics state.
    pub fn restore(&mut self) {
        if self.state_stack.len() > 1 {
            self.state_stack.pop();
            self.write_op("Q\n");
        }
    }

    /// Apply a transform.
    pub fn concat(&mut self, matrix: &Matrix) {
        self.state_mut().matrix = self.state().matrix.concat(matrix);
        self.write_op(&format!(
            "{} {} {} {} {} {} cm\n",
            matrix.values[0],
            matrix.values[3],
            matrix.values[1],
            matrix.values[4],
            matrix.values[2],
            matrix.values[5]
        ));
    }

    /// Translate.
    pub fn translate(&mut self, dx: Scalar, dy: Scalar) {
        self.concat(&Matrix::translate(dx, dy));
    }

    /// Scale.
    pub fn scale(&mut self, sx: Scalar, sy: Scalar) {
        self.concat(&Matrix::scale(sx, sy));
    }

    /// Rotate (degrees).
    pub fn rotate(&mut self, degrees: Scalar) {
        let radians = degrees * std::f32::consts::PI / 180.0;
        self.concat(&Matrix::rotate(radians));
    }

    /// Set the fill color.
    pub fn set_fill_color(&mut self, color: Color) {
        let r = color.red() as f32 / 255.0;
        let g = color.green() as f32 / 255.0;
        let b = color.blue() as f32 / 255.0;
        self.state_mut().color = color;
        self.write_op(&format!("{:.3} {:.3} {:.3} rg\n", r, g, b));
    }

    /// Set the stroke color.
    pub fn set_stroke_color(&mut self, color: Color) {
        let r = color.red() as f32 / 255.0;
        let g = color.green() as f32 / 255.0;
        let b = color.blue() as f32 / 255.0;
        self.write_op(&format!("{:.3} {:.3} {:.3} RG\n", r, g, b));
    }

    /// Set the line width.
    pub fn set_line_width(&mut self, width: Scalar) {
        self.state_mut().line_width = width;
        self.write_op(&format!("{} w\n", width));
    }

    /// Draw a rectangle.
    pub fn draw_rect(&mut self, rect: &Rect, paint: &Paint) {
        self.apply_paint(paint);
        self.write_op(&format!(
            "{} {} {} {} re ",
            rect.left,
            rect.top,
            rect.width(),
            rect.height()
        ));
        self.stroke_or_fill(paint);
    }

    /// Draw a line.
    pub fn draw_line(&mut self, p0: Point, p1: Point, paint: &Paint) {
        self.apply_paint(paint);
        self.write_op(&format!("{} {} m {} {} l S\n", p0.x, p0.y, p1.x, p1.y));
    }

    /// Draw a circle.
    pub fn draw_circle(&mut self, center: Point, radius: Scalar, paint: &Paint) {
        // Approximate circle with bezier curves
        let k = radius * 0.5522847498; // Magic constant for circle approximation

        self.apply_paint(paint);
        self.write_op(&format!("{} {} m\n", center.x + radius, center.y));

        // Four quadrants
        self.write_op(&format!(
            "{} {} {} {} {} {} c\n",
            center.x + radius,
            center.y + k,
            center.x + k,
            center.y + radius,
            center.x,
            center.y + radius
        ));
        self.write_op(&format!(
            "{} {} {} {} {} {} c\n",
            center.x - k,
            center.y + radius,
            center.x - radius,
            center.y + k,
            center.x - radius,
            center.y
        ));
        self.write_op(&format!(
            "{} {} {} {} {} {} c\n",
            center.x - radius,
            center.y - k,
            center.x - k,
            center.y - radius,
            center.x,
            center.y - radius
        ));
        self.write_op(&format!(
            "{} {} {} {} {} {} c\n",
            center.x + k,
            center.y - radius,
            center.x + radius,
            center.y - k,
            center.x + radius,
            center.y
        ));

        self.stroke_or_fill(paint);
    }

    /// Draw a path.
    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        self.apply_paint(paint);

        let mut current = Point::zero();
        // Subpath start — used to correctly advance `current` on Close.
        let mut subpath_start = Point::zero();

        for element in path.iter() {
            match element {
                PathElement::Move(p) => {
                    self.write_op(&format!("{} {} m\n", p.x, p.y));
                    current = p;
                    subpath_start = p;
                }
                PathElement::Line(p) => {
                    self.write_op(&format!("{} {} l\n", p.x, p.y));
                    current = p;
                }
                PathElement::Quad(ctrl, end) => {
                    // Convert quadratic to cubic
                    let c1 = Point::new(
                        current.x + 2.0 / 3.0 * (ctrl.x - current.x),
                        current.y + 2.0 / 3.0 * (ctrl.y - current.y),
                    );
                    let c2 = Point::new(
                        end.x + 2.0 / 3.0 * (ctrl.x - end.x),
                        end.y + 2.0 / 3.0 * (ctrl.y - end.y),
                    );
                    self.write_op(&format!(
                        "{} {} {} {} {} {} c\n",
                        c1.x, c1.y, c2.x, c2.y, end.x, end.y
                    ));
                    current = end;
                }
                PathElement::Conic(ctrl, end, w) => {
                    // A rational quadratic cannot be expressed exactly as a
                    // cubic Bezier, so flatten to line segments using
                    // skia-rs-path's adaptive conic flattener. Using `l`
                    // ops keeps the result visually correct for any weight.
                    let mut points = Vec::new();
                    skia_rs_path::flatten::flatten_conic_adaptive(
                        &mut points,
                        current,
                        ctrl,
                        end,
                        w,
                        0.25, // 0.25 pt tolerance
                    );
                    for p in points {
                        self.write_op(&format!("{} {} l\n", p.x, p.y));
                    }
                    current = end;
                }
                PathElement::Cubic(c1, c2, end) => {
                    self.write_op(&format!(
                        "{} {} {} {} {} {} c\n",
                        c1.x, c1.y, c2.x, c2.y, end.x, end.y
                    ));
                    current = end;
                }
                PathElement::Close => {
                    self.write_op("h\n");
                    current = subpath_start;
                }
            }
        }

        self.stroke_or_fill(paint);
    }

    /// Draw text using the currently selected font (see [`set_font`]).
    ///
    /// Panics if no font is selected; use [`draw_text_with_font`] to provide
    /// one explicitly.
    pub fn draw_text(
        &mut self,
        text: &str,
        x: Scalar,
        y: Scalar,
        font_size: Scalar,
        paint: &Paint,
    ) {
        let font_idx = self
            .current_font
            .expect("draw_text requires a font: call set_font or use_standard_font first");
        self.draw_text_with_font(text, x, y, font_size, font_idx, paint);
    }

    /// Draw text with an explicit font.
    pub fn draw_text_with_font(
        &mut self,
        text: &str,
        x: Scalar,
        y: Scalar,
        font_size: Scalar,
        font_idx: usize,
        paint: &Paint,
    ) {
        assert!(
            self.fonts.get(font_idx).is_some(),
            "font index {} out of range",
            font_idx
        );
        self.used_fonts.insert(font_idx);

        // Track the characters that were written so the font can produce an
        // accurate ToUnicode CMap at emit time.
        if let Some(font) = self.fonts.get_mut(font_idx) {
            for ch in text.chars() {
                font.record_char(ch);
            }
        }

        self.apply_paint(paint);
        self.write_op("BT\n");
        self.write_op(&format!("/F{} {} Tf\n", font_idx + 1, font_size));
        self.write_op(&format!("{} {} Td\n", x, y));

        // Emit the text literal. For ASCII-clean WinAnsi strings we can use a
        // PDF literal string `(...)`; for anything that contains non-ASCII
        // we fall back to a UTF-16BE hex string `<FEFF...>` so that readers
        // can recover the codepoints via the ToUnicode CMap.
        if text.is_ascii() {
            self.write_op(&format!("({}) Tj\n", escape_pdf_string(text)));
        } else {
            self.write_op(&format!("<{}> Tj\n", utf16be_hex(text)));
        }
        self.write_op("ET\n");
    }

    /// Draw a PDF image registered with the document's image manager.
    ///
    /// The image is painted at the given position at its natural size in
    /// user-space units (1 unit == 1/72 inch). Use [`concat`] or [`save`]/
    /// [`restore`] to scale as needed.
    pub fn draw_image(&mut self, image_idx: usize, x: Scalar, y: Scalar, w: Scalar, h: Scalar) {
        assert!(
            self.images.get(image_idx).is_some(),
            "image index {} out of range",
            image_idx
        );
        self.used_images.insert(image_idx);

        // cm: scale + translate; then Do. We save/restore so we don't leak
        // the transform.
        self.write_op("q\n");
        self.write_op(&format!("{} 0 0 {} {} {} cm\n", w, h, x, y));
        self.write_op(&format!("/Im{} Do\n", image_idx + 1));
        self.write_op("Q\n");
    }

    /// Set the fill/stroke alpha for subsequent drawing via an ExtGState.
    ///
    /// `alpha` is clamped to `[0, 1]`. Emits a `/GS<n> gs` operator and
    /// registers the state with the document's transparency manager.
    pub fn set_alpha(&mut self, alpha: Scalar) {
        let a = alpha.clamp(0.0, 1.0);
        let idx = self.transparency.get_or_create_alpha_state(a);
        self.used_ext_gstates.insert(idx);
        self.write_op(&format!("/GS{} gs\n", idx + 1));
    }

    /// Set the blend mode for subsequent drawing via an ExtGState.
    pub fn set_blend_mode(&mut self, mode: PdfBlendMode) {
        let idx = self.transparency.get_or_create_blend_state(mode);
        self.used_ext_gstates.insert(idx);
        self.write_op(&format!("/GS{} gs\n", idx + 1));
    }

    /// Apply paint settings.
    fn apply_paint(&mut self, paint: &Paint) {
        let color = paint.color32();

        match paint.style() {
            Style::Fill => self.set_fill_color(color),
            Style::Stroke => {
                self.set_stroke_color(color);
                self.set_line_width(paint.stroke_width());
            }
            Style::StrokeAndFill => {
                self.set_fill_color(color);
                self.set_stroke_color(color);
                self.set_line_width(paint.stroke_width());
            }
        }

        // Derive an ExtGState when the paint has a non-opaque alpha or a
        // non-Normal blend mode. We intentionally read the alpha from the
        // raw color channel (not paint.alpha(), which is Color4f.a and
        // therefore already folded into the rg/RG operator) so that partial
        // alpha on a Color32 propagates into /ca /CA.
        let alpha8 = color.alpha();
        let alpha = alpha8 as Scalar / 255.0;
        let pdf_blend = PdfBlendMode::from_skia_blend_mode(paint.blend_mode());
        let needs_alpha_gs = alpha < 1.0;
        let needs_blend_gs =
            !matches!(pdf_blend, Some(PdfBlendMode::Normal)) && pdf_blend.is_some();

        if needs_alpha_gs || needs_blend_gs {
            let mut state = crate::transparency::ExtGraphicsState::default();
            if needs_alpha_gs {
                state.fill_alpha = Some(alpha);
                state.stroke_alpha = Some(alpha);
            }
            if needs_blend_gs {
                state.blend_mode = pdf_blend;
            }
            let idx = self.transparency.add_ext_gstate(state);
            self.used_ext_gstates.insert(idx);
            self.write_op(&format!("/GS{} gs\n", idx + 1));
        }

        // Ensure paint's own blend mode (if Normal) doesn't trip us up
        // further — nothing to do. Unused is fine.
        let _ = BlendMode::SrcOver;
    }

    /// Write stroke or fill operator.
    fn stroke_or_fill(&mut self, paint: &Paint) {
        match paint.style() {
            Style::Fill => self.write_op("f\n"),
            Style::Stroke => self.write_op("S\n"),
            Style::StrokeAndFill => self.write_op("B\n"),
        }
    }
}

/// Escape special characters in a PDF literal string.
///
/// Handles `(`, `)`, `\\`, and non-printable bytes (mapped to octal
/// escapes `\ddd`). Newlines and carriage returns get their dedicated
/// escapes `\n` / `\r` so editors don't rewrap them.
pub fn escape_pdf_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '(' => result.push_str("\\("),
            ')' => result.push_str("\\)"),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x08' => result.push_str("\\b"),
            '\x0C' => result.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) == 0x7F => {
                // Non-printable: octal escape for each UTF-8 byte.
                let mut buf = [0u8; 4];
                for &byte in c.encode_utf8(&mut buf).as_bytes() {
                    result.push_str(&format!("\\{:03o}", byte));
                }
            }
            c => result.push(c),
        }
    }
    result
}

/// Encode a UTF-8 string as an uppercase hex UTF-16BE literal with a BOM,
/// for use inside a PDF hex-string `<FEFF...>`.
pub fn utf16be_hex(s: &str) -> String {
    let mut out = String::with_capacity(4 + s.len() * 4);
    // UTF-16BE BOM
    out.push_str("FEFF");
    for unit in s.encode_utf16() {
        out.push_str(&format!("{:04X}", unit));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::PdfFontManager;
    use crate::image::PdfImageManager;
    use crate::transparency::TransparencyManager;

    fn mk_canvas<'a>(
        fonts: &'a mut PdfFontManager,
        images: &'a mut PdfImageManager,
        transparency: &'a mut TransparencyManager,
    ) -> PdfCanvas<'a> {
        PdfCanvas::new(612.0, 792.0, 1, fonts, images, transparency)
    }

    #[test]
    fn test_pdf_canvas_rect() {
        let mut fonts = PdfFontManager::new();
        let mut images = PdfImageManager::new();
        let mut transparency = TransparencyManager::new();
        let mut canvas = mk_canvas(&mut fonts, &mut images, &mut transparency);

        let mut paint = Paint::new();
        paint.set_color32(Color::from_rgb(255, 0, 0));

        canvas.draw_rect(&Rect::from_xywh(100.0, 100.0, 200.0, 150.0), &paint);

        let page = canvas.finish();
        let content = String::from_utf8(page.content).unwrap();
        assert!(content.contains("re"));
        assert!(content.contains("f")); // Fill operator
    }

    #[test]
    fn test_pdf_canvas_save_restore() {
        let mut fonts = PdfFontManager::new();
        let mut images = PdfImageManager::new();
        let mut transparency = TransparencyManager::new();
        let mut canvas = mk_canvas(&mut fonts, &mut images, &mut transparency);

        canvas.save();
        canvas.translate(100.0, 100.0);
        canvas.restore();

        let page = canvas.finish();
        let content = String::from_utf8(page.content).unwrap();
        assert!(content.contains("q")); // Save
        assert!(content.contains("Q")); // Restore
    }

    #[test]
    fn test_canvas_draw_text_registers_font() {
        let mut fonts = PdfFontManager::new();
        let mut images = PdfImageManager::new();
        let mut transparency = TransparencyManager::new();

        let page = {
            let mut canvas = mk_canvas(&mut fonts, &mut images, &mut transparency);
            let f = canvas.use_standard_font(StandardFont::Helvetica);
            assert_eq!(f, 0);

            let paint = Paint::new();
            canvas.draw_text("Hello", 72.0, 72.0, 12.0, &paint);

            canvas.finish()
        };

        assert_eq!(page.used_fonts, vec![0]);
        let content = String::from_utf8(page.content).unwrap();
        assert!(content.contains("/F1 12 Tf"));
        assert!(content.contains("(Hello) Tj"));
    }

    #[test]
    fn test_canvas_draw_text_unicode_uses_utf16be() {
        let mut fonts = PdfFontManager::new();
        let mut images = PdfImageManager::new();
        let mut transparency = TransparencyManager::new();

        let page = {
            let mut canvas = mk_canvas(&mut fonts, &mut images, &mut transparency);
            canvas.use_standard_font(StandardFont::Helvetica);

            let paint = Paint::new();
            canvas.draw_text("héllo", 72.0, 72.0, 12.0, &paint);
            canvas.finish()
        };

        let content = String::from_utf8(page.content).unwrap();
        // "héllo" encoded as UTF-16BE is 0068 00E9 006C 006C 006F, preceded
        // by a BOM FEFF.
        assert!(
            content.contains("<FEFF006800E9006C006C006F> Tj"),
            "content missing UTF-16BE encoded literal: {}",
            content
        );
    }

    #[test]
    fn test_canvas_draw_image_records_use() {
        let mut fonts = PdfFontManager::new();
        let mut images = PdfImageManager::new();
        let mut transparency = TransparencyManager::new();

        // Seed an image.
        let idx = images.add_rgb(4, 4, &[0u8; 4 * 4 * 3]);

        let page = {
            let mut canvas = mk_canvas(&mut fonts, &mut images, &mut transparency);
            canvas.draw_image(idx, 10.0, 20.0, 30.0, 40.0);
            canvas.finish()
        };

        assert_eq!(page.used_images, vec![0]);
        let content = String::from_utf8(page.content).unwrap();
        assert!(content.contains("/Im1 Do"));
    }

    #[test]
    fn test_canvas_alpha_paint_emits_gs() {
        let mut fonts = PdfFontManager::new();
        let mut images = PdfImageManager::new();
        let mut transparency = TransparencyManager::new();

        let page = {
            let mut canvas = mk_canvas(&mut fonts, &mut images, &mut transparency);
            let mut paint = Paint::new();
            paint.set_color32(Color::from_argb(128, 255, 0, 0));
            canvas.draw_rect(&Rect::from_xywh(0.0, 0.0, 10.0, 10.0), &paint);
            canvas.finish()
        };

        assert!(!page.used_ext_gstates.is_empty());
        let content = String::from_utf8(page.content).unwrap();
        assert!(
            content.contains("/GS1 gs"),
            "content missing ExtGState reference: {}",
            content
        );
    }

    #[test]
    fn test_escape_pdf_string_controls() {
        assert_eq!(escape_pdf_string("a\nb"), "a\\nb");
        assert_eq!(escape_pdf_string("a\rb"), "a\\rb");
        assert_eq!(escape_pdf_string("a\tb"), "a\\tb");
        assert_eq!(escape_pdf_string("(hi)"), "\\(hi\\)");
        assert_eq!(escape_pdf_string("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn test_utf16be_hex() {
        assert_eq!(utf16be_hex("A"), "FEFF0041");
        assert_eq!(utf16be_hex("é"), "FEFF00E9");
    }
}
