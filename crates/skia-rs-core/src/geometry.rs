//! Geometric primitives: points, sizes, rectangles, and matrices.
//!
//! This module provides Skia-compatible geometry types.

use crate::Scalar;
use bytemuck::{Pod, Zeroable};

// =============================================================================
// Point Types
// =============================================================================

/// A point with integer coordinates.
///
/// Equivalent to Skia's `SkIPoint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct IPoint {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
}

impl IPoint {
    /// Creates a new point.
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Returns the origin (0, 0).
    #[inline]
    pub const fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    /// Returns true if both coordinates are zero.
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.x == 0 && self.y == 0
    }

    /// Negates both coordinates.
    #[inline]
    pub const fn negate(&self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }

    /// Offsets the point by (dx, dy).
    #[inline]
    pub const fn offset(&self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

/// A point with floating-point coordinates.
///
/// Equivalent to Skia's `SkPoint` / `SkVector`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Point {
    /// X coordinate.
    pub x: Scalar,
    /// Y coordinate.
    pub y: Scalar,
}

impl Point {
    /// Creates a new point.
    #[inline]
    pub const fn new(x: Scalar, y: Scalar) -> Self {
        Self { x, y }
    }

    /// Returns the origin (0, 0).
    #[inline]
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Returns true if both coordinates are zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.x == 0.0 && self.y == 0.0
    }

    /// Returns true if either coordinate is NaN or infinite.
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Negates both coordinates.
    #[inline]
    pub fn negate(&self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }

    /// Offsets the point by (dx, dy).
    #[inline]
    pub fn offset(&self, dx: Scalar, dy: Scalar) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    /// Returns the length of the vector from origin to this point.
    #[inline]
    pub fn length(&self) -> Scalar {
        self.x.hypot(self.y)
    }

    /// Returns the squared length (avoids sqrt).
    #[inline]
    pub fn length_squared(&self) -> Scalar {
        self.x * self.x + self.y * self.y
    }

    /// Returns a normalized (unit length) vector, or zero if length is zero.
    #[inline]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            Self::zero()
        }
    }

    /// Dot product with another point/vector.
    #[inline]
    pub fn dot(&self, other: &Self) -> Scalar {
        self.x * other.x + self.y * other.y
    }

    /// Cross product (returns the z-component of the 3D cross product).
    #[inline]
    pub fn cross(&self, other: &Self) -> Scalar {
        self.x * other.y - self.y * other.x
    }

    /// Returns the distance to another point.
    #[inline]
    pub fn distance(&self, other: &Self) -> Scalar {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx.hypot(dy)
    }

    /// Scales the point by a factor.
    #[inline]
    pub fn scale(&self, factor: Scalar) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
        }
    }

    /// Linear interpolation between this point and another.
    #[inline]
    pub fn lerp(&self, other: Self, t: Scalar) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

impl From<IPoint> for Point {
    #[inline]
    fn from(p: IPoint) -> Self {
        Self {
            x: p.x as Scalar,
            y: p.y as Scalar,
        }
    }
}

// Operator implementations for Point
impl std::ops::Add for Point {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::AddAssign for Point {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::Sub for Point {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::SubAssign for Point {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl std::ops::Mul<Scalar> for Point {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Scalar) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl std::ops::MulAssign<Scalar> for Point {
    #[inline]
    fn mul_assign(&mut self, rhs: Scalar) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl std::ops::Div<Scalar> for Point {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Scalar) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl std::ops::Neg for Point {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

/// A 3D point with floating-point coordinates.
///
/// Equivalent to Skia's `SkPoint3`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Point3 {
    /// X coordinate.
    pub x: Scalar,
    /// Y coordinate.
    pub y: Scalar,
    /// Z coordinate.
    pub z: Scalar,
}

impl Point3 {
    /// Creates a new 3D point.
    #[inline]
    pub const fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self { x, y, z }
    }

    /// Returns the origin (0, 0, 0).
    #[inline]
    pub const fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Returns the length of the vector.
    #[inline]
    pub fn length(&self) -> Scalar {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Dot product with another 3D point/vector.
    #[inline]
    pub fn dot(&self, other: &Self) -> Scalar {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product.
    #[inline]
    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
}

// =============================================================================
// Size Types
// =============================================================================

/// A size with integer dimensions.
///
/// Equivalent to Skia's `SkISize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct ISize {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
}

impl ISize {
    /// Creates a new size.
    #[inline]
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    /// Returns an empty size (0, 0).
    #[inline]
    pub const fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }

    /// Returns true if width or height is <= 0.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    /// Returns the area (width * height).
    #[inline]
    pub const fn area(&self) -> i64 {
        self.width as i64 * self.height as i64
    }
}

/// A size with floating-point dimensions.
///
/// Equivalent to Skia's `SkSize`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Size {
    /// Width.
    pub width: Scalar,
    /// Height.
    pub height: Scalar,
}

impl Size {
    /// Creates a new size.
    #[inline]
    pub const fn new(width: Scalar, height: Scalar) -> Self {
        Self { width, height }
    }

    /// Returns an empty size (0, 0).
    #[inline]
    pub const fn empty() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }

    /// Returns true if width or height is <= 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Returns the area (width * height).
    #[inline]
    pub fn area(&self) -> Scalar {
        self.width * self.height
    }

    /// Converts to integer size by truncating.
    #[inline]
    pub fn to_isize(&self) -> ISize {
        ISize {
            width: self.width as i32,
            height: self.height as i32,
        }
    }

    /// Converts to integer size by rounding.
    #[inline]
    pub fn to_isize_round(&self) -> ISize {
        ISize {
            width: self.width.round() as i32,
            height: self.height.round() as i32,
        }
    }
}

impl From<ISize> for Size {
    #[inline]
    fn from(s: ISize) -> Self {
        Self {
            width: s.width as Scalar,
            height: s.height as Scalar,
        }
    }
}

// =============================================================================
// Rectangle Types
// =============================================================================

/// A rectangle with integer coordinates.
///
/// Equivalent to Skia's `SkIRect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct IRect {
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge.
    pub right: i32,
    /// Bottom edge.
    pub bottom: i32,
}

impl IRect {
    /// Creates a new rectangle from edges.
    #[inline]
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Creates a rectangle from origin and size.
    #[inline]
    pub const fn from_xywh(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }

    /// Creates a rectangle from size (origin at 0,0).
    #[inline]
    pub const fn from_size(size: ISize) -> Self {
        Self {
            left: 0,
            top: 0,
            right: size.width,
            bottom: size.height,
        }
    }

    /// Returns an empty rectangle.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }

    /// Returns the width.
    #[inline]
    pub const fn width(&self) -> i32 {
        self.right - self.left
    }

    /// Returns the height.
    #[inline]
    pub const fn height(&self) -> i32 {
        self.bottom - self.top
    }

    /// Returns the size.
    #[inline]
    pub const fn size(&self) -> ISize {
        ISize {
            width: self.width(),
            height: self.height(),
        }
    }

    /// Returns true if the rectangle has zero or negative area.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    /// Returns true if the point is inside the rectangle.
    #[inline]
    pub const fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    /// Returns the intersection of two rectangles, or None if they don't intersect.
    #[inline]
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        let right = self.right.min(other.right);
        let bottom = self.bottom.min(other.bottom);

        if left < right && top < bottom {
            Some(Self {
                left,
                top,
                right,
                bottom,
            })
        } else {
            None
        }
    }

    /// Returns the union (bounding box) of two rectangles.
    #[inline]
    pub fn union(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    /// Offsets the rectangle by (dx, dy).
    #[inline]
    pub const fn offset(&self, dx: i32, dy: i32) -> Self {
        Self {
            left: self.left + dx,
            top: self.top + dy,
            right: self.right + dx,
            bottom: self.bottom + dy,
        }
    }

    /// Convert to a floating-point Rect.
    #[inline]
    pub fn to_rect(&self) -> Rect {
        Rect::new(
            self.left as Scalar,
            self.top as Scalar,
            self.right as Scalar,
            self.bottom as Scalar,
        )
    }

    /// Insets the rectangle by (dx, dy) on each side.
    #[inline]
    pub const fn inset(&self, dx: i32, dy: i32) -> Self {
        Self {
            left: self.left + dx,
            top: self.top + dy,
            right: self.right - dx,
            bottom: self.bottom - dy,
        }
    }
}

/// A rectangle with floating-point coordinates.
///
/// Equivalent to Skia's `SkRect`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Rect {
    /// Left edge.
    pub left: Scalar,
    /// Top edge.
    pub top: Scalar,
    /// Right edge.
    pub right: Scalar,
    /// Bottom edge.
    pub bottom: Scalar,
}

