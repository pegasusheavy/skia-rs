//! Path geometry and operations for skia-rs.
//!
//! This crate provides path-related functionality:
//! - Path construction and manipulation
//! - Path effects (dash, corner, etc.)
//! - Path operations (union, intersect, difference)
//! - Path measurement and traversal
//! - SVG path parsing
//! - Stroke-to-fill conversion

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod builder;
pub mod effects;
pub mod measure;
pub mod ops;
pub mod path;
pub mod path_utils;
pub mod svg;

pub use builder::*;
pub use effects::*;
pub use measure::*;
pub use ops::*;
pub use path::{FillType, Path, PathConvexity, PathDirection, PathElement, PathIter, Verb};
pub use path_utils::{StrokeCap, StrokeJoin, StrokeParams, stroke_to_fill};
pub use svg::{SvgPathError, parse_svg_path};
