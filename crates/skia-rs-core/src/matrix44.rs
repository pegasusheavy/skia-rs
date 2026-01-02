//! 4x4 transformation matrix for 3D transformations.
//!
//! This module provides a 4x4 matrix type for 3D transformations,
//! corresponding to Skia's `SkM44` / `SkMatrix44`.

use crate::Scalar;
use crate::geometry::{Matrix, Point, Point3};

/// A 4x4 transformation matrix for 3D transformations.
///
/// Corresponds to Skia's `SkM44`.
///
/// The matrix is stored in column-major order:
/// ```text
/// | m[0]  m[4]  m[8]   m[12] |
/// | m[1]  m[5]  m[9]   m[13] |
/// | m[2]  m[6]  m[10]  m[14] |
/// | m[3]  m[7]  m[11]  m[15] |
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix44 {
    /// Matrix values in column-major order.
    pub values: [Scalar; 16],
}

impl Default for Matrix44 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrix44 {
    /// The identity matrix constant.
    pub const IDENTITY: Self = Self {
        values: [
            1.0, 0.0, 0.0, 0.0, // column 0
            0.0, 1.0, 0.0, 0.0, // column 1
            0.0, 0.0, 1.0, 0.0, // column 2
            0.0, 0.0, 0.0, 1.0, // column 3
        ],
    };

    /// Creates a new matrix from column-major values.
    #[inline]
    pub const fn from_cols(values: [Scalar; 16]) -> Self {
        Self { values }
    }

    /// Creates a new matrix from row-major values.
    pub fn from_rows(r0: [Scalar; 4], r1: [Scalar; 4], r2: [Scalar; 4], r3: [Scalar; 4]) -> Self {
        Self {
            values: [
                r0[0], r1[0], r2[0], r3[0], // column 0
                r0[1], r1[1], r2[1], r3[1], // column 1
                r0[2], r1[2], r2[2], r3[2], // column 2
                r0[3], r1[3], r2[3], r3[3], // column 3
            ],
        }
    }

    /// Creates the identity matrix.
    #[inline]
    pub const fn identity() -> Self {
        Self::IDENTITY
    }

    /// Creates a translation matrix.
    pub const fn translate(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self {
            values: [
                1.0, 0.0, 0.0, 0.0, // column 0
                0.0, 1.0, 0.0, 0.0, // column 1
                0.0, 0.0, 1.0, 0.0, // column 2
                x, y, z, 1.0, // column 3
            ],
        }
    }

    /// Creates a scale matrix.
    pub const fn scale(sx: Scalar, sy: Scalar, sz: Scalar) -> Self {
        Self {
            values: [
                sx, 0.0, 0.0, 0.0, // column 0
                0.0, sy, 0.0, 0.0, // column 1
                0.0, 0.0, sz, 0.0, // column 2
                0.0, 0.0, 0.0, 1.0, // column 3
            ],
        }
    }

