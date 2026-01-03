//! GPU backends for skia-rs.
//!
//! This crate provides hardware-accelerated rendering:
//! - Vulkan backend (via ash)
//! - OpenGL backend (via glow)
//! - Metal backend (via metal-rs)
//! - WebGPU/cross-platform backend (via wgpu)
//!
//! ## Features
//!
//! - **Pipeline State Management**: Render and compute pipeline configuration
//! - **Shader Compilation**: WGSL shader compilation and caching
//! - **Command Buffer Recording**: Efficient command batching and submission
//! - **Path Tessellation**: Convert paths to GPU-friendly triangle meshes
//! - **Stencil-Then-Cover**: Complex path rendering with correct winding rules
//! - **Atlas Management**: Efficient batching of small elements
//! - **Glyph Cache**: Fast text rendering with cached glyphs
//! - **Gradient Textures**: Generate gradient lookup textures
//! - **Image Tiling**: Tile modes for image rendering
//! - **MSAA Support**: Multi-sample anti-aliasing
//! - **SDF Rendering**: Signed distance field for resolution-independent shapes

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod atlas;
pub mod command;
pub mod context;
pub mod debug;
pub mod glyph_cache;
pub mod gradient;
pub mod msaa;
pub mod pipeline;
pub mod sdf;
pub mod shader;
pub mod stencil_cover;
pub mod surface;
pub mod tessellation;
pub mod texture;
pub mod tiling;

#[cfg(feature = "wgpu-backend")]
pub mod wgpu_backend;

#[cfg(feature = "vulkan")]
pub mod vulkan_backend;

#[cfg(feature = "opengl")]
pub mod opengl_backend;

#[cfg(feature = "metal")]
pub mod metal_backend;

pub use atlas::*;
pub use command::*;
pub use context::*;
pub use glyph_cache::*;
pub use gradient::*;
pub use msaa::*;
pub use pipeline::*;
pub use sdf::*;
pub use shader::*;
pub use stencil_cover::*;
pub use surface::*;
pub use tessellation::*;
pub use texture::*;
pub use tiling::*;

#[cfg(feature = "wgpu-backend")]
pub use wgpu_backend::*;

#[cfg(feature = "vulkan")]
pub use vulkan_backend::*;

#[cfg(feature = "opengl")]
pub use opengl_backend::*;

#[cfg(feature = "metal")]
pub use metal_backend::*;
