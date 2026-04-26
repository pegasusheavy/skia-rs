//! SVG rendering to canvas.
//!
//! Walks the `SvgDom`, resolves paint references (gradients), applies the
//! parsed CSS stylesheet, renders text via skia-rs-text glyph paths, and
//! decodes `<image>` data URIs through skia-rs-codec. Clip paths are
//! applied via `Canvas::clip_path` using the union of geometry inside
//! the referenced `<clipPath>` element.

use crate::css::apply_stylesheet;
use crate::dom::*;
use skia_rs_canvas::{Canvas, ClipOp, Surface};
use skia_rs_codec::decode_image;
use skia_rs_core::{Color, Color4f, Matrix, Point, Rect, Scalar};
use skia_rs_paint::{
    LinearGradient as PaintLinearGradient, Paint, RadialGradient as PaintRadialGradient,
    ShaderRef, Style, TileMode,
};
use skia_rs_path::{Path, PathBuilder};
use skia_rs_text::Font;
use std::collections::HashMap;
use std::sync::Arc;

/// Resolved context passed through the render walk.
///
/// Built once at the top of `render_svg` so that every call to
/// `create_paint_from_svg_paint` has O(1) access to gradient/clipPath
/// lookup instead of re-walking the DOM for each element.
struct RenderContext<'a> {
    /// id -> SvgNode for all nodes in the DOM that carry an id.
    defs: HashMap<String, &'a SvgNode>,
}

impl<'a> RenderContext<'a> {
    fn new(dom: &'a SvgDom) -> Self {
        let mut defs = HashMap::new();
        collect_defs(&dom.root, &mut defs);
        Self { defs }
    }

    fn lookup(&self, href: &str) -> Option<&'a SvgNode> {
        let id = href.trim_start_matches('#');
        self.defs.get(id).copied()
    }
}

fn collect_defs<'a>(node: &'a SvgNode, defs: &mut HashMap<String, &'a SvgNode>) {
    if let Some(ref id) = node.id {
        defs.insert(id.clone(), node);
    }
    for child in &node.children {
        collect_defs(child, defs);
    }
}

/// Render an SVG DOM to a surface.
pub fn render_svg_to_surface(dom: &SvgDom, surface: &mut Surface) {
    let mut canvas = surface.canvas();
    render_svg(dom, &mut canvas);
}

/// Render an SVG DOM to a raster canvas.
///
/// Applies the DOM's parsed stylesheet onto a working clone of the DOM
/// before walking so that CSS-styled attributes (fill/stroke/opacity/etc.)
/// take effect without mutating the caller's DOM.
pub fn render_svg(dom: &SvgDom, canvas: &mut Canvas<'_>) {
    // Apply stylesheet onto a cloned DOM so the caller's is untouched.
    let mut working = dom.clone();
    let sheet = working.stylesheet.clone();
    if !sheet.rules.is_empty() {
        apply_stylesheet(&mut working, &sheet);
    } else if dom_has_inline_styles(&working.root) {
        // Even with no <style> block, inline style= attributes may still
        // need to take effect. apply_stylesheet always walks inline
        // styles, so call it with an empty stylesheet whenever any node
        // has a style attribute.
        apply_stylesheet(&mut working, &crate::css::Stylesheet::new());
    }

    let ctx = RenderContext::new(&working);

    // Calculate scale to fit.
    let view_box = working.get_view_box();
    if view_box.width() <= 0.0 || view_box.height() <= 0.0 {
        return;
    }
    let scale_x = canvas.width() as Scalar / view_box.width();
    let scale_y = canvas.height() as Scalar / view_box.height();
    let scale = scale_x.min(scale_y);

    canvas.save();
    canvas.scale(scale, scale);
    canvas.translate(-view_box.left, -view_box.top);

    render_node(&working.root, canvas, &ctx);

    canvas.restore();
}

fn dom_has_inline_styles(node: &SvgNode) -> bool {
    if node.attributes.contains_key("style") {
        return true;
    }
    node.children.iter().any(dom_has_inline_styles)
}