impl Rect {
    /// Empty rectangle constant.
    pub const EMPTY: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    /// Creates a new rectangle from edges.
    #[inline]
    pub const fn new(left: Scalar, top: Scalar, right: Scalar, bottom: Scalar) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Creates a rectangle from origin and size.
    #[inline]
    pub const fn from_xywh(x: Scalar, y: Scalar, width: Scalar, height: Scalar) -> Self {
        Self {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }

    /// Creates a rectangle from size (origin at 0,0).
    #[inline]
    pub const fn from_size(size: Size) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: size.width,
            bottom: size.height,
        }
    }

    /// Creates a rectangle from center and half-width/half-height.
    #[inline]
    pub fn from_center(center: Point, half_width: Scalar, half_height: Scalar) -> Self {
        Self {
            left: center.x - half_width,
            top: center.y - half_height,
            right: center.x + half_width,
            bottom: center.y + half_height,
        }
    }

    /// Returns an empty rectangle.
    #[inline]
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Returns the width.
    #[inline]
    pub fn width(&self) -> Scalar {
        self.right - self.left
    }

    /// Returns the height.
    #[inline]
    pub fn height(&self) -> Scalar {
        self.bottom - self.top
    }

    /// Returns the size.
    #[inline]
    pub fn size(&self) -> Size {
        Size {
            width: self.width(),
            height: self.height(),
        }
    }

    /// Returns the center point.
    #[inline]
    pub fn center(&self) -> Point {
        Point {
            x: (self.left + self.right) * 0.5,
            y: (self.top + self.bottom) * 0.5,
        }
    }

    /// Returns true if the rectangle has zero or negative area.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    /// Returns true if all coordinates are finite.
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.left.is_finite()
            && self.top.is_finite()
            && self.right.is_finite()
            && self.bottom.is_finite()
    }

    /// Returns true if the point (x, y) is inside the rectangle.
    #[inline]
    pub fn contains_xy(&self, x: Scalar, y: Scalar) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    /// Returns true if the point is inside the rectangle.
    #[inline]
    pub fn contains(&self, point: Point) -> bool {
        self.contains_xy(point.x, point.y)
    }

    /// Returns true if this rectangle contains the other rectangle.
    #[inline]
    pub fn contains_rect(&self, other: &Self) -> bool {
        self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }

    /// Returns the intersection of two rectangles, or None if they don't intersect.
    #[inline]
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        let right = self.right.min(other.right);
        let bottom = self.bottom.min(other.bottom);

        if left < right && top < bottom {
            Some(Self {
                left,
                top,
                right,
                bottom,
            })
        } else {
            None
        }
    }

    /// Returns true if this rectangle intersects with another.
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }

    /// Returns the union (bounding box) of two rectangles.
    #[inline]
    pub fn union(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    /// Alias for union - joins two rectangles into their bounding box.
    #[inline]
    pub fn join(&self, other: &Self) -> Self {
        self.union(other)
    }

    /// Offsets the rectangle by (dx, dy).
    #[inline]
    pub fn offset(&self, dx: Scalar, dy: Scalar) -> Self {
        Self {
            left: self.left + dx,
            top: self.top + dy,
            right: self.right + dx,
            bottom: self.bottom + dy,
        }
    }

    /// Insets the rectangle by (dx, dy) on each side.
    #[inline]
    pub fn inset(&self, dx: Scalar, dy: Scalar) -> Self {
        Self {
            left: self.left + dx,
            top: self.top + dy,
            right: self.right - dx,
            bottom: self.bottom - dy,
        }
    }

    /// Rounds to the smallest enclosing integer rectangle.
    #[inline]
    pub fn round_out(&self) -> IRect {
        IRect {
            left: self.left.floor() as i32,
            top: self.top.floor() as i32,
            right: self.right.ceil() as i32,
            bottom: self.bottom.ceil() as i32,
        }
    }

    /// Rounds to the largest enclosed integer rectangle.
    #[inline]
    pub fn round_in(&self) -> IRect {
        IRect {
            left: self.left.ceil() as i32,
            top: self.top.ceil() as i32,
            right: self.right.floor() as i32,
            bottom: self.bottom.floor() as i32,
        }
    }

    /// Rounds to nearest integer rectangle.
    #[inline]
    pub fn round(&self) -> IRect {
        IRect {
            left: self.left.round() as i32,
            top: self.top.round() as i32,
            right: self.right.round() as i32,
            bottom: self.bottom.round() as i32,
        }
    }
}

