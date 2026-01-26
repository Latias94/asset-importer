//! Light representation and utilities

use crate::{
    ptr::SharedPtr,
    scene::Scene,
    sys,
    types::{
        Color3D, Vector2D, Vector3D, ai_string_to_string, from_ai_color3d, from_ai_vector2d,
        from_ai_vector3d,
    },
};

/// A light source in the scene
#[derive(Clone)]
pub struct Light {
    #[allow(dead_code)]
    scene: Scene,
    light_ptr: SharedPtr<sys::aiLight>,
}

impl Light {
    pub(crate) fn from_sys_ptr(scene: Scene, light_ptr: *mut sys::aiLight) -> Option<Self> {
        let light_ptr = SharedPtr::new(light_ptr as *const sys::aiLight)?;
        Some(Self { scene, light_ptr })
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw_sys(&self) -> *const sys::aiLight {
        self.light_ptr.as_ptr()
    }

    /// Get the raw light pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> *const sys::aiLight {
        self.as_raw_sys()
    }

    #[inline]
    fn raw(&self) -> &sys::aiLight {
        self.light_ptr.as_ref()
    }

    /// Get the name of the light
    pub fn name(&self) -> String {
        ai_string_to_string(&self.raw().mName)
    }

    /// Get the type of the light
    pub fn light_type(&self) -> LightType {
        LightType::from_raw(self.raw().mType)
    }

    /// Get the position of the light
    pub fn position(&self) -> Vector3D {
        from_ai_vector3d(self.raw().mPosition)
    }

    /// Get the direction of the light
    pub fn direction(&self) -> Vector3D {
        from_ai_vector3d(self.raw().mDirection)
    }

    /// Get the up vector of the light
    pub fn up(&self) -> Vector3D {
        from_ai_vector3d(self.raw().mUp)
    }

    /// Get the diffuse color of the light
    pub fn color_diffuse(&self) -> Color3D {
        from_ai_color3d(self.raw().mColorDiffuse)
    }

    /// Get the specular color of the light
    pub fn color_specular(&self) -> Color3D {
        from_ai_color3d(self.raw().mColorSpecular)
    }

    /// Get the ambient color of the light
    pub fn color_ambient(&self) -> Color3D {
        from_ai_color3d(self.raw().mColorAmbient)
    }

    /// Get the constant attenuation factor
    pub fn attenuation_constant(&self) -> f32 {
        self.raw().mAttenuationConstant
    }

    /// Get the linear attenuation factor
    pub fn attenuation_linear(&self) -> f32 {
        self.raw().mAttenuationLinear
    }

    /// Get the quadratic attenuation factor
    pub fn attenuation_quadratic(&self) -> f32 {
        self.raw().mAttenuationQuadratic
    }

    /// Get the inner cone angle for spot lights (in radians)
    pub fn angle_inner_cone(&self) -> f32 {
        self.raw().mAngleInnerCone
    }

    /// Get the outer cone angle for spot lights (in radians)
    pub fn angle_outer_cone(&self) -> f32 {
        self.raw().mAngleOuterCone
    }

    /// Get the size of the area light
    pub fn size(&self) -> Vector2D {
        from_ai_vector2d(self.raw().mSize)
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
