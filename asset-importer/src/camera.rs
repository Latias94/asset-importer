//! Camera representation and utilities

use crate::{
    ptr::SharedPtr,
    scene::Scene,
    sys,
    types::{Vector3D, ai_string_to_string, from_ai_vector3d},
};

/// A camera in the scene
#[derive(Clone)]
pub struct Camera {
    #[allow(dead_code)]
    scene: Scene,
    camera_ptr: SharedPtr<sys::aiCamera>,
}

impl Camera {
    /// Create a Camera from a raw Assimp camera pointer
    ///
    /// # Safety
    /// Caller must ensure `camera_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(scene: Scene, camera_ptr: *const sys::aiCamera) -> Self {
        debug_assert!(!camera_ptr.is_null());
        let camera_ptr = unsafe { SharedPtr::new_unchecked(camera_ptr) };
        Self { scene, camera_ptr }
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw_sys(&self) -> *const sys::aiCamera {
        self.camera_ptr.as_ptr()
    }

    /// Get the raw camera pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> *const sys::aiCamera {
        self.as_raw_sys()
    }

    #[inline]
    fn raw(&self) -> &sys::aiCamera {
        unsafe { &*self.camera_ptr.as_ptr() }
    }

    /// Get the name of the camera
    pub fn name(&self) -> String {
        ai_string_to_string(&self.raw().mName)
    }

    /// Get the position of the camera
    pub fn position(&self) -> Vector3D {
        from_ai_vector3d(self.raw().mPosition)
    }

    /// Get the up vector of the camera
    pub fn up(&self) -> Vector3D {
        from_ai_vector3d(self.raw().mUp)
    }

    /// Get the look-at vector of the camera
    pub fn look_at(&self) -> Vector3D {
        from_ai_vector3d(self.raw().mLookAt)
    }

    /// Get the horizontal field of view in radians
    pub fn horizontal_fov(&self) -> f32 {
        self.raw().mHorizontalFOV
    }

    /// Get the near clipping plane distance
    pub fn clip_plane_near(&self) -> f32 {
        self.raw().mClipPlaneNear
    }

    /// Get the far clipping plane distance
    pub fn clip_plane_far(&self) -> f32 {
        self.raw().mClipPlaneFar
    }

    /// Get the aspect ratio
    pub fn aspect(&self) -> f32 {
        self.raw().mAspect
    }

    /// Get the orthographic width (for orthographic cameras)
    pub fn orthographic_width(&self) -> f32 {
        self.raw().mOrthographicWidth
    }
}
