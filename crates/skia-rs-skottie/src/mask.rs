//! Mask and matte support for Lottie animations.
//!
//! This module handles:
//! - Mask shapes (Add, Subtract, Intersect, Difference)
//! - Track mattes (Alpha, Luma)
//! - Mask expansion and feathering

use crate::keyframe::{AnimatedProperty, KeyframeValue, PathData};
use crate::model::MaskModel;
use skia_rs_core::{Rect, Scalar};
use skia_rs_path::ops::{PathOp, op};
use skia_rs_path::{FillType, Path, PathBuilder};

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
        let c2 = [data.vertices[i][0] + in_t[0], data.vertices[i][1] + in_t[1]];

        if out_t == [0.0, 0.0] && in_t == [0.0, 0.0] {
            builder.line_to(data.vertices[i][0], data.vertices[i][1]);
        } else {
            builder.cubic_to(
                c1[0],
                c1[1],
                c2[0],
                c2[1],
                data.vertices[i][0],
                data.vertices[i][1],
            );
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
        let c2 = [data.vertices[0][0] + in_t[0], data.vertices[0][1] + in_t[1]];

        if out_t == [0.0, 0.0] && in_t == [0.0, 0.0] {
            builder.close();
        } else {
            builder.cubic_to(
                c1[0],
                c1[1],
                c2[0],
                c2[1],
                data.vertices[0][0],
                data.vertices[0][1],
            );
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
}

/// Build the combined clip path for a set of masks at a frame.
///
/// Matches upstream `AttachMask` (`Layer.cpp`): Add unions, Subtract
/// subtracts (`kDstOut`), Intersect intersects, Difference xors; `inv`
/// inverts an individual mask's geometry (relative to `bounds`); the
/// *first* mask in the stack always draws in "source" mode, with its
/// effective inversion flipped when its own mode is Subtract.
pub fn build_clip(masks: &[Mask], frame: Scalar, bounds: Rect) -> Option<Path> {
    let mut result: Option<Path> = None;

    for (i, mask) in masks.iter().filter(|m| m.is_active(frame)).enumerate() {
        let Some(path) = mask.path_at(frame) else {
            continue;
        };

        let effective_inverted = if i == 0 {
            // First mask: always "source" mode; Subtract's geometry is
            // implicitly inverted, so an explicit `inv` flag flips it back.
            mask.inverted != (mask.mode == MaskMode::Subtract)
        } else {
            mask.inverted
        };

        let path = if effective_inverted {
            invert_path(&path, bounds)
        } else {
            path
        };

        result = Some(match result {
            None => path,
            Some(acc) => {
                let combined = match mask.mode {
                    MaskMode::Add => op(&acc, &path, PathOp::Union),
                    MaskMode::Subtract => op(&acc, &path, PathOp::Difference),
                    MaskMode::Intersect => op(&acc, &path, PathOp::Intersect),
                    MaskMode::Difference => op(&acc, &path, PathOp::Xor),
                    _ => Some(acc.clone()),
                };
                combined.unwrap_or(acc)
            }
        });
    }

    result
}

/// Invert a path's geometry relative to `bounds` (i.e. "everywhere in
/// `bounds` except `path`"), used to resolve Lottie's `inv` mask flag.
fn invert_path(path: &Path, bounds: Rect) -> Path {
    let mut universe = PathBuilder::new();
    universe.add_rect(&bounds);
    let universe = universe.build();
    op(&universe, path, PathOp::Difference).unwrap_or_else(|| {
        let mut p = path.clone();
        p.set_fill_type(FillType::InverseWinding);
        p
    })
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

    fn rect_mask(mode: MaskMode, x: Scalar, y: Scalar, w: Scalar, h: Scalar, inv: bool) -> Mask {
        let path_data = PathData {
            vertices: vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
            in_tangents: vec![[0.0, 0.0]; 4],
            out_tangents: vec![[0.0, 0.0]; 4],
            closed: true,
        };

        Mask {
            name: String::new(),
            mode,
            path: AnimatedProperty::static_value(KeyframeValue::Path(path_data)),
            opacity: AnimatedProperty::static_value(KeyframeValue::Scalar(100.0)),
            inverted: inv,
            expansion: AnimatedProperty::static_value(KeyframeValue::Scalar(0.0)),
        }
    }

    #[test]
    fn test_build_clip_subtract_removes_geometry() {
        let masks = vec![
            rect_mask(MaskMode::Add, 0.0, 0.0, 100.0, 100.0, false),
            rect_mask(MaskMode::Subtract, 25.0, 25.0, 50.0, 50.0, false),
        ];
        let bounds = skia_rs_core::Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
        let clip = build_clip(&masks, 0.0, bounds).unwrap();

        // The subtracted center should no longer be inside the clip region.
        assert!(!clip.contains(skia_rs_core::Point::new(50.0, 50.0)));
        // A corner still inside the outer rect (but outside the hole) should remain.
        assert!(clip.contains(skia_rs_core::Point::new(5.0, 5.0)));
    }

    #[test]
    fn test_build_clip_intersect_shrinks_geometry() {
        let masks = vec![
            rect_mask(MaskMode::Add, 0.0, 0.0, 100.0, 100.0, false),
            rect_mask(MaskMode::Intersect, 25.0, 25.0, 50.0, 50.0, false),
        ];
        let bounds = skia_rs_core::Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
        let clip = build_clip(&masks, 0.0, bounds).unwrap();

        assert!(clip.contains(skia_rs_core::Point::new(50.0, 50.0)));
        // Outside the intersected (smaller) region.
        assert!(!clip.contains(skia_rs_core::Point::new(5.0, 5.0)));
    }

    #[test]
    fn test_build_clip_add_unions_geometry() {
        let masks = vec![
            rect_mask(MaskMode::Add, 0.0, 0.0, 40.0, 40.0, false),
            rect_mask(MaskMode::Add, 60.0, 60.0, 40.0, 40.0, false),
        ];
        let bounds = skia_rs_core::Rect::from_xywh(0.0, 0.0, 100.0, 100.0);
        let clip = build_clip(&masks, 0.0, bounds).unwrap();

        assert!(clip.contains(skia_rs_core::Point::new(20.0, 20.0)));
        assert!(clip.contains(skia_rs_core::Point::new(80.0, 80.0)));
        assert!(!clip.contains(skia_rs_core::Point::new(50.0, 50.0)));
    }
}
