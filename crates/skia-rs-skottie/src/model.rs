//! Lottie JSON data model.
//!
//! This module defines the data structures that map to the Lottie/Bodymovin
//! JSON format. These are deserialized from JSON and then converted to
//! internal animation structures.

use serde::{Deserialize, Serialize};
use skia_rs_core::Scalar;

/// Root Lottie animation model.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LottieModel {
    /// Lottie format version.
    #[serde(rename = "v")]
    pub version: String,
    /// Animation name.
    #[serde(rename = "nm", default)]
    pub name: String,
    /// Frames per second.
    #[serde(rename = "fr")]
    pub frame_rate: Scalar,
    /// In point (first frame).
    #[serde(rename = "ip")]
    pub in_point: Scalar,
    /// Out point (last frame).
    #[serde(rename = "op")]
    pub out_point: Scalar,
    /// Composition width.
    #[serde(rename = "w")]
    pub width: Scalar,
    /// Composition height.
    #[serde(rename = "h")]
    pub height: Scalar,
    /// Layers.
    #[serde(default)]
    pub layers: Vec<LayerModel>,
    /// Assets (precomps, images).
    #[serde(default)]
    pub assets: Vec<AssetModel>,
    /// Fonts.
    #[serde(default)]
    pub fonts: Option<FontList>,
    /// Characters (for text).
    #[serde(default)]
    pub chars: Option<Vec<CharModel>>,
    /// Markers.
    #[serde(default)]
    pub markers: Vec<MarkerModel>,
}

impl LottieModel {
    /// Get the total number of frames.
    pub fn total_frames(&self) -> Scalar {
        self.out_point - self.in_point
    }

    /// Get the duration in seconds.
    pub fn duration(&self) -> Scalar {
        self.total_frames() / self.frame_rate
    }
}

/// Layer model from Lottie JSON.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LayerModel {
    /// Layer name.
    #[serde(rename = "nm", default)]
    pub name: String,
    /// Layer type.
    #[serde(rename = "ty")]
    pub layer_type: i32,
    /// Layer index.
    #[serde(rename = "ind", default)]
    pub index: i32,
    /// Parent layer index.
    #[serde(rename = "parent", default)]
    pub parent: Option<i32>,
    /// In point.
    #[serde(rename = "ip")]
    pub in_point: Scalar,
    /// Out point.
    #[serde(rename = "op")]
    pub out_point: Scalar,
    /// Start time.
    #[serde(rename = "st", default)]
    pub start_time: Scalar,
    /// Transform.
    #[serde(rename = "ks", default)]
    pub transform: Option<TransformModel>,
    /// Auto-orient.
    #[serde(rename = "ao", default)]
    pub auto_orient: i32,
    /// Blend mode.
    #[serde(rename = "bm", default)]
    pub blend_mode: i32,
    /// 3D layer.
    #[serde(rename = "ddd", default)]
    pub is_3d: i32,
    /// Hidden.
    #[serde(rename = "hd", default)]
    pub hidden: bool,
    /// Shapes (for shape layers).
    #[serde(rename = "shapes", default)]
    pub shapes: Vec<ShapeModel>,
    /// Reference ID (for precomps/images).
    #[serde(rename = "refId", default)]
    pub ref_id: Option<String>,
    /// Solid color (for solid layers).
    #[serde(rename = "sc", default)]
    pub solid_color: Option<String>,
    /// Solid width.
    #[serde(rename = "sw", default)]
    pub solid_width: Option<Scalar>,
    /// Solid height.
    #[serde(rename = "sh", default)]
    pub solid_height: Option<Scalar>,
    /// Text data.
    #[serde(rename = "t", default)]
    pub text: Option<TextDataModel>,
    /// Masks.
    #[serde(rename = "masksProperties", default)]
    pub masks: Vec<MaskModel>,
    /// Track matte type.
    #[serde(rename = "tt", default)]
    pub track_matte_type: Option<i32>,
    /// Track matte layer.
    #[serde(rename = "td", default)]
    pub track_matte_layer: Option<i32>,
    /// Effects.
    #[serde(rename = "ef", default)]
    pub effects: Vec<EffectModel>,
}

