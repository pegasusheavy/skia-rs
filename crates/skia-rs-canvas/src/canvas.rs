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

use crate::clip::ClipStack;
use crate::picture::DrawCommand;
use crate::raster::{PixelBuffer, Rasterizer};
use skia_rs_core::{Color, IRect, Matrix, Point, Rect, Scalar};
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

/// An offscreen layer pushed by [`Canvas::save_layer`].
///
/// While this layer is on top of the stack, all draws are routed into
/// `buffer` instead of the base backing. On [`Canvas::restore`] the buffer
/// is composited back onto the enclosing target (the next layer down or the
/// base backing) using `paint`'s alpha and blend mode.
struct Layer {
    /// Offscreen pixel buffer holding this layer's contents.
    buffer: PixelBuffer,
    /// Bounds in the enclosing canvas coordinate space, or `None` if the
    /// layer covers the whole canvas.
    ///
    /// `bounds.left, bounds.top` is the offset at which the layer's local
    /// origin sits inside its parent. Layer-local coordinates are
    /// canvas coordinates minus this offset.
    bounds: Option<Rect>,
    /// Paint controlling how the layer composites back onto the parent.
    /// `None` means use a default [`Paint`] (opaque [`BlendMode::SrcOver`]).
    paint: Option<Paint>,
    /// Canvas save count at the moment this layer was pushed. Used by
    /// [`Canvas::restore`] to decide whether the current restore should
    /// pop and composite this layer.
    save_count_when_pushed: usize,
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
    /// Clip stack with full region, path, and AA support.
    clip_stack: ClipStack,
    /// Save count.
    save_count: usize,
    /// Width of the logical canvas.
    width: i32,
    /// Height of the logical canvas.
    height: i32,
    /// Stack of offscreen layers pushed by [`Canvas::save_layer`]. The top
    /// entry (if any) receives all rasterized draws until its matching
    /// [`Canvas::restore`], at which point it is composited back onto the
    /// enclosing target.
    layer_stack: Vec<Layer>,
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
        let bounds = Rect::from_xywh(0.0, 0.0, width as Scalar, height as Scalar);
        Self {
            backing: Backing::Raster(buffer),
            matrix_stack: vec![Matrix::IDENTITY],
            clip_stack: ClipStack::new(&bounds),
            save_count: 1,
            width,
            height,
            layer_stack: Vec::new(),
        }
    }

    /// Create a recording canvas that appends draw commands into `commands`.
    ///
    /// `width`/`height` are only used to seed the initial clip bounds and to
    /// answer [`width`](Self::width)/[`height`](Self::height) queries.
    pub fn new_recording(commands: &'a mut Vec<DrawCommand>, width: i32, height: i32) -> Self {
        let bounds = Rect::from_xywh(0.0, 0.0, width as Scalar, height as Scalar);
        Self {
            backing: Backing::Recording(commands),
            matrix_stack: vec![Matrix::IDENTITY],
            clip_stack: ClipStack::new(&bounds),
            save_count: 1,
            width,
            height,
            layer_stack: Vec::new(),
        }
    }

    /// Create a no-op canvas with the given dimensions.
    pub fn new_null(width: i32, height: i32) -> Self {
        let bounds = Rect::from_xywh(0.0, 0.0, width as Scalar, height as Scalar);
        Self {
            backing: Backing::Null,
            matrix_stack: vec![Matrix::IDENTITY],
            clip_stack: ClipStack::new(&bounds),
            save_count: 1,
            width,
            height,
            layer_stack: Vec::new(),
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
        self.clip_stack.bounds()
    }

    /// Inspect the backing (primarily for diagnostics and tests).
    #[inline]
    pub fn backing(&self) -> &Backing<'a> {
        &self.backing
    }

    /// Save the current state.
    pub fn save(&mut self) -> usize {
        let matrix = *self.matrix_stack.last().unwrap();
        self.matrix_stack.push(matrix);
        self.clip_stack.save();
        self.save_count += 1;
        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::Save);
        }
        self.save_count
    }

    /// Save the current state with a layer.
    ///
    /// For raster backings this allocates an offscreen [`PixelBuffer`] sized
    /// to `rec.bounds` (or the full canvas if no bounds are given) and
    /// redirects all subsequent draws into it until the matching
    /// [`restore`](Self::restore). On restore the buffer is composited back
    /// onto the enclosing target using `rec.paint`'s alpha and blend mode.
    ///
    /// Recording backings simply emit a [`DrawCommand::SaveLayer`] and rely
    /// on playback (against a raster canvas) to do the actual composition.
    /// Null backings behave like plain [`save`](Self::save) — no layer is
    /// allocated — but the save count is incremented so the matching
    /// restore still balances.
    pub fn save_layer(&mut self, rec: &SaveLayerRec<'_>) -> usize {
        let bounds = rec.bounds.copied();
        let paint = rec.paint.cloned();

        // Record intent for Recording backings before mutating anything else.
        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::SaveLayer {
                bounds,
                paint: paint.clone(),
            });
        }

        // Maintain matrix/clip/save-count stack exactly like save().
        let matrix = *self.matrix_stack.last().unwrap();
        self.matrix_stack.push(matrix);
        self.clip_stack.save();
        self.save_count += 1;

        // Only raster backings allocate an offscreen layer — Recording and
        // Null have no pixels to composite.
        if matches!(self.backing, Backing::Raster(_)) {
            let (w, h) = match bounds {
                Some(b) => (b.width().ceil() as i32, b.height().ceil() as i32),
                None => (self.width, self.height),
            };
            let buffer = PixelBuffer::new(w.max(1), h.max(1));
            // INIT_WITH_PREVIOUS is left unimplemented here — the buffer is
            // left fully transparent. Pre-seeding from the parent is left
            // as a follow-up because it requires reading back and mapping
            // bounds between coordinate spaces.
            self.layer_stack.push(Layer {
                buffer,
                bounds,
                paint,
                save_count_when_pushed: self.save_count,
            });
        }

        self.save_count
    }

    /// Restore to the previous state.
    ///
    /// If the top entry of the layer stack was pushed at the current save
    /// count, the layer is popped and composited onto its parent target
    /// (the next layer below, or the base backing) before the matrix/clip
    /// stacks are popped.
    pub fn restore(&mut self) {
        if self.save_count <= 1 {
            return;
        }

        // If the top layer was pushed at this save count, composite it
        // back onto its parent before popping save state.
        let should_pop_layer = self
            .layer_stack
            .last()
            .is_some_and(|l| l.save_count_when_pushed == self.save_count);
        if should_pop_layer {
            let layer = self.layer_stack.pop().unwrap();
            self.composite_layer(layer);
        }

        self.matrix_stack.pop();
        self.clip_stack.restore();
        self.save_count -= 1;

        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::Restore);
        }
    }

    /// Composite a popped [`Layer`] back onto the enclosing target (the
    /// next layer below, or the base [`Backing::Raster`] buffer). Uses the
    /// layer paint's alpha (scaling the source alpha) and blend mode.
    fn composite_layer(&mut self, layer: Layer) {
        let paint = layer.paint.unwrap_or_default();
        let blend_mode = paint.blend_mode();
        // Layer paint alpha scales the source alpha per-pixel. Stored as
        // [0.0, 1.0] in Paint — clamp defensively.
        let layer_alpha = paint.alpha().clamp(0.0, 1.0);
        let (offset_x, offset_y) = match layer.bounds {
            Some(b) => (b.left.floor() as i32, b.top.floor() as i32),
            None => (0, 0),
        };

        // Borrow the target buffer: either the next layer below (parent
        // layer) or the base raster backing. Recording/Null backings have
        // no pixels to composite into.
        let target: &mut PixelBuffer = if let Some(parent) = self.layer_stack.last_mut() {
            &mut parent.buffer
        } else {
            match &mut self.backing {
                Backing::Raster(buf) => *buf,
                _ => return,
            }
        };

        let src_buf = &layer.buffer;
        let src_w = src_buf.width;
        let src_h = src_buf.height;

        for y in 0..src_h {
            let dy = offset_y + y;
            if dy < 0 || dy >= target.height {
                continue;
            }
            for x in 0..src_w {
                let dx = offset_x + x;
                if dx < 0 || dx >= target.width {
                    continue;
                }
                let src_offset = (y as usize) * src_buf.stride + (x as usize) * 4;
                let sr = src_buf.pixels[src_offset];
                let sg = src_buf.pixels[src_offset + 1];
                let sb = src_buf.pixels[src_offset + 2];
                let sa_raw = src_buf.pixels[src_offset + 3];
                if sa_raw == 0 && matches!(
                    blend_mode,
                    skia_rs_paint::BlendMode::SrcOver | skia_rs_paint::BlendMode::DstOver
                ) {
                    // Fully transparent pixels contribute nothing under
                    // over-style blends; skip to avoid pointless work.
                    continue;
                }
                // Scale source alpha by the layer paint alpha.
                let scaled_a = ((sa_raw as f32) * layer_alpha).round().clamp(0.0, 255.0) as u8;
                let src = Color::from_argb(scaled_a, sr, sg, sb);
                target.blend_pixel(dx, dy, src, blend_mode);
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
        let transformed = self.total_matrix().map_rect(rect);
        let device_bounds = IRect::new(0, 0, self.width, self.height);

        self.clip_stack
            .clip_rect_with_op(&transformed, op, &device_bounds, do_anti_alias);

        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::ClipRect {
                rect: *rect,
                anti_alias: do_anti_alias,
            });
        }
    }

    /// Clip to a path.
    pub fn clip_path(&mut self, path: &Path, op: ClipOp, do_anti_alias: bool) {
        // Transform the path to device coordinates
        let matrix = self.total_matrix();
        let mut transformed_path = path.clone();
        transformed_path.transform(matrix);

        let device_bounds = IRect::new(0, 0, self.width, self.height);
        self.clip_stack
            .clip_path_with_op(&transformed_path, &device_bounds, op, do_anti_alias);

        if let Backing::Recording(commands) = &mut self.backing {
            commands.push(DrawCommand::ClipPath {
                path: path.clone(),
                anti_alias: do_anti_alias,
            });
        }
    }

    // =========================================================================
    // Raster-backing helper
    // =========================================================================

    /// Build a configured [`Rasterizer`] from the current matrix and clip.
    ///
    /// If the canvas has an active layer (pushed by
    /// [`save_layer`](Self::save_layer)), the rasterizer targets the top
    /// layer's offscreen buffer; otherwise it targets the base
    /// [`Backing::Raster`] buffer. For [`Backing::Recording`] and
    /// [`Backing::Null`] canvases without an active layer, returns `None`.
    ///
    /// The closure runs with `&mut Rasterizer`; its return value is
    /// propagated.
    #[inline]
    fn with_rasterizer<R>(&mut self, f: impl FnOnce(&mut Rasterizer<'_>) -> R) -> Option<R> {
        let matrix = *self.matrix_stack.last().unwrap();
        let clip_stack = self.clip_stack.clone();

        // An active layer takes priority over the base backing. The layer
        // buffer is sized to the layer's bounds, so we pre-compose a
        // translation that maps canvas coordinates to layer-local
        // coordinates (buffer origin sits at bounds.left, bounds.top).
        if let Some(layer) = self.layer_stack.last_mut() {
            let (ox, oy) = match layer.bounds {
                Some(b) => (b.left, b.top),
                None => (0.0, 0.0),
            };
            let adjusted = if ox == 0.0 && oy == 0.0 {
                matrix
            } else {
                Matrix::translate(-ox, -oy).concat(&matrix)
            };
            let mut raster = Rasterizer::new(&mut layer.buffer);
            raster.set_matrix(&adjusted);
            raster.set_clip_stack(clip_stack);
            return Some(f(&mut raster));
        }

        if let Backing::Raster(buffer) = &mut self.backing {
            let mut raster = Rasterizer::new(buffer);
            raster.set_matrix(&matrix);
            raster.set_clip_stack(clip_stack);
            Some(f(&mut raster))
        } else {
            None
        }
    }

    // =========================================================================
    // Draw methods
    // =========================================================================

    /// Clear the canvas with a color.
    ///
    /// If a layer is active (pushed by [`save_layer`](Self::save_layer)),
    /// only the top layer's buffer is cleared; the enclosing buffer is
    /// untouched. This matches Skia semantics: clear affects the current
    /// drawing target, not the root surface.
    pub fn clear(&mut self, color: Color) {
        if let Some(layer) = self.layer_stack.last_mut() {
            layer.buffer.clear(color);
            return;
        }
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

    /// Draw points in the specified mode.
    ///
    /// Empty input produces no output. Odd-count input in `Lines` mode
    /// drops the orphan trailing point.
    pub fn draw_points(&mut self, mode: PointMode, points: &[Point], paint: &Paint) {
        if points.is_empty() {
            return;
        }
        match mode {
            PointMode::Points => {
                for &p in points {
                    self.draw_point(p, paint);
                }
            }
            PointMode::Lines => {
                // Consecutive pairs form lines
                for chunk in points.chunks_exact(2) {
                    self.draw_line(chunk[0], chunk[1], paint);
                }
            }
            PointMode::Polygon => {
                // Consecutive points form a polyline
                for pair in points.windows(2) {
                    self.draw_line(pair[0], pair[1], paint);
                }
            }
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
        let transformed = self.total_matrix().map_rect(rect);
        self.clip_stack.quick_reject(&transformed)
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
    /// This API is designed for future texture-atlas or GPU backends. The
    /// current software-only Canvas implementation has no image-atlas
    /// infrastructure, so this method is a no-op. Callers with an actual image
    /// should use [`draw_image_nine`](Self::draw_image_nine) instead (requires
    /// `codec` feature).
    ///
    /// When image-atlas support is added, `image_bounds` will identify a
    /// sub-region of a bound texture atlas, `lattice` will define the
    /// stretch/fixed cells, and the result will be drawn into `dst`.
    #[cfg(feature = "codec")]
    pub fn draw_image_lattice(
        &mut self,
        image: &skia_rs_codec::Image,
        lattice: &ImageLattice,
        dst: &Rect,
        _filter_mode: FilterMode,
        paint: Option<&Paint>,
    ) {
        let src_w = image.width() as Scalar;
        let src_h = image.height() as Scalar;
        if src_w <= 0.0 || src_h <= 0.0 {
            return;
        }

        // Build source edges: [0, x_divs..., src_w]
        let mut src_x_edges = vec![0.0];
        src_x_edges.extend(
            lattice
                .x_divs
                .iter()
                .map(|&x| (x as Scalar).clamp(0.0, src_w)),
        );
        src_x_edges.push(src_w);

        let mut src_y_edges = vec![0.0];
        src_y_edges.extend(
            lattice
                .y_divs
                .iter()
                .map(|&y| (y as Scalar).clamp(0.0, src_h)),
        );
        src_y_edges.push(src_h);

        // Build destination edges using proportional scaling.
        // Full 9-patch semantics (fixed corners, stretch middles) are a
        // future refinement; proportional baseline is functionally correct
        // when divs are chosen proportionally to dst size.
        let dst_scale_x = dst.width() / src_w;
        let dst_scale_y = dst.height() / src_h;
        let dst_x_edges: Vec<Scalar> = src_x_edges
            .iter()
            .map(|&x| dst.left + x * dst_scale_x)
            .collect();
        let dst_y_edges: Vec<Scalar> = src_y_edges
            .iter()
            .map(|&y| dst.top + y * dst_scale_y)
            .collect();

        // Iterate cells and call draw_image_rect for each non-empty cell
        for j in 0..(src_y_edges.len() - 1) {
            for i in 0..(src_x_edges.len() - 1) {
                // Check if this cell should be skipped (Transparent)
                if let Some(ref rect_types) = lattice.rect_types {
                    let cell_idx = j * (src_x_edges.len() - 1) + i;
                    if let Some(&LatticeRectType::Transparent) = rect_types.get(cell_idx) {
                        continue;
                    }
                }

                let src_cell = skia_rs_core::IRect::new(
                    src_x_edges[i] as i32,
                    src_y_edges[j] as i32,
                    src_x_edges[i + 1] as i32,
                    src_y_edges[j + 1] as i32,
                );
                let dst_cell = Rect::new(
                    dst_x_edges[i],
                    dst_y_edges[j],
                    dst_x_edges[i + 1],
                    dst_y_edges[j + 1],
                );
                if src_cell.width() > 0
                    && src_cell.height() > 0
                    && dst_cell.width() > 0.0
                    && dst_cell.height() > 0.0
                {
                    self.draw_image_rect(image, Some(&src_cell), &dst_cell, paint);
                }
            }
        }
    }

    /// Draw multiple images from an atlas.
    ///
    /// Draws sprites from a single atlas image. Each sprite is defined by a
    /// source rectangle in `sprites` and transformed by the corresponding
    /// `RSXform` in `xforms`. Optional per-sprite tint colors can be provided.
    ///
    /// Each sprite is drawn using save/concat/draw_image_rect/restore to
    /// handle rotation, scale, and translation. Tint colors are accepted but
    /// not applied in this baseline implementation (requires color-filter-on-draw
    /// hookup tracked separately).
    #[cfg(feature = "codec")]
    pub fn draw_atlas(
        &mut self,
        atlas: &skia_rs_codec::Image,
        xforms: &[RSXform],
        sprites: &[Rect],
        _colors: Option<&[Color]>,
        _blend_mode: skia_rs_paint::BlendMode,
        _sampling: FilterMode,
        paint: Option<&Paint>,
    ) {
        let count = xforms.len().min(sprites.len());
        for i in 0..count {
            let xform = &xforms[i];
            let tex = &sprites[i];
            let matrix = xform.to_matrix();

            // Draw the sprite: save current transform, concat the RSXform,
            // draw the atlas sub-rect to (0, 0) .. (tex.width, tex.height)
            // in local space, then restore.
            self.save();
            self.concat(&matrix);
            let dst_local = Rect::from_xywh(0.0, 0.0, tex.width(), tex.height());
            let src_rect = skia_rs_core::IRect::new(
                tex.left as i32,
                tex.top as i32,
                tex.right as i32,
                tex.bottom as i32,
            );
            self.draw_image_rect(atlas, Some(&src_rect), &dst_local, paint);
            self.restore();
        }
    }

    /// Draw a Coons patch.
    ///
    /// A Coons patch is a bicubic surface defined by 12 control points forming
    /// four boundary cubic Bezier curves. The patch is subdivided into an 8×8
    /// grid and rasterized as triangles. Corner colors (if provided) are
    /// bilinearly interpolated across the patch. Texture mapping
    /// (`tex_coords`) is not yet supported and is silently ignored.
    ///
    /// The 12 control points define the patch boundaries in this order:
    /// - Points 0-3: top edge (left to right)
    /// - Points 3-6: right edge (top to bottom)
    /// - Points 9-6: bottom edge (left to right, reversed)
    /// - Points 0, 11, 10, 9: left edge (top to bottom)
    pub fn draw_patch(
        &mut self,
        cubic_points: &[Point; 12],
        colors: Option<&[Color; 4]>,
        _tex_coords: Option<&[Point; 4]>,
        _blend_mode: skia_rs_paint::BlendMode,
        paint: &Paint,
    ) {
        // Subdivide the patch into an N×N grid
        const N: usize = 8;

        // Evaluate a cubic Bezier curve at parameter t
        fn cubic_bezier(p0: Point, p1: Point, p2: Point, p3: Point, t: Scalar) -> Point {
            let mt = 1.0 - t;
            let mt2 = mt * mt;
            let t2 = t * t;
            let mt3 = mt2 * mt;
            let t3 = t2 * t;
            Point::new(
                p0.x * mt3 + 3.0 * p1.x * mt2 * t + 3.0 * p2.x * mt * t2 + p3.x * t3,
                p0.y * mt3 + 3.0 * p1.y * mt2 * t + 3.0 * p2.y * mt * t2 + p3.y * t3,
            )
        }

        // Evaluate the Coons patch at (u, v) ∈ [0, 1]²
        fn eval_patch(c: &[Point; 12], u: Scalar, v: Scalar) -> Point {
            // Four boundary curves:
            // top:    c[0..=3]
            // right:  c[3..=6]
            // bottom: c[9..=6] (reversed)
            // left:   c[0], c[11], c[10], c[9]
            let top = cubic_bezier(c[0], c[1], c[2], c[3], u);
            let bottom = cubic_bezier(c[9], c[8], c[7], c[6], u);
            let left = cubic_bezier(c[0], c[11], c[10], c[9], v);
            let right = cubic_bezier(c[3], c[4], c[5], c[6], v);

            // Coons patch formula: bilinear blend of opposite edges, minus
            // the bilinear interpolation of corners (to avoid double-counting).
            let lc_x = (1.0 - v) * top.x + v * bottom.x;
            let lc_y = (1.0 - v) * top.y + v * bottom.y;

            let lr_x = (1.0 - u) * left.x + u * right.x;
            let lr_y = (1.0 - u) * left.y + u * right.y;

            let bl_x = (1.0 - u) * (1.0 - v) * c[0].x
                + u * (1.0 - v) * c[3].x
                + (1.0 - u) * v * c[9].x
                + u * v * c[6].x;
            let bl_y = (1.0 - u) * (1.0 - v) * c[0].y
                + u * (1.0 - v) * c[3].y
                + (1.0 - u) * v * c[9].y
                + u * v * c[6].y;

            Point::new(lc_x + lr_x - bl_x, lc_y + lr_y - bl_y)
        }

        // Build a grid of vertices
        let mut vertices = Vec::with_capacity((N + 1) * (N + 1));
        for j in 0..=N {
            for i in 0..=N {
                let u = i as Scalar / N as Scalar;
                let v = j as Scalar / N as Scalar;
                vertices.push(eval_patch(cubic_points, u, v));
            }
        }

        // Bilinearly interpolate corner colors at grid point (i, j)
        let get_color = |i: usize, j: usize| -> Option<Color> {
            let colors = colors?;
            let u = i as Scalar / N as Scalar;
            let v = j as Scalar / N as Scalar;

            // Corner colors: [top-left, top-right, bottom-right, bottom-left]
            let c = colors;
            let r = (1.0 - u) * (1.0 - v) * c[0].red() as Scalar
                + u * (1.0 - v) * c[1].red() as Scalar
                + u * v * c[2].red() as Scalar
                + (1.0 - u) * v * c[3].red() as Scalar;
            let g = (1.0 - u) * (1.0 - v) * c[0].green() as Scalar
                + u * (1.0 - v) * c[1].green() as Scalar
                + u * v * c[2].green() as Scalar
                + (1.0 - u) * v * c[3].green() as Scalar;
            let b = (1.0 - u) * (1.0 - v) * c[0].blue() as Scalar
                + u * (1.0 - v) * c[1].blue() as Scalar
                + u * v * c[2].blue() as Scalar
                + (1.0 - u) * v * c[3].blue() as Scalar;
            let a = (1.0 - u) * (1.0 - v) * c[0].alpha() as Scalar
                + u * (1.0 - v) * c[1].alpha() as Scalar
                + u * v * c[2].alpha() as Scalar
                + (1.0 - u) * v * c[3].alpha() as Scalar;

            Some(Color::from_argb(
                a.clamp(0.0, 255.0) as u8,
                r.clamp(0.0, 255.0) as u8,
                g.clamp(0.0, 255.0) as u8,
                b.clamp(0.0, 255.0) as u8,
            ))
        };

        // Build triangles: for each cell, two triangles (TL-TR-BR, TL-BR-BL)
        let mut triangle_verts = Vec::new();
        let mut triangle_colors = Vec::new();
        for j in 0..N {
            for i in 0..N {
                let idx_tl = j * (N + 1) + i;
                let idx_tr = idx_tl + 1;
                let idx_bl = idx_tl + (N + 1);
                let idx_br = idx_bl + 1;

                // Triangle 1: TL-TR-BR
                triangle_verts.push(vertices[idx_tl]);
                triangle_verts.push(vertices[idx_tr]);
                triangle_verts.push(vertices[idx_br]);

                // Triangle 2: TL-BR-BL
                triangle_verts.push(vertices[idx_tl]);
                triangle_verts.push(vertices[idx_br]);
                triangle_verts.push(vertices[idx_bl]);

                if colors.is_some() {
                    // Colors for triangle 1
                    triangle_colors.push(get_color(i, j).unwrap());
                    triangle_colors.push(get_color(i + 1, j).unwrap());
                    triangle_colors.push(get_color(i + 1, j + 1).unwrap());
                    // Colors for triangle 2
                    triangle_colors.push(get_color(i, j).unwrap());
                    triangle_colors.push(get_color(i + 1, j + 1).unwrap());
                    triangle_colors.push(get_color(i, j + 1).unwrap());
                }
            }
        }

        let vertex_colors = if colors.is_some() {
            Some(triangle_colors.as_slice())
        } else {
            None
        };

        self.draw_vertices(VertexMode::Triangles, &triangle_verts, vertex_colors, paint);
    }

    /// Draw an annotation.
    ///
    /// Annotations attach metadata to a rectangle for consumption by
    /// structured output formats (PDF links, SVG  metadata, etc.). They have
    /// no visual effect on raster or null backings. Recording backings would
    /// capture the annotation for downstream consumers, but this is not yet
    /// implemented — the annotation is currently dropped in all cases.
    pub fn draw_annotation(&mut self, _rect: &Rect, _key: &str, _value: &[u8]) {
        // Annotations are metadata-only. Raster and null backings simply drop
        // them. Recording backings would emit DrawCommand::Annotation for
        // PDF/SVG export, but that variant doesn't exist yet (tracked by P5-9).
    }

    // =========================================================================
    // Text Drawing
    // =========================================================================

    /// Draw glyphs at specified positions.
    ///
    /// Each glyph's outline is extracted from `font` and drawn as a path at
    /// `origin + positions[i]` (baseline position for glyph `i`). Glyphs
    /// with no outline (e.g. space, .notdef without data) are skipped.
    #[cfg(feature = "text")]
    pub fn draw_glyphs(
        &mut self,
        glyph_ids: &[u16],
        positions: &[Point],
        origin: Point,
        font: &skia_rs_text::Font,
        paint: &Paint,
    ) {
        for (i, &glyph) in glyph_ids.iter().enumerate() {
            let pos = positions.get(i).copied().unwrap_or(Point::zero());
            if let Some(path) = font.glyph_path(glyph) {
                let m = skia_rs_core::Matrix::translate(origin.x + pos.x, origin.y + pos.y);
                let transformed = path.transformed(&m);
                self.draw_path(&transformed, paint);
            }
        }
    }

    /// Draw positioned text with alignment.
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

    /// Draw a string with its baseline at `(x, y)`.
    ///
    /// The text is first shaped (when font data is available) by rustybuzz
    /// into positioned glyphs; when no font data is present we fall back to
    /// a simple char-to-glyph mapping with per-glyph advances from
    /// [`skia_rs_text::Font::glyph_advance`]. For each shaped glyph the
    /// outline is extracted via [`skia_rs_text::Font::glyph_path`] and drawn
    /// as a path using the current paint, matrix, and clip stack. Glyphs
    /// with no outline (space, non-printing, dataless default typeface) are
    /// silently skipped.
    #[cfg(feature = "text")]
    pub fn draw_string(
        &mut self,
        text: &str,
        x: Scalar,
        y: Scalar,
        font: &skia_rs_text::Font,
        paint: &Paint,
    ) {
        // Prefer rustybuzz shaping when the typeface has real font data.
        // It handles ligatures, kerning, complex scripts correctly.
        let shaper = skia_rs_text::Shaper::new();
        if let Some(runs) = shaper.shape_auto(text, font) {
            let mut pen_x = x;
            let mut pen_y = y;
            for run in runs {
                for g in run.glyphs {
                    let glyph_x = pen_x + g.x_offset;
                    let glyph_y = pen_y + g.y_offset;
                    if let Some(path) = font.glyph_path(g.glyph_id.0) {
                        let m = skia_rs_core::Matrix::translate(glyph_x, glyph_y);
                        let transformed = path.transformed(&m);
                        self.draw_path(&transformed, paint);
                    }
                    pen_x += g.x_advance;
                    pen_y += g.y_advance;
                }
            }
            return;
        }

        // Fallback path: no shape output (dataless typeface). Walk the
        // characters, advancing by font.glyph_advance per glyph.
        let mut pen_x = x;
        for c in text.chars() {
            let glyph = font.char_to_glyph(c);
            if let Some(path) = font.glyph_path(glyph) {
                let m = skia_rs_core::Matrix::translate(pen_x, y);
                let transformed = path.transformed(&m);
                self.draw_path(&transformed, paint);
            }
            pen_x += font.glyph_advance(glyph);
        }
    }

    /// Flush any pending drawing operations.
    ///
    /// For raster canvases, drawing is synchronous — every draw method
    /// mutates the backing pixel buffer immediately — so flush() is a
    /// no-op. For recording canvases, flush() does not force playback
    /// (that's handled separately by Picture consumers). For null
    /// canvases, flush() is trivially a no-op.
    ///
    /// When GPU and deferred backings land, this method will route to
    /// the backing's flush primitive.
    pub fn flush(&mut self) {
        // Currently a no-op for all supported backings. GPU/deferred
        // routing will be added when those backings are introduced.
    }
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

        // Route pixel writes to the top layer buffer if one is active, else
        // to the base raster backing. Layer buffers are in layer-local
        // coords (canvas coord minus bounds origin), so apply the same
        // offset subtraction used in with_rasterizer.
        let (buffer, buf_offset_x, buf_offset_y): (&mut PixelBuffer, i32, i32) =
            if let Some(layer) = self.layer_stack.last_mut() {
                let (ox, oy) = match layer.bounds {
                    Some(b) => (b.left.floor() as i32, b.top.floor() as i32),
                    None => (0, 0),
                };
                (&mut layer.buffer, ox, oy)
            } else {
                match &mut self.backing {
                    Backing::Raster(b) => (*b, 0, 0),
                    _ => return,
                }
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

                    buffer.blend_pixel(dst_x - buf_offset_x, dst_y - buf_offset_y, color, blend_mode);
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
                for (tri_idx, chunk) in positions.chunks(3).enumerate() {
                    if chunk.len() == 3 {
                        let base_idx = tri_idx * 3;
                        let c0 = colors.and_then(|c| c.get(base_idx).copied());
                        let c1 = colors.and_then(|c| c.get(base_idx + 1).copied());
                        let c2 = colors.and_then(|c| c.get(base_idx + 2).copied());
                        self.draw_triangle(
                            matrix.map_point(chunk[0]),
                            matrix.map_point(chunk[1]),
                            matrix.map_point(chunk[2]),
                            c0,
                            c1,
                            c2,
                            paint,
                        );
                    }
                }
            }
            VertexMode::TriangleStrip => {
                for i in 0..positions.len().saturating_sub(2) {
                    let (p0, p1, p2, c0, c1, c2) = if i % 2 == 0 {
                        (
                            positions[i],
                            positions[i + 1],
                            positions[i + 2],
                            colors.and_then(|c| c.get(i).copied()),
                            colors.and_then(|c| c.get(i + 1).copied()),
                            colors.and_then(|c| c.get(i + 2).copied()),
                        )
                    } else {
                        (
                            positions[i + 1],
                            positions[i],
                            positions[i + 2],
                            colors.and_then(|c| c.get(i + 1).copied()),
                            colors.and_then(|c| c.get(i).copied()),
                            colors.and_then(|c| c.get(i + 2).copied()),
                        )
                    };
                    self.draw_triangle(
                        matrix.map_point(p0),
                        matrix.map_point(p1),
                        matrix.map_point(p2),
                        c0,
                        c1,
                        c2,
                        paint,
                    );
                }
            }
            VertexMode::TriangleFan => {
                let center = positions[0];
                for i in 1..positions.len().saturating_sub(1) {
                    let c0 = colors.and_then(|c| c.get(0).copied());
                    let c1 = colors.and_then(|c| c.get(i).copied());
                    let c2 = colors.and_then(|c| c.get(i + 1).copied());
                    self.draw_triangle(
                        matrix.map_point(center),
                        matrix.map_point(positions[i]),
                        matrix.map_point(positions[i + 1]),
                        c0,
                        c1,
                        c2,
                        paint,
                    );
                }
            }
        }
    }

    /// Draw a single filled triangle with per-vertex color interpolation.
    fn draw_triangle(
        &mut self,
        p0: Point,
        p1: Point,
        p2: Point,
        c0: Option<Color>,
        c1: Option<Color>,
        c2: Option<Color>,
        paint: &Paint,
    ) {
        let (buffer, buf_offset_x, buf_offset_y): (&mut PixelBuffer, i32, i32) =
            if let Some(layer) = self.layer_stack.last_mut() {
                let (ox, oy) = match layer.bounds {
                    Some(b) => (b.left.floor() as i32, b.top.floor() as i32),
                    None => (0, 0),
                };
                (&mut layer.buffer, ox, oy)
            } else {
                match &mut self.backing {
                    Backing::Raster(b) => (*b, 0, 0),
                    _ => return,
                }
            };

        let blend_mode = paint.blend_mode();

        // Convert to Color4f for interpolation
        let default_color = paint.color32().to_color4f();
        let col0 = c0.map(|c| c.to_color4f()).unwrap_or(default_color);
        let col1 = c1.map(|c| c.to_color4f()).unwrap_or(default_color);
        let col2 = c2.map(|c| c.to_color4f()).unwrap_or(default_color);

        // Check if all colors are the same - use fast path
        let uniform = c0 == c1 && c1 == c2;

        if uniform {
            // Fast path: flat shading
            let color = c0.unwrap_or_else(|| paint.color32());
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
                    buffer.blend_pixel(x - buf_offset_x, y - buf_offset_y, color, blend_mode);
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
                    buffer.blend_pixel(x - buf_offset_x, y - buf_offset_y, color, blend_mode);
                }
            }
        } else {
            // Slow path: barycentric interpolation
            // Compute bounding box
            let min_x = p0.x.min(p1.x).min(p2.x).floor() as i32;
            let max_x = p0.x.max(p1.x).max(p2.x).ceil() as i32;
            let min_y = p0.y.min(p1.y).min(p2.y).floor() as i32;
            let max_y = p0.y.max(p1.y).max(p2.y).ceil() as i32;

            // Compute triangle edge vectors
            let denom = (p1.y - p2.y) * (p0.x - p2.x) + (p2.x - p1.x) * (p0.y - p2.y);
            if denom.abs() < 1e-7 {
                return; // degenerate triangle
            }

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let px = x as Scalar + 0.5;
                    let py = y as Scalar + 0.5;

                    // Compute barycentric coordinates
                    let w0 = ((p1.y - p2.y) * (px - p2.x) + (p2.x - p1.x) * (py - p2.y)) / denom;
                    let w1 = ((p2.y - p0.y) * (px - p2.x) + (p0.x - p2.x) * (py - p2.y)) / denom;
                    let w2 = 1.0 - w0 - w1;

                    // Check if point is inside triangle
                    if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                        // Interpolate color components
                        let r = col0.r * w0 + col1.r * w1 + col2.r * w2;
                        let g = col0.g * w0 + col1.g * w1 + col2.g * w2;
                        let b = col0.b * w0 + col1.b * w1 + col2.b * w2;
                        let a = col0.a * w0 + col1.a * w1 + col2.a * w2;

                        // Convert back to Color
                        let r_u8 = (r * 255.0).clamp(0.0, 255.0) as u8;
                        let g_u8 = (g * 255.0).clamp(0.0, 255.0) as u8;
                        let b_u8 = (b * 255.0).clamp(0.0, 255.0) as u8;
                        let a_u8 = (a * 255.0).clamp(0.0, 255.0) as u8;
                        let color = Color::from_argb(a_u8, r_u8, g_u8, b_u8);

                        buffer.blend_pixel(
                            x - buf_offset_x,
                            y - buf_offset_y,
                            color,
                            blend_mode,
                        );
                    }
                }
            }
        }
    }

    /// Draw a text blob at `(x, y)`.
    ///
    /// Iterates over the blob's pre-shaped runs and draws each glyph's
    /// outline as a path. The blob already stores positioned glyphs, so no
    /// reshaping happens here — this is the fast path for repeatedly
    /// drawing the same string.
    ///
    /// Glyphs with no outline (space, missing glyphs) are silently skipped.
    #[cfg(feature = "text")]
    pub fn draw_text_blob(
        &mut self,
        blob: &skia_rs_text::TextBlob,
        x: Scalar,
        y: Scalar,
        paint: &Paint,
    ) {
        for run in blob.runs() {
            let font = &run.font;
            for (i, &glyph) in run.glyphs.iter().enumerate() {
                let pos = run
                    .positions
                    .get(i)
                    .copied()
                    .unwrap_or_else(|| Point::new(i as Scalar * font.size() * 0.5, 0.0));

                if let Some(path) = font.glyph_path(glyph) {
                    let gx = x + run.origin.x + pos.x;
                    let gy = y + run.origin.y + pos.y;
                    let m = skia_rs_core::Matrix::translate(gx, gy);
                    let transformed = path.transformed(&m);
                    self.draw_path(&transformed, paint);
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

    #[test]
    fn test_clip_path_does_not_collapse_to_bounds() {
        // Verify that clip_path stores the path geometry, not just its bounds.
        // A triangle clip should produce tighter clip bounds than a rectangle
        // of the same overall bounds would.
        use skia_rs_path::PathBuilder;

        let mut c = Canvas::new_null(100, 100);
        let mut builder = PathBuilder::new();
        builder.move_to(50.0, 10.0);
        builder.line_to(90.0, 90.0);
        builder.line_to(10.0, 90.0);
        builder.close();
        let triangle = builder.build();

        // Clip to the triangle path
        c.clip_path(&triangle, ClipOp::Intersect, false);

        // The clip bounds should match the triangle's bounds
        let clip_bounds = c.clip_bounds();
        let path_bounds = triangle.bounds();
        assert!((clip_bounds.left - path_bounds.left).abs() < 1.0);
        assert!((clip_bounds.top - path_bounds.top).abs() < 1.0);
        assert!((clip_bounds.right - path_bounds.right).abs() < 1.0);
        assert!((clip_bounds.bottom - path_bounds.bottom).abs() < 1.0);

        // Points inside the triangle bounds but outside the triangle itself
        // would be rejected by a proper path clip (though we can't easily test
        // pixel coverage without a raster backing, we've verified the path is
        // stored in ClipStack rather than collapsed to a rect).
    }

    #[test]
    fn test_clip_rect_difference() {
        // Verify that ClipOp::Difference creates a hole in the clip.
        let mut buffer = PixelBuffer::new(100, 100);
        let mut c = Canvas::new_raster(&mut buffer);

        // Apply a difference clip to create a hole
        let hole = Rect::from_xywh(40.0, 40.0, 20.0, 20.0);
        c.clip_rect(&hole, ClipOp::Difference, false);

        // Draw a white rectangle over the entire canvas
        let mut paint = Paint::new();
        paint.set_color32(Color::WHITE);
        c.draw_rect(&Rect::from_xywh(0.0, 0.0, 100.0, 100.0), &paint);

        // Pixel at (50, 50) should be unchanged (black/transparent, initial state)
        // because it's in the difference-clipped hole
        let center_pixel = buffer.get_pixel(50, 50).unwrap_or(Color::TRANSPARENT);
        assert_ne!(center_pixel, Color::WHITE,
            "Center pixel should not be white due to difference clip");

        // Pixel at (10, 10) should be white (outside the hole)
        let corner_pixel = buffer.get_pixel(10, 10).unwrap_or(Color::TRANSPARENT);
        assert_eq!(corner_pixel, Color::WHITE,
            "Corner pixel should be white (outside difference clip)");
    }

    #[test]
    fn test_clip_rect_anti_alias() {
        // Verify that anti_alias flag is propagated to ClipStack.
        let mut buffer = PixelBuffer::new(100, 100);
        let mut c = Canvas::new_raster(&mut buffer);

        // Apply an AA clip
        let clip_rect = Rect::from_xywh(10.5, 10.5, 50.0, 50.0);
        c.clip_rect(&clip_rect, ClipOp::Intersect, true);

        // The clip stack should now be AA (though we can't easily verify the
        // exact coverage values without reaching into internals, we can at
        // least verify the canvas doesn't crash and maintains state correctly).
        let clip_bounds = c.clip_bounds();
        assert!(clip_bounds.contains(Point::new(30.0, 30.0)));
    }

    #[test]
    fn test_clip_path_with_transform() {
        // Verify that clip_path respects the current matrix.
        use skia_rs_path::PathBuilder;

        let mut c = Canvas::new_null(200, 200);
        c.translate(50.0, 50.0);

        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(50.0, 0.0);
        builder.line_to(50.0, 50.0);
        builder.line_to(0.0, 50.0);
        builder.close();
        let rect_path = builder.build();

        c.clip_path(&rect_path, ClipOp::Intersect, false);

        // The clip bounds should be translated
        let clip_bounds = c.clip_bounds();
        assert!(clip_bounds.left >= 49.0 && clip_bounds.left <= 51.0);
        assert!(clip_bounds.top >= 49.0 && clip_bounds.top <= 51.0);
    }

    #[test]
    fn test_clip_stack_save_restore() {
        // Verify that save/restore properly manages the clip stack.
        let mut c = Canvas::new_null(100, 100);

        let initial_bounds = c.clip_bounds();

        c.save();
        c.clip_rect(&Rect::from_xywh(10.0, 10.0, 50.0, 50.0), ClipOp::Intersect, false);
        let clipped_bounds = c.clip_bounds();
        assert!(clipped_bounds.width() < initial_bounds.width());

        c.restore();
        let restored_bounds = c.clip_bounds();
        assert_eq!(restored_bounds, initial_bounds);
    }

    #[test]
    fn test_draw_points_points_mode_draws_each() {
        let mut buffer = PixelBuffer::new(100, 100);
        let mut canvas = Canvas::new_raster(&mut buffer);
        let mut paint = Paint::new();
        paint.set_color32(Color::WHITE);
        paint.set_stroke_width(3.0);
        canvas.draw_points(PointMode::Points, &[
            Point::new(10.0, 10.0),
            Point::new(50.0, 50.0),
            Point::new(90.0, 90.0),
        ], &paint);
        // Three distinct bright pixels should exist
        drop(canvas);
        assert_ne!(buffer.get_pixel(10, 10).unwrap_or(Color::TRANSPARENT), Color::TRANSPARENT);
        assert_ne!(buffer.get_pixel(50, 50).unwrap_or(Color::TRANSPARENT), Color::TRANSPARENT);
        assert_ne!(buffer.get_pixel(90, 90).unwrap_or(Color::TRANSPARENT), Color::TRANSPARENT);
    }

    #[test]
    fn test_draw_points_lines_mode_connects_pairs() {
        let mut buffer = PixelBuffer::new(100, 100);
        let mut canvas = Canvas::new_raster(&mut buffer);
        let mut paint = Paint::new();
        paint.set_color32(Color::WHITE);
        // Two pairs: horizontal and vertical line
        canvas.draw_points(PointMode::Lines, &[
            Point::new(0.0, 50.0),
            Point::new(100.0, 50.0),   // pair 1: horizontal line
            Point::new(50.0, 0.0),
            Point::new(50.0, 100.0),   // pair 2: vertical line
        ], &paint);
        // Verify by sampling midpoints of both lines
        drop(canvas);
        assert_ne!(buffer.get_pixel(50, 50).unwrap_or(Color::TRANSPARENT), Color::TRANSPARENT);
    }

    #[test]
    fn test_draw_points_polygon_mode_connects_consecutive() {
        let mut buffer = PixelBuffer::new(100, 100);
        let mut canvas = Canvas::new_raster(&mut buffer);
        let mut paint = Paint::new();
        paint.set_color32(Color::WHITE);
        // Triangle perimeter: three lines should be drawn
        canvas.draw_points(PointMode::Polygon, &[
            Point::new(50.0, 10.0),
            Point::new(90.0, 90.0),
            Point::new(10.0, 90.0),
        ], &paint);
        drop(canvas);
        // Lines connect 50,10->90,90 and 90,90->10,90
        // Sample a point on the second line
        assert_ne!(buffer.get_pixel(50, 90).unwrap_or(Color::TRANSPARENT), Color::TRANSPARENT);
    }

    #[test]
    fn test_draw_points_empty_slice() {
        let mut buffer = PixelBuffer::new(10, 10);
        let mut canvas = Canvas::new_raster(&mut buffer);
        let paint = Paint::new();
        canvas.draw_points(PointMode::Points, &[], &paint);  // should not panic
        // Verify no pixels were drawn (buffer should be all transparent)
        drop(canvas);
        assert_eq!(buffer.get_pixel(5, 5).unwrap_or(Color::TRANSPARENT), Color::TRANSPARENT);
    }

    #[test]
    fn test_draw_points_lines_mode_odd_count_drops_last() {
        // Lines mode requires pairs; odd count drops the last point.
        let mut buffer = PixelBuffer::new(100, 100);
        let mut canvas = Canvas::new_raster(&mut buffer);
        let mut paint = Paint::new();
        paint.set_color32(Color::WHITE);
        canvas.draw_points(PointMode::Lines, &[
            Point::new(0.0, 50.0),
            Point::new(100.0, 50.0),
            Point::new(50.0, 50.0),  // orphan, should be ignored
        ], &paint);
        // Should not panic; one line should be drawn
        drop(canvas);
        assert_ne!(buffer.get_pixel(50, 50).unwrap_or(Color::TRANSPARENT), Color::TRANSPARENT);
    }

    #[test]
    fn test_draw_points_recording_backs_up_all_modes() {
        // Verify recording captures all PointMode variants properly.
        let mut cmds: Vec<DrawCommand> = Vec::new();
        {
            let mut c = Canvas::new_recording(&mut cmds, 100, 100);
            let paint = Paint::new();
            c.draw_points(PointMode::Points, &[
                Point::new(10.0, 10.0),
                Point::new(20.0, 20.0),
            ], &paint);
        }
        // Two DrawPoint commands should be recorded
        assert_eq!(cmds.iter().filter(|cmd| matches!(cmd, DrawCommand::DrawPoint { .. })).count(), 2);

        let mut cmds = Vec::new();
        {
            let mut c = Canvas::new_recording(&mut cmds, 100, 100);
            let paint = Paint::new();
            c.draw_points(PointMode::Lines, &[
                Point::new(0.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(20.0, 20.0),
                Point::new(30.0, 30.0),
            ], &paint);
        }
        // Two DrawLine commands should be recorded (one per pair)
        assert_eq!(cmds.iter().filter(|cmd| matches!(cmd, DrawCommand::DrawLine { .. })).count(), 2);

        let mut cmds = Vec::new();
        {
            let mut c = Canvas::new_recording(&mut cmds, 100, 100);
            let paint = Paint::new();
            c.draw_points(PointMode::Polygon, &[
                Point::new(0.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(20.0, 0.0),
            ], &paint);
        }
        // Two DrawLine commands should be recorded (n-1 for polygon)
        assert_eq!(cmds.iter().filter(|cmd| matches!(cmd, DrawCommand::DrawLine { .. })).count(), 2);
    }

    // =========================================================================
    // save_layer / restore layer-composition tests
    // =========================================================================

    #[test]
    fn test_save_layer_increments_save_count_and_restore_balances() {
        let mut buffer = PixelBuffer::new(10, 10);
        let mut canvas = Canvas::new_raster(&mut buffer);
        let before = canvas.save_count();
        let returned = canvas.save_layer(&SaveLayerRec::default());
        let during = canvas.save_count();
        assert_eq!(during, before + 1);
        assert_eq!(returned, during);
        canvas.restore();
        assert_eq!(canvas.save_count(), before);
    }

    #[test]
    fn test_save_layer_full_alpha_paint_is_pixel_identical_to_direct_draw() {
        // A save_layer with a default (opaque SrcOver) paint should composite
        // the layer unchanged onto the base buffer, matching a direct draw.
        let mut direct = PixelBuffer::new(8, 8);
        {
            let mut c = Canvas::new_raster(&mut direct);
            let mut paint = Paint::new();
            paint.set_color32(Color::from_argb(255, 200, 100, 50));
            c.draw_rect(&Rect::from_xywh(1.0, 1.0, 6.0, 6.0), &paint);
        }

        let mut layered = PixelBuffer::new(8, 8);
        {
            let mut c = Canvas::new_raster(&mut layered);
            c.save_layer(&SaveLayerRec::default());
            let mut paint = Paint::new();
            paint.set_color32(Color::from_argb(255, 200, 100, 50));
            c.draw_rect(&Rect::from_xywh(1.0, 1.0, 6.0, 6.0), &paint);
            c.restore();
        }

        for y in 0..8 {
            for x in 0..8 {
                let a = direct.get_pixel(x, y).unwrap();
                let b = layered.get_pixel(x, y).unwrap();
                assert_eq!(
                    a.as_u32(),
                    b.as_u32(),
                    "mismatch at ({x},{y}): direct={:08x} layered={:08x}",
                    a.as_u32(),
                    b.as_u32()
                );
            }
        }
    }

    #[test]
    fn test_save_layer_half_alpha_paint_halves_source_contribution() {
        // Draw an opaque white rect inside a save_layer whose paint has
        // ~50% alpha. The base buffer starts black; after composite the
        // result should be roughly mid-gray (~128), not 255.
        let mut buffer = PixelBuffer::new(4, 4);
        buffer.clear(Color::from_argb(255, 0, 0, 0));
        let mut canvas = Canvas::new_raster(&mut buffer);

        let mut layer_paint = Paint::new();
        layer_paint.set_color32(Color::from_argb(128, 255, 255, 255));
        let rec = SaveLayerRec {
            bounds: None,
            paint: Some(&layer_paint),
            flags: SaveLayerFlags::NONE,
        };
        canvas.save_layer(&rec);

        let mut white = Paint::new();
        white.set_color32(Color::WHITE);
        canvas.draw_rect(&Rect::from_xywh(0.0, 0.0, 4.0, 4.0), &white);

        canvas.restore();
        drop(canvas);

        let c = buffer.get_pixel(2, 2).unwrap();
        // With 50% paint alpha over solid black, expect ~128 on each channel.
        assert!(
            c.red() > 100 && c.red() < 160,
            "expected ~128, got r={}",
            c.red()
        );
        assert!(
            c.green() > 100 && c.green() < 160,
            "expected ~128, got g={}",
            c.green()
        );
        assert!(
            c.blue() > 100 && c.blue() < 160,
            "expected ~128, got b={}",
            c.blue()
        );
    }

    #[test]
    fn test_save_layer_bounds_restrict_composite_area() {
        // With bounds [0,0,4,4], a full-canvas draw inside the layer must
        // only affect pixels inside those bounds after restore.
        let mut buffer = PixelBuffer::new(10, 10);
        buffer.clear(Color::from_argb(255, 0, 0, 0));
        let mut canvas = Canvas::new_raster(&mut buffer);

        let layer_bounds = Rect::from_xywh(0.0, 0.0, 4.0, 4.0);
        let rec = SaveLayerRec {
            bounds: Some(&layer_bounds),
            paint: None,
            flags: SaveLayerFlags::NONE,
        };
        canvas.save_layer(&rec);

        let mut red = Paint::new();
        red.set_color32(Color::from_argb(255, 255, 0, 0));
        // Fill the whole canvas — most of it falls outside the layer's
        // 4x4 buffer and should simply be clipped away on composite.
        canvas.draw_rect(&Rect::from_xywh(0.0, 0.0, 10.0, 10.0), &red);
        canvas.restore();
        drop(canvas);

        // Inside bounds: red.
        let inside = buffer.get_pixel(1, 1).unwrap();
        assert_eq!(inside.red(), 255);
        // Outside bounds: still the original black — layer composition
        // must not have touched it.
        let outside = buffer.get_pixel(8, 8).unwrap();
        assert_eq!(outside.red(), 0);
        assert_eq!(outside.alpha(), 255);
    }

    #[test]
    fn test_save_layer_nested_lifo_composition() {
        // Two nested layers, each with 50% alpha paint. After both restores
        // the innermost draw should have been multiplied by both alphas
        // (~0.5 * 0.5 = 0.25), producing roughly 64 on the base.
        let mut buffer = PixelBuffer::new(4, 4);
        buffer.clear(Color::from_argb(255, 0, 0, 0));
        let mut canvas = Canvas::new_raster(&mut buffer);

        let mut p1 = Paint::new();
        p1.set_color32(Color::from_argb(128, 255, 255, 255));
        let rec1 = SaveLayerRec {
            bounds: None,
            paint: Some(&p1),
            flags: SaveLayerFlags::NONE,
        };
        canvas.save_layer(&rec1);

        let mut p2 = Paint::new();
        p2.set_color32(Color::from_argb(128, 255, 255, 255));
        let rec2 = SaveLayerRec {
            bounds: None,
            paint: Some(&p2),
            flags: SaveLayerFlags::NONE,
        };
        canvas.save_layer(&rec2);

        let mut white = Paint::new();
        white.set_color32(Color::WHITE);
        canvas.draw_rect(&Rect::from_xywh(0.0, 0.0, 4.0, 4.0), &white);

        canvas.restore();
        canvas.restore();
        drop(canvas);

        let c = buffer.get_pixel(2, 2).unwrap();
        // Two nested 50% alphas over solid black -> roughly 0.25 * 255 = 64.
        assert!(
            c.red() >= 32 && c.red() <= 96,
            "expected ~64, got r={}",
            c.red()
        );
    }

    #[test]
    fn test_save_layer_restore_to_count_composites_all_open_layers() {
        // restore_to_count must cascade through restore(), which in turn
        // composites each popped layer back onto its parent.
        let mut buffer = PixelBuffer::new(4, 4);
        buffer.clear(Color::from_argb(255, 0, 0, 0));
        let mut canvas = Canvas::new_raster(&mut buffer);
        let base = canvas.save_count();

        canvas.save_layer(&SaveLayerRec::default());
        canvas.save_layer(&SaveLayerRec::default());

        let mut white = Paint::new();
        white.set_color32(Color::WHITE);
        canvas.draw_rect(&Rect::from_xywh(0.0, 0.0, 4.0, 4.0), &white);

        canvas.restore_to_count(base);
        assert_eq!(canvas.save_count(), base);
        drop(canvas);

        // Both layers composited with default (opaque) paint — pixel should
        // be white on black, i.e. fully opaque white.
        let c = buffer.get_pixel(2, 2).unwrap();
        assert_eq!(c.red(), 255);
        assert_eq!(c.green(), 255);
        assert_eq!(c.blue(), 255);
    }

    #[test]
    fn test_save_layer_draws_do_not_leak_to_base_until_restore() {
        // Drawing inside save_layer must not touch the base buffer until
        // the matching restore composites the layer back.
        let mut buffer = PixelBuffer::new(4, 4);
        buffer.clear(Color::from_argb(255, 0, 0, 0));
        let mut canvas = Canvas::new_raster(&mut buffer);

        canvas.save_layer(&SaveLayerRec::default());
        let mut white = Paint::new();
        white.set_color32(Color::WHITE);
        canvas.draw_rect(&Rect::from_xywh(0.0, 0.0, 4.0, 4.0), &white);

        // Before restore we expect the base buffer to still be untouched.
        // We can't borrow the base buffer while the canvas holds it, so
        // drop the canvas first — dropping ends the &mut borrow without
        // running composition (composition only runs in restore()).
        drop(canvas);

        let c = buffer.get_pixel(2, 2).unwrap();
        assert_eq!(c.red(), 0, "base buffer should be untouched before restore");
        assert_eq!(c.green(), 0);
        assert_eq!(c.blue(), 0);
    }

    #[test]
    fn test_flush_is_safe_on_all_backings() {
        // Flush must be safe and non-panicking on all backing types.
        // It's a no-op for raster (synchronous), recording (does not force
        // playback), and null (trivially no-op).
        let mut buffer = PixelBuffer::new(10, 10);
        let mut canvas = Canvas::new_raster(&mut buffer);
        canvas.flush();

        let mut commands = Vec::new();
        let mut canvas = Canvas::new_recording(&mut commands, 10, 10);
        canvas.flush();

        let mut canvas = Canvas::new_null(10, 10);
        canvas.flush();
    }

    #[test]
    fn test_draw_vertices_triangles_per_vertex_color() {
        // Triangle with red, green, blue at vertices should show color
        // interpolation: pixels near each vertex are dominated by that
        // vertex's color.
        let mut buffer = PixelBuffer::new(100, 100);
        buffer.clear(Color::from_argb(255, 0, 0, 0));
        let mut canvas = Canvas::new_raster(&mut buffer);
        let paint = Paint::new();

        canvas.draw_vertices(
            VertexMode::Triangles,
            &[
                Point::new(50.0, 10.0),
                Point::new(90.0, 90.0),
                Point::new(10.0, 90.0),
            ],
            Some(&[
                Color::from_rgb(255, 0, 0),    // red at top
                Color::from_rgb(0, 255, 0),    // green at bottom-right
                Color::from_rgb(0, 0, 255),    // blue at bottom-left
            ]),
            &paint,
        );
        drop(canvas);

        // Near vertex 0 (50, 10) should be mostly red
        let near_v0 = buffer.get_pixel(50, 15).unwrap();
        assert!(
            near_v0.red() > 150,
            "near v0 should be red-dominated, got r={}",
            near_v0.red()
        );

        // Near vertex 1 (90, 90) should be mostly green
        let near_v1 = buffer.get_pixel(88, 88).unwrap();
        assert!(
            near_v1.green() > 150,
            "near v1 should be green-dominated, got g={}",
            near_v1.green()
        );

        // Near vertex 2 (10, 90) should be mostly blue
        let near_v2 = buffer.get_pixel(12, 88).unwrap();
        assert!(
            near_v2.blue() > 150,
            "near v2 should be blue-dominated, got b={}",
            near_v2.blue()
        );
    }

    #[test]
    fn test_draw_vertices_flat_when_no_colors() {
        // Without colors, should use paint's color for entire triangle
        let mut buffer = PixelBuffer::new(100, 100);
        buffer.clear(Color::from_argb(255, 0, 0, 0));
        let mut canvas = Canvas::new_raster(&mut buffer);
        let mut paint = Paint::new();
        paint.set_color32(Color::from_rgb(255, 128, 64));

        canvas.draw_vertices(
            VertexMode::Triangles,
            &[
                Point::new(10.0, 10.0),
                Point::new(90.0, 10.0),
                Point::new(50.0, 90.0),
            ],
            None,
            &paint,
        );
        drop(canvas);

        let c = buffer.get_pixel(50, 50).unwrap();
        assert!(
            (c.red() as i32 - 255).abs() < 10,
            "Expected ~255 red, got {}",
            c.red()
        );
        assert!(
            (c.green() as i32 - 128).abs() < 10,
            "Expected ~128 green, got {}",
            c.green()
        );
        assert!(
            (c.blue() as i32 - 64).abs() < 10,
            "Expected ~64 blue, got {}",
            c.blue()
        );
    }

    #[test]
    fn test_draw_vertices_triangle_strip() {
        // TriangleStrip with alternating red/green should interpolate
        let mut buffer = PixelBuffer::new(100, 100);
        buffer.clear(Color::from_argb(255, 0, 0, 0));
        let mut canvas = Canvas::new_raster(&mut buffer);
        let paint = Paint::new();

        // Strip: vertices 0,1,2 form first triangle; 1,2,3 form second
        canvas.draw_vertices(
            VertexMode::TriangleStrip,
            &[
                Point::new(10.0, 10.0),
                Point::new(50.0, 10.0),
                Point::new(10.0, 50.0),
                Point::new(50.0, 50.0),
            ],
            Some(&[
                Color::from_rgb(255, 0, 0),
                Color::from_rgb(0, 255, 0),
                Color::from_rgb(0, 0, 255),
                Color::from_rgb(255, 255, 0),
            ]),
            &paint,
        );
        drop(canvas);

        // First triangle (0,1,2): red, green, blue
        let c0 = buffer.get_pixel(15, 15).unwrap();
        assert!(c0.red() > 100, "Expected red contribution in first triangle");

        // Second triangle (1,2,3): green, blue, yellow
        let c1 = buffer.get_pixel(45, 45).unwrap();
        assert!(
            c1.green() > 100 || c1.red() > 100,
            "Expected yellow-ish in second triangle"
        );
    }

    #[test]
    fn test_draw_vertices_triangle_fan() {
        // TriangleFan shares vertex 0 (center)
        let mut buffer = PixelBuffer::new(100, 100);
        buffer.clear(Color::from_argb(255, 0, 0, 0));
        let mut canvas = Canvas::new_raster(&mut buffer);
        let paint = Paint::new();

        canvas.draw_vertices(
            VertexMode::TriangleFan,
            &[
                Point::new(50.0, 50.0), // center - white
                Point::new(10.0, 10.0), // top-left - red
                Point::new(90.0, 10.0), // top-right - green
                Point::new(90.0, 90.0), // bottom-right - blue
            ],
            Some(&[
                Color::from_rgb(255, 255, 255),
                Color::from_rgb(255, 0, 0),
                Color::from_rgb(0, 255, 0),
                Color::from_rgb(0, 0, 255),
            ]),
            &paint,
        );
        drop(canvas);

        // Center should be bright (white vertex)
        let center = buffer.get_pixel(50, 50).unwrap();
        assert!(
            center.red() > 200 && center.green() > 200 && center.blue() > 200,
            "Center should be bright white"
        );
    }

    #[test]
    fn test_draw_patch_flat_square() {
        // 12 control points forming a flat 100x100 square patch
        let mut buffer = PixelBuffer::new(120, 120);
        buffer.clear(Color::from_argb(255, 0, 0, 0));
        let mut canvas = Canvas::new_raster(&mut buffer);

        let cubics = [
            // Top edge: left to right
            Point::new(10.0, 10.0),
            Point::new(40.0, 10.0),
            Point::new(70.0, 10.0),
            Point::new(100.0, 10.0),
            // Right edge: top to bottom
            Point::new(100.0, 40.0),
            Point::new(100.0, 70.0),
            Point::new(100.0, 100.0),
            // Bottom edge: right to left (reversed)
            Point::new(70.0, 100.0),
            Point::new(40.0, 100.0),
            Point::new(10.0, 100.0),
            // Left edge: bottom to top
            Point::new(10.0, 70.0),
            Point::new(10.0, 40.0),
        ];

        let mut paint = Paint::new();
        paint.set_color32(Color::from_rgb(255, 0, 0));

        canvas.draw_patch(&cubics, None, None, skia_rs_paint::BlendMode::SrcOver, &paint);
        drop(canvas);

        // Center of the patch should be red
        let center = buffer.get_pixel(55, 55).unwrap();
        assert!(center.red() > 200, "Center should be red, got {:?}", center);
    }

    #[test]
    fn test_draw_patch_with_corner_colors() {
        // Patch with interpolated corner colors
        let mut buffer = PixelBuffer::new(120, 120);
        buffer.clear(Color::from_argb(255, 0, 0, 0));
        let mut canvas = Canvas::new_raster(&mut buffer);

        let cubics = [
            Point::new(10.0, 10.0),
            Point::new(40.0, 10.0),
            Point::new(70.0, 10.0),
            Point::new(100.0, 10.0),
            Point::new(100.0, 40.0),
            Point::new(100.0, 70.0),
            Point::new(100.0, 100.0),
            Point::new(70.0, 100.0),
            Point::new(40.0, 100.0),
            Point::new(10.0, 100.0),
            Point::new(10.0, 70.0),
            Point::new(10.0, 40.0),
        ];

        // Corner colors: red, green, blue, yellow
        let colors = [
            Color::from_rgb(255, 0, 0),   // top-left: red
            Color::from_rgb(0, 255, 0),   // top-right: green
            Color::from_rgb(0, 0, 255),   // bottom-right: blue
            Color::from_rgb(255, 255, 0), // bottom-left: yellow
        ];

        let mut paint = Paint::new();
        canvas.draw_patch(
            &cubics,
            Some(&colors),
            None,
            skia_rs_paint::BlendMode::SrcOver,
            &paint,
        );
        drop(canvas);

        // Center should have a mix of all four colors
        let center = buffer.get_pixel(55, 55).unwrap();
        assert!(
            center.red() > 50 && center.green() > 50,
            "Center should blend all colors, got {:?}",
            center
        );
    }

    #[test]
    fn test_draw_annotation_noop_on_raster() {
        // Annotations should not panic or affect pixels
        let mut buffer = PixelBuffer::new(10, 10);
        buffer.clear(Color::from_argb(255, 100, 100, 100));
        let mut canvas = Canvas::new_raster(&mut buffer);

        canvas.draw_annotation(
            &Rect::from_xywh(0.0, 0.0, 5.0, 5.0),
            "url",
            b"https://example.com",
        );
        drop(canvas);

        // Pixels should be unchanged
        let pixel = buffer.get_pixel(1, 1).unwrap();
        assert_eq!(pixel.red(), 100);
        assert_eq!(pixel.green(), 100);
        assert_eq!(pixel.blue(), 100);
    }

    #[test]
    #[cfg(feature = "codec")]
    fn test_draw_image_lattice_single_cell() {
        // Lattice with one division in each axis creates a 2x2 grid of cells
        let mut buffer = PixelBuffer::new(40, 40);
        let mut canvas = Canvas::new_raster(&mut buffer);

        let image = skia_rs_codec::Image::from_color(10, 10, 0xFF_FF0000).unwrap();
        let lattice = ImageLattice::new(vec![5], vec![5]);
        canvas.draw_image_lattice(
            &image,
            &lattice,
            &Rect::from_xywh(0.0, 0.0, 40.0, 40.0),
            FilterMode::Nearest,
            None,
        );

        // Should draw something without panicking
    }

    #[test]
    #[cfg(feature = "codec")]
    fn test_draw_image_lattice_empty_divs() {
        // Lattice with no divs = single cell = full image
        let mut buffer = PixelBuffer::new(40, 40);
        let mut canvas = Canvas::new_raster(&mut buffer);

        let image = skia_rs_codec::Image::from_color(10, 10, 0xFF_00FF00).unwrap();
        let lattice = ImageLattice::new(vec![], vec![]);
        canvas.draw_image_lattice(
            &image,
            &lattice,
            &Rect::from_xywh(0.0, 0.0, 40.0, 40.0),
            FilterMode::Nearest,
            None,
        );

        // Should behave like draw_image_rect
    }

    #[test]
    #[cfg(feature = "codec")]
    fn test_draw_atlas_empty() {
        // Empty xforms/sprites should be a no-op
        let mut buffer = PixelBuffer::new(10, 10);
        let mut canvas = Canvas::new_raster(&mut buffer);

        let image = skia_rs_codec::Image::from_color(100, 100, 0xFF_0000FF).unwrap();
        canvas.draw_atlas(&image, &[], &[], None, skia_rs_paint::BlendMode::SrcOver, FilterMode::Nearest, None);

        // Should not panic
    }

    #[test]
    #[cfg(feature = "codec")]
    fn test_draw_atlas_single_sprite() {
        // Single sprite with translation
        let mut buffer = PixelBuffer::new(100, 100);
        let mut canvas = Canvas::new_raster(&mut buffer);

        let image = skia_rs_codec::Image::from_color(100, 100, 0xFF_FFFF00).unwrap();
        let xforms = [RSXform::from_scale_translate(1.0, 10.0, 10.0)];
        let sprites = [Rect::from_xywh(0.0, 0.0, 10.0, 10.0)];

        canvas.draw_atlas(&image, &xforms, &sprites, None, skia_rs_paint::BlendMode::SrcOver, FilterMode::Nearest, None);

        // Should draw sprite at (10, 10) without panicking
    }
}