impl From<IRect> for Rect {
    #[inline]
    fn from(r: IRect) -> Self {
        Self {
            left: r.left as Scalar,
            top: r.top as Scalar,
            right: r.right as Scalar,
            bottom: r.bottom as Scalar,
        }
    }
}

// =============================================================================
// Rounded Rectangle
// =============================================================================

/// A rectangle with rounded corners.
///
/// Equivalent to Skia's `SkRRect`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RRect {
    /// The bounding rectangle.
    pub rect: Rect,
    /// Corner radii: [top-left, top-right, bottom-right, bottom-left].
    /// Each corner has (x-radius, y-radius).
    pub radii: [Point; 4],
}

/// Corner indices for `RRect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Corner {
    /// Top-left corner.
    TopLeft = 0,
    /// Top-right corner.
    TopRight = 1,
    /// Bottom-right corner.
    BottomRight = 2,
    /// Bottom-left corner.
    BottomLeft = 3,
}

impl RRect {
    /// Creates a rounded rectangle with the same radius for all corners.
    #[inline]
    pub fn from_rect_xy(rect: Rect, x_rad: Scalar, y_rad: Scalar) -> Self {
        let radius = Point::new(x_rad, y_rad);
        Self {
            rect,
            radii: [radius, radius, radius, radius],
        }
    }

    /// Creates a rounded rectangle with a uniform radius.
    #[inline]
    pub fn from_rect_radius(rect: Rect, radius: Scalar) -> Self {
        Self::from_rect_xy(rect, radius, radius)
    }

    /// Creates a simple (non-rounded) rectangle.
    #[inline]
    pub fn from_rect(rect: Rect) -> Self {
        Self {
            rect,
            radii: [Point::zero(); 4],
        }
    }

    /// Creates an oval inscribed in the rectangle.
    #[inline]
    pub fn from_oval(rect: Rect) -> Self {
        let x_rad = rect.width() * 0.5;
        let y_rad = rect.height() * 0.5;
        Self::from_rect_xy(rect, x_rad, y_rad)
    }

    /// Returns the bounding rectangle.
    #[inline]
    pub fn rect(&self) -> &Rect {
        &self.rect
    }

    /// Returns the radius for a specific corner.
    #[inline]
    pub fn radius(&self, corner: Corner) -> Point {
        self.radii[corner as usize]
    }

    /// Returns true if all corners have zero radius.
    #[inline]
    pub fn is_rect(&self) -> bool {
        self.radii.iter().all(|r| r.x == 0.0 && r.y == 0.0)
    }

    /// Returns true if this is an oval (all corners have the same radius equal to half the dimensions).
    #[inline]
    pub fn is_oval(&self) -> bool {
        let x_rad = self.rect.width() * 0.5;
        let y_rad = self.rect.height() * 0.5;
        self.radii
            .iter()
            .all(|r| (r.x - x_rad).abs() < 1e-6 && (r.y - y_rad).abs() < 1e-6)
    }

    /// Returns true if all corners have the same radius.
    #[inline]
    pub fn is_simple(&self) -> bool {
        let first = self.radii[0];
        self.radii.iter().all(|r| *r == first)
    }
}

// =============================================================================
// Matrix (3x3)
// =============================================================================

/// A 3x3 transformation matrix.
///
/// Equivalent to Skia's `SkMatrix`.
///
/// The matrix is stored in row-major order:
/// ```text
/// | scale_x  skew_x   trans_x |
/// | skew_y   scale_y  trans_y |
/// | persp_0  persp_1  persp_2 |
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    /// Matrix values in row-major order.
    pub values: [Scalar; 9],
}

impl Default for Matrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrix {
    /// The identity matrix constant.
    pub const IDENTITY: Self = Self {
        values: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    };

