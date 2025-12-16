//! Light representation and utilities

use std::marker::PhantomData;
use std::ptr::NonNull;

use crate::{
    sys,
    types::{
        Color3D, Vector2D, Vector3D, ai_string_to_string, from_ai_color3d, from_ai_vector2d,
        from_ai_vector3d,
    },
};

/// A light source in the scene
#[derive(Clone, Copy)]
pub struct Light<'a> {
    light_ptr: NonNull<sys::aiLight>,
    _marker: PhantomData<&'a sys::aiScene>,
}

impl<'a> Light<'a> {
    /// Create a Light from a raw Assimp light pointer
    ///
    /// # Safety
    /// Caller must ensure `light_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(light_ptr: *const sys::aiLight) -> Self {
        let light_ptr =
            NonNull::new(light_ptr as *mut sys::aiLight).expect("aiLight pointer is null");
        Self {
            light_ptr,
            _marker: PhantomData,
        }
    }

    /// Get the raw light pointer
    pub fn as_raw(&self) -> *const sys::aiLight {
        self.light_ptr.as_ptr()
    }

    /// Get the name of the light
    pub fn name(&self) -> String {
        unsafe { ai_string_to_string(&self.light_ptr.as_ref().mName) }
    }

    /// Get the type of the light
    pub fn light_type(&self) -> LightType {
        unsafe {
            let light = self.light_ptr.as_ref();
            LightType::from_raw(light.mType)
        }
    }

    /// Get the position of the light
    pub fn position(&self) -> Vector3D {
        unsafe {
            let light = self.light_ptr.as_ref();
            from_ai_vector3d(light.mPosition)
        }
    }

    /// Get the direction of the light
    pub fn direction(&self) -> Vector3D {
        unsafe {
            let light = self.light_ptr.as_ref();
            from_ai_vector3d(light.mDirection)
        }
    }

    /// Get the up vector of the light
    pub fn up(&self) -> Vector3D {
        unsafe {
            let light = self.light_ptr.as_ref();
            from_ai_vector3d(light.mUp)
        }
    }

    /// Get the diffuse color of the light
    pub fn color_diffuse(&self) -> Color3D {
        unsafe {
            let light = self.light_ptr.as_ref();
            from_ai_color3d(light.mColorDiffuse)
        }
    }

    /// Get the specular color of the light
    pub fn color_specular(&self) -> Color3D {
        unsafe {
            let light = self.light_ptr.as_ref();
            from_ai_color3d(light.mColorSpecular)
        }
    }

    /// Get the ambient color of the light
    pub fn color_ambient(&self) -> Color3D {
        unsafe {
            let light = self.light_ptr.as_ref();
            from_ai_color3d(light.mColorAmbient)
        }
    }

    /// Get the constant attenuation factor
    pub fn attenuation_constant(&self) -> f32 {
        unsafe { self.light_ptr.as_ref().mAttenuationConstant }
    }

    /// Get the linear attenuation factor
    pub fn attenuation_linear(&self) -> f32 {
        unsafe { self.light_ptr.as_ref().mAttenuationLinear }
    }

    /// Get the quadratic attenuation factor
    pub fn attenuation_quadratic(&self) -> f32 {
        unsafe { self.light_ptr.as_ref().mAttenuationQuadratic }
    }

    /// Get the inner cone angle for spot lights (in radians)
    pub fn angle_inner_cone(&self) -> f32 {
        unsafe { self.light_ptr.as_ref().mAngleInnerCone }
    }

    /// Get the outer cone angle for spot lights (in radians)
    pub fn angle_outer_cone(&self) -> f32 {
        unsafe { self.light_ptr.as_ref().mAngleOuterCone }
    }

    /// Get the size of the area light
    pub fn size(&self) -> Vector2D {
        unsafe {
            let light = self.light_ptr.as_ref();
            from_ai_vector2d(light.mSize)
        }
    }
}

/// Types of light sources
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    /// Undefined light type
    Undefined,
    /// Directional light (like sunlight)
    Directional,
    /// Point light (omnidirectional)
    Point,
    /// Spot light (cone-shaped)
    Spot,
    /// Ambient light
    Ambient,
    /// Area light
    Area,
}

impl LightType {
    fn from_raw(value: sys::aiLightSourceType) -> Self {
        let v = value as u32;
        match v {
            x if x == sys::aiLightSourceType::aiLightSource_UNDEFINED as u32 => Self::Undefined,
            x if x == sys::aiLightSourceType::aiLightSource_DIRECTIONAL as u32 => Self::Directional,
            x if x == sys::aiLightSourceType::aiLightSource_POINT as u32 => Self::Point,
            x if x == sys::aiLightSourceType::aiLightSource_SPOT as u32 => Self::Spot,
            x if x == sys::aiLightSourceType::aiLightSource_AMBIENT as u32 => Self::Ambient,
            x if x == sys::aiLightSourceType::aiLightSource_AREA as u32 => Self::Area,
            _ => Self::Undefined,
        }
    }
}
