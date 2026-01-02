//! SVG DOM representation.

use skia_rs_core::{Color, Matrix, Point, Rect, Scalar};
use skia_rs_paint::Paint;
use skia_rs_path::Path;
use std::collections::HashMap;

/// SVG document.
#[derive(Debug, Clone, Default)]
pub struct SvgDom {
    /// Root element.
    pub root: SvgNode,
    /// Width.
    pub width: Scalar,
    /// Height.
    pub height: Scalar,
    /// View box.
    pub view_box: Option<Rect>,
}

impl SvgDom {
    /// Create a new empty SVG DOM.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the intrinsic size.
    pub fn intrinsic_size(&self) -> (Scalar, Scalar) {
        (self.width, self.height)
    }

    /// Get the view box or calculate from size.
    pub fn get_view_box(&self) -> Rect {
        self.view_box
            .unwrap_or_else(|| Rect::from_xywh(0.0, 0.0, self.width, self.height))
    }
}

/// SVG node types.
#[derive(Debug, Clone)]
pub enum SvgNodeKind {
    /// Root SVG element.
    Svg,
    /// Group element.
    Group,
    /// Rectangle.
    Rect(SvgRect),
    /// Circle.
    Circle(SvgCircle),
    /// Ellipse.
    Ellipse(SvgEllipse),
    /// Line.
    Line(SvgLine),
    /// Polyline.
    Polyline(Vec<Point>),
    /// Polygon.
    Polygon(Vec<Point>),
    /// Path.
    Path(Path),
    /// Text.
    Text(SvgText),
    /// Image.
    Image(SvgImage),
    /// Use (reference to another element).
    Use(String),
    /// Definitions.
    Defs,
    /// Linear gradient.
    LinearGradient(SvgLinearGradient),
    /// Radial gradient.
    RadialGradient(SvgRadialGradient),
    /// Clip path.
    ClipPath(String),
    /// Unknown element.
    Unknown(String),
}

impl Default for SvgNodeKind {
    fn default() -> Self {
        Self::Group
    }
}

/// SVG node (element in the DOM tree).
#[derive(Debug, Clone, Default)]
pub struct SvgNode {
    /// Node kind.
    pub kind: SvgNodeKind,
    /// Element ID.
    pub id: Option<String>,
    /// CSS classes.
    pub classes: Vec<String>,
    /// Transform matrix.
    pub transform: Matrix,
    /// Fill paint.
    pub fill: Option<SvgPaint>,
    /// Stroke paint.
    pub stroke: Option<SvgPaint>,
    /// Stroke width.
    pub stroke_width: Scalar,
    /// Opacity.
    pub opacity: Scalar,
    /// Visibility.
    pub visible: bool,
    /// Child nodes.
    pub children: Vec<SvgNode>,
    /// Custom attributes.
    pub attributes: HashMap<String, String>,
}

