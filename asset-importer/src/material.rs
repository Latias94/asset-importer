//! Material representation and properties

use crate::{
    error::{c_str_to_string_or_empty, Error, Result},
    sys,
    types::{Color3D, Color4D},
};

/// A material containing properties like colors, textures, and shading parameters
pub struct Material {
    material_ptr: *const sys::aiMaterial,
}

impl Material {
    /// Create a Material from a raw Assimp material pointer
    pub(crate) fn from_raw(material_ptr: *const sys::aiMaterial) -> Self {
        Self { material_ptr }
    }

    /// Get the raw material pointer
    pub fn as_raw(&self) -> *const sys::aiMaterial {
        self.material_ptr
    }

    /// Get the name of the material
    pub fn name(&self) -> String {
        self.get_string_property("?mat.name").unwrap_or_default()
    }

    /// Get a string property from the material
    pub fn get_string_property(&self, key: &str) -> Option<String> {
        // This is a simplified implementation
        // The actual implementation would need to use aiGetMaterialString
        // which requires proper handling of aiString structure
        None
    }

    /// Get a float property from the material
    pub fn get_float_property(&self, key: &str) -> Option<f32> {
        // This is a simplified implementation
        // The actual implementation would need to use aiGetMaterialFloat
        None
    }

    /// Get an integer property from the material
    pub fn get_integer_property(&self, key: &str) -> Option<i32> {
        // This is a simplified implementation
        // The actual implementation would need to use aiGetMaterialInteger
        None
    }

    /// Get a color property from the material
    pub fn get_color_property(&self, key: &str) -> Option<Color4D> {
        // This is a simplified implementation
        // The actual implementation would need to use aiGetMaterialColor
        None
    }

    /// Get the diffuse color
    pub fn diffuse_color(&self) -> Option<Color3D> {
        self.get_color_property("$clr.diffuse")
            .map(|c| Color3D::new(c.x, c.y, c.z))
    }

    /// Get the specular color
    pub fn specular_color(&self) -> Option<Color3D> {
        self.get_color_property("$clr.specular")
            .map(|c| Color3D::new(c.x, c.y, c.z))
    }

    /// Get the ambient color
    pub fn ambient_color(&self) -> Option<Color3D> {
        self.get_color_property("$clr.ambient")
            .map(|c| Color3D::new(c.x, c.y, c.z))
    }

    /// Get the emissive color
    pub fn emissive_color(&self) -> Option<Color3D> {
        self.get_color_property("$clr.emissive")
            .map(|c| Color3D::new(c.x, c.y, c.z))
    }

    /// Get the shininess factor
    pub fn shininess(&self) -> Option<f32> {
        self.get_float_property("$mat.shininess")
    }

    /// Get the opacity factor
    pub fn opacity(&self) -> Option<f32> {
        self.get_float_property("$mat.opacity")
    }

    /// Get the number of textures for a specific type
    pub fn texture_count(&self, texture_type: TextureType) -> usize {
        unsafe { sys::aiGetMaterialTextureCount(self.material_ptr, texture_type as i32) as usize }
    }

    /// Get texture information for a specific type and index
    pub fn texture(&self, texture_type: TextureType, index: usize) -> Option<TextureInfo> {
        if index >= self.texture_count(texture_type) {
            return None;
        }

        unsafe {
            let mut path = sys::aiString::default();
            let mut mapping = 0i32;
            let mut uv_index = 0u32;
            let mut blend = 0.0f32;
            let mut op = 0i32;
            let mut map_mode = [0i32; 3];

            let result = sys::aiGetMaterialTexture(
                self.material_ptr,
                texture_type as i32,
                index as u32,
                &mut path,
                &mut mapping,
                &mut uv_index,
                &mut blend,
                &mut op,
                map_mode.as_mut_ptr(),
                std::ptr::null_mut(),
            );

            if result == sys::aiReturn_aiReturn_SUCCESS {
                let path_str = std::ffi::CStr::from_ptr(path.data.as_ptr() as *const i8)
                    .to_string_lossy()
                    .into_owned();

                Some(TextureInfo {
                    path: path_str,
                    mapping: TextureMapping::from_raw(mapping),
                    uv_index: uv_index,
                    blend_factor: blend,
                    operation: TextureOperation::from_raw(op),
                    map_modes: [
                        TextureMapMode::from_raw(map_mode[0]),
                        TextureMapMode::from_raw(map_mode[1]),
                        TextureMapMode::from_raw(map_mode[2]),
                    ],
                })
            } else {
                None
            }
        }
    }
}

