//! PDF/A compliance support.
//!
//! This module provides functionality for generating PDF/A compliant documents
//! for long-term archival.
//!
//! # Supported Standards
//!
//! - **PDF/A-1b** (ISO 19005-1) - Basic conformance, visual appearance
//! - **PDF/A-2b** (ISO 19005-2) - Based on PDF 1.7, JPEG2000, transparency
//! - **PDF/A-3b** (ISO 19005-3) - Allows embedded files
//!
//! # Example
//!
//! ```rust,ignore
//! use skia_rs_pdf::{PdfDocument, PdfALevel, PdfAValidator};
//!
//! let mut doc = PdfDocument::new();
//! doc.set_pdfa_conformance(PdfALevel::A1b);
//!
//! // ... add content ...
//!
//! // Validate before saving
//! let validator = PdfAValidator::new(PdfALevel::A1b);
//! if let Err(errors) = validator.validate(&doc) {
//!     for error in errors {
//!         eprintln!("PDF/A violation: {}", error);
//!     }
//! }
//! ```

use std::collections::HashSet;

// =============================================================================
// PDF/A Conformance Levels
// =============================================================================

/// PDF/A conformance level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PdfALevel {
    /// PDF/A-1a: Full conformance (structure, accessibility)
    A1a,
    /// PDF/A-1b: Basic conformance (visual appearance only)
    A1b,
    /// PDF/A-2a: Full conformance based on PDF 1.7
    A2a,
    /// PDF/A-2b: Basic conformance based on PDF 1.7
    A2b,
    /// PDF/A-2u: Unicode text extraction required
    A2u,
    /// PDF/A-3a: Full conformance with embedded files
    A3a,
    /// PDF/A-3b: Basic conformance with embedded files
    A3b,
    /// PDF/A-3u: Unicode with embedded files
    A3u,
}

impl PdfALevel {
    /// Get the PDF/A part number.
    pub fn part(&self) -> u8 {
        match self {
            Self::A1a | Self::A1b => 1,
            Self::A2a | Self::A2b | Self::A2u => 2,
            Self::A3a | Self::A3b | Self::A3u => 3,
        }
    }

    /// Get the conformance level identifier.
    pub fn conformance(&self) -> &'static str {
        match self {
            Self::A1a | Self::A2a | Self::A3a => "A",
            Self::A1b | Self::A2b | Self::A3b => "B",
            Self::A2u | Self::A3u => "U",
        }
    }

    /// Get the minimum PDF version required.
    pub fn min_pdf_version(&self) -> &'static str {
        match self {
            Self::A1a | Self::A1b => "1.4",
            Self::A2a | Self::A2b | Self::A2u => "1.7",
            Self::A3a | Self::A3b | Self::A3u => "1.7",
        }
    }

    /// Check if transparency is allowed.
    pub fn allows_transparency(&self) -> bool {
        !matches!(self, Self::A1a | Self::A1b)
    }

    /// Check if embedded files are allowed.
    pub fn allows_embedded_files(&self) -> bool {
        matches!(self, Self::A3a | Self::A3b | Self::A3u)
    }

    /// Check if JPEG2000 compression is allowed.
    pub fn allows_jpeg2000(&self) -> bool {
        !matches!(self, Self::A1a | Self::A1b)
    }

    /// Check if Unicode text is required.
    pub fn requires_unicode(&self) -> bool {
        matches!(self, Self::A2u | Self::A3u)
    }

    /// Check if structure (tagged PDF) is required.
    pub fn requires_structure(&self) -> bool {
        matches!(self, Self::A1a | Self::A2a | Self::A3a)
    }
}

// =============================================================================
// PDF/A Validation Errors
// =============================================================================

/// PDF/A validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfAError {
    /// Error code.
    pub code: PdfAErrorCode,
    /// Human-readable description.
    pub message: String,
    /// Location in document (if applicable).
    pub location: Option<String>,
}

impl std::fmt::Display for PdfAError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref loc) = self.location {
            write!(f, "{:?} at {}: {}", self.code, loc, self.message)
        } else {
            write!(f, "{:?}: {}", self.code, self.message)
        }
    }
}

impl std::error::Error for PdfAError {}

