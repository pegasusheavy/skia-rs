//! PDF canvas for drawing.

use skia_rs_core::{Color, Matrix, Point, Rect, Scalar};
use skia_rs_paint::{Paint, Style};
use skia_rs_path::{Path, PathElement};

/// A canvas that generates PDF content streams.
pub struct PdfCanvas {
    /// Page width.
    width: Scalar,
    /// Page height.
    height: Scalar,
    /// Object ID.
    object_id: u32,
    /// Content stream.
    content: Vec<u8>,
    /// Graphics state stack.
    state_stack: Vec<GraphicsState>,
}

/// Graphics state.
#[derive(Clone)]
struct GraphicsState {
    /// Current transformation matrix.
    matrix: Matrix,
    /// Current color.
    color: Color,
    /// Line width.
    line_width: Scalar,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            matrix: Matrix::IDENTITY,
            color: Color::BLACK,
            line_width: 1.0,
        }
    }
}

impl PdfCanvas {
    /// Create a new PDF canvas.
    pub fn new(width: Scalar, height: Scalar, object_id: u32) -> Self {
        let mut canvas = Self {
            width,
            height,
            object_id,
            content: Vec::new(),
            state_stack: vec![GraphicsState::default()],
        };

        // Set up coordinate system (PDF has origin at bottom-left)
        canvas.write_op(&format!("1 0 0 -1 0 {} cm\n", height));

        canvas
    }

    /// Get the width.
    pub fn width(&self) -> Scalar {
        self.width
    }

    /// Get the height.
    pub fn height(&self) -> Scalar {
        self.height
    }

    /// Get the object ID.
    pub fn object_id(&self) -> u32 {
        self.object_id
    }

    /// Convert to content bytes.
    pub fn into_content(self) -> Vec<u8> {
        self.content
    }

    /// Write a PDF operation.
    fn write_op(&mut self, op: &str) {
        self.content.extend_from_slice(op.as_bytes());
    }

    /// Get current state.
    fn state(&self) -> &GraphicsState {
        self.state_stack.last().unwrap()
    }

    /// Get mutable current state.
    fn state_mut(&mut self) -> &mut GraphicsState {
        self.state_stack.last_mut().unwrap()
    }

    /// Save graphics state.
    pub fn save(&mut self) {
        let state = self.state().clone();
        self.state_stack.push(state);
        self.write_op("q\n");
    }

    /// Restore graphics state.
    pub fn restore(&mut self) {
        if self.state_stack.len() > 1 {
            self.state_stack.pop();
            self.write_op("Q\n");
        }
    }

    /// Apply a transform.
    pub fn concat(&mut self, matrix: &Matrix) {
        self.state_mut().matrix = self.state().matrix.concat(matrix);
        self.write_op(&format!(
            "{} {} {} {} {} {} cm\n",
            matrix.values[0],
            matrix.values[3],
            matrix.values[1],
            matrix.values[4],
            matrix.values[2],
            matrix.values[5]
        ));
    }

    /// Translate.
    pub fn translate(&mut self, dx: Scalar, dy: Scalar) {
        self.concat(&Matrix::translate(dx, dy));
    }

    /// Scale.
    pub fn scale(&mut self, sx: Scalar, sy: Scalar) {
        self.concat(&Matrix::scale(sx, sy));
    }

    /// Rotate (degrees).
    pub fn rotate(&mut self, degrees: Scalar) {
        let radians = degrees * std::f32::consts::PI / 180.0;
        self.concat(&Matrix::rotate(radians));
    }

    /// Set the fill color.
    pub fn set_fill_color(&mut self, color: Color) {
        let r = color.red() as f32 / 255.0;
        let g = color.green() as f32 / 255.0;
        let b = color.blue() as f32 / 255.0;
        self.state_mut().color = color;
        self.write_op(&format!("{:.3} {:.3} {:.3} rg\n", r, g, b));
    }

    /// Set the stroke color.
    pub fn set_stroke_color(&mut self, color: Color) {
        let r = color.red() as f32 / 255.0;
        let g = color.green() as f32 / 255.0;
        let b = color.blue() as f32 / 255.0;
        self.write_op(&format!("{:.3} {:.3} {:.3} RG\n", r, g, b));
    }

    /// Set the line width.
    pub fn set_line_width(&mut self, width: Scalar) {
        self.state_mut().line_width = width;
        self.write_op(&format!("{} w\n", width));
    }

    /// Draw a rectangle.
    pub fn draw_rect(&mut self, rect: &Rect, paint: &Paint) {
        self.apply_paint(paint);
        self.write_op(&format!(
            "{} {} {} {} re ",
            rect.left,
            rect.top,
            rect.width(),
            rect.height()
        ));
        self.stroke_or_fill(paint);
    }

    /// Draw a line.
    pub fn draw_line(&mut self, p0: Point, p1: Point, paint: &Paint) {
        self.apply_paint(paint);
        self.write_op(&format!("{} {} m {} {} l S\n", p0.x, p0.y, p1.x, p1.y));
    }

