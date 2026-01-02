//! Text layout and rendering for skia-rs.
//!
//! This crate provides text functionality:
//! - Font loading and management
//! - Font manager and style sets
//! - Text shaping (via rustybuzz/HarfBuzz)
//! - Text layout and measurement
//! - Rich text paragraph layout
//! - Glyph rendering and paths
//! - Color glyph (emoji) support

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod font;
pub mod font_mgr;
pub mod paragraph;
pub mod shaper;
pub mod text_blob;
pub mod typeface;

pub use font::*;
pub use font_mgr::*;
pub use paragraph::*;
pub use shaper::*;
pub use text_blob::*;
pub use typeface::*;
