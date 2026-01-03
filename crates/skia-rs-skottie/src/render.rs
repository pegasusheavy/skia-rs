//! Canvas rendering for Lottie animations.
//!
//! This module provides the rendering context and methods for
//! drawing Lottie animations to a canvas.

use crate::animation::{Asset, PrecompAsset};
use crate::layers::{Layer, LayerContent, MatteMode};
use crate::shapes::{
    FillShape, GradientFillShape, Shape, ShapeGroup, StrokeShape, TrimPathShape,
};
use skia_rs_core::{Color4f, Matrix, Rect, Scalar};
use skia_rs_paint::{BlendMode, Paint, Style};
use skia_rs_path::Path;
use std::collections::HashMap;

/// Render context for drawing animations.
pub struct RenderContext<'a> {
    /// Canvas to draw on.
    canvas: &'a mut dyn Canvas,
    /// Transform stack.
    transform_stack: Vec<Matrix>,
    /// Opacity stack.
    opacity_stack: Vec<Scalar>,
    /// Current transform.
    current_transform: Matrix,
    /// Current opacity.
    current_opacity: Scalar,
}

/// Canvas trait for rendering.
pub trait Canvas {
    /// Save the current state.
    fn save(&mut self);
    /// Restore the previous state.
    fn restore(&mut self);
    /// Apply a transform.
    fn concat(&mut self, matrix: &Matrix);
    /// Draw a path with a paint.
    fn draw_path(&mut self, path: &Path, paint: &Paint);
    /// Draw a rect with a paint.
    fn draw_rect(&mut self, rect: &Rect, paint: &Paint);
    /// Set clip to a path.
    fn clip_path(&mut self, path: &Path);
    /// Set clip to a rect.
    fn clip_rect(&mut self, rect: &Rect);
    /// Get the current transform.
    fn get_transform(&self) -> Matrix;
    /// Set the transform.
    fn set_transform(&mut self, matrix: &Matrix);
}

impl<'a> RenderContext<'a> {
    /// Create a new render context.
    pub fn new(canvas: &'a mut dyn Canvas) -> Self {
        Self {
            canvas,
            transform_stack: Vec::new(),
            opacity_stack: Vec::new(),
            current_transform: Matrix::IDENTITY,
            current_opacity: 1.0,
        }
    }

    /// Save the current state.
    pub fn save(&mut self) {
        self.transform_stack.push(self.current_transform.clone());
        self.opacity_stack.push(self.current_opacity);
        self.canvas.save();
    }

    /// Restore the previous state.
    pub fn restore(&mut self) {
        if let Some(transform) = self.transform_stack.pop() {
            self.current_transform = transform;
        }
        if let Some(opacity) = self.opacity_stack.pop() {
            self.current_opacity = opacity;
        }
        self.canvas.restore();
    }

    /// Concatenate a transform.
    pub fn concat(&mut self, matrix: &Matrix) {
        self.current_transform = self.current_transform.concat(matrix);
        self.canvas.concat(matrix);
    }

    /// Multiply opacity.
    pub fn multiply_opacity(&mut self, opacity: Scalar) {
        self.current_opacity *= opacity;
    }

    /// Get current opacity.
    pub fn current_opacity(&self) -> Scalar {
        self.current_opacity
    }

