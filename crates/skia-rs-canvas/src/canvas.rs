//! Canvas drawing interface.

use skia_rs_core::{Color, Matrix, Point, Rect, Scalar};
use skia_rs_paint::Paint;
use skia_rs_path::Path;

/// Clip operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ClipOp {
    /// Intersect with clip.
    #[default]
    Intersect = 0,
    /// Difference from clip.
    Difference,
}

/// Save layer flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SaveLayerFlags(u32);

impl SaveLayerFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Preserve LCD text.
    pub const PRESERVE_LCD_TEXT: Self = Self(1 << 1);
    /// Initialize with previous layer.
    pub const INIT_WITH_PREVIOUS: Self = Self(1 << 2);
}

/// Save layer record.
#[derive(Debug, Clone, Default)]
pub struct SaveLayerRec<'a> {
    /// Bounds for the layer.
    pub bounds: Option<&'a Rect>,
    /// Paint for the layer.
    pub paint: Option<&'a Paint>,
    /// Flags.
    pub flags: SaveLayerFlags,
}

/// The main drawing interface.
pub struct Canvas {
    /// Current transformation matrix stack.
    matrix_stack: Vec<Matrix>,
    /// Clip stack.
    clip_stack: Vec<Rect>,
    /// Save count.
    save_count: usize,
    /// Width.
    width: i32,
    /// Height.
    height: i32,
}

