//! Stencil-then-cover algorithm for complex path rendering.
//!
//! This module implements the stencil-then-cover technique for GPU rendering
//! of complex paths with correct winding rule handling.

use crate::tessellation::{TessIndex, TessMesh, TessVertex};
use skia_rs_core::{Point, Rect, Scalar};
use skia_rs_path::{FillType, Path, PathBuilder, PathElement};

/// Fill rule for stencil operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StencilFillRule {
    /// Non-zero winding rule.
    #[default]
    NonZero,
    /// Even-odd (parity) rule.
    EvenOdd,
}

impl From<FillType> for StencilFillRule {
    fn from(fill_type: FillType) -> Self {
        match fill_type {
            FillType::Winding | FillType::InverseWinding => StencilFillRule::NonZero,
            FillType::EvenOdd | FillType::InverseEvenOdd => StencilFillRule::EvenOdd,
        }
    }
}

/// Stencil operation for path rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StencilOp {
    /// Keep current value.
    Keep,
    /// Set to zero.
    Zero,
    /// Replace with reference value.
    Replace,
    /// Increment (saturate).
    IncrSat,
    /// Decrement (saturate).
    DecrSat,
    /// Increment (wrap).
    IncrWrap,
    /// Decrement (wrap).
    DecrWrap,
    /// Invert bits.
    Invert,
}

/// Stencil state configuration.
#[derive(Debug, Clone)]
pub struct StencilState {
    /// Stencil test enabled.
    pub enabled: bool,
    /// Stencil function for front faces.
    pub front_func: StencilFunc,
    /// Stencil operations for front faces.
    pub front_ops: StencilOps,
    /// Stencil function for back faces.
    pub back_func: StencilFunc,
    /// Stencil operations for back faces.
    pub back_ops: StencilOps,
    /// Reference value.
    pub reference: u32,
    /// Read mask.
    pub read_mask: u32,
    /// Write mask.
    pub write_mask: u32,
}

impl Default for StencilState {
    fn default() -> Self {
        Self {
            enabled: false,
            front_func: StencilFunc::Always,
            front_ops: StencilOps::default(),
            back_func: StencilFunc::Always,
            back_ops: StencilOps::default(),
            reference: 0,
            read_mask: 0xFF,
            write_mask: 0xFF,
        }
    }
}

/// Stencil comparison function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StencilFunc {
    /// Never pass.
    Never,
    /// Always pass.
    #[default]
    Always,
    /// Pass if equal.
    Equal,
    /// Pass if not equal.
    NotEqual,
    /// Pass if less.
    Less,
    /// Pass if less or equal.
    LessEqual,
    /// Pass if greater.
    Greater,
    /// Pass if greater or equal.
    GreaterEqual,
}

/// Stencil operations for fail/pass conditions.
#[derive(Debug, Clone, Copy)]
pub struct StencilOps {
    /// Operation when stencil test fails.
    pub stencil_fail: StencilOp,
    /// Operation when stencil passes but depth fails.
    pub depth_fail: StencilOp,
    /// Operation when both stencil and depth pass.
    pub pass: StencilOp,
}

impl Default for StencilOps {
    fn default() -> Self {
        Self {
            stencil_fail: StencilOp::Keep,
            depth_fail: StencilOp::Keep,
            pass: StencilOp::Keep,
        }
    }
}

/// Configuration for stencil-then-cover rendering.
#[derive(Debug, Clone)]
pub struct StencilCoverConfig {
    /// Fill rule to use.
    pub fill_rule: StencilFillRule,
    /// Whether to use two-sided stencil (for non-zero winding).
    pub two_sided: bool,
}

impl Default for StencilCoverConfig {
    fn default() -> Self {
        Self {
            fill_rule: StencilFillRule::NonZero,
            two_sided: true,
        }
    }
}

