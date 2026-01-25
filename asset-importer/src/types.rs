//! Common math types and conversion helpers.
//!
//! This crate intentionally keeps its core API free of hard dependencies on external math crates.
//! It provides lightweight math types (`Vector2D/3D/4D`, `Matrix3x3/4x4`, `Quaternion`) and
//! conversions to/from Assimp C structs used internally.
//!
//! Optional integrations:
//! - `glam`: `From` conversions to/from `glam` types.
//! - `mint`: `From` conversions to/from `mint` types.

#![allow(missing_docs)]

use crate::ffi;
use crate::sys;
use std::borrow::Cow;

/// 2D vector (`f32`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Vector2D {
    pub x: f32,
    pub y: f32,
}

impl Vector2D {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(self.x.min(other.x), self.y.min(other.y))
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(self.x.max(other.x), self.y.max(other.y))
    }

    #[inline]
    pub fn distance_squared(self, other: Self) -> f32 {
        (self - other).length_squared()
    }
}

impl std::ops::Add for Vector2D {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Vector2D {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f32> for Vector2D {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl std::ops::Div<f32> for Vector2D {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

/// 3D vector (`f32`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Vector3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3D {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v, z: v }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len == 0.0 { Self::ZERO } else { self / len }
    }

    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }

    #[inline]
    pub fn distance_squared(self, other: Self) -> f32 {
        (self - other).length_squared()
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self * (1.0 - t) + other * t
    }

    #[inline]
    pub fn extend(self, w: f32) -> Vector4D {
        Vector4D::new(self.x, self.y, self.z, w)
    }
}

impl std::ops::Add for Vector3D {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vector3D {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for Vector3D {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl std::ops::Div<f32> for Vector3D {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

/// 4D vector (`f32`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Vector4D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vector4D {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

impl std::ops::Add for Vector4D {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.x + rhs.x,
            self.y + rhs.y,
            self.z + rhs.z,
            self.w + rhs.w,
        )
    }
}

impl std::ops::Sub for Vector4D {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(
            self.x - rhs.x,
            self.y - rhs.y,
            self.z - rhs.z,
            self.w - rhs.w,
        )
    }
}

impl std::ops::Mul<f32> for Vector4D {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
    }
}

/// 3x3 matrix (column-major; matches Assimp/glam conversion logic).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Matrix3x3 {
    pub x_axis: Vector3D,
    pub y_axis: Vector3D,
    pub z_axis: Vector3D,
}

impl Matrix3x3 {
    pub const IDENTITY: Self = Self {
        x_axis: Vector3D::new(1.0, 0.0, 0.0),
        y_axis: Vector3D::new(0.0, 1.0, 0.0),
        z_axis: Vector3D::new(0.0, 0.0, 1.0),
    };

    #[inline]
    pub const fn from_cols(x_axis: Vector3D, y_axis: Vector3D, z_axis: Vector3D) -> Self {
        Self {
            x_axis,
            y_axis,
            z_axis,
        }
    }

    #[inline]
    pub fn to_cols_array_2d(self) -> [[f32; 3]; 3] {
        [
            [self.x_axis.x, self.x_axis.y, self.x_axis.z],
            [self.y_axis.x, self.y_axis.y, self.y_axis.z],
            [self.z_axis.x, self.z_axis.y, self.z_axis.z],
        ]
    }
}

/// 4x4 matrix (column-major).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Matrix4x4 {
    pub x_axis: Vector4D,
    pub y_axis: Vector4D,
    pub z_axis: Vector4D,
    pub w_axis: Vector4D,
}

impl Matrix4x4 {
    pub const IDENTITY: Self = Self {
        x_axis: Vector4D::new(1.0, 0.0, 0.0, 0.0),
        y_axis: Vector4D::new(0.0, 1.0, 0.0, 0.0),
        z_axis: Vector4D::new(0.0, 0.0, 1.0, 0.0),
        w_axis: Vector4D::new(0.0, 0.0, 0.0, 1.0),
    };

