//! SVG parsing and rendering for skia-rs.
//!
//! This crate provides SVG support:
//! - SVG parsing (via custom parser)
//! - SVG rendering to canvas
//! - SVG DOM manipulation
//! - CSS styling support
//! - SVG export

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod css;
pub mod dom;
pub mod export;
pub mod parser;
pub mod render;

pub use css::{apply_stylesheet, parse_inline_style, CssRule, CssSelector, Stylesheet};
pub use dom::*;
pub use export::{export_svg, export_svg_with_options, SvgExportOptions};
pub use parser::*;
pub use render::*;
