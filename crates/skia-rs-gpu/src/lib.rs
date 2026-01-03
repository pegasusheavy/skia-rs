//! GPU backends for skia-rs.
//!
//! This crate provides hardware-accelerated rendering:
//! - Vulkan backend (via ash)
//! - OpenGL backend (via glow)
//! - WebGPU/cross-platform backend (via wgpu)
//!
//! ## Features
//!
//! - **Pipeline State Management**: Render and compute pipeline configuration
//! - **Shader Compilation**: WGSL shader compilation and caching
//! - **Command Buffer Recording**: Efficient command batching and submission

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod command;
pub mod context;
pub mod pipeline;
pub mod shader;
pub mod surface;
pub mod texture;

#[cfg(feature = "wgpu-backend")]
pub mod wgpu_backend;

#[cfg(feature = "vulkan")]
pub mod vulkan_backend;

#[cfg(feature = "opengl")]
pub mod opengl_backend;

#[cfg(feature = "metal")]
pub mod metal_backend;

pub use command::*;
pub use context::*;
pub use pipeline::*;
pub use shader::*;
pub use surface::*;
pub use texture::*;

#[cfg(feature = "wgpu-backend")]
pub use wgpu_backend::*;

#[cfg(feature = "vulkan")]
pub use vulkan_backend::*;

#[cfg(feature = "opengl")]
pub use opengl_backend::*;

#[cfg(feature = "metal")]
pub use metal_backend::*;
