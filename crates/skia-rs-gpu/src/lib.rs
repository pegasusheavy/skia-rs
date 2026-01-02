//! GPU backends for skia-rs.
//!
//! This crate provides hardware-accelerated rendering:
//! - Vulkan backend (via ash)
//! - OpenGL backend (via glow)
//! - WebGPU/cross-platform backend (via wgpu)

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod context;
pub mod surface;
pub mod texture;

#[cfg(feature = "wgpu-backend")]
pub mod wgpu_backend;

pub use context::*;
pub use surface::*;
pub use texture::*;

#[cfg(feature = "wgpu-backend")]
pub use wgpu_backend::*;