/// Transform model.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TransformModel {
    /// Anchor point.
    #[serde(rename = "a", default)]
    pub anchor: Option<AnimatedValue>,
    /// Position.
    #[serde(rename = "p", default)]
    pub position: Option<AnimatedValue>,
    /// Scale.
    #[serde(rename = "s", default)]
    pub scale: Option<AnimatedValue>,
    /// Rotation.
    #[serde(rename = "r", default)]
    pub rotation: Option<AnimatedValue>,
    /// Opacity.
    #[serde(rename = "o", default)]
    pub opacity: Option<AnimatedValue>,
    /// Position X (separated).
    #[serde(rename = "px", default)]
    pub position_x: Option<AnimatedValue>,
    /// Position Y (separated).
    #[serde(rename = "py", default)]
    pub position_y: Option<AnimatedValue>,
    /// Skew.
    #[serde(rename = "sk", default)]
    pub skew: Option<AnimatedValue>,
    /// Skew axis.
    #[serde(rename = "sa", default)]
    pub skew_axis: Option<AnimatedValue>,
}

/// Animated value (can be static or keyframed).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AnimatedValue {
    /// Keyframed value.
    Animated {
        /// Is animated.
        #[serde(rename = "a", default)]
        animated: i32,
        /// Keyframes.
        #[serde(rename = "k")]
        keyframes: Vec<KeyframeModel>,
    },
    /// Static value (single keyframe).
    Static {
        /// Is animated.
        #[serde(rename = "a", default)]
        animated: i32,
        /// Value.
        #[serde(rename = "k")]
        value: serde_json::Value,
    },
    /// Direct value.
    Direct(serde_json::Value),
}

impl Default for AnimatedValue {
    fn default() -> Self {
        AnimatedValue::Direct(serde_json::Value::Array(vec![
            serde_json::Value::Number(0.into()),
        ]))
    }
}

/// Keyframe model.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyframeModel {
    /// Time.
    #[serde(rename = "t")]
    pub time: Scalar,
    /// Start value.
    #[serde(rename = "s", default)]
    pub start: Option<Vec<Scalar>>,
    /// End value.
    #[serde(rename = "e", default)]
    pub end: Option<Vec<Scalar>>,
    /// In tangent (bezier).
    #[serde(rename = "i", default)]
    pub in_tangent: Option<TangentModel>,
    /// Out tangent (bezier).
    #[serde(rename = "o", default)]
    pub out_tangent: Option<TangentModel>,
    /// Hold keyframe.
    #[serde(rename = "h", default)]
    pub hold: Option<i32>,
}

/// Tangent model for bezier easing.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TangentModel {
    /// X values.
    pub x: TangentValue,
    /// Y values.
    pub y: TangentValue,
}

/// Tangent value (can be single or array).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TangentValue {
    /// Single value.
    Single(Scalar),
    /// Array of values.
    Array(Vec<Scalar>),
}

impl TangentValue {
    /// Get the first value.
    pub fn first(&self) -> Scalar {
        match self {
            TangentValue::Single(v) => *v,
            TangentValue::Array(arr) => arr.first().copied().unwrap_or(0.0),
        }
    }
}

