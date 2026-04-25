//! Canvas drawing interface.
//!
//! The [`Canvas`] type is the single public drawing surface. It carries a
//! [`Backing`] enum that selects where draws actually go:
//!
//! - [`Backing::Raster`] — CPU rasterizer against a borrowed [`PixelBuffer`].
//! - [`Backing::Recording`] — appends [`DrawCommand`]s to a borrowed buffer,
//!   used by [`PictureRecorder`] to build a replayable [`Picture`].
//! - [`Backing::Null`] — no-op. Useful for matrix/clip-only benchmarks and
//!   quick-reject probes.
//!
//! The enum is open to future variants (GPU, PDF, SVG) without breaking the
//! public method surface.
//!
//! [`PixelBuffer`]: crate::raster::PixelBuffer
//! [`DrawCommand`]: crate::picture::DrawCommand
//! [`PictureRecorder`]: crate::picture::PictureRecorder
//! [`Picture`]: crate::picture::Picture

use crate::picture::DrawCommand;
use crate::raster::{PixelBuffer, Rasterizer};
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

/// The rendering backend a [`Canvas`] is attached to.
///
/// New variants (GPU, PDF, SVG, …) can be added without rewriting the public
/// draw method surface — each method just adds another match arm.
pub enum Backing<'a> {
    /// Rasterize directly into the borrowed pixel buffer.
    Raster(&'a mut PixelBuffer),
    /// Append draw commands into the borrowed command buffer.
    ///
    /// This is how [`crate::picture::PictureRecorder`] captures a picture.
    Recording(&'a mut Vec<DrawCommand>),
    /// Discard every draw. Matrix and clip stack state is still maintained so
    /// that [`Canvas::quick_reject`] and friends can be used against a live
    /// canvas without a backing store.
    Null,
}

impl std::fmt::Debug for Backing<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backing::Raster(_) => f.debug_tuple("Raster").field(&"<PixelBuffer>").finish(),
            Backing::Recording(cmds) => f
                .debug_tuple("Recording")
                .field(&format_args!("<{} cmds>", cmds.len()))
                .finish(),
            Backing::Null => f.write_str("Null"),
        }
    }
}

/// The main drawing interface.
///
/// `Canvas` is the single public canvas type. Its [`Backing`] determines
/// whether draws go to a pixel buffer, into a recorded picture, or are
/// discarded.
pub struct Canvas<'a> {
    /// Where draws go.
    backing: Backing<'a>,
    /// Current transformation matrix stack.
    matrix_stack: Vec<Matrix>,
    /// Clip stack (rectangular; full ClipStack upgrade happens in P5-2).
    clip_stack: Vec<Rect>,
    /// Save count.
    save_count: usize,
    /// Width of the logical canvas.
    width: i32,
    /// Height of the logical canvas.
    height: i32,
}

impl<'a> Canvas<'a> {
    /// Create a no-op canvas with the given logical dimensions.
    ///
    /// This is equivalent to `Canvas::new_null(width, height)` and is kept as
    /// `new` for backwards compatibility. Draws are discarded; matrix and clip
    /// state is tracked.
    pub fn new(width: i32, height: i32) -> Self {
        Self::new_null(width, height)
    }

    /// Create a raster canvas that draws into the given pixel buffer.
    pub fn new_raster(buffer: &'a mut PixelBuffer) -> Self {
        let width = buffer.width;
        let height = buffer.height;
        Self {
            backing: Backing::Raster(buffer),
            matrix_stack: vec![Matrix::IDENTITY],
            clip_stack: vec![Rect::from_xywh(
                0.0,
                0.0,
                width as Scalar,
                height as Scalar,
            )],
            save_count: 1,
            width,
            height,
        }
    }

    /// Create a recording canvas that appends draw commands into `commands`.
    ///
    /// `width`/`height` are only used to seed the initial clip bounds and to
    /// answer [`width`](Self::width)/[`height`](Self::height) queries.
    pub fn new_recording(commands: &'a mut Vec<DrawCommand>, width: i32, height: i32) -> Self {
        Self {
            backing: Backing::Recording(commands),
            matrix_stack: vec![Matrix::IDENTITY],
            clip_stack: vec![Rect::from_xywh(
                0.0,
                0.0,
                width as Scalar,
                height as Scalar,
            )],
            save_count: 1,
            width,
            height,
        }
    }

