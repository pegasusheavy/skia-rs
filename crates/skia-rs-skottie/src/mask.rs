//! Mask and matte support for Lottie animations.
//!
//! This module handles:
//! - Mask shapes (Add, Subtract, Intersect, Difference)
//! - Track mattes (Alpha, Luma)
//! - Mask expansion and feathering

use crate::keyframe::{AnimatedProperty, KeyframeValue, PathData};
use crate::model::MaskModel;
use skia_rs_core::Scalar;
use skia_rs_path::{Path, PathBuilder};

/// Mask mode (boolean operation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskMode {
    /// No mask.
    None,
    /// Add to mask (union).
    Add,
    /// Subtract from mask.
    Subtract,
    /// Intersect with mask.
    Intersect,
    /// Lighten (max).
    Lighten,
    /// Darken (min).
    Darken,
    /// Difference.
    Difference,
}

impl From<&str> for MaskMode {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "n" | "none" => MaskMode::None,
            "a" | "add" => MaskMode::Add,
            "s" | "subtract" => MaskMode::Subtract,
            "i" | "intersect" => MaskMode::Intersect,
            "l" | "lighten" => MaskMode::Lighten,
            "d" | "darken" => MaskMode::Darken,
            "f" | "difference" => MaskMode::Difference,
            _ => MaskMode::Add,
        }
    }
}

/// Track matte mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatteMode {
    /// No matte.
    None,
    /// Alpha matte (use alpha channel).
    Alpha,
    /// Inverted alpha matte.
    AlphaInverted,
    /// Luma matte (use luminance).
    Luma,
    /// Inverted luma matte.
    LumaInverted,
}

impl From<i32> for MatteMode {
    fn from(value: i32) -> Self {
        match value {
            1 => MatteMode::Alpha,
            2 => MatteMode::AlphaInverted,
            3 => MatteMode::Luma,
            4 => MatteMode::LumaInverted,
            _ => MatteMode::None,
        }
    }
}

/// A mask on a layer.
#[derive(Debug, Clone)]
pub struct Mask {
    /// Mask name.
    pub name: String,
    /// Mask mode.
    pub mode: MaskMode,
    /// Mask path.
    pub path: AnimatedProperty,
    /// Mask opacity (0-100).
    pub opacity: AnimatedProperty,
    /// Inverted mask.
    pub inverted: bool,
    /// Mask expansion (pixels).
    pub expansion: AnimatedProperty,
}

impl Mask {
    /// Create a new mask.
    pub fn new(mode: MaskMode) -> Self {
        Self {
            name: String::new(),
            mode,
            path: AnimatedProperty::default(),
            opacity: AnimatedProperty::static_value(KeyframeValue::Scalar(100.0)),
            inverted: false,
            expansion: AnimatedProperty::static_value(KeyframeValue::Scalar(0.0)),
        }
    }

    /// Parse from Lottie mask model.
    pub fn from_lottie(model: &MaskModel) -> Self {
        Self {
            name: String::new(),
            mode: MaskMode::from(model.mode.as_str()),
            path: AnimatedProperty::from_lottie(&model.path),
            opacity: AnimatedProperty::from_lottie(&model.opacity),
            inverted: model.inverted,
            expansion: model
                .expansion
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_else(|| AnimatedProperty::static_value(KeyframeValue::Scalar(0.0))),
        }
    }

    /// Get the mask path at a specific frame.
    pub fn path_at(&self, frame: Scalar) -> Option<Path> {
        let value = self.path.value_at(frame);
        
        match value {
            KeyframeValue::Path(path_data) => Some(path_data_to_path(&path_data)),
            _ => None,
        }
    }

    /// Get the opacity at a specific frame (0.0 - 1.0).
    pub fn opacity_at(&self, frame: Scalar) -> Scalar {
        let opacity = self.opacity.value_at(frame).as_scalar().unwrap_or(100.0);
        (opacity / 100.0).clamp(0.0, 1.0)
    }

    /// Get the expansion at a specific frame.
    pub fn expansion_at(&self, frame: Scalar) -> Scalar {
        self.expansion.value_at(frame).as_scalar().unwrap_or(0.0)
    }

    /// Check if this mask affects rendering (not None mode with zero opacity).
    pub fn is_active(&self, frame: Scalar) -> bool {
        self.mode != MaskMode::None && self.opacity_at(frame) > 0.0
    }
}

/// Convert PathData to skia Path.
fn path_data_to_path(data: &PathData) -> Path {
    let mut builder = PathBuilder::new();

    if data.vertices.is_empty() {
        return builder.build();
    }

    let n = data.vertices.len();
    builder.move_to(data.vertices[0][0], data.vertices[0][1]);

    for i in 1..n {
        let prev = i - 1;
        let out_t = data.out_tangents.get(prev).copied().unwrap_or([0.0, 0.0]);
        let in_t = data.in_tangents.get(i).copied().unwrap_or([0.0, 0.0]);

        let c1 = [
            data.vertices[prev][0] + out_t[0],
            data.vertices[prev][1] + out_t[1],
        ];
        let c2 = [
            data.vertices[i][0] + in_t[0],
            data.vertices[i][1] + in_t[1],
        ];

        if out_t == [0.0, 0.0] && in_t == [0.0, 0.0] {
            builder.line_to(data.vertices[i][0], data.vertices[i][1]);
        } else {
            builder.cubic_to(c1[0], c1[1], c2[0], c2[1], data.vertices[i][0], data.vertices[i][1]);
        }
    }

    if data.closed && n > 1 {
        let last = n - 1;
        let out_t = data.out_tangents.get(last).copied().unwrap_or([0.0, 0.0]);
        let in_t = data.in_tangents.get(0).copied().unwrap_or([0.0, 0.0]);

        let c1 = [
            data.vertices[last][0] + out_t[0],
            data.vertices[last][1] + out_t[1],
        ];
        let c2 = [
            data.vertices[0][0] + in_t[0],
            data.vertices[0][1] + in_t[1],
        ];

        if out_t == [0.0, 0.0] && in_t == [0.0, 0.0] {
            builder.close();
        } else {
            builder.cubic_to(c1[0], c1[1], c2[0], c2[1], data.vertices[0][0], data.vertices[0][1]);
            builder.close();
        }
    }

    builder.build()
}