/// Render a single SVG node.
fn render_node(node: &SvgNode, canvas: &mut Canvas<'_>, ctx: &RenderContext<'_>) {
    if !node.visible {
        return;
    }

    canvas.save();
    canvas.concat(&node.transform);

    // clip-path reference applied before drawing the node contents.
    let clip_ref = node.attributes.get("clip-path").cloned();
    if let Some(href) = clip_ref.as_deref() {
        if let Some(id) = extract_url_id(href) {
            if let Some(clip_node) = ctx.lookup(id) {
                if let SvgNodeKind::ClipPath(_) = &clip_node.kind {
                    let path = build_clip_path(clip_node);
                    if !path.is_empty() {
                        canvas.clip_path(&path, ClipOp::Intersect, true);
                    }
                }
            }
        }
    }

    let node_bounds_for_gradient = node.bounds();

    let fill_paint = node.fill.as_ref().and_then(|fill| {
        create_paint_from_svg_paint(fill, Style::Fill, node, ctx, node_bounds_for_gradient)
    });

    let stroke_paint = node.stroke.as_ref().and_then(|stroke| {
        let mut paint =
            create_paint_from_svg_paint(stroke, Style::Stroke, node, ctx, node_bounds_for_gradient)?;
        paint.set_stroke_width(node.stroke_width);
        Some(paint)
    });

    match &node.kind {
        SvgNodeKind::Rect(rect) => {
            let r = Rect::from_xywh(rect.x, rect.y, rect.width, rect.height);
            if rect.rx > 0.0 || rect.ry > 0.0 {
                if let Some(paint) = &fill_paint {
                    canvas.draw_round_rect(&r, rect.rx, rect.ry, paint);
                }
                if let Some(paint) = &stroke_paint {
                    canvas.draw_round_rect(&r, rect.rx, rect.ry, paint);
                }
            } else {
                if let Some(paint) = &fill_paint {
                    canvas.draw_rect(&r, paint);
                }
                if let Some(paint) = &stroke_paint {
                    canvas.draw_rect(&r, paint);
                }
            }
        }
        SvgNodeKind::Circle(circle) => {
            let center = Point::new(circle.cx, circle.cy);
            if let Some(paint) = &fill_paint {
                canvas.draw_circle(center, circle.r, paint);
            }
            if let Some(paint) = &stroke_paint {
                canvas.draw_circle(center, circle.r, paint);
            }
        }
        SvgNodeKind::Ellipse(ellipse) => {
            let oval = Rect::from_xywh(
                ellipse.cx - ellipse.rx,
                ellipse.cy - ellipse.ry,
                ellipse.rx * 2.0,
                ellipse.ry * 2.0,
            );
            if let Some(paint) = &fill_paint {
                canvas.draw_oval(&oval, paint);
            }
            if let Some(paint) = &stroke_paint {
                canvas.draw_oval(&oval, paint);
            }
        }
        SvgNodeKind::Line(line) => {
            if let Some(paint) = &stroke_paint {
                canvas.draw_line(
                    Point::new(line.x1, line.y1),
                    Point::new(line.x2, line.y2),
                    paint,
                );
            }
        }
        SvgNodeKind::Polyline(points) => {
            if points.len() >= 2 {
                let mut builder = PathBuilder::new();
                builder.move_to(points[0].x, points[0].y);
                for p in &points[1..] {
                    builder.line_to(p.x, p.y);
                }
                let path = builder.build();
                if let Some(paint) = &stroke_paint {
                    canvas.draw_path(&path, paint);
                }
            }
        }
        SvgNodeKind::Polygon(points) => {
            if points.len() >= 3 {
                let mut builder = PathBuilder::new();
                builder.move_to(points[0].x, points[0].y);
                for p in &points[1..] {
                    builder.line_to(p.x, p.y);
                }
                builder.close();
                let path = builder.build();
                if let Some(paint) = &fill_paint {
                    canvas.draw_path(&path, paint);
                }
                if let Some(paint) = &stroke_paint {
                    canvas.draw_path(&path, paint);
                }
            }
        }
        SvgNodeKind::Path(path) => {
            if let Some(paint) = &fill_paint {
                canvas.draw_path(path, paint);
            }
            if let Some(paint) = &stroke_paint {
                canvas.draw_path(path, paint);
            }
        }
        SvgNodeKind::Text(text) => {
            render_text(text, canvas, fill_paint.as_ref(), stroke_paint.as_ref());
        }
        SvgNodeKind::Image(img) => {
            render_image(img, canvas, node.opacity);
        }
        SvgNodeKind::Use(href) => {
            if let Some(referenced) = ctx.lookup(href) {
                render_node(referenced, canvas, ctx);
            }
        }
        SvgNodeKind::Group | SvgNodeKind::Svg => {
            for child in &node.children {
                render_node(child, canvas, ctx);
            }
        }
        SvgNodeKind::Defs
        | SvgNodeKind::ClipPath(_)
        | SvgNodeKind::LinearGradient(_)
        | SvgNodeKind::RadialGradient(_) => {
            // Definition-only elements — not part of the rendered tree.
        }
        SvgNodeKind::Unknown(_) => {
            // Render children so that unknown wrappers (e.g. <switch>,
            // <a>) don't hide their contents. <style> elements are handled
            // separately during parse and don't reach render.
            for child in &node.children {
                render_node(child, canvas, ctx);
            }
        }
    }

    canvas.restore();
}

