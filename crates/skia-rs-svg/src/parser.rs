//! SVG parsing.

use crate::dom::*;
use skia_rs_core::{Color, Matrix, Point, Rect, Scalar};
use skia_rs_path::{parse_svg_path, PathBuilder};
use std::collections::HashMap;
use thiserror::Error;

/// SVG parsing error.
#[derive(Debug, Error)]
pub enum SvgError {
    /// XML parsing error.
    #[error("XML parsing error: {0}")]
    XmlError(String),
    /// Invalid attribute.
    #[error("Invalid attribute {0}: {1}")]
    InvalidAttribute(String, String),
    /// Unsupported feature.
    #[error("Unsupported SVG feature: {0}")]
    Unsupported(String),
}

/// Parse an SVG document from a string.
pub fn parse_svg(svg: &str) -> Result<SvgDom, SvgError> {
    let mut dom = SvgDom::new();

    // Simple state-machine parser for basic SVG
    // A full implementation would use roxmltree
    let mut in_element = false;
    let mut current_tag = String::new();
    let mut attributes = HashMap::new();
    let mut node_stack: Vec<SvgNode> = vec![SvgNode::new(SvgNodeKind::Svg)];

    let mut chars = svg.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            if chars.peek() == Some(&'/') {
                // Closing tag
                chars.next(); // Skip '/'
                let mut tag = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch == '>' {
                        chars.next();
                        break;
                    }
                    tag.push(chars.next().unwrap());
                }

                // Pop node from stack
                if node_stack.len() > 1 {
                    let node = node_stack.pop().unwrap();
                    if let Some(parent) = node_stack.last_mut() {
                        parent.add_child(node);
                    }
                }
            } else if chars.peek() == Some(&'!') {
                // Comment or DOCTYPE, skip
                while let Some(ch) = chars.next() {
                    if ch == '>' {
                        break;
                    }
                }
            } else if chars.peek() == Some(&'?') {
                // Processing instruction, skip
                while let Some(ch) = chars.next() {
                    if ch == '>' {
                        break;
                    }
                }
            } else {
                // Opening tag
                in_element = true;
                current_tag.clear();
                attributes.clear();

                // Read tag name
                while let Some(&ch) = chars.peek() {
                    if ch.is_whitespace() || ch == '>' || ch == '/' {
                        break;
                    }
                    current_tag.push(chars.next().unwrap());
                }

                // Read attributes
                while let Some(&ch) = chars.peek() {
                    if ch == '>' || ch == '/' {
                        break;
                    }
                    if ch.is_whitespace() {
                        chars.next();
                        continue;
                    }

                    // Read attribute name
                    let mut attr_name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch == '=' || ch.is_whitespace() || ch == '>' || ch == '/' {
                            break;
                        }
                        attr_name.push(chars.next().unwrap());
                    }

                    // Skip whitespace and =
                    while let Some(&ch) = chars.peek() {
                        if ch == '"' || ch == '\'' {
                            break;
                        }
                        chars.next();
                    }

                    // Read attribute value
                    let quote = chars.next(); // Opening quote
                    let mut attr_value = String::new();
                    if let Some(q) = quote {
                        while let Some(&ch) = chars.peek() {
                            if ch == q {
                                chars.next();
                                break;
                            }
                            attr_value.push(chars.next().unwrap());
                        }
                    }

                    if !attr_name.is_empty() {
                        attributes.insert(attr_name, attr_value);
                    }
                }

                // Check for self-closing tag
                let mut self_closing = false;
                if chars.peek() == Some(&'/') {
                    chars.next();
                    self_closing = true;
                }
                if chars.peek() == Some(&'>') {
                    chars.next();
                }

                // Create node
                let node = create_node(&current_tag, &attributes, &mut dom)?;

                if self_closing {
                    if let Some(parent) = node_stack.last_mut() {
                        parent.add_child(node);
                    }
                } else {
                    node_stack.push(node);
                }

                in_element = false;
            }
        }
    }

    // Finish
    while node_stack.len() > 1 {
        let node = node_stack.pop().unwrap();
        if let Some(parent) = node_stack.last_mut() {
            parent.add_child(node);
        }
    }

    dom.root = node_stack.pop().unwrap_or_default();
    Ok(dom)
}

