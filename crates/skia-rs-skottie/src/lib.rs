//! Lottie animation support for skia-rs (Skottie).
//!
//! This crate provides Lottie/Bodymovin animation playback capabilities,
//! compatible with Skia's Skottie library.
//!
//! ## Features
//!
//! - **JSON Parsing**: Full Lottie JSON format support
//! - **Animation Playback**: Timeline-based animation with frame interpolation
//! - **Shape Layers**: Paths, fills, strokes, gradients
//! - **Transform Animations**: Position, scale, rotation, opacity
//! - **Masks & Mattes**: Alpha masks, track mattes
//!
//! ## Example
//!
//! ```ignore
//! use skia_rs_skottie::Animation;
//!
//! let animation = Animation::from_json(json_string)?;
//! animation.seek_frame(30.0);
//! animation.render(&mut canvas);
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod animation;
pub mod expression;
pub mod keyframe;
pub mod layers;
pub mod mask;
pub mod model;
pub mod render;
pub mod shapes;
pub mod transform;

pub use animation::{Animation, AnimationBuilder, AnimationStats};
pub use keyframe::{Easing, Keyframe, KeyframeValue};
pub use layers::{Layer, LayerType};
pub use mask::{Mask, MaskMode, MatteMode};
pub use model::LottieModel;
pub use render::RenderContext;
pub use shapes::{Shape, ShapeGroup};
pub use transform::Transform;

use thiserror::Error;

/// Errors that can occur when working with Lottie animations.
#[derive(Debug, Error)]
pub enum SkottieError {
    /// JSON parsing error.
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    /// Invalid animation data.
    #[error("Invalid animation data: {0}")]
    InvalidData(String),
    /// Unsupported feature.
    #[error("Unsupported Lottie feature: {0}")]
    UnsupportedFeature(String),
    /// IO error.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for Skottie operations.
pub type Result<T> = std::result::Result<T, SkottieError>;