    #[inline]
    pub const fn from_cols(
        x_axis: Vector4D,
        y_axis: Vector4D,
        z_axis: Vector4D,
        w_axis: Vector4D,
    ) -> Self {
        Self {
            x_axis,
            y_axis,
            z_axis,
            w_axis,
        }
    }

    #[inline]
    pub fn to_cols_array_2d(self) -> [[f32; 4]; 4] {
        [
            [self.x_axis.x, self.x_axis.y, self.x_axis.z, self.x_axis.w],
            [self.y_axis.x, self.y_axis.y, self.y_axis.z, self.y_axis.w],
            [self.z_axis.x, self.z_axis.y, self.z_axis.z, self.z_axis.w],
            [self.w_axis.x, self.w_axis.y, self.w_axis.z, self.w_axis.w],
        ]
    }

    #[inline]
    pub fn mul_vec4(self, v: Vector4D) -> Vector4D {
        // Column-major: M * v = x_axis*v.x + y_axis*v.y + z_axis*v.z + w_axis*v.w
        self.x_axis * v.x + self.y_axis * v.y + self.z_axis * v.z + self.w_axis * v.w
    }

    #[inline]
    pub fn transform_point3(self, v: Vector3D) -> Vector3D {
        let out = self.mul_vec4(v.extend(1.0));
        if out.w == 0.0 {
            Vector3D::new(out.x, out.y, out.z)
        } else {
            Vector3D::new(out.x / out.w, out.y / out.w, out.z / out.w)
        }
    }

    /// Decompose into `(scale, rotation, translation)`.
    ///
    /// This matches the method name provided by `glam::Mat4` to keep the API ergonomic.
    pub fn to_scale_rotation_translation(self) -> (Vector3D, Quaternion, Vector3D) {
        let translation = Vector3D::new(self.w_axis.x, self.w_axis.y, self.w_axis.z);

        let x = Vector3D::new(self.x_axis.x, self.x_axis.y, self.x_axis.z);
        let y = Vector3D::new(self.y_axis.x, self.y_axis.y, self.y_axis.z);
        let z = Vector3D::new(self.z_axis.x, self.z_axis.y, self.z_axis.z);

        let scale = Vector3D::new(x.length(), y.length(), z.length());

        let rx = if scale.x == 0.0 {
            Vector3D::new(1.0, 0.0, 0.0)
        } else {
            x / scale.x
        };
        let ry = if scale.y == 0.0 {
            Vector3D::new(0.0, 1.0, 0.0)
        } else {
            y / scale.y
        };
        let rz = if scale.z == 0.0 {
            Vector3D::new(0.0, 0.0, 1.0)
        } else {
            z / scale.z
        };

        let rot_m = Matrix3x3::from_cols(rx, ry, rz);
        let rotation = Quaternion::from_matrix3(rot_m);

        (scale, rotation, translation)
    }

    /// Right-handed look-at matrix (OpenGL-style).
    pub fn look_at_rh(eye: Vector3D, target: Vector3D, up: Vector3D) -> Self {
        let f = (target - eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);

        // Column-major
        Self::from_cols(
            Vector4D::new(s.x, u.x, -f.x, 0.0),
            Vector4D::new(s.y, u.y, -f.y, 0.0),
            Vector4D::new(s.z, u.z, -f.z, 0.0),
            Vector4D::new(-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0),
        )
    }

    /// Right-handed perspective projection (OpenGL-style, NDC z in [-1, 1]).
    pub fn perspective_rh(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (0.5 * fov_y).tan();
        let nf = 1.0 / (near - far);

        Self::from_cols(
            Vector4D::new(f / aspect, 0.0, 0.0, 0.0),
            Vector4D::new(0.0, f, 0.0, 0.0),
            Vector4D::new(0.0, 0.0, (far + near) * nf, -1.0),
            Vector4D::new(0.0, 0.0, (2.0 * far * near) * nf, 0.0),
        )
    }

