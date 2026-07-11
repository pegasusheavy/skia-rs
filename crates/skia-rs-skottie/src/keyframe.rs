//! Keyframe animation and interpolation.
//!
//! This module provides keyframe-based animation with support for:
//! - Linear interpolation
//! - Bezier easing curves
//! - Hold keyframes
//! - Multi-dimensional values

use crate::model::{AnimatedValue, TangentModel};
use skia_rs_core::Scalar;

/// Easing function for keyframe interpolation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    /// Linear interpolation.
    Linear,
    /// Hold (step function).
    Hold,
    /// Bezier curve easing.
    Bezier {
        /// Out X (from previous keyframe).
        out_x: Scalar,
        /// Out Y (from previous keyframe).
        out_y: Scalar,
        /// In X (to next keyframe).
        in_x: Scalar,
        /// In Y (to next keyframe).
        in_y: Scalar,
    },
}

impl Default for Easing {
    fn default() -> Self {
        Easing::Linear
    }
}

impl Easing {
    /// Create a bezier easing from tangent models.
    pub fn from_tangents(out_tangent: &TangentModel, in_tangent: &TangentModel) -> Self {
        Easing::Bezier {
            out_x: out_tangent.x.first(),
            out_y: out_tangent.y.first(),
            in_x: in_tangent.x.first(),
            in_y: in_tangent.y.first(),
        }
    }

    /// Evaluate the easing function at time t (0..1).
    pub fn evaluate(&self, t: Scalar) -> Scalar {
        match self {
            Easing::Linear => t,
            Easing::Hold => 0.0,
            Easing::Bezier {
                out_x,
                out_y,
                in_x,
                in_y,
            } => {
                // Cubic bezier: P0=(0,0), P1=(out_x,out_y), P2=(in_x,in_y), P3=(1,1)
                // Find t for given x, then evaluate y
                let x_t = solve_cubic_bezier_t(*out_x, *in_x, t);
                cubic_bezier_y(*out_y, *in_y, x_t)
            }
        }
    }
}

/// Solve for t in cubic bezier given x.
fn solve_cubic_bezier_t(p1: Scalar, p2: Scalar, x: Scalar) -> Scalar {
    // Newton-Raphson iteration to find t where B(t).x = x
    let mut t = x;

    for _ in 0..8 {
        let x_at_t = cubic_bezier_x(p1, p2, t);
        let error = x_at_t - x;

        if error.abs() < 0.0001 {
            return t;
        }

        let dx = cubic_bezier_dx(p1, p2, t);
        if dx.abs() < 0.0001 {
            break;
        }

        t -= error / dx;
    }

    t.clamp(0.0, 1.0)
}

/// Cubic bezier x coordinate.
fn cubic_bezier_x(p1: Scalar, p2: Scalar, t: Scalar) -> Scalar {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let _mt3 = mt2 * mt;

    3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t3
}

/// Cubic bezier x derivative.
fn cubic_bezier_dx(p1: Scalar, p2: Scalar, t: Scalar) -> Scalar {
    let t2 = t * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;

    3.0 * mt2 * p1 + 6.0 * mt * t * (p2 - p1) + 3.0 * t2 * (1.0 - p2)
}

/// Cubic bezier y coordinate.
fn cubic_bezier_y(p1: Scalar, p2: Scalar, t: Scalar) -> Scalar {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let _mt3 = mt2 * mt;

    3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t3
}

/// A single keyframe in an animation.
#[derive(Debug, Clone)]
pub struct Keyframe {
    /// Time of this keyframe.
    pub time: Scalar,
    /// Value at this keyframe.
    pub value: KeyframeValue,
    /// Easing to next keyframe.
    pub easing: Easing,
    /// Spatial out-tangent (`to`), relative to this keyframe's value —
    /// only meaningful for `Vec2`/`Vec3` position properties. Per
    /// upstream `Vec2KeyframeAnimator`, the bezier motion path segment
    /// from this keyframe to the next is
    /// `cubicTo(value + spatial_out, next.value + next.spatial_in, next.value)`.
    pub spatial_out: Option<[Scalar; 2]>,
    /// Spatial in-tangent (`ti`), relative to this keyframe's value.
    pub spatial_in: Option<[Scalar; 2]>,
}

