//! PDF font embedding support.
//!
//! This module provides font embedding for PDF documents, including:
//! - Type 1 standard fonts (14 built-in fonts)
//! - TrueType font embedding
//! - Font subsetting (basic)
//! - Unicode mapping (ToUnicode CMap)

use skia_rs_core::Scalar;
use std::collections::HashMap;

/// Font type for PDF embedding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfFontType {
    /// Type 1 standard font (14 built-in).
    Type1,
    /// TrueType font.
    TrueType,
    /// OpenType font with CFF outlines.
    OpenTypeCff,
    /// Type 0 (composite) font for CID fonts.
    Type0,
}

/// Standard PDF fonts (the 14 base fonts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardFont {
    /// Times Roman.
    TimesRoman,
    /// Times Bold.
    TimesBold,
    /// Times Italic.
    TimesItalic,
    /// Times Bold Italic.
    TimesBoldItalic,
    /// Helvetica.
    Helvetica,
    /// Helvetica Bold.
    HelveticaBold,
    /// Helvetica Oblique.
    HelveticaOblique,
    /// Helvetica Bold Oblique.
    HelveticaBoldOblique,
    /// Courier.
    Courier,
    /// Courier Bold.
    CourierBold,
    /// Courier Oblique.
    CourierOblique,
    /// Courier Bold Oblique.
    CourierBoldOblique,
    /// Symbol.
    Symbol,
    /// Zapf Dingbats.
    ZapfDingbats,
}

impl StandardFont {
    /// Get the PDF base font name.
    pub fn pdf_name(&self) -> &'static str {
        match self {
            Self::TimesRoman => "Times-Roman",
            Self::TimesBold => "Times-Bold",
            Self::TimesItalic => "Times-Italic",
            Self::TimesBoldItalic => "Times-BoldItalic",
            Self::Helvetica => "Helvetica",
            Self::HelveticaBold => "Helvetica-Bold",
            Self::HelveticaOblique => "Helvetica-Oblique",
            Self::HelveticaBoldOblique => "Helvetica-BoldOblique",
            Self::Courier => "Courier",
            Self::CourierBold => "Courier-Bold",
            Self::CourierOblique => "Courier-Oblique",
            Self::CourierBoldOblique => "Courier-BoldOblique",
            Self::Symbol => "Symbol",
            Self::ZapfDingbats => "ZapfDingbats",
        }
    }

    /// Get the encoding.
    pub fn encoding(&self) -> &'static str {
        match self {
            Self::Symbol | Self::ZapfDingbats => "StandardEncoding",
            _ => "WinAnsiEncoding",
        }
    }
}

/// A PDF font resource.
#[derive(Debug, Clone)]
pub struct PdfFont {
    /// Font type.
    pub font_type: PdfFontType,
    /// Base font name.
    pub base_font: String,
    /// Object ID (assigned when writing).
    pub object_id: Option<u32>,
    /// Font descriptor object ID.
    pub descriptor_id: Option<u32>,
    /// Font encoding.
    pub encoding: String,
    /// Embedded font data (for TrueType/OpenType).
    pub font_data: Option<Vec<u8>>,
    /// Font flags.
    pub flags: u32,
    /// Italic angle.
    pub italic_angle: Scalar,
    /// Ascender.
    pub ascender: Scalar,
    /// Descender.
    pub descender: Scalar,
    /// Cap height.
    pub cap_height: Scalar,
    /// Stem vertical width.
    pub stem_v: Scalar,
    /// Font bounding box.
    pub bbox: [Scalar; 4],
    /// Character widths.
    pub widths: HashMap<u16, u16>,
    /// First character code.
    pub first_char: u16,
    /// Last character code.
    pub last_char: u16,
    /// Used glyphs (for subsetting).
    pub used_glyphs: Vec<u16>,
    /// ToUnicode CMap.
    pub to_unicode: Option<String>,
}

impl PdfFont {
    /// Create a new standard Type 1 font.
    pub fn standard(font: StandardFont) -> Self {
        Self {
            font_type: PdfFontType::Type1,
            base_font: font.pdf_name().to_string(),
            object_id: None,
            descriptor_id: None,
            encoding: font.encoding().to_string(),
            font_data: None,
            flags: 0,
            italic_angle: 0.0,
            ascender: 750.0,
            descender: -250.0,
            cap_height: 700.0,
            stem_v: 80.0,
            bbox: [-200.0, -300.0, 1200.0, 1000.0],
            widths: HashMap::new(),
            first_char: 32,
            last_char: 255,
            used_glyphs: Vec::new(),
            to_unicode: None,
        }
    }

