//! SVG export functionality.
//!
//! This module provides functionality to convert an `SvgDom` back to SVG markup,
//! enabling round-trip editing and programmatic SVG generation.

use crate::dom::*;
use skia_rs_core::{Color, Matrix, Scalar};
use std::fmt::Write;

/// Options for SVG export.
#[derive(Debug, Clone)]
pub struct SvgExportOptions {
    /// Indent string (default: 2 spaces).
    pub indent: String,
    /// Include XML declaration.
    pub xml_declaration: bool,
    /// Pretty print with indentation.
    pub pretty_print: bool,
    /// Precision for floating point numbers.
    pub precision: usize,
    /// Include default attributes.
    pub include_defaults: bool,
}

impl Default for SvgExportOptions {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            xml_declaration: true,
            pretty_print: true,
            precision: 3,
            include_defaults: false,
        }
    }
}

impl SvgExportOptions {
    /// Create options for minified output.
    pub fn minified() -> Self {
        Self {
            indent: String::new(),
            xml_declaration: false,
            pretty_print: false,
            precision: 2,
            include_defaults: false,
        }
    }
}

/// Export an SVG DOM to a string.
pub fn export_svg(dom: &SvgDom) -> String {
    export_svg_with_options(dom, &SvgExportOptions::default())
}

/// Export an SVG DOM to a string with custom options.
pub fn export_svg_with_options(dom: &SvgDom, options: &SvgExportOptions) -> String {
    let mut output = String::new();

    if options.xml_declaration {
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    }

    // Start SVG element
    write!(
        output,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"",
        format_scalar(dom.width, options.precision),
        format_scalar(dom.height, options.precision)
    )
    .unwrap();

    if let Some(vb) = &dom.view_box {
        write!(
            output,
            " viewBox=\"{} {} {} {}\"",
            format_scalar(vb.left, options.precision),
            format_scalar(vb.top, options.precision),
            format_scalar(vb.width(), options.precision),
            format_scalar(vb.height(), options.precision)
        )
        .unwrap();
    }

    output.push('>');

    if options.pretty_print {
        output.push('\n');
    }

    // Export children
    for child in &dom.root.children {
        export_node(&mut output, child, options, 1);
    }

    output.push_str("</svg>");

    if options.pretty_print {
        output.push('\n');
    }

    output
}

