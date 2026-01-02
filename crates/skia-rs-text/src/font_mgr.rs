//! Font manager for system font enumeration and fallback.
//!
//! This module provides functionality for:
//! - Enumerating system fonts
//! - Font family enumeration
//! - Font style set management
//! - Font fallback chains

use crate::typeface::{FontStyle, Typeface, TypefaceRef};
use std::sync::Arc;

/// Font manager for system font enumeration.
///
/// Corresponds to Skia's `SkFontMgr`.
pub trait FontMgr: Send + Sync {
    /// Get the number of font families.
    fn count_families(&self) -> usize;

    /// Get the family name at the given index.
    fn family_name(&self, index: usize) -> Option<String>;

    /// Create a style set for the given family name.
    fn create_style_set(&self, family_name: &str) -> Option<Box<dyn FontStyleSet>>;

    /// Match a family name and style to a typeface.
    fn match_family_style(&self, family_name: &str, style: FontStyle) -> Option<TypefaceRef>;

    /// Match a character to find a fallback font.
    fn match_family_style_character(
        &self,
        family_name: &str,
        style: FontStyle,
        bcp47: &[&str],
        character: char,
    ) -> Option<TypefaceRef>;

    /// Create a typeface from font data.
    fn make_from_data(&self, data: &[u8], index: i32) -> Option<TypefaceRef>;

    /// Create a typeface from a file.
    fn make_from_file(&self, path: &str, index: i32) -> Option<TypefaceRef>;
}

/// A set of font styles within a family.
///
/// Corresponds to Skia's `SkFontStyleSet`.
pub trait FontStyleSet: Send + Sync {
    /// Get the number of styles in this set.
    fn count(&self) -> usize;

    /// Get the style at the given index.
    fn style(&self, index: usize) -> Option<(FontStyle, String)>;

    /// Create a typeface for the style at the given index.
    fn create_typeface(&self, index: usize) -> Option<TypefaceRef>;

    /// Match a style to the closest typeface in this set.
    fn match_style(&self, style: FontStyle) -> Option<TypefaceRef>;
}

/// Default font manager implementation.
///
/// This provides basic font management with an in-memory font registry.
#[derive(Default)]
pub struct DefaultFontMgr {
    families: Vec<FontFamily>,
}

/// A font family with multiple styles.
#[derive(Clone)]
pub struct FontFamily {
    /// Family name.
    pub name: String,
    /// Typefaces in this family.
    pub typefaces: Vec<TypefaceEntry>,
}

/// Entry for a typeface with style information.
#[derive(Clone)]
pub struct TypefaceEntry {
    /// The typeface.
    pub typeface: TypefaceRef,
    /// Style name (e.g., "Regular", "Bold", "Italic").
    pub style_name: String,
    /// Font style.
    pub style: FontStyle,
}

impl DefaultFontMgr {
    /// Create a new default font manager.
    pub fn new() -> Self {
        let mut mgr = Self { families: Vec::new() };

        // Add a default family with a placeholder typeface
        let default_typeface = Arc::new(Typeface::default_typeface());
        mgr.families.push(FontFamily {
            name: "Default".to_string(),
            typefaces: vec![TypefaceEntry {
                typeface: default_typeface,
                style_name: "Regular".to_string(),
                style: FontStyle::default(),
            }],
        });

        mgr
    }

    /// Register a font family.
    pub fn register_family(&mut self, family: FontFamily) {
        self.families.push(family);
    }

    /// Register a typeface under a family name.
    pub fn register_typeface(&mut self, family_name: &str, typeface: TypefaceRef, style_name: &str, style: FontStyle) {
        // Find or create family
        let family = if let Some(f) = self.families.iter_mut().find(|f| f.name == family_name) {
            f
        } else {
            self.families.push(FontFamily {
                name: family_name.to_string(),
                typefaces: Vec::new(),
            });
            self.families.last_mut().unwrap()
        };

        family.typefaces.push(TypefaceEntry {
            typeface,
            style_name: style_name.to_string(),
            style,
        });
    }
}

impl FontMgr for DefaultFontMgr {
    fn count_families(&self) -> usize {
        self.families.len()
    }

    fn family_name(&self, index: usize) -> Option<String> {
        self.families.get(index).map(|f| f.name.clone())
    }