impl Keyframe {
    /// Create a new keyframe.
    pub fn new(time: Scalar, value: KeyframeValue) -> Self {
        Self {
            time,
            value,
            easing: Easing::Linear,
            spatial_out: None,
            spatial_in: None,
        }
    }

    /// Set the easing function.
    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

/// Value type for keyframes.
#[derive(Debug, Clone)]
pub enum KeyframeValue {
    /// Scalar value.
    Scalar(Scalar),
    /// 2D point/vector.
    Vec2([Scalar; 2]),
    /// 3D point/vector.
    Vec3([Scalar; 3]),
    /// Color (RGBA).
    Color([Scalar; 4]),
    /// Path data.
    Path(PathData),
    /// Raw float array (used for gradient stop tables, which pack an
    /// arbitrary number of color/opacity records).
    FloatArray(Vec<Scalar>),
}

impl KeyframeValue {
    /// Get as scalar.
    pub fn as_scalar(&self) -> Option<Scalar> {
        match self {
            KeyframeValue::Scalar(v) => Some(*v),
            KeyframeValue::Vec2(v) => Some(v[0]),
            _ => None,
        }
    }

    /// Get as vec2.
    ///
    /// Bodymovin frequently exports positions/anchors as 3-component
    /// arrays (with a z of 0 for 2D layers); accept `Vec3` by taking the
    /// first two components.
    pub fn as_vec2(&self) -> Option<[Scalar; 2]> {
        match self {
            KeyframeValue::Vec2(v) => Some(*v),
            KeyframeValue::Vec3(v) => Some([v[0], v[1]]),
            KeyframeValue::Scalar(v) => Some([*v, *v]),
            _ => None,
        }
    }

    /// Get as vec3.
    pub fn as_vec3(&self) -> Option<[Scalar; 3]> {
        match self {
            KeyframeValue::Vec3(v) => Some(*v),
            KeyframeValue::Vec2(v) => Some([v[0], v[1], 0.0]),
            KeyframeValue::Scalar(v) => Some([*v, *v, *v]),
            _ => None,
        }
    }

    /// Get as color.
    pub fn as_color(&self) -> Option<[Scalar; 4]> {
        match self {
            KeyframeValue::Color(v) => Some(*v),
            KeyframeValue::Vec3(v) => Some([v[0], v[1], v[2], 1.0]),
            _ => None,
        }
    }

    /// Get as a raw float array (e.g. gradient stop tables).
    pub fn as_float_array(&self) -> Option<&[Scalar]> {
        match self {
            KeyframeValue::FloatArray(v) => Some(v),
            _ => None,
        }
    }