impl Canvas {
    /// Create a new canvas with the given dimensions.
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            matrix_stack: vec![Matrix::IDENTITY],
            clip_stack: vec![Rect::from_xywh(0.0, 0.0, width as Scalar, height as Scalar)],
            save_count: 1,
            width,
            height,
        }
    }

    /// Get the width.
    #[inline]
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Get the height.
    #[inline]
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Get the current save count.
    #[inline]
    pub fn save_count(&self) -> usize {
        self.save_count
    }

    /// Get the current transformation matrix.
    #[inline]
    pub fn total_matrix(&self) -> &Matrix {
        self.matrix_stack.last().unwrap()
    }

    /// Get the current clip bounds.
    #[inline]
    pub fn clip_bounds(&self) -> Rect {
        self.clip_stack.last().copied().unwrap_or(Rect::EMPTY)
    }

    /// Save the current state.
    pub fn save(&mut self) -> usize {
        let matrix = *self.matrix_stack.last().unwrap();
        let clip = *self.clip_stack.last().unwrap();
        self.matrix_stack.push(matrix);
        self.clip_stack.push(clip);
        self.save_count += 1;
        self.save_count
    }

    /// Save the current state with a layer.
    pub fn save_layer(&mut self, _rec: &SaveLayerRec<'_>) -> usize {
        // TODO: Implement layer saving
        self.save()
    }

    /// Restore to the previous state.
    pub fn restore(&mut self) {
        if self.save_count > 1 {
            self.matrix_stack.pop();
            self.clip_stack.pop();
            self.save_count -= 1;
        }
    }

    /// Restore to a specific save count.
    pub fn restore_to_count(&mut self, count: usize) {
        while self.save_count > count {
            self.restore();
        }
    }

    /// Translate the canvas.
    pub fn translate(&mut self, dx: Scalar, dy: Scalar) {
        let matrix = Matrix::translate(dx, dy);
        self.concat(&matrix);
    }

    /// Scale the canvas.
    pub fn scale(&mut self, sx: Scalar, sy: Scalar) {
        let matrix = Matrix::scale(sx, sy);
        self.concat(&matrix);
    }

    /// Rotate the canvas (angle in degrees).
    pub fn rotate(&mut self, degrees: Scalar) {
        let radians = degrees * std::f32::consts::PI / 180.0;
        let matrix = Matrix::rotate(radians);
        self.concat(&matrix);
    }

    /// Skew the canvas.
    pub fn skew(&mut self, sx: Scalar, sy: Scalar) {
        let matrix = Matrix::skew(sx, sy);
        self.concat(&matrix);
    }

    /// Concatenate a matrix.
    pub fn concat(&mut self, matrix: &Matrix) {
        if let Some(current) = self.matrix_stack.last_mut() {
            *current = current.concat(matrix);
        }
    }

    /// Set the matrix.
    pub fn set_matrix(&mut self, matrix: &Matrix) {
        if let Some(current) = self.matrix_stack.last_mut() {
            *current = *matrix;
        }
    }

    /// Reset the matrix to identity.
    pub fn reset_matrix(&mut self) {
        self.set_matrix(&Matrix::IDENTITY);
    }

    /// Clip to a rectangle.
    pub fn clip_rect(&mut self, rect: &Rect, op: ClipOp, do_anti_alias: bool) {
        let _ = do_anti_alias;
        let transformed = self.total_matrix().map_rect(rect);

        if let Some(current) = self.clip_stack.last_mut() {
            match op {
                ClipOp::Intersect => {
                    if let Some(intersection) = current.intersect(&transformed) {
                        *current = intersection;
                    } else {
                        *current = Rect::EMPTY;
                    }
                }
                ClipOp::Difference => {
                    // TODO: Implement difference clipping
                }
            }
        }
    }

    /// Clip to a path.
    pub fn clip_path(&mut self, path: &Path, op: ClipOp, do_anti_alias: bool) {
        // Approximate with path bounds
        self.clip_rect(&path.bounds(), op, do_anti_alias);
    }

    /// Clear the canvas with a color.
    pub fn clear(&mut self, _color: Color) {
        // TODO: Implement clear
    }

    /// Draw a color.
    pub fn draw_color(&mut self, _color: Color, _blend_mode: skia_rs_paint::BlendMode) {
        // TODO: Implement draw_color
    }

    /// Draw a point.
    pub fn draw_point(&mut self, _point: Point, _paint: &Paint) {
        // TODO: Implement draw_point
    }

    /// Draw points.
    pub fn draw_points(&mut self, _mode: PointMode, _points: &[Point], _paint: &Paint) {
        // TODO: Implement draw_points
    }

    /// Draw a line.
    pub fn draw_line(&mut self, _p0: Point, _p1: Point, _paint: &Paint) {
        // TODO: Implement draw_line
    }

    /// Draw a rectangle.
    pub fn draw_rect(&mut self, _rect: &Rect, _paint: &Paint) {
        // TODO: Implement draw_rect
    }

    /// Draw an oval.
    pub fn draw_oval(&mut self, _rect: &Rect, _paint: &Paint) {
        // TODO: Implement draw_oval
    }

    /// Draw a circle.
    pub fn draw_circle(&mut self, _center: Point, _radius: Scalar, _paint: &Paint) {
        // TODO: Implement draw_circle
    }

    /// Draw an arc.
    pub fn draw_arc(&mut self, _oval: &Rect, _start_angle: Scalar, _sweep_angle: Scalar, _use_center: bool, _paint: &Paint) {
        // TODO: Implement draw_arc
    }

    /// Draw a rounded rectangle.
    pub fn draw_round_rect(&mut self, _rect: &Rect, _rx: Scalar, _ry: Scalar, _paint: &Paint) {
        // TODO: Implement draw_round_rect
    }

    /// Draw a path.
    pub fn draw_path(&mut self, _path: &Path, _paint: &Paint) {
        // TODO: Implement draw_path
    }

    /// Draw a picture.
    pub fn draw_picture(&mut self, picture: &crate::Picture, matrix: Option<&Matrix>, _paint: Option<&Paint>) {
        self.save();
        if let Some(m) = matrix {
            self.concat(m);
        }
        picture.playback(self);
        self.restore();
    }

    // =========================================================================
    // Quick Reject
    // =========================================================================

    /// Check if a rect would be fully clipped (quick reject).
    ///
    /// Returns true if drawing to this rect would have no visible effect.
    #[inline]
    pub fn quick_reject(&self, rect: &Rect) -> bool {
        let clip = self.clip_bounds();
        if clip.is_empty() {
            return true;
        }
        let transformed = self.total_matrix().map_rect(rect);
        !transformed.intersects(&clip)
    }

    /// Check if a path would be fully clipped.
    #[inline]
    pub fn quick_reject_path(&self, path: &Path) -> bool {
        self.quick_reject(&path.bounds())
    }

    // =========================================================================
    // Image Drawing
    // =========================================================================

    /// Draw an image lattice (nine-patch style stretching).
    ///
    /// The lattice divides the image into a grid, with some cells stretched
    /// and others drawn at their original size.
    pub fn draw_image_lattice(
        &mut self,
        _image_bounds: &Rect,
        _lattice: &ImageLattice,
        _dst: &Rect,
        _filter_mode: FilterMode,
        _paint: Option<&Paint>,
    ) {
        // Lattice drawing implementation placeholder
        // This divides the source image into a grid based on x_divs and y_divs,
        // then draws each cell to the corresponding destination cell
    }

    /// Draw multiple images from an atlas.
    ///
    /// Each sprite has a source rectangle in the atlas and a transformation.
    pub fn draw_atlas(
        &mut self,
        _atlas_bounds: &Rect,
        _xforms: &[RSXform],
        _sprites: &[Rect],
        _colors: Option<&[Color]>,
        _blend_mode: skia_rs_paint::BlendMode,
        _sampling: FilterMode,
        _paint: Option<&Paint>,
    ) {
        // Atlas drawing draws multiple sprites from a single texture
        // Each sprite has an RSXform for positioning and optional color tint
    }

    /// Draw a Coons patch.
    ///
    /// A Coons patch is defined by 12 control points forming a bicubic surface.
    pub fn draw_patch(
        &mut self,
        _cubic_points: &[Point; 12],
        _colors: Option<&[Color; 4]>,
        _tex_coords: Option<&[Point; 4]>,
        _blend_mode: skia_rs_paint::BlendMode,
        _paint: &Paint,
    ) {
        // Coons patch rendering - subdivide into triangles and interpolate
    }

    /// Draw an annotation.
    ///
    /// Annotations are used for PDF output (links, names, etc.).
    pub fn draw_annotation(&mut self, rect: &Rect, key: &str, value: &[u8]) {
        // Store annotation for PDF/SVG export
        let _ = (rect, key, value);
    }

    // =========================================================================
    // Text Drawing
    // =========================================================================

    /// Draw glyphs at specified positions.
    pub fn draw_glyphs(
        &mut self,
        _glyph_ids: &[u16],
        _positions: &[Point],
        _origin: Point,
        _font: &skia_rs_text::Font,
        _paint: &Paint,
    ) {
        // Draw each glyph at its specified position
    }

    /// Draw positioned text with alignment.
    pub fn draw_text_aligned(
        &mut self,
        text: &str,
        x: Scalar,
        y: Scalar,
        align: TextAlign,
        font: &skia_rs_text::Font,
        paint: &Paint,
    ) {
        let text_width = font.measure_text(text);
        let adjusted_x = match align {
            TextAlign::Left => x,
            TextAlign::Center => x - text_width / 2.0,
            TextAlign::Right => x - text_width,
        };
        self.draw_string(text, adjusted_x, y, font, paint);
    }

    /// Draw a string.
    pub fn draw_string(
        &mut self,
        _text: &str,
        _x: Scalar,
        _y: Scalar,
        _font: &skia_rs_text::Font,
        _paint: &Paint,
    ) {
        // Text drawing placeholder
    }

    /// Flush any pending operations.
    pub fn flush(&mut self) {
        // TODO: Implement flush
    }
}

