//! Animation playback and control.
//!
//! This module provides the main `Animation` type for loading and
//! rendering Lottie animations.

use crate::layers::{Layer, LayerContent, ShapeContent};
use crate::model::{AssetModel, LottieModel};
use crate::render::RenderContext;
use crate::{Result, SkottieError};
use skia_rs_core::{Matrix, Rect, Scalar};
use std::collections::HashMap;
use std::sync::Arc;

/// A loaded Lottie animation.
#[derive(Debug, Clone)]
pub struct Animation {
    /// Animation name.
    name: String,
    /// Lottie format version.
    version: String,
    /// Width in pixels.
    width: Scalar,
    /// Height in pixels.
    height: Scalar,
    /// Frame rate (fps).
    frame_rate: Scalar,
    /// In point (first frame).
    in_point: Scalar,
    /// Out point (last frame).
    out_point: Scalar,
    /// Layers.
    layers: Vec<Layer>,
    /// Assets (precomps, images).
    assets: HashMap<String, Asset>,
    /// Current frame.
    current_frame: Scalar,
}

/// Asset types.
#[derive(Debug, Clone)]
pub enum Asset {
    /// Precomposition.
    Precomp(PrecompAsset),
    /// Image asset.
    Image(ImageAsset),
}

/// Precomposition asset.
#[derive(Debug, Clone)]
pub struct PrecompAsset {
    /// Asset ID.
    pub id: String,
    /// Width.
    pub width: Scalar,
    /// Height.
    pub height: Scalar,
    /// Layers.
    pub layers: Vec<Layer>,
}

/// Image asset.
#[derive(Debug, Clone)]
pub struct ImageAsset {
    /// Asset ID.
    pub id: String,
    /// Width.
    pub width: Scalar,
    /// Height.
    pub height: Scalar,
    /// Path.
    pub path: String,
    /// Filename.
    pub filename: String,
    /// Embedded data (base64).
    pub embedded_data: Option<String>,
}