/// Types of textures that can be applied to materials
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureType {
    Diffuse = sys::aiTextureType_aiTextureType_DIFFUSE as u32,
    Specular = sys::aiTextureType_aiTextureType_SPECULAR as u32,
    Ambient = sys::aiTextureType_aiTextureType_AMBIENT as u32,
    Emissive = sys::aiTextureType_aiTextureType_EMISSIVE as u32,
    Height = sys::aiTextureType_aiTextureType_HEIGHT as u32,
    Normals = sys::aiTextureType_aiTextureType_NORMALS as u32,
    Shininess = sys::aiTextureType_aiTextureType_SHININESS as u32,
    Opacity = sys::aiTextureType_aiTextureType_OPACITY as u32,
    Displacement = sys::aiTextureType_aiTextureType_DISPLACEMENT as u32,
    Lightmap = sys::aiTextureType_aiTextureType_LIGHTMAP as u32,
    Reflection = sys::aiTextureType_aiTextureType_REFLECTION as u32,
    BaseColor = sys::aiTextureType_aiTextureType_BASE_COLOR as u32,
    NormalCamera = sys::aiTextureType_aiTextureType_NORMAL_CAMERA as u32,
    EmissionColor = sys::aiTextureType_aiTextureType_EMISSION_COLOR as u32,
    Metalness = sys::aiTextureType_aiTextureType_METALNESS as u32,
    DiffuseRoughness = sys::aiTextureType_aiTextureType_DIFFUSE_ROUGHNESS as u32,
    AmbientOcclusion = sys::aiTextureType_aiTextureType_AMBIENT_OCCLUSION as u32,
}

/// Texture mapping modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureMapping {
    UV,
    Sphere,
    Cylinder,
    Box,
    Plane,
    Other(u32),
}

impl TextureMapping {
    fn from_raw(value: i32) -> Self {
        match value {
            sys::aiTextureMapping_aiTextureMapping_UV => Self::UV,
            sys::aiTextureMapping_aiTextureMapping_SPHERE => Self::Sphere,
            sys::aiTextureMapping_aiTextureMapping_CYLINDER => Self::Cylinder,
            sys::aiTextureMapping_aiTextureMapping_BOX => Self::Box,
            sys::aiTextureMapping_aiTextureMapping_PLANE => Self::Plane,
            other => Self::Other(other as u32),
        }
    }
}

/// Texture operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureOperation {
    Multiply,
    Add,
    Subtract,
    Divide,
    SmoothAdd,
    SignedAdd,
    Other(u32),
}

impl TextureOperation {
    fn from_raw(value: i32) -> Self {
        match value {
            sys::aiTextureOp_aiTextureOp_Multiply => Self::Multiply,
            sys::aiTextureOp_aiTextureOp_Add => Self::Add,
            sys::aiTextureOp_aiTextureOp_Subtract => Self::Subtract,
            sys::aiTextureOp_aiTextureOp_Divide => Self::Divide,
            sys::aiTextureOp_aiTextureOp_SmoothAdd => Self::SmoothAdd,
            sys::aiTextureOp_aiTextureOp_SignedAdd => Self::SignedAdd,
            other => Self::Other(other as u32),
        }
    }
}

/// Texture mapping modes for UV coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureMapMode {
    Wrap,
    Clamp,
    Mirror,
    Decal,
    Other(u32),
}

impl TextureMapMode {
    fn from_raw(value: i32) -> Self {
        match value {
            sys::aiTextureMapMode_aiTextureMapMode_Wrap => Self::Wrap,
            sys::aiTextureMapMode_aiTextureMapMode_Clamp => Self::Clamp,
            sys::aiTextureMapMode_aiTextureMapMode_Mirror => Self::Mirror,
            sys::aiTextureMapMode_aiTextureMapMode_Decal => Self::Decal,
            other => Self::Other(other as u32),
        }
    }
}

/// Information about a texture applied to a material
#[derive(Debug, Clone)]
pub struct TextureInfo {
    /// Path to the texture file
    pub path: String,
    /// Texture mapping mode
    pub mapping: TextureMapping,
    /// UV channel index
    pub uv_index: u32,
    /// Blend factor
    pub blend_factor: f32,
    /// Texture operation
    pub operation: TextureOperation,
    /// Texture map modes for U, V, W coordinates
    pub map_modes: [TextureMapMode; 3],
}
