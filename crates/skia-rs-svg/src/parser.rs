//! SVG parsing.
//!
//! Uses roxmltree for correct XML handling (entities, CDATA, namespaces,
//! mixed text/element content) and walks the tree to build an `SvgDom`.

use crate::css::Stylesheet;
use crate::dom::*;
use skia_rs_core::{Color, Matrix, Point, Rect, Scalar};
use skia_rs_path::parse_svg_path;
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
///
/// Uses `roxmltree` to walk the document and construct an `SvgDom`. Text
/// content between tags is captured (including inside `<text>` elements),
/// `<stop>` children of gradients populate their `stops` vector, inline
/// `style` attributes are preserved on each node, and the contents of
/// `<style>` elements are parsed into `dom.stylesheet` for later
/// application during rendering.
pub fn parse_svg(svg: &str) -> Result<SvgDom, SvgError> {
    let doc = roxmltree::Document::parse(svg).map_err(|e| SvgError::XmlError(e.to_string()))?;

    let root = doc.root_element();
    let mut dom = SvgDom::new();

    // Parse the root <svg> element's own attributes.
    dom.width = parse_length(root.attribute("width").unwrap_or("100"));
    dom.height = parse_length(root.attribute("height").unwrap_or("100"));
    if let Some(vb) = root.attribute("viewBox") {
        dom.view_box = parse_viewbox(vb);
    }

    // Build an attribute HashMap from the root for common-attribute
    // application (id/transform/fill/stroke/opacity), then recursively
    // build children.
    let mut root_node = SvgNode::new(SvgNodeKind::Svg);
    apply_common_attrs(&mut root_node, &attrs_of(root));

    for child in root.children() {
        if let Some(node) = build_node(child, &mut dom.stylesheet)? {
            root_node.add_child(node);
        }
    }

    dom.root = root_node;
    Ok(dom)
}

/// Collect all attributes of a roxmltree node into a HashMap.
///
/// Namespaced attributes are stored under both their local name and their
/// `prefix:local` form so that common lookups like `xlink:href` still work.
fn attrs_of(node: roxmltree::Node) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for attr in node.attributes() {
        let name = attr.name();
        let value = attr.value().to_string();
        // Always store under local name.
        map.insert(name.to_string(), value.clone());
        // Also under namespaced form when the attribute carries a namespace.
        if let Some(ns) = attr.namespace() {
            // Convert common namespaces to a prefix.
            let prefix = ns_to_prefix(ns);
            if !prefix.is_empty() {
                map.insert(format!("{}:{}", prefix, name), value);
            }
        }
    }
    map
}

fn ns_to_prefix(ns: &str) -> &'static str {
    match ns {
        "http://www.w3.org/1999/xlink" => "xlink",
        "http://www.w3.org/XML/1998/namespace" => "xml",
        _ => "",
    }
}

/// Accumulate all text content in a subtree (like DOM `textContent`).
fn collect_text(node: roxmltree::Node) -> String {
    let mut out = String::new();
    for desc in node.descendants() {
        if desc.is_text() {
            if let Some(t) = desc.text() {
                out.push_str(t);
            }
        }
    }
    out
}