impl Animation {
    /// Load an animation from a JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        let model: LottieModel = serde_json::from_str(json)?;
        Self::from_model(model)
    }

    /// Load an animation from a JSON byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let model: LottieModel = serde_json::from_slice(bytes)?;
        Self::from_model(model)
    }

    /// Load an animation from a file.
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_json(&contents)
    }

    /// Build an animation from a parsed Lottie model.
    fn from_model(model: LottieModel) -> Result<Self> {
        // Parse layers
        let layers: Vec<Layer> = model
            .layers
            .iter()
            .map(Layer::from_lottie)
            .collect();

        // Parse assets
        let mut assets = HashMap::new();
        for asset in &model.assets {
            if !asset.layers.is_empty() {
                // Precomp
                let precomp = PrecompAsset {
                    id: asset.id.clone(),
                    width: asset.width.unwrap_or(model.width),
                    height: asset.height.unwrap_or(model.height),
                    layers: asset.layers.iter().map(Layer::from_lottie).collect(),
                };
                assets.insert(asset.id.clone(), Asset::Precomp(precomp));
            } else if asset.filename.is_some() {
                // Image
                let image = ImageAsset {
                    id: asset.id.clone(),
                    width: asset.width.unwrap_or(0.0),
                    height: asset.height.unwrap_or(0.0),
                    path: asset.path.clone().unwrap_or_default(),
                    filename: asset.filename.clone().unwrap_or_default(),
                    embedded_data: if asset.embedded == Some(1) {
                        asset.filename.clone()
                    } else {
                        None
                    },
                };
                assets.insert(asset.id.clone(), Asset::Image(image));
            }
        }

        Ok(Self {
            name: model.name,
            version: model.version,
            width: model.width,
            height: model.height,
            frame_rate: model.frame_rate,
            in_point: model.in_point,
            out_point: model.out_point,
            layers,
            assets,
            current_frame: model.in_point,
        })
    }

    /// Get the animation name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the Lottie format version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get the animation width.
    pub fn width(&self) -> Scalar {
        self.width
    }

    /// Get the animation height.
    pub fn height(&self) -> Scalar {
        self.height
    }

    /// Get the frame rate (fps).
    pub fn fps(&self) -> Scalar {
        self.frame_rate
    }

    /// Get the in point (first frame).
    pub fn in_point(&self) -> Scalar {
        self.in_point
    }

    /// Get the out point (last frame).
    pub fn out_point(&self) -> Scalar {
        self.out_point
    }

    /// Get the total number of frames.
    pub fn total_frames(&self) -> Scalar {
        self.out_point - self.in_point
    }

    /// Get the duration in seconds.
    pub fn duration(&self) -> Scalar {
        self.total_frames() / self.frame_rate
    }

    /// Get the current frame.
    pub fn current_frame(&self) -> Scalar {
        self.current_frame
    }

    /// Get the bounding rect.
    pub fn bounds(&self) -> Rect {
        Rect::from_xywh(0.0, 0.0, self.width, self.height)
    }

    /// Get the layers.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Get an asset by ID.
    pub fn asset(&self, id: &str) -> Option<&Asset> {
        self.assets.get(id)
    }

    /// Seek to a specific frame.
    pub fn seek_frame(&mut self, frame: Scalar) {
        self.current_frame = frame.clamp(self.in_point, self.out_point - 0.001);
    }

    /// Seek to a normalized position (0.0 - 1.0).
    pub fn seek(&mut self, t: Scalar) {
        let frame = self.in_point + t.clamp(0.0, 1.0) * self.total_frames();
        self.seek_frame(frame);
    }

    /// Seek to a specific time in seconds.
    pub fn seek_time(&mut self, seconds: Scalar) {
        let frame = self.in_point + seconds * self.frame_rate;
        self.seek_frame(frame);
    }

    /// Advance by a time delta in seconds.
    pub fn advance(&mut self, delta_seconds: Scalar) {
        let new_frame = self.current_frame + delta_seconds * self.frame_rate;

        // Loop animation
        if new_frame >= self.out_point {
            self.current_frame = self.in_point + (new_frame - self.out_point) % self.total_frames();
        } else {
            self.current_frame = new_frame;
        }
    }

    /// Advance by a time delta with optional looping.
    pub fn advance_with_loop(&mut self, delta_seconds: Scalar, should_loop: bool) {
        let new_frame = self.current_frame + delta_seconds * self.frame_rate;

        if new_frame >= self.out_point {
            if should_loop {
                self.current_frame = self.in_point + (new_frame - self.out_point) % self.total_frames();
            } else {
                self.current_frame = self.out_point - 0.001;
            }
        } else if new_frame < self.in_point {
            if should_loop {
                self.current_frame = self.out_point - (self.in_point - new_frame) % self.total_frames();
            } else {
                self.current_frame = self.in_point;
            }
        } else {
            self.current_frame = new_frame;
        }
    }

    /// Render the animation at the current frame.
    pub fn render(&self, ctx: &mut RenderContext) {
        self.render_frame(ctx, self.current_frame);
    }

    /// Render a specific frame.
    pub fn render_frame(&self, ctx: &mut RenderContext, frame: Scalar) {
        ctx.save();

        // Render layers in reverse order (bottom to top)
        for layer in self.layers.iter().rev() {
            if layer.is_visible_at(frame) {
                ctx.render_layer(layer, frame, &self.assets);
            }
        }

        ctx.restore();
    }

    /// Render to a target rect (scales to fit).
    pub fn render_to_rect(&self, ctx: &mut RenderContext, rect: &Rect) {
        let scale_x = rect.width() / self.width;
        let scale_y = rect.height() / self.height;
        let scale = scale_x.min(scale_y);

        let offset_x = rect.left + (rect.width() - self.width * scale) / 2.0;
        let offset_y = rect.top + (rect.height() - self.height * scale) / 2.0;

        ctx.save();
        ctx.concat(&Matrix::translate(offset_x, offset_y));
        ctx.concat(&Matrix::scale(scale, scale));

        self.render(ctx);

        ctx.restore();
    }

    /// Get statistics about the animation.
    pub fn stats(&self) -> AnimationStats {
        let mut shape_layer_count = 0;
        let mut precomp_layer_count = 0;
        let mut solid_layer_count = 0;
        let mut text_layer_count = 0;
        let mut image_layer_count = 0;
        let mut null_layer_count = 0;
        let mut total_shapes = 0;
        let mut total_masks = 0;

        for layer in &self.layers {
            match &layer.content {
                LayerContent::Shape(content) => {
                    shape_layer_count += 1;
                    total_shapes += count_shapes(&content.shapes);
                }
                LayerContent::Precomp(_) => precomp_layer_count += 1,
                LayerContent::Solid(_) => solid_layer_count += 1,
                LayerContent::Text(_) => text_layer_count += 1,
                LayerContent::Image(_) => image_layer_count += 1,
                LayerContent::None => null_layer_count += 1,
            }
            total_masks += layer.masks.len();
        }

        AnimationStats {
            name: self.name.clone(),
            version: self.version.clone(),
            width: self.width as u32,
            height: self.height as u32,
            frame_rate: self.frame_rate,
            duration_seconds: self.duration(),
            total_frames: self.total_frames() as u32,
            layer_count: self.layers.len(),
            shape_layer_count,
            precomp_layer_count,
            solid_layer_count,
            text_layer_count,
            image_layer_count,
            null_layer_count,
            total_shapes,
            total_masks,
            asset_count: self.assets.len(),
        }
    }
}