/// Extract the id from an `url(#id)` reference; returns None if `s` is
/// anything else (including a bare id without the `url()` wrapper — those
/// are handled by the caller when appropriate).
fn extract_url_id(s: &str) -> Option<&str> {
    let s = s.trim();
    if let Some(stripped) = s.strip_prefix("url(") {
        let inner = stripped.trim_end_matches(')');
        let inner = inner.trim_matches(|c| c == '"' || c == '\'');
        Some(inner.trim_start_matches('#'))
    } else if let Some(stripped) = s.strip_prefix('#') {
        Some(stripped)
    } else {
        None
    }
}

/// Build a combined Path representing the geometry inside a `<clipPath>`
/// element. Walks direct children and unions their geometry.
fn build_clip_path(clip_node: &SvgNode) -> Path {
    let mut builder = PathBuilder::new();
    for child in &clip_node.children {
        add_node_geometry(child, &mut builder);
    }
    builder.build()
}

fn add_node_geometry(node: &SvgNode, builder: &mut PathBuilder) {
    match &node.kind {
        SvgNodeKind::Rect(rect) => {
            builder.add_rect(&Rect::from_xywh(rect.x, rect.y, rect.width, rect.height));
        }
        SvgNodeKind::Circle(c) => {
            builder.add_oval(&Rect::from_xywh(
                c.cx - c.r,
                c.cy - c.r,
                c.r * 2.0,
                c.r * 2.0,
            ));
        }
        SvgNodeKind::Ellipse(e) => {
            builder.add_oval(&Rect::from_xywh(
                e.cx - e.rx,
                e.cy - e.ry,
                e.rx * 2.0,
                e.ry * 2.0,
            ));
        }
        SvgNodeKind::Path(p) => {
            let transformed = p.transformed(&node.transform);
            builder.add_path(&transformed);
        }
        SvgNodeKind::Polygon(points) if points.len() >= 3 => {
            builder.move_to(points[0].x, points[0].y);
            for p in &points[1..] {
                builder.line_to(p.x, p.y);
            }
            builder.close();
        }
        _ => {
            // Recurse so groups inside clipPath contribute their geometry.
            for child in &node.children {
                add_node_geometry(child, builder);
            }
        }
    }
}

/// Render a `<text>` element by converting each glyph to a path and
/// filling/stroking it. Uses skia-rs-text for glyph-to-path conversion.
fn render_text(
    text: &SvgText,
    canvas: &mut Canvas<'_>,
    fill_paint: Option<&Paint>,
    stroke_paint: Option<&Paint>,
) {
    if text.content.is_empty() {
        return;
    }

    let font = Font::from_size(text.font_size.max(1.0));
    let width = font.measure_text(&text.content);

    let anchor_offset = match text.text_anchor {
        TextAnchor::Start => 0.0,
        TextAnchor::Middle => -width / 2.0,
        TextAnchor::End => -width,
    };

    let mut x_offset = text.x + anchor_offset;
    let y_baseline = text.y;

    for ch in text.content.chars() {
        let glyph = font.char_to_glyph(ch);
        let advance = font.glyph_advance(glyph);

        if let Some(glyph_path) = font.glyph_path(glyph) {
            let translated =
                glyph_path.transformed(&Matrix::translate(x_offset, y_baseline));
            if let Some(paint) = fill_paint {
                canvas.draw_path(&translated, paint);
            }
            if let Some(paint) = stroke_paint {
                canvas.draw_path(&translated, paint);
            }
        }

        x_offset += advance;
    }
}

