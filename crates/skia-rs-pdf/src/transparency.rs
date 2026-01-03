//! PDF transparency support.
//!
//! This module provides transparency features for PDF documents, including:
//! - Extended Graphics State (ExtGState) for opacity
//! - Transparency groups
//! - Soft masks
//! - Blend modes

use skia_rs_core::Scalar;
use skia_rs_paint::BlendMode;
use std::collections::HashMap;

/// Extended Graphics State for PDF transparency.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtGraphicsState {
    /// Stroking alpha (CA).
    pub stroke_alpha: Option<Scalar>,
    /// Non-stroking alpha (ca).
    pub fill_alpha: Option<Scalar>,
    /// Blend mode (BM).
    pub blend_mode: Option<PdfBlendMode>,
    /// Soft mask dictionary reference.
    pub soft_mask: Option<u32>,
    /// Alpha is shape (AIS).
    pub alpha_is_shape: Option<bool>,
    /// Text knockout (TK).
    pub text_knockout: Option<bool>,
    /// Overprint for stroking (OP).
    pub stroke_overprint: Option<bool>,
    /// Overprint for non-stroking (op).
    pub fill_overprint: Option<bool>,
    /// Object ID (assigned when writing).
    pub object_id: Option<u32>,
}

impl Default for ExtGraphicsState {
    fn default() -> Self {
        Self {
            stroke_alpha: None,
            fill_alpha: None,
            blend_mode: None,
            soft_mask: None,
            alpha_is_shape: None,
            text_knockout: None,
            stroke_overprint: None,
            fill_overprint: None,
            object_id: None,
        }
    }
}

impl ExtGraphicsState {
    /// Create a new ExtGraphicsState with fill and stroke alpha.
    pub fn with_alpha(alpha: Scalar) -> Self {
        Self {
            stroke_alpha: Some(alpha),
            fill_alpha: Some(alpha),
            ..Default::default()
        }
    }

    /// Create a new ExtGraphicsState with blend mode.
    pub fn with_blend_mode(mode: PdfBlendMode) -> Self {
        Self {
            blend_mode: Some(mode),
            ..Default::default()
        }
    }

    /// Set fill alpha.
    pub fn set_fill_alpha(&mut self, alpha: Scalar) -> &mut Self {
        self.fill_alpha = Some(alpha);
        self
    }

    /// Set stroke alpha.
    pub fn set_stroke_alpha(&mut self, alpha: Scalar) -> &mut Self {
        self.stroke_alpha = Some(alpha);
        self
    }

    /// Set blend mode.
    pub fn set_blend_mode(&mut self, mode: PdfBlendMode) -> &mut Self {
        self.blend_mode = Some(mode);
        self
    }

    /// Set soft mask reference.
    pub fn set_soft_mask(&mut self, mask_id: u32) -> &mut Self {
        self.soft_mask = Some(mask_id);
        self
    }

    /// Generate the ExtGState PDF dictionary.
    pub fn to_pdf_dict(&self, id: u32) -> String {
        let mut dict = format!("{} 0 obj\n<<\n", id);
        dict.push_str("/Type /ExtGState\n");
        
        if let Some(alpha) = self.stroke_alpha {
            dict.push_str(&format!("/CA {:.3}\n", alpha));
        }
        
        if let Some(alpha) = self.fill_alpha {
            dict.push_str(&format!("/ca {:.3}\n", alpha));
        }
        
        if let Some(mode) = &self.blend_mode {
            dict.push_str(&format!("/BM /{}\n", mode.pdf_name()));
        }
        
        if let Some(mask_id) = self.soft_mask {
            dict.push_str(&format!("/SMask {} 0 R\n", mask_id));
        }
        
        if let Some(ais) = self.alpha_is_shape {
            dict.push_str(&format!("/AIS {}\n", if ais { "true" } else { "false" }));
        }
        
        if let Some(tk) = self.text_knockout {
            dict.push_str(&format!("/TK {}\n", if tk { "true" } else { "false" }));
        }
        
        if let Some(op) = self.stroke_overprint {
            dict.push_str(&format!("/OP {}\n", if op { "true" } else { "false" }));
        }
        
        if let Some(op) = self.fill_overprint {
            dict.push_str(&format!("/op {}\n", if op { "true" } else { "false" }));
        }
        
        dict.push_str(">>\nendobj\n");
        dict
    }

