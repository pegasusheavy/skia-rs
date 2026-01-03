//! High-level safe Rust API for skia-rs.
//!
//! This crate provides a convenient, idiomatic Rust API that wraps
//! the lower-level crates with ergonomic types and methods.
//!
//! # Features
//!
//! - `std` (default) - Enable standard library support
//! - `serde` - Enable serialization support
//! - `codec` - Enable image codec support (PNG, JPEG, etc.)
//! - `codec-all` - Enable all image codecs
//! - `svg` - Enable SVG support
//! - `pdf` - Enable PDF generation
//! - `text` - Enable text rendering
//! - `skottie` - Enable Lottie animation support
//! - `gpu` - Enable GPU rendering
//! - `wgpu-backend` - Enable WGPU backend
//! - `vulkan` - Enable Vulkan backend
//! - `opengl` - Enable OpenGL backend
//! - `metal` - Enable Metal backend (macOS/iOS only)
//! - `full` - Enable all features

#![warn(missing_docs)]
#![warn(clippy::all)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Core crates (always included)
pub use skia_rs_canvas as canvas;
pub use skia_rs_core as core;
pub use skia_rs_paint as paint;
pub use skia_rs_path as path;

// Optional features
#[cfg(feature = "text")]
#[cfg_attr(docsrs, doc(cfg(feature = "text")))]
pub use skia_rs_text as text;

#[cfg(feature = "codec")]
#[cfg_attr(docsrs, doc(cfg(feature = "codec")))]
pub use skia_rs_codec as codec;

#[cfg(feature = "svg")]
#[cfg_attr(docsrs, doc(cfg(feature = "svg")))]
pub use skia_rs_svg as svg;

#[cfg(feature = "pdf")]
#[cfg_attr(docsrs, doc(cfg(feature = "pdf")))]
pub use skia_rs_pdf as pdf;

#[cfg(feature = "skottie")]
#[cfg_attr(docsrs, doc(cfg(feature = "skottie")))]
pub use skia_rs_skottie as skottie;

#[cfg(feature = "gpu")]
#[cfg_attr(docsrs, doc(cfg(feature = "gpu")))]
pub use skia_rs_gpu as gpu;

// WASM support
#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

/// Convenience prelude for common types.
pub mod prelude {
    pub use skia_rs_canvas::{RasterCanvas, Surface};
    pub use skia_rs_core::{Color, Color4f, Matrix, Point, Rect, Scalar};
    pub use skia_rs_paint::{Paint, Style};
    pub use skia_rs_path::{Path, PathBuilder};
}
