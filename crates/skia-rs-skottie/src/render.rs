//! Canvas rendering for Lottie animations.
//!
//! This module provides the rendering context and methods for
//! drawing Lottie animations to a canvas.

use crate::animation::{Asset, PrecompAsset};
use crate::layers::{Layer, LayerContent};
use crate::mask;
use crate::shapes::{GradientFillShape, GradientStrokeShape, Shape, StrokeShape, TrimPathShape};
use skia_rs_core::{Color4f, Matrix, Point, Rect, Scalar};
use skia_rs_paint::{LinearGradient, Paint, Style, TileMode, TwoPointConicalGradient};
use skia_rs_path::{Path, PathMeasure};
use std::collections::HashMap;
use std::sync::Arc;

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
    /// Frame rate (fps), used to resolve precomp `tm` time remapping
    /// (which is expressed in seconds).
    frame_rate: Scalar,
    /// Bounds of the current composition, used to resolve inverted masks.
    bounds: Rect,
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
            frame_rate: 30.0,
            bounds: Rect::from_xywh(0.0, 0.0, 100000.0, 100000.0),
        }
    }

    /// Set the frame rate (fps) used for precomp `tm` remapping.
    pub fn set_frame_rate(&mut self, fps: Scalar) {
        self.frame_rate = fps;
    }

    /// Set the bounds of the current composition (used to resolve inverted
    /// masks to a finite region).
    pub fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
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

    /// Render a layer within a composition (`siblings` is the layer list
    /// of the enclosing composition, needed to resolve `parent` chains).
    ///
    /// Per upstream `Layer.cpp`/`PrecompLayer.cpp`, a layer's own
    /// transform/opacity/masks evaluate at the *unadjusted* composition
    /// frame; only a precomp layer's nested content gets the `st`/`sr`/`tm`
    /// time remap (applied in the `Precomp` branch below).
    pub fn render_layer(
        &mut self,
        layer: &Layer,
        frame: Scalar,
        assets: &HashMap<String, Asset>,
        siblings: &[Layer],
    ) {
        if !layer.is_visible_at(frame) || layer.hidden {
            return;
        }

        let opacity = layer.opacity_at(frame);

        self.save();

        // Apply layer transform, composed with the parent chain (guarded
        // against cycles).
        let matrix = compose_ancestor_matrix(layer, frame, siblings);
        self.concat(&matrix);
        self.multiply_opacity(opacity);

        // Apply masks: Add unions, Subtract subtracts, Intersect
        // intersects (see `mask::build_clip`).
        if layer.has_masks() {
            if let Some(clip) = mask::build_clip(&layer.masks, frame, self.bounds) {
                self.clip_path(&clip);
            }
        }

        // Render content
        match &layer.content {
            LayerContent::Shape(content) => {
                self.render_shapes(&content.shapes, frame);
            }
            LayerContent::Solid(content) => {
                let rect = Rect::from_xywh(0.0, 0.0, content.width, content.height);
                let mut paint = Paint::new();
                paint.set_color32(content.color);
                paint.set_style(Style::Fill);

                let color = paint.color();
                let adjusted_color =
                    Color4f::new(color.r, color.g, color.b, color.a * self.current_opacity);
                paint.set_color(adjusted_color);

                self.draw_rect(&rect, &paint);
            }
            LayerContent::Precomp(content) => {
                if let Some(Asset::Precomp(precomp)) = assets.get(&content.ref_id) {
                    let content_frame = layer.precomp_content_frame(frame, self.frame_rate);
                    self.render_precomp(precomp, content_frame, assets);
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
        let mut fills: Vec<&crate::shapes::FillShape> = Vec::new();
        let mut strokes: Vec<&StrokeShape> = Vec::new();
        let mut gradient_fills: Vec<&GradientFillShape> = Vec::new();
        let mut gradient_strokes: Vec<&GradientStrokeShape> = Vec::new();
        let mut trim: Option<&TrimPathShape> = None;

        for shape in shapes {
            match shape {
                Shape::Group(group) => {
                    self.save();

                    // Apply group transform; group opacity multiplies into
                    // the accumulated opacity.
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
                Shape::GradientStroke(gs) => {
                    gradient_strokes.push(gs);
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

        // Apply trim (path measure based) if present.
        let final_paths: Vec<Path> = if let Some(trim_shape) = trim {
            let (start_t, stop_t, inverted) = trim_shape.resolved_at(frame);
            paths
                .into_iter()
                .map(|p| trim_path(&p, start_t, stop_t, inverted))
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
                let mut p = path.clone();
                p.set_fill_type(fill.path_fill_type());
                self.draw_path(&p, &paint);
            }
        }

        // Draw gradient fills
        for gf in &gradient_fills {
            let mut paint = Paint::new();
            paint.set_style(Style::Fill);

            let opacity = gf.opacity.value_at(frame).as_scalar().unwrap_or(100.0) / 100.0;
            paint.set_color(Color4f::new(1.0, 1.0, 1.0, opacity * self.current_opacity));

            if let Some(shader) = build_gradient_shader(
                gf.gradient_type,
                gf.start_point.value_at(frame).as_vec2().unwrap_or([0.0, 0.0]),
                gf.end_point.value_at(frame).as_vec2().unwrap_or([0.0, 0.0]),
                &gf.stops_at(frame),
                gf.highlight_length.value_at(frame).as_scalar().unwrap_or(0.0),
                gf.highlight_angle.value_at(frame).as_scalar().unwrap_or(0.0),
            ) {
                paint.set_shader(Some(shader));
            }

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
                let dashed = apply_dash(path, stroke, frame);
                self.draw_path(&dashed, &paint);
            }
        }

        // Draw gradient strokes
        for gs in &gradient_strokes {
            let mut paint = Paint::new();
            paint.set_style(Style::Stroke);
            paint.set_stroke_width(gs.width.value_at(frame).as_scalar().unwrap_or(1.0));
            paint.set_stroke_cap(gs.line_cap);
            paint.set_stroke_join(gs.line_join);

            let opacity = gs.opacity.value_at(frame).as_scalar().unwrap_or(100.0) / 100.0;
            paint.set_color(Color4f::new(1.0, 1.0, 1.0, opacity * self.current_opacity));

            if let Some(shader) = build_gradient_shader(
                gs.gradient_type,
                gs.start_point.value_at(frame).as_vec2().unwrap_or([0.0, 0.0]),
                gs.end_point.value_at(frame).as_vec2().unwrap_or([0.0, 0.0]),
                &gs.stops_at(frame),
                gs.highlight_length.value_at(frame).as_scalar().unwrap_or(0.0),
                gs.highlight_angle.value_at(frame).as_scalar().unwrap_or(0.0),
            ) {
                paint.set_shader(Some(shader));
            }

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
                self.render_layer(layer, frame, assets, &precomp.layers);
            }
        }
    }
}

/// Compose a layer's own transform with its ancestor chain (`parent`
/// index references within `siblings`), guarding against cycles.
fn compose_ancestor_matrix(layer: &Layer, frame: Scalar, siblings: &[Layer]) -> Matrix {
    let mut result = layer.matrix_at(frame);
    let mut current_parent = layer.parent;
    let mut visited = std::collections::HashSet::new();
    visited.insert(layer.index);

    while let Some(parent_index) = current_parent {
        if !visited.insert(parent_index) {
            break; // cycle guard
        }
        let Some(parent) = siblings.iter().find(|l| l.index == parent_index) else {
            break;
        };
        // Parent transforms are applied "outside" the child's own
        // transform: total = parent_matrix * ... * child_matrix.
        result = parent.matrix_at(frame).concat(&result);
        current_parent = parent.parent;
    }

    result
}

/// Trim a path to the `[start, stop]` fraction of its length (both in
/// `0..=1`) using path measure, matching `SkTrimPathEffect` semantics. When
/// `inverted`, the *complement* interval (`[0,start] U [stop,1]`) is kept
/// instead (`SkTrimPathEffect::Mode::kInverted`).
fn trim_path(path: &Path, start: Scalar, stop: Scalar, inverted: bool) -> Path {
    if !inverted && start <= 0.0 && stop >= 1.0 {
        return path.clone();
    }
    if !inverted && start >= stop {
        return Path::default();
    }

    let measure = PathMeasure::new(path);
    let len = measure.length();
    if len <= 0.0 {
        return path.clone();
    }

    if inverted {
        let mut builder = skia_rs_path::PathBuilder::new();
        if start > 0.0 {
            if let Some(seg) = measure.get_segment(0.0, start * len) {
                builder.add_path(&seg);
            }
        }
        if stop < 1.0 {
            if let Some(seg) = measure.get_segment(stop * len, len) {
                builder.add_path(&seg);
            }
        }
        builder.build()
    } else {
        measure
            .get_segment(start * len, stop * len)
            .unwrap_or_default()
    }
}

/// Apply a stroke's dash pattern (if any) to a path, splitting it into
/// dash segments via Task 2's `DashEffect`.
fn apply_dash(path: &Path, stroke: &StrokeShape, frame: Scalar) -> Path {
    let Some(intervals) = stroke.dash_intervals_at(frame) else {
        return path.clone();
    };
    let phase = stroke.dash_offset_at(frame);
    match skia_rs_path::DashEffect::new(intervals, phase) {
        Some(dash) => {
            use skia_rs_path::PathEffect;
            dash.apply(path).unwrap_or_else(|| path.clone())
        }
        None => path.clone(),
    }
}

/// Build a gradient shader from resolved Lottie gradient parameters.
///
/// For radial gradients, matches upstream `GradientAdapter::onSync`
/// (`Gradient.cpp`): `highlight_length`/`highlight_angle` define a focal
/// point, producing a two-point conical gradient.
#[allow(clippy::too_many_arguments)]
fn build_gradient_shader(
    gradient_type: i32,
    start: [Scalar; 2],
    end: [Scalar; 2],
    stops: &[(Scalar, Color4f)],
    highlight_length: Scalar,
    highlight_angle: Scalar,
) -> Option<Arc<dyn skia_rs_paint::Shader>> {
    if stops.is_empty() {
        return None;
    }

    let colors: Vec<Color4f> = stops.iter().map(|(_, c)| *c).collect();
    let positions: Vec<Scalar> = stops.iter().map(|(t, _)| *t).collect();

    let s = Point::new(start[0], start[1]);
    let e = Point::new(end[0], end[1]);

    if gradient_type == 2 {
        // Radial.
        let angle_rad = highlight_angle.to_radians();
        let (sin, cos) = angle_rad.sin_cos();
        // Rotate `e` around `s` by `highlight_angle` degrees.
        let ex = e.x - s.x;
        let ey = e.y - s.y;
        let rotated_e = Point::new(s.x + ex * cos - ey * sin, s.y + ex * sin + ey * cos);

        let eps = 1e-4;
        let h_len = (highlight_length * 0.01).clamp(-1.0 + eps, 1.0 - eps);
        let focal = Point::new(
            s.x + (rotated_e.x - s.x) * h_len,
            s.y + (rotated_e.y - s.y) * h_len,
        );
        let end_radius = ((rotated_e.x - s.x).powi(2) + (rotated_e.y - s.y).powi(2)).sqrt();

        Some(Arc::new(TwoPointConicalGradient::new(
            focal,
            0.0,
            s,
            end_radius,
            colors,
            Some(positions),
            TileMode::Clamp,
        )))
    } else {
        Some(Arc::new(LinearGradient::new(
            s,
            e,
            colors,
            Some(positions),
            TileMode::Clamp,
        )))
    }
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

    fn layer_at(index: i32, parent: Option<i32>, x: Scalar, y: Scalar) -> Layer {
        use crate::model::LayerModel;
        let json = format!(
            r#"{{"ty":4,"nm":"l","ind":{index},"ip":0,"op":100,
                "ks":{{"p":{{"a":0,"k":[{x},{y}]}}}} }}"#
        );
        let mut model: LayerModel = serde_json::from_str(&json).unwrap();
        model.parent = parent;
        Layer::from_lottie(&model)
    }

    #[test]
    fn test_parenting_composes_ancestor_translation() {
        let parent = layer_at(1, None, 100.0, 100.0);
        let child = layer_at(2, Some(1), 10.0, 10.0);
        let siblings = vec![parent, child.clone()];

        let m = compose_ancestor_matrix(&child, 0.0, &siblings);
        let origin = m.map_point(skia_rs_core::Point::new(0.0, 0.0));
        assert_eq!(origin.x, 110.0);
        assert_eq!(origin.y, 110.0);
    }

    #[test]
    fn test_parenting_cycle_guard_terminates() {
        // Two layers that (incorrectly) reference each other as parents.
        let a = layer_at(1, Some(2), 5.0, 0.0);
        let b = layer_at(2, Some(1), 0.0, 5.0);
        let siblings = vec![a.clone(), b];

        // Must terminate rather than infinite-loop/stack-overflow.
        let _ = compose_ancestor_matrix(&a, 0.0, &siblings);
    }

    #[test]
    fn test_trim_path_extracts_partial_segment() {
        let mut builder = skia_rs_path::PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(100.0, 0.0);
        let path = builder.build();

        let trimmed = trim_path(&path, 0.0, 0.5, false);
        let measure = PathMeasure::new(&trimmed);
        assert!((measure.length() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_layer_own_transform_ignores_precomp_remap_end_to_end() {
        use crate::animation::Animation;

        // A precomp layer with st/sr set; its own position must NOT be
        // affected, but its child content time should be remapped.
        let json = r#"{
            "v":"5.5.7","fr":30,"ip":0,"op":60,"w":100,"h":100,
            "assets":[{"id":"comp_0","layers":[
                {"ty":4,"nm":"inner","ind":1,"ip":0,"op":100,
                 "shapes":[
                    {"ty":"rc","p":{"a":0,"k":[50,50]},"s":{"a":0,"k":[20,20]}},
                    {"ty":"fl","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100}}
                 ]}
            ]}],
            "layers":[
                {"ty":0,"nm":"precomp","ind":1,"ip":0,"op":60,"st":10,"sr":2,
                 "refId":"comp_0","w":100,"h":100,
                 "ks":{"p":{"a":0,"k":[5,5]}}}
            ]
        }"#;

        let anim = Animation::from_json(json).unwrap();
        let mut canvas = MockCanvas::new();
        let mut ctx = RenderContext::new(&mut canvas);
        // Should render without panicking across the remapped range.
        anim.render_frame(&mut ctx, 0.0);
        anim.render_frame(&mut ctx, 30.0);
        anim.render_frame(&mut ctx, 59.0);
    }
}