    /// Draw a circle.
    pub fn draw_circle(&mut self, center: Point, radius: Scalar, paint: &Paint) {
        // Approximate circle with bezier curves
        let k = radius * 0.5522847498; // Magic constant for circle approximation

        self.apply_paint(paint);
        self.write_op(&format!("{} {} m\n", center.x + radius, center.y));

        // Four quadrants
        self.write_op(&format!(
            "{} {} {} {} {} {} c\n",
            center.x + radius, center.y + k,
            center.x + k, center.y + radius,
            center.x, center.y + radius
        ));
        self.write_op(&format!(
            "{} {} {} {} {} {} c\n",
            center.x - k, center.y + radius,
            center.x - radius, center.y + k,
            center.x - radius, center.y
        ));
        self.write_op(&format!(
            "{} {} {} {} {} {} c\n",
            center.x - radius, center.y - k,
            center.x - k, center.y - radius,
            center.x, center.y - radius
        ));
        self.write_op(&format!(
            "{} {} {} {} {} {} c\n",
            center.x + k, center.y - radius,
            center.x + radius, center.y - k,
            center.x + radius, center.y
        ));

        self.stroke_or_fill(paint);
    }

    /// Draw a path.
    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        self.apply_paint(paint);

        let mut current = Point::zero();

        for element in path.iter() {
            match element {
                PathElement::Move(p) => {
                    self.write_op(&format!("{} {} m\n", p.x, p.y));
                    current = p;
                }
                PathElement::Line(p) => {
                    self.write_op(&format!("{} {} l\n", p.x, p.y));
                    current = p;
                }
                PathElement::Quad(ctrl, end) => {
                    // Convert quadratic to cubic
                    let c1 = Point::new(
                        current.x + 2.0 / 3.0 * (ctrl.x - current.x),
                        current.y + 2.0 / 3.0 * (ctrl.y - current.y),
                    );
                    let c2 = Point::new(
                        end.x + 2.0 / 3.0 * (ctrl.x - end.x),
                        end.y + 2.0 / 3.0 * (ctrl.y - end.y),
                    );
                    self.write_op(&format!(
                        "{} {} {} {} {} {} c\n",
                        c1.x, c1.y, c2.x, c2.y, end.x, end.y
                    ));
                    current = end;
                }
                PathElement::Conic(ctrl, end, _w) => {
                    // Approximate as quadratic
                    let c1 = Point::new(
                        current.x + 2.0 / 3.0 * (ctrl.x - current.x),
                        current.y + 2.0 / 3.0 * (ctrl.y - current.y),
                    );
                    let c2 = Point::new(
                        end.x + 2.0 / 3.0 * (ctrl.x - end.x),
                        end.y + 2.0 / 3.0 * (ctrl.y - end.y),
                    );
                    self.write_op(&format!(
                        "{} {} {} {} {} {} c\n",
                        c1.x, c1.y, c2.x, c2.y, end.x, end.y
                    ));
                    current = end;
                }
                PathElement::Cubic(c1, c2, end) => {
                    self.write_op(&format!(
                        "{} {} {} {} {} {} c\n",
                        c1.x, c1.y, c2.x, c2.y, end.x, end.y
                    ));
                    current = end;
                }
                PathElement::Close => {
                    self.write_op("h\n");
                }
            }
        }

        self.stroke_or_fill(paint);
    }

    /// Draw text (basic support).
    pub fn draw_text(&mut self, text: &str, x: Scalar, y: Scalar, font_size: Scalar, paint: &Paint) {
        self.apply_paint(paint);
        self.write_op("BT\n");
        self.write_op(&format!("/F1 {} Tf\n", font_size));
        self.write_op(&format!("{} {} Td\n", x, y));
        self.write_op(&format!("({}) Tj\n", escape_pdf_string(text)));
        self.write_op("ET\n");
    }

    /// Apply paint settings.
    fn apply_paint(&mut self, paint: &Paint) {
        let color = paint.color32();

        match paint.style() {
            Style::Fill => self.set_fill_color(color),
            Style::Stroke => {
                self.set_stroke_color(color);
                self.set_line_width(paint.stroke_width());
            }
            Style::StrokeAndFill => {
                self.set_fill_color(color);
                self.set_stroke_color(color);
                self.set_line_width(paint.stroke_width());
            }
        }
    }

    /// Write stroke or fill operator.
    fn stroke_or_fill(&mut self, paint: &Paint) {
        match paint.style() {
            Style::Fill => self.write_op("f\n"),
            Style::Stroke => self.write_op("S\n"),
            Style::StrokeAndFill => self.write_op("B\n"),
        }
    }
}

/// Escape special characters in a PDF string.
fn escape_pdf_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '(' => result.push_str("\\("),
            ')' => result.push_str("\\)"),
            '\\' => result.push_str("\\\\"),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_canvas_rect() {
        let mut canvas = PdfCanvas::new(612.0, 792.0, 1);

        let mut paint = Paint::new();
        paint.set_color32(Color::from_rgb(255, 0, 0));

        canvas.draw_rect(&Rect::from_xywh(100.0, 100.0, 200.0, 150.0), &paint);

        let content = String::from_utf8(canvas.into_content()).unwrap();
        assert!(content.contains("re"));
        assert!(content.contains("f")); // Fill operator
    }

    #[test]
    fn test_pdf_canvas_save_restore() {
        let mut canvas = PdfCanvas::new(612.0, 792.0, 1);

        canvas.save();
        canvas.translate(100.0, 100.0);
        canvas.restore();

        let content = String::from_utf8(canvas.into_content()).unwrap();
        assert!(content.contains("q")); // Save
        assert!(content.contains("Q")); // Restore
    }
}
