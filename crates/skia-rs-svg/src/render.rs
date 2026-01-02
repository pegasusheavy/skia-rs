//! SVG rendering to canvas.

use crate::dom::*;
use skia_rs_canvas::{RasterCanvas, Surface};
use skia_rs_core::{Color, Matrix, Point, Rect, Scalar};
use skia_rs_paint::{Paint, Style};
use skia_rs_path::PathBuilder;

/// Render an SVG DOM to a surface.
pub fn render_svg_to_surface(dom: &SvgDom, surface: &mut Surface) {
    let mut canvas = surface.raster_canvas();
    render_svg(&dom, &mut canvas);
}

/// Render an SVG DOM to a raster canvas.
pub fn render_svg(dom: &SvgDom, canvas: &mut RasterCanvas<'_>) {
    // Calculate scale to fit
    let view_box = dom.get_view_box();
    let scale_x = canvas.width() as Scalar / view_box.width();
    let scale_y = canvas.height() as Scalar / view_box.height();
    let scale = scale_x.min(scale_y);

    canvas.save();

    // Apply viewBox transform
    canvas.scale(scale, scale);
    canvas.translate(-view_box.left, -view_box.top);

    // Render root node
    render_node(&dom.root, canvas, dom);

    canvas.restore();
}

/// Render a single SVG node.
fn render_node(node: &SvgNode, canvas: &mut RasterCanvas<'_>, dom: &SvgDom) {
    if !node.visible {
        return;
    }

    canvas.save();

    // Apply transform
    canvas.concat(&node.transform);

    // Create paint for fill
    let fill_paint = node
        .fill
        .as_ref()
        .and_then(|fill| create_paint_from_svg_paint(fill, Style::Fill, node, dom));

    // Create paint for stroke
    let stroke_paint = node.stroke.as_ref().and_then(|stroke| {
        let mut paint = create_paint_from_svg_paint(stroke, Style::Stroke, node, dom)?;
        paint.set_stroke_width(node.stroke_width);
        Some(paint)
    });

    // Render based on node kind
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
        SvgNodeKind::Text(_text) => {
            // Text rendering requires font support
            // For now, skip text nodes
        }
        SvgNodeKind::Use(href) => {
            // Find referenced element
            let id = href.trim_start_matches('#');
            if let Some(referenced) = dom.root.find_by_id(id) {
                render_node(referenced, canvas, dom);
            }
        }
        SvgNodeKind::Group | SvgNodeKind::Svg | SvgNodeKind::Defs => {
            // Render children (except for defs which is just definitions)
            if !matches!(node.kind, SvgNodeKind::Defs) {
                for child in &node.children {
                    render_node(child, canvas, dom);
                }
            }
        }
        SvgNodeKind::Image(_img) => {
            // Image rendering requires image loading support
        }
        _ => {
            // Render children for unknown elements
            for child in &node.children {
                render_node(child, canvas, dom);
            }
        }
    }

    canvas.restore();
}

/// Create a Paint from an SVG paint specification.
fn create_paint_from_svg_paint(
    svg_paint: &SvgPaint,
    style: Style,
    node: &SvgNode,
    _dom: &SvgDom,
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
        SvgPaint::Url(_url) => {
            // Gradient/pattern lookup would go here
            // For now, return a default paint
            let mut paint = Paint::new();
            paint.set_style(style);
            paint.set_alpha(node.opacity);
            Some(paint)
        }
    }
}

/// Render an SVG string to a new surface.
pub fn render_svg_string(svg: &str, width: i32, height: i32) -> Option<Surface> {
    let dom = crate::parse_svg(svg).ok()?;
    let mut surface = Surface::new_raster_n32_premul(width, height)?;

    // Clear with white
    {
        let mut canvas = surface.raster_canvas();
        canvas.clear(Color::WHITE);
    }

    render_svg_to_surface(&dom, &mut surface);
    Some(surface)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_svg() {
        let svg = r#"<svg width="100" height="100">
            <rect x="10" y="10" width="80" height="80" fill="red"/>
        </svg>"#;

        let surface = render_svg_string(svg, 100, 100);
        assert!(surface.is_some());

        let surface = surface.unwrap();
        assert_eq!(surface.width(), 100);
        assert_eq!(surface.height(), 100);
    }

    #[test]
    fn test_render_circle() {
        let svg = r#"<svg width="100" height="100">
            <circle cx="50" cy="50" r="40" fill="blue"/>
        </svg>"#;

        let surface = render_svg_string(svg, 100, 100);
        assert!(surface.is_some());
    }

    #[test]
    fn test_render_path() {
        let svg = r#"<svg width="100" height="100">
            <path d="M10 10 L90 10 L90 90 L10 90 Z" fill="green"/>
        </svg>"#;

        let surface = render_svg_string(svg, 100, 100);
        assert!(surface.is_some());
    }
}