    /// Create a no-op canvas with the given dimensions.
    pub fn new_null(width: i32, height: i32) -> Self {
        Self {
            backing: Backing::Null,
            matrix_stack: vec![Matrix::IDENTITY],
            clip_stack: vec![Rect::from_xywh(
                0.0,
                0.0,
                width as Scalar,
                height as Scalar,
            )],
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

    /// Inspect the backing (primarily for diagnostics and tests).
    #[inline]
    pub fn backing(&self) -> &Backing<'a> {
        &self.backing
    }

    /// Save the current state.
    pub fn save(&mut self) -> usize {
        let matrix = *self.matrix_stack.last().unwrap();
        let clip = *self.clip_stack.last().unwrap();
        self.matrix_stack.push(matrix);
        self.clip_stack.push(clip);
        self.save_count += 1;
        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::Save);
        }
        self.save_count
    }

    /// Save the current state with a layer.
    ///
    /// Layer composition itself is tracked by P5-4; this currently behaves
    /// like a plain [`save`](Self::save) but records a `SaveLayer` command in
    /// the recording backing so playback preserves intent.
    pub fn save_layer(&mut self, rec: &SaveLayerRec<'_>) -> usize {
        let bounds = rec.bounds.copied();
        let paint = rec.paint.cloned();
        // Maintain matrix/clip stack.
        let matrix = *self.matrix_stack.last().unwrap();
        let clip = *self.clip_stack.last().unwrap();
        self.matrix_stack.push(matrix);
        self.clip_stack.push(clip);
        self.save_count += 1;
        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::SaveLayer { bounds, paint });
        }
        self.save_count
    }

    /// Restore to the previous state.
    pub fn restore(&mut self) {
        if self.save_count > 1 {
            self.matrix_stack.pop();
            self.clip_stack.pop();
            self.save_count -= 1;
            if let Backing::Recording(commands) = &mut self.backing {
                commands.push(DrawCommand::Restore);
            }
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
        self.concat_internal(&matrix);
        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::Translate { dx, dy });
        }
    }

    /// Scale the canvas.
    pub fn scale(&mut self, sx: Scalar, sy: Scalar) {
        let matrix = Matrix::scale(sx, sy);
        self.concat_internal(&matrix);
        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::Scale { sx, sy });
        }
    }

    /// Rotate the canvas (angle in degrees).
    pub fn rotate(&mut self, degrees: Scalar) {
        let radians = degrees * std::f32::consts::PI / 180.0;
        let matrix = Matrix::rotate(radians);
        self.concat_internal(&matrix);
        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::Rotate { degrees });
        }
    }

    /// Skew the canvas.
    pub fn skew(&mut self, sx: Scalar, sy: Scalar) {
        let matrix = Matrix::skew(sx, sy);
        self.concat_internal(&matrix);
        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::Skew { sx, sy });
        }
    }

    /// Concatenate a matrix.
    pub fn concat(&mut self, matrix: &Matrix) {
        self.concat_internal(matrix);
        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::Concat { matrix: *matrix });
        }
    }

    /// Internal concat that only mutates the matrix stack, no recording.
    fn concat_internal(&mut self, matrix: &Matrix) {
        if let Some(current) = self.matrix_stack.last_mut() {
            *current = current.concat(matrix);
        }
    }

    /// Set the matrix.
    pub fn set_matrix(&mut self, matrix: &Matrix) {
        if let Some(current) = self.matrix_stack.last_mut() {
            *current = *matrix;
        }
        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::SetMatrix { matrix: *matrix });
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
                    // Difference clipping requires region math; tracked as
                    // GAP-C8 / P5-2.
                }
            }
        }

        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::ClipRect {
                rect: *rect,
                anti_alias: do_anti_alias,
            });
        }
    }

    /// Clip to a path.
    pub fn clip_path(&mut self, path: &Path, op: ClipOp, do_anti_alias: bool) {
        // Approximate with path bounds; full path clipping is tracked as
        // GAP-C9 / P5-2.
        self.clip_rect(&path.bounds(), op, do_anti_alias);
        // clip_rect above already recorded a ClipRect for the recording
        // backing; replace it with the more precise ClipPath.
        if let Backing::Recording(commands) = &mut self.backing {
            if matches!(commands.last(), Some(DrawCommand::ClipRect { .. })) {
                commands.pop();
            }
            commands.push(DrawCommand::ClipPath {
                path: path.clone(),
                anti_alias: do_anti_alias,
            });
        }
    }

    // =========================================================================
    // Raster-backing helper
    // =========================================================================

    /// Build a configured [`Rasterizer`] from the current matrix and clip
    /// when the backing is [`Backing::Raster`]. Returns `None` for
    /// non-raster backings.
    ///
    /// The closure runs with `&mut Rasterizer`; its return value is
    /// propagated.
    #[inline]
    fn with_rasterizer<R>(&mut self, f: impl FnOnce(&mut Rasterizer<'_>) -> R) -> Option<R> {
        let matrix = *self.matrix_stack.last().unwrap();
        let clip = self.clip_stack.last().copied().unwrap_or(Rect::EMPTY);
        if let Backing::Raster(buffer) = &mut self.backing {
            let mut raster = Rasterizer::new(buffer);
            raster.set_matrix(&matrix);
            raster.set_clip(clip);
            Some(f(&mut raster))
        } else {
            None
        }
    }

    // =========================================================================
    // Draw methods
    // =========================================================================

    /// Clear the canvas with a color.
    pub fn clear(&mut self, color: Color) {
        match &mut self.backing {
            Backing::Raster(buffer) => buffer.clear(color),
            Backing::Recording(commands) => commands.push(DrawCommand::Clear { color }),
            Backing::Null => {}
        }
    }

    /// Draw a color.
    pub fn draw_color(&mut self, color: Color, blend_mode: skia_rs_paint::BlendMode) {
        match &mut self.backing {
            Backing::Raster(_) => {
                let width = self.width;
                let height = self.height;
                self.with_rasterizer(|raster| {
                    let mut paint = Paint::new();
                    paint.set_color32(color);
                    paint.set_blend_mode(blend_mode);
                    let rect = Rect::from_xywh(0.0, 0.0, width as Scalar, height as Scalar);
                    raster.fill_rect(&rect, &paint);
                });
            }
            Backing::Recording(commands) => {
                commands.push(DrawCommand::DrawColor { color, blend_mode });
            }
            Backing::Null => {}
        }
    }

    /// Draw a point.
    pub fn draw_point(&mut self, point: Point, paint: &Paint) {
        match &mut self.backing {
            Backing::Raster(_) => {
                self.with_rasterizer(|raster| raster.draw_point(point, paint));
            }
            Backing::Recording(commands) => {
                commands.push(DrawCommand::DrawPoint {
                    point,
                    paint: paint.clone(),
                });
            }
            Backing::Null => {}
        }
    }

    /// Draw points.
    ///
    /// Full `PointMode` dispatch is tracked by P5-3. This currently draws
    /// each point individually (equivalent to `PointMode::Points`).
    pub fn draw_points(&mut self, mode: PointMode, points: &[Point], paint: &Paint) {
        let _ = mode; // TODO(P5-3): dispatch on PointMode::Lines/Polygon.
        for p in points {
            self.draw_point(*p, paint);
        }
    }

    /// Draw a line.
    pub fn draw_line(&mut self, p0: Point, p1: Point, paint: &Paint) {
        match &mut self.backing {
            Backing::Raster(_) => {
                self.with_rasterizer(|raster| raster.draw_line(p0, p1, paint));
            }
            Backing::Recording(commands) => {
                commands.push(DrawCommand::DrawLine {
                    p0,
                    p1,
                    paint: paint.clone(),
                });
            }
            Backing::Null => {}
        }
    }

    /// Draw a rectangle.
    pub fn draw_rect(&mut self, rect: &Rect, paint: &Paint) {
        match &mut self.backing {
            Backing::Raster(_) => {
                self.with_rasterizer(|raster| raster.draw_rect(rect, paint));
            }
            Backing::Recording(commands) => {
                commands.push(DrawCommand::DrawRect {
                    rect: *rect,
                    paint: paint.clone(),
                });
            }
            Backing::Null => {}
        }
    }

    /// Draw an oval.
    pub fn draw_oval(&mut self, rect: &Rect, paint: &Paint) {
        match &mut self.backing {
            Backing::Raster(_) => {
                self.with_rasterizer(|raster| raster.draw_oval(rect, paint));
            }
            Backing::Recording(commands) => {
                commands.push(DrawCommand::DrawOval {
                    rect: *rect,
                    paint: paint.clone(),
                });
            }
            Backing::Null => {}
        }
    }

    /// Draw a circle.
    pub fn draw_circle(&mut self, center: Point, radius: Scalar, paint: &Paint) {
        match &mut self.backing {
            Backing::Raster(_) => {
                self.with_rasterizer(|raster| raster.draw_circle(center, radius, paint));
            }
            Backing::Recording(commands) => {
                commands.push(DrawCommand::DrawCircle {
                    center,
                    radius,
                    paint: paint.clone(),
                });
            }
            Backing::Null => {}
        }
    }

    /// Draw an arc.
    pub fn draw_arc(
        &mut self,
        oval: &Rect,
        start_angle: Scalar,
        sweep_angle: Scalar,
        use_center: bool,
        paint: &Paint,
    ) {
        match &mut self.backing {
            Backing::Raster(_) => {
                // Build the arc path here so recording stays authoritative
                // and the raster path matches the former RasterCanvas
                // implementation.
                let path = build_arc_path(oval, start_angle, sweep_angle, use_center);
                self.with_rasterizer(|raster| raster.draw_path(&path, paint));
            }
            Backing::Recording(commands) => {
                commands.push(DrawCommand::DrawArc {
                    oval: *oval,
                    start_angle,
                    sweep_angle,
                    use_center,
                    paint: paint.clone(),
                });
            }
            Backing::Null => {}
        }
    }

    /// Draw a rounded rectangle.
    pub fn draw_round_rect(&mut self, rect: &Rect, rx: Scalar, ry: Scalar, paint: &Paint) {
        match &mut self.backing {
            Backing::Raster(_) => {
                let path = build_round_rect_path(rect, rx, ry);
                self.with_rasterizer(|raster| raster.draw_path(&path, paint));
            }
            Backing::Recording(commands) => {
                commands.push(DrawCommand::DrawRoundRect {
                    rect: *rect,
                    rx,
                    ry,
                    paint: paint.clone(),
                });
            }
            Backing::Null => {}
        }
    }

    /// Draw a path.
    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        match &mut self.backing {
            Backing::Raster(_) => {
                self.with_rasterizer(|raster| raster.draw_path(path, paint));
            }
            Backing::Recording(commands) => {
                commands.push(DrawCommand::DrawPath {
                    path: path.clone(),
                    paint: paint.clone(),
                });
            }
            Backing::Null => {}
        }
    }

    /// Draw a picture.
    pub fn draw_picture(
        &mut self,
        picture: &crate::Picture,
        matrix: Option<&Matrix>,
        _paint: Option<&Paint>,
    ) {
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
    /// Tracked by P5-9 (GAP-T4). Currently a no-op placeholder.
    pub fn draw_image_lattice(
        &mut self,
        _image_bounds: &Rect,
        _lattice: &ImageLattice,
        _dst: &Rect,
        _filter_mode: FilterMode,
        _paint: Option<&Paint>,
    ) {
    }

    /// Draw multiple images from an atlas.
    ///
    /// Tracked by P5-9 (GAP-T4). Currently a no-op placeholder.
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
    }

    /// Draw a Coons patch.
    ///
    /// Tracked by P5-9 (GAP-T4). Currently a no-op placeholder.
    pub fn draw_patch(
        &mut self,
        _cubic_points: &[Point; 12],
        _colors: Option<&[Color; 4]>,
        _tex_coords: Option<&[Point; 4]>,
        _blend_mode: skia_rs_paint::BlendMode,
        _paint: &Paint,
    ) {
    }

    /// Draw an annotation.
    ///
    /// Annotations are used for PDF output (links, names, etc.). Currently a
    /// placeholder; the full implementation is tracked by P5-9.
    pub fn draw_annotation(&mut self, rect: &Rect, key: &str, value: &[u8]) {
        let _ = (rect, key, value);
    }

    // =========================================================================
    // Text Drawing
    // =========================================================================

    /// Draw glyphs at specified positions.
    ///
    /// Tracked by P5-10. Currently a no-op placeholder.
    #[cfg(feature = "text")]
    pub fn draw_glyphs(
        &mut self,
        _glyph_ids: &[u16],
        _positions: &[Point],
        _origin: Point,
        _font: &skia_rs_text::Font,
        _paint: &Paint,
    ) {
    }

    /// Draw positioned text with alignment.
    ///
    /// Tracked by P5-10.
    #[cfg(feature = "text")]
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
    ///
    /// Tracked by P5-10. Currently a no-op placeholder.
    #[cfg(feature = "text")]
    pub fn draw_string(
        &mut self,
        _text: &str,
        _x: Scalar,
        _y: Scalar,
        _font: &skia_rs_text::Font,
        _paint: &Paint,
    ) {
    }

    /// Flush any pending operations.
    ///
    /// Raster and Null backings perform every draw eagerly so `flush` is a
    /// no-op. Future GPU backings will drain the command queue here. Tracked
    /// by P5-5.
    pub fn flush(&mut self) {}
}

