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

/// Numeric cast helpers local to `skia-rs-gpu`.
///
/// Mirrors `skia_rs_core::cast`: narrowing/widening-but-lossy casts that have
/// no safe standard-library equivalent get a single, documented home here
/// instead of scattered bare `as` casts. Every value passed through these
/// helpers in this crate is a loop counter, vertex/index count, or pixel
/// dimension — always far below the precision or range limits these guard
/// against, so the theoretical loss they flag never occurs in practice.
pub(crate) mod cast_util {
    /// Convert a `u32` count/index to `f32` (e.g. loop-ratio interpolation).
    #[inline]
    #[must_use]
    pub const fn scalar_from_u32(x: u32) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        {
            x as f32
        }
    }

    /// Convert a `usize` count/index to `f32` (e.g. loop-ratio interpolation).
    #[inline]
    #[must_use]
    pub const fn scalar_from_usize(x: usize) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        {
            x as f32
        }
    }

    /// Convert a `u64` count/hash to `f64`.
    #[inline]
    #[must_use]
    pub const fn f64_from_u64(x: u64) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        {
            x as f64
        }
    }

    /// Narrow a `usize` length/index to `u32`.
    ///
    /// # Panics
    /// Panics if `x` exceeds `u32::MAX`, which never happens for the
    /// vertex/index counts and dimensions this crate deals with.
    #[inline]
    #[must_use]
    pub fn u32_from_usize(x: usize) -> u32 {
        u32::try_from(x).expect("value exceeds u32::MAX")
    }

    /// Saturating `f32` -> `u32` conversion (truncates toward zero, then
    /// saturates via [`skia_rs_core::cast::saturate_to_i32`]; negative inputs
    /// map to 0).
    #[inline]
    #[must_use]
    pub fn u32_from_scalar_sat(x: f32) -> u32 {
        u32::try_from(skia_rs_core::cast::saturate_to_i32(x)).unwrap_or(0)
    }

    /// Saturating `f32` -> `usize` conversion (see [`u32_from_scalar_sat`]).
    #[inline]
    #[must_use]
    pub fn usize_from_scalar_sat(x: f32) -> usize {
        usize::try_from(skia_rs_core::cast::saturate_to_i32(x)).unwrap_or(0)
    }

    /// Saturating `f32` -> `u8` conversion (see [`u32_from_scalar_sat`]).
    #[inline]
    #[must_use]
    pub fn u8_from_scalar_sat(x: f32) -> u8 {
        u8::try_from(skia_rs_core::cast::saturate_to_i32(x)).unwrap_or(if x < 0.0 {
            0
        } else {
            u8::MAX
        })
    }
}

pub mod atlas;
pub mod command;
pub mod context;
pub mod debug;
pub mod glyph_cache;
pub mod gradient;
pub mod msaa;
pub mod paint_bridge;
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

// The metal crate only compiles on Apple targets, so gate the backend
// module on both the feature and the target family.
#[cfg(all(feature = "metal", any(target_os = "macos", target_os = "ios")))]
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

#[cfg(all(feature = "metal", any(target_os = "macos", target_os = "ios")))]
pub use metal_backend::*;
