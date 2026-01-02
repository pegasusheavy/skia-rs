//! Matrix transformations (2D affine and 3x3).

use crate::{Point, Rect, Scalar};
use bytemuck::{Pod, Zeroable};

/// A 3x3 transformation matrix for 2D graphics.
///
/// The matrix is stored in row-major order:
/// ```text
/// | scale_x  skew_x   trans_x |
/// | skew_y   scale_y  trans_y |
/// | persp_0  persp_1  persp_2 |
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct Matrix {
    /// Scale X (m[0][0])
    pub scale_x: Scalar,
    /// Skew X (m[0][1])
    pub skew_x: Scalar,
    /// Translate X (m[0][2])
    pub trans_x: Scalar,
    /// Skew Y (m[1][0])
    pub skew_y: Scalar,
    /// Scale Y (m[1][1])
    pub scale_y: Scalar,
    /// Translate Y (m[1][2])
    pub trans_y: Scalar,
    /// Perspective 0 (m[2][0])
    pub persp_0: Scalar,
    /// Perspective 1 (m[2][1])
    pub persp_1: Scalar,
    /// Perspective 2 (m[2][2])
    pub persp_2: Scalar,
}

impl Default for Matrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrix {
    /// The identity matrix.
    pub const IDENTITY: Self = Self {
        scale_x: 1.0, skew_x: 0.0, trans_x: 0.0,
        skew_y: 0.0, scale_y: 1.0, trans_y: 0.0,
        persp_0: 0.0, persp_1: 0.0, persp_2: 1.0,
    };

    /// Create a translation matrix.
    #[inline]
    pub fn translate(dx: Scalar, dy: Scalar) -> Self {
        Self {
            trans_x: dx,
            trans_y: dy,
            ..Self::IDENTITY
        }
    }

    /// Create a scale matrix.
    #[inline]
    pub fn scale(sx: Scalar, sy: Scalar) -> Self {
        Self {
            scale_x: sx,
            scale_y: sy,
            ..Self::IDENTITY
        }
    }

