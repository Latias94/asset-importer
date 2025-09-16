//! Material representation and properties

use std::ffi::CString;

use crate::{
    sys,
    types::{Color3D, Color4D, Vector2D, Vector3D},
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

    // PBR-related keys (from material.h)
    /// Base color factor (RGBA)
    pub const BASE_COLOR: &str = "$clr.base";
    /// Metallic factor
    pub const METALLIC_FACTOR: &str = "$mat.metallicFactor";
    /// Roughness factor
    pub const ROUGHNESS_FACTOR: &str = "$mat.roughnessFactor";
    /// Specular factor
    pub const SPECULAR_FACTOR: &str = "$mat.specularFactor";
    /// Glossiness factor (spec/gloss workflow)
    pub const GLOSSINESS_FACTOR: &str = "$mat.glossinessFactor";
    /// Sheen color factor
    pub const SHEEN_COLOR_FACTOR: &str = "$clr.sheen.factor";
    /// Sheen roughness factor
    pub const SHEEN_ROUGHNESS_FACTOR: &str = "$mat.sheen.roughnessFactor";
    /// Clearcoat factor
    pub const CLEARCOAT_FACTOR: &str = "$mat.clearcoat.factor";
    /// Clearcoat roughness factor
    pub const CLEARCOAT_ROUGHNESS_FACTOR: &str = "$mat.clearcoat.roughnessFactor";
    /// Transmission factor
    pub const TRANSMISSION_FACTOR: &str = "$mat.transmission.factor";
    /// Volume thickness factor
    pub const VOLUME_THICKNESS_FACTOR: &str = "$mat.volume.thicknessFactor";
    /// Volume attenuation distance
    pub const VOLUME_ATTENUATION_DISTANCE: &str = "$mat.volume.attenuationDistance";
    /// Volume attenuation color
    pub const VOLUME_ATTENUATION_COLOR: &str = "$mat.volume.attenuationColor";
    /// Emissive intensity
    pub const EMISSIVE_INTENSITY: &str = "$mat.emissiveIntensity";
    /// Anisotropy factor
    pub const ANISOTROPY_FACTOR: &str = "$mat.anisotropyFactor";
    /// Anisotropy rotation
    pub const ANISOTROPY_ROTATION: &str = "$mat.anisotropyRotation";
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

    /// Base color factor (RGBA)
    pub fn base_color(&self) -> Option<Color4D> {
        self.get_color_property(material_keys::BASE_COLOR)
    }

    /// Metallic factor
    pub fn metallic_factor(&self) -> Option<f32> {
        self.get_float_property(material_keys::METALLIC_FACTOR)
    }

    /// Roughness factor
    pub fn roughness_factor(&self) -> Option<f32> {
        self.get_float_property(material_keys::ROUGHNESS_FACTOR)
    }

    /// Glossiness factor (spec/gloss workflow)
    pub fn glossiness_factor(&self) -> Option<f32> {
        self.get_float_property(material_keys::GLOSSINESS_FACTOR)
    }

    /// Specular factor
    pub fn specular_factor(&self) -> Option<f32> {
        self.get_float_property(material_keys::SPECULAR_FACTOR)
    }

    /// Sheen color factor
    pub fn sheen_color_factor(&self) -> Option<Color4D> {
        self.get_color_property(material_keys::SHEEN_COLOR_FACTOR)
    }

    /// Sheen roughness factor
    pub fn sheen_roughness_factor(&self) -> Option<f32> {
        self.get_float_property(material_keys::SHEEN_ROUGHNESS_FACTOR)
    }

    /// Clearcoat factor
    pub fn clearcoat_factor(&self) -> Option<f32> {
        self.get_float_property(material_keys::CLEARCOAT_FACTOR)
    }

    /// Clearcoat roughness factor
    pub fn clearcoat_roughness_factor(&self) -> Option<f32> {
        self.get_float_property(material_keys::CLEARCOAT_ROUGHNESS_FACTOR)
    }

    /// Transmission factor
    pub fn transmission_factor(&self) -> Option<f32> {
        self.get_float_property(material_keys::TRANSMISSION_FACTOR)
    }

    /// Volume thickness factor
    pub fn volume_thickness_factor(&self) -> Option<f32> {
        self.get_float_property(material_keys::VOLUME_THICKNESS_FACTOR)
    }

    /// Volume attenuation distance
    pub fn volume_attenuation_distance(&self) -> Option<f32> {
        self.get_float_property(material_keys::VOLUME_ATTENUATION_DISTANCE)
    }

    /// Volume attenuation color
    pub fn volume_attenuation_color(&self) -> Option<Color3D> {
        self.get_color_property(material_keys::VOLUME_ATTENUATION_COLOR)
            .map(|c| Color3D::new(c.x, c.y, c.z))
    }

    /// Emissive intensity
    pub fn emissive_intensity(&self) -> Option<f32> {
        self.get_float_property(material_keys::EMISSIVE_INTENSITY)
    }

    /// Anisotropy factor
    pub fn anisotropy_factor(&self) -> Option<f32> {
        self.get_float_property(material_keys::ANISOTROPY_FACTOR)
    }

    /// Anisotropy rotation
    pub fn anisotropy_rotation(&self) -> Option<f32> {
        self.get_float_property(material_keys::ANISOTROPY_ROTATION)
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

    /// Get the shading model as an enum
    pub fn shading_model_enum(&self) -> Option<ShadingModel> {
        self.shading_model()
            .map(|v| ShadingModel::from_raw(v as u32))
    }

    /// Check if the material is two-sided
    pub fn is_two_sided(&self) -> bool {
        self.get_integer_property(material_keys::TWOSIDED)
            .map(|v| v != 0)
            .unwrap_or(false)
    }

    /// Check if the material is unlit (NoShading/Unlit)
    pub fn is_unlit(&self) -> bool {
        match self.shading_model_enum() {
            Some(ShadingModel::NoShading) => true,
            _ => false,
        }
    }

    /// Get the blend mode for the material
    pub fn blend_mode(&self) -> Option<BlendMode> {
        self.get_integer_property(material_keys::BLEND_FUNC)
            .map(|v| BlendMode::from_raw(v as u32))
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
            let mut mapping = std::mem::MaybeUninit::<sys::aiTextureMapping::Type>::uninit();
            let mut uv_index = std::mem::MaybeUninit::<u32>::uninit();
            let mut blend = std::mem::MaybeUninit::<f32>::uninit();
            let mut op = std::mem::MaybeUninit::<sys::aiTextureOp::Type>::uninit();
            let mut map_mode = [0u32; 3];
            let mut tex_flags: u32 = 0;

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
                &mut tex_flags as *mut u32,
            );

            if result == sys::aiReturn::aiReturn_SUCCESS {
                let path_str = std::ffi::CStr::from_ptr(path.data.as_ptr())
                    .to_string_lossy()
                    .into_owned();

                let mapping_val = mapping.assume_init();
                let uv_index_val = uv_index.assume_init();
                let blend_val = blend.assume_init();
                let op_val = op.assume_init();

                // Try read UV transform
                let mut uv_transform = std::mem::MaybeUninit::<sys::aiUVTransform>::uninit();
                let uv_key = std::ffi::CString::new("$tex.uvtrafo").unwrap();
                let uv_ok = sys::aiGetMaterialUVTransform(
                    self.material_ptr,
                    uv_key.as_ptr(),
                    texture_type as u32,
                    index as u32,
                    uv_transform.as_mut_ptr(),
                ) == sys::aiReturn::aiReturn_SUCCESS;

                let uv_transform = if uv_ok {
                    let t = uv_transform.assume_init();
                    Some(UVTransform {
                        translation: Vector2D::new(t.mTranslation.x, t.mTranslation.y),
                        scaling: Vector2D::new(t.mScaling.x, t.mScaling.y),
                        rotation: t.mRotation,
                    })
                } else {
                    None
                };

                // Try read TEXMAP_AXIS via property API ("$tex.mapaxis")
                let axis = {
                    let key = std::ffi::CString::new("$tex.mapaxis").unwrap();
                    let mut prop_ptr: *const sys::aiMaterialProperty = std::ptr::null();
                    let ok = sys::aiGetMaterialProperty(
                        self.material_ptr,
                        key.as_ptr(),
                        texture_type as u32,
                        index as u32,
                        &mut prop_ptr,
                    ) == sys::aiReturn::aiReturn_SUCCESS;
                    if ok && !prop_ptr.is_null() {
                        let prop = &*prop_ptr;
                        if prop.mData.is_null()
                            || prop.mDataLength < std::mem::size_of::<sys::aiVector3D>() as u32
                        {
                            None
                        } else {
                            let v = *(prop.mData as *const sys::aiVector3D);
                            Some(Vector3D::new(v.x, v.y, v.z))
                        }
                    } else {
                        None
                    }
                };

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
                    flags: TextureFlags::from_bits_truncate(tex_flags),
                    uv_transform,
                    axis,
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
    /// Unknown texture type
    Unknown = sys::aiTextureType::aiTextureType_UNKNOWN as u32,
    /// Sheen layer (PBR)
    Sheen = sys::aiTextureType::aiTextureType_SHEEN as u32,
    /// Clearcoat layer (PBR)
    Clearcoat = sys::aiTextureType::aiTextureType_CLEARCOAT as u32,
    /// Transmission layer (PBR)
    Transmission = sys::aiTextureType::aiTextureType_TRANSMISSION as u32,
    /// Maya base color (compat)
    MayaBase = sys::aiTextureType::aiTextureType_MAYA_BASE as u32,
    /// Maya specular (compat)
    MayaSpecular = sys::aiTextureType::aiTextureType_MAYA_SPECULAR as u32,
    /// Maya specular color (compat)
    MayaSpecularColor = sys::aiTextureType::aiTextureType_MAYA_SPECULAR_COLOR as u32,
    /// Maya specular roughness (compat)
    MayaSpecularRoughness = sys::aiTextureType::aiTextureType_MAYA_SPECULAR_ROUGHNESS as u32,
    /// Anisotropy (PBR)
    Anisotropy = sys::aiTextureType::aiTextureType_ANISOTROPY as u32,
    /// glTF metallic-roughness packed
    GltfMetallicRoughness = sys::aiTextureType::aiTextureType_GLTF_METALLIC_ROUGHNESS as u32,
}

/// High-level shading model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadingModel {
    Flat,
    Gouraud,
    Phong,
    Blinn,
    Toon,
    OrenNayar,
    Minnaert,
    CookTorrance,
    NoShading,
    Fresnel,
    PbrSpecularGlossiness,
    PbrMetallicRoughness,
    Unknown(u32),
}

impl ShadingModel {
    fn from_raw(v: u32) -> Self {
        use sys::aiShadingMode;
        match v {
            x if x == aiShadingMode::aiShadingMode_Flat as u32 => ShadingModel::Flat,
            x if x == aiShadingMode::aiShadingMode_Gouraud as u32 => ShadingModel::Gouraud,
            x if x == aiShadingMode::aiShadingMode_Phong as u32 => ShadingModel::Phong,
            x if x == aiShadingMode::aiShadingMode_Blinn as u32 => ShadingModel::Blinn,
            x if x == aiShadingMode::aiShadingMode_Toon as u32 => ShadingModel::Toon,
            x if x == aiShadingMode::aiShadingMode_OrenNayar as u32 => ShadingModel::OrenNayar,
            x if x == aiShadingMode::aiShadingMode_Minnaert as u32 => ShadingModel::Minnaert,
            x if x == aiShadingMode::aiShadingMode_CookTorrance as u32 => {
                ShadingModel::CookTorrance
            }
            x if x == aiShadingMode::aiShadingMode_NoShading as u32 => ShadingModel::NoShading,
            x if x == aiShadingMode::aiShadingMode_Fresnel as u32 => ShadingModel::Fresnel,
            x if x == aiShadingMode::aiShadingMode_PBR_BRDF as u32 => {
                ShadingModel::PbrSpecularGlossiness
            }
            other => ShadingModel::Unknown(other),
        }
    }
}

/// Blend mode for material layers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Default,
    Additive,
    Unknown(u32),
}