/// PDF/A error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PdfAErrorCode {
    // Metadata errors
    /// Missing XMP metadata
    MissingXmpMetadata,
    /// Invalid XMP metadata
    InvalidXmpMetadata,
    /// Missing document ID
    MissingDocumentId,
    /// Missing PDF/A identification
    MissingPdfaId,
    /// PDF version mismatch
    PdfVersionMismatch,

    // Font errors
    /// Font not embedded
    FontNotEmbedded,
    /// Font missing character mapping
    FontMissingCmap,
    /// Invalid font subset
    InvalidFontSubset,
    /// Font missing glyph widths
    FontMissingWidths,

    // Color errors
    /// Device-dependent color without output intent
    DeviceColorWithoutIntent,
    /// Missing output intent
    MissingOutputIntent,
    /// Invalid ICC profile
    InvalidIccProfile,
    /// Uncalibrated color space
    UncalibratedColorSpace,

    // Image errors
    /// Image compression not allowed
    DisallowedImageCompression,
    /// LZW compression not allowed (PDF/A-1)
    LzwCompressionNotAllowed,
    /// JPEG2000 not allowed (PDF/A-1)
    Jpeg2000NotAllowed,

    // Transparency errors
    /// Transparency not allowed (PDF/A-1)
    TransparencyNotAllowed,
    /// Invalid blend mode
    InvalidBlendMode,

    // Structure errors
    /// Missing document structure (PDF/A-a levels)
    MissingDocumentStructure,
    /// Missing alternative text
    MissingAltText,
    /// Invalid structure element
    InvalidStructureElement,

    // Security errors
    /// Encryption not allowed
    EncryptionNotAllowed,
    /// JavaScript not allowed
    JavaScriptNotAllowed,
    /// Actions not allowed
    ActionsNotAllowed,

    // Embedded file errors
    /// Embedded files not allowed (PDF/A-1, PDF/A-2)
    EmbeddedFilesNotAllowed,
    /// Missing file relationship
    MissingFileRelationship,

    // Other
    /// External content references
    ExternalContentReference,
    /// Audio/video content not allowed
    MultimediaNotAllowed,
}

// =============================================================================
// XMP Metadata
// =============================================================================

/// XMP metadata for PDF/A compliance.
#[derive(Debug, Clone, Default)]
pub struct XmpMetadata {
    /// Document title.
    pub title: Option<String>,
    /// Document author.
    pub author: Option<String>,
    /// Document subject/description.
    pub subject: Option<String>,
    /// Keywords.
    pub keywords: Vec<String>,
    /// Creator application.
    pub creator: Option<String>,
    /// Creation date (ISO 8601).
    pub create_date: Option<String>,
    /// Modification date (ISO 8601).
    pub modify_date: Option<String>,
    /// PDF/A conformance level.
    pub pdfa_level: Option<PdfALevel>,
    /// Document ID (UUID).
    pub document_id: Option<String>,
    /// Instance ID (UUID).
    pub instance_id: Option<String>,
}

impl XmpMetadata {
    /// Create new XMP metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set document title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set document author.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set PDF/A level.
    pub fn with_pdfa_level(mut self, level: PdfALevel) -> Self {
        self.pdfa_level = Some(level);
        self
    }

