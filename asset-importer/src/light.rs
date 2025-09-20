//! Light representation and utilities

use crate::{
    error::c_str_to_string_or_empty,
    sys,
    types::{Color3D, Vector2D, Vector3D, from_ai_color3d, from_ai_vector2d, from_ai_vector3d},
};

/// A light source in the scene
pub struct Light {
    light_ptr: *const sys::aiLight,
}

impl Light {
    /// Create a Light from a raw Assimp light pointer
    pub(crate) fn from_raw(light_ptr: *const sys::aiLight) -> Self {
        Self { light_ptr }
    }

    /// Get the raw light pointer
    pub fn as_raw(&self) -> *const sys::aiLight {
        self.light_ptr
    }

    /// Get the name of the light
    pub fn name(&self) -> String {
        unsafe {
            let light = &*self.light_ptr;
            c_str_to_string_or_empty(light.mName.data.as_ptr())
        }
    }

    /// Get the type of the light
    pub fn light_type(&self) -> LightType {
        unsafe {
            let light = &*self.light_ptr;
            LightType::from_raw(light.mType)
        }
    }

    /// Get the position of the light
    pub fn position(&self) -> Vector3D {
        unsafe {
            let light = &*self.light_ptr;
            from_ai_vector3d(light.mPosition)
        }
    }

    /// Get the direction of the light
    pub fn direction(&self) -> Vector3D {
        unsafe {
            let light = &*self.light_ptr;
            from_ai_vector3d(light.mDirection)
        }
    }

    /// Get the up vector of the light
    pub fn up(&self) -> Vector3D {
        unsafe {
            let light = &*self.light_ptr;
            from_ai_vector3d(light.mUp)
        }
    }

    /// Get the diffuse color of the light
    pub fn color_diffuse(&self) -> Color3D {
        unsafe {
            let light = &*self.light_ptr;
            from_ai_color3d(light.mColorDiffuse)
        }
    }

    /// Get the specular color of the light
    pub fn color_specular(&self) -> Color3D {
        unsafe {
            let light = &*self.light_ptr;
            from_ai_color3d(light.mColorSpecular)
        }
    }

    /// Get the ambient color of the light
    pub fn color_ambient(&self) -> Color3D {
        unsafe {
            let light = &*self.light_ptr;
            from_ai_color3d(light.mColorAmbient)
        }
    }

    /// Get the constant attenuation factor
    pub fn attenuation_constant(&self) -> f32 {
        unsafe { (*self.light_ptr).mAttenuationConstant }
    }

    /// Get the linear attenuation factor
    pub fn attenuation_linear(&self) -> f32 {
        unsafe { (*self.light_ptr).mAttenuationLinear }
    }

    /// Get the quadratic attenuation factor
    pub fn attenuation_quadratic(&self) -> f32 {
        unsafe { (*self.light_ptr).mAttenuationQuadratic }
    }

    /// Get the inner cone angle for spot lights (in radians)
    pub fn angle_inner_cone(&self) -> f32 {
        unsafe { (*self.light_ptr).mAngleInnerCone }
    }

    /// Get the outer cone angle for spot lights (in radians)
    pub fn angle_outer_cone(&self) -> f32 {
        unsafe { (*self.light_ptr).mAngleOuterCone }
    }

    /// Get the size of the area light
    pub fn size(&self) -> Vector2D {
        unsafe {
            let light = &*self.light_ptr;
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

impl Clone for Light {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Light {}

// Send and Sync are safe because:
// 1. Light only holds a pointer to data owned by the Scene
// 2. The Scene manages the lifetime of all Assimp data
// 3. Assimp doesn't use global state and is thread-safe for read operations
// 4. The pointer remains valid as long as the Scene exists
unsafe impl Send for Light {}
unsafe impl Sync for Light {}