fn export_node(output: &mut String, node: &SvgNode, options: &SvgExportOptions, depth: usize) {
    if !node.visible && !options.include_defaults {
        return;
    }

    let indent = if options.pretty_print {
        options.indent.repeat(depth)
    } else {
        String::new()
    };

    let newline = if options.pretty_print { "\n" } else { "" };

    match &node.kind {
        SvgNodeKind::Svg => {
            // Already handled at top level
            for child in &node.children {
                export_node(output, child, options, depth);
            }
        }
        SvgNodeKind::Group => {
            output.push_str(&indent);
            output.push_str("<g");
            export_common_attrs(output, node, options);

            if node.children.is_empty() {
                output.push_str("/>");
            } else {
                output.push('>');
                output.push_str(newline);

                for child in &node.children {
                    export_node(output, child, options, depth + 1);
                }

                output.push_str(&indent);
                output.push_str("</g>");
            }
            output.push_str(newline);
        }
        SvgNodeKind::Rect(rect) => {
            output.push_str(&indent);
            output.push_str("<rect");

            write!(
                output,
                " x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
                format_scalar(rect.x, options.precision),
                format_scalar(rect.y, options.precision),
                format_scalar(rect.width, options.precision),
                format_scalar(rect.height, options.precision)
            )
            .unwrap();

            if rect.rx > 0.0 {
                write!(
                    output,
                    " rx=\"{}\"",
                    format_scalar(rect.rx, options.precision)
                )
                .unwrap();
            }
            if rect.ry > 0.0 {
                write!(
                    output,
                    " ry=\"{}\"",
                    format_scalar(rect.ry, options.precision)
                )
                .unwrap();
            }

            export_common_attrs(output, node, options);
            output.push_str("/>");
            output.push_str(newline);
        }
        SvgNodeKind::Circle(circle) => {
            output.push_str(&indent);
            output.push_str("<circle");

            write!(
                output,
                " cx=\"{}\" cy=\"{}\" r=\"{}\"",
                format_scalar(circle.cx, options.precision),
                format_scalar(circle.cy, options.precision),
                format_scalar(circle.r, options.precision)
            )
            .unwrap();

            export_common_attrs(output, node, options);
            output.push_str("/>");
            output.push_str(newline);
        }
        SvgNodeKind::Ellipse(ellipse) => {
            output.push_str(&indent);
            output.push_str("<ellipse");

            write!(
                output,
                " cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"",
                format_scalar(ellipse.cx, options.precision),
                format_scalar(ellipse.cy, options.precision),
                format_scalar(ellipse.rx, options.precision),
                format_scalar(ellipse.ry, options.precision)
            )
            .unwrap();

            export_common_attrs(output, node, options);
            output.push_str("/>");
            output.push_str(newline);
        }
        SvgNodeKind::Line(line) => {
            output.push_str(&indent);
            output.push_str("<line");

            write!(
                output,
                " x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"",
                format_scalar(line.x1, options.precision),
                format_scalar(line.y1, options.precision),
                format_scalar(line.x2, options.precision),
                format_scalar(line.y2, options.precision)
            )
            .unwrap();

            export_common_attrs(output, node, options);
            output.push_str("/>");
            output.push_str(newline);
        }
        SvgNodeKind::Polyline(points) => {
            output.push_str(&indent);
            output.push_str("<polyline");
            export_points_attr(output, points, options);
            export_common_attrs(output, node, options);
            output.push_str("/>");
            output.push_str(newline);
        }
        SvgNodeKind::Polygon(points) => {
            output.push_str(&indent);
            output.push_str("<polygon");
            export_points_attr(output, points, options);
            export_common_attrs(output, node, options);
            output.push_str("/>");
            output.push_str(newline);
        }
        SvgNodeKind::Path(path) => {
            output.push_str(&indent);
            output.push_str("<path");

            let path_data = export_path_data(path, options);
            write!(output, " d=\"{}\"", path_data).unwrap();

            export_common_attrs(output, node, options);
            output.push_str("/>");
            output.push_str(newline);
        }
        SvgNodeKind::Text(text) => {
            output.push_str(&indent);
            output.push_str("<text");

            write!(
                output,
                " x=\"{}\" y=\"{}\"",
                format_scalar(text.x, options.precision),
                format_scalar(text.y, options.precision)
            )
            .unwrap();

            if let Some(ref family) = text.font_family {
                write!(output, " font-family=\"{}\"", escape_xml(family)).unwrap();
            }

            write!(
                output,
                " font-size=\"{}\"",
                format_scalar(text.font_size, options.precision)
            )
            .unwrap();

            if text.font_weight != 400 {
                write!(output, " font-weight=\"{}\"", text.font_weight).unwrap();
            }

            match text.text_anchor {
                TextAnchor::Middle => output.push_str(" text-anchor=\"middle\""),
                TextAnchor::End => output.push_str(" text-anchor=\"end\""),
                TextAnchor::Start => {}
            }

            export_common_attrs(output, node, options);
            output.push('>');
            output.push_str(&escape_xml(&text.content));
            output.push_str("</text>");
            output.push_str(newline);
        }
        SvgNodeKind::Image(image) => {
            output.push_str(&indent);
            output.push_str("<image");

            write!(
                output,
                " x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
                format_scalar(image.x, options.precision),
                format_scalar(image.y, options.precision),
                format_scalar(image.width, options.precision),
                format_scalar(image.height, options.precision)
            )
            .unwrap();

            write!(output, " href=\"{}\"", escape_xml(&image.href)).unwrap();

            export_common_attrs(output, node, options);
            output.push_str("/>");
            output.push_str(newline);
        }
        SvgNodeKind::Use(href) => {
            output.push_str(&indent);
            output.push_str("<use");
            write!(output, " href=\"{}\"", escape_xml(href)).unwrap();
            export_common_attrs(output, node, options);
            output.push_str("/>");
            output.push_str(newline);
        }
        SvgNodeKind::Defs => {
            output.push_str(&indent);
            output.push_str("<defs>");
            output.push_str(newline);

            for child in &node.children {
                export_node(output, child, options, depth + 1);
            }

            output.push_str(&indent);
            output.push_str("</defs>");
            output.push_str(newline);
        }
        SvgNodeKind::LinearGradient(grad) => {
            output.push_str(&indent);
            output.push_str("<linearGradient");

            if let Some(ref id) = node.id {
                write!(output, " id=\"{}\"", escape_xml(id)).unwrap();
            }

            write!(
                output,
                " x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"",
                format_scalar(grad.x1, options.precision),
                format_scalar(grad.y1, options.precision),
                format_scalar(grad.x2, options.precision),
                format_scalar(grad.y2, options.precision)
            )
            .unwrap();

            export_gradient_attrs(output, &grad.spread, &grad.units);

            if !grad.transform.is_identity() {
                export_transform_attr(output, &grad.transform, options);
            }

            output.push('>');
            output.push_str(newline);

            for stop in &grad.stops {
                export_gradient_stop(output, stop, options, depth + 1);
            }

            output.push_str(&indent);
            output.push_str("</linearGradient>");
            output.push_str(newline);
        }
        SvgNodeKind::RadialGradient(grad) => {
            output.push_str(&indent);
            output.push_str("<radialGradient");

            if let Some(ref id) = node.id {
                write!(output, " id=\"{}\"", escape_xml(id)).unwrap();
            }

            write!(
                output,
                " cx=\"{}\" cy=\"{}\" r=\"{}\"",
                format_scalar(grad.cx, options.precision),
                format_scalar(grad.cy, options.precision),
                format_scalar(grad.r, options.precision)
            )
            .unwrap();

            if (grad.fx - grad.cx).abs() > 0.001 || (grad.fy - grad.cy).abs() > 0.001 {
                write!(
                    output,
                    " fx=\"{}\" fy=\"{}\"",
                    format_scalar(grad.fx, options.precision),
                    format_scalar(grad.fy, options.precision)
                )
                .unwrap();
            }

            export_gradient_attrs(output, &grad.spread, &grad.units);

            if !grad.transform.is_identity() {
                export_transform_attr(output, &grad.transform, options);
            }

            output.push('>');
            output.push_str(newline);

            for stop in &grad.stops {
                export_gradient_stop(output, stop, options, depth + 1);
            }

            output.push_str(&indent);
            output.push_str("</radialGradient>");
            output.push_str(newline);
        }
        SvgNodeKind::ClipPath(id) => {
            output.push_str(&indent);
            write!(output, "<clipPath id=\"{}\">", escape_xml(id)).unwrap();
            output.push_str(newline);

            for child in &node.children {
                export_node(output, child, options, depth + 1);
            }

            output.push_str(&indent);
            output.push_str("</clipPath>");
            output.push_str(newline);
        }
        SvgNodeKind::Unknown(tag) => {
            output.push_str(&indent);
            write!(output, "<{}", tag).unwrap();
            export_common_attrs(output, node, options);

            if node.children.is_empty() {
                output.push_str("/>");
            } else {
                output.push('>');
                output.push_str(newline);

                for child in &node.children {
                    export_node(output, child, options, depth + 1);
                }

                output.push_str(&indent);
                write!(output, "</{}>", tag).unwrap();
            }
            output.push_str(newline);
        }
    }
}