/// Stencil pass data for GPU rendering.
#[derive(Debug, Clone)]
pub struct StencilPass {
    /// Mesh to render for stencil filling.
    pub mesh: TessMesh,
    /// Stencil state for this pass.
    pub stencil_state: StencilState,
    /// Whether color writes should be disabled.
    pub color_write_disabled: bool,
}

/// Cover pass data for GPU rendering.
#[derive(Debug, Clone)]
pub struct CoverPass {
    /// Mesh to render for covering.
    pub mesh: TessMesh,
    /// Stencil state for this pass.
    pub stencil_state: StencilState,
    /// Bounding rect of the path.
    pub bounds: Rect,
}

/// Result of stencil-then-cover preparation.
#[derive(Debug, Clone)]
pub struct StencilCoverResult {
    /// Stencil pass.
    pub stencil_pass: StencilPass,
    /// Cover pass.
    pub cover_pass: CoverPass,
}

/// Prepare stencil-then-cover data for a path.
pub fn prepare_stencil_cover(path: &Path, config: &StencilCoverConfig) -> StencilCoverResult {
    // Create stencil mesh from path triangulation
    let stencil_mesh = tessellate_path_for_stencil(path);

    // Create cover mesh (bounding box)
    let bounds = path.bounds();
    let cover_mesh = create_cover_mesh(bounds);

    // Configure stencil states based on fill rule
    let (stencil_state, cover_stencil_state) = match config.fill_rule {
        StencilFillRule::NonZero => create_nonzero_stencil_states(config.two_sided),
        StencilFillRule::EvenOdd => create_evenodd_stencil_states(),
    };

    StencilCoverResult {
        stencil_pass: StencilPass {
            mesh: stencil_mesh,
            stencil_state,
            color_write_disabled: true,
        },
        cover_pass: CoverPass {
            mesh: cover_mesh,
            stencil_state: cover_stencil_state,
            bounds,
        },
    }
}

/// Tessellate a path for stencil rendering.
fn tessellate_path_for_stencil(path: &Path) -> TessMesh {
    let mut mesh = TessMesh::new();

    // Use centroid as fan origin
    let bounds = path.bounds();
    let center = bounds.center();
    let fan_origin = Point::new(center.x, center.y);
    let origin_idx = mesh.add_vertex(TessVertex::from_point(fan_origin));

    let mut current_point = Point::zero();
    let mut prev_vertex: Option<TessIndex> = None;

    for element in path.iter() {
        match element {
            PathElement::Move(p) => {
                current_point = p;
                prev_vertex = Some(mesh.add_vertex(TessVertex::from_point(p)));
            }
            PathElement::Line(p) => {
                let curr_vertex = mesh.add_vertex(TessVertex::from_point(p));
                if let Some(prev) = prev_vertex {
                    mesh.add_triangle(origin_idx, prev, curr_vertex);
                }
                prev_vertex = Some(curr_vertex);
                current_point = p;
            }
            PathElement::Quad(ctrl, end) => {
                let steps = 8;
                for i in 1..=steps {
                    let t = i as Scalar / steps as Scalar;
                    let p = eval_quad(current_point, ctrl, end, t);
                    let curr_vertex = mesh.add_vertex(TessVertex::from_point(p));
                    if let Some(prev) = prev_vertex {
                        mesh.add_triangle(origin_idx, prev, curr_vertex);
                    }
                    prev_vertex = Some(curr_vertex);
                }
                current_point = end;
            }
            PathElement::Conic(ctrl, end, weight) => {
                let steps = 8;
                for i in 1..=steps {
                    let t = i as Scalar / steps as Scalar;
                    let p = eval_conic(current_point, ctrl, end, weight, t);
                    let curr_vertex = mesh.add_vertex(TessVertex::from_point(p));
                    if let Some(prev) = prev_vertex {
                        mesh.add_triangle(origin_idx, prev, curr_vertex);
                    }
                    prev_vertex = Some(curr_vertex);
                }
                current_point = end;
            }
            PathElement::Cubic(ctrl1, ctrl2, end) => {
                let steps = 12;
                for i in 1..=steps {
                    let t = i as Scalar / steps as Scalar;
                    let p = eval_cubic(current_point, ctrl1, ctrl2, end, t);
                    let curr_vertex = mesh.add_vertex(TessVertex::from_point(p));
                    if let Some(prev) = prev_vertex {
                        mesh.add_triangle(origin_idx, prev, curr_vertex);
                    }
                    prev_vertex = Some(curr_vertex);
                }
                current_point = end;
            }
            PathElement::Close => {
                prev_vertex = None;
            }
        }
    }

    mesh
}

