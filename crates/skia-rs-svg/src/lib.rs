//! SVG parsing and rendering for skia-rs.
//!
//! This crate provides SVG support:
//! - SVG parsing (via roxmltree/usvg)
//! - SVG rendering to canvas
//! - SVG DOM manipulation

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod dom;
pub mod parser;
pub mod render;

pub use dom::*;
pub use parser::*;
pub use render::*;