/// Shape model.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShapeModel {
    /// Shape type.
    #[serde(rename = "ty")]
    pub shape_type: String,
    /// Shape name.
    #[serde(rename = "nm", default)]
    pub name: String,
    /// Hidden.
    #[serde(rename = "hd", default)]
    pub hidden: bool,
    /// Match name.
    #[serde(rename = "mn", default)]
    pub match_name: Option<String>,
    /// Items (for groups).
    #[serde(rename = "it", default)]
    pub items: Vec<ShapeModel>,
    /// Path data (for paths).
    #[serde(rename = "ks", default)]
    pub path: Option<AnimatedValue>,
    /// Size (for rect/ellipse).
    #[serde(rename = "s", default)]
    pub size: Option<AnimatedValue>,
    /// Position.
    #[serde(rename = "p", default)]
    pub position: Option<AnimatedValue>,
    /// Roundness.
    #[serde(rename = "r", default)]
    pub roundness: Option<AnimatedValue>,
    /// Color.
    #[serde(rename = "c", default)]
    pub color: Option<AnimatedValue>,
    /// Opacity.
    #[serde(rename = "o", default)]
    pub opacity: Option<AnimatedValue>,
    /// Stroke width.
    #[serde(rename = "w", default)]
    pub stroke_width: Option<AnimatedValue>,
    /// Line cap.
    #[serde(rename = "lc", default)]
    pub line_cap: Option<i32>,
    /// Line join.
    #[serde(rename = "lj", default)]
    pub line_join: Option<i32>,
    /// Miter limit.
    #[serde(rename = "ml", default)]
    pub miter_limit: Option<Scalar>,
    /// Gradient type.
    #[serde(rename = "t", default)]
    pub gradient_type: Option<i32>,
    /// Gradient start point.
    #[serde(rename = "sp", default)]
    pub gradient_start: Option<AnimatedValue>,
    /// Gradient end point.
    #[serde(rename = "ep", default)]
    pub gradient_end: Option<AnimatedValue>,
    /// Gradient colors.
    #[serde(rename = "g", default)]
    pub gradient_colors: Option<GradientColorsModel>,
    /// Transform.
    #[serde(rename = "a", default)]
    pub transform: Option<AnimatedValue>,
    /// Points (for polystar).
    #[serde(rename = "pt", default)]
    pub points: Option<AnimatedValue>,
    /// Outer radius (for polystar).
    #[serde(rename = "or", default)]
    pub outer_radius: Option<AnimatedValue>,
    /// Inner radius (for polystar).
    #[serde(rename = "ir", default)]
    pub inner_radius: Option<AnimatedValue>,
    /// Star type (1=star, 2=polygon).
    #[serde(rename = "sy", default)]
    pub star_type: Option<i32>,
    /// Trim start (0-100%).
    #[serde(skip)]
    pub trim_start: Option<AnimatedValue>,
    /// Trim end (0-100%).
    #[serde(skip)]
    pub trim_end: Option<AnimatedValue>,
    /// Trim offset (degrees).
    #[serde(skip)]
    pub trim_offset: Option<AnimatedValue>,
    /// Multiple shapes mode.
    #[serde(rename = "m", default)]
    pub trim_mode: Option<i32>,
    /// Direction.
    #[serde(rename = "d", default)]
    pub direction: Option<i32>,
}

/// Gradient colors model.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GradientColorsModel {
    /// Number of colors.
    #[serde(rename = "p")]
    pub count: i32,
    /// Color values.
    #[serde(rename = "k")]
    pub colors: AnimatedValue,
}

/// Mask model.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MaskModel {
    /// Mask mode.
    #[serde(rename = "mode")]
    pub mode: String,
    /// Mask path.
    #[serde(rename = "pt")]
    pub path: AnimatedValue,
    /// Mask opacity.
    #[serde(rename = "o")]
    pub opacity: AnimatedValue,
    /// Inverted.
    #[serde(rename = "inv", default)]
    pub inverted: bool,
    /// Mask expansion.
    #[serde(rename = "x", default)]
    pub expansion: Option<AnimatedValue>,
}

/// Asset model (precomp or image).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetModel {
    /// Asset ID.
    pub id: String,
    /// Layers (for precomps).
    #[serde(default)]
    pub layers: Vec<LayerModel>,
    /// Width (for images/precomps).
    #[serde(rename = "w", default)]
    pub width: Option<Scalar>,
    /// Height (for images/precomps).
    #[serde(rename = "h", default)]
    pub height: Option<Scalar>,
    /// Image path.
    #[serde(rename = "u", default)]
    pub path: Option<String>,
    /// Image filename.
    #[serde(rename = "p", default)]
    pub filename: Option<String>,
    /// Embedded image data (base64).
    #[serde(rename = "e", default)]
    pub embedded: Option<i32>,
}

/// Font list.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FontList {
    /// Fonts.
    pub list: Vec<FontModel>,
}

/// Font model.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FontModel {
    /// Font family.
    #[serde(rename = "fFamily")]
    pub family: String,
    /// Font name.
    #[serde(rename = "fName")]
    pub name: String,
    /// Font style.
    #[serde(rename = "fStyle")]
    pub style: String,
    /// Font path.
    #[serde(rename = "fPath", default)]
    pub path: Option<String>,
    /// Font weight.
    #[serde(rename = "fWeight", default)]
    pub weight: Option<String>,
}

