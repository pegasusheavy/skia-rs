//! Image encoding and decoding for skia-rs.
//!
//! This crate provides image I/O:
//! - Image type for immutable pixel data
//! - GPU-backed images for efficient GPU rendering
//! - Lazy/deferred images for memory efficiency
//! - ImageGenerator trait for custom image generation
//! - Codec trait for format-specific encoders/decoders
//! - PNG encode/decode
//! - JPEG encode/decode
//! - GIF decode
//! - WebP encode/decode
//! - BMP encode/decode
//! - ICO decode
//! - WBMP encode/decode (Wireless Bitmap)

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod codec;
pub mod generator;
pub mod gpu_image;
pub mod image;
pub mod lazy_image;

pub use codec::*;
pub use generator::*;
pub use gpu_image::*;
pub use image::*;
pub use lazy_image::*;
