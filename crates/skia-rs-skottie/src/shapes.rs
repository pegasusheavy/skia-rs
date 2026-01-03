//! Shape layers for Lottie animations.
//!
//! This module handles shape layer elements:
//! - Rectangle
//! - Ellipse
//! - Path
//! - Polystar (star/polygon)
//! - Fill
//! - Stroke
//! - Gradient fill/stroke
//! - Group
//! - Trim paths
//! - Merge paths
//! - Repeater

use crate::keyframe::{AnimatedProperty, KeyframeValue, PathData};
use crate::model::ShapeModel;
use crate::transform::Transform;
use skia_rs_core::{Color4f, Scalar};
use skia_rs_paint::{StrokeCap, StrokeJoin};
use skia_rs_path::{Path, PathBuilder};

/// Shape element types.
#[derive(Debug, Clone)]
pub enum Shape {
    /// Group of shapes.
    Group(ShapeGroup),
    /// Rectangle.
    Rectangle(RectangleShape),
    /// Ellipse.
    Ellipse(EllipseShape),
    /// Path.
    Path(PathShape),
    /// Polystar (star or polygon).
    Polystar(PolystarShape),
    /// Fill.
    Fill(FillShape),
    /// Stroke.
    Stroke(StrokeShape),
    /// Gradient fill.
    GradientFill(GradientFillShape),
    /// Gradient stroke.
    GradientStroke(GradientStrokeShape),
    /// Trim paths.
    TrimPath(TrimPathShape),
    /// Merge paths.
    MergePaths(MergePathsShape),
    /// Round corners.
    RoundCorners(RoundCornersShape),
    /// Repeater.
    Repeater(RepeaterShape),
    /// Transform.
    Transform(ShapeTransform),
}

impl Shape {
    /// Parse from Lottie shape model.
    pub fn from_lottie(model: &ShapeModel) -> Option<Self> {
        if model.hidden {
            return None;
        }

        match model.shape_type.as_str() {
            "gr" => Some(Shape::Group(ShapeGroup::from_lottie(model))),
            "rc" => Some(Shape::Rectangle(RectangleShape::from_lottie(model))),
            "el" => Some(Shape::Ellipse(EllipseShape::from_lottie(model))),
            "sh" => Some(Shape::Path(PathShape::from_lottie(model))),
            "sr" => Some(Shape::Polystar(PolystarShape::from_lottie(model))),
            "fl" => Some(Shape::Fill(FillShape::from_lottie(model))),
            "st" => Some(Shape::Stroke(StrokeShape::from_lottie(model))),
            "gf" => Some(Shape::GradientFill(GradientFillShape::from_lottie(model))),
            "gs" => Some(Shape::GradientStroke(GradientStrokeShape::from_lottie(model))),
            "tm" => Some(Shape::TrimPath(TrimPathShape::from_lottie(model))),
            "mm" => Some(Shape::MergePaths(MergePathsShape::from_lottie(model))),
            "rd" => Some(Shape::RoundCorners(RoundCornersShape::from_lottie(model))),
            "rp" => Some(Shape::Repeater(RepeaterShape::from_lottie(model))),
            "tr" => Some(Shape::Transform(ShapeTransform::from_lottie(model))),
            _ => None, // Unknown shape type
        }
    }
}

/// Group of shapes.
#[derive(Debug, Clone)]
pub struct ShapeGroup {
    /// Group name.
    pub name: String,
    /// Child shapes.
    pub shapes: Vec<Shape>,
    /// Group transform (usually the last "tr" item).
    pub transform: Option<Transform>,
}

impl ShapeGroup {
    /// Create a new empty group.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            shapes: Vec::new(),
            transform: None,
        }
    }

    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        let mut group = Self::new(&model.name);

        for item in &model.items {
            if item.shape_type == "tr" {
                // Transform item - shape transforms use a simplified structure
                group.transform = Some(Transform::default());
            } else if let Some(shape) = Shape::from_lottie(item) {
                group.shapes.push(shape);
            }
        }

        group
    }

    /// Build paths for this group at a specific frame.
    pub fn build_paths(&self, frame: Scalar) -> Vec<Path> {
        let mut paths = Vec::new();

        for shape in &self.shapes {
            match shape {
                Shape::Rectangle(rect) => {
                    if let Some(path) = rect.to_path(frame) {
                        paths.push(path);
                    }
                }
                Shape::Ellipse(ellipse) => {
                    if let Some(path) = ellipse.to_path(frame) {
                        paths.push(path);
                    }
                }
                Shape::Path(path_shape) => {
                    if let Some(path) = path_shape.to_path(frame) {
                        paths.push(path);
                    }
                }
                Shape::Polystar(star) => {
                    if let Some(path) = star.to_path(frame) {
                        paths.push(path);
                    }
                }
                Shape::Group(sub_group) => {
                    paths.extend(sub_group.build_paths(frame));
                }
                _ => {}
            }
        }

        paths
    }
}

