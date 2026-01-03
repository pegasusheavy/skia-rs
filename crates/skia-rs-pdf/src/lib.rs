//! PDF generation for skia-rs.
//!
//! This crate provides PDF output:
//! - PDF document creation
//! - Drawing to PDF canvas
//! - Font embedding (Type 1, TrueType)
//! - Image embedding (JPEG, PNG)
//! - Transparency (ExtGState, soft masks, transparency groups)

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod canvas;
pub mod document;
pub mod font;
pub mod image;
pub mod stream;
pub mod transparency;

pub use canvas::*;
pub use document::*;
pub use font::{PdfFont, PdfFontManager, PdfFontType, StandardFont};
pub use image::{PdfColorSpace, PdfImage, PdfImageFilter, PdfImageManager};
pub use stream::*;
pub use transparency::{
    ExtGStateKey, ExtGraphicsState, PdfBlendMode, SoftMask, SoftMaskSubtype,
    TransparencyGroup, TransparencyManager,
};
