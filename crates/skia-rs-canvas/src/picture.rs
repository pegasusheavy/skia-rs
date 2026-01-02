//! Picture recording and playback.
//!
//! Pictures are display lists that record drawing commands for later playback.
//! This is useful for caching complex drawings, serialization, and deferred rendering.

use crate::Canvas;
use skia_rs_core::{Color, Matrix, Point, Rect, Scalar};
use skia_rs_paint::{BlendMode, Paint};
use skia_rs_path::Path;
use std::sync::Arc;

/// A recorded picture that can be played back to a canvas.
///
/// Corresponds to Skia's `SkPicture`.
#[derive(Debug, Clone)]
pub struct Picture {
    /// The recorded drawing commands.
    commands: Vec<DrawCommand>,
    /// Bounding box of the picture.
    cull_rect: Rect,
}

impl Picture {
    /// Create a new picture from recorded commands.
    pub(crate) fn new(commands: Vec<DrawCommand>, cull_rect: Rect) -> Self {
        Self {
            commands,
            cull_rect,
        }
    }

    /// Get the cull rect (bounding box).
    #[inline]
    pub fn cull_rect(&self) -> Rect {
        self.cull_rect
    }

    /// Play the picture back to a canvas.
    pub fn playback(&self, canvas: &mut Canvas) {
        for command in &self.commands {
            command.execute(canvas);
        }
    }

    /// Get the approximate byte size of this picture.
    pub fn approximate_bytes_used(&self) -> usize {
        std::mem::size_of::<Self>() + self.commands.len() * std::mem::size_of::<DrawCommand>()
    }

    /// Get the number of operations in this picture.
    pub fn approximate_op_count(&self) -> usize {
        self.commands.len()
    }
}

/// A picture reference (shared ownership).
pub type PictureRef = Arc<Picture>;

/// A recorded drawing command.
#[derive(Debug, Clone)]
pub enum DrawCommand {
    /// Save the canvas state.
    Save,
    /// Restore the canvas state.
    Restore,
    /// Save with a layer.
    SaveLayer {
        /// Bounds for the layer.
        bounds: Option<Rect>,
        /// Paint for the layer.
        paint: Option<Paint>,
    },
    /// Translate the canvas.
    Translate {
        /// X translation.
        dx: Scalar,
        /// Y translation.
        dy: Scalar,
    },
    /// Scale the canvas.
    Scale {
        /// X scale.
        sx: Scalar,
        /// Y scale.
        sy: Scalar,
    },
    /// Rotate the canvas.
    Rotate {
        /// Rotation in degrees.
        degrees: Scalar,
    },
    /// Skew the canvas.
    Skew {
        /// X skew.
        sx: Scalar,
        /// Y skew.
        sy: Scalar,
    },
    /// Concatenate a matrix.
    Concat {
        /// The matrix to concatenate.
        matrix: Matrix,
    },
    /// Set the matrix.
    SetMatrix {
        /// The matrix to set.
        matrix: Matrix,
    },
    /// Clip to a rectangle.
    ClipRect {
        /// The rectangle to clip to.
        rect: Rect,
        /// Whether to antialias.
        anti_alias: bool,
    },
    /// Clip to a path.
    ClipPath {
        /// The path to clip to.
        path: Path,
        /// Whether to antialias.
        anti_alias: bool,
    },
    /// Clear the canvas.
    Clear {
        /// The color to clear with.
        color: Color,
    },
    /// Draw a color.
    DrawColor {
        /// The color to draw.
        color: Color,
        /// The blend mode.
        blend_mode: BlendMode,
    },
    /// Draw a point.
    DrawPoint {
        /// The point to draw.
        point: Point,
        /// The paint to use.
        paint: Paint,
    },
    /// Draw a line.
    DrawLine {
        /// Start point.
        p0: Point,
        /// End point.
        p1: Point,
        /// The paint to use.
        paint: Paint,
    },
    /// Draw a rectangle.
    DrawRect {
        /// The rectangle to draw.
        rect: Rect,
        /// The paint to use.
        paint: Paint,
    },
    /// Draw an oval.
    DrawOval {
        /// The bounding rectangle of the oval.
        rect: Rect,
        /// The paint to use.
        paint: Paint,
    },
    /// Draw a circle.
    DrawCircle {
        /// Center of the circle.
        center: Point,
        /// Radius of the circle.
        radius: Scalar,
        /// The paint to use.
        paint: Paint,
    },
    /// Draw an arc.
    DrawArc {
        /// The bounding oval.
        oval: Rect,
        /// Start angle in degrees.
        start_angle: Scalar,
        /// Sweep angle in degrees.
        sweep_angle: Scalar,
        /// Whether to draw lines to center.
        use_center: bool,
        /// The paint to use.
        paint: Paint,
    },
    /// Draw a rounded rectangle.
    DrawRoundRect {
        /// The rectangle.
        rect: Rect,
        /// X radius.
        rx: Scalar,
        /// Y radius.
        ry: Scalar,
        /// The paint to use.
        paint: Paint,
    },
    /// Draw a path.
    DrawPath {
        /// The path to draw.
        path: Path,
        /// The paint to use.
        paint: Paint,
    },
    /// Draw another picture.
    DrawPicture {
        /// The picture to draw.
        picture: PictureRef,
        /// Optional matrix to apply.
        matrix: Option<Matrix>,
        /// Optional paint to apply.
        paint: Option<Paint>,
    },
}