    /// Right-handed orthographic projection (OpenGL-style, NDC z in [-1, 1]).
    pub fn orthographic_rh(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let rml = right - left;
        let tmb = top - bottom;
        let fmn = far - near;

        Self::from_cols(
            Vector4D::new(2.0 / rml, 0.0, 0.0, 0.0),
            Vector4D::new(0.0, 2.0 / tmb, 0.0, 0.0),
            Vector4D::new(0.0, 0.0, -2.0 / fmn, 0.0),
            Vector4D::new(
                -(right + left) / rml,
                -(top + bottom) / tmb,
                -(far + near) / fmn,
                1.0,
            ),
        )
    }

    /// Build from `(scale, rotation, translation)` (column-major).
    pub fn from_scale_rotation_translation(
        scale: Vector3D,
        rotation: Quaternion,
        translation: Vector3D,
    ) -> Self {
        let r = rotation.to_matrix3();
        let sx = Vector3D::new(
            r.x_axis.x * scale.x,
            r.x_axis.y * scale.x,
            r.x_axis.z * scale.x,
        );
        let sy = Vector3D::new(
            r.y_axis.x * scale.y,
            r.y_axis.y * scale.y,
            r.y_axis.z * scale.y,
        );
        let sz = Vector3D::new(
            r.z_axis.x * scale.z,
            r.z_axis.y * scale.z,
            r.z_axis.z * scale.z,
        );

        Self::from_cols(
            Vector4D::new(sx.x, sx.y, sx.z, 0.0),
            Vector4D::new(sy.x, sy.y, sy.z, 0.0),
            Vector4D::new(sz.x, sz.y, sz.z, 0.0),
            Vector4D::new(translation.x, translation.y, translation.z, 1.0),
        )
    }
}

/// Quaternion (x, y, z, w).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    #[inline]
    pub const fn from_xyzw(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    #[inline]
    pub fn to_matrix3(self) -> Matrix3x3 {
        // Standard quaternion to a rotation matrix (column-major).
        let x2 = self.x + self.x;
        let y2 = self.y + self.y;
        let z2 = self.z + self.z;

        let xx = self.x * x2;
        let yy = self.y * y2;
        let zz = self.z * z2;
        let xy = self.x * y2;
        let xz = self.x * z2;
        let yz = self.y * z2;
        let wx = self.w * x2;
        let wy = self.w * y2;
        let wz = self.w * z2;

        // Row-major 3x3:
        // [1-(yy+zz)  xy-wz      xz+wy]
        // [xy+wz      1-(xx+zz)  yz-wx]
        // [xz-wy      yz+wx      1-(xx+yy)]
        // Convert to column-major output:
        Matrix3x3::from_cols(
            Vector3D::new(1.0 - (yy + zz), xy + wz, xz - wy),
            Vector3D::new(xy - wz, 1.0 - (xx + zz), yz + wx),
            Vector3D::new(xz + wy, yz - wx, 1.0 - (xx + yy)),
        )
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = (self.dot(self)).sqrt();
        if len == 0.0 {
            Self::IDENTITY
        } else {
            Self::from_xyzw(self.x / len, self.y / len, self.z / len, self.w / len)
        }
    }

    pub fn slerp(self, other: Self, t: f32) -> Self {
        // Standard quaternion slerp with shortest-path handling.
        let q1 = self.normalize();
        let mut q2 = other.normalize();

        let mut dot = q1.dot(q2);
        if dot < 0.0 {
            dot = -dot;
            q2 = Self::from_xyzw(-q2.x, -q2.y, -q2.z, -q2.w);
        }

        const DOT_THRESHOLD: f32 = 0.9995;
        if dot > DOT_THRESHOLD {
            // Fall back to normalized lerp to avoid precision issues.
            let out = Self::from_xyzw(
                q1.x + t * (q2.x - q1.x),
                q1.y + t * (q2.y - q1.y),
                q1.z + t * (q2.z - q1.z),
                q1.w + t * (q2.w - q1.w),
            );
            return out.normalize();
        }

        let theta_0 = dot.acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();

        let s0 = (theta_0 - theta).sin() / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;

        Self::from_xyzw(
            q1.x * s0 + q2.x * s1,
            q1.y * s0 + q2.y * s1,
            q1.z * s0 + q2.z * s1,
            q1.w * s0 + q2.w * s1,
        )
    }

    pub fn from_matrix3(m: Matrix3x3) -> Self {
        // Based on the classic matrix-to-quaternion conversion (assumes orthonormal basis).
        let c = m.to_cols_array_2d();

        // Convert column-major to individual elements in row-major form:
        let m00 = c[0][0];
        let m01 = c[1][0];
        let m02 = c[2][0];
        let m10 = c[0][1];
        let m11 = c[1][1];
        let m12 = c[2][1];
        let m20 = c[0][2];
        let m21 = c[1][2];
        let m22 = c[2][2];

        let trace = m00 + m11 + m22;
        if trace > 0.0 {
            let s = (trace + 1.0).sqrt() * 2.0;
            let w = 0.25 * s;
            let x = (m21 - m12) / s;
            let y = (m02 - m20) / s;
            let z = (m10 - m01) / s;
            Self::from_xyzw(x, y, z, w)
        } else if m00 > m11 && m00 > m22 {
            let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
            let w = (m21 - m12) / s;
            let x = 0.25 * s;
            let y = (m01 + m10) / s;
            let z = (m02 + m20) / s;
            Self::from_xyzw(x, y, z, w)
        } else if m11 > m22 {
            let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
            let w = (m02 - m20) / s;
            let x = (m01 + m10) / s;
            let y = 0.25 * s;
            let z = (m12 + m21) / s;
            Self::from_xyzw(x, y, z, w)
        } else {
            let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
            let w = (m10 - m01) / s;
            let x = (m02 + m20) / s;
            let y = (m12 + m21) / s;
            let z = 0.25 * s;
            Self::from_xyzw(x, y, z, w)
        }
    }
}