// =============================================================================
// Auxiliary raster-only methods.
//
// These mirror the surface previously exposed through `RasterCanvas` but are
// not part of the unified `DrawCommand` set yet. They no-op on Recording /
// Null backings; adding recording variants is tracked by P5-9 / P5-6.
// =============================================================================

impl<'a> Canvas<'a> {
    /// Draw an image at the specified position.
    #[cfg(feature = "codec")]
    pub fn draw_image(
        &mut self,
        image: &skia_rs_codec::Image,
        left: Scalar,
        top: Scalar,
        paint: Option<&Paint>,
    ) {
        let src_rect = skia_rs_core::IRect::new(0, 0, image.width(), image.height());
        let dst_rect =
            Rect::from_xywh(left, top, image.width() as Scalar, image.height() as Scalar);
        self.draw_image_rect(image, Some(&src_rect), &dst_rect, paint);
    }

    /// Draw an image with source and destination rectangles.
    #[cfg(feature = "codec")]
    pub fn draw_image_rect(
        &mut self,
        image: &skia_rs_codec::Image,
        src: Option<&skia_rs_core::IRect>,
        dst: &Rect,
        paint: Option<&Paint>,
    ) {
        let src_rect = src
            .cloned()
            .unwrap_or_else(|| skia_rs_core::IRect::new(0, 0, image.width(), image.height()));

        let matrix = *self.total_matrix();
        let clip = self.clip_bounds();
        let transformed_dst = matrix.map_rect(dst);
        let visible_dst = match transformed_dst.intersect(&clip) {
            Some(r) => r,
            None => return,
        };

        let scale_x = (src_rect.width() as Scalar) / dst.width();
        let scale_y = (src_rect.height() as Scalar) / dst.height();

        let blend_mode = paint
            .map(|p| p.blend_mode())
            .unwrap_or(skia_rs_paint::BlendMode::SrcOver);
        let alpha = paint.map(|p| p.alpha()).unwrap_or(1.0);

        let dst_x_start = visible_dst.left.floor() as i32;
        let dst_x_end = visible_dst.right.ceil() as i32;
        let dst_y_start = visible_dst.top.floor() as i32;
        let dst_y_end = visible_dst.bottom.ceil() as i32;

        let buffer = match &mut self.backing {
            Backing::Raster(b) => b,
            _ => return,
        };

        for dst_y in dst_y_start..dst_y_end {
            for dst_x in dst_x_start..dst_x_end {
                let rel_x = (dst_x as Scalar - transformed_dst.left) * scale_x;
                let rel_y = (dst_y as Scalar - transformed_dst.top) * scale_y;

                let src_x = (src_rect.left as Scalar + rel_x) as i32;
                let src_y = (src_rect.top as Scalar + rel_y) as i32;

                if src_x < 0 || src_x >= image.width() || src_y < 0 || src_y >= image.height() {
                    continue;
                }

                if let Some(src_color) = image.read_pixel(src_x, src_y) {
                    let mut color = Color::from_argb(
                        (src_color.a * alpha * 255.0) as u8,
                        (src_color.r * 255.0) as u8,
                        (src_color.g * 255.0) as u8,
                        (src_color.b * 255.0) as u8,
                    );

                    if alpha < 1.0 {
                        let a = (color.alpha() as f32 * alpha) as u8;
                        color = Color::from_argb(a, color.red(), color.green(), color.blue());
                    }

                    buffer.blend_pixel(dst_x, dst_y, color, blend_mode);
                }
            }
        }
    }

