//! Camera representation and utilities

use crate::{
    error::c_str_to_string_or_empty,
    sys,
    types::{Vector3D, from_ai_vector3d},
};

/// A camera in the scene
pub struct Camera {
    camera_ptr: *const sys::aiCamera,
}

impl Camera {
    /// Create a Camera from a raw Assimp camera pointer
    pub(crate) fn from_raw(camera_ptr: *const sys::aiCamera) -> Self {
        Self { camera_ptr }
    }

    /// Get the raw camera pointer
    pub fn as_raw(&self) -> *const sys::aiCamera {
        self.camera_ptr
    }

    /// Get the name of the camera
    pub fn name(&self) -> String {
        unsafe {
            let camera = &*self.camera_ptr;
            c_str_to_string_or_empty(camera.mName.data.as_ptr())
        }
    }

    /// Get the position of the camera
    pub fn position(&self) -> Vector3D {
        unsafe {
            let camera = &*self.camera_ptr;
            from_ai_vector3d(camera.mPosition)
        }
    }

    /// Get the up vector of the camera
    pub fn up(&self) -> Vector3D {
        unsafe {
            let camera = &*self.camera_ptr;
            from_ai_vector3d(camera.mUp)
        }
    }

    /// Get the look-at vector of the camera
    pub fn look_at(&self) -> Vector3D {
        unsafe {
            let camera = &*self.camera_ptr;
            from_ai_vector3d(camera.mLookAt)
        }
    }

    /// Get the horizontal field of view in radians
    pub fn horizontal_fov(&self) -> f32 {
        unsafe { (*self.camera_ptr).mHorizontalFOV }
    }

    /// Get the near clipping plane distance
    pub fn clip_plane_near(&self) -> f32 {
        unsafe { (*self.camera_ptr).mClipPlaneNear }
    }

    /// Get the far clipping plane distance
    pub fn clip_plane_far(&self) -> f32 {
        unsafe { (*self.camera_ptr).mClipPlaneFar }
    }

    /// Get the aspect ratio
    pub fn aspect(&self) -> f32 {
        unsafe { (*self.camera_ptr).mAspect }
    }

    /// Get the orthographic width (for orthographic cameras)
    pub fn orthographic_width(&self) -> f32 {
        unsafe { (*self.camera_ptr).mOrthographicWidth }
    }
}

impl Clone for Camera {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Camera {}

// Send and Sync are safe because:
// 1. Camera only holds a pointer to data owned by the Scene
// 2. The Scene manages the lifetime of all Assimp data
// 3. Assimp doesn't use global state and is thread-safe for read operations
// 4. The pointer remains valid as long as the Scene exists
unsafe impl Send for Camera {}
unsafe impl Sync for Camera {}