    /// Generate XMP packet.
    pub fn to_xmp(&self) -> String {
        let mut xmp = String::new();

        xmp.push_str(r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>"#);
        xmp.push('\n');
        xmp.push_str(r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">"#);
        xmp.push('\n');
        xmp.push_str(r#"  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">"#);
        xmp.push('\n');

        // Dublin Core metadata
        xmp.push_str(
            r#"    <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/">"#,
        );
        xmp.push('\n');

        if let Some(ref title) = self.title {
            xmp.push_str(&format!(
                r#"      <dc:title><rdf:Alt><rdf:li xml:lang="x-default">{}</rdf:li></rdf:Alt></dc:title>"#,
                escape_xml(title)
            ));
            xmp.push('\n');
        }

        if let Some(ref author) = self.author {
            xmp.push_str(&format!(
                r#"      <dc:creator><rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq></dc:creator>"#,
                escape_xml(author)
            ));
            xmp.push('\n');
        }

        if let Some(ref subject) = self.subject {
            xmp.push_str(&format!(
                r#"      <dc:description><rdf:Alt><rdf:li xml:lang="x-default">{}</rdf:li></rdf:Alt></dc:description>"#,
                escape_xml(subject)
            ));
            xmp.push('\n');
        }

        xmp.push_str(r#"    </rdf:Description>"#);
        xmp.push('\n');

        // XMP Basic metadata
        xmp.push_str(
            r#"    <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/">"#,
        );
        xmp.push('\n');

        if let Some(ref creator) = self.creator {
            xmp.push_str(&format!(
                r#"      <xmp:CreatorTool>{}</xmp:CreatorTool>"#,
                escape_xml(creator)
            ));
            xmp.push('\n');
        }

        if let Some(ref create_date) = self.create_date {
            xmp.push_str(&format!(
                r#"      <xmp:CreateDate>{}</xmp:CreateDate>"#,
                create_date
            ));
            xmp.push('\n');
        }

        if let Some(ref modify_date) = self.modify_date {
            xmp.push_str(&format!(
                r#"      <xmp:ModifyDate>{}</xmp:ModifyDate>"#,
                modify_date
            ));
            xmp.push('\n');
        }

        xmp.push_str(r#"    </rdf:Description>"#);
        xmp.push('\n');

        // PDF metadata
        xmp.push_str(
            r#"    <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/">"#,
        );
        xmp.push('\n');
        xmp.push_str(r#"      <pdf:Producer>skia-rs 0.1.0</pdf:Producer>"#);
        xmp.push('\n');
        xmp.push_str(r#"    </rdf:Description>"#);
        xmp.push('\n');

        // PDF/A identification
        if let Some(level) = self.pdfa_level {
            xmp.push_str(r#"    <rdf:Description rdf:about="" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">"#);
            xmp.push('\n');
            xmp.push_str(&format!(
                r#"      <pdfaid:part>{}</pdfaid:part>"#,
                level.part()
            ));
            xmp.push('\n');
            xmp.push_str(&format!(
                r#"      <pdfaid:conformance>{}</pdfaid:conformance>"#,
                level.conformance()
            ));
            xmp.push('\n');
            xmp.push_str(r#"    </rdf:Description>"#);
            xmp.push('\n');
        }

        // XMP Media Management
        if self.document_id.is_some() || self.instance_id.is_some() {
            xmp.push_str(r#"    <rdf:Description rdf:about="" xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/">"#);
            xmp.push('\n');

            if let Some(ref doc_id) = self.document_id {
                xmp.push_str(&format!(
                    r#"      <xmpMM:DocumentID>uuid:{}</xmpMM:DocumentID>"#,
                    doc_id
                ));
                xmp.push('\n');
            }

            if let Some(ref inst_id) = self.instance_id {
                xmp.push_str(&format!(
                    r#"      <xmpMM:InstanceID>uuid:{}</xmpMM:InstanceID>"#,
                    inst_id
                ));
                xmp.push('\n');
            }

            xmp.push_str(r#"    </rdf:Description>"#);
            xmp.push('\n');
        }

        xmp.push_str(r#"  </rdf:RDF>"#);
        xmp.push('\n');
        xmp.push_str(r#"</x:xmpmeta>"#);
        xmp.push('\n');

        // Padding for in-place updates
        for _ in 0..20 {
            xmp.push_str("                                                                                \n");
        }

        xmp.push_str(r#"<?xpacket end="w"?>"#);

        xmp
    }
}

/// Escape XML special characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// =============================================================================
// Output Intent
// =============================================================================

/// PDF output intent for color management.
#[derive(Debug, Clone)]
pub struct OutputIntent {
    /// Output condition identifier.
    pub output_condition: String,
    /// Output condition identifier type.
    pub output_condition_identifier: String,
    /// Registry name (e.g., "http://www.color.org").
    pub registry_name: Option<String>,
    /// Human-readable info.
    pub info: Option<String>,
    /// ICC profile data.
    pub icc_profile: Option<Vec<u8>>,
}

impl OutputIntent {
    /// Create sRGB output intent.
    pub fn srgb() -> Self {
        Self {
            output_condition: "sRGB IEC61966-2.1".to_string(),
            output_condition_identifier: "sRGB IEC61966-2.1".to_string(),
            registry_name: Some("http://www.color.org".to_string()),
            info: Some("sRGB IEC61966-2.1".to_string()),
            icc_profile: None, // Would contain actual sRGB ICC profile
        }
    }

    /// Create FOGRA39 (coated) output intent for print.
    pub fn fogra39() -> Self {
        Self {
            output_condition: "FOGRA39".to_string(),
            output_condition_identifier: "FOGRA39L".to_string(),
            registry_name: Some("http://www.color.org".to_string()),
            info: Some("Coated FOGRA39 (ISO 12647-2:2004)".to_string()),
            icc_profile: None,
        }
    }

    /// Create custom output intent with ICC profile.
    pub fn custom(condition: &str, icc_profile: Vec<u8>) -> Self {
        Self {
            output_condition: condition.to_string(),
            output_condition_identifier: condition.to_string(),
            registry_name: Some("http://www.color.org".to_string()),
            info: None,
            icc_profile: Some(icc_profile),
        }
    }
}

// =============================================================================
// PDF/A Validator
// =============================================================================

/// PDF/A compliance validator.
pub struct PdfAValidator {
    level: PdfALevel,
    errors: Vec<PdfAError>,
}

impl PdfAValidator {
    /// Create a new validator for the specified level.
    pub fn new(level: PdfALevel) -> Self {
        Self {
            level,
            errors: Vec::new(),
        }
    }

    /// Validate a document and return errors.
    pub fn validate(&mut self, doc: &PdfADocument) -> Result<(), Vec<PdfAError>> {
        self.errors.clear();

        self.check_metadata(doc);
        self.check_fonts(doc);
        self.check_colors(doc);
        self.check_images(doc);
        self.check_transparency(doc);
        self.check_structure(doc);
        self.check_security(doc);
        self.check_embedded_files(doc);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    fn check_metadata(&mut self, doc: &PdfADocument) {
        if doc.xmp_metadata.is_none() {
            self.errors.push(PdfAError {
                code: PdfAErrorCode::MissingXmpMetadata,
                message: "XMP metadata stream is required".to_string(),
                location: None,
            });
        }

        if doc.document_id.is_none() {
            self.errors.push(PdfAError {
                code: PdfAErrorCode::MissingDocumentId,
                message: "Document ID is required in trailer".to_string(),
                location: None,
            });
        }

        if let Some(ref xmp) = doc.xmp_metadata {
            if xmp.pdfa_level.is_none() {
                self.errors.push(PdfAError {
                    code: PdfAErrorCode::MissingPdfaId,
                    message: "PDF/A identification in XMP metadata is required".to_string(),
                    location: None,
                });
            }
        }
    }

    fn check_fonts(&mut self, doc: &PdfADocument) {
        for (name, font) in &doc.fonts {
            if !font.is_embedded {
                self.errors.push(PdfAError {
                    code: PdfAErrorCode::FontNotEmbedded,
                    message: format!("Font '{}' must be embedded", name),
                    location: Some(format!("Font: {}", name)),
                });
            }

            if !font.has_cmap {
                self.errors.push(PdfAError {
                    code: PdfAErrorCode::FontMissingCmap,
                    message: format!("Font '{}' missing ToUnicode CMap", name),
                    location: Some(format!("Font: {}", name)),
                });
            }

            if !font.has_widths {
                self.errors.push(PdfAError {
                    code: PdfAErrorCode::FontMissingWidths,
                    message: format!("Font '{}' missing glyph widths", name),
                    location: Some(format!("Font: {}", name)),
                });
            }
        }
    }

    fn check_colors(&mut self, doc: &PdfADocument) {
        if doc.output_intent.is_none() && doc.uses_device_colors {
            self.errors.push(PdfAError {
                code: PdfAErrorCode::MissingOutputIntent,
                message: "Output intent required when using device-dependent colors".to_string(),
                location: None,
            });
        }

        for color_space in &doc.uncalibrated_colors {
            self.errors.push(PdfAError {
                code: PdfAErrorCode::UncalibratedColorSpace,
                message: format!("Uncalibrated color space '{}' not allowed", color_space),
                location: Some(format!("ColorSpace: {}", color_space)),
            });
        }
    }

    fn check_images(&mut self, doc: &PdfADocument) {
        if !self.level.allows_jpeg2000() && doc.uses_jpeg2000 {
            self.errors.push(PdfAError {
                code: PdfAErrorCode::Jpeg2000NotAllowed,
                message: "JPEG2000 compression not allowed in PDF/A-1".to_string(),
                location: None,
            });
        }

        if doc.uses_lzw {
            self.errors.push(PdfAError {
                code: PdfAErrorCode::LzwCompressionNotAllowed,
                message: "LZW compression not allowed in PDF/A".to_string(),
                location: None,
            });
        }
    }

    fn check_transparency(&mut self, doc: &PdfADocument) {
        if !self.level.allows_transparency() && doc.uses_transparency {
            self.errors.push(PdfAError {
                code: PdfAErrorCode::TransparencyNotAllowed,
                message: "Transparency not allowed in PDF/A-1".to_string(),
                location: None,
            });
        }
    }

    fn check_structure(&mut self, doc: &PdfADocument) {
        if self.level.requires_structure() && !doc.has_structure {
            self.errors.push(PdfAError {
                code: PdfAErrorCode::MissingDocumentStructure,
                message: "Tagged PDF structure required for PDF/A-a conformance".to_string(),
                location: None,
            });
        }
    }

    fn check_security(&mut self, doc: &PdfADocument) {
        if doc.is_encrypted {
            self.errors.push(PdfAError {
                code: PdfAErrorCode::EncryptionNotAllowed,
                message: "Encryption not allowed in PDF/A".to_string(),
                location: None,
            });
        }

        if doc.has_javascript {
            self.errors.push(PdfAError {
                code: PdfAErrorCode::JavaScriptNotAllowed,
                message: "JavaScript not allowed in PDF/A".to_string(),
                location: None,
            });
        }
    }

    fn check_embedded_files(&mut self, doc: &PdfADocument) {
        if !self.level.allows_embedded_files() && !doc.embedded_files.is_empty() {
            self.errors.push(PdfAError {
                code: PdfAErrorCode::EmbeddedFilesNotAllowed,
                message: format!("Embedded files not allowed in PDF/A-{}", self.level.part()),
                location: None,
            });
        }

        // PDF/A-3 requires file relationship
        if self.level.allows_embedded_files() {
            for file in &doc.embedded_files {
                if file.relationship.is_none() {
                    self.errors.push(PdfAError {
                        code: PdfAErrorCode::MissingFileRelationship,
                        message: format!("Embedded file '{}' missing AFRelationship", file.name),
                        location: Some(format!("File: {}", file.name)),
                    });
                }
            }
        }
    }
}

// =============================================================================
// PDF/A Document (validation model)
// =============================================================================

/// PDF/A font info for validation.
#[derive(Debug, Clone, Default)]
pub struct PdfAFontInfo {
    /// Font is embedded.
    pub is_embedded: bool,
    /// Has ToUnicode CMap.
    pub has_cmap: bool,
    /// Has glyph widths.
    pub has_widths: bool,
}

/// Embedded file info.
#[derive(Debug, Clone)]
pub struct EmbeddedFileInfo {
    /// File name.
    pub name: String,
    /// MIME type.
    pub mime_type: Option<String>,
    /// AFRelationship (Source, Data, Alternative, etc.)
    pub relationship: Option<String>,
}

/// PDF/A document model for validation.
#[derive(Debug, Clone, Default)]
pub struct PdfADocument {
    /// XMP metadata.
    pub xmp_metadata: Option<XmpMetadata>,
    /// Document ID.
    pub document_id: Option<String>,
    /// Output intent.
    pub output_intent: Option<OutputIntent>,
    /// Fonts used in document.
    pub fonts: std::collections::HashMap<String, PdfAFontInfo>,
    /// Uses device-dependent colors (DeviceRGB, DeviceCMYK, DeviceGray).
    pub uses_device_colors: bool,
    /// Uncalibrated color spaces used.
    pub uncalibrated_colors: HashSet<String>,
    /// Uses JPEG2000 compression.
    pub uses_jpeg2000: bool,
    /// Uses LZW compression.
    pub uses_lzw: bool,
    /// Uses transparency.
    pub uses_transparency: bool,
    /// Has tagged structure.
    pub has_structure: bool,
    /// Is encrypted.
    pub is_encrypted: bool,
    /// Has JavaScript.
    pub has_javascript: bool,
    /// Embedded files.
    pub embedded_files: Vec<EmbeddedFileInfo>,
}

impl PdfADocument {
    /// Create a new PDF/A document model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create XMP metadata with PDF/A identification.
    pub fn create_xmp_metadata(&mut self, level: PdfALevel) -> &mut XmpMetadata {
        let doc_id = uuid_v4();
        let inst_id = uuid_v4();

        self.document_id = Some(doc_id.clone());

        self.xmp_metadata = Some(XmpMetadata {
            title: None,
            author: None,
            subject: None,
            keywords: Vec::new(),
            creator: Some("skia-rs".to_string()),
            create_date: Some(iso8601_now()),
            modify_date: Some(iso8601_now()),
            pdfa_level: Some(level),
            document_id: Some(doc_id),
            instance_id: Some(inst_id),
        });

        self.xmp_metadata.as_mut().unwrap()
    }

    /// Set sRGB output intent.
    pub fn set_srgb_output_intent(&mut self) {
        self.output_intent = Some(OutputIntent::srgb());
    }

    /// Register a font.
    pub fn register_font(&mut self, name: &str, info: PdfAFontInfo) {
        self.fonts.insert(name.to_string(), info);
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Generate a UUID v4.
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    // Simple pseudo-random UUID (not cryptographically secure)
    let seed = now.as_nanos() as u64;
    let a = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    let b = a.wrapping_mul(6364136223846793005).wrapping_add(1);

    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (a >> 32) as u32,
        (a >> 16) as u16 & 0xFFFF,
        (a & 0x0FFF),
        0x8000 | ((b >> 48) as u16 & 0x3FFF),
        b & 0xFFFFFFFFFFFF
    )
}

/// Get current time in ISO 8601 format.
fn iso8601_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = now.as_secs();

    // Simple date calculation (not accounting for leap seconds, etc.)
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Approximate year/month/day (simplified)
    let mut year = 1970;
    let mut remaining_days = days as i64;

    while remaining_days >= 365 {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days >= days_in_year {
            remaining_days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }

    let month_days = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &days_in_month in &month_days {
        if remaining_days >= days_in_month as i64 {
            remaining_days -= days_in_month as i64;
            month += 1;
        } else {
            break;
        }
    }

    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdfa_level() {
        assert_eq!(PdfALevel::A1b.part(), 1);
        assert_eq!(PdfALevel::A2b.part(), 2);
        assert_eq!(PdfALevel::A3b.part(), 3);

        assert!(!PdfALevel::A1b.allows_transparency());
        assert!(PdfALevel::A2b.allows_transparency());

        assert!(!PdfALevel::A2b.allows_embedded_files());
        assert!(PdfALevel::A3b.allows_embedded_files());
    }

    #[test]
    fn test_xmp_generation() {
        let xmp = XmpMetadata::new()
            .with_title("Test Document")
            .with_author("Test Author")
            .with_pdfa_level(PdfALevel::A1b);

        let xml = xmp.to_xmp();
        assert!(xml.contains("Test Document"));
        assert!(xml.contains("Test Author"));
        assert!(xml.contains("pdfaid:part>1"));
        assert!(xml.contains("pdfaid:conformance>B"));
    }

    #[test]
    fn test_validator_missing_metadata() {
        let doc = PdfADocument::new();
        let mut validator = PdfAValidator::new(PdfALevel::A1b);

        let result = validator.validate(&doc);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.code == PdfAErrorCode::MissingXmpMetadata)
        );
    }

    #[test]
    fn test_validator_font_not_embedded() {
        let mut doc = PdfADocument::new();
        doc.create_xmp_metadata(PdfALevel::A1b);
        doc.document_id = Some("test".to_string());

        doc.register_font(
            "Helvetica",
            PdfAFontInfo {
                is_embedded: false,
                has_cmap: true,
                has_widths: true,
            },
        );

        let mut validator = PdfAValidator::new(PdfALevel::A1b);
        let result = validator.validate(&doc);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.code == PdfAErrorCode::FontNotEmbedded)
        );
    }

    #[test]
    fn test_validator_transparency_a1() {
        let mut doc = PdfADocument::new();
        doc.create_xmp_metadata(PdfALevel::A1b);
        doc.document_id = Some("test".to_string());
        doc.uses_transparency = true;

        let mut validator = PdfAValidator::new(PdfALevel::A1b);
        let result = validator.validate(&doc);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.code == PdfAErrorCode::TransparencyNotAllowed)
        );
    }

    #[test]
    fn test_validator_transparency_a2() {
        let mut doc = PdfADocument::new();
        doc.create_xmp_metadata(PdfALevel::A2b);
        doc.document_id = Some("test".to_string());
        doc.uses_transparency = true;

        let mut validator = PdfAValidator::new(PdfALevel::A2b);
        let result = validator.validate(&doc);

        // Should pass - transparency allowed in PDF/A-2
        assert!(
            result.is_ok()
                || !result
                    .unwrap_err()
                    .iter()
                    .any(|e| e.code == PdfAErrorCode::TransparencyNotAllowed)
        );
    }
}