    /// Draw a path.
    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        self.canvas.draw_path(path, paint);
    }

    /// Draw a rect.
    pub fn draw_rect(&mut self, rect: &Rect, paint: &Paint) {
        self.canvas.draw_rect(rect, paint);
    }

    /// Clip to a path.
    pub fn clip_path(&mut self, path: &Path) {
        self.canvas.clip_path(path);
    }

    /// Clip to a rect.
    pub fn clip_rect(&mut self, rect: &Rect) {
        self.canvas.clip_rect(rect);
    }

    /// Render a layer.
    pub fn render_layer(
        &mut self,
        layer: &Layer,
        frame: Scalar,
        assets: &HashMap<String, Asset>,
    ) {
        if !layer.is_visible_at(frame) || layer.hidden {
            return;
        }

        let local_frame = layer.local_frame(frame);
        let opacity = layer.opacity_at(local_frame);

        self.save();

        // Apply layer transform
        let matrix = layer.matrix_at(local_frame);
        self.concat(&matrix);
        self.multiply_opacity(opacity);

        // Apply masks
        if layer.has_masks() {
            for mask in &layer.masks {
                if let Some(mask_path) = mask.path_at(local_frame) {
                    self.clip_path(&mask_path);
                }
            }
        }

        // Render content
        match &layer.content {
            LayerContent::Shape(content) => {
                self.render_shapes(&content.shapes, local_frame);
            }
            LayerContent::Solid(content) => {
                let rect = Rect::from_xywh(0.0, 0.0, content.width, content.height);
                let mut paint = Paint::new();
                paint.set_color32(content.color);
                paint.set_style(Style::Fill);
                
                let color = paint.color();
                let adjusted_color = Color4f::new(
                    color.r,
                    color.g,
                    color.b,
                    color.a * self.current_opacity,
                );
                paint.set_color(adjusted_color);
                
                self.draw_rect(&rect, &paint);
            }
            LayerContent::Precomp(content) => {
                if let Some(Asset::Precomp(precomp)) = assets.get(&content.ref_id) {
                    self.render_precomp(precomp, local_frame, assets);
                }
            }
            LayerContent::Image(_content) => {
                // Image rendering would require image loading support
            }
            LayerContent::Text(_content) => {
                // Text rendering would require font support
            }
            LayerContent::None => {}
        }

        self.restore();
    }

    /// Render shapes.
    fn render_shapes(&mut self, shapes: &[Shape], frame: Scalar) {
        // Collect geometry and style
        let mut paths: Vec<Path> = Vec::new();
        let mut fills: Vec<&FillShape> = Vec::new();
        let mut strokes: Vec<&StrokeShape> = Vec::new();
        let mut gradient_fills: Vec<&GradientFillShape> = Vec::new();
        let mut trim: Option<&TrimPathShape> = None;

        for shape in shapes {
            match shape {
                Shape::Group(group) => {
                    self.save();
                    
                    // Apply group transform
                    if let Some(ref transform) = group.transform {
                        let matrix = transform.matrix_at(frame);
                        self.concat(&matrix);
                        self.multiply_opacity(transform.opacity_at(frame));
                    }
                    
                    self.render_shapes(&group.shapes, frame);
                    
                    self.restore();
                }
                Shape::Rectangle(rect) => {
                    if let Some(path) = rect.to_path(frame) {
                        paths.push(path);
                    }
                }
                Shape::Ellipse(ellipse) => {
                    if let Some(path) = ellipse.to_path(frame) {
                        paths.push(path);
                    }
                }
                Shape::Path(path_shape) => {
                    if let Some(path) = path_shape.to_path(frame) {
                        paths.push(path);
                    }
                }
                Shape::Polystar(star) => {
                    if let Some(path) = star.to_path(frame) {
                        paths.push(path);
                    }
                }
                Shape::Fill(fill) => {
                    fills.push(fill);
                }
                Shape::Stroke(stroke) => {
                    strokes.push(stroke);
                }
                Shape::GradientFill(gf) => {
                    gradient_fills.push(gf);
                }
                Shape::TrimPath(tp) => {
                    trim = Some(tp);
                }
                Shape::Transform(st) => {
                    let matrix = st.transform.matrix_at(frame);
                    self.concat(&matrix);
                    self.multiply_opacity(st.transform.opacity_at(frame));
                }
                _ => {}
            }
        }

        // Apply trim if present
        let final_paths: Vec<Path> = if let Some(trim_shape) = trim {
            let (start, end, _offset) = trim_shape.values_at(frame);
            paths
                .into_iter()
                .map(|p| trim_path(&p, start, end))
                .collect()
        } else {
            paths
        };

        // Draw fills
        for fill in &fills {
            let mut paint = Paint::new();
            let color = fill.color_at(frame);
            paint.set_color(Color4f::new(
                color.r,
                color.g,
                color.b,
                color.a * self.current_opacity,
            ));
            paint.set_style(Style::Fill);

            for path in &final_paths {
                self.draw_path(path, &paint);
            }
        }

        // Draw gradient fills
        for gf in &gradient_fills {
            // Simplified gradient - just use first color
            let mut paint = Paint::new();
            paint.set_style(Style::Fill);
            
            let opacity = gf.opacity.value_at(frame).as_scalar().unwrap_or(100.0) / 100.0;
            paint.set_color(Color4f::new(0.5, 0.5, 0.5, opacity * self.current_opacity));

            for path in &final_paths {
                self.draw_path(path, &paint);
            }
        }

        // Draw strokes
        for stroke in &strokes {
            let mut paint = Paint::new();
            let color = stroke.color_at(frame);
            paint.set_color(Color4f::new(
                color.r,
                color.g,
                color.b,
                color.a * self.current_opacity,
            ));
            paint.set_style(Style::Stroke);
            paint.set_stroke_width(stroke.width_at(frame));
            paint.set_stroke_cap(stroke.line_cap);
            paint.set_stroke_join(stroke.line_join);
            paint.set_stroke_miter(stroke.miter_limit);

            for path in &final_paths {
                self.draw_path(path, &paint);
            }
        }
    }

    /// Render a precomposition.
    fn render_precomp(
        &mut self,
        precomp: &PrecompAsset,
        frame: Scalar,
        assets: &HashMap<String, Asset>,
    ) {
        for layer in precomp.layers.iter().rev() {
            if layer.is_visible_at(frame) {
                self.render_layer(layer, frame, assets);
            }
        }
    }
}