    /// Create a unique key for caching.
    pub fn cache_key(&self) -> ExtGStateKey {
        ExtGStateKey {
            stroke_alpha: self.stroke_alpha.map(|a| (a * 1000.0) as i32),
            fill_alpha: self.fill_alpha.map(|a| (a * 1000.0) as i32),
            blend_mode: self.blend_mode,
        }
    }
}

/// Key for caching ExtGraphicsState objects.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExtGStateKey {
    /// Stroking alpha (scaled to int for hashing).
    stroke_alpha: Option<i32>,
    /// Non-stroking alpha (scaled to int for hashing).
    fill_alpha: Option<i32>,
    /// Blend mode.
    blend_mode: Option<PdfBlendMode>,
}

/// PDF blend modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PdfBlendMode {
    /// Normal (default).
    Normal,
    /// Multiply.
    Multiply,
    /// Screen.
    Screen,
    /// Overlay.
    Overlay,
    /// Darken.
    Darken,
    /// Lighten.
    Lighten,
    /// Color dodge.
    ColorDodge,
    /// Color burn.
    ColorBurn,
    /// Hard light.
    HardLight,
    /// Soft light.
    SoftLight,
    /// Difference.
    Difference,
    /// Exclusion.
    Exclusion,
    /// Hue.
    Hue,
    /// Saturation.
    Saturation,
    /// Color.
    Color,
    /// Luminosity.
    Luminosity,
}

impl PdfBlendMode {
    /// Get the PDF name for this blend mode.
    pub fn pdf_name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Overlay => "Overlay",
            Self::Darken => "Darken",
            Self::Lighten => "Lighten",
            Self::ColorDodge => "ColorDodge",
            Self::ColorBurn => "ColorBurn",
            Self::HardLight => "HardLight",
            Self::SoftLight => "SoftLight",
            Self::Difference => "Difference",
            Self::Exclusion => "Exclusion",
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Color => "Color",
            Self::Luminosity => "Luminosity",
        }
    }

    /// Convert from skia BlendMode.
    pub fn from_skia_blend_mode(mode: BlendMode) -> Option<Self> {
        match mode {
            BlendMode::SrcOver => Some(Self::Normal),
            BlendMode::Multiply => Some(Self::Multiply),
            BlendMode::Screen => Some(Self::Screen),
            BlendMode::Overlay => Some(Self::Overlay),
            BlendMode::Darken => Some(Self::Darken),
            BlendMode::Lighten => Some(Self::Lighten),
            BlendMode::ColorDodge => Some(Self::ColorDodge),
            BlendMode::ColorBurn => Some(Self::ColorBurn),
            BlendMode::HardLight => Some(Self::HardLight),
            BlendMode::SoftLight => Some(Self::SoftLight),
            BlendMode::Difference => Some(Self::Difference),
            BlendMode::Exclusion => Some(Self::Exclusion),
            BlendMode::Hue => Some(Self::Hue),
            BlendMode::Saturation => Some(Self::Saturation),
            BlendMode::Color => Some(Self::Color),
            BlendMode::Luminosity => Some(Self::Luminosity),
            _ => None, // Not directly supported
        }
    }
}

/// Soft mask dictionary for PDF transparency.
#[derive(Debug, Clone)]
pub struct SoftMask {
    /// Subtype (Alpha or Luminosity).
    pub subtype: SoftMaskSubtype,
    /// Transparency group XObject reference.
    pub group_ref: u32,
    /// Backdrop color (optional).
    pub backdrop: Option<Vec<Scalar>>,
    /// Transfer function (optional).
    pub transfer: Option<u32>,
}

/// Soft mask subtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftMaskSubtype {
    /// Alpha-based soft mask.
    Alpha,
    /// Luminosity-based soft mask.
    Luminosity,
}

impl SoftMask {
    /// Create an alpha soft mask.
    pub fn alpha(group_ref: u32) -> Self {
        Self {
            subtype: SoftMaskSubtype::Alpha,
            group_ref,
            backdrop: None,
            transfer: None,
        }
    }

    /// Create a luminosity soft mask.
    pub fn luminosity(group_ref: u32) -> Self {
        Self {
            subtype: SoftMaskSubtype::Luminosity,
            group_ref,
            backdrop: None,
            transfer: None,
        }
    }

