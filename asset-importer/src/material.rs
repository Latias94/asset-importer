//! Material representation and properties

use std::ffi::CString;

use crate::{
    sys,
    types::{Color3D, Color4D},
};

/// Standard material property keys as defined by Assimp
pub mod material_keys {
    /// Material name
    pub const NAME: &str = "?mat.name";
    /// Diffuse color
    pub const COLOR_DIFFUSE: &str = "$clr.diffuse";
    /// Ambient color
    pub const COLOR_AMBIENT: &str = "$clr.ambient";
    /// Specular color
    pub const COLOR_SPECULAR: &str = "$clr.specular";
    /// Emissive color
    pub const COLOR_EMISSIVE: &str = "$clr.emissive";
    /// Transparent color
    pub const COLOR_TRANSPARENT: &str = "$clr.transparent";
    /// Reflective color
    pub const COLOR_REFLECTIVE: &str = "$clr.reflective";
    /// Shininess factor
    pub const SHININESS: &str = "$mat.shininess";
    /// Shininess strength
    pub const SHININESS_STRENGTH: &str = "$mat.shinpercent";
    /// Opacity
    pub const OPACITY: &str = "$mat.opacity";
    /// Transparency factor
    pub const TRANSPARENCYFACTOR: &str = "$mat.transparencyfactor";
    /// Bump scaling
    pub const BUMPSCALING: &str = "$mat.bumpscaling";
    /// Refraction index
    pub const REFRACTI: &str = "$mat.refracti";
    /// Reflectivity
    pub const REFLECTIVITY: &str = "$mat.reflectivity";
    /// Shading model
    pub const SHADING_MODEL: &str = "$mat.shadingm";
    /// Blend function
    pub const BLEND_FUNC: &str = "$mat.blend";
    /// Two sided
    pub const TWOSIDED: &str = "$mat.twosided";
}

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
        self.get_string_property(material_keys::NAME)
            .unwrap_or_default()
    }

    /// Get a string property from the material
    pub fn get_string_property(&self, key: &str) -> Option<String> {
        let c_key = CString::new(key).ok()?;
        let mut ai_string = sys::aiString::default();

        let result = unsafe {
            sys::aiGetMaterialString(
                self.material_ptr,
                c_key.as_ptr(),
                0, // type
                0, // index
                &mut ai_string,
            )
        };

        if result == sys::aiReturn::aiReturn_SUCCESS {
            // Convert aiString to Rust String
            let len = ai_string.length as usize;
            if len > 0 && len < ai_string.data.len() {
                let bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(ai_string.data.as_ptr() as *const u8, len)
                };
                String::from_utf8(bytes.to_vec()).ok()
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get a float property from the material
    pub fn get_float_property(&self, key: &str) -> Option<f32> {
        let c_key = CString::new(key).ok()?;
        let mut value = 0.0f32;
        let mut max = 1u32;

        let result = unsafe {
            sys::aiGetMaterialFloatArray(
                self.material_ptr,
                c_key.as_ptr(),
                0, // type
                0, // index
                &mut value,
                &mut max,
            )
        };

        if result == sys::aiReturn::aiReturn_SUCCESS && max > 0 {
            Some(value)
        } else {
            None
        }
    }

    /// Get an integer property from the material
    pub fn get_integer_property(&self, key: &str) -> Option<i32> {
        let c_key = CString::new(key).ok()?;
        let mut value = 0i32;
        let mut max = 1u32;

        let result = unsafe {
            sys::aiGetMaterialIntegerArray(
                self.material_ptr,
                c_key.as_ptr(),
                0, // type
                0, // index
                &mut value,
                &mut max,
            )
        };

        if result == sys::aiReturn::aiReturn_SUCCESS && max > 0 {
            Some(value)
        } else {
            None
        }
    }

    /// Get a color property from the material
    pub fn get_color_property(&self, key: &str) -> Option<Color4D> {
        let c_key = CString::new(key).ok()?;
        let mut color = sys::aiColor4D {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };

        let result = unsafe {
            sys::aiGetMaterialColor(
                self.material_ptr,
                c_key.as_ptr(),
                0, // type
                0, // index
                &mut color,
            )
        };

        if result == sys::aiReturn::aiReturn_SUCCESS {
            Some(Color4D::new(color.r, color.g, color.b, color.a))
        } else {
            None
        }
    }

    /// Get the diffuse color
    pub fn diffuse_color(&self) -> Option<Color3D> {
        self.get_color_property(material_keys::COLOR_DIFFUSE)
            .map(|c| Color3D::new(c.x, c.y, c.z))
    }

    /// Get the specular color
    pub fn specular_color(&self) -> Option<Color3D> {
        self.get_color_property(material_keys::COLOR_SPECULAR)
            .map(|c| Color3D::new(c.x, c.y, c.z))
    }

    /// Get the ambient color
    pub fn ambient_color(&self) -> Option<Color3D> {
        self.get_color_property(material_keys::COLOR_AMBIENT)
            .map(|c| Color3D::new(c.x, c.y, c.z))
    }

    /// Get the emissive color
    pub fn emissive_color(&self) -> Option<Color3D> {
        self.get_color_property(material_keys::COLOR_EMISSIVE)
            .map(|c| Color3D::new(c.x, c.y, c.z))
    }

    /// Get the transparent color
    pub fn transparent_color(&self) -> Option<Color3D> {
        self.get_color_property(material_keys::COLOR_TRANSPARENT)
            .map(|c| Color3D::new(c.x, c.y, c.z))
    }

    /// Get the reflective color
    pub fn reflective_color(&self) -> Option<Color3D> {
        self.get_color_property(material_keys::COLOR_REFLECTIVE)
            .map(|c| Color3D::new(c.x, c.y, c.z))
    }

    /// Get the shininess factor
    pub fn shininess(&self) -> Option<f32> {
        self.get_float_property(material_keys::SHININESS)
    }

    /// Get the shininess strength
    pub fn shininess_strength(&self) -> Option<f32> {
        self.get_float_property(material_keys::SHININESS_STRENGTH)
    }

    /// Get the opacity factor
    pub fn opacity(&self) -> Option<f32> {
        self.get_float_property(material_keys::OPACITY)
    }

    /// Get the transparency factor
    pub fn transparency_factor(&self) -> Option<f32> {
        self.get_float_property(material_keys::TRANSPARENCYFACTOR)
    }

    /// Get the bump scaling factor
    pub fn bump_scaling(&self) -> Option<f32> {
        self.get_float_property(material_keys::BUMPSCALING)
    }

    /// Get the refraction index
    pub fn refraction_index(&self) -> Option<f32> {
        self.get_float_property(material_keys::REFRACTI)
    }

    /// Get the reflectivity factor
    pub fn reflectivity(&self) -> Option<f32> {
        self.get_float_property(material_keys::REFLECTIVITY)
    }

    /// Get the shading model
    pub fn shading_model(&self) -> Option<i32> {
        self.get_integer_property(material_keys::SHADING_MODEL)
    }

    /// Check if the material is two-sided
    pub fn is_two_sided(&self) -> bool {
        self.get_integer_property(material_keys::TWOSIDED)
            .map(|v| v != 0)
            .unwrap_or(false)
    }

    /// Get the number of textures for a specific type
    pub fn texture_count(&self, texture_type: TextureType) -> usize {
        unsafe { sys::aiGetMaterialTextureCount(self.material_ptr, texture_type as _) as usize }
    }

    /// Get texture information for a specific type and index
    pub fn texture(&self, texture_type: TextureType, index: usize) -> Option<TextureInfo> {
        if index >= self.texture_count(texture_type) {
            return None;
        }

        unsafe {
            let mut path = sys::aiString::default();
            let mut mapping = std::mem::MaybeUninit::uninit();
            let mut uv_index = std::mem::MaybeUninit::uninit();
            let mut blend = std::mem::MaybeUninit::uninit();
            let mut op = std::mem::MaybeUninit::uninit();
            let mut map_mode = [0u32; 3];

            let result = sys::aiGetMaterialTexture(
                self.material_ptr,
                texture_type as _,
                index as u32,
                &mut path,
                mapping.as_mut_ptr(),
                uv_index.as_mut_ptr(),
                blend.as_mut_ptr(),
                op.as_mut_ptr(),
                map_mode.as_mut_ptr() as *mut _,
                std::ptr::null_mut(),
            );

            if result == sys::aiReturn::aiReturn_SUCCESS {
                let path_str = std::ffi::CStr::from_ptr(path.data.as_ptr())
                    .to_string_lossy()
                    .into_owned();

                let mapping_val = mapping.assume_init();
                let uv_index_val = uv_index.assume_init();
                let blend_val = blend.assume_init();
                let op_val = op.assume_init();

                Some(TextureInfo {
                    path: path_str,
                    mapping: TextureMapping::from_raw(mapping_val),
                    uv_index: uv_index_val,
                    blend_factor: blend_val,
                    operation: TextureOperation::from_raw(op_val),
                    map_modes: [
                        TextureMapMode::from_raw(map_mode[0] as i32),
                        TextureMapMode::from_raw(map_mode[1] as i32),
                        TextureMapMode::from_raw(map_mode[2] as i32),
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
    /// Diffuse texture (base color)
    Diffuse = sys::aiTextureType::aiTextureType_DIFFUSE as u32,
    /// Specular texture (reflectivity)
    Specular = sys::aiTextureType::aiTextureType_SPECULAR as u32,
    /// Ambient texture (ambient lighting)
    Ambient = sys::aiTextureType::aiTextureType_AMBIENT as u32,
    /// Emissive texture (self-illumination)
    Emissive = sys::aiTextureType::aiTextureType_EMISSIVE as u32,
    /// Height texture (displacement mapping)
    Height = sys::aiTextureType::aiTextureType_HEIGHT as u32,
    /// Normal texture (surface detail)
    Normals = sys::aiTextureType::aiTextureType_NORMALS as u32,
    /// Shininess texture (specular power)
    Shininess = sys::aiTextureType::aiTextureType_SHININESS as u32,
    /// Opacity texture (transparency)
    Opacity = sys::aiTextureType::aiTextureType_OPACITY as u32,
    /// Displacement texture (geometry displacement)
    Displacement = sys::aiTextureType::aiTextureType_DISPLACEMENT as u32,
    /// Lightmap texture (pre-computed lighting)
    Lightmap = sys::aiTextureType::aiTextureType_LIGHTMAP as u32,
    /// Reflection texture (environment mapping)
    Reflection = sys::aiTextureType::aiTextureType_REFLECTION as u32,
    /// Base color texture (PBR albedo)
    BaseColor = sys::aiTextureType::aiTextureType_BASE_COLOR as u32,
    /// Normal camera texture (camera-space normals)
    NormalCamera = sys::aiTextureType::aiTextureType_NORMAL_CAMERA as u32,
    /// Emission color texture (PBR emission)
    EmissionColor = sys::aiTextureType::aiTextureType_EMISSION_COLOR as u32,
    /// Metalness texture (PBR metallic)
    Metalness = sys::aiTextureType::aiTextureType_METALNESS as u32,
    /// Diffuse roughness texture (PBR roughness)
    DiffuseRoughness = sys::aiTextureType::aiTextureType_DIFFUSE_ROUGHNESS as u32,
    /// Ambient occlusion texture (shadowing)
    AmbientOcclusion = sys::aiTextureType::aiTextureType_AMBIENT_OCCLUSION as u32,
}

/// Texture mapping modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureMapping {
    /// UV coordinate mapping
    UV,
    /// Spherical mapping
    Sphere,
    /// Cylindrical mapping
    Cylinder,
    /// Box mapping
    Box,
    /// Planar mapping
    Plane,
    /// Other mapping mode
    Other(u32),
}

impl TextureMapping {
    fn from_raw(value: i32) -> Self {
        let value_u32 = value as u32;
        match value_u32 {
            v if v == sys::aiTextureMapping::aiTextureMapping_UV as u32 => Self::UV,
            v if v == sys::aiTextureMapping::aiTextureMapping_SPHERE as u32 => Self::Sphere,
            v if v == sys::aiTextureMapping::aiTextureMapping_CYLINDER as u32 => Self::Cylinder,
            v if v == sys::aiTextureMapping::aiTextureMapping_BOX as u32 => Self::Box,
            v if v == sys::aiTextureMapping::aiTextureMapping_PLANE as u32 => Self::Plane,
            other => Self::Other(other),
        }
    }
}

/// Texture operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureOperation {
    /// Multiply operation
    Multiply,
    /// Add operation
    Add,
    /// Subtract operation
    Subtract,
    /// Divide operation
    Divide,
    /// Smooth add operation
    SmoothAdd,
    /// Signed add operation
    SignedAdd,
    /// Other operation
    Other(u32),
}

impl TextureOperation {
    fn from_raw(value: i32) -> Self {
        let value_u32 = value as u32;
        match value_u32 {
            v if v == sys::aiTextureOp::aiTextureOp_Multiply as u32 => Self::Multiply,
            v if v == sys::aiTextureOp::aiTextureOp_Add as u32 => Self::Add,
            v if v == sys::aiTextureOp::aiTextureOp_Subtract as u32 => Self::Subtract,
            v if v == sys::aiTextureOp::aiTextureOp_Divide as u32 => Self::Divide,
            v if v == sys::aiTextureOp::aiTextureOp_SmoothAdd as u32 => Self::SmoothAdd,
            v if v == sys::aiTextureOp::aiTextureOp_SignedAdd as u32 => Self::SignedAdd,
            other => Self::Other(other),
        }
    }
}

/// Texture mapping modes for UV coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureMapMode {
    /// Wrap texture coordinates
    Wrap,
    /// Clamp texture coordinates to edge
    Clamp,
    /// Mirror texture coordinates
    Mirror,
    /// Decal texture mode
    Decal,
    /// Other texture map mode
    Other(u32),
}

impl TextureMapMode {
    fn from_raw(value: i32) -> Self {
        let value_u32 = value as u32;
        match value_u32 {
            v if v == sys::aiTextureMapMode::aiTextureMapMode_Wrap as u32 => Self::Wrap,
            v if v == sys::aiTextureMapMode::aiTextureMapMode_Clamp as u32 => Self::Clamp,
            v if v == sys::aiTextureMapMode::aiTextureMapMode_Mirror as u32 => Self::Mirror,
            v if v == sys::aiTextureMapMode::aiTextureMapMode_Decal as u32 => Self::Decal,
            other => Self::Other(other),
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