/// Build an `SvgNode` from a roxmltree element. Returns `None` for text or
/// non-element nodes at the position being iterated. Stop elements are
/// handled by their parent gradient builder and are skipped here.
fn build_node(
    xml_node: roxmltree::Node,
    stylesheet: &mut Stylesheet,
) -> Result<Option<SvgNode>, SvgError> {
    if !xml_node.is_element() {
        return Ok(None);
    }

    let tag = xml_node.tag_name().name();
    let attrs = attrs_of(xml_node);

    // <style> elements contribute to the document stylesheet rather than
    // the rendered DOM. We still emit an Unknown("style") node so that
    // `extract_stylesheets` continues to work on already-parsed DOMs.
    if tag == "style" {
        let css_text = collect_text(xml_node);
        let parsed = Stylesheet::parse(&css_text);
        stylesheet.rules.extend(parsed.rules);

        let mut node = SvgNode::new(SvgNodeKind::Unknown("style".to_string()));
        apply_common_attrs(&mut node, &attrs);
        node.attributes
            .insert("__text_content".to_string(), css_text);
        return Ok(Some(node));
    }

    let mut node = match tag {
        "svg" => SvgNode::new(SvgNodeKind::Svg),
        "g" => SvgNode::new(SvgNodeKind::Group),
        "rect" => SvgNode::new(SvgNodeKind::Rect(SvgRect {
            x: parse_length(attrs.get("x").map(String::as_str).unwrap_or("0")),
            y: parse_length(attrs.get("y").map(String::as_str).unwrap_or("0")),
            width: parse_length(attrs.get("width").map(String::as_str).unwrap_or("0")),
            height: parse_length(attrs.get("height").map(String::as_str).unwrap_or("0")),
            rx: parse_length(attrs.get("rx").map(String::as_str).unwrap_or("0")),
            ry: parse_length(attrs.get("ry").map(String::as_str).unwrap_or("0")),
        })),
        "circle" => SvgNode::new(SvgNodeKind::Circle(SvgCircle {
            cx: parse_length(attrs.get("cx").map(String::as_str).unwrap_or("0")),
            cy: parse_length(attrs.get("cy").map(String::as_str).unwrap_or("0")),
            r: parse_length(attrs.get("r").map(String::as_str).unwrap_or("0")),
        })),
        "ellipse" => SvgNode::new(SvgNodeKind::Ellipse(SvgEllipse {
            cx: parse_length(attrs.get("cx").map(String::as_str).unwrap_or("0")),
            cy: parse_length(attrs.get("cy").map(String::as_str).unwrap_or("0")),
            rx: parse_length(attrs.get("rx").map(String::as_str).unwrap_or("0")),
            ry: parse_length(attrs.get("ry").map(String::as_str).unwrap_or("0")),
        })),
        "line" => SvgNode::new(SvgNodeKind::Line(SvgLine {
            x1: parse_length(attrs.get("x1").map(String::as_str).unwrap_or("0")),
            y1: parse_length(attrs.get("y1").map(String::as_str).unwrap_or("0")),
            x2: parse_length(attrs.get("x2").map(String::as_str).unwrap_or("0")),
            y2: parse_length(attrs.get("y2").map(String::as_str).unwrap_or("0")),
        })),
        "polyline" => {
            let points = parse_points(attrs.get("points").map(String::as_str).unwrap_or(""));
            SvgNode::new(SvgNodeKind::Polyline(points))
        }
        "polygon" => {
            let points = parse_points(attrs.get("points").map(String::as_str).unwrap_or(""));
            SvgNode::new(SvgNodeKind::Polygon(points))
        }
        "path" => {
            let d = attrs.get("d").map(String::as_str).unwrap_or("");
            let path = parse_svg_path(d).unwrap_or_default();
            SvgNode::new(SvgNodeKind::Path(path))
        }
        "text" => {
            let content = collect_text(xml_node);
            SvgNode::new(SvgNodeKind::Text(SvgText {
                x: parse_length(attrs.get("x").map(String::as_str).unwrap_or("0")),
                y: parse_length(attrs.get("y").map(String::as_str).unwrap_or("0")),
                content,
                font_family: attrs.get("font-family").cloned(),
                font_size: parse_length(
                    attrs
                        .get("font-size")
                        .map(String::as_str)
                        .unwrap_or("12"),
                ),
                font_weight: attrs
                    .get("font-weight")
                    .and_then(|w| w.parse().ok())
                    .unwrap_or(400),
                text_anchor: match attrs.get("text-anchor").map(String::as_str) {
                    Some("middle") => TextAnchor::Middle,
                    Some("end") => TextAnchor::End,
                    _ => TextAnchor::Start,
                },
            }))
        }
        "image" => {
            let href = attrs
                .get("href")
                .or_else(|| attrs.get("xlink:href"))
                .cloned()
                .unwrap_or_default();
            SvgNode::new(SvgNodeKind::Image(SvgImage {
                x: parse_length(attrs.get("x").map(String::as_str).unwrap_or("0")),
                y: parse_length(attrs.get("y").map(String::as_str).unwrap_or("0")),
                width: parse_length(attrs.get("width").map(String::as_str).unwrap_or("0")),
                height: parse_length(attrs.get("height").map(String::as_str).unwrap_or("0")),
                href,
            }))
        }
        "defs" => SvgNode::new(SvgNodeKind::Defs),
        "clipPath" => {
            let id = attrs.get("id").cloned().unwrap_or_default();
            SvgNode::new(SvgNodeKind::ClipPath(id))
        }
        "linearGradient" => {
            let gradient = SvgLinearGradient {
                x1: parse_length(attrs.get("x1").map(String::as_str).unwrap_or("0")),
                y1: parse_length(attrs.get("y1").map(String::as_str).unwrap_or("0")),
                x2: parse_length(attrs.get("x2").map(String::as_str).unwrap_or("100%")),
                y2: parse_length(attrs.get("y2").map(String::as_str).unwrap_or("0")),
                stops: collect_stops(xml_node),
                spread: parse_spread(attrs.get("spreadMethod").map(String::as_str)),
                units: parse_gradient_units(attrs.get("gradientUnits").map(String::as_str)),
                transform: attrs
                    .get("gradientTransform")
                    .map(|s| parse_transform_str(s))
                    .unwrap_or(Matrix::IDENTITY),
            };
            SvgNode::new(SvgNodeKind::LinearGradient(gradient))
        }
        "radialGradient" => {
            let cx = parse_length(attrs.get("cx").map(String::as_str).unwrap_or("50%"));
            let cy = parse_length(attrs.get("cy").map(String::as_str).unwrap_or("50%"));
            let fx_str = attrs.get("fx").cloned();
            let fy_str = attrs.get("fy").cloned();
            let gradient = SvgRadialGradient {
                cx,
                cy,
                r: parse_length(attrs.get("r").map(String::as_str).unwrap_or("50%")),
                fx: fx_str.map(|s| parse_length(&s)).unwrap_or(cx),
                fy: fy_str.map(|s| parse_length(&s)).unwrap_or(cy),
                stops: collect_stops(xml_node),
                spread: parse_spread(attrs.get("spreadMethod").map(String::as_str)),
                units: parse_gradient_units(attrs.get("gradientUnits").map(String::as_str)),
                transform: attrs
                    .get("gradientTransform")
                    .map(|s| parse_transform_str(s))
                    .unwrap_or(Matrix::IDENTITY),
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
        // Stops are consumed by their parent gradient; drop them here so
        // they don't appear as Unknown nodes in the rendered tree.
        "stop" => return Ok(None),
        _ => SvgNode::new(SvgNodeKind::Unknown(tag.to_string())),
    };

    apply_common_attrs(&mut node, &attrs);

    // Recurse into children.
    for child in xml_node.children() {
        if let Some(child_node) = build_node(child, stylesheet)? {
            node.add_child(child_node);
        }
    }

    Ok(Some(node))
}

/// Apply common presentation attributes to a node.
fn apply_common_attrs(node: &mut SvgNode, attrs: &HashMap<String, String>) {
    node.id = attrs.get("id").cloned();

    if let Some(class) = attrs.get("class") {
        node.classes = class.split_whitespace().map(|s| s.to_string()).collect();
    }

    if let Some(transform) = attrs.get("transform") {
        node.transform = parse_transform_str(transform);
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

    // Preserve `style="..."` so css::apply_stylesheet can reapply inline
    // declarations after stylesheet rules.
    if let Some(style) = attrs.get("style") {
        node.attributes
            .insert("style".to_string(), style.to_string());
    }

    // Preserve a handful of non-standard attributes that downstream code
    // reads directly. The CSS module stores the same keys on its own when
    // applying stroke-linecap / stroke-linejoin / etc. from stylesheets,
    // but inline attribute form needs a parser-side hook too.
    for key in [
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-dasharray",
        "stroke-dashoffset",
        "fill-rule",
        "clip-path",
    ] {
        if let Some(v) = attrs.get(key) {
            node.attributes.insert(key.to_string(), v.to_string());
        }
    }
}

/// Collect `<stop>` children of a gradient element into GradientStops.
fn collect_stops(gradient: roxmltree::Node) -> Vec<GradientStop> {
    let mut stops = Vec::new();
    for child in gradient.children() {
        if !child.is_element() {
            continue;
        }
        if child.tag_name().name() != "stop" {
            continue;
        }
        let attrs = attrs_of(child);

        // Offset may be a plain number (0..1) or a percentage.
        let offset_str = attrs
            .get("offset")
            .map(String::as_str)
            .unwrap_or("0");
        let offset = parse_length(offset_str).clamp(0.0, 1.0);

        // stop-color can live on the element or in a style attribute.
        let (stop_color, stop_opacity) = resolve_stop_style(&attrs);

        stops.push(GradientStop {
            offset,
            color: stop_color.unwrap_or(Color::BLACK),
            opacity: stop_opacity.unwrap_or(1.0),
        });
    }
    stops
}

fn resolve_stop_style(attrs: &HashMap<String, String>) -> (Option<Color>, Option<Scalar>) {
    let mut color = attrs.get("stop-color").and_then(|s| parse_color(s));
    let mut opacity = attrs
        .get("stop-opacity")
        .and_then(|s| s.parse::<Scalar>().ok());

    if let Some(style) = attrs.get("style") {
        for decl in style.split(';') {
            let decl = decl.trim();
            if let Some((prop, val)) = decl.split_once(':') {
                let prop = prop.trim();
                let val = val.trim();
                match prop {
                    "stop-color" if color.is_none() => {
                        color = parse_color(val);
                    }
                    "stop-opacity" if opacity.is_none() => {
                        opacity = val.parse().ok();
                    }
                    _ => {}
                }
            }
        }
    }

    (color, opacity)
}

fn parse_spread(s: Option<&str>) -> SpreadMethod {
    match s {
        Some("reflect") => SpreadMethod::Reflect,
        Some("repeat") => SpreadMethod::Repeat,
        _ => SpreadMethod::Pad,
    }
}

fn parse_gradient_units(s: Option<&str>) -> GradientUnits {
    match s {
        Some("userSpaceOnUse") => GradientUnits::UserSpaceOnUse,
        _ => GradientUnits::ObjectBoundingBox,
    }
}

/// Parse an SVG length value.
///
/// Recognises the unit suffixes defined by CSS Values 4. Viewport-relative
/// units (`vw`, `vh`, `vmin`, `vmax`) resolve against a 1000-unit-square
/// default viewport because the parser has no viewport context; the
/// renderer applies the real viewport transform separately via the
/// viewBox. Font-relative units (`em`, `rem`, `ex`, `ch`) assume a 16px
/// default font size — this matches browsers when no parent font is set.
/// Physical units (`cm`, `mm`, `in`, `pt`, `pc`) convert to CSS pixels at
/// 96dpi.
pub fn parse_length(s: &str) -> Scalar {
    let s = s.trim();
    if s.is_empty() {
        return 0.0;
    }

    // Strip optional trailing unit; anything else is treated as a plain
    // number in user units. Ordered longest-first so that `1rem` matches
    // `rem` before falling through to `em`.
    const UNITS: &[&str] = &[
        "vmin", "vmax", "rem", "px", "pt", "pc", "em", "ex", "ch", "vw", "vh", "cm", "mm", "in",
    ];

    // Percentage is a different beast: return as a fraction in [0,1].
    if let Some(stripped) = s.strip_suffix('%') {
        return stripped.parse::<Scalar>().unwrap_or(0.0) / 100.0;
    }

    for unit in UNITS {
        if let Some(num) = s.strip_suffix(unit) {
            let n: Scalar = num.parse().unwrap_or(0.0);
            return match *unit {
                "px" => n,
                "pt" => n * 96.0 / 72.0,
                "pc" => n * 16.0, // 1pc = 12pt = 16px
                "em" | "rem" => n * 16.0,
                "ex" => n * 8.0, // rough x-height approximation
                "ch" => n * 8.0, // rough 0-character width approximation
                "vw" | "vh" | "vmin" | "vmax" => n * 10.0, // 1% of a 1000-unit viewport
                "cm" => n * 96.0 / 2.54,
                "mm" => n * 96.0 / 25.4,
                "in" => n * 96.0,
                _ => n,
            };
        }
    }

    s.parse().unwrap_or(0.0)
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
pub(crate) fn parse_paint(s: &str) -> Option<SvgPaint> {
    let s = s.trim();
    if s == "none" {
        Some(SvgPaint::None)
    } else if let Some(url_body) = s.strip_prefix("url(") {
        let url = url_body
            .trim_end_matches(')')
            .trim_matches(|c| c == '"' || c == '\'');
        Some(SvgPaint::Url(url.to_string()))
    } else {
        parse_color(s).map(SvgPaint::Color)
    }
}

/// Parse a color value.
///
/// Supports `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`,
/// `hsl()`, `hsla()`, and the full X11 / CSS Color Level 3 named color
/// table.
pub(crate) fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();

    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    if let Some(inner) = s.strip_prefix("rgba(").and_then(|r| r.strip_suffix(')')) {
        return parse_rgba_inner(inner);
    }
    if let Some(inner) = s.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
        return parse_rgb_inner(inner);
    }
    if let Some(inner) = s.strip_prefix("hsla(").and_then(|r| r.strip_suffix(')')) {
        return parse_hsl_inner(inner, true);
    }
    if let Some(inner) = s.strip_prefix("hsl(").and_then(|r| r.strip_suffix(')')) {
        return parse_hsl_inner(inner, false);
    }

    named_color(&s.to_ascii_lowercase())
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Color::from_rgb(r, g, b))
        }
        4 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            let a = u8::from_str_radix(&hex[3..4], 16).ok()? * 17;
            Some(Color::from_argb(a, r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::from_rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Color::from_argb(a, r, g, b))
        }
        _ => None,
    }
}