// =============================================================================
// Supporting Types
// =============================================================================

/// Text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TextAlign {
    /// Left-aligned text.
    #[default]
    Left = 0,
    /// Center-aligned text.
    Center,
    /// Right-aligned text.
    Right,
}

/// Image lattice for nine-patch style drawing.
#[derive(Debug, Clone)]
pub struct ImageLattice {
    /// X division points.
    pub x_divs: Vec<i32>,
    /// Y division points.
    pub y_divs: Vec<i32>,
    /// Rectangle flags (which cells are fixed vs. scalable).
    pub rect_types: Option<Vec<LatticeRectType>>,
    /// Bounds within the source image.
    pub bounds: Option<skia_rs_core::IRect>,
}

impl ImageLattice {
    /// Create a new image lattice.
    pub fn new(x_divs: Vec<i32>, y_divs: Vec<i32>) -> Self {
        Self {
            x_divs,
            y_divs,
            rect_types: None,
            bounds: None,
        }
    }
}

/// Lattice rectangle type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum LatticeRectType {
    /// Default - draw the cell.
    #[default]
    Default = 0,
    /// Transparent - don't draw this cell.
    Transparent,
    /// Fixed color - fill with a solid color.
    FixedColor,
}

/// Rotation-scale transformation for atlas drawing.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct RSXform {
    /// Scale * cos(rotation).
    pub scos: Scalar,
    /// Scale * sin(rotation).
    pub ssin: Scalar,
    /// Translation X.
    pub tx: Scalar,
    /// Translation Y.
    pub ty: Scalar,
}

impl RSXform {
    /// Create from rotation and scale.
    pub fn from_radians(scale: Scalar, radians: Scalar, tx: Scalar, ty: Scalar, ax: Scalar, ay: Scalar) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            scos: scale * cos,
            ssin: scale * sin,
            tx: tx + -scale * (ax * cos - ay * sin),
            ty: ty + -scale * (ax * sin + ay * cos),
        }
    }

    /// Create a simple translation + scale.
    pub fn from_scale_translate(scale: Scalar, tx: Scalar, ty: Scalar) -> Self {
        Self {
            scos: scale,
            ssin: 0.0,
            tx,
            ty,
        }
    }

    /// Convert to a matrix.
    pub fn to_matrix(&self) -> Matrix {
        // Create a combined rotation-scale-translation matrix
        let rotation_scale = Matrix::rotate(self.ssin.atan2(self.scos));
        let scale = (self.scos * self.scos + self.ssin * self.ssin).sqrt();
        let scaled = rotation_scale.concat(&Matrix::scale(scale, scale));
        scaled.concat(&Matrix::translate(self.tx, self.ty))
    }
}

/// Filter mode for image sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FilterMode {
    /// Nearest neighbor sampling.
    #[default]
    Nearest = 0,
    /// Bilinear filtering.
    Linear,
}

/// Point drawing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PointMode {
    /// Draw each point.
    Points = 0,
    /// Draw lines between pairs.
    Lines,
    /// Draw connected line strip.
    Polygon,
}