    /// Create a TrueType font from font data.
    pub fn truetype(name: &str, data: Vec<u8>) -> Self {
        // Parse basic TrueType metrics (simplified)
        let metrics = parse_truetype_metrics(&data);

        Self {
            font_type: PdfFontType::TrueType,
            base_font: name.to_string(),
            object_id: None,
            descriptor_id: None,
            encoding: "WinAnsiEncoding".to_string(),
            font_data: Some(data),
            flags: metrics.flags,
            italic_angle: metrics.italic_angle,
            ascender: metrics.ascender,
            descender: metrics.descender,
            cap_height: metrics.cap_height,
            stem_v: metrics.stem_v,
            bbox: metrics.bbox,
            widths: metrics.widths,
            first_char: 32,
            last_char: 255,
            used_glyphs: Vec::new(),
            to_unicode: None,
        }
    }

    /// Mark a glyph as used (for subsetting).
    pub fn use_glyph(&mut self, glyph_id: u16) {
        if !self.used_glyphs.contains(&glyph_id) {
            self.used_glyphs.push(glyph_id);
        }
    }

    /// Generate the font dictionary PDF object.
    pub fn to_pdf_dict(&self, id: u32) -> String {
        let mut dict = format!("{} 0 obj\n<<\n", id);

        match self.font_type {
            PdfFontType::Type1 => {
                dict.push_str("/Type /Font\n");
                dict.push_str("/Subtype /Type1\n");
                dict.push_str(&format!("/BaseFont /{}\n", self.base_font));
                dict.push_str(&format!("/Encoding /{}\n", self.encoding));
            }
            PdfFontType::TrueType => {
                dict.push_str("/Type /Font\n");
                dict.push_str("/Subtype /TrueType\n");
                dict.push_str(&format!("/BaseFont /{}\n", self.base_font.replace(' ', "")));
                dict.push_str(&format!("/FirstChar {}\n", self.first_char));
                dict.push_str(&format!("/LastChar {}\n", self.last_char));

                // Widths array
                dict.push_str("/Widths [");
                for i in self.first_char..=self.last_char {
                    let width = self.widths.get(&i).copied().unwrap_or(600);
                    dict.push_str(&format!("{} ", width));
                }
                dict.push_str("]\n");

                if let Some(desc_id) = self.descriptor_id {
                    dict.push_str(&format!("/FontDescriptor {} 0 R\n", desc_id));
                }
                dict.push_str(&format!("/Encoding /{}\n", self.encoding));
            }
            PdfFontType::OpenTypeCff | PdfFontType::Type0 => {
                // Composite font handling (simplified)
                dict.push_str("/Type /Font\n");
                dict.push_str("/Subtype /Type0\n");
                dict.push_str(&format!("/BaseFont /{}\n", self.base_font));
                dict.push_str("/Encoding /Identity-H\n");
            }
        }

        dict.push_str(">>\nendobj\n");
        dict
    }

    /// Generate the font descriptor PDF object.
    pub fn to_font_descriptor(&self, id: u32, font_file_id: Option<u32>) -> String {
        let mut dict = format!("{} 0 obj\n<<\n", id);
        dict.push_str("/Type /FontDescriptor\n");
        dict.push_str(&format!("/FontName /{}\n", self.base_font.replace(' ', "")));
        dict.push_str(&format!("/Flags {}\n", self.flags | 32)); // Non-symbolic
        dict.push_str(&format!(
            "/FontBBox [{} {} {} {}]\n",
            self.bbox[0] as i32,
            self.bbox[1] as i32,
            self.bbox[2] as i32,
            self.bbox[3] as i32
        ));
        dict.push_str(&format!("/ItalicAngle {}\n", self.italic_angle as i32));
        dict.push_str(&format!("/Ascent {}\n", self.ascender as i32));
        dict.push_str(&format!("/Descent {}\n", self.descender as i32));
        dict.push_str(&format!("/CapHeight {}\n", self.cap_height as i32));
        dict.push_str(&format!("/StemV {}\n", self.stem_v as i32));

        if let Some(file_id) = font_file_id {
            match self.font_type {
                PdfFontType::TrueType => {
                    dict.push_str(&format!("/FontFile2 {} 0 R\n", file_id));
                }
                PdfFontType::OpenTypeCff => {
                    dict.push_str(&format!("/FontFile3 {} 0 R\n", file_id));
                }
                _ => {}
            }
        }

        dict.push_str(">>\nendobj\n");
        dict
    }

