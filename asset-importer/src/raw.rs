//! Raw, zero-copy view types for Assimp-owned scene memory.
//!
//! These types are `#[repr(C)]` mirrors of selected Assimp structs, intended for
//! borrowing data without allocation while keeping `asset_importer::sys` optional.

#![allow(non_snake_case)]

/// Mirror of Assimp `aiVector3D`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
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
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
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
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
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

    /// Index slice (zero-copy).
    ///
    /// # Safety
    /// `self.mIndices` must point to at least `self.mNumIndices` valid `u32` entries.
    /// This is true for values borrowed from an Assimp-owned scene. It is **not**
    /// guaranteed for arbitrary `AiFace` values constructed by user code.
    pub unsafe fn indices_unchecked(&self) -> &[u32] {
        unsafe {
            crate::ffi::slice_from_ptr_len(self, self.mIndices as *const u32, self.num_indices())
        }
    }
}

/// Mirror of Assimp `aiQuaternion`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
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
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
#[repr(C)]
pub struct AiVertexWeight {
    /// Vertex index.
    pub mVertexId: u32,
    /// Weight value.
    pub mWeight: f32,
}

// Every zero-copy cast must preserve size, alignment, and field offsets in all builds.
macro_rules! assert_layout {
    ($view:ty, $native:ty, $($field:ident),+ $(,)?) => {
        const _: () = {
            assert!(std::mem::size_of::<$view>() == std::mem::size_of::<$native>());
            assert!(std::mem::align_of::<$view>() == std::mem::align_of::<$native>());
            $(assert!(std::mem::offset_of!($view, $field) == std::mem::offset_of!($native, $field));)+
        };
    };
}

assert_layout!(AiVector3D, crate::sys::aiVector3D, x, y, z);
assert_layout!(AiColor4D, crate::sys::aiColor4D, r, g, b, a);
assert_layout!(AiTexel, crate::sys::aiTexel, b, g, r, a);
assert_layout!(AiFace, crate::sys::aiFace, mNumIndices, mIndices);
assert_layout!(AiQuaternion, crate::sys::aiQuaternion, w, x, y, z);
assert_layout!(
    AiVectorKey,
    crate::sys::aiVectorKey,
    mTime,
    mValue,
    mInterpolation
);
assert_layout!(
    AiQuatKey,
    crate::sys::aiQuatKey,
    mTime,
    mValue,
    mInterpolation
);
assert_layout!(
    AiVertexWeight,
    crate::sys::aiVertexWeight,
    mVertexId,
    mWeight
);