/// Rectangle shape.
#[derive(Debug, Clone)]
pub struct RectangleShape {
    /// Shape name.
    pub name: String,
    /// Position.
    pub position: AnimatedProperty,
    /// Size.
    pub size: AnimatedProperty,
    /// Corner roundness.
    pub roundness: AnimatedProperty,
    /// Direction (1=clockwise, 3=counter-clockwise).
    pub direction: i32,
}

impl RectangleShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        Self {
            name: model.name.clone(),
            position: model
                .position
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            size: model
                .size
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            roundness: model
                .roundness
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            direction: model.direction.unwrap_or(1),
        }
    }

    /// Build a path at a specific frame.
    pub fn to_path(&self, frame: Scalar) -> Option<Path> {
        let pos = self.position.value_at(frame).as_vec2().unwrap_or([0.0, 0.0]);
        let size = self.size.value_at(frame).as_vec2().unwrap_or([0.0, 0.0]);
        let roundness = self.roundness.value_at(frame).as_scalar().unwrap_or(0.0);

        if size[0] <= 0.0 || size[1] <= 0.0 {
            return None;
        }

        let half_w = size[0] / 2.0;
        let half_h = size[1] / 2.0;
        let left = pos[0] - half_w;
        let top = pos[1] - half_h;

        let mut builder = PathBuilder::new();

        if roundness > 0.0 {
            let r = roundness.min(half_w).min(half_h);

            builder.move_to(left + r, top);
            builder.line_to(left + size[0] - r, top);
            builder.quad_to(left + size[0], top, left + size[0], top + r);
            builder.line_to(left + size[0], top + size[1] - r);
            builder.quad_to(left + size[0], top + size[1], left + size[0] - r, top + size[1]);
            builder.line_to(left + r, top + size[1]);
            builder.quad_to(left, top + size[1], left, top + size[1] - r);
            builder.line_to(left, top + r);
            builder.quad_to(left, top, left + r, top);
        } else {
            builder.move_to(left, top);
            builder.line_to(left + size[0], top);
            builder.line_to(left + size[0], top + size[1]);
            builder.line_to(left, top + size[1]);
        }

        builder.close();
        Some(builder.build())
    }
}

/// Ellipse shape.
#[derive(Debug, Clone)]
pub struct EllipseShape {
    /// Shape name.
    pub name: String,
    /// Position.
    pub position: AnimatedProperty,
    /// Size.
    pub size: AnimatedProperty,
    /// Direction.
    pub direction: i32,
}

impl EllipseShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        Self {
            name: model.name.clone(),
            position: model
                .position
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            size: model
                .size
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            direction: model.direction.unwrap_or(1),
        }
    }

    /// Build a path at a specific frame.
    pub fn to_path(&self, frame: Scalar) -> Option<Path> {
        let pos = self.position.value_at(frame).as_vec2().unwrap_or([0.0, 0.0]);
        let size = self.size.value_at(frame).as_vec2().unwrap_or([0.0, 0.0]);

        if size[0] <= 0.0 || size[1] <= 0.0 {
            return None;
        }

        let rx = size[0] / 2.0;
        let ry = size[1] / 2.0;
        let cx = pos[0];
        let cy = pos[1];

        // Approximate ellipse with 4 cubic bezier curves
        let k = 0.5522847498; // (4/3) * tan(π/8)
        let kx = rx * k;
        let ky = ry * k;

        let mut builder = PathBuilder::new();
        builder.move_to(cx + rx, cy);
        builder.cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry);
        builder.cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy);
        builder.cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry);
        builder.cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy);
        builder.close();

        Some(builder.build())
    }
}

/// Path shape (bezier path).
#[derive(Debug, Clone)]
pub struct PathShape {
    /// Shape name.
    pub name: String,
    /// Path data.
    pub path: AnimatedProperty,
    /// Direction.
    pub direction: i32,
}