fn export_common_attrs(output: &mut String, node: &SvgNode, options: &SvgExportOptions) {
    // ID (but not for gradients, which handle it specially)
    if let Some(ref id) = node.id {
        if !matches!(
            node.kind,
            SvgNodeKind::LinearGradient(_) | SvgNodeKind::RadialGradient(_)
        ) {
            write!(output, " id=\"{}\"", escape_xml(id)).unwrap();
        }
    }

    // Classes
    if !node.classes.is_empty() {
        write!(output, " class=\"{}\"", node.classes.join(" ")).unwrap();
    }

    // Transform
    if !node.transform.is_identity() {
        export_transform_attr(output, &node.transform, options);
    }

    // Fill
    if let Some(ref fill) = node.fill {
        let fill_str = format_paint(fill);
        if fill_str != "black" || options.include_defaults {
            write!(output, " fill=\"{}\"", fill_str).unwrap();
        }
    } else {
        output.push_str(" fill=\"none\"");
    }

    // Stroke
    if let Some(ref stroke) = node.stroke {
        write!(output, " stroke=\"{}\"", format_paint(stroke)).unwrap();

        if node.stroke_width != 1.0 || options.include_defaults {
            write!(
                output,
                " stroke-width=\"{}\"",
                format_scalar(node.stroke_width, options.precision)
            )
            .unwrap();
        }
    }

    // Opacity
    if (node.opacity - 1.0).abs() > 0.001 {
        write!(
            output,
            " opacity=\"{}\"",
            format_scalar(node.opacity, options.precision)
        )
        .unwrap();
    }

    // Visibility
    if !node.visible {
        output.push_str(" visibility=\"hidden\"");
    }

    // Custom attributes (except internal ones)
    for (key, value) in &node.attributes {
        if !key.starts_with("__") && !is_standard_attr(key) {
            write!(output, " {}=\"{}\"", key, escape_xml(value)).unwrap();
        }
    }
}