fn parse_rgb_inner(inner: &str) -> Option<Color> {
    let parts: Vec<u8> = inner
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    if parts.len() == 3 {
        Some(Color::from_rgb(parts[0], parts[1], parts[2]))
    } else {
        None
    }
}

fn parse_rgba_inner(inner: &str) -> Option<Color> {
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() == 4 {
        let r: u8 = parts[0].parse().ok()?;
        let g: u8 = parts[1].parse().ok()?;
        let b: u8 = parts[2].parse().ok()?;
        let a: f32 = parts[3].parse().ok()?;
        Some(Color::from_argb((a * 255.0).clamp(0.0, 255.0) as u8, r, g, b))
    } else {
        None
    }
}

fn parse_hsl_inner(inner: &str, with_alpha: bool) -> Option<Color> {
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }
    let h: f32 = parts[0].trim_end_matches("deg").parse().ok()?;
    let s: f32 = parts[1].trim_end_matches('%').parse::<f32>().ok()? / 100.0;
    let l: f32 = parts[2].trim_end_matches('%').parse::<f32>().ok()? / 100.0;
    let a: f32 = if with_alpha && parts.len() >= 4 {
        parts[3].parse().ok()?
    } else {
        1.0
    };

    let (r, g, b) = hsl_to_rgb(h, s, l);
    Some(Color::from_argb((a * 255.0).clamp(0.0, 255.0) as u8, r, g, b))
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0) / 360.0;

    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return (v, v, v);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let to_u8 = |x: f32| (x * 255.0).round().clamp(0.0, 255.0) as u8;
    (
        to_u8(hue_to_rgb(p, q, h + 1.0 / 3.0)),
        to_u8(hue_to_rgb(p, q, h)),
        to_u8(hue_to_rgb(p, q, h - 1.0 / 3.0)),
    )
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