impl PathShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        Self {
            name: model.name.clone(),
            path: model
                .path
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            direction: model.direction.unwrap_or(1),
        }
    }

    /// Build a path at a specific frame.
    pub fn to_path(&self, frame: Scalar) -> Option<Path> {
        let value = self.path.value_at(frame);

        match value {
            KeyframeValue::Path(path_data) => Some(path_data_to_path(&path_data)),
            _ => None,
        }
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

/// Polystar shape (star or polygon).
#[derive(Debug, Clone)]
pub struct PolystarShape {
    /// Shape name.
    pub name: String,
    /// Position.
    pub position: AnimatedProperty,
    /// Number of points.
    pub points: AnimatedProperty,
    /// Outer radius.
    pub outer_radius: AnimatedProperty,
    /// Inner radius (for stars).
    pub inner_radius: AnimatedProperty,
    /// Outer roundness.
    pub outer_roundness: AnimatedProperty,
    /// Inner roundness (for stars).
    pub inner_roundness: AnimatedProperty,
    /// Star type (1=star, 2=polygon).
    pub star_type: i32,
    /// Rotation.
    pub rotation: AnimatedProperty,
    /// Direction.
    pub direction: i32,
}

impl PolystarShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        Self {
            name: model.name.clone(),
            position: model
                .position
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            points: model
                .points
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            outer_radius: model
                .outer_radius
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            inner_radius: model
                .inner_radius
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            outer_roundness: AnimatedProperty::default(),
            inner_roundness: AnimatedProperty::default(),
            star_type: model.star_type.unwrap_or(1),
            rotation: model
                .roundness
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            direction: model.direction.unwrap_or(1),
        }
    }

    /// Build a path at a specific frame.
    pub fn to_path(&self, frame: Scalar) -> Option<Path> {
        let pos = self.position.value_at(frame).as_vec2().unwrap_or([0.0, 0.0]);
        let points = self.points.value_at(frame).as_scalar().unwrap_or(5.0);
        let outer_r = self.outer_radius.value_at(frame).as_scalar().unwrap_or(100.0);
        let inner_r = self.inner_radius.value_at(frame).as_scalar().unwrap_or(50.0);
        let rotation = self.rotation.value_at(frame).as_scalar().unwrap_or(0.0);

        let n = points.round() as i32;
        if n < 3 {
            return None;
        }

        let mut builder = PathBuilder::new();
        let rot_rad = (rotation - 90.0) * std::f32::consts::PI / 180.0;

        let is_star = self.star_type == 1;
        let step_count = if is_star { n * 2 } else { n };
        let angle_step = std::f32::consts::TAU / step_count as Scalar;

        for i in 0..step_count {
            let angle = rot_rad + angle_step * i as Scalar;
            let radius = if is_star && i % 2 == 1 {
                inner_r
            } else {
                outer_r
            };

            let x = pos[0] + angle.cos() * radius;
            let y = pos[1] + angle.sin() * radius;

            if i == 0 {
                builder.move_to(x, y);
            } else {
                builder.line_to(x, y);
            }
        }

        builder.close();
        Some(builder.build())
    }
}

/// Fill shape.
#[derive(Debug, Clone)]
pub struct FillShape {
    /// Shape name.
    pub name: String,
    /// Color.
    pub color: AnimatedProperty,
    /// Opacity (0-100).
    pub opacity: AnimatedProperty,
    /// Fill rule (1=non-zero, 2=even-odd).
    pub fill_rule: i32,
}

impl FillShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        Self {
            name: model.name.clone(),
            color: model
                .color
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            opacity: model
                .opacity
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_else(|| AnimatedProperty::static_value(KeyframeValue::Scalar(100.0))),
            fill_rule: 1,
        }
    }

    /// Get the color at a specific frame.
    pub fn color_at(&self, frame: Scalar) -> Color4f {
        let c = self.color.value_at(frame).as_color().unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let opacity = self.opacity.value_at(frame).as_scalar().unwrap_or(100.0) / 100.0;
        Color4f::new(c[0], c[1], c[2], c[3] * opacity)
    }
}

/// Stroke shape.
#[derive(Debug, Clone)]
pub struct StrokeShape {
    /// Shape name.
    pub name: String,
    /// Color.
    pub color: AnimatedProperty,
    /// Opacity (0-100).
    pub opacity: AnimatedProperty,
    /// Stroke width.
    pub width: AnimatedProperty,
    /// Line cap.
    pub line_cap: StrokeCap,
    /// Line join.
    pub line_join: StrokeJoin,
    /// Miter limit.
    pub miter_limit: Scalar,
    /// Dash pattern.
    pub dashes: Vec<AnimatedProperty>,
    /// Dash offset.
    pub dash_offset: AnimatedProperty,
}