impl DrawCommand {
    /// Execute this command on a canvas.
    pub fn execute(&self, canvas: &mut Canvas) {
        match self {
            DrawCommand::Save => {
                canvas.save();
            }
            DrawCommand::Restore => {
                canvas.restore();
            }
            DrawCommand::SaveLayer { bounds, paint } => {
                let rec = crate::SaveLayerRec {
                    bounds: bounds.as_ref(),
                    paint: paint.as_ref(),
                    flags: crate::SaveLayerFlags::NONE,
                };
                canvas.save_layer(&rec);
            }
            DrawCommand::Translate { dx, dy } => {
                canvas.translate(*dx, *dy);
            }
            DrawCommand::Scale { sx, sy } => {
                canvas.scale(*sx, *sy);
            }
            DrawCommand::Rotate { degrees } => {
                canvas.rotate(*degrees);
            }
            DrawCommand::Skew { sx, sy } => {
                canvas.skew(*sx, *sy);
            }
            DrawCommand::Concat { matrix } => {
                canvas.concat(matrix);
            }
            DrawCommand::SetMatrix { matrix } => {
                canvas.set_matrix(matrix);
            }
            DrawCommand::ClipRect { rect, anti_alias } => {
                canvas.clip_rect(rect, crate::ClipOp::Intersect, *anti_alias);
            }
            DrawCommand::ClipPath { path, anti_alias } => {
                canvas.clip_path(path, crate::ClipOp::Intersect, *anti_alias);
            }
            DrawCommand::Clear { color } => {
                canvas.clear(*color);
            }
            DrawCommand::DrawColor { color, blend_mode } => {
                canvas.draw_color(*color, *blend_mode);
            }
            DrawCommand::DrawPoint { point, paint } => {
                canvas.draw_point(*point, paint);
            }
            DrawCommand::DrawLine { p0, p1, paint } => {
                canvas.draw_line(*p0, *p1, paint);
            }
            DrawCommand::DrawRect { rect, paint } => {
                canvas.draw_rect(rect, paint);
            }
            DrawCommand::DrawOval { rect, paint } => {
                canvas.draw_oval(rect, paint);
            }
            DrawCommand::DrawCircle {
                center,
                radius,
                paint,
            } => {
                canvas.draw_circle(*center, *radius, paint);
            }
            DrawCommand::DrawArc {
                oval,
                start_angle,
                sweep_angle,
                use_center,
                paint,
            } => {
                canvas.draw_arc(oval, *start_angle, *sweep_angle, *use_center, paint);
            }
            DrawCommand::DrawRoundRect {
                rect,
                rx,
                ry,
                paint,
            } => {
                canvas.draw_round_rect(rect, *rx, *ry, paint);
            }
            DrawCommand::DrawPath { path, paint } => {
                canvas.draw_path(path, paint);
            }
            DrawCommand::DrawPicture {
                picture,
                matrix,
                paint,
            } => {
                canvas.save();
                if let Some(m) = matrix {
                    canvas.concat(m);
                }
                // Note: paint is used for layer effects in a full implementation
                let _ = paint;
                picture.playback(canvas);
                canvas.restore();
            }
        }
    }
}