    /// Interpolate between two values.
    pub fn lerp(&self, other: &KeyframeValue, t: Scalar) -> KeyframeValue {
        match (self, other) {
            (KeyframeValue::Scalar(a), KeyframeValue::Scalar(b)) => {
                KeyframeValue::Scalar(a + (b - a) * t)
            }
            (KeyframeValue::Vec2(a), KeyframeValue::Vec2(b)) => {
                KeyframeValue::Vec2([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t])
            }
            (KeyframeValue::Vec3(a), KeyframeValue::Vec3(b)) => KeyframeValue::Vec3([
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]),
            (KeyframeValue::Color(a), KeyframeValue::Color(b)) => KeyframeValue::Color([
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
                a[3] + (b[3] - a[3]) * t,
            ]),
            (KeyframeValue::Path(a), KeyframeValue::Path(b)) => KeyframeValue::Path(a.lerp(b, t)),
            (KeyframeValue::FloatArray(a), KeyframeValue::FloatArray(b))
                if a.len() == b.len() =>
            {
                KeyframeValue::FloatArray(
                    a.iter()
                        .zip(b.iter())
                        .map(|(x, y)| x + (y - x) * t)
                        .collect(),
                )
            }
            // Mismatched types - return first
            _ => self.clone(),
        }
    }
}

/// Path data for shape keyframes.
#[derive(Debug, Clone)]
pub struct PathData {
    /// Control points.
    pub vertices: Vec<[Scalar; 2]>,
    /// In tangents.
    pub in_tangents: Vec<[Scalar; 2]>,
    /// Out tangents.
    pub out_tangents: Vec<[Scalar; 2]>,
    /// Closed path.
    pub closed: bool,
}

impl PathData {
    /// Create an empty path.
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            in_tangents: Vec::new(),
            out_tangents: Vec::new(),
            closed: false,
        }
    }

    /// Interpolate between two paths.
    pub fn lerp(&self, other: &PathData, t: Scalar) -> PathData {
        let len = self.vertices.len().min(other.vertices.len());

        PathData {
            vertices: (0..len)
                .map(|i| {
                    [
                        self.vertices[i][0] + (other.vertices[i][0] - self.vertices[i][0]) * t,
                        self.vertices[i][1] + (other.vertices[i][1] - self.vertices[i][1]) * t,
                    ]
                })
                .collect(),
            in_tangents: (0..len)
                .map(|i| {
                    [
                        self.in_tangents.get(i).map(|v| v[0]).unwrap_or(0.0)
                            + (other.in_tangents.get(i).map(|v| v[0]).unwrap_or(0.0)
                                - self.in_tangents.get(i).map(|v| v[0]).unwrap_or(0.0))
                                * t,
                        self.in_tangents.get(i).map(|v| v[1]).unwrap_or(0.0)
                            + (other.in_tangents.get(i).map(|v| v[1]).unwrap_or(0.0)
                                - self.in_tangents.get(i).map(|v| v[1]).unwrap_or(0.0))
                                * t,
                    ]
                })
                .collect(),
            out_tangents: (0..len)
                .map(|i| {
                    [
                        self.out_tangents.get(i).map(|v| v[0]).unwrap_or(0.0)
                            + (other.out_tangents.get(i).map(|v| v[0]).unwrap_or(0.0)
                                - self.out_tangents.get(i).map(|v| v[0]).unwrap_or(0.0))
                                * t,
                        self.out_tangents.get(i).map(|v| v[1]).unwrap_or(0.0)
                            + (other.out_tangents.get(i).map(|v| v[1]).unwrap_or(0.0)
                                - self.out_tangents.get(i).map(|v| v[1]).unwrap_or(0.0))
                                * t,
                    ]
                })
                .collect(),
            closed: self.closed || other.closed,
        }
    }
}

impl Default for PathData {
    fn default() -> Self {
        Self::new()
    }
}

/// An animated property with keyframes.
#[derive(Debug, Clone)]
pub struct AnimatedProperty {
    /// Keyframes (sorted by time).
    pub keyframes: Vec<Keyframe>,
}

impl AnimatedProperty {
    /// Create a new animated property.
    pub fn new() -> Self {
        Self {
            keyframes: Vec::new(),
        }
    }

    /// Create from a static value.
    pub fn static_value(value: KeyframeValue) -> Self {
        Self {
            keyframes: vec![Keyframe::new(0.0, value)],
        }
    }