    /// Generate the soft mask PDF dictionary.
    pub fn to_pdf_dict(&self, id: u32) -> String {
        let mut dict = format!("{} 0 obj\n<<\n", id);
        dict.push_str("/Type /Mask\n");
        
        match self.subtype {
            SoftMaskSubtype::Alpha => dict.push_str("/S /Alpha\n"),
            SoftMaskSubtype::Luminosity => dict.push_str("/S /Luminosity\n"),
        }
        
        dict.push_str(&format!("/G {} 0 R\n", self.group_ref));
        
        if let Some(ref backdrop) = self.backdrop {
            dict.push_str("/BC [");
            for val in backdrop {
                dict.push_str(&format!("{:.3} ", val));
            }
            dict.push_str("]\n");
        }
        
        if let Some(transfer_ref) = self.transfer {
            dict.push_str(&format!("/TR {} 0 R\n", transfer_ref));
        }
        
        dict.push_str(">>\nendobj\n");
        dict
    }
}

/// Transparency group for PDF.
#[derive(Debug, Clone)]
pub struct TransparencyGroup {
    /// Color space.
    pub color_space: Option<String>,
    /// Isolated flag.
    pub isolated: bool,
    /// Knockout flag.
    pub knockout: bool,
    /// Content stream.
    pub content: Vec<u8>,
    /// Bounding box.
    pub bbox: [Scalar; 4],
}