    /// Draw an image with nine-patch stretching.
    #[cfg(feature = "codec")]
    pub fn draw_image_nine(
        &mut self,
        image: &skia_rs_codec::Image,
        center: &skia_rs_core::IRect,
        dst: &Rect,
        paint: Option<&Paint>,
    ) {
        let img_w = image.width();
        let img_h = image.height();

        let left_w = center.left as Scalar;
        let right_w = (img_w - center.right) as Scalar;
        let top_h = center.top as Scalar;
        let bottom_h = (img_h - center.bottom) as Scalar;

        let center_w = dst.width() - left_w - right_w;
        let center_h = dst.height() - top_h - bottom_h;

        self.draw_image_rect(
            image,
            Some(&skia_rs_core::IRect::new(0, 0, center.left, center.top)),
            &Rect::from_xywh(dst.left, dst.top, left_w, top_h),
            paint,
        );
        self.draw_image_rect(
            image,
            Some(&skia_rs_core::IRect::new(
                center.left,
                0,
                center.right,
                center.top,
            )),
            &Rect::from_xywh(dst.left + left_w, dst.top, center_w, top_h),
            paint,
        );
        self.draw_image_rect(
            image,
            Some(&skia_rs_core::IRect::new(center.right, 0, img_w, center.top)),
            &Rect::from_xywh(dst.right - right_w, dst.top, right_w, top_h),
            paint,
        );
        self.draw_image_rect(
            image,
            Some(&skia_rs_core::IRect::new(
                0,
                center.top,
                center.left,
                center.bottom,
            )),
            &Rect::from_xywh(dst.left, dst.top + top_h, left_w, center_h),
            paint,
        );
        self.draw_image_rect(
            image,
            Some(&skia_rs_core::IRect::new(
                center.left,
                center.top,
                center.right,
                center.bottom,
            )),
            &Rect::from_xywh(dst.left + left_w, dst.top + top_h, center_w, center_h),
            paint,
        );
        self.draw_image_rect(
            image,
            Some(&skia_rs_core::IRect::new(
                center.right,
                center.top,
                img_w,
                center.bottom,
            )),
            &Rect::from_xywh(dst.right - right_w, dst.top + top_h, right_w, center_h),
            paint,
        );
        self.draw_image_rect(
            image,
            Some(&skia_rs_core::IRect::new(
                0,
                center.bottom,
                center.left,
                img_h,
            )),
            &Rect::from_xywh(dst.left, dst.bottom - bottom_h, left_w, bottom_h),
            paint,
        );
        self.draw_image_rect(
            image,
            Some(&skia_rs_core::IRect::new(
                center.left,
                center.bottom,
                center.right,
                img_h,
            )),
            &Rect::from_xywh(dst.left + left_w, dst.bottom - bottom_h, center_w, bottom_h),
            paint,
        );
        self.draw_image_rect(
            image,
            Some(&skia_rs_core::IRect::new(
                center.right,
                center.bottom,
                img_w,
                img_h,
            )),
            &Rect::from_xywh(
                dst.right - right_w,
                dst.bottom - bottom_h,
                right_w,
                bottom_h,
            ),
            paint,
        );
    }

