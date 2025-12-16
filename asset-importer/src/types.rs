//! Common types and type aliases used throughout the asset importer
//!
//! This module re-exports glam types for mathematical operations and provides
//! conversion utilities between Assimp C types and Rust types.
//!
//! # Why glam?
//!
//! We use glam as our primary math library because:
//! - **Performance**: SIMD-optimized operations for vectors, matrices, and quaternions
//! - **Ecosystem**: Widely adopted in the Rust gamedev community (Bevy, wgpu, etc.)
//! - **API**: Clean, modern API with comprehensive mathematical operations
//! - **Maintenance**: Well-maintained with regular updates and optimizations
//!
//! # Usage
//!
//! ```rust,no_run
//! use asset_importer::types::*;
//! use asset_importer::Importer;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let scene = Importer::new().import_file("model.obj")?;
//! for mesh in scene.meshes() {
//!     for vertex in mesh.vertices() {
//!         // vertex is a Vector3D (glam::Vec3)
//!         let normalized = vertex.normalize();
//!         let distance = vertex.distance(Vector3D::ZERO);
//!         
//!         // All glam operations are available
//!         let transformed = Matrix4x4::from_rotation_x(1.57) * vertex.extend(1.0);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use crate::sys;
use std::borrow::Cow;

// Re-export glam types as our primary math types
pub use glam::{
    Mat3 as Matrix3x3, Mat4 as Matrix4x4, Quat as Quaternion, Vec2 as Vector2D, Vec3 as Vector3D,
    Vec4 as Vector4D,
};

/// RGB color type (alias for Vector3D)
pub type Color3D = Vector3D;

/// RGBA color type (alias for Vector4D)
pub type Color4D = Vector4D;

// Conversion functions between Assimp C types and glam types
// We use functions instead of From implementations to avoid orphan rule issues

/// Convert Assimp `aiString` to a UTF-8 string (lossy).
///
/// Assimp stores the length explicitly; do not assume the buffer is NUL-terminated.
#[inline]
pub fn ai_string_to_str(value: &sys::aiString) -> Cow<'_, str> {
    let len = (value.length as usize).min(value.data.len());
    if len == 0 {
        return Cow::Borrowed("");
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(value.data.as_ptr() as *const u8, len) };
    String::from_utf8_lossy(bytes)
}

/// Convert Assimp `aiString` to an owned UTF-8 string (lossy).
#[inline]
pub fn ai_string_to_string(value: &sys::aiString) -> String {
    ai_string_to_str(value).into_owned()
}

/// Convert aiVector3D to glam Vec3
#[inline]
pub fn from_ai_vector3d(v: sys::aiVector3D) -> Vector3D {
    Vector3D::new(v.x, v.y, v.z)
}

