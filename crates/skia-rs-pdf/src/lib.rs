//! PDF generation for skia-rs.
//!
//! This crate provides PDF output:
//! - PDF document creation
//! - Drawing to PDF canvas
//! - Font embedding (Type 1, TrueType)
//! - Image embedding (JPEG, PNG)
//! - Transparency (ExtGState, soft masks, transparency groups)
//! - PDF/A compliance (ISO 19005)

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod canvas;
pub mod document;
pub mod font;
pub mod image;
pub mod pdfa;
pub mod stream;
pub mod transparency;

pub use canvas::*;
pub use document::*;
pub use font::{PdfFont, PdfFontManager, PdfFontType, StandardFont};
pub use image::{PdfColorSpace, PdfImage, PdfImageFilter, PdfImageManager};
pub use pdfa::{
    EmbeddedFileInfo, OutputIntent, PdfADocument, PdfAError, PdfAErrorCode, PdfAFontInfo,
    PdfALevel, PdfAValidator, XmpMetadata,
};
pub use stream::*;
pub use transparency::{
    ExtGStateKey, ExtGraphicsState, PdfBlendMode, SoftMask, SoftMaskSubtype, TransparencyGroup,
    TransparencyManager,
};

/// Test-only re-export of the TrueType subsetter. Stable under `#[cfg(test)]`
/// and integration tests; not part of the public SemVer surface. Exposes
/// the byte-level glyf/loca pruning so tests can drive it against custom
/// fixtures without going through the PDF emission pipeline.
#[doc(hidden)]
pub fn subset_truetype_for_tests(
    data: &[u8],
    glyphs: &std::collections::BTreeSet<u16>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    font::subset::subset_truetype(data, glyphs).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
