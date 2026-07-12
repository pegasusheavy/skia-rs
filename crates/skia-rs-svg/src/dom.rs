//! SVG DOM representation.

use crate::css::Stylesheet;
use skia_rs_core::{Color, Matrix, Point, Rect, Scalar};
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
    /// `preserveAspectRatio` on the root `<svg>` (default `xMidYMid meet`).
    pub preserve_aspect_ratio: PreserveAspectRatio,
    /// Merged stylesheet extracted from `<style>` elements during parsing.
    ///
    /// Populated by `parse_svg`. Applied automatically by `render_svg` so
    /// that CSS-styled nodes inherit their declared properties. Empty when
    /// the source document contains no `<style>` blocks.
    pub stylesheet: Stylesheet,
}

impl SvgDom {
    /// Create a new empty SVG DOM.
    #[must_use] 
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the intrinsic size.
    pub const fn intrinsic_size(&self) -> (Scalar, Scalar) {
        (self.width, self.height)
    }

    /// Get the view box or calculate from size.
    pub fn get_view_box(&self) -> Rect {
        self.view_box
            .unwrap_or_else(|| Rect::from_xywh(0.0, 0.0, self.width, self.height))
    }
}

/// Horizontal alignment component of `preserveAspectRatio`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignX {
    /// `xMin` — align the min edge (coefficient 0.0).
    Min,
    /// `xMid` — center (coefficient 0.5).
    #[default]
    Mid,
    /// `xMax` — align the max edge (coefficient 1.0).
    Max,
}

/// Vertical alignment component of `preserveAspectRatio`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignY {
    /// `YMin` — align the min edge (coefficient 0.0).
    Min,
    /// `YMid` — center (coefficient 0.5).
    #[default]
    Mid,
    /// `YMax` — align the max edge (coefficient 1.0).
    Max,
}

/// The meet-or-slice scaling behaviour of `preserveAspectRatio`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeetOrSlice {
    /// Uniform scale that fits the whole viewBox inside the viewport
    /// (`min(sx, sy)`).
    #[default]
    Meet,
    /// Uniform scale that covers the whole viewport (`max(sx, sy)`).
    Slice,
}

/// `preserveAspectRatio` value (SVG 1.1 §7.8, `SkSVGPreserveAspectRatio`).
///
/// `align == None` selects anisotropic (non-uniform) scaling that stretches
/// the viewBox to exactly fill the viewport; otherwise scaling is isotropic
/// and `align`/`meet_or_slice` position the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreserveAspectRatio {
    /// Alignment; `None` means `preserveAspectRatio="none"`.
    pub align: Option<(AlignX, AlignY)>,
    /// Meet or slice.
    pub meet_or_slice: MeetOrSlice,
}

impl Default for PreserveAspectRatio {
    /// The initial value is `xMidYMid meet` (SVG 1.1 §7.8).
    fn default() -> Self {
        Self {
            align: Some((AlignX::Mid, AlignY::Mid)),
            meet_or_slice: MeetOrSlice::Meet,
        }
    }
}

/// SVG node types.
#[derive(Debug, Clone)]
#[derive(Default)]
pub enum SvgNodeKind {
    /// Root SVG element.
    Svg,
    /// Group element.
    #[default]
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


/// SVG node (element in the DOM tree).
///
/// Presentation properties that participate in inheritance (`fill`,
/// `stroke`, `stroke_width`, `color`, `fill_opacity`, `stroke_opacity`) are
/// stored as `Option`: `None` means "not specified on this element, inherit
/// from the parent presentation context". Only the root defaults are
/// materialised (per `SkSVGPresentationAttributes::MakeInitial`) — see
/// `render::PresentationState`. `opacity` is *not* inherited (it composites
/// the element/group as a unit) so it stays a plain `Scalar` defaulting to
/// `1.0`.
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
    /// Fill paint (`None` = inherit).
    pub fill: Option<SvgPaint>,
    /// Stroke paint (`None` = inherit).
    pub stroke: Option<SvgPaint>,
    /// Stroke width (`None` = inherit).
    pub stroke_width: Option<Scalar>,
    /// `color` property, the source of `currentColor` (`None` = inherit).
    pub color: Option<Color>,
    /// Fill opacity (`None` = inherit).
    pub fill_opacity: Option<Scalar>,
    /// Stroke opacity (`None` = inherit).
    pub stroke_opacity: Option<Scalar>,
    /// Element/group opacity (not inherited; default `1.0`).
    pub opacity: Scalar,
    /// Visibility.
    pub visible: bool,
    /// Child nodes.
    pub children: Vec<Self>,
    /// Custom attributes.
    pub attributes: HashMap<String, String>,
}

impl SvgNode {
    /// Create a new SVG node.
    ///
    /// All inherited presentation properties start unset (`None`) so that
    /// the render walk can resolve them against the parent presentation
    /// context. The `fill: black` initial value lives at the root of that
    /// context, not on every node.
    pub fn new(kind: SvgNodeKind) -> Self {
        Self {
            kind,
            id: None,
            classes: Vec::new(),
            transform: Matrix::IDENTITY,
            fill: None,
            stroke: None,
            stroke_width: None,
            color: None,
            fill_opacity: None,
            stroke_opacity: None,
            opacity: 1.0,
            visible: true,
            children: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    /// Add a child node.
    pub fn add_child(&mut self, child: Self) {
        self.children.push(child);
    }

    /// Find a node by ID.
    pub fn find_by_id(&self, id: &str) -> Option<&Self> {
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
            SvgNodeKind::Circle(c) => Rect::from_xywh(c.cx - c.r, c.cy - c.r, c.r * 2.0, c.r * 2.0),
            SvgNodeKind::Ellipse(e) => {
                Rect::from_xywh(e.cx - e.rx, e.cy - e.ry, e.rx * 2.0, e.ry * 2.0)
            }
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
    /// The `currentColor` keyword — resolves to the inherited `color`
    /// property at paint-build time (`SkSVGColor::Type::kCurrentColor`).
    CurrentColor,
    /// Reference to a gradient or pattern, with an optional fallback color
    /// used when the reference cannot be resolved (`url(#id) <color>`).
    Url(String, Option<Color>),
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