impl SvgNode {
    /// Create a new SVG node.
    pub fn new(kind: SvgNodeKind) -> Self {
        Self {
            kind,
            id: None,
            classes: Vec::new(),
            transform: Matrix::IDENTITY,
            fill: Some(SvgPaint::Color(Color::BLACK)),
            stroke: None,
            stroke_width: 1.0,
            opacity: 1.0,
            visible: true,
            children: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    /// Add a child node.
    pub fn add_child(&mut self, child: SvgNode) {
        self.children.push(child);
    }

    /// Find a node by ID.
    pub fn find_by_id(&self, id: &str) -> Option<&SvgNode> {
        if self.id.as_deref() == Some(id) {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_id(id) {
                return Some(found);
            }
        }
        None
    }

    /// Get the bounds of this node.
    pub fn bounds(&self) -> Rect {
        match &self.kind {
            SvgNodeKind::Rect(r) => Rect::from_xywh(r.x, r.y, r.width, r.height),
            SvgNodeKind::Circle(c) => Rect::from_xywh(
                c.cx - c.r,
                c.cy - c.r,
                c.r * 2.0,
                c.r * 2.0,
            ),
            SvgNodeKind::Ellipse(e) => Rect::from_xywh(
                e.cx - e.rx,
                e.cy - e.ry,
                e.rx * 2.0,
                e.ry * 2.0,
            ),
            SvgNodeKind::Line(l) => Rect::new(
                l.x1.min(l.x2),
                l.y1.min(l.y2),
                l.x1.max(l.x2),
                l.y1.max(l.y2),
            ),
            SvgNodeKind::Path(p) => p.bounds(),
            _ => {
                // Calculate from children
                let mut bounds = Rect::EMPTY;
                for child in &self.children {
                    let child_bounds = child.bounds();
                    bounds = bounds.join(&child_bounds);
                }
                bounds
            }
        }
    }
}

/// SVG paint (fill or stroke).
#[derive(Debug, Clone)]
pub enum SvgPaint {
    /// Solid color.
    Color(Color),
    /// Reference to gradient or pattern.
    Url(String),
    /// No paint.
    None,
}

/// SVG rectangle.
#[derive(Debug, Clone, Copy, Default)]
pub struct SvgRect {
    /// X position.
    pub x: Scalar,
    /// Y position.
    pub y: Scalar,
    /// Width.
    pub width: Scalar,
    /// Height.
    pub height: Scalar,
    /// Horizontal corner radius.
    pub rx: Scalar,
    /// Vertical corner radius.
    pub ry: Scalar,
}

/// SVG circle.
#[derive(Debug, Clone, Copy, Default)]
pub struct SvgCircle {
    /// Center X.
    pub cx: Scalar,
    /// Center Y.
    pub cy: Scalar,
    /// Radius.
    pub r: Scalar,
}

/// SVG ellipse.
#[derive(Debug, Clone, Copy, Default)]
pub struct SvgEllipse {
    /// Center X.
    pub cx: Scalar,
    /// Center Y.
    pub cy: Scalar,
    /// Horizontal radius.
    pub rx: Scalar,
    /// Vertical radius.
    pub ry: Scalar,
}

/// SVG line.
#[derive(Debug, Clone, Copy, Default)]
pub struct SvgLine {
    /// Start X.
    pub x1: Scalar,
    /// Start Y.
    pub y1: Scalar,
    /// End X.
    pub x2: Scalar,
    /// End Y.
    pub y2: Scalar,
}

/// SVG text.
#[derive(Debug, Clone, Default)]
pub struct SvgText {
    /// X position.
    pub x: Scalar,
    /// Y position.
    pub y: Scalar,
    /// Text content.
    pub content: String,
    /// Font family.
    pub font_family: Option<String>,
    /// Font size.
    pub font_size: Scalar,
    /// Font weight.
    pub font_weight: u16,
    /// Text anchor.
    pub text_anchor: TextAnchor,
}

/// Text anchor alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAnchor {
    /// Start alignment.
    #[default]
    Start,
    /// Middle alignment.
    Middle,
    /// End alignment.
    End,
}

/// SVG image.
#[derive(Debug, Clone, Default)]
pub struct SvgImage {
    /// X position.
    pub x: Scalar,
    /// Y position.
    pub y: Scalar,
    /// Width.
    pub width: Scalar,
    /// Height.
    pub height: Scalar,
    /// Image href (data URI or URL).
    pub href: String,
}

/// SVG linear gradient.
#[derive(Debug, Clone, Default)]
pub struct SvgLinearGradient {
    /// Start X.
    pub x1: Scalar,
    /// Start Y.
    pub y1: Scalar,
    /// End X.
    pub x2: Scalar,
    /// End Y.
    pub y2: Scalar,
    /// Gradient stops.
    pub stops: Vec<GradientStop>,
    /// Spread method.
    pub spread: SpreadMethod,
    /// Gradient units.
    pub units: GradientUnits,
    /// Transform.
    pub transform: Matrix,
}

/// SVG radial gradient.
#[derive(Debug, Clone, Default)]
pub struct SvgRadialGradient {
    /// Center X.
    pub cx: Scalar,
    /// Center Y.
    pub cy: Scalar,
    /// Radius.
    pub r: Scalar,
    /// Focus X.
    pub fx: Scalar,
    /// Focus Y.
    pub fy: Scalar,
    /// Gradient stops.
    pub stops: Vec<GradientStop>,
    /// Spread method.
    pub spread: SpreadMethod,
    /// Gradient units.
    pub units: GradientUnits,
    /// Transform.
    pub transform: Matrix,
}

/// Gradient stop.
#[derive(Debug, Clone, Copy, Default)]
pub struct GradientStop {
    /// Offset (0.0 to 1.0).
    pub offset: Scalar,
    /// Color.
    pub color: Color,
    /// Opacity.
    pub opacity: Scalar,
}

/// Gradient spread method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpreadMethod {
    /// Pad (extend colors).
    #[default]
    Pad,
    /// Reflect.
    Reflect,
    /// Repeat.
    Repeat,
}

/// Gradient units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradientUnits {
    /// User space coordinates.
    UserSpaceOnUse,
    /// Object bounding box.
    #[default]
    ObjectBoundingBox,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_dom() {
        let mut dom = SvgDom::new();
        dom.width = 100.0;
        dom.height = 100.0;

        assert_eq!(dom.intrinsic_size(), (100.0, 100.0));
    }

    #[test]
    fn test_svg_node() {
        let mut group = SvgNode::new(SvgNodeKind::Group);
        group.id = Some("group1".to_string());

        let rect = SvgNode::new(SvgNodeKind::Rect(SvgRect {
            x: 10.0,
            y: 10.0,
            width: 50.0,
            height: 50.0,
            rx: 0.0,
            ry: 0.0,
        }));
        group.add_child(rect);

        assert_eq!(group.children.len(), 1);
        assert!(group.find_by_id("group1").is_some());
    }
}