    /// Draw a region.
    pub fn draw_region(&mut self, region: &skia_rs_core::Region, paint: &Paint) {
        for rect in region.iter() {
            let rect_f = rect.to_rect();
            self.draw_rect(&rect_f, paint);
        }
    }

    /// Draw vertices (triangles).
    pub fn draw_vertices(
        &mut self,
        mode: VertexMode,
        positions: &[Point],
        colors: Option<&[Color]>,
        paint: &Paint,
    ) {
        if positions.len() < 3 {
            return;
        }

        let matrix = *self.total_matrix();

        match mode {
            VertexMode::Triangles => {
                for chunk in positions.chunks(3) {
                    if chunk.len() == 3 {
                        self.draw_triangle(
                            matrix.map_point(chunk[0]),
                            matrix.map_point(chunk[1]),
                            matrix.map_point(chunk[2]),
                            colors.and_then(|c| c.first().copied()),
                            paint,
                        );
                    }
                }
            }
            VertexMode::TriangleStrip => {
                for i in 0..positions.len().saturating_sub(2) {
                    let (p0, p1, p2) = if i % 2 == 0 {
                        (positions[i], positions[i + 1], positions[i + 2])
                    } else {
                        (positions[i + 1], positions[i], positions[i + 2])
                    };
                    self.draw_triangle(
                        matrix.map_point(p0),
                        matrix.map_point(p1),
                        matrix.map_point(p2),
                        colors.and_then(|c| c.get(i).copied()),
                        paint,
                    );
                }
            }
            VertexMode::TriangleFan => {
                let center = positions[0];
                for i in 1..positions.len().saturating_sub(1) {
                    self.draw_triangle(
                        matrix.map_point(center),
                        matrix.map_point(positions[i]),
                        matrix.map_point(positions[i + 1]),
                        colors.and_then(|c| c.get(i).copied()),
                        paint,
                    );
                }
            }
        }
    }

