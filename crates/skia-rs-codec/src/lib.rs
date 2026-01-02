//! Image encoding and decoding for skia-rs.
//!
//! This crate provides image I/O:
//! - Image type for immutable pixel data
//! - Codec trait for format-specific encoders/decoders
//! - PNG encode/decode
//! - JPEG encode/decode
//! - GIF decode
//! - WebP encode/decode

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod codec;
pub mod image;

pub use codec::*;
pub use image::*;