/// Character model (for text).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharModel {
    /// Character.
    pub ch: String,
    /// Font family.
    #[serde(rename = "fFamily")]
    pub family: String,
    /// Font size.
    pub size: Scalar,
    /// Font style.
    pub style: String,
    /// Width.
    pub w: Scalar,
    /// Shape data.
    #[serde(default)]
    pub data: Option<CharDataModel>,
}

/// Character data.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharDataModel {
    /// Shapes.
    #[serde(default)]
    pub shapes: Vec<ShapeModel>,
}

/// Text data model.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextDataModel {
    /// Document keyframes.
    #[serde(rename = "d")]
    pub document: TextDocumentModel,
}

/// Text document.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextDocumentModel {
    /// Keyframes.
    #[serde(rename = "k")]
    pub keyframes: Vec<TextDocumentKeyframe>,
}

/// Text document keyframe.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextDocumentKeyframe {
    /// Start time.
    #[serde(rename = "s")]
    pub data: TextDocumentData,
    /// Time.
    #[serde(rename = "t", default)]
    pub time: Scalar,
}

/// Text document data.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextDocumentData {
    /// Text.
    #[serde(rename = "t")]
    pub text: String,
    /// Font size.
    #[serde(rename = "s")]
    pub size: Scalar,
    /// Font family.
    #[serde(rename = "f")]
    pub font: String,
    /// Fill color.
    #[serde(rename = "fc", default)]
    pub fill_color: Option<Vec<Scalar>>,
    /// Stroke color.
    #[serde(rename = "sc", default)]
    pub stroke_color: Option<Vec<Scalar>>,
    /// Stroke width.
    #[serde(rename = "sw", default)]
    pub stroke_width: Option<Scalar>,
    /// Justification.
    #[serde(rename = "j", default)]
    pub justify: Option<i32>,
    /// Tracking.
    #[serde(rename = "tr", default)]
    pub tracking: Option<Scalar>,
    /// Line height.
    #[serde(rename = "lh", default)]
    pub line_height: Option<Scalar>,
}

/// Marker model.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarkerModel {
    /// Comment.
    #[serde(rename = "cm")]
    pub comment: String,
    /// Time.
    #[serde(rename = "tm")]
    pub time: Scalar,
    /// Duration.
    #[serde(rename = "dr")]
    pub duration: Scalar,
}

/// Effect model.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EffectModel {
    /// Effect type.
    #[serde(rename = "ty")]
    pub effect_type: i32,
    /// Effect name.
    #[serde(rename = "nm", default)]
    pub name: String,
    /// Effect values.
    #[serde(rename = "ef", default)]
    pub values: Vec<EffectValueModel>,
}

/// Effect value.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EffectValueModel {
    /// Value type.
    #[serde(rename = "ty")]
    pub value_type: i32,
    /// Name.
    #[serde(rename = "nm", default)]
    pub name: String,
    /// Value.
    #[serde(rename = "v", default)]
    pub value: Option<AnimatedValue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_lottie() {
        let json = r#"{
            "v": "5.5.7",
            "nm": "Test",
            "fr": 30,
            "ip": 0,
            "op": 60,
            "w": 100,
            "h": 100,
            "layers": []
        }"#;

        let model: LottieModel = serde_json::from_str(json).unwrap();
        assert_eq!(model.name, "Test");
        assert_eq!(model.frame_rate, 30.0);
        assert_eq!(model.total_frames(), 60.0);
        assert_eq!(model.duration(), 2.0);
    }

    #[test]
    fn test_parse_layer() {
        let json = r#"{
            "ty": 4,
            "nm": "Shape Layer",
            "ind": 1,
            "ip": 0,
            "op": 60,
            "st": 0,
            "shapes": []
        }"#;

        let layer: LayerModel = serde_json::from_str(json).unwrap();
        assert_eq!(layer.layer_type, 4);
        assert_eq!(layer.name, "Shape Layer");
    }
}
