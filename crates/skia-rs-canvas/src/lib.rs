//! Canvas, surface, and recording for skia-rs.
//!
//! This crate provides the drawing surface abstraction:
//! - Canvas (the main drawing interface)
//! - Surface (backing store for canvas)
//! - Picture (recorded drawing commands)
//! - Rasterizer (software rendering)
//! - SIMD-optimized blitting (SSE4.2, AVX2, NEON)
//! - Advanced clipping (anti-aliased, region-based)
//! - Save/restore layer stack

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod canvas;
pub mod clip;
pub mod picture;
pub mod raster;
pub mod simd;
pub mod surface;

pub use canvas::*;
pub use clip::{ClipMask, ClipStack, ClipState};
pub use picture::*;
pub use raster::*;
pub use simd::{simd_capabilities, SimdCapabilities};
pub use surface::{RasterCanvas, Surface, VertexMode};

// Re-export Image for drawing
#[cfg(feature = "codec")]
pub use skia_rs_codec::Image;
