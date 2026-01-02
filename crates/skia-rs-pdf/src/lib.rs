//! PDF generation for skia-rs.
//!
//! This crate provides PDF output:
//! - PDF document creation
//! - Drawing to PDF canvas
//! - Font embedding

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod document;
pub mod canvas;
pub mod stream;

pub use document::*;
pub use canvas::*;
pub use stream::*;