/// RGB color (alias).
pub type Color3D = Vector3D;
/// RGBA color (alias).
pub type Color4D = Vector4D;

/// Convert Assimp `aiString` to a UTF-8 string (lossy).
///
/// Assimp stores the length explicitly; do not assume the buffer is NUL-terminated.
#[inline]
pub(crate) fn ai_string_to_str(value: &sys::aiString) -> Cow<'_, str> {
    let len = (value.length as usize).min(value.data.len());
    if len == 0 {
        return Cow::Borrowed("");
    }
    let bytes = ffi::slice_from_ptr_len(value, value.data.as_ptr() as *const u8, len);
    String::from_utf8_lossy(bytes)
}

/// Convert Assimp `aiString` to an owned UTF-8 string (lossy).
#[inline]
pub(crate) fn ai_string_to_string(value: &sys::aiString) -> String {
    ai_string_to_str(value).into_owned()
}

// ---- Assimp <-> crate math conversions (internal) ----

#[inline]
pub(crate) fn from_ai_vector3d(v: sys::aiVector3D) -> Vector3D {
    Vector3D::new(v.x, v.y, v.z)
}

#[inline]
pub(crate) fn to_ai_vector3d(v: Vector3D) -> sys::aiVector3D {
    sys::aiVector3D {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

#[inline]
pub(crate) fn from_ai_vector2d(v: sys::aiVector2D) -> Vector2D {
    Vector2D::new(v.x, v.y)
}

#[inline]
pub(crate) fn to_ai_vector2d(v: Vector2D) -> sys::aiVector2D {
    sys::aiVector2D { x: v.x, y: v.y }
}

#[inline]
pub(crate) fn from_ai_matrix4x4(m: sys::aiMatrix4x4) -> Matrix4x4 {
    // Assimp stores matrices row-major (a1..d4 are rows); `Matrix4x4` is column-major.
    Matrix4x4::from_cols(
        Vector4D::new(m.a1, m.b1, m.c1, m.d1),
        Vector4D::new(m.a2, m.b2, m.c2, m.d2),
        Vector4D::new(m.a3, m.b3, m.c3, m.d3),
        Vector4D::new(m.a4, m.b4, m.c4, m.d4),
    )
}

#[inline]
pub(crate) fn to_ai_matrix4x4(m: Matrix4x4) -> sys::aiMatrix4x4 {
    let cols = m.to_cols_array_2d();
    sys::aiMatrix4x4 {
        a1: cols[0][0],
        a2: cols[1][0],
        a3: cols[2][0],
        a4: cols[3][0],
        b1: cols[0][1],
        b2: cols[1][1],
        b3: cols[2][1],
        b4: cols[3][1],
        c1: cols[0][2],
        c2: cols[1][2],
        c3: cols[2][2],
        c4: cols[3][2],
        d1: cols[0][3],
        d2: cols[1][3],
        d3: cols[2][3],
        d4: cols[3][3],
    }
}

#[inline]
pub(crate) fn from_ai_matrix3x3(m: sys::aiMatrix3x3) -> Matrix3x3 {
    Matrix3x3::from_cols(
        Vector3D::new(m.a1, m.b1, m.c1),
        Vector3D::new(m.a2, m.b2, m.c2),
        Vector3D::new(m.a3, m.b3, m.c3),
    )
}

#[inline]
pub(crate) fn to_ai_matrix3x3(m: Matrix3x3) -> sys::aiMatrix3x3 {
    let cols = m.to_cols_array_2d();
    sys::aiMatrix3x3 {
        a1: cols[0][0],
        a2: cols[1][0],
        a3: cols[2][0],
        b1: cols[0][1],
        b2: cols[1][1],
        b3: cols[2][1],
        c1: cols[0][2],
        c2: cols[1][2],
        c3: cols[2][2],
    }
}

#[inline]
pub(crate) fn from_ai_quaternion(q: sys::aiQuaternion) -> Quaternion {
    Quaternion::from_xyzw(q.x, q.y, q.z, q.w)
}

#[inline]
pub(crate) fn from_ai_color3d(c: sys::aiColor3D) -> Color3D {
    Color3D::new(c.r, c.g, c.b)
}

#[cfg(feature = "raw-sys")]
#[inline]
pub(crate) fn to_ai_quaternion(q: Quaternion) -> sys::aiQuaternion {
    sys::aiQuaternion {
        w: q.w,
        x: q.x,
        y: q.y,
        z: q.z,
    }
}

#[cfg(feature = "raw-sys")]
#[inline]
pub(crate) fn to_ai_color3d(c: Color3D) -> sys::aiColor3D {
    sys::aiColor3D {
        r: c.x,
        g: c.y,
        b: c.z,
    }
}

#[cfg(feature = "raw-sys")]
#[inline]
pub(crate) fn from_ai_color4d(c: sys::aiColor4D) -> Color4D {
    Color4D::new(c.r, c.g, c.b, c.a)
}

#[cfg(feature = "raw-sys")]
#[inline]
pub(crate) fn to_ai_color4d(c: Color4D) -> sys::aiColor4D {
    sys::aiColor4D {
        r: c.x,
        g: c.y,
        b: c.z,
        a: c.w,
    }
}

// ---- Optional sys interop helpers (requires `raw-sys`) ----

/// Convert Assimp `aiString` to a UTF-8 string (lossy), for `raw-sys` users.
#[cfg(feature = "raw-sys")]
pub fn ai_string_to_str_sys(value: &sys::aiString) -> Cow<'_, str> {
    ai_string_to_str(value)
}

/// Convert Assimp `aiString` to an owned UTF-8 string (lossy), for `raw-sys` users.
#[cfg(feature = "raw-sys")]
pub fn ai_string_to_string_sys(value: &sys::aiString) -> String {
    ai_string_to_string(value)
}

#[cfg(feature = "raw-sys")]
pub fn from_ai_vector3d_sys(v: sys::aiVector3D) -> Vector3D {
    from_ai_vector3d(v)
}

#[cfg(feature = "raw-sys")]
pub fn to_ai_vector3d_sys(v: Vector3D) -> sys::aiVector3D {
    to_ai_vector3d(v)
}

#[cfg(feature = "raw-sys")]
pub fn from_ai_vector2d_sys(v: sys::aiVector2D) -> Vector2D {
    from_ai_vector2d(v)
}

#[cfg(feature = "raw-sys")]
pub fn to_ai_vector2d_sys(v: Vector2D) -> sys::aiVector2D {
    to_ai_vector2d(v)
}

#[cfg(feature = "raw-sys")]
pub fn from_ai_matrix4x4_sys(m: sys::aiMatrix4x4) -> Matrix4x4 {
    from_ai_matrix4x4(m)
}

#[cfg(feature = "raw-sys")]
pub fn to_ai_matrix4x4_sys(m: Matrix4x4) -> sys::aiMatrix4x4 {
    to_ai_matrix4x4(m)
}

#[cfg(feature = "raw-sys")]
pub fn from_ai_matrix3x3_sys(m: sys::aiMatrix3x3) -> Matrix3x3 {
    from_ai_matrix3x3(m)
}

#[cfg(feature = "raw-sys")]
pub fn to_ai_matrix3x3_sys(m: Matrix3x3) -> sys::aiMatrix3x3 {
    to_ai_matrix3x3(m)
}

#[cfg(feature = "raw-sys")]
pub fn from_ai_quaternion_sys(q: sys::aiQuaternion) -> Quaternion {
    from_ai_quaternion(q)
}

#[cfg(feature = "raw-sys")]
pub fn to_ai_quaternion_sys(q: Quaternion) -> sys::aiQuaternion {
    to_ai_quaternion(q)
}

#[cfg(feature = "raw-sys")]
pub fn from_ai_color3d_sys(c: sys::aiColor3D) -> Color3D {
    from_ai_color3d(c)
}

#[cfg(feature = "raw-sys")]
pub fn to_ai_color3d_sys(c: Color3D) -> sys::aiColor3D {
    to_ai_color3d(c)
}

#[cfg(feature = "raw-sys")]
pub fn from_ai_color4d_sys(c: sys::aiColor4D) -> Color4D {
    from_ai_color4d(c)
}

#[cfg(feature = "raw-sys")]
pub fn to_ai_color4d_sys(c: Color4D) -> sys::aiColor4D {
    to_ai_color4d(c)
}

// ---- Optional integrations ----

#[cfg(feature = "mint")]
mod mint_integration {
    use super::*;

    impl From<mint::Vector3<f32>> for Vector3D {
        fn from(v: mint::Vector3<f32>) -> Self {
            Self::new(v.x, v.y, v.z)
        }
    }

    impl From<Vector3D> for mint::Vector3<f32> {
        fn from(v: Vector3D) -> Self {
            Self {
                x: v.x,
                y: v.y,
                z: v.z,
            }
        }
    }

    impl From<mint::Vector2<f32>> for Vector2D {
        fn from(v: mint::Vector2<f32>) -> Self {
            Self::new(v.x, v.y)
        }
    }

    impl From<Vector2D> for mint::Vector2<f32> {
        fn from(v: Vector2D) -> Self {
            Self { x: v.x, y: v.y }
        }
    }

    impl From<mint::ColumnMatrix4<f32>> for Matrix4x4 {
        fn from(m: mint::ColumnMatrix4<f32>) -> Self {
            Self::from_cols(
                Vector4D::new(m.x.x, m.x.y, m.x.z, m.x.w),
                Vector4D::new(m.y.x, m.y.y, m.y.z, m.y.w),
                Vector4D::new(m.z.x, m.z.y, m.z.z, m.z.w),
                Vector4D::new(m.w.x, m.w.y, m.w.z, m.w.w),
            )
        }
    }

    impl From<Matrix4x4> for mint::ColumnMatrix4<f32> {
        fn from(m: Matrix4x4) -> Self {
            let cols = m.to_cols_array_2d();
            mint::ColumnMatrix4 {
                x: mint::Vector4 {
                    x: cols[0][0],
                    y: cols[0][1],
                    z: cols[0][2],
                    w: cols[0][3],
                },
                y: mint::Vector4 {
                    x: cols[1][0],
                    y: cols[1][1],
                    z: cols[1][2],
                    w: cols[1][3],
                },
                z: mint::Vector4 {
                    x: cols[2][0],
                    y: cols[2][1],
                    z: cols[2][2],
                    w: cols[2][3],
                },
                w: mint::Vector4 {
                    x: cols[3][0],
                    y: cols[3][1],
                    z: cols[3][2],
                    w: cols[3][3],
                },
            }
        }
    }

    impl From<mint::Quaternion<f32>> for Quaternion {
        fn from(q: mint::Quaternion<f32>) -> Self {
            Quaternion::from_xyzw(q.v.x, q.v.y, q.v.z, q.s)
        }
    }

    impl From<Quaternion> for mint::Quaternion<f32> {
        fn from(q: Quaternion) -> Self {
            mint::Quaternion {
                s: q.w,
                v: mint::Vector3 {
                    x: q.x,
                    y: q.y,
                    z: q.z,
                },
            }
        }
    }
}

#[cfg(feature = "glam")]
mod glam_integration {
    use super::*;

    impl From<glam::Vec2> for Vector2D {
        fn from(v: glam::Vec2) -> Self {
            Self::new(v.x, v.y)
        }
    }

    impl From<Vector2D> for glam::Vec2 {
        fn from(v: Vector2D) -> Self {
            glam::Vec2::new(v.x, v.y)
        }
    }

    impl From<glam::Vec3> for Vector3D {
        fn from(v: glam::Vec3) -> Self {
            Self::new(v.x, v.y, v.z)
        }
    }

    impl From<Vector3D> for glam::Vec3 {
        fn from(v: Vector3D) -> Self {
            glam::Vec3::new(v.x, v.y, v.z)
        }
    }

    impl From<glam::Vec4> for Vector4D {
        fn from(v: glam::Vec4) -> Self {
            Self::new(v.x, v.y, v.z, v.w)
        }
    }

    impl From<Vector4D> for glam::Vec4 {
        fn from(v: Vector4D) -> Self {
            glam::Vec4::new(v.x, v.y, v.z, v.w)
        }
    }

    impl From<glam::Quat> for Quaternion {
        fn from(q: glam::Quat) -> Self {
            Self::from_xyzw(q.x, q.y, q.z, q.w)
        }
    }

    impl From<Quaternion> for glam::Quat {
        fn from(q: Quaternion) -> Self {
            glam::Quat::from_xyzw(q.x, q.y, q.z, q.w)
        }
    }

    impl From<glam::Mat3> for Matrix3x3 {
        fn from(m: glam::Mat3) -> Self {
            let cols = m.to_cols_array_2d();
            Self::from_cols(
                Vector3D::new(cols[0][0], cols[0][1], cols[0][2]),
                Vector3D::new(cols[1][0], cols[1][1], cols[1][2]),
                Vector3D::new(cols[2][0], cols[2][1], cols[2][2]),
            )
        }
    }

    impl From<Matrix3x3> for glam::Mat3 {
        fn from(m: Matrix3x3) -> Self {
            let cols = m.to_cols_array_2d();
            glam::Mat3::from_cols_array_2d(&cols)
        }
    }

    impl From<glam::Mat4> for Matrix4x4 {
        fn from(m: glam::Mat4) -> Self {
            let cols = m.to_cols_array_2d();
            Self::from_cols(
                Vector4D::new(cols[0][0], cols[0][1], cols[0][2], cols[0][3]),
                Vector4D::new(cols[1][0], cols[1][1], cols[1][2], cols[1][3]),
                Vector4D::new(cols[2][0], cols[2][1], cols[2][2], cols[2][3]),
                Vector4D::new(cols[3][0], cols[3][1], cols[3][2], cols[3][3]),
            )
        }
    }

    impl From<Matrix4x4> for glam::Mat4 {
        fn from(m: Matrix4x4) -> Self {
            let cols = m.to_cols_array_2d();
            glam::Mat4::from_cols_array_2d(&cols)
        }
    }
}