impl Default for TransparencyGroup {
    fn default() -> Self {
        Self {
            color_space: None,
            isolated: false,
            knockout: false,
            content: Vec::new(),
            bbox: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

impl TransparencyGroup {
    /// Create a new transparency group.
    pub fn new(bbox: [Scalar; 4]) -> Self {
        Self {
            bbox,
            ..Default::default()
        }
    }

    /// Set as isolated group.
    pub fn set_isolated(&mut self, isolated: bool) -> &mut Self {
        self.isolated = isolated;
        self
    }

    /// Set as knockout group.
    pub fn set_knockout(&mut self, knockout: bool) -> &mut Self {
        self.knockout = knockout;
        self
    }

    /// Set color space.
    pub fn set_color_space(&mut self, cs: &str) -> &mut Self {
        self.color_space = Some(cs.to_string());
        self
    }

    /// Generate the transparency group XObject.
    pub fn to_pdf_xobject(&self, id: u32) -> Vec<u8> {
        let mut output = Vec::new();
        
        use std::io::Write;
        write!(output, "{} 0 obj\n<<\n", id).unwrap();
        write!(output, "/Type /XObject\n").unwrap();
        write!(output, "/Subtype /Form\n").unwrap();
        write!(output, "/FormType 1\n").unwrap();
        write!(
            output,
            "/BBox [{} {} {} {}]\n",
            self.bbox[0], self.bbox[1], self.bbox[2], self.bbox[3]
        )
        .unwrap();
        
        // Transparency group dictionary
        write!(output, "/Group <<\n").unwrap();
        write!(output, "/Type /Group\n").unwrap();
        write!(output, "/S /Transparency\n").unwrap();
        
        if let Some(ref cs) = self.color_space {
            write!(output, "/CS /{}\n", cs).unwrap();
        }
        
        if self.isolated {
            write!(output, "/I true\n").unwrap();
        }
        
        if self.knockout {
            write!(output, "/K true\n").unwrap();
        }
        
        write!(output, ">>\n").unwrap();
        write!(output, "/Length {}\n", self.content.len()).unwrap();
        write!(output, ">>\nstream\n").unwrap();
        output.extend_from_slice(&self.content);
        write!(output, "\nendstream\nendobj\n").unwrap();
        
        output
    }
}

/// Manager for PDF transparency resources.
#[derive(Debug, Default)]
pub struct TransparencyManager {
    /// ExtGState objects.
    ext_gstates: Vec<ExtGraphicsState>,
    /// ExtGState cache.
    gstate_cache: HashMap<ExtGStateKey, usize>,
    /// Soft masks.
    soft_masks: Vec<SoftMask>,
    /// Transparency groups.
    groups: Vec<TransparencyGroup>,
}

impl TransparencyManager {
    /// Create a new transparency manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create an ExtGState for the given alpha value.
    pub fn get_or_create_alpha_state(&mut self, alpha: Scalar) -> usize {
        let state = ExtGraphicsState::with_alpha(alpha);
        let key = state.cache_key();
        
        if let Some(&idx) = self.gstate_cache.get(&key) {
            return idx;
        }
        
        let idx = self.ext_gstates.len();
        self.ext_gstates.push(state);
        self.gstate_cache.insert(key, idx);
        idx
    }

    /// Get or create an ExtGState for the given blend mode.
    pub fn get_or_create_blend_state(&mut self, mode: PdfBlendMode) -> usize {
        let state = ExtGraphicsState::with_blend_mode(mode);
        let key = state.cache_key();
        
        if let Some(&idx) = self.gstate_cache.get(&key) {
            return idx;
        }
        
        let idx = self.ext_gstates.len();
        self.ext_gstates.push(state);
        self.gstate_cache.insert(key, idx);
        idx
    }

    /// Add a custom ExtGState.
    pub fn add_ext_gstate(&mut self, state: ExtGraphicsState) -> usize {
        let key = state.cache_key();
        
        if let Some(&idx) = self.gstate_cache.get(&key) {
            return idx;
        }
        
        let idx = self.ext_gstates.len();
        self.ext_gstates.push(state);
        self.gstate_cache.insert(key, idx);
        idx
    }

    /// Add a soft mask.
    pub fn add_soft_mask(&mut self, mask: SoftMask) -> usize {
        let idx = self.soft_masks.len();
        self.soft_masks.push(mask);
        idx
    }

    /// Add a transparency group.
    pub fn add_group(&mut self, group: TransparencyGroup) -> usize {
        let idx = self.groups.len();
        self.groups.push(group);
        idx
    }

    /// Get all ExtGState objects.
    pub fn ext_gstates(&self) -> &[ExtGraphicsState] {
        &self.ext_gstates
    }

    /// Get mutable ExtGState objects.
    pub fn ext_gstates_mut(&mut self) -> &mut [ExtGraphicsState] {
        &mut self.ext_gstates
    }

    /// Get all soft masks.
    pub fn soft_masks(&self) -> &[SoftMask] {
        &self.soft_masks
    }

    /// Get all transparency groups.
    pub fn groups(&self) -> &[TransparencyGroup] {
        &self.groups
    }

    /// Get mutable groups.
    pub fn groups_mut(&mut self) -> &mut [TransparencyGroup] {
        &mut self.groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ext_gstate_alpha() {
        let state = ExtGraphicsState::with_alpha(0.5);
        let dict = state.to_pdf_dict(5);
        
        assert!(dict.contains("/CA 0.500"));
        assert!(dict.contains("/ca 0.500"));
    }

    #[test]
    fn test_ext_gstate_blend_mode() {
        let state = ExtGraphicsState::with_blend_mode(PdfBlendMode::Multiply);
        let dict = state.to_pdf_dict(6);
        
        assert!(dict.contains("/BM /Multiply"));
    }

    #[test]
    fn test_transparency_manager_caching() {
        let mut manager = TransparencyManager::new();
        
        let idx1 = manager.get_or_create_alpha_state(0.5);
        let idx2 = manager.get_or_create_alpha_state(0.5);
        
        assert_eq!(idx1, idx2); // Should be cached
    }

    #[test]
    fn test_soft_mask() {
        let mask = SoftMask::alpha(10);
        let dict = mask.to_pdf_dict(7);
        
        assert!(dict.contains("/S /Alpha"));
        assert!(dict.contains("/G 10 0 R"));
    }

    #[test]
    fn test_transparency_group() {
        let mut group = TransparencyGroup::new([0.0, 0.0, 100.0, 100.0]);
        group.set_isolated(true).set_knockout(false);
        
        let xobject = group.to_pdf_xobject(8);
        let content = String::from_utf8_lossy(&xobject);
        
        assert!(content.contains("/S /Transparency"));
        assert!(content.contains("/I true"));
    }

    #[test]
    fn test_blend_mode_conversion() {
        assert_eq!(
            PdfBlendMode::from_skia_blend_mode(BlendMode::Multiply),
            Some(PdfBlendMode::Multiply)
        );
        assert_eq!(
            PdfBlendMode::from_skia_blend_mode(BlendMode::SrcOver),
            Some(PdfBlendMode::Normal)
        );
    }
}