    /// Generate ToUnicode CMap for proper text extraction.
    pub fn generate_to_unicode(&self) -> String {
        let mut cmap = String::new();
        cmap.push_str("/CIDInit /ProcSet findresource begin\n");
        cmap.push_str("12 dict begin\n");
        cmap.push_str("begincmap\n");
        cmap.push_str("/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n");
        cmap.push_str("/CMapName /Adobe-Identity-UCS def\n");
        cmap.push_str("/CMapType 2 def\n");
        cmap.push_str("1 begincodespacerange\n");
        cmap.push_str("<0000> <FFFF>\n");
        cmap.push_str("endcodespacerange\n");

        // Simple ASCII mapping
        cmap.push_str("95 beginbfchar\n");
        for i in 32..127 {
            cmap.push_str(&format!("<{:04X}> <{:04X}>\n", i, i));
        }
        cmap.push_str("endbfchar\n");

        cmap.push_str("endcmap\n");
        cmap.push_str("CMapName currentdict /CMap defineresource pop\n");
        cmap.push_str("end\n");
        cmap.push_str("end\n");

        cmap
    }
}

/// Parsed TrueType metrics.
struct TrueTypeMetrics {
    flags: u32,
    italic_angle: Scalar,
    ascender: Scalar,
    descender: Scalar,
    cap_height: Scalar,
    stem_v: Scalar,
    bbox: [Scalar; 4],
    widths: HashMap<u16, u16>,
}

/// Parse basic metrics from TrueType font data.
fn parse_truetype_metrics(data: &[u8]) -> TrueTypeMetrics {
    // Simplified TrueType parsing - returns defaults
    // A full implementation would parse the font tables

    let mut metrics = TrueTypeMetrics {
        flags: 0,
        italic_angle: 0.0,
        ascender: 750.0,
        descender: -250.0,
        cap_height: 700.0,
        stem_v: 80.0,
        bbox: [0.0, -250.0, 1000.0, 750.0],
        widths: HashMap::new(),
    };

    // Check if this looks like a TrueType font
    if data.len() < 12 {
        return metrics;
    }

    // Read sfnt version
    let version = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);

    // 0x00010000 = TrueType, "OTTO" = OpenType CFF
    if version != 0x00010000 && version != 0x4F54544F {
        return metrics;
    }

    // Default widths (600 units for most characters)
    for i in 32u16..=255 {
        metrics.widths.insert(i, 600);
    }

    metrics
}

/// Font manager for PDF documents.
#[derive(Debug, Default)]
pub struct PdfFontManager {
    /// Registered fonts.
    fonts: Vec<PdfFont>,
    /// Font name to index mapping.
    name_to_index: HashMap<String, usize>,
}

impl PdfFontManager {
    /// Create a new font manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a standard font.
    pub fn register_standard(&mut self, font: StandardFont) -> usize {
        let name = font.pdf_name().to_string();
        if let Some(&idx) = self.name_to_index.get(&name) {
            return idx;
        }

        let idx = self.fonts.len();
        self.fonts.push(PdfFont::standard(font));
        self.name_to_index.insert(name, idx);
        idx
    }

    /// Register a TrueType font.
    pub fn register_truetype(&mut self, name: &str, data: Vec<u8>) -> usize {
        if let Some(&idx) = self.name_to_index.get(name) {
            return idx;
        }

        let idx = self.fonts.len();
        self.fonts.push(PdfFont::truetype(name, data));
        self.name_to_index.insert(name.to_string(), idx);
        idx
    }

    /// Get font by index.
    pub fn get(&self, index: usize) -> Option<&PdfFont> {
        self.fonts.get(index)
    }

    /// Get mutable font by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PdfFont> {
        self.fonts.get_mut(index)
    }

    /// Get font by name.
    pub fn get_by_name(&self, name: &str) -> Option<&PdfFont> {
        self.name_to_index.get(name).and_then(|&idx| self.fonts.get(idx))
    }

    /// Get all fonts.
    pub fn fonts(&self) -> &[PdfFont] {
        &self.fonts
    }

    /// Get mutable fonts.
    pub fn fonts_mut(&mut self) -> &mut [PdfFont] {
        &mut self.fonts
    }

    /// Get number of fonts.
    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_font() {
        let font = PdfFont::standard(StandardFont::Helvetica);
        assert_eq!(font.base_font, "Helvetica");
        assert_eq!(font.font_type, PdfFontType::Type1);
    }

    #[test]
    fn test_font_manager() {
        let mut manager = PdfFontManager::new();

        let idx1 = manager.register_standard(StandardFont::Helvetica);
        let idx2 = manager.register_standard(StandardFont::Helvetica); // Same font

        assert_eq!(idx1, idx2); // Should return same index
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_font_pdf_dict() {
        let font = PdfFont::standard(StandardFont::TimesRoman);
        let dict = font.to_pdf_dict(5);

        assert!(dict.contains("/Type /Font"));
        assert!(dict.contains("/BaseFont /Times-Roman"));
    }
}