/// Render an `<image>` element. Supports `data:` URIs; falls back silently
/// for other schemes since network fetching is out of scope for this
/// crate.
fn render_image(img: &SvgImage, canvas: &mut Canvas<'_>, opacity: Scalar) {
    let data = match decode_image_href(&img.href) {
        Some(d) => d,
        None => return,
    };

    let image = match decode_image(&data) {
        Ok(image) => image,
        Err(_) => return,
    };

    let target_w = if img.width > 0.0 {
        img.width
    } else {
        image.width() as Scalar
    };
    let target_h = if img.height > 0.0 {
        img.height
    } else {
        image.height() as Scalar
    };
    let dst = Rect::from_xywh(img.x, img.y, target_w, target_h);

    let mut paint = Paint::new();
    paint.set_alpha(opacity);
    canvas.draw_image_rect(&image, None, &dst, Some(&paint));
}

/// Decode a `data:` URI into raw bytes. Returns None for non-data-URI
/// hrefs or malformed input.
fn decode_image_href(href: &str) -> Option<Vec<u8>> {
    let rest = href.strip_prefix("data:")?;
    // mime/params,payload
    let (meta, payload) = rest.split_once(',')?;
    let is_base64 = meta.contains(";base64");
    if is_base64 {
        decode_base64(payload.trim())
    } else {
        // URL-encoded text — fall back to raw bytes since SVG data URIs
        // typically use base64 for binary formats. Caller gets a decode
        // failure if the format doesn't match.
        Some(payload.as_bytes().to_vec())
    }
}

/// Decode a base64-encoded string (standard alphabet, ignores whitespace
/// and padding).
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;

    for byte in input.bytes() {
        let value: u32 = match byte {
            b'A'..=b'Z' => (byte - b'A') as u32,
            b'a'..=b'z' => (byte - b'a' + 26) as u32,
            b'0'..=b'9' => (byte - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            b'=' => continue, // Skip padding.
            b' ' | b'\t' | b'\n' | b'\r' => continue,
            _ => return None, // Invalid byte in base64.
        };

        buffer = (buffer << 6) | value;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
        }
    }

    Some(output)
}

/// Create a Paint from an SVG paint specification, resolving gradient
/// URL references against the context's defs.
fn create_paint_from_svg_paint(
    svg_paint: &SvgPaint,
    style: Style,
    node: &SvgNode,
    ctx: &RenderContext<'_>,
    node_bounds: Rect,
) -> Option<Paint> {
    match svg_paint {
        SvgPaint::None => None,
        SvgPaint::Color(color) => {
            let mut paint = Paint::new();
            paint.set_color32(*color);
            paint.set_style(style);
            paint.set_alpha(node.opacity);
            Some(paint)
        }
        SvgPaint::Url(url) => {
            let id = url.trim_start_matches('#');
            let referenced = ctx.defs.get(id).copied();
            let shader = referenced.and_then(|r| build_gradient_shader(r, node_bounds));

            let mut paint = Paint::new();
            paint.set_style(style);
            paint.set_alpha(node.opacity);
            if let Some(shader) = shader {
                paint.set_shader(Some(shader));
            } else {
                // No gradient found / unresolvable URL: fall back to black
                // so the geometry is still visible rather than silently
                // vanishing.
                paint.set_color32(Color::BLACK);
            }
            Some(paint)
        }
    }
}

/// Convert an SVG gradient node into a skia-rs-paint shader.
fn build_gradient_shader(node: &SvgNode, object_bounds: Rect) -> Option<ShaderRef> {
    match &node.kind {
        SvgNodeKind::LinearGradient(grad) => {
            let (start, end) = resolve_linear_endpoints(grad, object_bounds);
            let (colors, positions) = stops_to_colors(&grad.stops);
            if colors.is_empty() {
                return None;
            }
            let tile_mode = spread_to_tile_mode(grad.spread);
            let gradient = PaintLinearGradient::new(start, end, colors, positions, tile_mode);
            let gradient = if grad.transform != Matrix::IDENTITY {
                gradient.with_local_matrix(grad.transform)
            } else {
                gradient
            };
            Some(Arc::new(gradient) as ShaderRef)
        }
        SvgNodeKind::RadialGradient(grad) => {
            let (center, radius) = resolve_radial_params(grad, object_bounds);
            let (colors, positions) = stops_to_colors(&grad.stops);
            if colors.is_empty() {
                return None;
            }
            let tile_mode = spread_to_tile_mode(grad.spread);
            let gradient =
                PaintRadialGradient::new(center, radius, colors, positions, tile_mode);
            let gradient = if grad.transform != Matrix::IDENTITY {
                gradient.with_local_matrix(grad.transform)
            } else {
                gradient
            };
            Some(Arc::new(gradient) as ShaderRef)
        }
        _ => None,
    }
}

