//! Typeface (font face) abstraction.
//!
//! A typeface represents a specific font file or font family member.

use std::sync::Arc;

/// Font weight (100-900).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    /// Invisible weight.
    pub const INVISIBLE: Self = Self(0);
    /// Thin weight (100).
    pub const THIN: Self = Self(100);
    /// Extra-light weight (200).
    pub const EXTRA_LIGHT: Self = Self(200);
    /// Light weight (300).
    pub const LIGHT: Self = Self(300);
    /// Normal weight (400).
    pub const NORMAL: Self = Self(400);
    /// Medium weight (500).
    pub const MEDIUM: Self = Self(500);
    /// Semi-bold weight (600).
    pub const SEMI_BOLD: Self = Self(600);
    /// Bold weight (700).
    pub const BOLD: Self = Self(700);
    /// Extra-bold weight (800).
    pub const EXTRA_BOLD: Self = Self(800);
    /// Black weight (900).
    pub const BLACK: Self = Self(900);
    /// Extra-black weight (1000).
    pub const EXTRA_BLACK: Self = Self(1000);
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Font width (condensed to expanded).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontWidth(pub u8);

impl FontWidth {
    /// Ultra-condensed (1).
    pub const ULTRA_CONDENSED: Self = Self(1);
    /// Extra-condensed (2).
    pub const EXTRA_CONDENSED: Self = Self(2);
    /// Condensed (3).
    pub const CONDENSED: Self = Self(3);
    /// Semi-condensed (4).
    pub const SEMI_CONDENSED: Self = Self(4);
    /// Normal (5).
    pub const NORMAL: Self = Self(5);
    /// Semi-expanded (6).
    pub const SEMI_EXPANDED: Self = Self(6);
    /// Expanded (7).
    pub const EXPANDED: Self = Self(7);
    /// Extra-expanded (8).
    pub const EXTRA_EXPANDED: Self = Self(8);
    /// Ultra-expanded (9).
    pub const ULTRA_EXPANDED: Self = Self(9);
}

impl Default for FontWidth {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Font slant (upright, italic, oblique).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FontSlant {
    /// Upright (roman).
    #[default]
    Upright = 0,
    /// Italic.
    Italic,
    /// Oblique.
    Oblique,
}

/// Font style combining weight, width, and slant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontStyle {
    /// Weight.
    pub weight: FontWeight,
    /// Width.
    pub width: FontWidth,
    /// Slant.
    pub slant: FontSlant,
}

impl FontStyle {
    /// Normal style.
    pub const NORMAL: Self = Self {
        weight: FontWeight::NORMAL,
        width: FontWidth::NORMAL,
        slant: FontSlant::Upright,
    };

    /// Bold style.
    pub const BOLD: Self = Self {
        weight: FontWeight::BOLD,
        width: FontWidth::NORMAL,
        slant: FontSlant::Upright,
    };

    /// Italic style.
    pub const ITALIC: Self = Self {
        weight: FontWeight::NORMAL,
        width: FontWidth::NORMAL,
        slant: FontSlant::Italic,
    };

    /// Bold italic style.
    pub const BOLD_ITALIC: Self = Self {
        weight: FontWeight::BOLD,
        width: FontWidth::NORMAL,
        slant: FontSlant::Italic,
    };

    /// Create a new font style.
    pub const fn new(weight: FontWeight, width: FontWidth, slant: FontSlant) -> Self {
        Self {
            weight,
            width,
            slant,
        }
    }
}

/// A typeface (font face) represents a specific font.
///
/// Corresponds to Skia's `SkTypeface`.
#[derive(Debug, Clone)]
pub struct Typeface {
    /// Font family name.
    family_name: String,
    /// Font style.
    style: FontStyle,
    /// Unique ID.
    id: u32,
    /// Font data (if loaded from bytes).
    data: Option<Arc<Vec<u8>>>,
    /// Units per EM.
    units_per_em: u16,
    /// Number of glyphs.
    glyph_count: u16,
}

impl Typeface {
    /// Create a default typeface.
    pub fn default_typeface() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

        Self {
            family_name: "sans-serif".to_string(),
            style: FontStyle::NORMAL,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            data: None,
            units_per_em: 2048,
            glyph_count: 256,
        }
    }

    /// Create a typeface from font data.
    pub fn from_data(data: Vec<u8>) -> Option<Self> {
        static NEXT_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

        if data.len() < 12 {
            return None;
        }

        Some(Self {
            family_name: "Unknown".to_string(),
            style: FontStyle::NORMAL,
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            data: Some(Arc::new(data)),
            units_per_em: 2048,
            glyph_count: 256,
        })
    }

    /// Get the family name.
    #[inline]
    pub fn family_name(&self) -> &str {
        &self.family_name
    }

    /// Get the font style.
    #[inline]
    pub fn style(&self) -> FontStyle {
        self.style
    }

    /// Get the unique ID.
    #[inline]
    pub fn unique_id(&self) -> u32 {
        self.id
    }

    /// Get units per EM.
    #[inline]
    pub fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    /// Get the number of glyphs.
    #[inline]
    pub fn glyph_count(&self) -> u16 {
        self.glyph_count
    }

    /// Check if this is a bold typeface.
    #[inline]
    pub fn is_bold(&self) -> bool {
        self.style.weight.0 >= FontWeight::BOLD.0
    }

    /// Check if this is an italic typeface.
    #[inline]
    pub fn is_italic(&self) -> bool {
        self.style.slant != FontSlant::Upright
    }

    /// Check if this typeface has fixed width glyphs.
    #[inline]
    pub fn is_fixed_pitch(&self) -> bool {
        // Would need to parse font tables to determine this
        false
    }

    /// Get the glyph ID for a character.
    pub fn char_to_glyph(&self, c: char) -> u16 {
        // Simple ASCII mapping for now
        // A real implementation would use font tables
        if c.is_ascii() {
            c as u16
        } else {
            0 // .notdef glyph
        }
    }

    /// Get glyph IDs for a string.
    pub fn chars_to_glyphs(&self, chars: &str) -> Vec<u16> {
        chars.chars().map(|c| self.char_to_glyph(c)).collect()
    }

    /// Get direct access to font data (for shaping).
    pub fn font_data(&self) -> Option<&[u8]> {
        self.data.as_ref().map(|d| d.as_slice())
    }
}

/// A reference to a typeface (shared ownership).
pub type TypefaceRef = Arc<Typeface>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_style() {
        let normal = FontStyle::NORMAL;
        assert_eq!(normal.weight, FontWeight::NORMAL);
        assert_eq!(normal.slant, FontSlant::Upright);

        let bold = FontStyle::BOLD;
        assert_eq!(bold.weight, FontWeight::BOLD);
    }

    #[test]
    fn test_typeface_default() {
        let tf = Typeface::default_typeface();
        assert_eq!(tf.family_name(), "sans-serif");
        assert!(!tf.is_bold());
        assert!(!tf.is_italic());
    }

    #[test]
    fn test_char_to_glyph() {
        let tf = Typeface::default_typeface();
        assert_eq!(tf.char_to_glyph('A'), 65);
        assert_eq!(tf.char_to_glyph('a'), 97);
    }
}