/// Create a cover mesh from bounding rect.
fn create_cover_mesh(bounds: Rect) -> TessMesh {
    // Add small padding to ensure full coverage
    let padding = 1.0;
    let padded = Rect::new(
        bounds.left - padding,
        bounds.top - padding,
        bounds.right + padding,
        bounds.bottom + padding,
    );

    crate::tessellation::tessellate_rect(padded)
}

/// Create stencil states for non-zero winding rule.
fn create_nonzero_stencil_states(two_sided: bool) -> (StencilState, StencilState) {
    if two_sided {
        let stencil = StencilState {
            enabled: true,
            front_func: StencilFunc::Always,
            front_ops: StencilOps {
                stencil_fail: StencilOp::Keep,
                depth_fail: StencilOp::Keep,
                pass: StencilOp::IncrWrap,
            },
            back_func: StencilFunc::Always,
            back_ops: StencilOps {
                stencil_fail: StencilOp::Keep,
                depth_fail: StencilOp::Keep,
                pass: StencilOp::DecrWrap,
            },
            reference: 0,
            read_mask: 0xFF,
            write_mask: 0xFF,
        };

        let cover = StencilState {
            enabled: true,
            front_func: StencilFunc::NotEqual,
            front_ops: StencilOps {
                stencil_fail: StencilOp::Keep,
                depth_fail: StencilOp::Keep,
                pass: StencilOp::Zero,
            },
            back_func: StencilFunc::NotEqual,
            back_ops: StencilOps {
                stencil_fail: StencilOp::Keep,
                depth_fail: StencilOp::Keep,
                pass: StencilOp::Zero,
            },
            reference: 0,
            read_mask: 0xFF,
            write_mask: 0xFF,
        };

        (stencil, cover)
    } else {
        let stencil = StencilState {
            enabled: true,
            front_func: StencilFunc::Always,
            front_ops: StencilOps {
                stencil_fail: StencilOp::Keep,
                depth_fail: StencilOp::Keep,
                pass: StencilOp::IncrWrap,
            },
            back_func: StencilFunc::Always,
            back_ops: StencilOps {
                stencil_fail: StencilOp::Keep,
                depth_fail: StencilOp::Keep,
                pass: StencilOp::IncrWrap,
            },
            reference: 0,
            read_mask: 0xFF,
            write_mask: 0xFF,
        };

        let cover = StencilState {
            enabled: true,
            front_func: StencilFunc::NotEqual,
            front_ops: StencilOps {
                stencil_fail: StencilOp::Keep,
                depth_fail: StencilOp::Keep,
                pass: StencilOp::Zero,
            },
            back_func: StencilFunc::NotEqual,
            back_ops: StencilOps {
                stencil_fail: StencilOp::Keep,
                depth_fail: StencilOp::Keep,
                pass: StencilOp::Zero,
            },
            reference: 0,
            read_mask: 0xFF,
            write_mask: 0xFF,
        };

        (stencil, cover)
    }
}