/// A recorder that captures drawing commands into a Picture.
///
/// Corresponds to Skia's `SkPictureRecorder`.
pub struct PictureRecorder {
    commands: Vec<DrawCommand>,
    cull_rect: Rect,
    is_recording: bool,
}

impl Default for PictureRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl PictureRecorder {
    /// Create a new picture recorder.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            cull_rect: Rect::EMPTY,
            is_recording: false,
        }
    }

    /// Begin recording with a cull rect.
    pub fn begin_recording(&mut self, cull_rect: Rect) -> &mut RecordingCanvas {
        self.commands.clear();
        self.cull_rect = cull_rect;
        self.is_recording = true;
        // Safety: we're returning a reference that allows recording
        unsafe { &mut *(self as *mut Self as *mut RecordingCanvas) }
    }

    /// Finish recording and return the picture.
    pub fn finish_recording(&mut self) -> Option<PictureRef> {
        if !self.is_recording {
            return None;
        }
        self.is_recording = false;
        let commands = std::mem::take(&mut self.commands);
        Some(Arc::new(Picture::new(commands, self.cull_rect)))
    }

    /// Check if currently recording.
    #[inline]
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }
}

/// A canvas that records drawing commands.
///
/// This is actually a PictureRecorder with a canvas-like interface.
#[repr(transparent)]
pub struct RecordingCanvas {
    inner: PictureRecorder,
}

impl RecordingCanvas {
    /// Record a save command.
    pub fn save(&mut self) -> usize {
        self.inner.commands.push(DrawCommand::Save);
        self.inner.commands.len()
    }

    /// Record a restore command.
    pub fn restore(&mut self) {
        self.inner.commands.push(DrawCommand::Restore);
    }

    /// Record a save layer command.
    pub fn save_layer(&mut self, bounds: Option<Rect>, paint: Option<&Paint>) {
        self.inner.commands.push(DrawCommand::SaveLayer {
            bounds,
            paint: paint.cloned(),
        });
    }

    /// Record a translate command.
    pub fn translate(&mut self, dx: Scalar, dy: Scalar) {
        self.inner.commands.push(DrawCommand::Translate { dx, dy });
    }

    /// Record a scale command.
    pub fn scale(&mut self, sx: Scalar, sy: Scalar) {
        self.inner.commands.push(DrawCommand::Scale { sx, sy });
    }

    /// Record a rotate command.
    pub fn rotate(&mut self, degrees: Scalar) {
        self.inner.commands.push(DrawCommand::Rotate { degrees });
    }

    /// Record a skew command.
    pub fn skew(&mut self, sx: Scalar, sy: Scalar) {
        self.inner.commands.push(DrawCommand::Skew { sx, sy });
    }

    /// Record a concat command.
    pub fn concat(&mut self, matrix: &Matrix) {
        self.inner
            .commands
            .push(DrawCommand::Concat { matrix: *matrix });
    }

    /// Record a set matrix command.
    pub fn set_matrix(&mut self, matrix: &Matrix) {
        self.inner
            .commands
            .push(DrawCommand::SetMatrix { matrix: *matrix });
    }

    /// Record a clip rect command.
    pub fn clip_rect(&mut self, rect: &Rect, anti_alias: bool) {
        self.inner.commands.push(DrawCommand::ClipRect {
            rect: *rect,
            anti_alias,
        });
    }

    /// Record a clip path command.
    pub fn clip_path(&mut self, path: &Path, anti_alias: bool) {
        self.inner.commands.push(DrawCommand::ClipPath {
            path: path.clone(),
            anti_alias,
        });
    }

    /// Record a clear command.
    pub fn clear(&mut self, color: Color) {
        self.inner.commands.push(DrawCommand::Clear { color });
    }

    /// Record a draw color command.
    pub fn draw_color(&mut self, color: Color, blend_mode: BlendMode) {
        self.inner
            .commands
            .push(DrawCommand::DrawColor { color, blend_mode });
    }