    /// Draw a single filled triangle.
    fn draw_triangle(
        &mut self,
        p0: Point,
        p1: Point,
        p2: Point,
        color: Option<Color>,
        paint: &Paint,
    ) {
        let buffer = match &mut self.backing {
            Backing::Raster(b) => b,
            _ => return,
        };

        let color = color.unwrap_or_else(|| paint.color32());
        let blend_mode = paint.blend_mode();

        let mut verts = [(p0.x, p0.y), (p1.x, p1.y), (p2.x, p2.y)];
        verts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let (x0, y0) = verts[0];
        let (x1, y1) = verts[1];
        let (x2, y2) = verts[2];

        let inv_slope_02 = if (y2 - y0).abs() > 0.001 {
            (x2 - x0) / (y2 - y0)
        } else {
            0.0
        };
        let inv_slope_01 = if (y1 - y0).abs() > 0.001 {
            (x1 - x0) / (y1 - y0)
        } else {
            0.0
        };
        let inv_slope_12 = if (y2 - y1).abs() > 0.001 {
            (x2 - x1) / (y2 - y1)
        } else {
            0.0
        };

        let y_start = y0.ceil() as i32;
        let y_mid = y1.ceil() as i32;
        let y_end = y2.ceil() as i32;

        for y in y_start..y_mid {
            let x_left = x0 + (y as Scalar - y0) * inv_slope_02;
            let x_right = x0 + (y as Scalar - y0) * inv_slope_01;

            let (xa, xb) = if x_left < x_right {
                (x_left, x_right)
            } else {
                (x_right, x_left)
            };

            for x in (xa.ceil() as i32)..(xb.floor() as i32) {
                buffer.blend_pixel(x, y, color, blend_mode);
            }
        }

        for y in y_mid..y_end {
            let x_left = x0 + (y as Scalar - y0) * inv_slope_02;
            let x_right = x1 + (y as Scalar - y1) * inv_slope_12;

            let (xa, xb) = if x_left < x_right {
                (x_left, x_right)
            } else {
                (x_right, x_left)
            };

            for x in (xa.ceil() as i32)..(xb.floor() as i32) {
                buffer.blend_pixel(x, y, color, blend_mode);
            }
        }
    }

