//! Paint, shaders, and effects for skia-rs.
//!
//! This crate provides drawing style configuration:
//! - Paint (stroke/fill settings, anti-aliasing)
//! - Shaders (gradients, images, blend modes)
//! - Color filters
//! - Mask filters (blur)
//! - Image filters
//! - Runtime effects (SkSL custom shaders)

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod blend;
pub mod filter;
pub mod paint;
pub mod runtime_effect;
pub mod shader;
pub mod sksl;

pub use blend::*;
pub use filter::*;
pub use paint::*;
pub use runtime_effect::*;
pub use shader::*;
pub use sksl::{SkslProgram, SkslType};