/// Create stencil states for even-odd rule.
fn create_evenodd_stencil_states() -> (StencilState, StencilState) {
    let stencil = StencilState {
        enabled: true,
        front_func: StencilFunc::Always,
        front_ops: StencilOps {
            stencil_fail: StencilOp::Keep,
            depth_fail: StencilOp::Keep,
            pass: StencilOp::Invert,
        },
        back_func: StencilFunc::Always,
        back_ops: StencilOps {
            stencil_fail: StencilOp::Keep,
            depth_fail: StencilOp::Keep,
            pass: StencilOp::Invert,
        },
        reference: 0,
        read_mask: 0xFF,
        write_mask: 0x01,
    };

    let cover = StencilState {
        enabled: true,
        front_func: StencilFunc::NotEqual,
        front_ops: StencilOps {
            stencil_fail: StencilOp::Keep,
            depth_fail: StencilOp::Keep,
            pass: StencilOp::Zero,
        },
        back_func: StencilFunc::NotEqual,
        back_ops: StencilOps {
            stencil_fail: StencilOp::Keep,
            depth_fail: StencilOp::Keep,
            pass: StencilOp::Zero,
        },
        reference: 0,
        read_mask: 0x01,
        write_mask: 0xFF,
    };

    (stencil, cover)
}

// Curve evaluation helpers

fn eval_quad(p0: Point, p1: Point, p2: Point, t: Scalar) -> Point {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let t2 = t * t;
    Point::new(
        mt2 * p0.x + 2.0 * mt * t * p1.x + t2 * p2.x,
        mt2 * p0.y + 2.0 * mt * t * p1.y + t2 * p2.y,
    )
}

fn eval_conic(p0: Point, p1: Point, p2: Point, w: Scalar, t: Scalar) -> Point {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let t2 = t * t;
    let wt = 2.0 * w * mt * t;
    let denom = mt2 + wt + t2;
    Point::new(
        (mt2 * p0.x + wt * p1.x + t2 * p2.x) / denom,
        (mt2 * p0.y + wt * p1.y + t2 * p2.y) / denom,
    )
}

fn eval_cubic(p0: Point, p1: Point, p2: Point, p3: Point, t: Scalar) -> Point {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    let t2 = t * t;
    let t3 = t2 * t;
    Point::new(
        mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x,
        mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stencil_fill_rule_conversion() {
        assert_eq!(StencilFillRule::from(FillType::Winding), StencilFillRule::NonZero);
        assert_eq!(StencilFillRule::from(FillType::EvenOdd), StencilFillRule::EvenOdd);
    }

    #[test]
    fn test_stencil_state_default() {
        let state = StencilState::default();
        assert!(!state.enabled);
        assert_eq!(state.reference, 0);
        assert_eq!(state.read_mask, 0xFF);
    }

    #[test]
    fn test_prepare_stencil_cover() {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0)
            .line_to(100.0, 0.0)
            .line_to(100.0, 100.0)
            .line_to(0.0, 100.0)
            .close();
        let path = builder.build();

        let result = prepare_stencil_cover(&path, &StencilCoverConfig::default());

        assert!(!result.stencil_pass.mesh.is_empty());
        assert!(result.stencil_pass.color_write_disabled);
        assert!(result.stencil_pass.stencil_state.enabled);

        assert!(!result.cover_pass.mesh.is_empty());
        assert!(result.cover_pass.stencil_state.enabled);
    }

    #[test]
    fn test_nonzero_stencil_states() {
        let (stencil, cover) = create_nonzero_stencil_states(true);

        assert!(stencil.enabled);
        assert_eq!(stencil.front_ops.pass, StencilOp::IncrWrap);
        assert_eq!(stencil.back_ops.pass, StencilOp::DecrWrap);

        assert!(cover.enabled);
        assert_eq!(cover.front_func, StencilFunc::NotEqual);
    }

    #[test]
    fn test_evenodd_stencil_states() {
        let (stencil, cover) = create_evenodd_stencil_states();

        assert!(stencil.enabled);
        assert_eq!(stencil.front_ops.pass, StencilOp::Invert);
        assert_eq!(stencil.write_mask, 0x01);

        assert!(cover.enabled);
        assert_eq!(cover.read_mask, 0x01);
    }
}