    /// Creates a rotation matrix around the X axis.
    pub fn rotate_x(radians: Scalar) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            values: [
                1.0, 0.0, 0.0, 0.0, // column 0
                0.0, cos, sin, 0.0, // column 1
                0.0, -sin, cos, 0.0, // column 2
                0.0, 0.0, 0.0, 1.0, // column 3
            ],
        }
    }

    /// Creates a rotation matrix around the Y axis.
    pub fn rotate_y(radians: Scalar) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            values: [
                cos, 0.0, -sin, 0.0, // column 0
                0.0, 1.0, 0.0, 0.0, // column 1
                sin, 0.0, cos, 0.0, // column 2
                0.0, 0.0, 0.0, 1.0, // column 3
            ],
        }
    }

    /// Creates a rotation matrix around the Z axis.
    pub fn rotate_z(radians: Scalar) -> Self {
        let (sin, cos) = radians.sin_cos();
        Self {
            values: [
                cos, sin, 0.0, 0.0, // column 0
                -sin, cos, 0.0, 0.0, // column 1
                0.0, 0.0, 1.0, 0.0, // column 2
                0.0, 0.0, 0.0, 1.0, // column 3
            ],
        }
    }

    /// Creates a rotation matrix around an arbitrary axis.
    pub fn rotate(axis: Point3, radians: Scalar) -> Self {
        let len = axis.length();
        if len == 0.0 {
            return Self::IDENTITY;
        }

        let x = axis.x / len;
        let y = axis.y / len;
        let z = axis.z / len;

        let (sin, cos) = radians.sin_cos();
        let t = 1.0 - cos;

        Self {
            values: [
                t * x * x + cos,
                t * x * y + sin * z,
                t * x * z - sin * y,
                0.0,
                t * x * y - sin * z,
                t * y * y + cos,
                t * y * z + sin * x,
                0.0,
                t * x * z + sin * y,
                t * y * z - sin * x,
                t * z * z + cos,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
            ],
        }
    }

    /// Creates a look-at matrix (camera transformation).
    pub fn look_at(eye: Point3, center: Point3, up: Point3) -> Self {
        let f = Point3::new(center.x - eye.x, center.y - eye.y, center.z - eye.z);
        let f_len = f.length();
        if f_len == 0.0 {
            return Self::IDENTITY;
        }
        let f = Point3::new(f.x / f_len, f.y / f_len, f.z / f_len);

        let s = f.cross(&up);
        let s_len = s.length();
        if s_len == 0.0 {
            return Self::IDENTITY;
        }
        let s = Point3::new(s.x / s_len, s.y / s_len, s.z / s_len);

        let u = s.cross(&f);

        Self {
            values: [
                s.x,
                u.x,
                -f.x,
                0.0, // column 0
                s.y,
                u.y,
                -f.y,
                0.0, // column 1
                s.z,
                u.z,
                -f.z,
                0.0, // column 2
                -s.dot(&Point3::new(eye.x, eye.y, eye.z)),
                -u.dot(&Point3::new(eye.x, eye.y, eye.z)),
                f.dot(&Point3::new(eye.x, eye.y, eye.z)),
                1.0, // column 3
            ],
        }
    }

    /// Creates a perspective projection matrix.
    pub fn perspective(fov_y: Scalar, aspect: Scalar, near: Scalar, far: Scalar) -> Self {
        let f = 1.0 / (fov_y / 2.0).tan();
        let nf = 1.0 / (near - far);

        Self {
            values: [
                f / aspect,
                0.0,
                0.0,
                0.0, // column 0
                0.0,
                f,
                0.0,
                0.0, // column 1
                0.0,
                0.0,
                (far + near) * nf,
                -1.0, // column 2
                0.0,
                0.0,
                2.0 * far * near * nf,
                0.0, // column 3
            ],
        }
    }

    /// Creates an orthographic projection matrix.
    pub fn ortho(
        left: Scalar,
        right: Scalar,
        bottom: Scalar,
        top: Scalar,
        near: Scalar,
        far: Scalar,
    ) -> Self {
        let dx = right - left;
        let dy = top - bottom;
        let dz = far - near;

        Self {
            values: [
                2.0 / dx,
                0.0,
                0.0,
                0.0, // column 0
                0.0,
                2.0 / dy,
                0.0,
                0.0, // column 1
                0.0,
                0.0,
                -2.0 / dz,
                0.0, // column 2
                -(right + left) / dx,
                -(top + bottom) / dy,
                -(far + near) / dz,
                1.0, // column 3
            ],
        }
    }

    /// Returns true if this is the identity matrix.
    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }

    /// Get a matrix element by row and column.
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> Scalar {
        self.values[col * 4 + row]
    }

    /// Set a matrix element by row and column.
    #[inline]
    pub fn set(&mut self, row: usize, col: usize, value: Scalar) {
        self.values[col * 4 + row] = value;
    }

    /// Get a column as a 4-element array.
    #[inline]
    pub fn col(&self, idx: usize) -> [Scalar; 4] {
        let base = idx * 4;
        [
            self.values[base],
            self.values[base + 1],
            self.values[base + 2],
            self.values[base + 3],
        ]
    }

    /// Get a row as a 4-element array.
    #[inline]
    pub fn row(&self, idx: usize) -> [Scalar; 4] {
        [
            self.values[idx],
            self.values[idx + 4],
            self.values[idx + 8],
            self.values[idx + 12],
        ]
    }

    /// Concatenates this matrix with another (self * other).
    pub fn concat(&self, other: &Self) -> Self {
        let mut result = [0.0; 16];

        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.get(row, k) * other.get(k, col);
                }
                result[col * 4 + row] = sum;
            }
        }

        Self { values: result }
    }

    /// Pre-concatenates this matrix with a translation.
    pub fn pre_translate(&self, x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self::translate(x, y, z).concat(self)
    }

    /// Post-concatenates this matrix with a translation.
    pub fn post_translate(&self, x: Scalar, y: Scalar, z: Scalar) -> Self {
        self.concat(&Self::translate(x, y, z))
    }

    /// Pre-concatenates this matrix with a scale.
    pub fn pre_scale(&self, sx: Scalar, sy: Scalar, sz: Scalar) -> Self {
        Self::scale(sx, sy, sz).concat(self)
    }

    /// Post-concatenates this matrix with a scale.
    pub fn post_scale(&self, sx: Scalar, sy: Scalar, sz: Scalar) -> Self {
        self.concat(&Self::scale(sx, sy, sz))
    }

    /// Transforms a 3D point by this matrix.
    pub fn map_point3(&self, point: Point3) -> Point3 {
        let x = self.get(0, 0) * point.x
            + self.get(0, 1) * point.y
            + self.get(0, 2) * point.z
            + self.get(0, 3);
        let y = self.get(1, 0) * point.x
            + self.get(1, 1) * point.y
            + self.get(1, 2) * point.z
            + self.get(1, 3);
        let z = self.get(2, 0) * point.x
            + self.get(2, 1) * point.y
            + self.get(2, 2) * point.z
            + self.get(2, 3);
        let w = self.get(3, 0) * point.x
            + self.get(3, 1) * point.y
            + self.get(3, 2) * point.z
            + self.get(3, 3);

        if w != 0.0 && w != 1.0 {
            Point3::new(x / w, y / w, z / w)
        } else {
            Point3::new(x, y, z)
        }
    }

    /// Transforms a 2D point by this matrix (z=0, w=1).
    pub fn map_point(&self, point: Point) -> Point {
        let x = self.get(0, 0) * point.x + self.get(0, 1) * point.y + self.get(0, 3);
        let y = self.get(1, 0) * point.x + self.get(1, 1) * point.y + self.get(1, 3);
        let w = self.get(3, 0) * point.x + self.get(3, 1) * point.y + self.get(3, 3);

        if w != 0.0 && w != 1.0 {
            Point::new(x / w, y / w)
        } else {
            Point::new(x, y)
        }
    }

    /// Computes the determinant of this matrix.
    pub fn determinant(&self) -> Scalar {
        let m = &self.values;

        let s0 = m[0] * m[5] - m[4] * m[1];
        let s1 = m[0] * m[9] - m[8] * m[1];
        let s2 = m[0] * m[13] - m[12] * m[1];
        let s3 = m[4] * m[9] - m[8] * m[5];
        let s4 = m[4] * m[13] - m[12] * m[5];
        let s5 = m[8] * m[13] - m[12] * m[9];

        let c5 = m[10] * m[15] - m[14] * m[11];
        let c4 = m[6] * m[15] - m[14] * m[7];
        let c3 = m[6] * m[11] - m[10] * m[7];
        let c2 = m[2] * m[15] - m[14] * m[3];
        let c1 = m[2] * m[11] - m[10] * m[3];
        let c0 = m[2] * m[7] - m[6] * m[3];

        s0 * c5 - s1 * c4 + s2 * c3 + s3 * c2 - s4 * c1 + s5 * c0
    }

    /// Computes the inverse of this matrix, or None if singular.
    pub fn invert(&self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < 1e-10 {
            return None;
        }

        let m = &self.values;
        let inv_det = 1.0 / det;

        let s0 = m[0] * m[5] - m[4] * m[1];
        let s1 = m[0] * m[9] - m[8] * m[1];
        let s2 = m[0] * m[13] - m[12] * m[1];
        let s3 = m[4] * m[9] - m[8] * m[5];
        let s4 = m[4] * m[13] - m[12] * m[5];
        let s5 = m[8] * m[13] - m[12] * m[9];

        let c5 = m[10] * m[15] - m[14] * m[11];
        let c4 = m[6] * m[15] - m[14] * m[7];
        let c3 = m[6] * m[11] - m[10] * m[7];
        let c2 = m[2] * m[15] - m[14] * m[3];
        let c1 = m[2] * m[11] - m[10] * m[3];
        let c0 = m[2] * m[7] - m[6] * m[3];

        Some(Self {
            values: [
                (m[5] * c5 - m[9] * c4 + m[13] * c3) * inv_det,
                (-m[1] * c5 + m[9] * c2 - m[13] * c1) * inv_det,
                (m[1] * c4 - m[5] * c2 + m[13] * c0) * inv_det,
                (-m[1] * c3 + m[5] * c1 - m[9] * c0) * inv_det,
                (-m[4] * c5 + m[8] * c4 - m[12] * c3) * inv_det,
                (m[0] * c5 - m[8] * c2 + m[12] * c1) * inv_det,
                (-m[0] * c4 + m[4] * c2 - m[12] * c0) * inv_det,
                (m[0] * c3 - m[4] * c1 + m[8] * c0) * inv_det,
                (m[7] * s5 - m[11] * s4 + m[15] * s3) * inv_det,
                (-m[3] * s5 + m[11] * s2 - m[15] * s1) * inv_det,
                (m[3] * s4 - m[7] * s2 + m[15] * s0) * inv_det,
                (-m[3] * s3 + m[7] * s1 - m[11] * s0) * inv_det,
                (-m[6] * s5 + m[10] * s4 - m[14] * s3) * inv_det,
                (m[2] * s5 - m[10] * s2 + m[14] * s1) * inv_det,
                (-m[2] * s4 + m[6] * s2 - m[14] * s0) * inv_det,
                (m[2] * s3 - m[6] * s1 + m[10] * s0) * inv_det,
            ],
        })
    }

    /// Returns the transpose of this matrix.
    pub fn transpose(&self) -> Self {
        Self {
            values: [
                self.values[0],
                self.values[4],
                self.values[8],
                self.values[12],
                self.values[1],
                self.values[5],
                self.values[9],
                self.values[13],
                self.values[2],
                self.values[6],
                self.values[10],
                self.values[14],
                self.values[3],
                self.values[7],
                self.values[11],
                self.values[15],
            ],
        }
    }

    /// Extracts the upper-left 3x3 as a 2D Matrix (ignoring z and perspective).
    pub fn to_matrix(&self) -> Matrix {
        Matrix {
            values: [
                self.get(0, 0),
                self.get(0, 1),
                self.get(0, 3),
                self.get(1, 0),
                self.get(1, 1),
                self.get(1, 3),
                self.get(3, 0),
                self.get(3, 1),
                self.get(3, 3),
            ],
        }
    }

    /// Creates a 4x4 matrix from a 2D Matrix.
    pub fn from_matrix(m: &Matrix) -> Self {
        Self {
            values: [
                m.values[0],
                m.values[3],
                0.0,
                m.values[6], // column 0
                m.values[1],
                m.values[4],
                0.0,
                m.values[7], // column 1
                0.0,
                0.0,
                1.0,
                0.0, // column 2
                m.values[2],
                m.values[5],
                0.0,
                m.values[8], // column 3
            ],
        }
    }
}

