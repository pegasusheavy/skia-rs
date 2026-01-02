//! Paint, shaders, and effects for skia-rs.
//!
//! This crate provides drawing style configuration:
//! - Paint (stroke/fill settings, anti-aliasing)
//! - Shaders (gradients, images, blend modes)
//! - Color filters
//! - Mask filters (blur)
//! - Image filters

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod blend;
pub mod filter;
pub mod paint;
pub mod shader;

pub use blend::*;
pub use filter::*;
pub use paint::*;
pub use shader::*;