    fn create_style_set(&self, family_name: &str) -> Option<Box<dyn FontStyleSet>> {
        self.families
            .iter()
            .find(|f| f.name == family_name)
            .map(|f| Box::new(DefaultFontStyleSet { family: f.clone() }) as Box<dyn FontStyleSet>)
    }

    fn match_family_style(&self, family_name: &str, style: FontStyle) -> Option<TypefaceRef> {
        self.create_style_set(family_name)?
            .match_style(style)
    }

    fn match_family_style_character(
        &self,
        family_name: &str,
        style: FontStyle,
        _bcp47: &[&str],
        _character: char,
    ) -> Option<TypefaceRef> {
        // Simple fallback: just match by family and style
        // A real implementation would check if the character is in the font
        self.match_family_style(family_name, style)
            .or_else(|| self.match_family_style("Default", style))
    }

    fn make_from_data(&self, _data: &[u8], _index: i32) -> Option<TypefaceRef> {
        // Placeholder - a real implementation would parse the font data
        Some(Arc::new(Typeface::default_typeface()))
    }

    fn make_from_file(&self, _path: &str, _index: i32) -> Option<TypefaceRef> {
        // Placeholder - a real implementation would load the font file
        Some(Arc::new(Typeface::default_typeface()))
    }
}

/// Default font style set implementation.
struct DefaultFontStyleSet {
    family: FontFamily,
}

impl FontStyleSet for DefaultFontStyleSet {
    fn count(&self) -> usize {
        self.family.typefaces.len()
    }

    fn style(&self, index: usize) -> Option<(FontStyle, String)> {
        self.family.typefaces.get(index).map(|e| (e.style, e.style_name.clone()))
    }

    fn create_typeface(&self, index: usize) -> Option<TypefaceRef> {
        self.family.typefaces.get(index).map(|e| e.typeface.clone())
    }

    fn match_style(&self, style: FontStyle) -> Option<TypefaceRef> {
        // Find best match based on style distance
        self.family.typefaces
            .iter()
            .min_by_key(|e| style_distance(&e.style, &style))
            .map(|e| e.typeface.clone())
    }
}

/// Calculate the "distance" between two font styles for matching.
fn style_distance(a: &FontStyle, b: &FontStyle) -> u32 {
    let weight_diff = (a.weight.0 as i32 - b.weight.0 as i32).unsigned_abs();
    let width_diff = (a.width.0 as i32 - b.width.0 as i32).unsigned_abs();
    let slant_diff = if a.slant == b.slant { 0 } else { 100 };

    weight_diff + width_diff * 10 + slant_diff
}

/// Font fallback chain for handling missing glyphs.
#[derive(Clone, Default)]
pub struct FontFallback {
    /// Ordered list of fallback fonts.
    fallback_chain: Vec<TypefaceRef>,
}

impl FontFallback {
    /// Create a new empty fallback chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a fallback font to the chain.
    pub fn add_fallback(&mut self, typeface: TypefaceRef) {
        self.fallback_chain.push(typeface);
    }

    /// Find a font that contains the given character.
    pub fn find_font_for_char(&self, c: char, primary: &TypefaceRef) -> TypefaceRef {
        // Check primary font first
        if primary.char_to_glyph(c) != 0 {
            return primary.clone();
        }

        // Check fallback chain
        for fallback in &self.fallback_chain {
            if fallback.char_to_glyph(c) != 0 {
                return fallback.clone();
            }
        }

        // Return primary if no fallback found
        primary.clone()
    }

    /// Get the fallback chain.
    pub fn chain(&self) -> &[TypefaceRef] {
        &self.fallback_chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_font_mgr() {
        let mgr = DefaultFontMgr::new();
        assert!(mgr.count_families() >= 1);
        assert!(mgr.family_name(0).is_some());
    }

    #[test]
    fn test_match_family_style() {
        let mgr = DefaultFontMgr::new();
        let typeface = mgr.match_family_style("Default", FontStyle::default());
        assert!(typeface.is_some());
    }

    #[test]
    fn test_font_fallback() {
        let fallback = FontFallback::new();
        let primary = Arc::new(Typeface::default_typeface());
        let result = fallback.find_font_for_char('A', &primary);
        assert!(Arc::ptr_eq(&result, &primary));
    }
}