/// Create an SVG node from tag name and attributes.
fn create_node(
    tag: &str,
    attrs: &HashMap<String, String>,
    dom: &mut SvgDom,
) -> Result<SvgNode, SvgError> {
    let mut node = match tag {
        "svg" => {
            dom.width = parse_length(attrs.get("width").map(|s| s.as_str()).unwrap_or("100"));
            dom.height = parse_length(attrs.get("height").map(|s| s.as_str()).unwrap_or("100"));

            if let Some(vb) = attrs.get("viewBox") {
                dom.view_box = parse_viewbox(vb);
            }

            SvgNode::new(SvgNodeKind::Svg)
        }
        "g" => SvgNode::new(SvgNodeKind::Group),
        "rect" => {
            let rect = SvgRect {
                x: parse_length(attrs.get("x").map(|s| s.as_str()).unwrap_or("0")),
                y: parse_length(attrs.get("y").map(|s| s.as_str()).unwrap_or("0")),
                width: parse_length(attrs.get("width").map(|s| s.as_str()).unwrap_or("0")),
                height: parse_length(attrs.get("height").map(|s| s.as_str()).unwrap_or("0")),
                rx: parse_length(attrs.get("rx").map(|s| s.as_str()).unwrap_or("0")),
                ry: parse_length(attrs.get("ry").map(|s| s.as_str()).unwrap_or("0")),
            };
            SvgNode::new(SvgNodeKind::Rect(rect))
        }
        "circle" => {
            let circle = SvgCircle {
                cx: parse_length(attrs.get("cx").map(|s| s.as_str()).unwrap_or("0")),
                cy: parse_length(attrs.get("cy").map(|s| s.as_str()).unwrap_or("0")),
                r: parse_length(attrs.get("r").map(|s| s.as_str()).unwrap_or("0")),
            };
            SvgNode::new(SvgNodeKind::Circle(circle))
        }
        "ellipse" => {
            let ellipse = SvgEllipse {
                cx: parse_length(attrs.get("cx").map(|s| s.as_str()).unwrap_or("0")),
                cy: parse_length(attrs.get("cy").map(|s| s.as_str()).unwrap_or("0")),
                rx: parse_length(attrs.get("rx").map(|s| s.as_str()).unwrap_or("0")),
                ry: parse_length(attrs.get("ry").map(|s| s.as_str()).unwrap_or("0")),
            };
            SvgNode::new(SvgNodeKind::Ellipse(ellipse))
        }
        "line" => {
            let line = SvgLine {
                x1: parse_length(attrs.get("x1").map(|s| s.as_str()).unwrap_or("0")),
                y1: parse_length(attrs.get("y1").map(|s| s.as_str()).unwrap_or("0")),
                x2: parse_length(attrs.get("x2").map(|s| s.as_str()).unwrap_or("0")),
                y2: parse_length(attrs.get("y2").map(|s| s.as_str()).unwrap_or("0")),
            };
            SvgNode::new(SvgNodeKind::Line(line))
        }
        "polyline" => {
            let points = parse_points(attrs.get("points").map(|s| s.as_str()).unwrap_or(""));
            SvgNode::new(SvgNodeKind::Polyline(points))
        }
        "polygon" => {
            let points = parse_points(attrs.get("points").map(|s| s.as_str()).unwrap_or(""));
            SvgNode::new(SvgNodeKind::Polygon(points))
        }
        "path" => {
            let d = attrs.get("d").map(|s| s.as_str()).unwrap_or("");
            let path = parse_svg_path(d).unwrap_or_default();
            SvgNode::new(SvgNodeKind::Path(path))
        }
        "text" => {
            let text = SvgText {
                x: parse_length(attrs.get("x").map(|s| s.as_str()).unwrap_or("0")),
                y: parse_length(attrs.get("y").map(|s| s.as_str()).unwrap_or("0")),
                content: String::new(), // Will be filled with text content
                font_family: attrs.get("font-family").cloned(),
                font_size: parse_length(attrs.get("font-size").map(|s| s.as_str()).unwrap_or("12")),
                font_weight: attrs
                    .get("font-weight")
                    .and_then(|w| w.parse().ok())
                    .unwrap_or(400),
                text_anchor: match attrs.get("text-anchor").map(|s| s.as_str()) {
                    Some("middle") => TextAnchor::Middle,
                    Some("end") => TextAnchor::End,
                    _ => TextAnchor::Start,
                },
            };
            SvgNode::new(SvgNodeKind::Text(text))
        }
        "defs" => SvgNode::new(SvgNodeKind::Defs),
        "linearGradient" => {
            let gradient = SvgLinearGradient {
                x1: parse_length(attrs.get("x1").map(|s| s.as_str()).unwrap_or("0")),
                y1: parse_length(attrs.get("y1").map(|s| s.as_str()).unwrap_or("0")),
                x2: parse_length(attrs.get("x2").map(|s| s.as_str()).unwrap_or("100%")),
                y2: parse_length(attrs.get("y2").map(|s| s.as_str()).unwrap_or("0")),
                stops: Vec::new(),
                spread: SpreadMethod::Pad,
                units: GradientUnits::ObjectBoundingBox,
                transform: Matrix::IDENTITY,
            };
            SvgNode::new(SvgNodeKind::LinearGradient(gradient))
        }
        "radialGradient" => {
            let cx = parse_length(attrs.get("cx").map(|s| s.as_str()).unwrap_or("50%"));
            let cy = parse_length(attrs.get("cy").map(|s| s.as_str()).unwrap_or("50%"));
            let gradient = SvgRadialGradient {
                cx,
                cy,
                r: parse_length(attrs.get("r").map(|s| s.as_str()).unwrap_or("50%")),
                fx: parse_length(attrs.get("fx").map(|s| s.as_str()).unwrap_or(&cx.to_string())),
                fy: parse_length(attrs.get("fy").map(|s| s.as_str()).unwrap_or(&cy.to_string())),
                stops: Vec::new(),
                spread: SpreadMethod::Pad,
                units: GradientUnits::ObjectBoundingBox,
                transform: Matrix::IDENTITY,
            };
            SvgNode::new(SvgNodeKind::RadialGradient(gradient))
        }
        "use" => {
            let href = attrs
                .get("href")
                .or_else(|| attrs.get("xlink:href"))
                .cloned()
                .unwrap_or_default();
            SvgNode::new(SvgNodeKind::Use(href))
        }
        _ => SvgNode::new(SvgNodeKind::Unknown(tag.to_string())),
    };

    // Parse common attributes
    node.id = attrs.get("id").cloned();

    if let Some(class) = attrs.get("class") {
        node.classes = class.split_whitespace().map(|s| s.to_string()).collect();
    }

    if let Some(transform) = attrs.get("transform") {
        node.transform = parse_transform(transform);
    }

    if let Some(fill) = attrs.get("fill") {
        node.fill = parse_paint(fill);
    }

    if let Some(stroke) = attrs.get("stroke") {
        node.stroke = parse_paint(stroke);
    }

    if let Some(sw) = attrs.get("stroke-width") {
        node.stroke_width = parse_length(sw);
    }

    if let Some(opacity) = attrs.get("opacity") {
        node.opacity = opacity.parse().unwrap_or(1.0);
    }

    if let Some(visibility) = attrs.get("visibility") {
        node.visible = visibility != "hidden";
    }

    if let Some(display) = attrs.get("display") {
        if display == "none" {
            node.visible = false;
        }
    }

    Ok(node)
}