    /// Draw a text blob.
    ///
    /// Placeholder implementation; full glyph rendering is tracked by P5-10.
    #[cfg(feature = "text")]
    pub fn draw_text_blob(
        &mut self,
        blob: &skia_rs_text::TextBlob,
        x: Scalar,
        y: Scalar,
        paint: &Paint,
    ) {
        let matrix = *self.total_matrix();
        let clip = self.clip_bounds();
        let color = paint.color32();
        let blend_mode = paint.blend_mode();

        let buffer = match &mut self.backing {
            Backing::Raster(b) => b,
            _ => return,
        };

        for run in blob.runs() {
            let font = &run.font;
            let char_width = font.size() * 0.5;
            let char_height = font.size();

            for (i, &glyph) in run.glyphs.iter().enumerate() {
                if glyph == 0 {
                    continue;
                }

                let pos = if i < run.positions.len() {
                    run.positions[i]
                } else {
                    Point::new(i as Scalar * char_width, 0.0)
                };

                let world_pos = matrix.map_point(Point::new(
                    x + run.origin.x + pos.x,
                    y + run.origin.y + pos.y - char_height * 0.8,
                ));

                let rect = Rect::from_xywh(
                    world_pos.x,
                    world_pos.y,
                    char_width * matrix.scale_x().abs(),
                    char_height * matrix.scale_y().abs(),
                );

                if let Some(clipped) = rect.intersect(&clip) {
                    let r = clipped.round_out();
                    for py in r.top..r.bottom {
                        for px in r.left..r.right {
                            buffer.blend_pixel(px, py, color, blend_mode);
                        }
                    }
                }
            }
        }
    }
}

/// Vertex drawing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum VertexMode {
    /// Separate triangles (every 3 vertices).
    #[default]
    Triangles = 0,
    /// Triangle strip (shared edges).
    TriangleStrip,
    /// Triangle fan (shared center vertex).
    TriangleFan,
}

