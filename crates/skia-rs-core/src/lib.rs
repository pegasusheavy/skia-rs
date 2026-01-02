//! # skia-rs-core
//!
//! Core types for the skia-rs graphics library.
//!
//! This crate provides fundamental types used throughout skia-rs:
//! - **Geometry**: Points, sizes, rectangles, matrices
//! - **Color**: Color types, color spaces, alpha handling
//! - **Pixels**: Image info, pixel storage, format conversion
//! - **Region**: Complex clip regions composed of rectangles
//!
//! ## Skia API Compatibility
//!
//! Types in this crate mirror Skia's core types:
//! - [`Scalar`] ↔ `SkScalar`
//! - [`Point`] ↔ `SkPoint`
//! - [`Rect`] ↔ `SkRect`
//! - [`Matrix`] ↔ `SkMatrix`
//! - [`Color`] ↔ `SkColor`
//! - [`ColorSpace`] ↔ `SkColorSpace`
//! - [`Region`] ↔ `SkRegion`

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod color;
pub mod geometry;
pub mod matrix44;
pub mod pixel;
pub mod region;

// Re-exports for convenience
pub use color::{
    color4f_linear_to_srgb, color4f_srgb_to_linear, color_to_linear, contrast_ratio,
    hsl_to_rgb, hsv_to_rgb, lab_to_rgb, linear_to_color, linear_to_srgb, luminance, mix_colors,
    premultiply_color, rgb_to_hsl, rgb_to_hsv, rgb_to_lab, rgb_to_xyz, srgb_to_linear,
    unpremultiply_color, xyz_to_rgb, AlphaType, Color, Color4f, ColorFilterFlags, ColorGamut,
    ColorSpace, ColorType, IccColorSpace, IccPcs, IccProfile, IccProfileClass, TransferFunction,
};
pub use geometry::{Corner, IPoint, IRect, ISize, Matrix, Point, Point3, Rect, RRect, Size};
pub use matrix44::Matrix44;
pub use pixel::{
    convert_pixels, premultiply_in_place, swizzle_rb_in_place, unpremultiply_in_place, Bitmap,
    ImageInfo, PixelError, PixelGeometry, Pixmap, SurfaceProps, SurfacePropsFlags,
};
pub use region::{Region, RegionOp};

/// Scalar type used for all floating-point geometry.
///
/// This is `f32` by default, matching Skia's standard configuration.
/// Skia can be built with `f64` scalars, but this is rare.
pub type Scalar = f32;

/// A trait for types that can be converted to/from Skia scalar values.
pub trait AsScalar {
    /// Convert to scalar.
    fn as_scalar(self) -> Scalar;
}

impl AsScalar for f32 {
    #[inline]
    fn as_scalar(self) -> Scalar {
        self
    }
}

impl AsScalar for f64 {
    #[inline]
    fn as_scalar(self) -> Scalar {
        self as Scalar
    }
}

impl AsScalar for i32 {
    #[inline]
    fn as_scalar(self) -> Scalar {
        self as Scalar
    }
}

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::color::{
        hsl_to_rgb, hsv_to_rgb, linear_to_srgb, luminance, mix_colors, premultiply_color,
        rgb_to_hsl, rgb_to_hsv, srgb_to_linear, unpremultiply_color, AlphaType, Color, Color4f,
        ColorSpace, ColorType,
    };
    pub use crate::geometry::{
        Corner, IPoint, IRect, ISize, Matrix, Point, Point3, Rect, RRect, Size,
    };
    pub use crate::matrix44::Matrix44;
    pub use crate::pixel::{Bitmap, ImageInfo, PixelGeometry, Pixmap, SurfaceProps};
    pub use crate::region::{Region, RegionOp};
    pub use crate::{AsScalar, Scalar};
}