/// Parse an SVG length value.
fn parse_length(s: &str) -> Scalar {
    let s = s.trim();
    if s.ends_with('%') {
        // Percentage - return as fraction (will need context to resolve)
        s[..s.len() - 1].parse::<Scalar>().unwrap_or(0.0) / 100.0
    } else if s.ends_with("px") {
        s[..s.len() - 2].parse().unwrap_or(0.0)
    } else if s.ends_with("pt") {
        s[..s.len() - 2].parse::<Scalar>().unwrap_or(0.0) * 1.333
    } else if s.ends_with("em") {
        s[..s.len() - 2].parse::<Scalar>().unwrap_or(0.0) * 16.0
    } else {
        s.parse().unwrap_or(0.0)
    }
}

/// Parse a viewBox attribute.
fn parse_viewbox(s: &str) -> Option<Rect> {
    let parts: Vec<Scalar> = s
        .split_whitespace()
        .flat_map(|p| p.split(','))
        .filter_map(|p| p.parse().ok())
        .collect();

    if parts.len() == 4 {
        Some(Rect::from_xywh(parts[0], parts[1], parts[2], parts[3]))
    } else {
        None
    }
}

/// Parse points attribute (for polyline/polygon).
fn parse_points(s: &str) -> Vec<Point> {
    let nums: Vec<Scalar> = s
        .split_whitespace()
        .flat_map(|p| p.split(','))
        .filter_map(|p| p.parse().ok())
        .collect();

    nums.chunks(2)
        .filter_map(|c| {
            if c.len() == 2 {
                Some(Point::new(c[0], c[1]))
            } else {
                None
            }
        })
        .collect()
}