/// Mask group for a layer (combines multiple masks).
#[derive(Debug, Clone, Default)]
pub struct MaskGroup {
    /// Individual masks.
    pub masks: Vec<Mask>,
}

impl MaskGroup {
    /// Create a new empty mask group.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a mask.
    pub fn add(&mut self, mask: Mask) {
        self.masks.push(mask);
    }

    /// Check if the group has any active masks.
    pub fn has_active_masks(&self, frame: Scalar) -> bool {
        self.masks.iter().any(|m| m.is_active(frame))
    }

    /// Get all active mask paths at a frame.
    pub fn get_mask_paths(&self, frame: Scalar) -> Vec<(Path, MaskMode, Scalar)> {
        self.masks
            .iter()
            .filter(|m| m.is_active(frame))
            .filter_map(|m| {
                m.path_at(frame).map(|path| {
                    (path, m.mode, m.opacity_at(frame))
                })
            })
            .collect()
    }

    /// Compute the combined mask path.
    ///
    /// This performs boolean operations to combine masks.
    pub fn compute_combined_path(&self, frame: Scalar) -> Option<Path> {
        let mask_paths = self.get_mask_paths(frame);
        
        if mask_paths.is_empty() {
            return None;
        }

        // Start with first mask
        let mut result = mask_paths[0].0.clone();

        // Combine subsequent masks
        for (path, mode, _opacity) in mask_paths.iter().skip(1) {
            result = match mode {
                MaskMode::Add => {
                    // Union - combine paths
                    combine_paths(&result, path)
                }
                MaskMode::Subtract => {
                    // Difference
                    subtract_path(&result, path)
                }
                MaskMode::Intersect => {
                    // Intersection
                    intersect_paths(&result, path)
                }
                _ => result,
            };
        }

        Some(result)
    }
}

/// Combine two paths (union).
fn combine_paths(a: &Path, b: &Path) -> Path {
    // Simple implementation: just append paths
    // A proper implementation would use path boolean operations
    let mut builder = PathBuilder::new();
    
    for element in a.iter() {
        match element {
            skia_rs_path::PathElement::Move(p) => builder.move_to(p.x, p.y),
            skia_rs_path::PathElement::Line(p) => builder.line_to(p.x, p.y),
            skia_rs_path::PathElement::Quad(c, p) => builder.quad_to(c.x, c.y, p.x, p.y),
            skia_rs_path::PathElement::Conic(c, p, w) => builder.conic_to(c.x, c.y, p.x, p.y, w),
            skia_rs_path::PathElement::Cubic(c1, c2, p) => {
                builder.cubic_to(c1.x, c1.y, c2.x, c2.y, p.x, p.y)
            }
            skia_rs_path::PathElement::Close => builder.close(),
        };
    }
    
    for element in b.iter() {
        match element {
            skia_rs_path::PathElement::Move(p) => builder.move_to(p.x, p.y),
            skia_rs_path::PathElement::Line(p) => builder.line_to(p.x, p.y),
            skia_rs_path::PathElement::Quad(c, p) => builder.quad_to(c.x, c.y, p.x, p.y),
            skia_rs_path::PathElement::Conic(c, p, w) => builder.conic_to(c.x, c.y, p.x, p.y, w),
            skia_rs_path::PathElement::Cubic(c1, c2, p) => {
                builder.cubic_to(c1.x, c1.y, c2.x, c2.y, p.x, p.y)
            }
            skia_rs_path::PathElement::Close => builder.close(),
        };
    }
    
    builder.build()
}

/// Subtract path b from path a.
fn subtract_path(a: &Path, _b: &Path) -> Path {
    // Simplified: just return a (proper implementation would use path ops)
    a.clone()
}

/// Intersect two paths.
fn intersect_paths(a: &Path, _b: &Path) -> Path {
    // Simplified: just return a (proper implementation would use path ops)
    a.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_mode_from_string() {
        assert_eq!(MaskMode::from("a"), MaskMode::Add);
        assert_eq!(MaskMode::from("s"), MaskMode::Subtract);
        assert_eq!(MaskMode::from("i"), MaskMode::Intersect);
        assert_eq!(MaskMode::from("n"), MaskMode::None);
    }

    #[test]
    fn test_matte_mode_from_int() {
        assert_eq!(MatteMode::from(1), MatteMode::Alpha);
        assert_eq!(MatteMode::from(3), MatteMode::Luma);
        assert_eq!(MatteMode::from(0), MatteMode::None);
    }

    #[test]
    fn test_mask_opacity() {
        let mask = Mask::new(MaskMode::Add);
        assert_eq!(mask.opacity_at(0.0), 1.0); // Default 100%
    }

    #[test]
    fn test_mask_group() {
        let mut group = MaskGroup::new();
        group.add(Mask::new(MaskMode::Add));
        group.add(Mask::new(MaskMode::Subtract));
        
        assert_eq!(group.masks.len(), 2);
    }
}
