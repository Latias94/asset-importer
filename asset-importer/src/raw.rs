//! Raw, zero-copy view types for Assimp-owned scene memory.
//!
//! These types are `#[repr(C)]` mirrors of selected Assimp structs, intended for
//! borrowing data without allocation while keeping `asset_importer::sys` optional.

#![allow(non_snake_case)]

/// Mirror of Assimp `aiVector3D`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct AiVector3D {
    /// X component
    pub x: f32,
    /// Y component
    pub y: f32,
    /// Z component
    pub z: f32,
}

/// Mirror of Assimp `aiColor4D`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct AiColor4D {
    /// Red component
    pub r: f32,
    /// Green component
    pub g: f32,
    /// Blue component
    pub b: f32,
    /// Alpha component
    pub a: f32,
}

/// Mirror of Assimp `aiTexel` (ARGB8888).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct AiTexel {
    /// Blue component
    pub b: u8,
    /// Green component
    pub g: u8,
    /// Red component
    pub r: u8,
    /// Alpha component
    pub a: u8,
}

/// Mirror of Assimp `aiFace`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct AiFace {
    /// Number of indices in this face.
    pub mNumIndices: u32,
    /// Pointer to index array.
    pub mIndices: *mut u32,
}

impl AiFace {
    /// Number of indices.
    pub fn num_indices(&self) -> usize {
        self.mNumIndices as usize
    }

    /// Index slice (zero-copy). Returns empty slice if pointer is null or count is 0.
    pub fn indices(&self) -> &[u32] {
        unsafe {
            if self.mIndices.is_null() || self.mNumIndices == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(self.mIndices, self.mNumIndices as usize)
            }
        }
    }
}

/// Mirror of Assimp `aiQuaternion`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct AiQuaternion {
    /// W component
    pub w: f32,
    /// X component
    pub x: f32,
    /// Y component
    pub y: f32,
    /// Z component
    pub z: f32,
}

/// Mirror of Assimp `aiVectorKey`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct AiVectorKey {
    /// Time of this key in ticks.
    pub mTime: f64,
    /// Value at this time.
    pub mValue: AiVector3D,
    /// Interpolation enum value (Assimp `aiAnimInterpolation`).
    pub mInterpolation: i32,
}

/// Mirror of Assimp `aiQuatKey`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct AiQuatKey {
    /// Time of this key in ticks.
    pub mTime: f64,
    /// Quaternion value at this time.
    pub mValue: AiQuaternion,
    /// Interpolation enum value (Assimp `aiAnimInterpolation`).
    pub mInterpolation: i32,
}

/// Mirror of Assimp `aiVertexWeight`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct AiVertexWeight {
    /// Vertex index.
    pub mVertexId: u32,
    /// Weight value.
    pub mWeight: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys;

    #[test]
    fn test_layout_matches_sys() {
        assert_eq!(
            std::mem::size_of::<AiVector3D>(),
            std::mem::size_of::<sys::aiVector3D>()
        );
        assert_eq!(
            std::mem::align_of::<AiVector3D>(),
            std::mem::align_of::<sys::aiVector3D>()
        );

        assert_eq!(
            std::mem::size_of::<AiColor4D>(),
            std::mem::size_of::<sys::aiColor4D>()
        );
        assert_eq!(
            std::mem::align_of::<AiColor4D>(),
            std::mem::align_of::<sys::aiColor4D>()
        );

        assert_eq!(
            std::mem::size_of::<AiFace>(),
            std::mem::size_of::<sys::aiFace>()
        );
        assert_eq!(
            std::mem::align_of::<AiFace>(),
            std::mem::align_of::<sys::aiFace>()
        );

        assert_eq!(
            std::mem::size_of::<AiTexel>(),
            std::mem::size_of::<sys::aiTexel>()
        );
        assert_eq!(
            std::mem::align_of::<AiTexel>(),
            std::mem::align_of::<sys::aiTexel>()
        );

        assert_eq!(
            std::mem::size_of::<AiQuaternion>(),
            std::mem::size_of::<sys::aiQuaternion>()
        );
        assert_eq!(
            std::mem::align_of::<AiQuaternion>(),
            std::mem::align_of::<sys::aiQuaternion>()
        );

        assert_eq!(
            std::mem::size_of::<AiVectorKey>(),
            std::mem::size_of::<sys::aiVectorKey>()
        );
        assert_eq!(
            std::mem::align_of::<AiVectorKey>(),
            std::mem::align_of::<sys::aiVectorKey>()
        );

        assert_eq!(
            std::mem::size_of::<AiQuatKey>(),
            std::mem::size_of::<sys::aiQuatKey>()
        );
        assert_eq!(
            std::mem::align_of::<AiQuatKey>(),
            std::mem::align_of::<sys::aiQuatKey>()
        );

        assert_eq!(
            std::mem::size_of::<AiVertexWeight>(),
            std::mem::size_of::<sys::aiVertexWeight>()
        );
        assert_eq!(
            std::mem::align_of::<AiVertexWeight>(),
            std::mem::align_of::<sys::aiVertexWeight>()
        );
    }
}