fn count_shapes(shapes: &[crate::shapes::Shape]) -> usize {
    let mut count = 0;
    for shape in shapes {
        count += 1;
        if let crate::shapes::Shape::Group(group) = shape {
            count += count_shapes(&group.shapes);
        }
    }
    count
}

/// Animation statistics.
#[derive(Debug, Clone)]
pub struct AnimationStats {
    /// Animation name.
    pub name: String,
    /// Lottie version.
    pub version: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Frame rate.
    pub frame_rate: Scalar,
    /// Duration in seconds.
    pub duration_seconds: Scalar,
    /// Total frames.
    pub total_frames: u32,
    /// Number of layers.
    pub layer_count: usize,
    /// Shape layers.
    pub shape_layer_count: usize,
    /// Precomp layers.
    pub precomp_layer_count: usize,
    /// Solid layers.
    pub solid_layer_count: usize,
    /// Text layers.
    pub text_layer_count: usize,
    /// Image layers.
    pub image_layer_count: usize,
    /// Null layers.
    pub null_layer_count: usize,
    /// Total shape count.
    pub total_shapes: usize,
    /// Total mask count.
    pub total_masks: usize,
    /// Asset count.
    pub asset_count: usize,
}

/// Builder for loading animations with options.
#[derive(Debug, Clone, Default)]
pub struct AnimationBuilder {
    /// Resource provider for images.
    resource_provider: Option<Arc<dyn ResourceProvider>>,
    /// Font provider for text.
    font_provider: Option<Arc<dyn FontProvider>>,
}

impl AnimationBuilder {
    /// Create a new animation builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the resource provider for loading images.
    pub fn with_resource_provider(mut self, provider: Arc<dyn ResourceProvider>) -> Self {
        self.resource_provider = Some(provider);
        self
    }

    /// Set the font provider for loading fonts.
    pub fn with_font_provider(mut self, provider: Arc<dyn FontProvider>) -> Self {
        self.font_provider = Some(provider);
        self
    }

    /// Load an animation from JSON.
    pub fn load(self, json: &str) -> Result<Animation> {
        Animation::from_json(json)
    }

    /// Load an animation from a file.
    pub fn load_file(self, path: &std::path::Path) -> Result<Animation> {
        Animation::from_file(path)
    }
}

/// Resource provider for loading external assets.
pub trait ResourceProvider: Send + Sync + std::fmt::Debug {
    /// Load an image asset.
    fn load_image(&self, path: &str, name: &str) -> Option<Vec<u8>>;
}

/// Font provider for loading fonts.
pub trait FontProvider: Send + Sync + std::fmt::Debug {
    /// Load a font by family name.
    fn load_font(&self, family: &str, style: &str) -> Option<Vec<u8>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_ANIMATION: &str = r#"{
        "v": "5.5.7",
        "nm": "Test Animation",
        "fr": 30,
        "ip": 0,
        "op": 60,
        "w": 200,
        "h": 200,
        "layers": []
    }"#;

    #[test]
    fn test_load_animation() {
        let anim = Animation::from_json(SIMPLE_ANIMATION).unwrap();

        assert_eq!(anim.name(), "Test Animation");
        assert_eq!(anim.width(), 200.0);
        assert_eq!(anim.height(), 200.0);
        assert_eq!(anim.fps(), 30.0);
        assert_eq!(anim.total_frames(), 60.0);
        assert_eq!(anim.duration(), 2.0);
    }

    #[test]
    fn test_seek() {
        let mut anim = Animation::from_json(SIMPLE_ANIMATION).unwrap();

        anim.seek(0.5);
        assert_eq!(anim.current_frame(), 30.0);

        anim.seek_frame(45.0);
        assert_eq!(anim.current_frame(), 45.0);

        anim.seek_time(1.0);
        assert_eq!(anim.current_frame(), 30.0);
    }

    #[test]
    fn test_advance() {
        let mut anim = Animation::from_json(SIMPLE_ANIMATION).unwrap();

        anim.advance(0.5); // Advance 0.5 seconds = 15 frames
        assert_eq!(anim.current_frame(), 15.0);

        // Test looping
        anim.seek_frame(55.0);
        anim.advance(0.5); // Should loop
        assert!(anim.current_frame() < 20.0); // Wrapped around
    }

    #[test]
    fn test_stats() {
        let anim = Animation::from_json(SIMPLE_ANIMATION).unwrap();
        let stats = anim.stats();

        assert_eq!(stats.name, "Test Animation");
        assert_eq!(stats.layer_count, 0);
        assert_eq!(stats.total_frames, 60);
    }

    #[test]
    fn test_builder() {
        let anim = AnimationBuilder::new()
            .load(SIMPLE_ANIMATION)
            .unwrap();

        assert_eq!(anim.name(), "Test Animation");
    }
}