/// Convert glam Vec3 to aiVector3D
#[inline]
pub fn to_ai_vector3d(v: Vector3D) -> sys::aiVector3D {
    sys::aiVector3D {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

/// Convert aiVector2D to glam Vec2
#[inline]
pub fn from_ai_vector2d(v: sys::aiVector2D) -> Vector2D {
    Vector2D::new(v.x, v.y)
}

/// Convert glam Vec2 to aiVector2D
#[inline]
pub fn to_ai_vector2d(v: Vector2D) -> sys::aiVector2D {
    sys::aiVector2D { x: v.x, y: v.y }
}

/// Convert aiMatrix4x4 to glam Mat4
#[inline]
pub fn from_ai_matrix4x4(m: sys::aiMatrix4x4) -> Matrix4x4 {
    // Assimp uses row-major matrix with members a1..d4 being rows.
    // glam Mat4 expects columns. Map rows -> columns appropriately.
    Matrix4x4::from_cols(
        Vector4D::new(m.a1, m.b1, m.c1, m.d1),
        Vector4D::new(m.a2, m.b2, m.c2, m.d2),
        Vector4D::new(m.a3, m.b3, m.c3, m.d3),
        Vector4D::new(m.a4, m.b4, m.c4, m.d4),
    )
}

/// Convert glam Mat4 to aiMatrix4x4
#[inline]
pub fn to_ai_matrix4x4(m: Matrix4x4) -> sys::aiMatrix4x4 {
    // Convert glam column vectors into Assimp row-major fields
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

/// Convert aiMatrix3x3 to glam Mat3
#[inline]
pub fn from_ai_matrix3x3(m: sys::aiMatrix3x3) -> Matrix3x3 {
    // aiMatrix3x3 is row-major. glam Mat3 expects columns.
    Matrix3x3::from_cols(
        Vector3D::new(m.a1, m.b1, m.c1),
        Vector3D::new(m.a2, m.b2, m.c2),
        Vector3D::new(m.a3, m.b3, m.c3),
    )
}

/// Convert glam Mat3 to aiMatrix3x3
#[inline]
pub fn to_ai_matrix3x3(m: Matrix3x3) -> sys::aiMatrix3x3 {
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

/// Convert aiQuaternion to glam Quat
#[inline]
pub fn from_ai_quaternion(q: sys::aiQuaternion) -> Quaternion {
    Quaternion::from_xyzw(q.x, q.y, q.z, q.w)
}

/// Convert glam Quat to aiQuaternion
#[inline]
pub fn to_ai_quaternion(q: Quaternion) -> sys::aiQuaternion {
    sys::aiQuaternion {
        w: q.w,
        x: q.x,
        y: q.y,
        z: q.z,
    }
}

/// Convert aiColor3D to glam Vec3
#[inline]
pub fn from_ai_color3d(c: sys::aiColor3D) -> Color3D {
    Color3D::new(c.r, c.g, c.b)
}

/// Convert glam Vec3 to aiColor3D
#[inline]
pub fn to_ai_color3d(c: Color3D) -> sys::aiColor3D {
    sys::aiColor3D {
        r: c.x,
        g: c.y,
        b: c.z,
    }
}

/// Convert aiColor4D to glam Vec4
#[inline]
pub fn from_ai_color4d(c: sys::aiColor4D) -> Color4D {
    Color4D::new(c.r, c.g, c.b, c.a)
}

/// Convert glam Vec4 to aiColor4D
#[inline]
pub fn to_ai_color4d(c: Color4D) -> sys::aiColor4D {
    sys::aiColor4D {
        r: c.x,
        g: c.y,
        b: c.z,
        a: c.w,
    }
}

// Mint integration (optional)
#[cfg(feature = "mint")]
mod mint_integration {
    use super::*;

    /// Trait for converting to mint types
    pub trait ToMint<T> {
        /// Convert this type to a mint type
        fn to_mint(self) -> T;
    }

    /// Trait for converting from mint types
    pub trait FromMint<T> {
        /// Convert from a mint type to this type
        fn from_mint(value: T) -> Self;
    }

    impl FromMint<mint::Vector3<f32>> for Vector3D {
        #[inline]
        fn from_mint(v: mint::Vector3<f32>) -> Self {
            Vector3D::new(v.x, v.y, v.z)
        }
    }

    impl ToMint<mint::Vector3<f32>> for Vector3D {
        #[inline]
        fn to_mint(self) -> mint::Vector3<f32> {
            mint::Vector3 {
                x: self.x,
                y: self.y,
                z: self.z,
            }
        }
    }

    impl FromMint<mint::Vector2<f32>> for Vector2D {
        #[inline]
        fn from_mint(v: mint::Vector2<f32>) -> Self {
            Vector2D::new(v.x, v.y)
        }
    }

    impl ToMint<mint::Vector2<f32>> for Vector2D {
        #[inline]
        fn to_mint(self) -> mint::Vector2<f32> {
            mint::Vector2 {
                x: self.x,
                y: self.y,
            }
        }
    }

    impl FromMint<mint::ColumnMatrix4<f32>> for Matrix4x4 {
        #[inline]
        fn from_mint(m: mint::ColumnMatrix4<f32>) -> Self {
            Matrix4x4::from_cols(
                Vector4D::new(m.x.x, m.x.y, m.x.z, m.x.w),
                Vector4D::new(m.y.x, m.y.y, m.y.z, m.y.w),
                Vector4D::new(m.z.x, m.z.y, m.z.z, m.z.w),
                Vector4D::new(m.w.x, m.w.y, m.w.z, m.w.w),
            )
        }
    }

    impl ToMint<mint::ColumnMatrix4<f32>> for Matrix4x4 {
        #[inline]
        fn to_mint(self) -> mint::ColumnMatrix4<f32> {
            let cols = self.to_cols_array_2d();
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

    impl FromMint<mint::Quaternion<f32>> for Quaternion {
        #[inline]
        fn from_mint(q: mint::Quaternion<f32>) -> Self {
            Quaternion::from_xyzw(q.v.x, q.v.y, q.v.z, q.s)
        }
    }

    impl ToMint<mint::Quaternion<f32>> for Quaternion {
        #[inline]
        fn to_mint(self) -> mint::Quaternion<f32> {
            mint::Quaternion {
                s: self.w,
                v: mint::Vector3 {
                    x: self.x,
                    y: self.y,
                    z: self.z,
                },
            }
        }
    }
}

// Re-export the traits for public use when mint feature is enabled
#[cfg(feature = "mint")]
pub use mint_integration::{FromMint, ToMint};