fn resolve_linear_endpoints(grad: &SvgLinearGradient, bounds: Rect) -> (Point, Point) {
    match grad.units {
        GradientUnits::ObjectBoundingBox => {
            // Stored endpoints are in the 0..1 domain (e.g. from a `50%`
            // attribute parsed as 0.5); map into object-local coords.
            let w = bounds.width();
            let h = bounds.height();
            (
                Point::new(bounds.left + grad.x1 * w, bounds.top + grad.y1 * h),
                Point::new(bounds.left + grad.x2 * w, bounds.top + grad.y2 * h),
            )
        }
        GradientUnits::UserSpaceOnUse => (
            Point::new(grad.x1, grad.y1),
            Point::new(grad.x2, grad.y2),
        ),
    }
}

fn resolve_radial_params(grad: &SvgRadialGradient, bounds: Rect) -> (Point, Scalar) {
    match grad.units {
        GradientUnits::ObjectBoundingBox => {
            let w = bounds.width();
            let h = bounds.height();
            let size = w.min(h);
            (
                Point::new(bounds.left + grad.cx * w, bounds.top + grad.cy * h),
                grad.r * size,
            )
        }
        GradientUnits::UserSpaceOnUse => (Point::new(grad.cx, grad.cy), grad.r),
    }
}

fn stops_to_colors(stops: &[GradientStop]) -> (Vec<Color4f>, Option<Vec<Scalar>>) {
    let colors: Vec<Color4f> = stops
        .iter()
        .map(|s| {
            let mut c = Color4f::from_color(s.color);
            c.a *= s.opacity;
            c
        })
        .collect();
    let positions: Vec<Scalar> = stops.iter().map(|s| s.offset).collect();
    let positions_opt = if positions.is_empty() {
        None
    } else {
        Some(positions)
    };
    (colors, positions_opt)
}

fn spread_to_tile_mode(spread: SpreadMethod) -> TileMode {
    match spread {
        SpreadMethod::Pad => TileMode::Clamp,
        SpreadMethod::Reflect => TileMode::Mirror,
        SpreadMethod::Repeat => TileMode::Repeat,
    }
}