/// Trim a path to a portion.
fn trim_path(path: &Path, start: Scalar, end: Scalar) -> Path {
    if start >= end || (start == 0.0 && end == 1.0) {
        return path.clone();
    }

    // Simplified trim - just return the original for now
    // A proper implementation would use path measure
    path.clone()
}

/// Simple canvas implementation using skia-rs-canvas.
#[cfg(feature = "canvas")]
pub struct SkiaCanvas<'a> {
    inner: &'a mut skia_rs_canvas::Canvas,
}

#[cfg(feature = "canvas")]
impl<'a> SkiaCanvas<'a> {
    /// Create a new Skia canvas wrapper.
    pub fn new(canvas: &'a mut skia_rs_canvas::Canvas) -> Self {
        Self { inner: canvas }
    }
}

#[cfg(feature = "canvas")]
impl<'a> Canvas for SkiaCanvas<'a> {
    fn save(&mut self) {
        self.inner.save();
    }

    fn restore(&mut self) {
        self.inner.restore();
    }

    fn concat(&mut self, matrix: &Matrix) {
        self.inner.concat(matrix);
    }

    fn draw_path(&mut self, path: &Path, paint: &Paint) {
        self.inner.draw_path(path, paint);
    }

    fn draw_rect(&mut self, rect: &Rect, paint: &Paint) {
        self.inner.draw_rect(rect, paint);
    }

    fn clip_path(&mut self, path: &Path) {
        self.inner.clip_path(path);
    }

    fn clip_rect(&mut self, rect: &Rect) {
        self.inner.clip_rect(*rect);
    }

    fn get_transform(&self) -> Matrix {
        self.inner.get_transform()
    }

    fn set_transform(&mut self, matrix: &Matrix) {
        self.inner.set_transform(matrix);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCanvas {
        save_count: usize,
        draw_count: usize,
    }

    impl MockCanvas {
        fn new() -> Self {
            Self {
                save_count: 0,
                draw_count: 0,
            }
        }
    }

    impl Canvas for MockCanvas {
        fn save(&mut self) {
            self.save_count += 1;
        }

        fn restore(&mut self) {
            if self.save_count > 0 {
                self.save_count -= 1;
            }
        }

        fn concat(&mut self, _matrix: &Matrix) {}

        fn draw_path(&mut self, _path: &Path, _paint: &Paint) {
            self.draw_count += 1;
        }

        fn draw_rect(&mut self, _rect: &Rect, _paint: &Paint) {
            self.draw_count += 1;
        }

        fn clip_path(&mut self, _path: &Path) {}

        fn clip_rect(&mut self, _rect: &Rect) {}

        fn get_transform(&self) -> Matrix {
            Matrix::IDENTITY
        }

        fn set_transform(&mut self, _matrix: &Matrix) {}
    }

    #[test]
    fn test_render_context() {
        let mut canvas = MockCanvas::new();
        let mut ctx = RenderContext::new(&mut canvas);

        ctx.save();
        ctx.multiply_opacity(0.5);
        assert_eq!(ctx.current_opacity(), 0.5);
        ctx.restore();
        assert_eq!(ctx.current_opacity(), 1.0);
    }

    #[test]
    fn test_opacity_stack() {
        let mut canvas = MockCanvas::new();
        let mut ctx = RenderContext::new(&mut canvas);

        ctx.multiply_opacity(0.5);
        ctx.save();
        ctx.multiply_opacity(0.5);
        assert_eq!(ctx.current_opacity(), 0.25);
        ctx.restore();
        assert_eq!(ctx.current_opacity(), 0.5);
    }
}