    /// Index constants for matrix elements.
    pub const SCALE_X: usize = 0;
    /// Skew X index.
    pub const SKEW_X: usize = 1;
    /// Translate X index.
    pub const TRANS_X: usize = 2;
    /// Skew Y index.
    pub const SKEW_Y: usize = 3;
    /// Scale Y index.
    pub const SCALE_Y: usize = 4;
    /// Translate Y index.
    pub const TRANS_Y: usize = 5;
    /// Perspective 0 index.
    pub const PERSP_0: usize = 6;
    /// Perspective 1 index.
    pub const PERSP_1: usize = 7;
    /// Perspective 2 index.
    pub const PERSP_2: usize = 8;

    /// Creates the identity matrix.
    #[inline]
    pub const fn identity() -> Self {
        Self::IDENTITY
    }

    /// Creates a translation matrix.
    #[inline]
    pub const fn translate(dx: Scalar, dy: Scalar) -> Self {
        Self {
            values: [1.0, 0.0, dx, 0.0, 1.0, dy, 0.0, 0.0, 1.0],
        }
    }

    /// Creates a scale matrix.
    #[inline]
    pub const fn scale(sx: Scalar, sy: Scalar) -> Self {
        Self {
            values: [sx, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Creates a rotation matrix (angle in radians).
    #[inline]
    pub fn rotate(radians: Scalar) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            values: [cos, -sin, 0.0, sin, cos, 0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Creates a rotation matrix around a pivot point.
    #[inline]
    pub fn rotate_around(radians: Scalar, pivot: Point) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            values: [
                cos,
                -sin,
                pivot.x - pivot.x * cos + pivot.y * sin,
                sin,
                cos,
                pivot.y - pivot.x * sin - pivot.y * cos,
                0.0,
                0.0,
                1.0,
            ],
        }
    }

    /// Creates a skew matrix.
    #[inline]
    pub fn skew(kx: Scalar, ky: Scalar) -> Self {
        Self {
            values: [1.0, kx.tan(), 0.0, ky.tan(), 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Returns true if this is the identity matrix.
    #[inline]
    pub fn is_identity(&self) -> bool {
        *self == Self::identity()
    }

    /// Returns true if the matrix only contains translation.
    #[inline]
    pub fn is_translate(&self) -> bool {
        self.values[Self::SCALE_X] == 1.0
            && self.values[Self::SKEW_X] == 0.0
            && self.values[Self::SKEW_Y] == 0.0
            && self.values[Self::SCALE_Y] == 1.0
            && self.values[Self::PERSP_0] == 0.0
            && self.values[Self::PERSP_1] == 0.0
            && self.values[Self::PERSP_2] == 1.0
    }

    /// Returns true if the matrix only contains scale and translation.
    #[inline]
    pub fn is_scale_translate(&self) -> bool {
        self.values[Self::SKEW_X] == 0.0
            && self.values[Self::SKEW_Y] == 0.0
            && self.values[Self::PERSP_0] == 0.0
            && self.values[Self::PERSP_1] == 0.0
            && self.values[Self::PERSP_2] == 1.0
    }

    /// Returns the translation component.
    #[inline]
    pub fn translation(&self) -> Point {
        Point {
            x: self.values[Self::TRANS_X],
            y: self.values[Self::TRANS_Y],
        }
    }

    /// Returns the X scale factor.
    #[inline]
    pub fn scale_x(&self) -> Scalar {
        self.values[Self::SCALE_X]
    }

    /// Returns the Y scale factor.
    #[inline]
    pub fn scale_y(&self) -> Scalar {
        self.values[Self::SCALE_Y]
    }

    /// Returns the X skew factor.
    #[inline]
    pub fn skew_x(&self) -> Scalar {
        self.values[Self::SKEW_X]
    }

    /// Returns the Y skew factor.
    #[inline]
    pub fn skew_y(&self) -> Scalar {
        self.values[Self::SKEW_Y]
    }

    /// Concatenates this matrix with another (self * other).
    #[inline]
    pub fn concat(&self, other: &Self) -> Self {
        let a = &self.values;
        let b = &other.values;
        Self {
            values: [
                a[0] * b[0] + a[1] * b[3] + a[2] * b[6],
                a[0] * b[1] + a[1] * b[4] + a[2] * b[7],
                a[0] * b[2] + a[1] * b[5] + a[2] * b[8],
                a[3] * b[0] + a[4] * b[3] + a[5] * b[6],
                a[3] * b[1] + a[4] * b[4] + a[5] * b[7],
                a[3] * b[2] + a[4] * b[5] + a[5] * b[8],
                a[6] * b[0] + a[7] * b[3] + a[8] * b[6],
                a[6] * b[1] + a[7] * b[4] + a[8] * b[7],
                a[6] * b[2] + a[7] * b[5] + a[8] * b[8],
            ],
        }
    }

    /// Transforms a point by this matrix.
    #[inline]
    pub fn map_point(&self, point: Point) -> Point {
        let m = &self.values;
        let x = m[0] * point.x + m[1] * point.y + m[2];
        let y = m[3] * point.x + m[4] * point.y + m[5];

        // Handle perspective
        if m[6] != 0.0 || m[7] != 0.0 || m[8] != 1.0 {
            let w = m[6] * point.x + m[7] * point.y + m[8];
            Point {
                x: x / w,
                y: y / w,
            }
        } else {
            Point { x, y }
        }
    }

    /// Transforms a rectangle by this matrix (returns bounding box of transformed corners).
    #[inline]
    pub fn map_rect(&self, rect: &Rect) -> Rect {
        let corners = [
            self.map_point(Point::new(rect.left, rect.top)),
            self.map_point(Point::new(rect.right, rect.top)),
            self.map_point(Point::new(rect.right, rect.bottom)),
            self.map_point(Point::new(rect.left, rect.bottom)),
        ];

        let mut min_x = corners[0].x;
        let mut min_y = corners[0].y;
        let mut max_x = corners[0].x;
        let mut max_y = corners[0].y;

        for p in &corners[1..] {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }

        Rect::new(min_x, min_y, max_x, max_y)
    }

    /// Computes the determinant.
    #[inline]
    pub fn determinant(&self) -> Scalar {
        let m = &self.values;
        m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
            + m[2] * (m[3] * m[7] - m[4] * m[6])
    }

    /// Computes the inverse matrix, or None if singular.
    pub fn invert(&self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < 1e-10 {
            return None;
        }

        let m = &self.values;
        let inv_det = 1.0 / det;

        Some(Self {
            values: [
                (m[4] * m[8] - m[5] * m[7]) * inv_det,
                (m[2] * m[7] - m[1] * m[8]) * inv_det,
                (m[1] * m[5] - m[2] * m[4]) * inv_det,
                (m[5] * m[6] - m[3] * m[8]) * inv_det,
                (m[0] * m[8] - m[2] * m[6]) * inv_det,
                (m[2] * m[3] - m[0] * m[5]) * inv_det,
                (m[3] * m[7] - m[4] * m[6]) * inv_det,
                (m[1] * m[6] - m[0] * m[7]) * inv_det,
                (m[0] * m[4] - m[1] * m[3]) * inv_det,
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_operations() {
        let p1 = Point::new(3.0, 4.0);
        assert!((p1.length() - 5.0).abs() < 1e-6);

        let p2 = Point::new(1.0, 2.0);
        assert!((p1.dot(&p2) - 11.0).abs() < 1e-6);
    }

    #[test]
    fn test_rect_intersection() {
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::new(5.0, 5.0, 15.0, 15.0);
        let intersection = r1.intersect(&r2).unwrap();
        assert_eq!(intersection, Rect::new(5.0, 5.0, 10.0, 10.0));
    }

    #[test]
    fn test_matrix_identity() {
        let m = Matrix::identity();
        let p = Point::new(5.0, 7.0);
        let result = m.map_point(p);
        assert_eq!(result, p);
    }

    #[test]
    fn test_matrix_translate() {
        let m = Matrix::translate(10.0, 20.0);
        let p = Point::new(5.0, 7.0);
        let result = m.map_point(p);
        assert_eq!(result, Point::new(15.0, 27.0));
    }

    #[test]
    fn test_matrix_inverse() {
        let m = Matrix::translate(10.0, 20.0);
        let inv = m.invert().unwrap();
        let result = m.concat(&inv);
        assert!((result.values[0] - 1.0).abs() < 1e-6);
        assert!((result.values[4] - 1.0).abs() < 1e-6);
    }
}