impl StrokeShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        let line_cap = match model.line_cap.unwrap_or(2) {
            1 => StrokeCap::Butt,
            2 => StrokeCap::Round,
            3 => StrokeCap::Square,
            _ => StrokeCap::Round,
        };

        let line_join = match model.line_join.unwrap_or(2) {
            1 => StrokeJoin::Miter,
            2 => StrokeJoin::Round,
            3 => StrokeJoin::Bevel,
            _ => StrokeJoin::Round,
        };

        Self {
            name: model.name.clone(),
            color: model
                .color
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            opacity: model
                .opacity
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_else(|| AnimatedProperty::static_value(KeyframeValue::Scalar(100.0))),
            width: model
                .stroke_width
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_else(|| AnimatedProperty::static_value(KeyframeValue::Scalar(1.0))),
            line_cap,
            line_join,
            miter_limit: model.miter_limit.unwrap_or(4.0),
            dashes: Vec::new(),
            dash_offset: AnimatedProperty::default(),
        }
    }

    /// Get the color at a specific frame.
    pub fn color_at(&self, frame: Scalar) -> Color4f {
        let c = self.color.value_at(frame).as_color().unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let opacity = self.opacity.value_at(frame).as_scalar().unwrap_or(100.0) / 100.0;
        Color4f::new(c[0], c[1], c[2], c[3] * opacity)
    }

    /// Get the stroke width at a specific frame.
    pub fn width_at(&self, frame: Scalar) -> Scalar {
        self.width.value_at(frame).as_scalar().unwrap_or(1.0)
    }
}

/// Gradient fill shape.
#[derive(Debug, Clone)]
pub struct GradientFillShape {
    /// Shape name.
    pub name: String,
    /// Gradient type (1=linear, 2=radial).
    pub gradient_type: i32,
    /// Start point.
    pub start_point: AnimatedProperty,
    /// End point.
    pub end_point: AnimatedProperty,
    /// Gradient colors and stops.
    pub colors: AnimatedProperty,
    /// Number of colors.
    pub color_count: i32,
    /// Opacity (0-100).
    pub opacity: AnimatedProperty,
}

impl GradientFillShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        Self {
            name: model.name.clone(),
            gradient_type: model.gradient_type.unwrap_or(1),
            start_point: model
                .gradient_start
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            end_point: model
                .gradient_end
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            colors: model
                .gradient_colors
                .as_ref()
                .map(|gc| AnimatedProperty::from_lottie(&gc.colors))
                .unwrap_or_default(),
            color_count: model.gradient_colors.as_ref().map(|gc| gc.count).unwrap_or(2),
            opacity: model
                .opacity
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_else(|| AnimatedProperty::static_value(KeyframeValue::Scalar(100.0))),
        }
    }
}

/// Gradient stroke shape.
#[derive(Debug, Clone)]
pub struct GradientStrokeShape {
    /// Shape name.
    pub name: String,
    /// Gradient type (1=linear, 2=radial).
    pub gradient_type: i32,
    /// Start point.
    pub start_point: AnimatedProperty,
    /// End point.
    pub end_point: AnimatedProperty,
    /// Gradient colors and stops.
    pub colors: AnimatedProperty,
    /// Number of colors.
    pub color_count: i32,
    /// Opacity (0-100).
    pub opacity: AnimatedProperty,
    /// Stroke width.
    pub width: AnimatedProperty,
    /// Line cap.
    pub line_cap: StrokeCap,
    /// Line join.
    pub line_join: StrokeJoin,
}

impl GradientStrokeShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        Self {
            name: model.name.clone(),
            gradient_type: model.gradient_type.unwrap_or(1),
            start_point: model
                .gradient_start
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            end_point: model
                .gradient_end
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            colors: model
                .gradient_colors
                .as_ref()
                .map(|gc| AnimatedProperty::from_lottie(&gc.colors))
                .unwrap_or_default(),
            color_count: model.gradient_colors.as_ref().map(|gc| gc.count).unwrap_or(2),
            opacity: model
                .opacity
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_else(|| AnimatedProperty::static_value(KeyframeValue::Scalar(100.0))),
            width: model
                .stroke_width
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_else(|| AnimatedProperty::static_value(KeyframeValue::Scalar(1.0))),
            line_cap: StrokeCap::Round,
            line_join: StrokeJoin::Round,
        }
    }
}

