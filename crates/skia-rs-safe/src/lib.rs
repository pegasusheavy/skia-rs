//! High-level safe Rust API for skia-rs.
//!
//! This crate provides a convenient, idiomatic Rust API that wraps
//! the lower-level crates with ergonomic types and methods.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub use skia_rs_canvas as canvas;
pub use skia_rs_core as core;
pub use skia_rs_paint as paint;
pub use skia_rs_path as path;
pub use skia_rs_text as text;

#[cfg(feature = "codec")]
pub use skia_rs_codec as codec;

#[cfg(feature = "svg")]
pub use skia_rs_svg as svg;

#[cfg(feature = "pdf")]
pub use skia_rs_pdf as pdf;

#[cfg(feature = "gpu")]
pub use skia_rs_gpu as gpu;