/// Render an SVG string to a new surface.
pub fn render_svg_string(svg: &str, width: i32, height: i32) -> Option<Surface> {
    let dom = crate::parse_svg(svg).ok()?;
    let mut surface = Surface::new_raster_n32_premul(width, height)?;

    {
        let mut canvas = surface.canvas();
        canvas.clear(Color::WHITE);
    }

    render_svg_to_surface(&dom, &mut surface);
    Some(surface)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sample a pixel from the surface backing buffer.
    ///
    /// The raster surface stores pixels as RGBA8 row-major with 4 bytes
    /// per pixel.
    fn pixel_at(surface: &Surface, x: i32, y: i32) -> Option<Color> {
        let pixels = surface.pixels();
        let width = surface.width();
        let row_bytes = (width * 4) as usize;
        let off = (y as usize) * row_bytes + (x as usize) * 4;
        if off + 4 > pixels.len() {
            return None;
        }
        Some(Color::from_argb(
            pixels[off + 3],
            pixels[off],
            pixels[off + 1],
            pixels[off + 2],
        ))
    }

    #[test]
    fn test_render_simple_svg() {
        let svg = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
            <rect x="10" y="10" width="80" height="80" fill="red"/>
        </svg>"#;

        let surface = render_svg_string(svg, 100, 100);
        assert!(surface.is_some());

        let surface = surface.unwrap();
        assert_eq!(surface.width(), 100);
        assert_eq!(surface.height(), 100);

        // Interior of the rect (50,50) should be red.
        let c = pixel_at(&surface, 50, 50).unwrap();
        assert!(c.red() > 200, "expected mostly red, got {:?}", c);
        assert!(c.green() < 60);
        assert!(c.blue() < 60);
    }

    #[test]
    fn test_render_circle() {
        let svg = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
            <circle cx="50" cy="50" r="40" fill="blue"/>
        </svg>"#;

        let surface = render_svg_string(svg, 100, 100).unwrap();
        // Center of the circle should be blue.
        let c = pixel_at(&surface, 50, 50).unwrap();
        assert!(c.blue() > 200, "center should be blue, got {:?}", c);
        assert!(c.red() < 60);
    }

    #[test]
    fn test_render_path() {
        let svg = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
            <path d="M10 10 L90 10 L90 90 L10 90 Z" fill="green"/>
        </svg>"#;

        let surface = render_svg_string(svg, 100, 100).unwrap();
        let c = pixel_at(&surface, 50, 50).unwrap();
        assert!(c.green() > 100, "center should be green-ish, got {:?}", c);
    }

    #[test]
    fn test_render_css_styled() {
        // The stylesheet says fill:red; rect tag has no inline fill.
        // Expect the rect to render red.
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">
            <style>rect { fill: red; }</style>
            <rect x="0" y="0" width="20" height="20"/>
        </svg>"#;
        let surface = render_svg_string(svg, 20, 20).unwrap();
        let c = pixel_at(&surface, 10, 10).unwrap();
        assert!(c.red() > 200, "css fill should apply, got {:?}", c);
        assert!(c.green() < 60);
    }

    #[test]
    fn test_render_inline_style() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">
            <rect x="0" y="0" width="20" height="20" style="fill: blue"/>
        </svg>"#;
        let surface = render_svg_string(svg, 20, 20).unwrap();
        let c = pixel_at(&surface, 10, 10).unwrap();
        assert!(c.blue() > 200, "inline style should apply, got {:?}", c);
    }

    #[test]
    fn test_render_linear_gradient() {
        // Horizontal gradient from red at x=0 to blue at x=100.
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="20">
          <defs>
            <linearGradient id="g" x1="0" y1="0" x2="1" y2="0">
              <stop offset="0" stop-color="red"/>
              <stop offset="1" stop-color="blue"/>
            </linearGradient>
          </defs>
          <rect x="0" y="0" width="100" height="20" fill="url(#g)"/>
        </svg>"#;
        let surface = render_svg_string(svg, 100, 20).unwrap();

        let left = pixel_at(&surface, 5, 10).unwrap();
        let right = pixel_at(&surface, 95, 10).unwrap();
        assert!(
            left.red() > 200,
            "left edge should be red, got {:?}",
            left
        );
        assert!(
            right.blue() > 200,
            "right edge should be blue, got {:?}",
            right
        );
    }

    #[test]
    fn test_render_clip_path() {
        // Without clipPath the rect would cover the entire surface.
        // With clipPath=url(#tinyClip) only a tiny 5x5 area in the
        // top-left remains filled — the rest of the canvas stays the
        // white background from render_svg_string's clear().
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50">
          <defs>
            <clipPath id="tinyClip">
              <rect x="0" y="0" width="5" height="5"/>
            </clipPath>
          </defs>
          <rect x="0" y="0" width="50" height="50" fill="red" clip-path="url(#tinyClip)"/>
        </svg>"#;
        let surface = render_svg_string(svg, 50, 50).unwrap();
        let inside = pixel_at(&surface, 2, 2).unwrap();
        let outside = pixel_at(&surface, 30, 30).unwrap();
        assert!(
            inside.red() > 200 && inside.green() < 60,
            "inside clip should be red, got {:?}",
            inside
        );
        assert!(
            outside.red() > 240 && outside.green() > 240 && outside.blue() > 240,
            "outside clip should remain the white background, got {:?}",
            outside
        );
    }

    #[test]
    fn test_url_id_extraction() {
        assert_eq!(extract_url_id("url(#foo)"), Some("foo"));
        assert_eq!(extract_url_id("url(\"#bar\")"), Some("bar"));
        assert_eq!(extract_url_id("#baz"), Some("baz"));
        assert_eq!(extract_url_id("plain"), None);
    }

    #[test]
    fn test_base64_decode() {
        assert_eq!(decode_base64("SGVsbG8=").unwrap(), b"Hello");
        assert_eq!(decode_base64("SGVsbG8gV29ybGQ=").unwrap(), b"Hello World");
        // Whitespace tolerated.
        assert_eq!(decode_base64("SGVs\nbG8=").unwrap(), b"Hello");
    }

    #[test]
    fn test_render_text_does_not_panic() {
        // The default typeface has no glyph data so nothing visible will
        // be drawn, but the render path must complete without panicking
        // when encountering text content (regression test for the prior
        // "Text rendering is a no-op" behaviour).
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="40">
            <text x="5" y="25" font-size="14" fill="black">hello</text>
        </svg>"#;
        let surface = render_svg_string(svg, 100, 40);
        assert!(surface.is_some());
    }
}