fn named_color(s: &str) -> Option<Color> {
    // CSS Color Level 3 named colors (subset of the X11 set). The compiler
    // collapses this to a perfect hash at -O1.
    Some(match s {
        "aliceblue" => Color::from_rgb(240, 248, 255),
        "antiquewhite" => Color::from_rgb(250, 235, 215),
        "aqua" | "cyan" => Color::from_rgb(0, 255, 255),
        "aquamarine" => Color::from_rgb(127, 255, 212),
        "azure" => Color::from_rgb(240, 255, 255),
        "beige" => Color::from_rgb(245, 245, 220),
        "bisque" => Color::from_rgb(255, 228, 196),
        "black" => Color::BLACK,
        "blanchedalmond" => Color::from_rgb(255, 235, 205),
        "blue" => Color::from_rgb(0, 0, 255),
        "blueviolet" => Color::from_rgb(138, 43, 226),
        "brown" => Color::from_rgb(165, 42, 42),
        "burlywood" => Color::from_rgb(222, 184, 135),
        "cadetblue" => Color::from_rgb(95, 158, 160),
        "chartreuse" => Color::from_rgb(127, 255, 0),
        "chocolate" => Color::from_rgb(210, 105, 30),
        "coral" => Color::from_rgb(255, 127, 80),
        "cornflowerblue" => Color::from_rgb(100, 149, 237),
        "cornsilk" => Color::from_rgb(255, 248, 220),
        "crimson" => Color::from_rgb(220, 20, 60),
        "darkblue" => Color::from_rgb(0, 0, 139),
        "darkcyan" => Color::from_rgb(0, 139, 139),
        "darkgoldenrod" => Color::from_rgb(184, 134, 11),
        "darkgray" | "darkgrey" => Color::from_rgb(169, 169, 169),
        "darkgreen" => Color::from_rgb(0, 100, 0),
        "darkkhaki" => Color::from_rgb(189, 183, 107),
        "darkmagenta" => Color::from_rgb(139, 0, 139),
        "darkolivegreen" => Color::from_rgb(85, 107, 47),
        "darkorange" => Color::from_rgb(255, 140, 0),
        "darkorchid" => Color::from_rgb(153, 50, 204),
        "darkred" => Color::from_rgb(139, 0, 0),
        "darksalmon" => Color::from_rgb(233, 150, 122),
        "darkseagreen" => Color::from_rgb(143, 188, 143),
        "darkslateblue" => Color::from_rgb(72, 61, 139),
        "darkslategray" | "darkslategrey" => Color::from_rgb(47, 79, 79),
        "darkturquoise" => Color::from_rgb(0, 206, 209),
        "darkviolet" => Color::from_rgb(148, 0, 211),
        "deeppink" => Color::from_rgb(255, 20, 147),
        "deepskyblue" => Color::from_rgb(0, 191, 255),
        "dimgray" | "dimgrey" => Color::from_rgb(105, 105, 105),
        "dodgerblue" => Color::from_rgb(30, 144, 255),
        "firebrick" => Color::from_rgb(178, 34, 34),
        "floralwhite" => Color::from_rgb(255, 250, 240),
        "forestgreen" => Color::from_rgb(34, 139, 34),
        "fuchsia" | "magenta" => Color::from_rgb(255, 0, 255),
        "gainsboro" => Color::from_rgb(220, 220, 220),
        "ghostwhite" => Color::from_rgb(248, 248, 255),
        "gold" => Color::from_rgb(255, 215, 0),
        "goldenrod" => Color::from_rgb(218, 165, 32),
        "gray" | "grey" => Color::from_rgb(128, 128, 128),
        "green" => Color::from_rgb(0, 128, 0),
        "greenyellow" => Color::from_rgb(173, 255, 47),
        "honeydew" => Color::from_rgb(240, 255, 240),
        "hotpink" => Color::from_rgb(255, 105, 180),
        "indianred" => Color::from_rgb(205, 92, 92),
        "indigo" => Color::from_rgb(75, 0, 130),
        "ivory" => Color::from_rgb(255, 255, 240),
        "khaki" => Color::from_rgb(240, 230, 140),
        "lavender" => Color::from_rgb(230, 230, 250),
        "lavenderblush" => Color::from_rgb(255, 240, 245),
        "lawngreen" => Color::from_rgb(124, 252, 0),
        "lemonchiffon" => Color::from_rgb(255, 250, 205),
        "lightblue" => Color::from_rgb(173, 216, 230),
        "lightcoral" => Color::from_rgb(240, 128, 128),
        "lightcyan" => Color::from_rgb(224, 255, 255),
        "lightgoldenrodyellow" => Color::from_rgb(250, 250, 210),
        "lightgray" | "lightgrey" => Color::from_rgb(211, 211, 211),
        "lightgreen" => Color::from_rgb(144, 238, 144),
        "lightpink" => Color::from_rgb(255, 182, 193),
        "lightsalmon" => Color::from_rgb(255, 160, 122),
        "lightseagreen" => Color::from_rgb(32, 178, 170),
        "lightskyblue" => Color::from_rgb(135, 206, 250),
        "lightslategray" | "lightslategrey" => Color::from_rgb(119, 136, 153),
        "lightsteelblue" => Color::from_rgb(176, 196, 222),
        "lightyellow" => Color::from_rgb(255, 255, 224),
        "lime" => Color::from_rgb(0, 255, 0),
        "limegreen" => Color::from_rgb(50, 205, 50),
        "linen" => Color::from_rgb(250, 240, 230),
        "maroon" => Color::from_rgb(128, 0, 0),
        "mediumaquamarine" => Color::from_rgb(102, 205, 170),
        "mediumblue" => Color::from_rgb(0, 0, 205),
        "mediumorchid" => Color::from_rgb(186, 85, 211),
        "mediumpurple" => Color::from_rgb(147, 112, 219),
        "mediumseagreen" => Color::from_rgb(60, 179, 113),
        "mediumslateblue" => Color::from_rgb(123, 104, 238),
        "mediumspringgreen" => Color::from_rgb(0, 250, 154),
        "mediumturquoise" => Color::from_rgb(72, 209, 204),
        "mediumvioletred" => Color::from_rgb(199, 21, 133),
        "midnightblue" => Color::from_rgb(25, 25, 112),
        "mintcream" => Color::from_rgb(245, 255, 250),
        "mistyrose" => Color::from_rgb(255, 228, 225),
        "moccasin" => Color::from_rgb(255, 228, 181),
        "navajowhite" => Color::from_rgb(255, 222, 173),
        "navy" => Color::from_rgb(0, 0, 128),
        "oldlace" => Color::from_rgb(253, 245, 230),
        "olive" => Color::from_rgb(128, 128, 0),
        "olivedrab" => Color::from_rgb(107, 142, 35),
        "orange" => Color::from_rgb(255, 165, 0),
        "orangered" => Color::from_rgb(255, 69, 0),
        "orchid" => Color::from_rgb(218, 112, 214),
        "palegoldenrod" => Color::from_rgb(238, 232, 170),
        "palegreen" => Color::from_rgb(152, 251, 152),
        "paleturquoise" => Color::from_rgb(175, 238, 238),
        "palevioletred" => Color::from_rgb(219, 112, 147),
        "papayawhip" => Color::from_rgb(255, 239, 213),
        "peachpuff" => Color::from_rgb(255, 218, 185),
        "peru" => Color::from_rgb(205, 133, 63),
        "pink" => Color::from_rgb(255, 192, 203),
        "plum" => Color::from_rgb(221, 160, 221),
        "powderblue" => Color::from_rgb(176, 224, 230),
        "purple" => Color::from_rgb(128, 0, 128),
        "rebeccapurple" => Color::from_rgb(102, 51, 153),
        "red" => Color::from_rgb(255, 0, 0),
        "rosybrown" => Color::from_rgb(188, 143, 143),
        "royalblue" => Color::from_rgb(65, 105, 225),
        "saddlebrown" => Color::from_rgb(139, 69, 19),
        "salmon" => Color::from_rgb(250, 128, 114),
        "sandybrown" => Color::from_rgb(244, 164, 96),
        "seagreen" => Color::from_rgb(46, 139, 87),
        "seashell" => Color::from_rgb(255, 245, 238),
        "sienna" => Color::from_rgb(160, 82, 45),
        "silver" => Color::from_rgb(192, 192, 192),
        "skyblue" => Color::from_rgb(135, 206, 235),
        "slateblue" => Color::from_rgb(106, 90, 205),
        "slategray" | "slategrey" => Color::from_rgb(112, 128, 144),
        "snow" => Color::from_rgb(255, 250, 250),
        "springgreen" => Color::from_rgb(0, 255, 127),
        "steelblue" => Color::from_rgb(70, 130, 180),
        "tan" => Color::from_rgb(210, 180, 140),
        "teal" => Color::from_rgb(0, 128, 128),
        "thistle" => Color::from_rgb(216, 191, 216),
        "tomato" => Color::from_rgb(255, 99, 71),
        "turquoise" => Color::from_rgb(64, 224, 208),
        "violet" => Color::from_rgb(238, 130, 238),
        "wheat" => Color::from_rgb(245, 222, 179),
        "white" => Color::WHITE,
        "whitesmoke" => Color::from_rgb(245, 245, 245),
        "yellow" => Color::from_rgb(255, 255, 0),
        "yellowgreen" => Color::from_rgb(154, 205, 50),
        "transparent" => Color::TRANSPARENT,
        _ => return None,
    })
}