/// Parse a paint value.
fn parse_paint(s: &str) -> Option<SvgPaint> {
    let s = s.trim();
    if s == "none" {
        Some(SvgPaint::None)
    } else if s.starts_with("url(") {
        let url = s[4..].trim_end_matches(')').trim_matches(|c| c == '"' || c == '\'');
        Some(SvgPaint::Url(url.to_string()))
    } else {
        parse_color(s).map(SvgPaint::Color)
    }
}

/// Parse a color value.
fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();

    if s.starts_with('#') {
        // Hex color
        let hex = &s[1..];
        let (r, g, b) = if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            (r, g, b)
        } else if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b)
        } else {
            return None;
        };
        Some(Color::from_rgb(r, g, b))
    } else if s.starts_with("rgb(") {
        let inner = s[4..].trim_end_matches(')');
        let parts: Vec<u8> = inner
            .split(',')
            .filter_map(|p| p.trim().parse().ok())
            .collect();
        if parts.len() == 3 {
            Some(Color::from_rgb(parts[0], parts[1], parts[2]))
        } else {
            None
        }
    } else if s.starts_with("rgba(") {
        let inner = s[5..].trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() == 4 {
            let r: u8 = parts[0].parse().ok()?;
            let g: u8 = parts[1].parse().ok()?;
            let b: u8 = parts[2].parse().ok()?;
            let a: f32 = parts[3].parse().ok()?;
            Some(Color::from_argb((a * 255.0) as u8, r, g, b))
        } else {
            None
        }
    } else {
        // Named colors
        match s.to_lowercase().as_str() {
            "black" => Some(Color::BLACK),
            "white" => Some(Color::WHITE),
            "red" => Some(Color::from_rgb(255, 0, 0)),
            "green" => Some(Color::from_rgb(0, 128, 0)),
            "blue" => Some(Color::from_rgb(0, 0, 255)),
            "yellow" => Some(Color::from_rgb(255, 255, 0)),
            "cyan" | "aqua" => Some(Color::from_rgb(0, 255, 255)),
            "magenta" | "fuchsia" => Some(Color::from_rgb(255, 0, 255)),
            "gray" | "grey" => Some(Color::from_rgb(128, 128, 128)),
            "silver" => Some(Color::from_rgb(192, 192, 192)),
            "orange" => Some(Color::from_rgb(255, 165, 0)),
            "purple" => Some(Color::from_rgb(128, 0, 128)),
            "pink" => Some(Color::from_rgb(255, 192, 203)),
            "brown" => Some(Color::from_rgb(165, 42, 42)),
            "transparent" => Some(Color::TRANSPARENT),
            _ => None,
        }
    }
}