fn is_standard_attr(key: &str) -> bool {
    matches!(
        key,
        "id" | "class"
            | "transform"
            | "fill"
            | "stroke"
            | "stroke-width"
            | "opacity"
            | "visibility"
    )
}

fn export_transform_attr(output: &mut String, matrix: &Matrix, options: &SvgExportOptions) {
    let v = &matrix.values;

    // Check for special cases
    let is_translate = (v[0] - 1.0).abs() < 0.001
        && v[1].abs() < 0.001
        && v[3].abs() < 0.001
        && (v[4] - 1.0).abs() < 0.001;

    let is_scale =
        v[1].abs() < 0.001 && v[3].abs() < 0.001 && v[2].abs() < 0.001 && v[5].abs() < 0.001;

    if is_translate && (v[2].abs() > 0.001 || v[5].abs() > 0.001) {
        write!(
            output,
            " transform=\"translate({}, {})\"",
            format_scalar(v[2], options.precision),
            format_scalar(v[5], options.precision)
        )
        .unwrap();
    } else if is_scale && ((v[0] - 1.0).abs() > 0.001 || (v[4] - 1.0).abs() > 0.001) {
        if (v[0] - v[4]).abs() < 0.001 {
            write!(
                output,
                " transform=\"scale({})\"",
                format_scalar(v[0], options.precision)
            )
            .unwrap();
        } else {
            write!(
                output,
                " transform=\"scale({}, {})\"",
                format_scalar(v[0], options.precision),
                format_scalar(v[4], options.precision)
            )
            .unwrap();
        }
    } else {
        // Use matrix form
        write!(
            output,
            " transform=\"matrix({}, {}, {}, {}, {}, {})\"",
            format_scalar(v[0], options.precision),
            format_scalar(v[3], options.precision),
            format_scalar(v[1], options.precision),
            format_scalar(v[4], options.precision),
            format_scalar(v[2], options.precision),
            format_scalar(v[5], options.precision)
        )
        .unwrap();
    }
}

fn export_points_attr(
    output: &mut String,
    points: &[skia_rs_core::Point],
    options: &SvgExportOptions,
) {
    output.push_str(" points=\"");
    for (i, p) in points.iter().enumerate() {
        if i > 0 {
            output.push(' ');
        }
        write!(
            output,
            "{},{}",
            format_scalar(p.x, options.precision),
            format_scalar(p.y, options.precision)
        )
        .unwrap();
    }
    output.push('"');
}

fn export_path_data(path: &skia_rs_path::Path, options: &SvgExportOptions) -> String {
    use skia_rs_path::PathElement;

    let mut data = String::new();

    for elem in path.iter() {
        match elem {
            PathElement::Move(p) => {
                write!(
                    data,
                    "M{} {}",
                    format_scalar(p.x, options.precision),
                    format_scalar(p.y, options.precision)
                )
                .unwrap();
            }
            PathElement::Line(p) => {
                write!(
                    data,
                    "L{} {}",
                    format_scalar(p.x, options.precision),
                    format_scalar(p.y, options.precision)
                )
                .unwrap();
            }
            PathElement::Quad(p1, p2) => {
                write!(
                    data,
                    "Q{} {} {} {}",
                    format_scalar(p1.x, options.precision),
                    format_scalar(p1.y, options.precision),
                    format_scalar(p2.x, options.precision),
                    format_scalar(p2.y, options.precision)
                )
                .unwrap();
            }
            PathElement::Conic(p1, p2, _w) => {
                // Conics aren't directly supported in SVG, approximate with quad
                // For now, output as quad (lossy)
                write!(
                    data,
                    "Q{} {} {} {}",
                    format_scalar(p1.x, options.precision),
                    format_scalar(p1.y, options.precision),
                    format_scalar(p2.x, options.precision),
                    format_scalar(p2.y, options.precision)
                )
                .unwrap();
            }
            PathElement::Cubic(p1, p2, p3) => {
                write!(
                    data,
                    "C{} {} {} {} {} {}",
                    format_scalar(p1.x, options.precision),
                    format_scalar(p1.y, options.precision),
                    format_scalar(p2.x, options.precision),
                    format_scalar(p2.y, options.precision),
                    format_scalar(p3.x, options.precision),
                    format_scalar(p3.y, options.precision)
                )
                .unwrap();
            }
            PathElement::Close => {
                data.push('Z');
            }
        }
    }

    data
}

