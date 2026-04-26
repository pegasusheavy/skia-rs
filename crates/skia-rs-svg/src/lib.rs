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
pub mod glyph_svg;
pub mod parser;
pub mod render;

pub use css::{CssRule, CssSelector, Stylesheet, apply_stylesheet, parse_inline_style};
pub use dom::*;
pub use export::{SvgExportOptions, export_svg, export_svg_with_options};
pub use glyph_svg::{decode_glyph_svg_bytes, draw_glyph_svg, glyph_svg_to_dom};
pub use parser::*;
pub use render::*;