    /// Record a draw point command.
    pub fn draw_point(&mut self, point: Point, paint: &Paint) {
        self.inner.commands.push(DrawCommand::DrawPoint {
            point,
            paint: paint.clone(),
        });
    }

    /// Record a draw line command.
    pub fn draw_line(&mut self, p0: Point, p1: Point, paint: &Paint) {
        self.inner.commands.push(DrawCommand::DrawLine {
            p0,
            p1,
            paint: paint.clone(),
        });
    }

    /// Record a draw rect command.
    pub fn draw_rect(&mut self, rect: &Rect, paint: &Paint) {
        self.inner.commands.push(DrawCommand::DrawRect {
            rect: *rect,
            paint: paint.clone(),
        });
    }

    /// Record a draw oval command.
    pub fn draw_oval(&mut self, rect: &Rect, paint: &Paint) {
        self.inner.commands.push(DrawCommand::DrawOval {
            rect: *rect,
            paint: paint.clone(),
        });
    }

    /// Record a draw circle command.
    pub fn draw_circle(&mut self, center: Point, radius: Scalar, paint: &Paint) {
        self.inner.commands.push(DrawCommand::DrawCircle {
            center,
            radius,
            paint: paint.clone(),
        });
    }

    /// Record a draw arc command.
    pub fn draw_arc(
        &mut self,
        oval: &Rect,
        start_angle: Scalar,
        sweep_angle: Scalar,
        use_center: bool,
        paint: &Paint,
    ) {
        self.inner.commands.push(DrawCommand::DrawArc {
            oval: *oval,
            start_angle,
            sweep_angle,
            use_center,
            paint: paint.clone(),
        });
    }

    /// Record a draw round rect command.
    pub fn draw_round_rect(&mut self, rect: &Rect, rx: Scalar, ry: Scalar, paint: &Paint) {
        self.inner.commands.push(DrawCommand::DrawRoundRect {
            rect: *rect,
            rx,
            ry,
            paint: paint.clone(),
        });
    }

    /// Record a draw path command.
    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        self.inner.commands.push(DrawCommand::DrawPath {
            path: path.clone(),
            paint: paint.clone(),
        });
    }

    /// Record a draw picture command.
    pub fn draw_picture(
        &mut self,
        picture: &PictureRef,
        matrix: Option<&Matrix>,
        paint: Option<&Paint>,
    ) {
        self.inner.commands.push(DrawCommand::DrawPicture {
            picture: picture.clone(),
            matrix: matrix.copied(),
            paint: paint.cloned(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_picture_recorder() {
        let mut recorder = PictureRecorder::new();
        assert!(!recorder.is_recording());

        let canvas = recorder.begin_recording(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        canvas.save();
        canvas.translate(10.0, 20.0);
        let paint = Paint::new();
        canvas.draw_rect(&Rect::from_xywh(0.0, 0.0, 50.0, 50.0), &paint);
        canvas.restore();

        let picture = recorder.finish_recording().unwrap();
        assert!(!recorder.is_recording());
        assert_eq!(picture.approximate_op_count(), 4); // save, translate, draw_rect, restore
    }

    #[test]
    fn test_picture_playback() {
        let mut recorder = PictureRecorder::new();
        let canvas = recorder.begin_recording(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        canvas.translate(10.0, 20.0);
        let picture = recorder.finish_recording().unwrap();

        let mut canvas = Canvas::new(100, 100);
        picture.playback(&mut canvas);
        // Verify canvas was modified
        let matrix = canvas.total_matrix();
        assert!(!matrix.is_identity());
    }

    #[test]
    fn test_nested_pictures() {
        // Create inner picture
        let mut recorder = PictureRecorder::new();
        let canvas = recorder.begin_recording(Rect::from_xywh(0.0, 0.0, 50.0, 50.0));
        canvas.translate(5.0, 5.0);
        let inner = recorder.finish_recording().unwrap();

        // Create outer picture that contains inner
        let mut recorder2 = PictureRecorder::new();
        let canvas = recorder2.begin_recording(Rect::from_xywh(0.0, 0.0, 100.0, 100.0));
        canvas.draw_picture(&inner, None, None);
        let outer = recorder2.finish_recording().unwrap();

        assert_eq!(outer.approximate_op_count(), 1);
    }
}