    /// Create a rotation matrix (angle in radians).
    #[inline]
    pub fn rotate(radians: Scalar) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            scale_x: cos, skew_x: -sin, trans_x: 0.0,
            skew_y: sin, scale_y: cos, trans_y: 0.0,
            persp_0: 0.0, persp_1: 0.0, persp_2: 1.0,
        }
    }

    /// Create a rotation matrix around a point.
    #[inline]
    pub fn rotate_around(radians: Scalar, pivot: Point) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            scale_x: cos,
            skew_x: -sin,
            trans_x: pivot.x - pivot.x * cos + pivot.y * sin,
            skew_y: sin,
            scale_y: cos,
            trans_y: pivot.y - pivot.x * sin - pivot.y * cos,
            persp_0: 0.0,
            persp_1: 0.0,
            persp_2: 1.0,
        }
    }

    /// Create a skew matrix.
    #[inline]
    pub fn skew(kx: Scalar, ky: Scalar) -> Self {
        Self {
            scale_x: 1.0, skew_x: kx, trans_x: 0.0,
            skew_y: ky, scale_y: 1.0, trans_y: 0.0,
            persp_0: 0.0, persp_1: 0.0, persp_2: 1.0,
        }
    }

    /// Check if this is the identity matrix.
    #[inline]
    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }

    /// Check if this matrix only has translation.
    #[inline]
    pub fn is_translate(&self) -> bool {
        self.scale_x == 1.0 && self.skew_x == 0.0 &&
        self.skew_y == 0.0 && self.scale_y == 1.0 &&
        self.persp_0 == 0.0 && self.persp_1 == 0.0 && self.persp_2 == 1.0
    }

    /// Check if this matrix has perspective.
    #[inline]
    pub fn has_perspective(&self) -> bool {
        self.persp_0 != 0.0 || self.persp_1 != 0.0 || self.persp_2 != 1.0
    }

    /// Concatenate with another matrix (self * other).
    #[inline]
    pub fn concat(&self, other: &Matrix) -> Matrix {
        Matrix {
            scale_x: self.scale_x * other.scale_x + self.skew_x * other.skew_y + self.trans_x * other.persp_0,
            skew_x: self.scale_x * other.skew_x + self.skew_x * other.scale_y + self.trans_x * other.persp_1,
            trans_x: self.scale_x * other.trans_x + self.skew_x * other.trans_y + self.trans_x * other.persp_2,
            skew_y: self.skew_y * other.scale_x + self.scale_y * other.skew_y + self.trans_y * other.persp_0,
            scale_y: self.skew_y * other.skew_x + self.scale_y * other.scale_y + self.trans_y * other.persp_1,
            trans_y: self.skew_y * other.trans_x + self.scale_y * other.trans_y + self.trans_y * other.persp_2,
            persp_0: self.persp_0 * other.scale_x + self.persp_1 * other.skew_y + self.persp_2 * other.persp_0,
            persp_1: self.persp_0 * other.skew_x + self.persp_1 * other.scale_y + self.persp_2 * other.persp_1,
            persp_2: self.persp_0 * other.trans_x + self.persp_1 * other.trans_y + self.persp_2 * other.persp_2,
        }
    }

    /// Transform a point.
    #[inline]
    pub fn map_point(&self, p: Point) -> Point {
        if self.has_perspective() {
            let w = self.persp_0 * p.x + self.persp_1 * p.y + self.persp_2;
            let w_inv = if w != 0.0 { 1.0 / w } else { 0.0 };
            Point::new(
                (self.scale_x * p.x + self.skew_x * p.y + self.trans_x) * w_inv,
                (self.skew_y * p.x + self.scale_y * p.y + self.trans_y) * w_inv,
            )
        } else {
            Point::new(
                self.scale_x * p.x + self.skew_x * p.y + self.trans_x,
                self.skew_y * p.x + self.scale_y * p.y + self.trans_y,
            )
        }
    }

    /// Transform a rectangle (returns bounding box of transformed corners).
    #[inline]
    pub fn map_rect(&self, r: &Rect) -> Rect {
        let p0 = self.map_point(Point::new(r.left, r.top));
        let p1 = self.map_point(Point::new(r.right, r.top));
        let p2 = self.map_point(Point::new(r.right, r.bottom));
        let p3 = self.map_point(Point::new(r.left, r.bottom));

        Rect::new(
            p0.x.min(p1.x).min(p2.x).min(p3.x),
            p0.y.min(p1.y).min(p2.y).min(p3.y),
            p0.x.max(p1.x).max(p2.x).max(p3.x),
            p0.y.max(p1.y).max(p2.y).max(p3.y),
        )
    }

    /// Compute the inverse matrix.
    pub fn invert(&self) -> Option<Matrix> {
        let det = self.scale_x * (self.scale_y * self.persp_2 - self.trans_y * self.persp_1)
                - self.skew_x * (self.skew_y * self.persp_2 - self.trans_y * self.persp_0)
                + self.trans_x * (self.skew_y * self.persp_1 - self.scale_y * self.persp_0);

        if det == 0.0 {
            return None;
        }

        let inv_det = 1.0 / det;

        Some(Matrix {
            scale_x: (self.scale_y * self.persp_2 - self.trans_y * self.persp_1) * inv_det,
            skew_x: (self.trans_x * self.persp_1 - self.skew_x * self.persp_2) * inv_det,
            trans_x: (self.skew_x * self.trans_y - self.trans_x * self.scale_y) * inv_det,
            skew_y: (self.trans_y * self.persp_0 - self.skew_y * self.persp_2) * inv_det,
            scale_y: (self.scale_x * self.persp_2 - self.trans_x * self.persp_0) * inv_det,
            trans_y: (self.trans_x * self.skew_y - self.scale_x * self.trans_y) * inv_det,
            persp_0: (self.skew_y * self.persp_1 - self.scale_y * self.persp_0) * inv_det,
            persp_1: (self.skew_x * self.persp_0 - self.scale_x * self.persp_1) * inv_det,
            persp_2: (self.scale_x * self.scale_y - self.skew_x * self.skew_y) * inv_det,
        })
    }
}

impl std::ops::Mul for Matrix {
    type Output = Matrix;
    #[inline]
    fn mul(self, rhs: Matrix) -> Matrix {
        self.concat(&rhs)
    }
}
