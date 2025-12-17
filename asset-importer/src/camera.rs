//! Camera representation and utilities

use std::marker::PhantomData;

use crate::{
    ptr::SharedPtr,
    sys,
    types::{Vector3D, ai_string_to_string, from_ai_vector3d},
};

/// A camera in the scene
#[derive(Clone, Copy)]
pub struct Camera<'a> {
    camera_ptr: SharedPtr<sys::aiCamera>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Camera<'a> {
    /// Create a Camera from a raw Assimp camera pointer
    ///
    /// # Safety
    /// Caller must ensure `camera_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(camera_ptr: *const sys::aiCamera) -> Self {
        debug_assert!(!camera_ptr.is_null());
        let camera_ptr = unsafe { SharedPtr::new_unchecked(camera_ptr) };
        Self {
            camera_ptr,
            _marker: PhantomData,
        }
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

    /// Get the name of the camera
    pub fn name(&self) -> String {
        unsafe { ai_string_to_string(&(*self.camera_ptr.as_ptr()).mName) }
    }

    /// Get the position of the camera
    pub fn position(&self) -> Vector3D {
        unsafe {
            let camera = &*self.camera_ptr.as_ptr();
            from_ai_vector3d(camera.mPosition)
        }
    }

    /// Get the up vector of the camera
    pub fn up(&self) -> Vector3D {
        unsafe {
            let camera = &*self.camera_ptr.as_ptr();
            from_ai_vector3d(camera.mUp)
        }
    }

    /// Get the look-at vector of the camera
    pub fn look_at(&self) -> Vector3D {
        unsafe {
            let camera = &*self.camera_ptr.as_ptr();
            from_ai_vector3d(camera.mLookAt)
        }
    }

    /// Get the horizontal field of view in radians
    pub fn horizontal_fov(&self) -> f32 {
        unsafe { (*self.camera_ptr.as_ptr()).mHorizontalFOV }
    }

    /// Get the near clipping plane distance
    pub fn clip_plane_near(&self) -> f32 {
        unsafe { (*self.camera_ptr.as_ptr()).mClipPlaneNear }
    }

    /// Get the far clipping plane distance
    pub fn clip_plane_far(&self) -> f32 {
        unsafe { (*self.camera_ptr.as_ptr()).mClipPlaneFar }
    }

    /// Get the aspect ratio
    pub fn aspect(&self) -> f32 {
        unsafe { (*self.camera_ptr.as_ptr()).mAspect }
    }

    /// Get the orthographic width (for orthographic cameras)
    pub fn orthographic_width(&self) -> f32 {
        unsafe { (*self.camera_ptr.as_ptr()).mOrthographicWidth }
    }
}
