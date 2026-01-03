//! # skia-rs
//!
//! A pure Rust implementation of Google's Skia 2D graphics library.
//!
//! This crate provides a comprehensive 2D graphics API for rendering shapes,
//! images, text, and more. It is designed to be a drop-in replacement for
//! the original Skia library while providing a safe, idiomatic Rust API.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use skia_rs::prelude::*;
//!
//! // Create a surface to draw on
//! let mut surface = Surface::new_raster_n32_premul(800, 600).unwrap();
//! let mut canvas = surface.canvas();
//!
//! // Create a paint for styling
//! let mut paint = Paint::default();
//! paint.set_color(Color::RED.into());
//! paint.set_anti_alias(true);
//!
//! // Draw a circle
//! canvas.draw_circle(Point::new(400.0, 300.0), 100.0, &paint);
//! ```
//!
//! ## Feature Flags
//!
//! This crate uses feature flags to control which components are included:
//!
//! - **default** = `["std", "codec", "svg"]` - Standard features for most use cases
//! - **std** - Standard library support (disable for `no_std` environments)
//! - **serde** - Serialization support via serde
//!
//! ### Image Codecs
//! - **codec** - Base image codec support
//! - **codec-png** - PNG encoding/decoding
//! - **codec-jpeg** - JPEG encoding/decoding
//! - **codec-webp** - WebP encoding/decoding
//! - **codec-gif** - GIF decoding
//! - **codec-avif** - AVIF encoding/decoding
//! - **codec-raw** - RAW image processing
//! - **codec-all** - All image codecs
//!
//! ### Specialized Modules
//! - **svg** - SVG parsing and rendering
//! - **pdf** - PDF document generation
//! - **text** - Advanced text rendering and shaping
//! - **skottie** - Lottie animation support
//!
//! ### GPU Backends
//! - **gpu** - Base GPU support
//! - **vulkan** - Vulkan backend
//! - **opengl** - OpenGL backend
//! - **metal** - Metal backend (macOS/iOS only)
//! - **wgpu-backend** / **webgpu** - wgpu backend (cross-platform)
//!
//! ### FFI
//! - **ffi** - C FFI bindings for interoperability
//!
//! ### Full Features
//! - **full** - Enables all features except platform-specific GPU backends
//!
//! ## Module Organization
//!
//! - [`core`] - Fundamental types: `Scalar`, `Point`, `Rect`, `Color`, `Matrix`
//! - [`path`] - Path geometry: `Path`, `PathBuilder`, `PathOps`, `PathEffect`
//! - [`paint`] - Styling: `Paint`, `Shader`, `BlendMode`, `Filter`
//! - [`canvas`] - Drawing: `Canvas`, `Surface`, `Picture`
//! - [`safe`] - High-level ergonomic API (recommended for most users)
//!
//! Optional modules (enabled via features):
//! - [`text`] - Typography: font loading, text shaping, layout
//! - [`codec`] - Image I/O: PNG, JPEG, GIF, WebP, AVIF
//! - [`svg`] - SVG parsing and rendering
//! - [`pdf`] - PDF document generation
//! - [`gpu`] - GPU rendering backends
//! - [`skottie`] - Lottie/Skottie animation
//! - [`ffi`] - C FFI bindings

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::module_inception)]

// Re-export core crates
pub use skia_rs_core as core;
pub use skia_rs_path as path;
pub use skia_rs_paint as paint;
pub use skia_rs_canvas as canvas;
pub use skia_rs_safe as safe;

// Optional crate re-exports
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

#[cfg(feature = "gpu")]
#[cfg_attr(docsrs, doc(cfg(feature = "gpu")))]
pub use skia_rs_gpu as gpu;

#[cfg(feature = "skottie")]
#[cfg_attr(docsrs, doc(cfg(feature = "skottie")))]
pub use skia_rs_skottie as skottie;

#[cfg(feature = "ffi")]
#[cfg_attr(docsrs, doc(cfg(feature = "ffi")))]
pub use skia_rs_ffi as ffi;

/// Prelude module for convenient imports.
///
/// Import all commonly used types with:
/// ```rust
/// use skia_rs::prelude::*;
/// ```
pub mod prelude {
    // Core types
    pub use skia_rs_core::{
        AlphaType, Color, Color4f, ColorSpace, ColorType, IPoint, IRect, ISize, ImageInfo, Matrix,
        Point, Rect, Scalar, Size,
    };

    // Path types
    pub use skia_rs_path::{FillType, Path, PathBuilder, PathDirection};

    // Paint types
    pub use skia_rs_paint::{BlendMode, Paint, Style};

    // Canvas types
    pub use skia_rs_canvas::{Canvas, ClipOp, SaveLayerRec, Surface};

    // Safe wrapper types (high-level API)
    pub use skia_rs_safe::prelude::*;

    // Optional module prelude re-exports
    #[cfg(feature = "text")]
    #[cfg_attr(docsrs, doc(cfg(feature = "text")))]
    pub use skia_rs_text::{Font, FontStyle, TextBlob, Typeface};

    #[cfg(feature = "codec")]
    #[cfg_attr(docsrs, doc(cfg(feature = "codec")))]
    pub use skia_rs_codec::{ImageDecoder, ImageEncoder, ImageFormat};

    #[cfg(feature = "svg")]
    #[cfg_attr(docsrs, doc(cfg(feature = "svg")))]
    pub use skia_rs_svg::SvgDom;

    #[cfg(feature = "pdf")]
    #[cfg_attr(docsrs, doc(cfg(feature = "pdf")))]
    pub use skia_rs_pdf::PdfDocument;

    #[cfg(feature = "gpu")]
    #[cfg_attr(docsrs, doc(cfg(feature = "gpu")))]
    pub use skia_rs_gpu::GpuContext;

    #[cfg(feature = "skottie")]
    #[cfg_attr(docsrs, doc(cfg(feature = "skottie")))]
    pub use skia_rs_skottie::Animation;
}

/// Version information for the skia-rs library.
pub mod version {
    /// The major version number.
    pub const MAJOR: u32 = 0;
    /// The minor version number.
    pub const MINOR: u32 = 2;
    /// The patch version number.
    pub const PATCH: u32 = 0;

    /// The full version string.
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

    /// Returns the version as a tuple.
    #[inline]
    pub const fn as_tuple() -> (u32, u32, u32) {
        (MAJOR, MINOR, PATCH)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version::VERSION, "0.2.0");
        assert_eq!(version::as_tuple(), (0, 2, 0));
    }

    #[test]
    fn test_core_re_export() {
        let point = core::Point::new(10.0, 20.0);
        assert_eq!(point.x, 10.0);
        assert_eq!(point.y, 20.0);
    }

    #[test]
    fn test_prelude_imports() {
        use crate::prelude::*;

        let color = Color::RED;
        assert_eq!(color.red(), 255);

        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
        assert_eq!(rect.width(), 100.0);
    }
}