/// Parse a transform attribute.
fn parse_transform(s: &str) -> Matrix {
    let mut result = Matrix::IDENTITY;
    let s = s.trim();

    // Simple parser for common transforms
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_alphabetic() {
            // Read function name
            let mut name = String::from(c);
            while let Some(&ch) = chars.peek() {
                if ch == '(' {
                    break;
                }
                name.push(chars.next().unwrap());
            }

            // Skip '('
            if chars.peek() == Some(&'(') {
                chars.next();
            }

            // Read arguments
            let mut args = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == ')' {
                    chars.next();
                    break;
                }
                args.push(chars.next().unwrap());
            }

            let nums: Vec<Scalar> = args
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter_map(|p| p.parse().ok())
                .collect();

            let transform = match name.as_str() {
                "translate" => {
                    let dx = nums.first().copied().unwrap_or(0.0);
                    let dy = nums.get(1).copied().unwrap_or(0.0);
                    Matrix::translate(dx, dy)
                }
                "scale" => {
                    let sx = nums.first().copied().unwrap_or(1.0);
                    let sy = nums.get(1).copied().unwrap_or(sx);
                    Matrix::scale(sx, sy)
                }
                "rotate" => {
                    let angle = nums.first().copied().unwrap_or(0.0);
                    let radians = angle * std::f32::consts::PI / 180.0;
                    if nums.len() >= 3 {
                        let cx = nums[1];
                        let cy = nums[2];
                        Matrix::translate(cx, cy)
                            .concat(&Matrix::rotate(radians))
                            .concat(&Matrix::translate(-cx, -cy))
                    } else {
                        Matrix::rotate(radians)
                    }
                }
                "skewX" => {
                    let angle = nums.first().copied().unwrap_or(0.0);
                    Matrix::skew(angle.to_radians().tan(), 0.0)
                }
                "skewY" => {
                    let angle = nums.first().copied().unwrap_or(0.0);
                    Matrix::skew(0.0, angle.to_radians().tan())
                }
                "matrix" if nums.len() >= 6 => Matrix {
                    values: [
                        nums[0], nums[2], nums[4], nums[1], nums[3], nums[5], 0.0, 0.0, 1.0,
                    ],
                },
                _ => Matrix::IDENTITY,
            };

            result = result.concat(&transform);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_length() {
        assert_eq!(parse_length("100"), 100.0);
        assert_eq!(parse_length("50px"), 50.0);
        assert!((parse_length("50%") - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("#ff0000"), Some(Color::from_rgb(255, 0, 0)));
        assert_eq!(parse_color("#f00"), Some(Color::from_rgb(255, 0, 0)));
        assert_eq!(parse_color("rgb(255, 0, 0)"), Some(Color::from_rgb(255, 0, 0)));
        assert_eq!(parse_color("red"), Some(Color::from_rgb(255, 0, 0)));
    }

    #[test]
    fn test_parse_simple_svg() {
        let svg = r#"<svg width="100" height="100">
            <rect x="10" y="10" width="80" height="80" fill="red"/>
        </svg>"#;

        let dom = parse_svg(svg).unwrap();
        assert_eq!(dom.width, 100.0);
        assert_eq!(dom.height, 100.0);
    }

    #[test]
    fn test_parse_transform() {
        let m = parse_transform("translate(10, 20)");
        assert!((m.values[2] - 10.0).abs() < 0.01);
        assert!((m.values[5] - 20.0).abs() < 0.01);

        let m = parse_transform("scale(2)");
        assert!((m.values[0] - 2.0).abs() < 0.01);
        assert!((m.values[4] - 2.0).abs() < 0.01);
    }
}