/// Trim paths shape.
#[derive(Debug, Clone)]
pub struct TrimPathShape {
    /// Shape name.
    pub name: String,
    /// Start (0-100%).
    pub start: AnimatedProperty,
    /// End (0-100%).
    pub end: AnimatedProperty,
    /// Offset (degrees).
    pub offset: AnimatedProperty,
    /// Trim mode (1=simultaneously, 2=individually).
    pub mode: i32,
}

impl TrimPathShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        Self {
            name: model.name.clone(),
            start: model
                .trim_start
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            end: model
                .trim_end
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_else(|| AnimatedProperty::static_value(KeyframeValue::Scalar(100.0))),
            offset: model
                .trim_offset
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
            mode: model.trim_mode.unwrap_or(1),
        }
    }

    /// Get trim values at a specific frame.
    pub fn values_at(&self, frame: Scalar) -> (Scalar, Scalar, Scalar) {
        let start = self.start.value_at(frame).as_scalar().unwrap_or(0.0) / 100.0;
        let end = self.end.value_at(frame).as_scalar().unwrap_or(100.0) / 100.0;
        let offset = self.offset.value_at(frame).as_scalar().unwrap_or(0.0) / 360.0;
        (start, end, offset)
    }
}

/// Merge paths shape.
#[derive(Debug, Clone)]
pub struct MergePathsShape {
    /// Shape name.
    pub name: String,
    /// Merge mode (1=merge, 2=add, 3=subtract, 4=intersect, 5=exclude).
    pub mode: i32,
}

impl MergePathsShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        Self {
            name: model.name.clone(),
            mode: 1,
        }
    }
}

/// Round corners shape.
#[derive(Debug, Clone)]
pub struct RoundCornersShape {
    /// Shape name.
    pub name: String,
    /// Radius.
    pub radius: AnimatedProperty,
}

impl RoundCornersShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        Self {
            name: model.name.clone(),
            radius: model
                .roundness
                .as_ref()
                .map(AnimatedProperty::from_lottie)
                .unwrap_or_default(),
        }
    }
}

/// Repeater shape.
#[derive(Debug, Clone)]
pub struct RepeaterShape {
    /// Shape name.
    pub name: String,
    /// Number of copies.
    pub copies: AnimatedProperty,
    /// Offset.
    pub offset: AnimatedProperty,
    /// Transform for each copy.
    pub transform: Option<Transform>,
}

impl RepeaterShape {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        Self {
            name: model.name.clone(),
            copies: AnimatedProperty::static_value(KeyframeValue::Scalar(3.0)),
            offset: AnimatedProperty::default(),
            transform: None,
        }
    }
}

/// Shape transform (at end of group).
#[derive(Debug, Clone)]
pub struct ShapeTransform {
    /// Shape name.
    pub name: String,
    /// Transform.
    pub transform: Transform,
}

impl ShapeTransform {
    /// Parse from Lottie model.
    pub fn from_lottie(model: &ShapeModel) -> Self {
        // Shape transforms store data differently than layer transforms
        Self {
            name: model.name.clone(),
            transform: Transform::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rectangle_path() {
        let rect = RectangleShape {
            name: "test".to_string(),
            position: AnimatedProperty::static_value(KeyframeValue::Vec2([50.0, 50.0])),
            size: AnimatedProperty::static_value(KeyframeValue::Vec2([100.0, 80.0])),
            roundness: AnimatedProperty::static_value(KeyframeValue::Scalar(0.0)),
            direction: 1,
        };

        let path = rect.to_path(0.0).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_ellipse_path() {
        let ellipse = EllipseShape {
            name: "test".to_string(),
            position: AnimatedProperty::static_value(KeyframeValue::Vec2([100.0, 100.0])),
            size: AnimatedProperty::static_value(KeyframeValue::Vec2([50.0, 30.0])),
            direction: 1,
        };

        let path = ellipse.to_path(0.0).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn test_fill_color() {
        let fill = FillShape {
            name: "test".to_string(),
            color: AnimatedProperty::static_value(KeyframeValue::Color([1.0, 0.0, 0.0, 1.0])),
            opacity: AnimatedProperty::static_value(KeyframeValue::Scalar(50.0)),
            fill_rule: 1,
        };

        let color = fill.color_at(0.0);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 0.5); // 50% opacity
    }
}
