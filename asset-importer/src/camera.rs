//! Camera representation and utilities

use std::marker::PhantomData;
use std::ptr::NonNull;

use crate::{
    sys,
    types::{Vector3D, ai_string_to_string, from_ai_vector3d},
};

/// A camera in the scene
#[derive(Clone, Copy)]
pub struct Camera<'a> {
    camera_ptr: NonNull<sys::aiCamera>,
    _marker: PhantomData<&'a sys::aiScene>,
}

unsafe impl<'a> Send for Camera<'a> {}
unsafe impl<'a> Sync for Camera<'a> {}

impl<'a> Camera<'a> {
    /// Create a Camera from a raw Assimp camera pointer
    ///
    /// # Safety
    /// Caller must ensure `camera_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(camera_ptr: *const sys::aiCamera) -> Self {
        let camera_ptr =
            NonNull::new(camera_ptr as *mut sys::aiCamera).expect("aiCamera pointer is null");
        Self {
            camera_ptr,
            _marker: PhantomData,
        }
    }

    /// Get the raw camera pointer
    pub fn as_raw(&self) -> *const sys::aiCamera {
        self.camera_ptr.as_ptr()
    }

    /// Get the name of the camera
    pub fn name(&self) -> String {
        unsafe { ai_string_to_string(&self.camera_ptr.as_ref().mName) }
    }

    /// Get the position of the camera
    pub fn position(&self) -> Vector3D {
        unsafe {
            let camera = self.camera_ptr.as_ref();
            from_ai_vector3d(camera.mPosition)
        }
    }

    /// Get the up vector of the camera
    pub fn up(&self) -> Vector3D {
        unsafe {
            let camera = self.camera_ptr.as_ref();
            from_ai_vector3d(camera.mUp)
        }
    }

    /// Get the look-at vector of the camera
    pub fn look_at(&self) -> Vector3D {
        unsafe {
            let camera = self.camera_ptr.as_ref();
            from_ai_vector3d(camera.mLookAt)
        }
    }

    /// Get the horizontal field of view in radians
    pub fn horizontal_fov(&self) -> f32 {
        unsafe { self.camera_ptr.as_ref().mHorizontalFOV }
    }

    /// Get the near clipping plane distance
    pub fn clip_plane_near(&self) -> f32 {
        unsafe { self.camera_ptr.as_ref().mClipPlaneNear }
    }

    /// Get the far clipping plane distance
    pub fn clip_plane_far(&self) -> f32 {
        unsafe { self.camera_ptr.as_ref().mClipPlaneFar }
    }

    /// Get the aspect ratio
    pub fn aspect(&self) -> f32 {
        unsafe { self.camera_ptr.as_ref().mAspect }
    }

    /// Get the orthographic width (for orthographic cameras)
    pub fn orthographic_width(&self) -> f32 {
        unsafe { self.camera_ptr.as_ref().mOrthographicWidth }
    }
}
