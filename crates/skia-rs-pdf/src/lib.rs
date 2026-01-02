//! PDF generation for skia-rs.
//!
//! This crate provides PDF output:
//! - PDF document creation
//! - Drawing to PDF canvas
//! - Font embedding

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod canvas;
pub mod document;
pub mod stream;

pub use canvas::*;
pub use document::*;
pub use stream::*;
