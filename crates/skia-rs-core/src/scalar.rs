//! Scalar type definition and utilities.

/// The scalar type used throughout skia-rs (matches Skia's SkScalar).
pub type Scalar = f32;

/// Positive infinity for Scalar.
pub const SCALAR_INFINITY: Scalar = f32::INFINITY;

/// Negative infinity for Scalar.
pub const SCALAR_NEG_INFINITY: Scalar = f32::NEG_INFINITY;

/// Not a number for Scalar.
pub const SCALAR_NAN: Scalar = f32::NAN;

/// Maximum finite value for Scalar.
pub const SCALAR_MAX: Scalar = f32::MAX;

/// Minimum positive value for Scalar.
pub const SCALAR_MIN: Scalar = f32::MIN_POSITIVE;

/// Nearly zero threshold for comparisons.
pub const SCALAR_NEARLY_ZERO: Scalar = 1.0 / (1 << 12) as Scalar;

/// Check if a scalar is nearly zero.
#[inline]
pub fn scalar_nearly_zero(x: Scalar) -> bool {
    x.abs() <= SCALAR_NEARLY_ZERO
}

/// Check if two scalars are nearly equal.
#[inline]
pub fn scalar_nearly_equal(a: Scalar, b: Scalar) -> bool {
    scalar_nearly_zero(a - b)
}

/// Check if a scalar is finite (not infinity or NaN).
#[inline]
pub fn scalar_is_finite(x: Scalar) -> bool {
    x.is_finite()
}

/// Linearly interpolate between two scalars.
#[inline]
pub fn scalar_interp(a: Scalar, b: Scalar, t: Scalar) -> Scalar {
    a + (b - a) * t
}