impl BlendMode {
    fn from_raw(v: u32) -> Self {
        match v {
            x if x == sys::aiBlendMode::aiBlendMode_Default as u32 => BlendMode::Default,
            x if x == sys::aiBlendMode::aiBlendMode_Additive as u32 => BlendMode::Additive,
            other => BlendMode::Unknown(other),
        }
    }
}

/// Which PBR workflow this material uses (heuristic from material.h docs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PbrWorkflow {
    MetallicRoughness,
    SpecularGlossiness,
    Unknown,
}

impl Material {
    /// Determine PBR workflow based on present factors
    pub fn pbr_workflow(&self) -> PbrWorkflow {
        if self.metallic_factor().is_some() || self.roughness_factor().is_some() {
            PbrWorkflow::MetallicRoughness
        } else if self.glossiness_factor().is_some() || self.specular_factor().is_some() {
            PbrWorkflow::SpecularGlossiness
        } else {
            PbrWorkflow::Unknown
        }
    }

    // ---------- Convenience texture getters ----------
    pub fn base_color_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::BaseColor, index)
    }

    pub fn metallic_roughness_texture(&self) -> Option<TextureInfo> {
        // glTF packed metallic-roughness (one texture, index 0)
        self.texture(TextureType::GltfMetallicRoughness, 0)
    }

    pub fn emission_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::EmissionColor, index)
    }

    pub fn normal_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::Normals, index)
    }

    pub fn sheen_color_texture(&self) -> Option<TextureInfo> {
        // sheen color texture is TextureType::Sheen, index 0
        self.texture(TextureType::Sheen, 0)
    }

    pub fn sheen_roughness_texture(&self) -> Option<TextureInfo> {
        // sheen roughness texture is TextureType::Sheen, index 1
        self.texture(TextureType::Sheen, 1)
    }

    pub fn clearcoat_texture(&self) -> Option<TextureInfo> {
        self.texture(TextureType::Clearcoat, 0)
    }

    pub fn clearcoat_roughness_texture(&self) -> Option<TextureInfo> {
        self.texture(TextureType::Clearcoat, 1)
    }

    pub fn clearcoat_normal_texture(&self) -> Option<TextureInfo> {
        self.texture(TextureType::Clearcoat, 2)
    }

    pub fn transmission_texture(&self) -> Option<TextureInfo> {
        self.texture(TextureType::Transmission, 0)
    }

    pub fn volume_thickness_texture(&self) -> Option<TextureInfo> {
        // Defined to use aiTextureType_TRANSMISSION, index 1
        self.texture(TextureType::Transmission, 1)
    }

    pub fn anisotropy_texture(&self) -> Option<TextureInfo> {
        self.texture(TextureType::Anisotropy, 0)
    }

    /// Albedo texture (alias of BaseColor)
    pub fn albedo_texture(&self, index: usize) -> Option<TextureInfo> {
        self.base_color_texture(index)
    }

    /// Metallic texture (separate channel, not glTF packed)
    pub fn metallic_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::Metalness, index)
    }

    /// Roughness texture (separate channel, not glTF packed)
    pub fn roughness_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::DiffuseRoughness, index)
    }

    /// Ambient occlusion texture
    pub fn ambient_occlusion_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::AmbientOcclusion, index)
    }

    /// Lightmap texture
    pub fn lightmap_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::Lightmap, index)
    }

    /// Displacement texture
    pub fn displacement_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::Displacement, index)
    }

    /// Reflection/environment texture
    pub fn reflection_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::Reflection, index)
    }

    /// Opacity texture
    pub fn opacity_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::Opacity, index)
    }

    /// Height map texture (some formats use this for parallax/bump)
    pub fn height_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::Height, index)
    }

    /// Specular map (spec/gloss workflow)
    pub fn specular_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::Specular, index)
    }

    /// Glossiness map (spec/gloss workflow)
    pub fn glossiness_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::Shininess, index)
    }

    /// Emissive map (PBR emission color)
    pub fn emissive_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::EmissionColor, index)
    }
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
    /// Texture flags
    pub flags: TextureFlags,
    /// Optional UV transform
    pub uv_transform: Option<UVTransform>,
    /// Optional texture mapping axis
    pub axis: Option<Vector3D>,
}

/// UV transform information
#[derive(Debug, Clone, Copy)]
pub struct UVTransform {
    pub translation: Vector2D,
    pub scaling: Vector2D,
    pub rotation: f32,
}

bitflags::bitflags! {
    /// Texture flags (material.h: aiTextureFlags)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TextureFlags: u32 {
        const INVERT        = sys::aiTextureFlags::aiTextureFlags_Invert as u32;
        const USE_ALPHA     = sys::aiTextureFlags::aiTextureFlags_UseAlpha as u32;
        const IGNORE_ALPHA  = sys::aiTextureFlags::aiTextureFlags_IgnoreAlpha as u32;
    }
}