// =============================================================================
// Arc / Round rect path construction shared with PictureRecorder playback.
// =============================================================================

fn build_round_rect_path(rect: &Rect, rx: Scalar, ry: Scalar) -> Path {
    use skia_rs_path::PathBuilder;
    let mut builder = PathBuilder::new();
    builder.move_to(rect.left + rx, rect.top);
    builder.line_to(rect.right - rx, rect.top);
    builder.quad_to(rect.right, rect.top, rect.right, rect.top + ry);
    builder.line_to(rect.right, rect.bottom - ry);
    builder.quad_to(rect.right, rect.bottom, rect.right - rx, rect.bottom);
    builder.line_to(rect.left + rx, rect.bottom);
    builder.quad_to(rect.left, rect.bottom, rect.left, rect.bottom - ry);
    builder.line_to(rect.left, rect.top + ry);
    builder.quad_to(rect.left, rect.top, rect.left + rx, rect.top);
    builder.close();
    builder.build()
}

fn build_arc_path(
    oval: &Rect,
    start_angle: Scalar,
    sweep_angle: Scalar,
    use_center: bool,
) -> Path {
    use skia_rs_path::PathBuilder;

    let center = Point::new(
        (oval.left + oval.right) / 2.0,
        (oval.top + oval.bottom) / 2.0,
    );
    let rx = oval.width() / 2.0;
    let ry = oval.height() / 2.0;

    let start_rad = start_angle.to_radians();
    let end_rad = (start_angle + sweep_angle).to_radians();

    let start_x = center.x + rx * start_rad.cos();
    let start_y = center.y + ry * start_rad.sin();

    let mut builder = PathBuilder::new();

    if use_center {
        builder.move_to(center.x, center.y);
        builder.line_to(start_x, start_y);
    } else {
        builder.move_to(start_x, start_y);
    }

    let steps = ((sweep_angle.abs() / 10.0).ceil() as usize).max(4);
    for i in 1..=steps {
        let t = i as Scalar / steps as Scalar;
        let angle = start_rad + (end_rad - start_rad) * t;
        let x = center.x + rx * angle.cos();
        let y = center.y + ry * angle.sin();
        builder.line_to(x, y);
    }

    if use_center {
        builder.close();
    }

    builder.build()
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
    pub fn from_radians(
        scale: Scalar,
        radians: Scalar,
        tx: Scalar,
        ty: Scalar,
        ax: Scalar,
        ay: Scalar,
    ) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use skia_rs_core::{Color, Rect};

    #[test]
    fn test_null_canvas_discards_draws() {
        // Null canvas must still maintain matrix/clip state and answer
        // quick-reject queries, but should never crash on draw calls.
        let mut c = Canvas::new_null(100, 100);
        c.translate(10.0, 10.0);
        c.save();
        c.clear(Color::from_argb(255, 255, 0, 0));
        let paint = Paint::new();
        c.draw_rect(&Rect::from_xywh(0.0, 0.0, 50.0, 50.0), &paint);
        c.restore();
        assert_eq!(c.save_count(), 1);
    }

    #[test]
    fn test_recording_backing_captures_commands() {
        let mut cmds: Vec<DrawCommand> = Vec::new();
        {
            let mut c = Canvas::new_recording(&mut cmds, 100, 100);
            c.save();
            c.translate(5.0, 5.0);
            let paint = Paint::new();
            c.draw_rect(&Rect::from_xywh(0.0, 0.0, 10.0, 10.0), &paint);
            c.restore();
        }
        // save, translate, draw_rect, restore
        assert_eq!(cmds.len(), 4);
        assert!(matches!(cmds[0], DrawCommand::Save));
        assert!(matches!(cmds[1], DrawCommand::Translate { .. }));
        assert!(matches!(cmds[2], DrawCommand::DrawRect { .. }));
        assert!(matches!(cmds[3], DrawCommand::Restore));
    }

    #[test]
    fn test_quick_reject_respects_clip_and_matrix() {
        let mut c = Canvas::new_null(100, 100);
        assert!(!c.quick_reject(&Rect::from_xywh(10.0, 10.0, 20.0, 20.0)));
        c.translate(200.0, 200.0);
        assert!(c.quick_reject(&Rect::from_xywh(0.0, 0.0, 10.0, 10.0)));
    }
}