    /// Add a keyframe.
    pub fn add_keyframe(&mut self, keyframe: Keyframe) {
        self.keyframes.push(keyframe);
        self.keyframes
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Check if this property is animated.
    pub fn is_animated(&self) -> bool {
        self.keyframes.len() > 1
    }

    /// Get the value at a specific frame.
    pub fn value_at(&self, frame: Scalar) -> KeyframeValue {
        if self.keyframes.is_empty() {
            return KeyframeValue::Scalar(0.0);
        }

        if self.keyframes.len() == 1 {
            return self.keyframes[0].value.clone();
        }

        // Find surrounding keyframes
        let mut prev_idx = 0;
        let mut next_idx = 0;

        for (i, kf) in self.keyframes.iter().enumerate() {
            if kf.time <= frame {
                prev_idx = i;
            }
            if kf.time >= frame {
                next_idx = i;
                break;
            }
            next_idx = i;
        }

        let prev = &self.keyframes[prev_idx];
        let next = &self.keyframes[next_idx];

        // Same keyframe or hold
        if prev_idx == next_idx || matches!(prev.easing, Easing::Hold) {
            return prev.value.clone();
        }

        // Calculate interpolation factor
        let duration = next.time - prev.time;
        if duration <= 0.0 {
            return prev.value.clone();
        }

        let linear_t = (frame - prev.time) / duration;
        let eased_t = prev.easing.evaluate(linear_t);

        match (&prev.value, &next.value) {
            (KeyframeValue::Vec2(v0), KeyframeValue::Vec2(v1)) => {
                if let Some(pos) =
                    spatial_bezier_at(*v0, prev.spatial_out, *v1, next.spatial_in, eased_t)
                {
                    return KeyframeValue::Vec2(pos);
                }
            }
            // Bodymovin commonly exports positions as 3-component arrays
            // (z = 0 for 2D layers): the spatial bezier applies to x/y,
            // z interpolates linearly.
            (KeyframeValue::Vec3(v0), KeyframeValue::Vec3(v1)) => {
                if let Some(pos) = spatial_bezier_at(
                    [v0[0], v0[1]],
                    prev.spatial_out,
                    [v1[0], v1[1]],
                    next.spatial_in,
                    eased_t,
                ) {
                    let z = v0[2] + (v1[2] - v0[2]) * eased_t;
                    return KeyframeValue::Vec3([pos[0], pos[1], z]);
                }
            }
            _ => {}
        }

        prev.value.lerp(&next.value, eased_t)
    }

    /// Parse from Lottie animated value.
    pub fn from_lottie(value: &AnimatedValue) -> Self {
        match value {
            AnimatedValue::Animated { keyframes, .. } => {
                let mut prop = Self::new();

                for kf in keyframes.iter() {
                    let value = if let Some(ref start) = kf.start {
                        parse_json_value(start)
                    } else if let Some(ref end) = kf.end {
                        parse_json_value(end)
                    } else if let Some(prev) = prop.keyframes.last() {
                        // A trailing `{"t":N}`-only keyframe (no "s"/"e")
                        // just marks an end time and inherits the previous
                        // keyframe's value.
                        prev.value.clone()
                    } else {
                        KeyframeValue::Scalar(0.0)
                    };

                    let easing = if kf.hold == Some(1) {
                        Easing::Hold
                    } else if let (Some(out_t), Some(in_t)) = (&kf.out_tangent, &kf.in_tangent) {
                        Easing::from_tangents(out_t, in_t)
                    } else {
                        Easing::Linear
                    };

                    let spatial_out = kf
                        .spatial_out_tangent
                        .as_ref()
                        .filter(|v| v.len() >= 2)
                        .map(|v| [v[0], v[1]]);
                    let spatial_in = kf
                        .spatial_in_tangent
                        .as_ref()
                        .filter(|v| v.len() >= 2)
                        .map(|v| [v[0], v[1]]);

                    prop.add_keyframe(Keyframe {
                        time: kf.time,
                        value,
                        easing,
                        spatial_out,
                        spatial_in,
                    });
                }

                prop
            }
            AnimatedValue::Static { value, .. } => {
                let kf_value = parse_json_value(value);
                Self::static_value(kf_value)
            }
            AnimatedValue::Direct(value) => {
                let kf_value = parse_json_value(value);
                Self::static_value(kf_value)
            }
        }
    }
}

impl Default for AnimatedProperty {
    fn default() -> Self {
        Self::new()
    }
}

/// Interpolate a position along the spatial bezier motion path defined by
/// `ti`/`to` tangents, matching upstream `Vec2KeyframeAnimator::onSeek`:
/// build the cubic `v0 -> v0+to -> v1+ti -> v1`, then walk it by arc length
/// using `weight` (the *eased* interpolation factor) as the length
/// fraction — not as the raw bezier parameter — so easing curves control
/// speed along the path rather than the curve parametrization.
///
/// Returns `None` when neither tangent is present (caller falls back to a
/// plain linear lerp).
fn spatial_bezier_at(
    v0: [Scalar; 2],
    to: Option<[Scalar; 2]>,
    v1: [Scalar; 2],
    ti: Option<[Scalar; 2]>,
    weight: Scalar,
) -> Option<[Scalar; 2]> {
    let to = to.unwrap_or([0.0, 0.0]);
    let ti = ti.unwrap_or([0.0, 0.0]);
    if to == [0.0, 0.0] && ti == [0.0, 0.0] {
        return None;
    }
    if v0 == v1 {
        // Spatial interpolation only makes sense for noncoincident values.
        return None;
    }

    let mut builder = skia_rs_path::PathBuilder::new();
    builder.move_to(v0[0], v0[1]);
    builder.cubic_to(
        v0[0] + to[0],
        v0[1] + to[1],
        v1[0] + ti[0],
        v1[1] + ti[1],
        v1[0],
        v1[1],
    );
    let path = builder.build();
    let measure = skia_rs_path::PathMeasure::new(&path);
    let len = measure.length();
    if len <= 0.0 {
        return None;
    }

    let distance = len * weight;
    let clamped = distance.clamp(0.0, len);
    let point = measure.get_point_at(clamped)?;

    if distance < 0.0 || distance > len {
        // Extrapolate past the endpoints using the endpoint tangent, matching
        // upstream's overshoot handling for sub/super-normal easing weights.
        let tan = measure.get_tangent_at(clamped)?;
        let overshoot = distance - clamped;
        return Some([point.x + tan.x * overshoot, point.y + tan.y * overshoot]);
    }

    Some([point.x, point.y])
}

fn parse_keyframe_value(values: &[Scalar]) -> KeyframeValue {
    match values.len() {
        0 => KeyframeValue::Scalar(0.0),
        1 => KeyframeValue::Scalar(values[0]),
        2 => KeyframeValue::Vec2([values[0], values[1]]),
        3 => KeyframeValue::Vec3([values[0], values[1], values[2]]),
        4 => KeyframeValue::Color([values[0], values[1], values[2], values[3]]),
        // Longer arrays (e.g. gradient stop tables, which pack an
        // arbitrary number of color/opacity records) are kept as-is.
        _ => KeyframeValue::FloatArray(values.to_vec()),
    }
}

fn parse_json_value(value: &serde_json::Value) -> KeyframeValue {
    match value {
        serde_json::Value::Number(n) => KeyframeValue::Scalar(n.as_f64().unwrap_or(0.0) as Scalar),
        serde_json::Value::Object(_) => {
            // Static (non-animated) bezier path value: `"k": {"i","o","v","c"}`.
            parse_path_object(value)
                .map(KeyframeValue::Path)
                .unwrap_or(KeyframeValue::Scalar(0.0))
        }
        serde_json::Value::Array(arr) => {
            // Animated bezier path value: `"s": [{"i","o","v","c"}]`.
            if let Some(first) = arr.first() {
                if first.is_object() {
                    return parse_path_object(first)
                        .map(KeyframeValue::Path)
                        .unwrap_or(KeyframeValue::Scalar(0.0));
                }
            }
            let values: Vec<Scalar> = arr
                .iter()
                .filter_map(|v| v.as_f64().map(|n| n as Scalar))
                .collect();
            parse_keyframe_value(&values)
        }
        _ => KeyframeValue::Scalar(0.0),
    }
}

/// Parse a bezier path object (`{"i":[[x,y],...],"o":[[x,y],...],"v":[[x,y],...],"c":bool}`)
/// into a [`PathData`]. `i`/`o` are tangent offsets *relative* to the
/// corresponding vertex in `v`, matching the Lottie/Bodymovin convention.
fn parse_path_object(value: &serde_json::Value) -> Option<PathData> {
    let obj = value.as_object()?;

    let get_points = |key: &str| -> Vec<[Scalar; 2]> {
        obj.get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| {
                        let pa = p.as_array()?;
                        let x = pa.first()?.as_f64()? as Scalar;
                        let y = pa.get(1)?.as_f64()? as Scalar;
                        Some([x, y])
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    Some(PathData {
        vertices: get_points("v"),
        in_tangents: get_points("i"),
        out_tangents: get_points("o"),
        closed: obj.get("c").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_easing() {
        let easing = Easing::Linear;
        assert_eq!(easing.evaluate(0.0), 0.0);
        assert_eq!(easing.evaluate(0.5), 0.5);
        assert_eq!(easing.evaluate(1.0), 1.0);
    }

    #[test]
    fn test_hold_easing() {
        let easing = Easing::Hold;
        assert_eq!(easing.evaluate(0.0), 0.0);
        assert_eq!(easing.evaluate(0.5), 0.0);
        assert_eq!(easing.evaluate(1.0), 0.0);
    }

    #[test]
    fn test_keyframe_interpolation() {
        let mut prop = AnimatedProperty::new();
        prop.add_keyframe(Keyframe::new(0.0, KeyframeValue::Scalar(0.0)));
        prop.add_keyframe(Keyframe::new(10.0, KeyframeValue::Scalar(100.0)));

        let v = prop.value_at(5.0);
        assert_eq!(v.as_scalar(), Some(50.0));
    }

    #[test]
    fn test_vec2_interpolation() {
        let a = KeyframeValue::Vec2([0.0, 0.0]);
        let b = KeyframeValue::Vec2([100.0, 200.0]);

        let result = a.lerp(&b, 0.5);
        assert_eq!(result.as_vec2(), Some([50.0, 100.0]));
    }

    #[test]
    fn test_as_vec2_accepts_vec3() {
        // Bodymovin frequently exports position/anchor as 3-component arrays.
        let v = KeyframeValue::Vec3([10.0, 20.0, 0.0]);
        assert_eq!(v.as_vec2(), Some([10.0, 20.0]));
    }

    #[test]
    fn test_static_bezier_path_value_parses_geometry() {
        use crate::model::AnimatedValue;

        let json = r#"{"a":0,"k":{"i":[[0,0],[0,0]],"o":[[0,0],[0,0]],"v":[[0,0],[10,10]],"c":false}}"#;
        let av: AnimatedValue = serde_json::from_str(json).unwrap();
        let prop = AnimatedProperty::from_lottie(&av);
        match prop.value_at(0.0) {
            KeyframeValue::Path(p) => {
                assert_eq!(p.vertices, vec![[0.0, 0.0], [10.0, 10.0]]);
                assert!(!p.closed);
            }
            other => panic!("expected Path, got {other:?}"),
        }
    }

    #[test]
    fn test_animated_bezier_path_keyframes_interpolate() {
        use crate::model::AnimatedValue;

        let json = r#"{"a":1,"k":[
            {"t":0,"s":[{"i":[[0,0]],"o":[[0,0]],"v":[[0,0]],"c":false}]},
            {"t":10,"s":[{"i":[[0,0]],"o":[[0,0]],"v":[[100,100]],"c":false}]}
        ]}"#;
        let av: AnimatedValue = serde_json::from_str(json).unwrap();
        let prop = AnimatedProperty::from_lottie(&av);

        match prop.value_at(5.0) {
            KeyframeValue::Path(p) => {
                assert_eq!(p.vertices[0], [50.0, 50.0]);
            }
            other => panic!("expected Path, got {other:?}"),
        }
    }

    #[test]
    fn test_spatial_bezier_ti_to_bulges_off_the_straight_line() {
        use crate::model::AnimatedValue;

        // Position keyframes with "to"/"ti" spatial tangents that bow the
        // motion path upward (negative y) away from the straight line
        // between (0,0) and (100,0). A plain linear lerp at the midpoint
        // would land on y=0; the spatial bezier should not.
        let json = r#"{"a":1,"k":[
            {"t":0,"s":[0,0],"to":[20,-40],"ti":[0,0]},
            {"t":10,"s":[100,0],"to":[0,0],"ti":[-20,-40]}
        ]}"#;
        let av: AnimatedValue = serde_json::from_str(json).unwrap();
        let prop = AnimatedProperty::from_lottie(&av);

        let mid = prop.value_at(5.0).as_vec2().unwrap();
        assert!(
            mid[1] < -1.0,
            "expected the midpoint to bow away from the straight line, got {mid:?}"
        );

        // Endpoints are still exact.
        let start = prop.value_at(0.0).as_vec2().unwrap();
        let end = prop.value_at(10.0).as_vec2().unwrap();
        assert!((start[0] - 0.0).abs() < 1e-6 && (start[1] - 0.0).abs() < 1e-6);
        assert!((end[0] - 100.0).abs() < 1e-6 && (end[1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_spatial_bezier_applies_to_vec3_positions() {
        use crate::model::AnimatedValue;

        // Bodymovin's dominant export format uses 3-component position
        // arrays (z = 0 for 2D layers); ti/to must not be inert for them.
        // Same geometry as the Vec2 test: the motion path bows upward
        // (negative y) away from the straight line.
        let json = r#"{"a":1,"k":[
            {"t":0,"s":[0,0,0],"to":[20,-40,0],"ti":[0,0,0]},
            {"t":10,"s":[100,0,0],"to":[0,0,0],"ti":[-20,-40,0]}
        ]}"#;
        let av: AnimatedValue = serde_json::from_str(json).unwrap();
        let prop = AnimatedProperty::from_lottie(&av);

        let mid = prop.value_at(5.0);
        // Value type stays Vec3 (z lerps linearly).
        let v3 = match &mid {
            KeyframeValue::Vec3(v) => *v,
            other => panic!("expected Vec3, got {other:?}"),
        };
        assert!(
            v3[1] < -1.0,
            "expected the Vec3 midpoint to bow away from the straight line, got {v3:?}"
        );
        assert!((v3[2] - 0.0).abs() < 1e-6);

        let start = prop.value_at(0.0).as_vec2().unwrap();
        let end = prop.value_at(10.0).as_vec2().unwrap();
        assert!((start[0] - 0.0).abs() < 1e-6 && (start[1] - 0.0).abs() < 1e-6);
        assert!((end[0] - 100.0).abs() < 1e-6 && (end[1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_no_spatial_tangents_falls_back_to_linear_lerp() {
        use crate::model::AnimatedValue;

        let json = r#"{"a":1,"k":[
            {"t":0,"s":[0,0]},
            {"t":10,"s":[100,200]}
        ]}"#;
        let av: AnimatedValue = serde_json::from_str(json).unwrap();
        let prop = AnimatedProperty::from_lottie(&av);

        let mid = prop.value_at(5.0).as_vec2().unwrap();
        assert_eq!(mid, [50.0, 100.0]);
    }

    #[test]
    fn test_trailing_time_only_keyframe_inherits_previous_value() {
        use crate::model::AnimatedValue;

        // Final keyframe has only "t" (no "s"/"e") - it should just mark an
        // end time and hold the previous keyframe's value.
        let json = r#"{"a":1,"k":[
            {"t":0,"s":[0]},
            {"t":10,"s":[100]},
            {"t":20}
        ]}"#;
        let av: AnimatedValue = serde_json::from_str(json).unwrap();
        let prop = AnimatedProperty::from_lottie(&av);

        assert_eq!(prop.value_at(15.0).as_scalar(), Some(100.0));
        assert_eq!(prop.value_at(20.0).as_scalar(), Some(100.0));
    }

    #[test]
    fn test_path_interpolation() {
        let a = PathData {
            vertices: vec![[0.0, 0.0], [10.0, 10.0]],
            in_tangents: vec![],
            out_tangents: vec![],
            closed: false,
        };
        let b = PathData {
            vertices: vec![[100.0, 100.0], [110.0, 110.0]],
            in_tangents: vec![],
            out_tangents: vec![],
            closed: false,
        };

        let result = a.lerp(&b, 0.5);
        assert_eq!(result.vertices[0], [50.0, 50.0]);
    }
}