impl std::ops::Mul for Matrix44 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.concat(&rhs)
    }
}

impl std::ops::MulAssign for Matrix44 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.concat(&rhs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let m = Matrix44::identity();
        assert!(m.is_identity());

        let p = Point3::new(1.0, 2.0, 3.0);
        let result = m.map_point3(p);
        assert!((result.x - p.x).abs() < 1e-6);
        assert!((result.y - p.y).abs() < 1e-6);
        assert!((result.z - p.z).abs() < 1e-6);
    }

    #[test]
    fn test_translate() {
        let m = Matrix44::translate(10.0, 20.0, 30.0);
        let p = Point3::new(1.0, 2.0, 3.0);
        let result = m.map_point3(p);
        assert!((result.x - 11.0).abs() < 1e-6);
        assert!((result.y - 22.0).abs() < 1e-6);
        assert!((result.z - 33.0).abs() < 1e-6);
    }

    #[test]
    fn test_scale() {
        let m = Matrix44::scale(2.0, 3.0, 4.0);
        let p = Point3::new(1.0, 2.0, 3.0);
        let result = m.map_point3(p);
        assert!((result.x - 2.0).abs() < 1e-6);
        assert!((result.y - 6.0).abs() < 1e-6);
        assert!((result.z - 12.0).abs() < 1e-6);
    }

    #[test]
    fn test_invert() {
        let m = Matrix44::translate(10.0, 20.0, 30.0);
        let inv = m.invert().unwrap();
        let result = m.concat(&inv);
        assert!((result.values[0] - 1.0).abs() < 1e-6);
        assert!((result.values[5] - 1.0).abs() < 1e-6);
        assert!((result.values[10] - 1.0).abs() < 1e-6);
        assert!((result.values[15] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_to_from_matrix() {
        let m3 = Matrix::translate(10.0, 20.0);
        let m4 = Matrix44::from_matrix(&m3);
        let m3_back = m4.to_matrix();

        let p = Point::new(5.0, 7.0);
        let r1 = m3.map_point(p);
        let r2 = m3_back.map_point(p);
        assert!((r1.x - r2.x).abs() < 1e-6);
        assert!((r1.y - r2.y).abs() < 1e-6);
    }
}