fn export_gradient_attrs(output: &mut String, spread: &SpreadMethod, units: &GradientUnits) {
    match spread {
        SpreadMethod::Reflect => output.push_str(" spreadMethod=\"reflect\""),
        SpreadMethod::Repeat => output.push_str(" spreadMethod=\"repeat\""),
        SpreadMethod::Pad => {} // default
    }

    match units {
        GradientUnits::UserSpaceOnUse => output.push_str(" gradientUnits=\"userSpaceOnUse\""),
        GradientUnits::ObjectBoundingBox => {} // default
    }
}

fn export_gradient_stop(
    output: &mut String,
    stop: &GradientStop,
    options: &SvgExportOptions,
    depth: usize,
) {
    let indent = if options.pretty_print {
        options.indent.repeat(depth)
    } else {
        String::new()
    };
    let newline = if options.pretty_print { "\n" } else { "" };

    output.push_str(&indent);
    write!(
        output,
        "<stop offset=\"{}\" stop-color=\"{}\"",
        format_scalar(stop.offset, options.precision),
        format_color(&stop.color)
    )
    .unwrap();

    if (stop.opacity - 1.0).abs() > 0.001 {
        write!(
            output,
            " stop-opacity=\"{}\"",
            format_scalar(stop.opacity, options.precision)
        )
        .unwrap();
    }

    output.push_str("/>");
    output.push_str(newline);
}

fn format_paint(paint: &SvgPaint) -> String {
    match paint {
        SvgPaint::Color(color) => format_color(color),
        SvgPaint::Url(url) => format!("url({})", url),
        SvgPaint::None => "none".to_string(),
    }
}

fn format_color(color: &Color) -> String {
    if color.alpha() == 255 {
        format!(
            "#{:02x}{:02x}{:02x}",
            color.red(),
            color.green(),
            color.blue()
        )
    } else {
        format!(
            "rgba({}, {}, {}, {})",
            color.red(),
            color.green(),
            color.blue(),
            color.alpha() as f32 / 255.0
        )
    }
}

fn format_scalar(value: Scalar, precision: usize) -> String {
    let formatted = format!("{:.prec$}", value, prec = precision);
    // Remove trailing zeros and decimal point if unnecessary
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Trait extension for Matrix to check identity.
trait MatrixExt {
    fn is_identity(&self) -> bool;
}

impl MatrixExt for Matrix {
    fn is_identity(&self) -> bool {
        (self.values[0] - 1.0).abs() < 0.001
            && self.values[1].abs() < 0.001
            && self.values[2].abs() < 0.001
            && self.values[3].abs() < 0.001
            && (self.values[4] - 1.0).abs() < 0.001
            && self.values[5].abs() < 0.001
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::SvgRect;

    #[test]
    fn test_export_simple_svg() {
        let mut dom = SvgDom::new();
        dom.width = 100.0;
        dom.height = 100.0;

        let mut rect_node = SvgNode::new(SvgNodeKind::Rect(SvgRect {
            x: 10.0,
            y: 10.0,
            width: 80.0,
            height: 80.0,
            rx: 0.0,
            ry: 0.0,
        }));
        rect_node.fill = Some(SvgPaint::Color(Color::from_rgb(255, 0, 0)));

        dom.root.add_child(rect_node);

        let svg = export_svg(&dom);
        assert!(svg.contains("<rect"));
        assert!(svg.contains("fill=\"#ff0000\""));
    }

    #[test]
    fn test_export_minified() {
        let mut dom = SvgDom::new();
        dom.width = 100.0;
        dom.height = 100.0;

        let svg = export_svg_with_options(&dom, &SvgExportOptions::minified());
        assert!(!svg.contains('\n'));
        assert!(!svg.contains("<?xml"));
    }

    #[test]
    fn test_format_scalar() {
        assert_eq!(format_scalar(10.0, 3), "10");
        assert_eq!(format_scalar(10.5, 3), "10.5");
        assert_eq!(format_scalar(10.123456, 2), "10.12");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("<test>"), "&lt;test&gt;");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
    }

    #[test]
    fn test_format_color() {
        assert_eq!(format_color(&Color::from_rgb(255, 0, 0)), "#ff0000");
        assert_eq!(
            format_color(&Color::from_argb(128, 255, 0, 0)),
            "rgba(255, 0, 0, 0.5019608)"
        );
    }
}