/// Parse a transform string (public for CSS module).
pub fn parse_transform_str(s: &str) -> Matrix {
    let mut result = Matrix::IDENTITY;
    let s = s.trim();

    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_alphabetic() {
            let mut name = String::from(c);
            while let Some(&ch) = chars.peek() {
                if ch == '(' {
                    break;
                }
                name.push(chars.next().unwrap());
            }

            if chars.peek() == Some(&'(') {
                chars.next();
            }

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
        assert!((parse_length("1in") - 96.0).abs() < 0.01);
        assert!((parse_length("1cm") - 96.0 / 2.54).abs() < 0.01);
        assert!((parse_length("1em") - 16.0).abs() < 0.01);
        assert!((parse_length("1rem") - 16.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("#ff0000"), Some(Color::from_rgb(255, 0, 0)));
        assert_eq!(parse_color("#f00"), Some(Color::from_rgb(255, 0, 0)));
        assert_eq!(
            parse_color("rgb(255, 0, 0)"),
            Some(Color::from_rgb(255, 0, 0))
        );
        assert_eq!(parse_color("red"), Some(Color::from_rgb(255, 0, 0)));
        // Extended named colors.
        assert_eq!(
            parse_color("cornflowerblue"),
            Some(Color::from_rgb(100, 149, 237))
        );
        assert_eq!(
            parse_color("rebeccapurple"),
            Some(Color::from_rgb(102, 51, 153))
        );
        // 8-digit hex.
        assert_eq!(
            parse_color("#ff000080"),
            Some(Color::from_argb(128, 255, 0, 0))
        );
        // 4-digit hex.
        assert_eq!(
            parse_color("#f008"),
            Some(Color::from_argb(136, 255, 0, 0))
        );
        // HSL.
        let hsl_red = parse_color("hsl(0, 100%, 50%)").unwrap();
        assert_eq!(hsl_red.red(), 255);
        assert!(hsl_red.green() < 5);
        assert!(hsl_red.blue() < 5);
    }

    #[test]
    fn test_parse_simple_svg() {
        let svg = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
            <rect x="10" y="10" width="80" height="80" fill="red"/>
        </svg>"#;

        let dom = parse_svg(svg).unwrap();
        assert_eq!(dom.width, 100.0);
        assert_eq!(dom.height, 100.0);
        assert_eq!(dom.root.children.len(), 1);
    }

    #[test]
    fn test_parse_transform() {
        let m = parse_transform_str("translate(10, 20)");
        assert!((m.values[2] - 10.0).abs() < 0.01);
        assert!((m.values[5] - 20.0).abs() < 0.01);

        let m = parse_transform_str("scale(2)");
        assert!((m.values[0] - 2.0).abs() < 0.01);
        assert!((m.values[4] - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_text_content_captured() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <text x="10" y="20" font-size="14">Hello, world!</text>
        </svg>"#;

        let dom = parse_svg(svg).unwrap();
        let text = &dom.root.children[0];
        if let SvgNodeKind::Text(t) = &text.kind {
            assert_eq!(t.content, "Hello, world!");
            assert_eq!(t.font_size, 14.0);
        } else {
            panic!("expected text node");
        }
    }

    #[test]
    fn test_xml_entities_decoded() {
        // roxmltree decodes entities for us; verify it reaches the text
        // field.
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <text x="0" y="20">AT&amp;T &lt;3</text>
        </svg>"#;

        let dom = parse_svg(svg).unwrap();
        if let SvgNodeKind::Text(t) = &dom.root.children[0].kind {
            assert_eq!(t.content, "AT&T <3");
        } else {
            panic!("expected text node");
        }
    }

    #[test]
    fn test_gradient_stops_parsed() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
          <defs>
            <linearGradient id="g">
              <stop offset="0" stop-color="red"/>
              <stop offset="0.5" stop-color="green" stop-opacity="0.5"/>
              <stop offset="100%" style="stop-color: blue"/>
            </linearGradient>
          </defs>
        </svg>"#;

        let dom = parse_svg(svg).unwrap();
        let gradient = dom.root.find_by_id("g").expect("gradient present");
        if let SvgNodeKind::LinearGradient(g) = &gradient.kind {
            assert_eq!(g.stops.len(), 3);
            assert_eq!(g.stops[0].color, Color::from_rgb(255, 0, 0));
            assert!((g.stops[1].opacity - 0.5).abs() < 0.001);
            assert_eq!(g.stops[2].color, Color::from_rgb(0, 0, 255));
            assert!((g.stops[2].offset - 1.0).abs() < 0.001);
        } else {
            panic!("expected linear gradient");
        }
    }

    #[test]
    fn test_style_element_populates_stylesheet() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
          <style>rect { fill: red; }</style>
          <rect x="0" y="0" width="10" height="10"/>
        </svg>"#;

        let dom = parse_svg(svg).unwrap();
        assert_eq!(dom.stylesheet.rules.len(), 1);
    }

    #[test]
    fn test_inline_style_preserved() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
          <rect x="0" y="0" width="10" height="10" style="fill: blue"/>
        </svg>"#;

        let dom = parse_svg(svg).unwrap();
        let rect = &dom.root.children[0];
        assert_eq!(rect.attributes.get("style").map(String::as_str), Some("fill: blue"));
    }

    #[test]
    fn test_image_parsed() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
          <image x="5" y="6" width="100" height="200" href="data:image/png;base64,AAAA"/>
        </svg>"#;
        let dom = parse_svg(svg).unwrap();
        if let SvgNodeKind::Image(img) = &dom.root.children[0].kind {
            assert_eq!(img.x, 5.0);
            assert_eq!(img.y, 6.0);
            assert_eq!(img.width, 100.0);
            assert_eq!(img.height, 200.0);
            assert!(img.href.starts_with("data:image/png;base64,"));
        } else {
            panic!("expected image node");
        }
    }

    #[test]
    fn test_radial_gradient_stops() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
          <defs>
            <radialGradient id="rg" cx="50%" cy="50%" r="50%">
              <stop offset="0" stop-color="white"/>
              <stop offset="1" stop-color="black"/>
            </radialGradient>
          </defs>
        </svg>"#;
        let dom = parse_svg(svg).unwrap();
        let grad = dom.root.find_by_id("rg").unwrap();
        if let SvgNodeKind::RadialGradient(g) = &grad.kind {
            assert_eq!(g.stops.len(), 2);
        } else {
            panic!("expected radial gradient");
        }
    }
}
