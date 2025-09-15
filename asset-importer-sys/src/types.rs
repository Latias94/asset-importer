//! Type extensions and convenience implementations for Assimp types
//!
//! This module provides additional implementations and convenience methods
//! for the generated Assimp types.

use crate::*;

// Implement common traits and convenience methods for Assimp types

impl aiVector3D {
    /// Create a new 3D vector
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Create a zero vector
    #[inline]
    pub const fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// Create a unit vector along the X axis
    #[inline]
    pub const fn unit_x() -> Self {
        Self::new(1.0, 0.0, 0.0)
    }

    /// Create a unit vector along the Y axis
    #[inline]
    pub const fn unit_y() -> Self {
        Self::new(0.0, 1.0, 0.0)
    }

    /// Create a unit vector along the Z axis
    #[inline]
    pub const fn unit_z() -> Self {
        Self::new(0.0, 0.0, 1.0)
    }
}

impl aiVector2D {
    /// Create a new 2D vector
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Create a zero vector
    #[inline]
    pub const fn zero() -> Self {
        Self::new(0.0, 0.0)
    }
}

impl aiColor3D {
    /// Create a new RGB color
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    /// Create a black color
    #[inline]
    pub const fn black() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// Create a white color
    #[inline]
    pub const fn white() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }
}

impl aiColor4D {
    /// Create a new RGBA color
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create a transparent black color
    #[inline]
    pub const fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Create an opaque white color
    #[inline]
    pub const fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
}

impl aiQuaternion {
    /// Create a new quaternion
    #[inline]
    pub const fn new(w: f32, x: f32, y: f32, z: f32) -> Self {
        Self { w, x, y, z }
    }

    /// Create an identity quaternion
    #[inline]
    pub const fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 0.0)
    }
}

// Conversions from standard Rust types
impl From<[f32; 3]> for aiVector3D {
    #[inline]
    fn from(array: [f32; 3]) -> Self {
        Self::new(array[0], array[1], array[2])
    }
}

impl From<(f32, f32, f32)> for aiVector3D {
    #[inline]
    fn from((x, y, z): (f32, f32, f32)) -> Self {
        Self::new(x, y, z)
    }
}

impl From<aiVector3D> for [f32; 3] {
    #[inline]
    fn from(v: aiVector3D) -> [f32; 3] {
        [v.x, v.y, v.z]
    }
}

impl From<aiVector3D> for (f32, f32, f32) {
    #[inline]
    fn from(v: aiVector3D) -> (f32, f32, f32) {
        (v.x, v.y, v.z)
    }
}

impl From<[f32; 2]> for aiVector2D {
    #[inline]
    fn from(array: [f32; 2]) -> Self {
        Self::new(array[0], array[1])
    }
}

impl From<(f32, f32)> for aiVector2D {
    #[inline]
    fn from((x, y): (f32, f32)) -> Self {
        Self::new(x, y)
    }
}

impl From<aiVector2D> for [f32; 2] {
    #[inline]
    fn from(v: aiVector2D) -> [f32; 2] {
        [v.x, v.y]
    }
}

impl From<aiVector2D> for (f32, f32) {
    #[inline]
    fn from(v: aiVector2D) -> (f32, f32) {
        (v.x, v.y)
    }
}

impl From<[f32; 3]> for aiColor3D {
    #[inline]
    fn from(array: [f32; 3]) -> Self {
        Self::new(array[0], array[1], array[2])
    }
}

impl From<[f32; 4]> for aiColor4D {
    #[inline]
    fn from(array: [f32; 4]) -> Self {
        Self::new(array[0], array[1], array[2], array[3])
    }
}

// Mint integration (if enabled)
#[cfg(feature = "mint")]
mod mint_integration {
    use super::*;

    impl From<mint::Vector3<f32>> for aiVector3D {
        #[inline]
        fn from(v: mint::Vector3<f32>) -> Self {
            Self::new(v.x, v.y, v.z)
        }
    }

    impl From<aiVector3D> for mint::Vector3<f32> {
        #[inline]
        fn from(v: aiVector3D) -> Self {
            mint::Vector3 {
                x: v.x,
                y: v.y,
                z: v.z,
            }
        }
    }

    impl From<mint::Vector2<f32>> for aiVector2D {
        #[inline]
        fn from(v: mint::Vector2<f32>) -> Self {
            Self::new(v.x, v.y)
        }
    }

    impl From<aiVector2D> for mint::Vector2<f32> {
        #[inline]
        fn from(v: aiVector2D) -> Self {
            mint::Vector2 { x: v.x, y: v.y }
        }
    }

    impl From<mint::Quaternion<f32>> for aiQuaternion {
        #[inline]
        fn from(q: mint::Quaternion<f32>) -> Self {
            Self::new(q.s, q.v.x, q.v.y, q.v.z)
        }
    }

    impl From<aiQuaternion> for mint::Quaternion<f32> {
        #[inline]
        fn from(q: aiQuaternion) -> Self {
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
