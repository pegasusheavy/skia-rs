//! Keyframe animation and interpolation.
//!
//! This module provides keyframe-based animation with support for:
//! - Linear interpolation
//! - Bezier easing curves
//! - Hold keyframes
//! - Multi-dimensional values

use crate::model::{AnimatedValue, KeyframeModel, TangentModel, TangentValue};
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
    let mt3 = mt2 * mt;

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
    let mt3 = mt2 * mt;

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
}

impl Keyframe {
    /// Create a new keyframe.
    pub fn new(time: Scalar, value: KeyframeValue) -> Self {
        Self {
            time,
            value,
            easing: Easing::Linear,
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
    pub fn as_vec2(&self) -> Option<[Scalar; 2]> {
        match self {
            KeyframeValue::Vec2(v) => Some(*v),
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

        prev.value.lerp(&next.value, eased_t)
    }

    /// Parse from Lottie animated value.
    pub fn from_lottie(value: &AnimatedValue) -> Self {
        match value {
            AnimatedValue::Animated { keyframes, .. } => {
                let mut prop = Self::new();

                for (i, kf) in keyframes.iter().enumerate() {
                    let value = if let Some(ref start) = kf.start {
                        parse_keyframe_value(start)
                    } else if let Some(ref end) = kf.end {
                        parse_keyframe_value(end)
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

                    prop.add_keyframe(Keyframe {
                        time: kf.time,
                        value,
                        easing,
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

fn parse_keyframe_value(values: &[Scalar]) -> KeyframeValue {
    match values.len() {
        0 => KeyframeValue::Scalar(0.0),
        1 => KeyframeValue::Scalar(values[0]),
        2 => KeyframeValue::Vec2([values[0], values[1]]),
        3 => KeyframeValue::Vec3([values[0], values[1], values[2]]),
        _ => KeyframeValue::Color([
            values.get(0).copied().unwrap_or(0.0),
            values.get(1).copied().unwrap_or(0.0),
            values.get(2).copied().unwrap_or(0.0),
            values.get(3).copied().unwrap_or(1.0),
        ]),
    }
}

fn parse_json_value(value: &serde_json::Value) -> KeyframeValue {
    match value {
        serde_json::Value::Number(n) => KeyframeValue::Scalar(n.as_f64().unwrap_or(0.0) as Scalar),
        serde_json::Value::Array(arr) => {
            let values: Vec<Scalar> = arr
                .iter()
                .filter_map(|v| v.as_f64().map(|n| n as Scalar))
                .collect();
            parse_keyframe_value(&values)
        }
        _ => KeyframeValue::Scalar(0.0),
    }
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
