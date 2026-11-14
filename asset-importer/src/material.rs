//! Material representation and properties

#![allow(clippy::unnecessary_cast)]

use crate::{
    error::{Error, Result},
    ffi,
    ptr::SharedPtr,
    scene::Scene,
    sys,
    types::{
        Color3D, Color4D, Vector2D, Vector3D, Vector4D, ai_string_to_str, ai_string_to_string,
    },
};
use std::borrow::Cow;
use std::ffi::CStr;
use std::ffi::CString;

/// Standard material property keys as defined by Assimp
pub mod material_keys {
    use std::ffi::CStr;

    macro_rules! cstr {
        ($lit:literal) => {
            unsafe { CStr::from_bytes_with_nul_unchecked(concat!($lit, "\0").as_bytes()) }
        };
    }

    /// Material name
    pub const NAME: &CStr = cstr!("?mat.name");
    /// Diffuse color
    pub const COLOR_DIFFUSE: &CStr = cstr!("$clr.diffuse");
    /// Ambient color
    pub const COLOR_AMBIENT: &CStr = cstr!("$clr.ambient");
    /// Specular color
    pub const COLOR_SPECULAR: &CStr = cstr!("$clr.specular");
    /// Emissive color
    pub const COLOR_EMISSIVE: &CStr = cstr!("$clr.emissive");
    /// Transparent color
    pub const COLOR_TRANSPARENT: &CStr = cstr!("$clr.transparent");
    /// Reflective color
    pub const COLOR_REFLECTIVE: &CStr = cstr!("$clr.reflective");
    /// Shininess factor
    pub const SHININESS: &CStr = cstr!("$mat.shininess");
    /// Shininess strength
    pub const SHININESS_STRENGTH: &CStr = cstr!("$mat.shinpercent");
    /// Opacity
    pub const OPACITY: &CStr = cstr!("$mat.opacity");
    /// Transparency factor
    pub const TRANSPARENCYFACTOR: &CStr = cstr!("$mat.transparencyfactor");
    /// Bump scaling
    pub const BUMPSCALING: &CStr = cstr!("$mat.bumpscaling");
    /// Refraction index
    pub const REFRACTI: &CStr = cstr!("$mat.refracti");
    /// Reflectivity
    pub const REFLECTIVITY: &CStr = cstr!("$mat.reflectivity");
    /// Shading model
    pub const SHADING_MODEL: &CStr = cstr!("$mat.shadingm");
    /// Blend function
    pub const BLEND_FUNC: &CStr = cstr!("$mat.blend");
    /// Two sided
    pub const TWOSIDED: &CStr = cstr!("$mat.twosided");
    /// glTF texture scale key, used by normal textures.
    pub const TEXTURE_SCALE: &CStr = cstr!("$tex.scale");
    /// glTF texture strength key, used by occlusion textures.
    pub const TEXTURE_STRENGTH: &CStr = cstr!("$tex.strength");

    // PBR-related keys (from material.h)
    /// Base color factor (RGBA)
    pub const BASE_COLOR: &CStr = cstr!("$clr.base");
    /// Metallic factor
    pub const METALLIC_FACTOR: &CStr = cstr!("$mat.metallicFactor");
    /// Roughness factor
    pub const ROUGHNESS_FACTOR: &CStr = cstr!("$mat.roughnessFactor");
    /// Specular factor
    pub const SPECULAR_FACTOR: &CStr = cstr!("$mat.specularFactor");
    /// Glossiness factor (spec/gloss workflow)
    pub const GLOSSINESS_FACTOR: &CStr = cstr!("$mat.glossinessFactor");
    /// Sheen color factor
    pub const SHEEN_COLOR_FACTOR: &CStr = cstr!("$clr.sheen.factor");
    /// Sheen roughness factor
    pub const SHEEN_ROUGHNESS_FACTOR: &CStr = cstr!("$mat.sheen.roughnessFactor");
    /// Clearcoat factor
    pub const CLEARCOAT_FACTOR: &CStr = cstr!("$mat.clearcoat.factor");
    /// Clearcoat roughness factor
    pub const CLEARCOAT_ROUGHNESS_FACTOR: &CStr = cstr!("$mat.clearcoat.roughnessFactor");
    /// Transmission factor
    pub const TRANSMISSION_FACTOR: &CStr = cstr!("$mat.transmission.factor");
    /// Volume thickness factor
    pub const VOLUME_THICKNESS_FACTOR: &CStr = cstr!("$mat.volume.thicknessFactor");
    /// Volume attenuation distance
    pub const VOLUME_ATTENUATION_DISTANCE: &CStr = cstr!("$mat.volume.attenuationDistance");
    /// Volume attenuation color
    pub const VOLUME_ATTENUATION_COLOR: &CStr = cstr!("$mat.volume.attenuationColor");
    /// Emissive intensity
    pub const EMISSIVE_INTENSITY: &CStr = cstr!("$mat.emissiveIntensity");
    /// Anisotropy factor
    pub const ANISOTROPY_FACTOR: &CStr = cstr!("$mat.anisotropyFactor");
    /// Anisotropy rotation
    pub const ANISOTROPY_ROTATION: &CStr = cstr!("$mat.anisotropyRotation");
}

/// A material containing properties like colors, textures, and shading parameters
#[derive(Clone)]
pub struct Material {
    scene: Scene,
    material_ptr: SharedPtr<sys::aiMaterial>,
}

/// A borrowed-ish string result backed by an owned `aiString` (no heap allocation).
#[derive(Debug, Clone)]
pub struct MaterialStringRef {
    value: sys::aiString,
}

impl MaterialStringRef {
    /// Access as UTF-8 (lossy) without allocation.
    pub fn as_str(&self) -> Cow<'_, str> {
        ai_string_to_str(&self.value)
    }

    /// Raw bytes (without assuming NUL-termination).
    pub fn as_bytes(&self) -> &[u8] {
        let len = (self.value.length as usize).min(self.value.data.len());
        unsafe { ffi::slice_from_ptr_len(self, self.value.data.as_ptr() as *const u8, len) }
    }

    /// Borrow the underlying Assimp `aiString`.
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> &sys::aiString {
        &self.value
    }

    /// Convert to an owned `String` (allocates).
    pub fn to_string_lossy(&self) -> String {
        ai_string_to_string(&self.value)
    }
}

impl std::fmt::Display for MaterialStringRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&ai_string_to_string(&self.value))
    }
}

impl Material {
    /// # Safety
    /// A non-null, aligned pointer and its reachable data must remain valid and immutable
    /// while any clone of the resulting view exists. `scene` must retain their owner.
    pub(crate) unsafe fn from_sys_ptr(
        scene: Scene,
        material_ptr: *mut sys::aiMaterial,
    ) -> Option<Self> {
        let material_ptr = unsafe { SharedPtr::new(material_ptr as *const sys::aiMaterial) }?;
        Some(Self {
            scene,
            material_ptr,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw_sys(&self) -> *const sys::aiMaterial {
        self.material_ptr.as_ptr()
    }

    /// Get the raw material pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> *const sys::aiMaterial {
        self.as_raw_sys()
    }

    #[inline]
    fn raw(&self) -> &sys::aiMaterial {
        self.material_ptr.as_ref()
    }

    /// Get the name of the material
    pub fn name(&self) -> String {
        self.name_ref().map(|s| s.to_string()).unwrap_or_default()
    }

    /// Get the material name (no heap allocation).
    pub fn name_ref(&self) -> Option<MaterialStringRef> {
        self.get_string_property_ref(material_keys::NAME)
    }

    /// Get a string property from the material (no heap allocation).
    pub fn get_string_property_ref(&self, key: &CStr) -> Option<MaterialStringRef> {
        self.property_data(key, None, 0)?.string_ref()
    }

    /// Get a string property from the material (allocates).
    pub fn get_string_property(&self, key: &CStr) -> Option<String> {
        self.get_string_property_ref(key).map(|s| s.to_string())
    }

    /// Get a string property from the material (allocates, convenience).
    pub fn get_string_property_str(&self, key: &str) -> Result<Option<String>> {
        let c_key = CString::new(key)
            .map_err(|_| Error::invalid_parameter("material key contains NUL byte".to_string()))?;
        Ok(self.get_string_property(c_key.as_c_str()))
    }

    /// Get the first numeric property element as a float.
    ///
    /// Buffers contain native-endian `f32` values; integers and doubles are converted.
    /// Numeric strings are parsed within their validated payload. Malformed data returns `None`.
    pub fn get_float_property(&self, key: &CStr) -> Option<f32> {
        Some(self.property_data(key, None, 0)?.read_f64(0)? as f32)
    }

    /// Get a float property from the material (allocates, convenience).
    pub fn get_float_property_str(&self, key: &str) -> Result<Option<f32>> {
        let c_key = CString::new(key)
            .map_err(|_| Error::invalid_parameter("material key contains NUL byte".to_string()))?;
        Ok(self.get_float_property(c_key.as_c_str()))
    }

    /// Get the first numeric property element as an integer.
    ///
    /// Floating-point values are truncated toward zero. Non-finite values, results outside
    /// the `i32` range, malformed payloads, and non-numeric property types return `None`.
    /// Buffers contain native-endian integers; strings must contain integer tokens.
    pub fn get_integer_property(&self, key: &CStr) -> Option<i32> {
        self.property_data(key, None, 0)?.read_i32(0)
    }

    /// Get an integer property from the material (allocates, convenience).
    pub fn get_integer_property_str(&self, key: &str) -> Result<Option<i32>> {
        let c_key = CString::new(key)
            .map_err(|_| Error::invalid_parameter("material key contains NUL byte".to_string()))?;
        Ok(self.get_integer_property(c_key.as_c_str()))
    }

    /// Get three or four numeric components as a color, defaulting RGB alpha to one.
    ///
    /// Fewer than three components or malformed data return `None`.
    pub fn get_color_property(&self, key: &CStr) -> Option<Color4D> {
        let data = self.property_data(key, None, 0)?;
        let [red, green, blue] = data.read_f32_array::<3>()?;
        let alpha = if data.float_count()? == 3 {
            1.0
        } else {
            data.read_f64(3)? as f32
        };
        Some(Color4D::new(red, green, blue, alpha))
    }

    /// Get a color property from the material (allocates, convenience).
    pub fn get_color_property_str(&self, key: &str) -> Result<Option<Color4D>> {
        let c_key = CString::new(key)
            .map_err(|_| Error::invalid_parameter("material key contains NUL byte".to_string()))?;
        Ok(self.get_color_property(c_key.as_c_str()))
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

    /// Get raw information about a material property by key/semantic/index
    ///
    /// - `key`: material key string (e.g. "$mat.shininess")
    /// - `semantic`: texture type semantic if the property is texture-dependent; use `None` for non-texture properties
    /// - `index`: texture index for texture-dependent properties; 0 otherwise
    pub fn property_info(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<MaterialPropertyInfo> {
        let prop_ptr = self.property_ptr(key, semantic, index)?;
        unsafe { MaterialPropertyRef::from_ptr(self.scene.clone(), prop_ptr) }
            .map(MaterialPropertyRef::into_info)
    }

    /// Get raw information about a material property by key/semantic/index (allocates, convenience).
    pub fn property_info_str(
        &self,
        key: &str,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<MaterialPropertyInfo> {
        let c_key = CString::new(key).ok()?;
        self.property_info(c_key.as_c_str(), semantic, index)
    }

    /// Get only the property type information (aiPropertyTypeInfo) for a given key/semantic/index
    pub fn property_type(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<PropertyTypeInfo> {
        self.property_info(key, semantic, index)
            .map(|p| p.type_info)
    }

    /// Get only the property type information (aiPropertyTypeInfo) for a given key/semantic/index (allocates, convenience).
    pub fn property_type_str(
        &self,
        key: &str,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<PropertyTypeInfo> {
        let c_key = CString::new(key).ok()?;
        self.property_type(c_key.as_c_str(), semantic, index)
    }

    fn property_ptr(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<*const sys::aiMaterialProperty> {
        let mut prop_ptr: *const sys::aiMaterialProperty = std::ptr::null();
        let ok = unsafe {
            sys::aiGetMaterialProperty(
                self.as_raw_sys(),
                key.as_ptr(),
                semantic.map(TextureType::to_semantic).unwrap_or(0),
                index,
                &mut prop_ptr,
            ) == sys::aiReturn::aiReturn_SUCCESS
        };
        (ok && !prop_ptr.is_null()).then_some(prop_ptr)
    }

    fn property_data(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<MaterialPropertyData<'_>> {
        let prop = unsafe { ffi::ref_from_ptr(self, self.property_ptr(key, semantic, index)?) }?;
        unsafe { MaterialPropertyData::from_sys(prop) }
    }

    /// Get the raw bytes of a property (as stored by Assimp)
    pub fn get_property_raw_ref(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<&[u8]> {
        Some(self.property_data(key, semantic, index)?.bytes)
    }

    /// Get the raw bytes of a property (as stored by Assimp, allocates).
    pub fn get_property_raw(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<Vec<u8>> {
        self.get_property_raw_ref(key, semantic, index)
            .map(|raw| raw.to_vec())
    }

    /// Get the raw bytes of a property (as stored by Assimp, allocates, convenience).
    pub fn get_property_raw_str(
        &self,
        key: &str,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<Vec<u8>> {
        let c_key = CString::new(key).ok()?;
        self.get_property_raw(c_key.as_c_str(), semantic, index)
    }

    /// Get a numeric property as an integer array by decoding its raw bytes.
    ///
    /// Floating-point values are truncated toward zero. The entire query returns `None`
    /// if any result is non-finite or outside the `i32` range, or the payload is malformed.
    /// A one-byte integer payload follows Assimp's native `char` compatibility rule.
    /// Buffers use the same integer layout; strings are parsed as whitespace-separated integers.
    /// Unknown property types are not converted.
    pub fn get_property_i32_array(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<Vec<i32>> {
        let data = self.property_data(key, semantic, index)?;
        (0..data.i32_count()?)
            .map(|index| data.read_i32(index))
            .collect()
    }

    /// Get an integer array property (converts from floats if necessary, allocates, convenience).
    pub fn get_property_i32_array_str(
        &self,
        key: &str,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<Vec<i32>> {
        let c_key = CString::new(key).ok()?;
        self.get_property_i32_array(c_key.as_c_str(), semantic, index)
    }

    /// Get a 32-bit float array property by bounded numeric conversion.
    ///
    /// Buffers contain native-endian `f32` values. Integer and double values are converted,
    /// and strings are parsed as whitespace-separated numbers. Malformed data returns `None`.
    pub fn get_property_f32_array(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<Vec<f32>> {
        let data = self.property_data(key, semantic, index)?;
        (0..data.float_count()?)
            .map(|index| data.read_f64(index).map(|value| value as f32))
            .collect()
    }

    /// Get a 32-bit float array property (allocates, convenience).
    pub fn get_property_f32_array_str(
        &self,
        key: &str,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<Vec<f32>> {
        let c_key = CString::new(key).ok()?;
        self.get_property_f32_array(c_key.as_c_str(), semantic, index)
    }

    /// Get a 64-bit float array property by decoding raw bytes.
    ///
    /// Float and buffer values are widened from `f32`; integers are converted from `i32`.
    /// Strings are parsed as whitespace-separated numbers. Malformed data returns `None`.
    pub fn get_property_f64_array(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<Vec<f64>> {
        let data = self.property_data(key, semantic, index)?;
        (0..data.float_count()?)
            .map(|index| data.read_f64(index))
            .collect()
    }

    /// Get a 64-bit float array property (allocates, convenience).
    pub fn get_property_f64_array_str(
        &self,
        key: &str,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<Vec<f64>> {
        let c_key = CString::new(key).ok()?;
        self.get_property_f64_array(c_key.as_c_str(), semantic, index)
    }

    /// Enumerate all properties stored in this material (raw info only)
    pub fn all_properties(&self) -> Vec<MaterialPropertyInfo> {
        self.all_properties_iter().collect()
    }

    /// Iterate properties as owned `MaterialPropertyInfo` (allocates per item, but avoids a `Vec`).
    pub fn all_properties_iter(&self) -> impl Iterator<Item = MaterialPropertyInfo> + '_ {
        self.properties().map(MaterialPropertyRef::into_info)
    }

    /// Iterate all material properties (zero allocation for keys and raw data).
    pub fn properties(&self) -> MaterialPropertyIterator {
        let m = self.raw();
        MaterialPropertyIterator {
            scene: self.scene.clone(),
            props: unsafe {
                SharedPtr::new(m.mProperties as *const *const sys::aiMaterialProperty)
            },
            count: m.mNumProperties as usize,
            index: 0,
        }
    }

    /// Check if the material is two-sided
    pub fn is_two_sided(&self) -> bool {
        self.get_integer_property(material_keys::TWOSIDED)
            .map(|v| v != 0)
            .unwrap_or(false)
    }

    /// Check if the material is unlit (NoShading/Unlit)
    pub fn is_unlit(&self) -> bool {
        matches!(self.shading_model_enum(), Some(ShadingModel::NoShading))
    }

    /// Get the blend mode for the material
    pub fn blend_mode(&self) -> Option<BlendMode> {
        self.get_integer_property(material_keys::BLEND_FUNC)
            .map(|v| BlendMode::from_raw(v as u32))
    }

    /// Get a texture-scoped float property from the material.
    ///
    /// This is useful for texture extension metadata such as glTF normal texture scale and
    /// occlusion texture strength.
    pub fn get_texture_float_property(
        &self,
        key: &CStr,
        texture_type: TextureType,
        index: usize,
    ) -> Option<f32> {
        let index = u32::try_from(index).ok()?;
        Some(
            self.property_data(key, Some(texture_type), index)?
                .read_f64(0)? as f32,
        )
    }

    /// Get the glTF texture scale value for a texture slot.
    ///
    /// Assimp stores this as `AI_MATKEY_GLTF_TEXTURE_SCALE(type, index)`. In glTF this is mainly
    /// used by `normalTexture.scale`.
    pub fn texture_scale(&self, texture_type: TextureType, index: usize) -> Option<f32> {
        self.get_texture_float_property(material_keys::TEXTURE_SCALE, texture_type, index)
    }

    /// Get the glTF normal texture scale for a normal texture slot.
    pub fn normal_texture_scale(&self, index: usize) -> Option<f32> {
        self.texture_scale(TextureType::Normals, index)
    }

    /// Get the glTF texture strength value for a texture slot.
    ///
    /// Assimp stores this as `AI_MATKEY_GLTF_TEXTURE_STRENGTH(type, index)`. In glTF this is mainly
    /// used by `occlusionTexture.strength`.
    pub fn texture_strength(&self, texture_type: TextureType, index: usize) -> Option<f32> {
        self.get_texture_float_property(material_keys::TEXTURE_STRENGTH, texture_type, index)
    }

    /// Get the glTF occlusion texture strength.
    ///
    /// Assimp imports glTF `occlusionTexture` into `aiTextureType_LIGHTMAP`.
    pub fn occlusion_texture_strength(&self, index: usize) -> Option<f32> {
        self.texture_strength(TextureType::Lightmap, index)
    }

    /// Get the number of textures for a specific type
    pub fn texture_count(&self, texture_type: TextureType) -> usize {
        unsafe { sys::aiGetMaterialTextureCount(self.as_raw_sys(), texture_type.to_sys()) as usize }
    }

    /// Get texture information for a specific type and index (no heap allocation).
    ///
    /// Missing optional properties use UV mapping, channel 0, blend 1, Multiply, Wrap,
    /// and no flags. Malformed optional properties use the same defaults.
    /// Non-UV mappings keep the default UV channel. UV transforms require five values.
    pub fn texture_ref(&self, texture_type: TextureType, index: usize) -> Option<TextureInfoRef> {
        let index = u32::try_from(index).ok()?;
        if index as usize >= self.texture_count(texture_type) {
            return None;
        }

        let property = |key: &CStr| self.property_data(key, Some(texture_type), index);
        let integer = |key: &CStr| property(key)?.read_i32(0);
        let path = property(c"$tex.file")?.string_ref()?.value;
        let mapping = TextureMapping::from_raw(
            integer(c"$tex.mapping")
                .map(|value| value as u32)
                .unwrap_or(sys::aiTextureMapping::aiTextureMapping_UV.0 as u32),
        );
        let uv_index = if mapping == TextureMapping::UV {
            integer(c"$tex.uvwsrc").unwrap_or(0) as u32
        } else {
            0
        };
        let blend_factor = property(c"$tex.blend")
            .and_then(|data| data.read_f64(0))
            .map(|value| value as f32)
            .unwrap_or(1.0);
        let operation = TextureOperation::from_raw(
            integer(c"$tex.op")
                .map(|value| value as u32)
                .unwrap_or(sys::aiTextureOp::aiTextureOp_Multiply.0 as u32),
        );
        let [map_mode_u, map_mode_v] = [c"$tex.mapmodeu", c"$tex.mapmodev"].map(|key| {
            TextureMapMode::from_raw(
                integer(key)
                    .map(|value| value as u32)
                    .unwrap_or(sys::aiTextureMapMode::aiTextureMapMode_Wrap.0 as u32),
            )
        });
        let flags = TextureFlags::from_bits_truncate(integer(c"$tex.flags").unwrap_or(0) as u32);
        let uv_transform = property(c"$tex.uvtrafo")
            .and_then(|data| data.read_f32_array::<5>())
            .map(|[tx, ty, sx, sy, rotation]| UVTransform {
                translation: Vector2D::new(tx, ty),
                scaling: Vector2D::new(sx, sy),
                rotation,
            });
        let axis = property(c"$tex.mapaxis")
            .and_then(|data| data.read_f32_array::<3>())
            .map(|[axis_x, axis_y, axis_z]| Vector3D::new(axis_x, axis_y, axis_z));

        Some(TextureInfoRef {
            path,
            mapping,
            uv_index,
            blend_factor,
            operation,
            map_modes: [map_mode_u, map_mode_v, TextureMapMode::Wrap],
            flags,
            uv_transform,
            axis,
        })
    }

    /// Iterate textures of a given type (no heap allocation).
    pub fn texture_refs(
        &self,
        texture_type: TextureType,
    ) -> impl Iterator<Item = TextureInfoRef> + '_ {
        let count = self.texture_count(texture_type);
        (0..count).filter_map(move |i| self.texture_ref(texture_type, i))
    }

    /// Get texture information for a specific type and index
    pub fn texture(&self, texture_type: TextureType, index: usize) -> Option<TextureInfo> {
        self.texture_ref(texture_type, index)
            .map(TextureInfoRef::into_owned)
    }
}

/// Types of textures that can be applied to materials
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureType {
    /// Diffuse texture (base color)
    Diffuse = sys::aiTextureType::aiTextureType_DIFFUSE.0 as u32,
    /// Specular texture (reflectivity)
    Specular = sys::aiTextureType::aiTextureType_SPECULAR.0 as u32,
    /// Ambient texture (ambient lighting)
    Ambient = sys::aiTextureType::aiTextureType_AMBIENT.0 as u32,
    /// Emissive texture (self-illumination)
    Emissive = sys::aiTextureType::aiTextureType_EMISSIVE.0 as u32,
    /// Height texture (displacement mapping)
    Height = sys::aiTextureType::aiTextureType_HEIGHT.0 as u32,
    /// Normal texture (surface detail)
    Normals = sys::aiTextureType::aiTextureType_NORMALS.0 as u32,
    /// Shininess texture (specular power)
    Shininess = sys::aiTextureType::aiTextureType_SHININESS.0 as u32,
    /// Opacity texture (transparency)
    Opacity = sys::aiTextureType::aiTextureType_OPACITY.0 as u32,
    /// Displacement texture (geometry displacement)
    Displacement = sys::aiTextureType::aiTextureType_DISPLACEMENT.0 as u32,
    /// Lightmap texture (pre-computed lighting)
    Lightmap = sys::aiTextureType::aiTextureType_LIGHTMAP.0 as u32,
    /// Reflection texture (environment mapping)
    Reflection = sys::aiTextureType::aiTextureType_REFLECTION.0 as u32,
    /// Base color texture (PBR albedo)
    BaseColor = sys::aiTextureType::aiTextureType_BASE_COLOR.0 as u32,
    /// Normal camera texture (camera-space normals)
    NormalCamera = sys::aiTextureType::aiTextureType_NORMAL_CAMERA.0 as u32,
    /// Emission color texture (PBR emission)
    EmissionColor = sys::aiTextureType::aiTextureType_EMISSION_COLOR.0 as u32,
    /// Metalness texture (PBR metallic)
    Metalness = sys::aiTextureType::aiTextureType_METALNESS.0 as u32,
    /// Diffuse roughness texture (PBR roughness)
    DiffuseRoughness = sys::aiTextureType::aiTextureType_DIFFUSE_ROUGHNESS.0 as u32,
    /// Ambient occlusion texture (shadowing)
    AmbientOcclusion = sys::aiTextureType::aiTextureType_AMBIENT_OCCLUSION.0 as u32,
    /// Unknown texture type
    Unknown = sys::aiTextureType::aiTextureType_UNKNOWN.0 as u32,
    /// Sheen layer (PBR)
    Sheen = sys::aiTextureType::aiTextureType_SHEEN.0 as u32,
    /// Clearcoat layer (PBR)
    Clearcoat = sys::aiTextureType::aiTextureType_CLEARCOAT.0 as u32,
    /// Transmission layer (PBR)
    Transmission = sys::aiTextureType::aiTextureType_TRANSMISSION.0 as u32,
    /// Maya base color (compat)
    MayaBase = sys::aiTextureType::aiTextureType_MAYA_BASE.0 as u32,
    /// Maya specular (compat)
    MayaSpecular = sys::aiTextureType::aiTextureType_MAYA_SPECULAR.0 as u32,
    /// Maya specular color (compat)
    MayaSpecularColor = sys::aiTextureType::aiTextureType_MAYA_SPECULAR_COLOR.0 as u32,
    /// Maya specular roughness (compat)
    MayaSpecularRoughness = sys::aiTextureType::aiTextureType_MAYA_SPECULAR_ROUGHNESS.0 as u32,
    /// Anisotropy (PBR)
    Anisotropy = sys::aiTextureType::aiTextureType_ANISOTROPY.0 as u32,
    /// glTF metallic-roughness packed
    GltfMetallicRoughness = sys::aiTextureType::aiTextureType_GLTF_METALLIC_ROUGHNESS.0 as u32,
}

/// High-level shading model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadingModel {
    /// Flat shading - no interpolation between vertices
    Flat,
    /// Gouraud shading - linear interpolation of vertex colors
    Gouraud,
    /// Phong shading - per-pixel lighting calculation
    Phong,
    /// Blinn-Phong shading - modified Phong with Blinn's halfway vector
    Blinn,
    /// Toon/cel shading - cartoon-like rendering
    Toon,
    /// Oren-Nayar reflectance model for rough surfaces
    OrenNayar,
    /// Minnaert reflectance model
    Minnaert,
    /// Cook-Torrance reflectance model for metals
    CookTorrance,
    /// No shading - unlit material
    NoShading,
    /// Fresnel reflectance model
    Fresnel,
    /// Generic PBR BRDF; available workflows are described by the material properties.
    PbrBrdf,
    /// Unknown shading model with raw value
    Unknown(u32),
}

impl ShadingModel {
    fn from_raw(v: u32) -> Self {
        use sys::aiShadingMode;
        match v {
            x if x == aiShadingMode::aiShadingMode_Flat.0 as u32 => ShadingModel::Flat,
            x if x == aiShadingMode::aiShadingMode_Gouraud.0 as u32 => ShadingModel::Gouraud,
            x if x == aiShadingMode::aiShadingMode_Phong.0 as u32 => ShadingModel::Phong,
            x if x == aiShadingMode::aiShadingMode_Blinn.0 as u32 => ShadingModel::Blinn,
            x if x == aiShadingMode::aiShadingMode_Toon.0 as u32 => ShadingModel::Toon,
            x if x == aiShadingMode::aiShadingMode_OrenNayar.0 as u32 => ShadingModel::OrenNayar,
            x if x == aiShadingMode::aiShadingMode_Minnaert.0 as u32 => ShadingModel::Minnaert,
            x if x == aiShadingMode::aiShadingMode_CookTorrance.0 as u32 => {
                ShadingModel::CookTorrance
            }
            x if x == aiShadingMode::aiShadingMode_NoShading.0 as u32 => ShadingModel::NoShading,
            x if x == aiShadingMode::aiShadingMode_Fresnel.0 as u32 => ShadingModel::Fresnel,
            x if x == aiShadingMode::aiShadingMode_PBR_BRDF.0 as u32 => ShadingModel::PbrBrdf,
            other => ShadingModel::Unknown(other),
        }
    }
}

/// High-level classification of material property data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyTypeInfo {
    /// Single-precision floating point value
    Float,
    /// Double-precision floating point value
    Double,
    /// String value
    String,
    /// Integer value
    Integer,
    /// Binary buffer/blob data
    Buffer,
    /// Unknown property type with raw value
    Unknown(u32),
}

impl PropertyTypeInfo {
    fn from_sys(t: sys::aiPropertyTypeInfo) -> Self {
        match t {
            sys::aiPropertyTypeInfo::aiPTI_Float => Self::Float,
            sys::aiPropertyTypeInfo::aiPTI_Double => Self::Double,
            sys::aiPropertyTypeInfo::aiPTI_String => Self::String,
            sys::aiPropertyTypeInfo::aiPTI_Integer => Self::Integer,
            sys::aiPropertyTypeInfo::aiPTI_Buffer => Self::Buffer,
            other => Self::Unknown(other.0 as u32),
        }
    }
}

/// Raw information about a material property
#[derive(Debug, Clone)]
pub struct MaterialPropertyInfo {
    /// Property key
    pub key: String,
    /// Semantic (texture type) if texture-related
    pub semantic: Option<TextureType>,
    /// Texture index (0 for non-texture)
    pub index: u32,
    /// Data length in bytes
    pub data_length: u32,
    /// Property type info
    pub type_info: PropertyTypeInfo,
}

impl MaterialPropertyInfo {
    fn from_ref(p: MaterialPropertyRef) -> Self {
        let semantic = p.semantic();
        Self {
            key: p.key_string(),
            semantic,
            index: p.index(),
            data_length: p.data().len() as u32,
            type_info: p.type_info(),
        }
    }
}

/// Zero-copy view of an Assimp material property.
#[derive(Debug, Clone)]
pub struct MaterialPropertyRef {
    #[allow(dead_code)]
    scene: Scene,
    prop_ptr: SharedPtr<sys::aiMaterialProperty>,
}

/// Decoder for `aiMaterialProperty::mData`.
///
/// Assimp treats `mData` as a byte blob; callers must not assume alignment.
struct MaterialPropertyData<'a> {
    bytes: &'a [u8],
    type_info: PropertyTypeInfo,
}

impl<'a> MaterialPropertyData<'a> {
    /// # Safety
    /// The returned slice borrows Assimp-owned memory and must not outlive the scene.
    unsafe fn from_sys(prop: &'a sys::aiMaterialProperty) -> Option<Self> {
        let len = prop.mDataLength as usize;
        if len > isize::MAX as usize {
            return None;
        }
        let bytes = unsafe { ffi::slice_from_ptr_len_opt(prop, prop.mData as *const u8, len) }?;
        Some(Self {
            bytes,
            type_info: PropertyTypeInfo::from_sys(prop.mType),
        })
    }

    fn i32_source_width(&self) -> Option<usize> {
        let width = match self.type_info {
            PropertyTypeInfo::Integer | PropertyTypeInfo::Buffer if self.bytes.len() == 1 => 1,
            PropertyTypeInfo::Integer | PropertyTypeInfo::Buffer | PropertyTypeInfo::Float => 4,
            PropertyTypeInfo::Double => 8,
            _ => return None,
        };
        (self.bytes.len() % width == 0).then_some(width)
    }

    fn i32_count(&self) -> Option<usize> {
        if self.type_info == PropertyTypeInfo::String {
            return self.float_count();
        }
        Some(self.bytes.len() / self.i32_source_width()?)
    }

    fn read_i32(&self, index: usize) -> Option<i32> {
        if self.type_info == PropertyTypeInfo::String {
            let value = self.string_ref()?;
            return std::str::from_utf8(value.as_bytes())
                .ok()?
                .split_whitespace()
                .nth(index)?
                .parse()
                .ok();
        }
        let offset = index.checked_mul(self.i32_source_width()?)?;
        let value = match self.type_info {
            PropertyTypeInfo::Integer | PropertyTypeInfo::Buffer if self.bytes.len() == 1 => {
                return Some(i32::from(*self.bytes.get(offset)? as std::os::raw::c_char));
            }
            PropertyTypeInfo::Integer | PropertyTypeInfo::Buffer => {
                return self.read_ne_i32(offset);
            }
            PropertyTypeInfo::Float => f64::from(self.read_ne_f32(offset)?),
            PropertyTypeInfo::Double => self.read_ne_f64(offset)?,
            _ => return None,
        };
        let truncated = value.trunc();
        if !truncated.is_finite()
            || truncated < f64::from(i32::MIN)
            || truncated > f64::from(i32::MAX)
        {
            return None;
        }
        Some(truncated as i32)
    }

    fn float_source_width(&self) -> Option<usize> {
        let width = match self.type_info {
            PropertyTypeInfo::Float | PropertyTypeInfo::Buffer | PropertyTypeInfo::Integer => 4,
            PropertyTypeInfo::Double => 8,
            _ => return None,
        };
        (self.bytes.len() % width == 0).then_some(width)
    }

    fn float_count(&self) -> Option<usize> {
        if self.type_info == PropertyTypeInfo::String {
            let value = self.string_ref()?;
            return Some(
                std::str::from_utf8(value.as_bytes())
                    .ok()?
                    .split_whitespace()
                    .count(),
            );
        }
        Some(self.bytes.len() / self.float_source_width()?)
    }

    fn read_f64(&self, index: usize) -> Option<f64> {
        if self.type_info == PropertyTypeInfo::String {
            let value = self.string_ref()?;
            return std::str::from_utf8(value.as_bytes())
                .ok()?
                .split_whitespace()
                .nth(index)?
                .parse()
                .ok();
        }
        let offset = index.checked_mul(self.float_source_width()?)?;
        match self.type_info {
            PropertyTypeInfo::Float | PropertyTypeInfo::Buffer => {
                Some(f64::from(self.read_ne_f32(offset)?))
            }
            PropertyTypeInfo::Double => self.read_ne_f64(offset),
            PropertyTypeInfo::Integer => Some(f64::from(self.read_ne_i32(offset)?)),
            _ => None,
        }
    }

    fn read_ne_u32(&self, offset: usize) -> Option<u32> {
        let raw: [u8; 4] = self
            .bytes
            .get(offset..offset.checked_add(4)?)?
            .try_into()
            .ok()?;
        Some(u32::from_ne_bytes(raw))
    }

    fn read_ne_i32(&self, offset: usize) -> Option<i32> {
        let raw: [u8; 4] = self
            .bytes
            .get(offset..offset.checked_add(4)?)?
            .try_into()
            .ok()?;
        Some(i32::from_ne_bytes(raw))
    }

    fn read_ne_f32(&self, offset: usize) -> Option<f32> {
        let raw: [u8; 4] = self
            .bytes
            .get(offset..offset.checked_add(4)?)?
            .try_into()
            .ok()?;
        Some(f32::from_ne_bytes(raw))
    }

    fn read_ne_f64(&self, offset: usize) -> Option<f64> {
        let raw: [u8; 8] = self
            .bytes
            .get(offset..offset.checked_add(8)?)?
            .try_into()
            .ok()?;
        Some(f64::from_ne_bytes(raw))
    }

    fn read_f32_array<const N: usize>(&self) -> Option<[f32; N]> {
        Some(self.read_f64_array::<N>()?.map(|value| value as f32))
    }

    fn read_f64_array<const N: usize>(&self) -> Option<[f64; N]> {
        self.float_count()?;
        let mut out = [0.0f64; N];
        for (index, slot) in out.iter_mut().enumerate() {
            *slot = self.read_f64(index)?;
        }
        Some(out)
    }

    fn string_ref(&self) -> Option<MaterialStringRef> {
        if self.type_info != PropertyTypeInfo::String {
            return None;
        }
        Some(MaterialStringRef {
            value: self.decode_ai_string()?,
        })
    }

    fn decode_ai_string(&self) -> Option<sys::aiString> {
        let declared_len = self.read_ne_u32(0)? as usize;
        let payload = self.bytes.get(4..)?;
        if declared_len >= sys::AI_MAXLEN as usize
            || payload.len() != declared_len.checked_add(1)?
            || *payload.get(declared_len)? != 0
        {
            return None;
        }

        let mut value = sys::aiString {
            length: declared_len as u32,
            data: [0; sys::AI_MAXLEN as usize],
        };

        for (dst, src) in value.data[..declared_len]
            .iter_mut()
            .zip(&payload[..declared_len])
        {
            *dst = *src as std::os::raw::c_char;
        }
        Some(value)
    }
}

impl MaterialPropertyRef {
    /// # Safety
    /// A non-null, aligned property and its data must remain valid and immutable while
    /// any clone of the resulting view exists. `scene` must retain their owner.
    unsafe fn from_ptr(scene: Scene, prop_ptr: *const sys::aiMaterialProperty) -> Option<Self> {
        let prop_ptr = unsafe { SharedPtr::new(prop_ptr) }?;
        Some(Self { scene, prop_ptr })
    }

    #[inline]
    fn raw(&self) -> &sys::aiMaterialProperty {
        self.prop_ptr.as_ref()
    }

    /// Property key as UTF-8 (lossy), without allocation.
    pub fn key_str(&self) -> Cow<'_, str> {
        ai_string_to_str(&self.raw().mKey)
    }

    /// Raw bytes of the key (without assuming NUL-termination).
    pub fn key_bytes(&self) -> &[u8] {
        let s = &self.raw().mKey;
        let len = (s.length as usize).min(s.data.len());
        unsafe { ffi::slice_from_ptr_len(self, s.data.as_ptr() as *const u8, len) }
    }

    /// Property key as owned `String` (allocates).
    pub fn key_string(&self) -> String {
        ai_string_to_string(&self.raw().mKey)
    }

    /// Semantic (texture type) if texture-related.
    pub fn semantic(&self) -> Option<TextureType> {
        TextureType::from_u32(self.raw().mSemantic)
    }

    /// Texture index (0 for non-texture properties).
    pub fn index(&self) -> u32 {
        self.raw().mIndex
    }

    /// Property type info.
    pub fn type_info(&self) -> PropertyTypeInfo {
        PropertyTypeInfo::from_sys(self.raw().mType)
    }

    /// Raw property bytes as stored by Assimp (zero-copy).
    pub fn data(&self) -> &[u8] {
        unsafe { MaterialPropertyData::from_sys(self.raw()) }
            .map(|data| data.bytes)
            .unwrap_or_default()
    }

    /// Interpret the property payload as an `i32` slice when stored as `Integer` (zero-copy).
    pub fn data_i32(&self) -> Option<&[i32]> {
        (self.type_info() == PropertyTypeInfo::Integer)
            .then(|| unsafe { self.data_cast_slice_opt() })
            .flatten()
    }

    /// Interpret the property payload as a `u32` slice when stored as `Integer` (zero-copy).
    ///
    /// Note: Assimp stores integer properties as signed `int`. This method simply
    /// reinterprets the bytes as `u32` when the payload is aligned and sized for `u32`.
    pub fn data_u32(&self) -> Option<&[u32]> {
        (self.type_info() == PropertyTypeInfo::Integer)
            .then(|| unsafe { self.data_cast_slice_opt() })
            .flatten()
    }

    /// Interpret the property payload as an `f32` slice when stored as `Float` (zero-copy).
    pub fn data_f32(&self) -> Option<&[f32]> {
        (self.type_info() == PropertyTypeInfo::Float)
            .then(|| unsafe { self.data_cast_slice_opt() })
            .flatten()
    }

    /// Interpret the property payload as an `f64` slice when stored as `Double` (zero-copy).
    pub fn data_f64(&self) -> Option<&[f64]> {
        (self.type_info() == PropertyTypeInfo::Double)
            .then(|| unsafe { self.data_cast_slice_opt() })
            .flatten()
    }

    /// Interpret the property payload as an `aiString` when stored as `String` (no heap allocation).
    ///
    /// Assimp material properties store strings in a compact `aiString`-compatible binary layout:
    /// a 32-bit byte length followed by exactly that many bytes and a trailing NUL.
    /// We decode this representation into an owned `aiString` buffer for safe, allocation-free use.
    /// Invalid lengths, missing terminators, and strings exceeding `AI_MAXLEN - 1` return `None`.
    pub fn string_ref(&self) -> Option<MaterialStringRef> {
        let p = self.raw();
        let d = unsafe { MaterialPropertyData::from_sys(p) }?;
        d.string_ref()
    }

    /// Read the first element as `i32` when stored as `Integer`.
    pub fn as_i32(&self) -> Option<i32> {
        if self.type_info() != PropertyTypeInfo::Integer {
            return None;
        }
        let p = self.raw();
        let d = unsafe { MaterialPropertyData::from_sys(p) }?;
        d.read_i32(0)
    }

    /// Read the first element as `u32` when stored as `Integer` and non-negative.
    pub fn as_u32(&self) -> Option<u32> {
        u32::try_from(self.as_i32()?).ok()
    }

    /// Read the first element as `bool` when stored as `Integer` (non-zero => true).
    pub fn as_bool(&self) -> Option<bool> {
        Some(self.as_i32()? != 0)
    }

    /// Read the first element as `f32` when stored as `Float`.
    pub fn as_f32(&self) -> Option<f32> {
        if self.type_info() != PropertyTypeInfo::Float {
            return None;
        }
        let p = self.raw();
        let d = unsafe { MaterialPropertyData::from_sys(p) }?;
        Some(d.read_f64(0)? as f32)
    }

    /// Read the first element as `f64` when stored as `Double`.
    pub fn as_f64(&self) -> Option<f64> {
        if self.type_info() != PropertyTypeInfo::Double {
            return None;
        }
        let p = self.raw();
        let d = unsafe { MaterialPropertyData::from_sys(p) }?;
        d.read_f64(0)
    }

    /// Read `N` elements as `f32` when stored as `Float`.
    pub fn as_f32_array<const N: usize>(&self) -> Option<[f32; N]> {
        if self.type_info() != PropertyTypeInfo::Float {
            return None;
        }
        let p = self.raw();
        let d = unsafe { MaterialPropertyData::from_sys(p) }?;
        d.read_f32_array::<N>()
    }

    /// Read `N` elements as `f64` when stored as `Double`.
    pub fn as_f64_array<const N: usize>(&self) -> Option<[f64; N]> {
        if self.type_info() != PropertyTypeInfo::Double {
            return None;
        }
        let p = self.raw();
        let d = unsafe { MaterialPropertyData::from_sys(p) }?;
        d.read_f64_array::<N>()
    }

    /// Read a `Vec2` when stored as `Float` and the payload has at least 2 floats.
    pub fn as_vec2(&self) -> Option<Vector2D> {
        let [x, y] = self.as_f32_array::<2>()?;
        Some(Vector2D::new(x, y))
    }

    /// Read a `Vec3` when stored as `Float` and the payload has at least 3 floats.
    pub fn as_vec3(&self) -> Option<Vector3D> {
        let [x, y, z] = self.as_f32_array::<3>()?;
        Some(Vector3D::new(x, y, z))
    }

    /// Read a `Vec4` when stored as `Float` and the payload has at least 4 floats.
    pub fn as_vec4(&self) -> Option<Vector4D> {
        let [x, y, z, w] = self.as_f32_array::<4>()?;
        Some(Vector4D::new(x, y, z, w))
    }

    /// Read an RGB color when stored as `Float` and the payload has at least 3 floats.
    pub fn as_color3(&self) -> Option<Color3D> {
        self.as_vec3()
    }

    /// Read an RGBA color when stored as `Float` and the payload has at least 4 floats.
    pub fn as_color4(&self) -> Option<Color4D> {
        self.as_vec4()
    }

    /// # Safety
    /// Every initialized payload bit pattern must be a valid `T`, without mutable
    /// interior state. Callers use only primitive numeric types.
    unsafe fn data_cast_slice_opt<T>(&self) -> Option<&[T]> {
        let data = unsafe { MaterialPropertyData::from_sys(self.raw()) }?;
        let len = data.bytes.len();
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        if len == 0 {
            return Some(&[]);
        }
        let ptr = data.bytes.as_ptr();
        if size == 0 {
            return Some(&[]);
        }
        if (ptr as usize) % align != 0 || len % size != 0 {
            return None;
        }
        Some(unsafe { ffi::slice_from_ptr_len(self, ptr as *const T, len / size) })
    }

    fn into_info(self) -> MaterialPropertyInfo {
        MaterialPropertyInfo::from_ref(self)
    }
}

/// Iterator over material properties (skips null entries).
pub struct MaterialPropertyIterator {
    scene: Scene,
    props: Option<SharedPtr<*const sys::aiMaterialProperty>>,
    count: usize,
    index: usize,
}

impl Iterator for MaterialPropertyIterator {
    type Item = MaterialPropertyRef;

    fn next(&mut self) -> Option<Self::Item> {
        let props = self.props?;
        let slice =
            unsafe { crate::ffi::slice_from_ptr_len_opt(&self.scene, props.as_ptr(), self.count) }?;
        while self.index < slice.len() {
            let ptr = slice[self.index];
            self.index += 1;
            if ptr.is_null() {
                continue;
            }
            if let Some(prop) = unsafe { MaterialPropertyRef::from_ptr(self.scene.clone(), ptr) } {
                return Some(prop);
            }
        }
        None
    }
}

impl TextureType {
    #[inline]
    fn to_sys(self) -> sys::aiTextureType {
        match self {
            Self::Diffuse => sys::aiTextureType::aiTextureType_DIFFUSE,
            Self::Specular => sys::aiTextureType::aiTextureType_SPECULAR,
            Self::Ambient => sys::aiTextureType::aiTextureType_AMBIENT,
            Self::Emissive => sys::aiTextureType::aiTextureType_EMISSIVE,
            Self::Height => sys::aiTextureType::aiTextureType_HEIGHT,
            Self::Normals => sys::aiTextureType::aiTextureType_NORMALS,
            Self::Shininess => sys::aiTextureType::aiTextureType_SHININESS,
            Self::Opacity => sys::aiTextureType::aiTextureType_OPACITY,
            Self::Displacement => sys::aiTextureType::aiTextureType_DISPLACEMENT,
            Self::Lightmap => sys::aiTextureType::aiTextureType_LIGHTMAP,
            Self::Reflection => sys::aiTextureType::aiTextureType_REFLECTION,
            Self::BaseColor => sys::aiTextureType::aiTextureType_BASE_COLOR,
            Self::NormalCamera => sys::aiTextureType::aiTextureType_NORMAL_CAMERA,
            Self::EmissionColor => sys::aiTextureType::aiTextureType_EMISSION_COLOR,
            Self::Metalness => sys::aiTextureType::aiTextureType_METALNESS,
            Self::DiffuseRoughness => sys::aiTextureType::aiTextureType_DIFFUSE_ROUGHNESS,
            Self::AmbientOcclusion => sys::aiTextureType::aiTextureType_AMBIENT_OCCLUSION,
            Self::Unknown => sys::aiTextureType::aiTextureType_UNKNOWN,
            Self::Sheen => sys::aiTextureType::aiTextureType_SHEEN,
            Self::Clearcoat => sys::aiTextureType::aiTextureType_CLEARCOAT,
            Self::Transmission => sys::aiTextureType::aiTextureType_TRANSMISSION,
            Self::MayaBase => sys::aiTextureType::aiTextureType_MAYA_BASE,
            Self::MayaSpecular => sys::aiTextureType::aiTextureType_MAYA_SPECULAR,
            Self::MayaSpecularColor => sys::aiTextureType::aiTextureType_MAYA_SPECULAR_COLOR,
            Self::MayaSpecularRoughness => {
                sys::aiTextureType::aiTextureType_MAYA_SPECULAR_ROUGHNESS
            }
            Self::Anisotropy => sys::aiTextureType::aiTextureType_ANISOTROPY,
            Self::GltfMetallicRoughness => {
                sys::aiTextureType::aiTextureType_GLTF_METALLIC_ROUGHNESS
            }
        }
    }

    #[inline]
    fn to_semantic(self) -> u32 {
        self.to_sys().0 as u32
    }

    /// Try convert a raw u32 (aiTextureType) into TextureType safely
    pub fn from_u32(v: u32) -> Option<Self> {
        Some(match v {
            x if x == sys::aiTextureType::aiTextureType_DIFFUSE.0 as u32 => Self::Diffuse,
            x if x == sys::aiTextureType::aiTextureType_SPECULAR.0 as u32 => Self::Specular,
            x if x == sys::aiTextureType::aiTextureType_AMBIENT.0 as u32 => Self::Ambient,
            x if x == sys::aiTextureType::aiTextureType_EMISSIVE.0 as u32 => Self::Emissive,
            x if x == sys::aiTextureType::aiTextureType_HEIGHT.0 as u32 => Self::Height,
            x if x == sys::aiTextureType::aiTextureType_NORMALS.0 as u32 => Self::Normals,
            x if x == sys::aiTextureType::aiTextureType_SHININESS.0 as u32 => Self::Shininess,
            x if x == sys::aiTextureType::aiTextureType_OPACITY.0 as u32 => Self::Opacity,
            x if x == sys::aiTextureType::aiTextureType_DISPLACEMENT.0 as u32 => Self::Displacement,
            x if x == sys::aiTextureType::aiTextureType_LIGHTMAP.0 as u32 => Self::Lightmap,
            x if x == sys::aiTextureType::aiTextureType_REFLECTION.0 as u32 => Self::Reflection,
            x if x == sys::aiTextureType::aiTextureType_BASE_COLOR.0 as u32 => Self::BaseColor,
            x if x == sys::aiTextureType::aiTextureType_NORMAL_CAMERA.0 as u32 => {
                Self::NormalCamera
            }
            x if x == sys::aiTextureType::aiTextureType_EMISSION_COLOR.0 as u32 => {
                Self::EmissionColor
            }
            x if x == sys::aiTextureType::aiTextureType_METALNESS.0 as u32 => Self::Metalness,
            x if x == sys::aiTextureType::aiTextureType_DIFFUSE_ROUGHNESS.0 as u32 => {
                Self::DiffuseRoughness
            }
            x if x == sys::aiTextureType::aiTextureType_AMBIENT_OCCLUSION.0 as u32 => {
                Self::AmbientOcclusion
            }
            x if x == sys::aiTextureType::aiTextureType_UNKNOWN.0 as u32 => Self::Unknown,
            x if x == sys::aiTextureType::aiTextureType_SHEEN.0 as u32 => Self::Sheen,
            x if x == sys::aiTextureType::aiTextureType_CLEARCOAT.0 as u32 => Self::Clearcoat,
            x if x == sys::aiTextureType::aiTextureType_TRANSMISSION.0 as u32 => Self::Transmission,
            x if x == sys::aiTextureType::aiTextureType_MAYA_BASE.0 as u32 => Self::MayaBase,
            x if x == sys::aiTextureType::aiTextureType_MAYA_SPECULAR.0 as u32 => {
                Self::MayaSpecular
            }
            x if x == sys::aiTextureType::aiTextureType_MAYA_SPECULAR_COLOR.0 as u32 => {
                Self::MayaSpecularColor
            }
            x if x == sys::aiTextureType::aiTextureType_MAYA_SPECULAR_ROUGHNESS.0 as u32 => {
                Self::MayaSpecularRoughness
            }
            x if x == sys::aiTextureType::aiTextureType_ANISOTROPY.0 as u32 => Self::Anisotropy,
            x if x == sys::aiTextureType::aiTextureType_GLTF_METALLIC_ROUGHNESS.0 as u32 => {
                Self::GltfMetallicRoughness
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod material_property_data_tests {
    use super::*;

    fn ai_string_from_cstr(value: &CStr) -> sys::aiString {
        let bytes = value.to_bytes();
        assert!(bytes.len() < 1024);

        let mut out = sys::aiString {
            length: bytes.len() as u32,
            ..Default::default()
        };
        for (idx, byte) in bytes.iter().copied().enumerate() {
            out.data[idx] = byte as std::os::raw::c_char;
        }
        out
    }

    fn make_prop_with_data(mut data: Vec<u8>) -> (sys::aiMaterialProperty, Vec<u8>) {
        let prop = sys::aiMaterialProperty {
            mDataLength: data.len() as u32,
            mData: data.as_mut_ptr().cast::<std::os::raw::c_char>(),
            ..Default::default()
        };
        (prop, data)
    }

    /// # Safety
    /// Every non-null native payload must point to initialized, readable memory valid
    /// for its declared length and remain alive and immutable throughout this call.
    /// Null payloads may only be used with operations that reject them before reading.
    /// The material and any cloned or derived views, references, or iterators must not
    /// escape the callback, including through other threads. The retained scene does
    /// not own the fixture buffers.
    unsafe fn with_material(
        properties: &mut [sys::aiMaterialProperty],
        check: impl FnOnce(&Material),
    ) {
        let scene = Scene::from_memory(b"o tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n", Some("obj"))
            .expect("import owner scene");
        let mut pointers: Vec<_> = properties.iter_mut().map(std::ptr::from_mut).collect();
        let raw = sys::aiMaterial {
            mProperties: pointers.as_mut_ptr(),
            mNumProperties: pointers.len() as u32,
            mNumAllocated: pointers.len() as u32,
        };
        // The fixture owns all native buffers; only the imported owner scene uses Assimp's allocator.
        // The material view is dropped before any of the borrowed fixture storage.
        let material = Material {
            scene,
            material_ptr: unsafe { SharedPtr::new(&raw) }.unwrap(),
        };
        check(&material);
    }

    fn assert_integer_getters(
        type_info: sys::aiPropertyTypeInfo,
        mut bytes: Vec<u8>,
        expected: Option<Vec<i32>>,
    ) {
        let property = sys::aiMaterialProperty {
            mKey: ai_string_from_cstr(c"$test.number"),
            mType: type_info,
            mData: bytes.as_mut_ptr().cast(),
            mDataLength: bytes.len() as u32,
            ..Default::default()
        };
        unsafe {
            with_material(&mut [property], |material| {
                assert_eq!(
                    material.get_property_i32_array(c"$test.number", None, 0),
                    expected
                );
                assert_eq!(
                    material.get_integer_property(c"$test.number"),
                    expected.as_ref().and_then(|values| values.first().copied())
                );
            });
        }
    }

    #[test]
    fn integer_getters_decode_one_byte_and_native_i32_payloads() {
        for byte in [0u8, 1, 2, u8::MAX] {
            assert_integer_getters(
                sys::aiPropertyTypeInfo::aiPTI_Integer,
                vec![byte],
                Some(vec![i32::from(byte as std::os::raw::c_char)]),
            );
        }
        assert_integer_getters(
            sys::aiPropertyTypeInfo::aiPTI_Integer,
            [i32::MIN, -42, 0, i32::MAX]
                .into_iter()
                .flat_map(i32::to_ne_bytes)
                .collect(),
            Some(vec![i32::MIN, -42, 0, i32::MAX]),
        );
        assert_integer_getters(sys::aiPropertyTypeInfo::aiPTI_Integer, vec![], Some(vec![]));
    }

    #[test]
    fn integer_getters_truncate_float_and_double_values_with_checked_bounds() {
        assert_integer_getters(
            sys::aiPropertyTypeInfo::aiPTI_Float,
            [1.75f32, -2.75, -2_147_483_648.0, 2_147_483_520.0]
                .into_iter()
                .flat_map(f32::to_ne_bytes)
                .collect(),
            Some(vec![1, -2, i32::MIN, 2_147_483_520]),
        );
        assert_integer_getters(
            sys::aiPropertyTypeInfo::aiPTI_Double,
            [
                1.75,
                -2.75,
                f64::from(i32::MIN) - 0.75,
                f64::from(i32::MAX) + 0.75,
            ]
            .into_iter()
            .flat_map(f64::to_ne_bytes)
            .collect(),
            Some(vec![1, -2, i32::MIN, i32::MAX]),
        );
        for value in [
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            2_147_483_648.0,
            -2_147_483_904.0,
        ] {
            assert_integer_getters(
                sys::aiPropertyTypeInfo::aiPTI_Float,
                value.to_ne_bytes().to_vec(),
                None,
            );
        }
        for value in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::from(i32::MAX) + 1.0,
            f64::from(i32::MIN) - 1.0,
        ] {
            assert_integer_getters(
                sys::aiPropertyTypeInfo::aiPTI_Double,
                value.to_ne_bytes().to_vec(),
                None,
            );
        }
    }

    #[test]
    fn integer_getters_reject_malformed_and_non_numeric_payloads() {
        // Assimp's generic AddProperty stores enum values as a Buffer.
        assert_integer_getters(
            sys::aiPropertyTypeInfo::aiPTI_Buffer,
            11i32.to_ne_bytes().to_vec(),
            Some(vec![11]),
        );
        for (type_info, lengths) in [
            (sys::aiPropertyTypeInfo::aiPTI_Integer, &[2, 3, 5][..]),
            (sys::aiPropertyTypeInfo::aiPTI_Buffer, &[2, 3, 5][..]),
            (sys::aiPropertyTypeInfo::aiPTI_Float, &[1, 3, 5][..]),
            (sys::aiPropertyTypeInfo::aiPTI_Double, &[1, 7, 9][..]),
        ] {
            for &length in lengths {
                assert_integer_getters(type_info, vec![0; length], None);
            }
        }
        for type_info in [
            sys::aiPropertyTypeInfo::aiPTI_String,
            sys::aiPropertyTypeInfo(77),
        ] {
            assert_integer_getters(type_info, vec![0; 8], None);
        }
        let property = sys::aiMaterialProperty {
            mKey: ai_string_from_cstr(c"$test.number"),
            mType: sys::aiPropertyTypeInfo::aiPTI_Integer,
            mData: std::ptr::null_mut(),
            mDataLength: 4,
            ..Default::default()
        };
        unsafe {
            with_material(&mut [property], |material| {
                assert_eq!(material.get_integer_property(c"$test.number"), None);
                assert_eq!(
                    material.get_property_i32_array(c"$test.number", None, 0),
                    None
                );
            });
        }
    }

    #[test]
    fn integer_array_rejects_invalid_later_elements_and_preserves_scoped_queries() {
        let mut bytes: Vec<_> = [1.5, f64::NAN]
            .into_iter()
            .flat_map(f64::to_ne_bytes)
            .collect();
        let property = sys::aiMaterialProperty {
            mKey: ai_string_from_cstr(c"$test.number"),
            mType: sys::aiPropertyTypeInfo::aiPTI_Double,
            mData: bytes.as_mut_ptr().cast(),
            mDataLength: bytes.len() as u32,
            ..Default::default()
        };
        unsafe {
            with_material(&mut [property], |material| {
                assert_eq!(material.get_integer_property(c"$test.number"), Some(1));
                assert_eq!(
                    material.get_property_i32_array(c"$test.number", None, 0),
                    None
                );
            });
        }
        bytes = (-2.75f64).to_ne_bytes().to_vec();
        let property = sys::aiMaterialProperty {
            mSemantic: TextureType::Normals.to_semantic(),
            mIndex: 2,
            mData: bytes.as_mut_ptr().cast(),
            mDataLength: bytes.len() as u32,
            ..property
        };
        unsafe {
            with_material(&mut [property], |material| {
                assert_eq!(
                    material.get_property_i32_array(c"$test.number", Some(TextureType::Normals), 2),
                    Some(vec![-2])
                );
                assert_eq!(
                    material.get_property_i32_array(c"$test.number", None, 0),
                    None
                );
            });
        }
    }

    #[test]
    fn float_and_color_getters_decode_numeric_types_and_preserve_scope() {
        for (type_info, bytes, expected) in [
            (
                sys::aiPropertyTypeInfo::aiPTI_Float,
                [0.25f32, 0.5, 0.75]
                    .into_iter()
                    .flat_map(f32::to_ne_bytes)
                    .collect(),
                vec![0.25, 0.5, 0.75],
            ),
            (
                sys::aiPropertyTypeInfo::aiPTI_Double,
                [0.25f64, 0.5, 0.75, 0.125]
                    .into_iter()
                    .flat_map(f64::to_ne_bytes)
                    .collect(),
                vec![0.25, 0.5, 0.75, 0.125],
            ),
            (
                sys::aiPropertyTypeInfo::aiPTI_Integer,
                [1i32, -2, 3, 4]
                    .into_iter()
                    .flat_map(i32::to_ne_bytes)
                    .collect(),
                vec![1.0, -2.0, 3.0, 4.0],
            ),
            (
                sys::aiPropertyTypeInfo::aiPTI_Buffer,
                [0.25f32, 0.5, 0.75]
                    .into_iter()
                    .flat_map(f32::to_ne_bytes)
                    .collect(),
                vec![0.25, 0.5, 0.75],
            ),
        ] {
            let (mut property, _owner) = make_prop_with_data(bytes);
            property.mKey = ai_string_from_cstr(c"$test.number");
            property.mType = type_info;
            let expected_f32: Vec<_> = expected.iter().map(|value| *value as f32).collect();
            unsafe {
                with_material(&mut [property], |material| {
                    assert_eq!(
                        material.get_float_property(c"$test.number"),
                        Some(expected_f32[0])
                    );
                    assert_eq!(
                        material
                            .get_property_f32_array(c"$test.number", None, 0)
                            .as_deref(),
                        Some(expected_f32.as_slice())
                    );
                    assert_eq!(
                        material
                            .get_property_f64_array(c"$test.number", None, 0)
                            .as_deref(),
                        Some(expected.as_slice())
                    );
                    assert_eq!(
                        material.get_color_property(c"$test.number"),
                        Some(Color4D::new(
                            expected_f32[0],
                            expected_f32[1],
                            expected_f32[2],
                            expected_f32.get(3).copied().unwrap_or(1.0),
                        ))
                    );
                });
            }
            property.mSemantic = TextureType::Normals.to_semantic();
            property.mIndex = 2;
            unsafe {
                with_material(&mut [property], |material| {
                    assert_eq!(
                        material.get_texture_float_property(
                            c"$test.number",
                            TextureType::Normals,
                            2
                        ),
                        Some(expected_f32[0])
                    );
                    assert_eq!(
                        material
                            .get_property_f64_array(c"$test.number", Some(TextureType::Normals), 2)
                            .as_deref(),
                        Some(expected.as_slice())
                    );
                    assert_eq!(material.get_float_property(c"$test.number"), None);
                    assert_eq!(
                        material.get_texture_float_property(
                            c"$test.number",
                            TextureType::Normals,
                            0
                        ),
                        None
                    );
                });
            }
        }
    }

    #[test]
    fn numeric_buffer_preserves_float_and_integer_interpretations() {
        let (mut property, _owner) = make_prop_with_data(0.5f32.to_ne_bytes().to_vec());
        property.mKey = ai_string_from_cstr(c"$test.number");
        property.mType = sys::aiPropertyTypeInfo::aiPTI_Buffer;
        unsafe {
            with_material(&mut [property], |material| {
                assert_eq!(material.get_float_property(c"$test.number"), Some(0.5));
                assert_eq!(
                    material.get_integer_property(c"$test.number"),
                    Some(i32::from_ne_bytes(0.5f32.to_ne_bytes()))
                );
            });
        }
        for byte in [0u8, 1, u8::MAX] {
            assert_integer_getters(
                sys::aiPropertyTypeInfo::aiPTI_Buffer,
                vec![byte],
                Some(vec![i32::from(byte as std::os::raw::c_char)]),
            );
        }
    }

    #[test]
    fn float_getters_reject_unknown_short_and_incomplete_numeric_data() {
        for (type_info, lengths) in [
            (sys::aiPropertyTypeInfo(77), &[0, 1, 4, 8][..]),
            (sys::aiPropertyTypeInfo::aiPTI_Float, &[1, 3, 5][..]),
            (sys::aiPropertyTypeInfo::aiPTI_Double, &[1, 7, 9][..]),
            (sys::aiPropertyTypeInfo::aiPTI_Integer, &[1, 2, 3, 5][..]),
            (sys::aiPropertyTypeInfo::aiPTI_Buffer, &[1, 2, 3, 5][..]),
            (sys::aiPropertyTypeInfo::aiPTI_String, &[1, 3, 4][..]),
        ] {
            for &length in lengths {
                let (mut property, _owner) = make_prop_with_data(vec![0; length]);
                property.mKey = ai_string_from_cstr(c"$test.number");
                property.mType = type_info;
                unsafe {
                    with_material(&mut [property], |material| {
                        assert_eq!(material.get_float_property(c"$test.number"), None);
                        assert_eq!(material.get_color_property(c"$test.number"), None);
                        assert_eq!(
                            material.get_property_f32_array(c"$test.number", None, 0),
                            None
                        );
                        assert_eq!(
                            material.get_property_f64_array(c"$test.number", None, 0),
                            None
                        );
                    });
                }
                property.mSemantic = TextureType::Normals.to_semantic();
                unsafe {
                    with_material(&mut [property], |material| {
                        assert_eq!(material.normal_texture_scale(0), None);
                        assert_eq!(
                            material.get_texture_float_property(
                                c"$test.number",
                                TextureType::Normals,
                                0
                            ),
                            None
                        );
                    });
                }
            }
        }
    }

    #[test]
    fn float_getters_preserve_ieee_values_and_require_three_color_components() {
        let bytes = [f64::INFINITY, f64::NEG_INFINITY, f64::NAN]
            .into_iter()
            .flat_map(f64::to_ne_bytes)
            .collect();
        let (mut property, _owner) = make_prop_with_data(bytes);
        property.mKey = ai_string_from_cstr(c"$test.number");
        property.mType = sys::aiPropertyTypeInfo::aiPTI_Double;
        unsafe {
            with_material(&mut [property], |material| {
                assert_eq!(
                    material.get_float_property(c"$test.number"),
                    Some(f32::INFINITY)
                );
                let color = material.get_color_property(c"$test.number").unwrap();
                assert_eq!(color.x, f32::INFINITY);
                assert_eq!(color.y, f32::NEG_INFINITY);
                assert!(color.z.is_nan());
                assert_eq!(color.w, 1.0);
            });
        }
        for count in [0, 1, 2] {
            property.mDataLength = count * 8;
            unsafe {
                with_material(&mut [property], |material| {
                    assert_eq!(material.get_color_property(c"$test.number"), None);
                    if count == 0 {
                        assert_eq!(material.get_float_property(c"$test.number"), None);
                        assert_eq!(
                            material.get_property_f64_array(c"$test.number", None, 0),
                            Some(vec![])
                        );
                    }
                });
            }
        }
    }

    #[test]
    fn numeric_strings_are_bounded_utf8_and_whitespace_parsed() {
        for text in [
            &b" 1\t-2\n3.5 4 "[..],
            &b"1 2 invalid 4"[..],
            &b"1.5suffix"[..],
            &b"\xff"[..],
        ] {
            let mut bytes = (text.len() as u32).to_ne_bytes().to_vec();
            bytes.extend_from_slice(text);
            bytes.push(0);
            let (mut property, _owner) = make_prop_with_data(bytes);
            property.mKey = ai_string_from_cstr(c"$test.number");
            property.mType = sys::aiPropertyTypeInfo::aiPTI_String;
            unsafe {
                with_material(&mut [property], |material| {
                    assert_eq!(
                        material
                            .get_string_property_ref(c"$test.number")
                            .unwrap()
                            .as_bytes(),
                        text
                    );
                    if text == b" 1\t-2\n3.5 4 " {
                        assert_eq!(material.get_float_property(c"$test.number"), Some(1.0));
                        assert_eq!(
                            material.get_color_property(c"$test.number"),
                            Some(Color4D::new(1.0, -2.0, 3.5, 4.0))
                        );
                        assert_eq!(
                            material.get_property_f32_array(c"$test.number", None, 0),
                            Some(vec![1.0, -2.0, 3.5, 4.0])
                        );
                        assert_eq!(
                            material.get_property_f64_array(c"$test.number", None, 0),
                            Some(vec![1.0, -2.0, 3.5, 4.0])
                        );
                    } else {
                        assert_eq!(material.get_color_property(c"$test.number"), None);
                        assert_eq!(
                            material.get_property_f64_array(c"$test.number", None, 0),
                            None
                        );
                    }
                });
            }
        }
        for (text, expected) in [
            (
                &b" 1 -2 2147483647 -2147483648 "[..],
                Some(vec![1, -2, i32::MAX, i32::MIN]),
            ),
            (&b"2147483648"[..], None),
            (&b"NaN"[..], None),
            (&b"1.5"[..], None),
        ] {
            let mut bytes = (text.len() as u32).to_ne_bytes().to_vec();
            bytes.extend_from_slice(text);
            bytes.push(0);
            assert_integer_getters(sys::aiPropertyTypeInfo::aiPTI_String, bytes, expected);
        }
    }

    #[test]
    fn texture_outputs_have_defaults_for_missing_and_non_uv_properties() {
        let mut path_bytes = (b"texture.png".len() as u32).to_ne_bytes().to_vec();
        path_bytes.extend_from_slice(b"texture.png\0");
        let path = sys::aiMaterialProperty {
            mKey: ai_string_from_cstr(c"$tex.file"),
            mSemantic: TextureType::Diffuse.to_semantic(),
            mType: sys::aiPropertyTypeInfo::aiPTI_String,
            mData: path_bytes.as_mut_ptr().cast(),
            mDataLength: path_bytes.len() as u32,
            ..Default::default()
        };
        unsafe {
            with_material(&mut [path], |material| {
                let texture = material.texture_ref(TextureType::Diffuse, 0).unwrap();
                assert_eq!(texture.path_bytes(), b"texture.png");
                assert_eq!(texture.mapping, TextureMapping::UV);
                assert_eq!(texture.uv_index, 0);
                assert_eq!(texture.blend_factor, 1.0);
                assert_eq!(texture.operation, TextureOperation::Multiply);
                assert_eq!(texture.map_modes, [TextureMapMode::Wrap; 3]);
                assert!(texture.flags.is_empty());
                assert!(texture.uv_transform.is_none());
            });
        }
        let mut mapping_bytes =
            (sys::aiTextureMapping::aiTextureMapping_SPHERE.0 as i32).to_ne_bytes();
        let mapping = sys::aiMaterialProperty {
            mKey: ai_string_from_cstr(c"$tex.mapping"),
            mType: sys::aiPropertyTypeInfo::aiPTI_Integer,
            mData: mapping_bytes.as_mut_ptr().cast(),
            mDataLength: mapping_bytes.len() as u32,
            ..path
        };
        let mut transform_bytes = 0.25f32.to_ne_bytes();
        let transform = sys::aiMaterialProperty {
            mKey: ai_string_from_cstr(c"$tex.uvtrafo"),
            mType: sys::aiPropertyTypeInfo::aiPTI_Float,
            mData: transform_bytes.as_mut_ptr().cast(),
            mDataLength: transform_bytes.len() as u32,
            ..path
        };
        unsafe {
            with_material(&mut [path, mapping, transform], |material| {
                let texture = material.texture_ref(TextureType::Diffuse, 0).unwrap();
                assert_eq!(texture.mapping, TextureMapping::Sphere);
                assert_eq!(texture.uv_index, 0);
                assert_eq!(texture.blend_factor, 1.0);
                assert_eq!(texture.operation, TextureOperation::Multiply);
                assert!(texture.uv_transform.is_none());
            });
        }
    }

    #[test]
    fn texture_integer_properties_reject_short_non_finite_and_out_of_range_values() {
        let mut path_bytes = 11u32.to_ne_bytes().to_vec();
        path_bytes.extend_from_slice(b"texture.png\0");
        let (mut path, _path_owner) = make_prop_with_data(path_bytes);
        path.mKey = ai_string_from_cstr(c"$tex.file");
        path.mSemantic = TextureType::Diffuse.to_semantic();
        path.mType = sys::aiPropertyTypeInfo::aiPTI_String;
        for (type_info, bytes) in [
            (sys::aiPropertyTypeInfo::aiPTI_Integer, vec![1, 2]),
            (sys::aiPropertyTypeInfo::aiPTI_Buffer, vec![1, 2]),
            (sys::aiPropertyTypeInfo(77), vec![1]),
            (sys::aiPropertyTypeInfo::aiPTI_String, vec![1]),
            (
                sys::aiPropertyTypeInfo::aiPTI_Float,
                f32::NAN.to_ne_bytes().to_vec(),
            ),
            (
                sys::aiPropertyTypeInfo::aiPTI_Float,
                f32::INFINITY.to_ne_bytes().to_vec(),
            ),
            (
                sys::aiPropertyTypeInfo::aiPTI_Float,
                2_147_483_648.0f32.to_ne_bytes().to_vec(),
            ),
            (
                sys::aiPropertyTypeInfo::aiPTI_Double,
                f64::NAN.to_ne_bytes().to_vec(),
            ),
            (
                sys::aiPropertyTypeInfo::aiPTI_Double,
                (f64::from(i32::MIN) - 1.0).to_ne_bytes().to_vec(),
            ),
        ] {
            let (data, _data_owner) = make_prop_with_data(bytes);
            let mut properties = vec![path];
            properties.extend(
                [
                    c"$tex.mapping",
                    c"$tex.uvwsrc",
                    c"$tex.op",
                    c"$tex.mapmodeu",
                    c"$tex.mapmodev",
                    c"$tex.flags",
                ]
                .map(|key| sys::aiMaterialProperty {
                    mKey: ai_string_from_cstr(key),
                    mType: type_info,
                    mData: data.mData,
                    mDataLength: data.mDataLength,
                    ..path
                }),
            );
            unsafe {
                with_material(&mut properties, |material| {
                    let texture = material.texture_ref(TextureType::Diffuse, 0).unwrap();
                    assert_eq!(texture.mapping, TextureMapping::UV);
                    assert_eq!(texture.uv_index, 0);
                    assert_eq!(texture.operation, TextureOperation::Multiply);
                    assert_eq!(texture.map_modes, [TextureMapMode::Wrap; 3]);
                    assert!(texture.flags.is_empty());
                });
            }
        }
    }

    #[test]
    fn texture_values_decode_scoped_buffers_doubles_and_unknown_enums() {
        let mut path_bytes = 11u32.to_ne_bytes().to_vec();
        path_bytes.extend_from_slice(b"texture.png\0");
        let (mut path, _path_owner) = make_prop_with_data(path_bytes);
        path.mKey = ai_string_from_cstr(c"$tex.file");
        path.mSemantic = TextureType::Normals.to_semantic();
        path.mIndex = 2;
        path.mType = sys::aiPropertyTypeInfo::aiPTI_String;
        let mut buffers = [
            (
                c"$tex.op",
                sys::aiPropertyTypeInfo::aiPTI_Double,
                77.75f64.to_ne_bytes().to_vec(),
            ),
            (
                c"$tex.blend",
                sys::aiPropertyTypeInfo::aiPTI_Buffer,
                0.5f32.to_ne_bytes().to_vec(),
            ),
            (
                c"$tex.uvwsrc",
                sys::aiPropertyTypeInfo::aiPTI_Buffer,
                vec![2],
            ),
            (
                c"$tex.mapmodeu",
                sys::aiPropertyTypeInfo::aiPTI_Integer,
                (-1i32).to_ne_bytes().to_vec(),
            ),
            (
                c"$tex.mapmodev",
                sys::aiPropertyTypeInfo::aiPTI_Buffer,
                78i32.to_ne_bytes().to_vec(),
            ),
            (
                c"$tex.flags",
                sys::aiPropertyTypeInfo::aiPTI_Buffer,
                vec![1],
            ),
        ];
        let mut properties = vec![path];
        properties.extend(buffers.iter_mut().map(|(key, type_info, bytes)| {
            sys::aiMaterialProperty {
                mKey: ai_string_from_cstr(key),
                mType: *type_info,
                mData: bytes.as_mut_ptr().cast(),
                mDataLength: bytes.len() as u32,
                ..path
            }
        }));
        unsafe {
            with_material(&mut properties, |material| {
                let texture = material.texture_ref(TextureType::Normals, 2).unwrap();
                assert_eq!(texture.path_bytes(), b"texture.png");
                assert_eq!(texture.mapping, TextureMapping::UV);
                assert_eq!(texture.uv_index, 2);
                assert_eq!(texture.blend_factor, 0.5);
                assert_eq!(texture.operation, TextureOperation::Unknown(77));
                assert_eq!(
                    texture.map_modes,
                    [
                        TextureMapMode::Unknown(u32::MAX),
                        TextureMapMode::Unknown(78),
                        TextureMapMode::Wrap,
                    ]
                );
                assert_eq!(texture.flags, TextureFlags::INVERT);
                assert!(material.texture_ref(TextureType::Normals, 0).is_none());
                assert!(material.texture_ref(TextureType::Diffuse, 2).is_none());
                assert!(
                    material
                        .texture_ref(TextureType::Normals, usize::MAX)
                        .is_none()
                );
            });
        }
        let mut mapping_bytes = 79i32.to_ne_bytes();
        properties.push(sys::aiMaterialProperty {
            mKey: ai_string_from_cstr(c"$tex.mapping"),
            mType: sys::aiPropertyTypeInfo::aiPTI_Integer,
            mData: mapping_bytes.as_mut_ptr().cast(),
            mDataLength: mapping_bytes.len() as u32,
            ..path
        });
        unsafe {
            with_material(&mut properties, |material| {
                let texture = material.texture_ref(TextureType::Normals, 2).unwrap();
                assert_eq!(texture.mapping, TextureMapping::Unknown(79));
                assert_eq!(texture.uv_index, 0);
            });
        }
    }

    #[test]
    fn texture_blend_transform_and_axis_reject_unknown_and_short_data() {
        let mut path_bytes = 11u32.to_ne_bytes().to_vec();
        path_bytes.extend_from_slice(b"texture.png\0");
        let (mut path, _path_owner) = make_prop_with_data(path_bytes);
        path.mKey = ai_string_from_cstr(c"$tex.file");
        path.mSemantic = TextureType::Diffuse.to_semantic();
        path.mType = sys::aiPropertyTypeInfo::aiPTI_String;
        for type_info in [
            sys::aiPropertyTypeInfo(77),
            sys::aiPropertyTypeInfo::aiPTI_Float,
            sys::aiPropertyTypeInfo::aiPTI_Double,
            sys::aiPropertyTypeInfo::aiPTI_Integer,
            sys::aiPropertyTypeInfo::aiPTI_Buffer,
            sys::aiPropertyTypeInfo::aiPTI_String,
        ] {
            let (data, _data_owner) = make_prop_with_data(vec![1]);
            let mut properties = vec![path];
            properties.extend(
                [c"$tex.blend", c"$tex.uvtrafo", c"$tex.mapaxis"].map(|key| {
                    sys::aiMaterialProperty {
                        mKey: ai_string_from_cstr(key),
                        mType: type_info,
                        mData: data.mData,
                        mDataLength: data.mDataLength,
                        ..path
                    }
                }),
            );
            unsafe {
                with_material(&mut properties, |material| {
                    let texture = material.texture_ref(TextureType::Diffuse, 0).unwrap();
                    assert_eq!(texture.blend_factor, 1.0);
                    assert!(texture.uv_transform.is_none());
                    assert!(texture.axis.is_none());
                });
            }
        }
    }

    #[test]
    fn texture_transform_and_axis_use_typed_bounded_numeric_decoding() {
        let mut path_bytes = 11u32.to_ne_bytes().to_vec();
        path_bytes.extend_from_slice(b"texture.png\0");
        let (mut path, _path_owner) = make_prop_with_data(path_bytes);
        path.mKey = ai_string_from_cstr(c"$tex.file");
        path.mSemantic = TextureType::Diffuse.to_semantic();
        path.mType = sys::aiPropertyTypeInfo::aiPTI_String;
        let mut string_bytes = 9u32.to_ne_bytes().to_vec();
        string_bytes.extend_from_slice(b"1 2 3 4 5\0");
        for (type_info, bytes) in [
            (
                sys::aiPropertyTypeInfo::aiPTI_Float,
                [1.0f32, 2.0, 3.0, 4.0, 5.0]
                    .into_iter()
                    .flat_map(f32::to_ne_bytes)
                    .collect(),
            ),
            (
                sys::aiPropertyTypeInfo::aiPTI_Double,
                [1.0f64, 2.0, 3.0, 4.0, 5.0]
                    .into_iter()
                    .flat_map(f64::to_ne_bytes)
                    .collect(),
            ),
            (
                sys::aiPropertyTypeInfo::aiPTI_Integer,
                [1i32, 2, 3, 4, 5]
                    .into_iter()
                    .flat_map(i32::to_ne_bytes)
                    .collect(),
            ),
            (
                sys::aiPropertyTypeInfo::aiPTI_Buffer,
                [1.0f32, 2.0, 3.0, 4.0, 5.0]
                    .into_iter()
                    .flat_map(f32::to_ne_bytes)
                    .collect(),
            ),
            (sys::aiPropertyTypeInfo::aiPTI_String, string_bytes),
        ] {
            let (data, _data_owner) = make_prop_with_data(bytes);
            let mut properties = vec![path];
            properties.extend([c"$tex.uvtrafo", c"$tex.mapaxis"].map(|key| {
                sys::aiMaterialProperty {
                    mKey: ai_string_from_cstr(key),
                    mType: type_info,
                    mData: data.mData,
                    mDataLength: data.mDataLength,
                    ..path
                }
            }));
            unsafe {
                with_material(&mut properties, |material| {
                    let texture = material.texture_ref(TextureType::Diffuse, 0).unwrap();
                    let transform = texture.uv_transform.unwrap();
                    assert_eq!(transform.translation, Vector2D::new(1.0, 2.0));
                    assert_eq!(transform.scaling, Vector2D::new(3.0, 4.0));
                    assert_eq!(transform.rotation, 5.0);
                    assert_eq!(texture.axis, Some(Vector3D::new(1.0, 2.0, 3.0)));
                });
            }
            if type_info == sys::aiPropertyTypeInfo::aiPTI_Float {
                for length in [0, 4, 8, 12, 16, 19] {
                    properties[1].mDataLength = length;
                    properties[2].mDataLength = length;
                    unsafe {
                        with_material(&mut properties, |material| {
                            let texture = material.texture_ref(TextureType::Diffuse, 0).unwrap();
                            assert!(texture.uv_transform.is_none());
                            assert_eq!(
                                texture.axis,
                                if matches!(length, 12 | 16) {
                                    Some(Vector3D::new(1.0, 2.0, 3.0))
                                } else {
                                    None
                                }
                            );
                        });
                    }
                }
            }
        }
    }

    #[test]
    fn string_and_texture_paths_reject_short_unknown_and_null_payloads() {
        for (type_info, bytes) in [
            (sys::aiPropertyTypeInfo(77), vec![1]),
            (sys::aiPropertyTypeInfo::aiPTI_String, vec![1]),
            (
                sys::aiPropertyTypeInfo::aiPTI_String,
                99u32.to_ne_bytes().to_vec(),
            ),
        ] {
            let (mut property, _owner) = make_prop_with_data(bytes);
            property.mKey = ai_string_from_cstr(c"$tex.file");
            property.mType = type_info;
            unsafe {
                with_material(
                    &mut [
                        property,
                        sys::aiMaterialProperty {
                            mSemantic: TextureType::Diffuse.to_semantic(),
                            ..property
                        },
                    ],
                    |material| {
                        assert!(material.get_string_property_ref(c"$tex.file").is_none());
                        assert!(material.texture_ref(TextureType::Diffuse, 0).is_none());
                    },
                );
            }
        }
        for length in [0, 4, 16] {
            let property = sys::aiMaterialProperty {
                mKey: ai_string_from_cstr(c"$tex.file"),
                mType: sys::aiPropertyTypeInfo::aiPTI_String,
                mDataLength: length,
                mData: std::ptr::null_mut(),
                ..Default::default()
            };
            unsafe {
                with_material(
                    &mut [
                        property,
                        sys::aiMaterialProperty {
                            mSemantic: TextureType::Diffuse.to_semantic(),
                            ..property
                        },
                    ],
                    |material| {
                        assert!(material.get_string_property_ref(c"$tex.file").is_none());
                        assert!(material.get_float_property(c"$tex.file").is_none());
                        assert!(material.get_color_property(c"$tex.file").is_none());
                        assert!(material.texture_ref(TextureType::Diffuse, 0).is_none());
                    },
                );
            }
        }
    }

    #[test]
    fn native_unknown_values_are_preserved() {
        assert_eq!(
            PropertyTypeInfo::from_sys(sys::aiPropertyTypeInfo(77)),
            PropertyTypeInfo::Unknown(77)
        );
        assert_eq!(
            TextureMapping::from_raw(sys::aiTextureMapping(77).0 as u32),
            TextureMapping::Unknown(77)
        );
        assert_eq!(
            TextureOperation::from_raw(sys::aiTextureOp(77).0 as u32),
            TextureOperation::Unknown(77)
        );
        assert_eq!(
            TextureMapMode::from_raw(sys::aiTextureMapMode(77).0 as u32),
            TextureMapMode::Unknown(77)
        );
        assert_eq!(ShadingModel::from_raw(77), ShadingModel::Unknown(77));
        assert_eq!(BlendMode::from_raw(77), BlendMode::Unknown(77));
    }

    #[test]
    fn emission_aliases_prefer_emission_color_and_fall_back_per_index() {
        let mut color_bytes = 7u32.to_ne_bytes().to_vec();
        color_bytes.extend_from_slice(b"pbr.png\0");
        let mut emissive_bytes = 8u32.to_ne_bytes().to_vec();
        emissive_bytes.extend_from_slice(b"gltf.png\0");
        let color = sys::aiMaterialProperty {
            mKey: ai_string_from_cstr(c"$tex.file"),
            mSemantic: TextureType::EmissionColor.to_semantic(),
            mType: sys::aiPropertyTypeInfo::aiPTI_String,
            mData: color_bytes.as_mut_ptr().cast(),
            mDataLength: color_bytes.len() as u32,
            ..Default::default()
        };
        let emissive = sys::aiMaterialProperty {
            mSemantic: TextureType::Emissive.to_semantic(),
            mData: emissive_bytes.as_mut_ptr().cast(),
            mDataLength: emissive_bytes.len() as u32,
            ..color
        };
        unsafe {
            with_material(
                &mut [
                    color,
                    emissive,
                    sys::aiMaterialProperty {
                        mIndex: 1,
                        ..emissive
                    },
                ],
                |material| {
                    assert_eq!(material.emission_texture(0).unwrap().path, "pbr.png");
                    assert_eq!(material.emissive_texture(0).unwrap().path, "pbr.png");
                    assert_eq!(material.emission_texture(1).unwrap().path, "gltf.png");
                    assert_eq!(material.emissive_texture(1).unwrap().path, "gltf.png");
                    assert!(material.emission_texture(2).is_none());
                    assert!(material.texture(TextureType::EmissionColor, 1).is_none());
                    assert_eq!(
                        material.texture(TextureType::Emissive, 0).unwrap().path,
                        "gltf.png"
                    );
                },
            );
        }
    }

    #[test]
    fn decode_ai_string_rejects_invalid_lengths_and_missing_terminators() {
        for (length, payload) in [
            (999u32, &b"abc"[..]),
            (3, &b"abc"[..]),
            (3, &b"abc!"[..]),
            (2, &b"abc\0"[..]),
            (sys::AI_MAXLEN, &b"\0"[..]),
        ] {
            let mut bytes = length.to_ne_bytes().to_vec();
            bytes.extend_from_slice(payload);
            let (prop, _data_owner) = make_prop_with_data(bytes);
            assert!(
                unsafe { MaterialPropertyData::from_sys(&prop) }
                    .and_then(|data| data.decode_ai_string())
                    .is_none()
            );
        }
    }

    #[test]
    fn read_ne_primitives_from_bytes() {
        let mut data = Vec::new();
        data.extend_from_slice(&42i32.to_ne_bytes());
        data.extend_from_slice(&1.0f32.to_ne_bytes());
        data.extend_from_slice(&2.0f32.to_ne_bytes());
        data.extend_from_slice(&3.5f32.to_ne_bytes());
        data.extend_from_slice(&9.0f64.to_ne_bytes());

        let (prop, _data_owner) = make_prop_with_data(data);
        let d = unsafe { MaterialPropertyData::from_sys(&prop) }.unwrap();

        assert_eq!(d.read_ne_i32(0), Some(42));
        assert_eq!(d.read_ne_f32(4), Some(1.0));
        assert_eq!(d.read_ne_f32(8), Some(2.0));
        assert_eq!(d.read_ne_f32(12), Some(3.5));
        assert_eq!(d.read_ne_f64(4 + 12), Some(9.0));
        assert_eq!(d.read_ne_u32(9999), None);
    }

    #[test]
    fn material_property_ref_rejects_unaligned_pointers() {
        let scene = Scene::from_memory(b"o tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n", Some("obj"))
            .expect("import OBJ scene");
        let buf = [0u64; 8];
        let unaligned =
            unsafe { (buf.as_ptr() as *const u8).add(1) } as *const sys::aiMaterialProperty;

        assert!(unsafe { MaterialPropertyRef::from_ptr(scene, unaligned) }.is_none());
    }

    #[test]
    fn texture_scale_and_strength_read_texture_scoped_float_properties() {
        let scene = Scene::from_memory(b"o tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n", Some("obj"))
            .expect("import OBJ scene");

        let mut scale_data = 0.42f32.to_ne_bytes();
        let mut strength_data = 0.73f32.to_ne_bytes();
        let mut scale_prop = sys::aiMaterialProperty {
            mKey: ai_string_from_cstr(material_keys::TEXTURE_SCALE),
            mSemantic: TextureType::Normals.to_semantic(),
            mIndex: 0,
            mDataLength: scale_data.len() as u32,
            mType: sys::aiPropertyTypeInfo::aiPTI_Float,
            mData: scale_data.as_mut_ptr().cast(),
        };
        let mut strength_prop = sys::aiMaterialProperty {
            mKey: ai_string_from_cstr(material_keys::TEXTURE_STRENGTH),
            mSemantic: TextureType::Lightmap.to_semantic(),
            mIndex: 1,
            mDataLength: strength_data.len() as u32,
            mType: sys::aiPropertyTypeInfo::aiPTI_Float,
            mData: strength_data.as_mut_ptr().cast(),
        };
        let mut props = [
            &mut scale_prop as *mut sys::aiMaterialProperty,
            &mut strength_prop as *mut sys::aiMaterialProperty,
        ];
        let mat = sys::aiMaterial {
            mProperties: props.as_mut_ptr(),
            mNumProperties: props.len() as u32,
            mNumAllocated: props.len() as u32,
        };
        let material = Material {
            scene,
            material_ptr: unsafe { SharedPtr::new(&mat as *const sys::aiMaterial) }.unwrap(),
        };

        assert_eq!(material.normal_texture_scale(0), Some(0.42));
        assert_eq!(material.occlusion_texture_strength(1), Some(0.73));
        assert_eq!(material.occlusion_texture_strength(0), None);
        assert_eq!(material.texture_scale(TextureType::Diffuse, 0), None);
        assert_eq!(
            material.texture_strength(TextureType::Lightmap, usize::MAX),
            None
        );
    }
}

/// Blend mode for material layers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Default blending mode (usually alpha blending)
    Default,
    /// Additive blending mode
    Additive,
    /// Unknown blend mode with raw value
    Unknown(u32),
}

impl BlendMode {
    fn from_raw(v: u32) -> Self {
        match v {
            x if x == sys::aiBlendMode::aiBlendMode_Default.0 as u32 => BlendMode::Default,
            x if x == sys::aiBlendMode::aiBlendMode_Additive.0 as u32 => BlendMode::Additive,
            other => BlendMode::Unknown(other),
        }
    }
}

/// Which PBR workflow this material uses (heuristic from material.h docs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PbrWorkflow {
    /// Metallic-roughness PBR workflow (glTF 2.0 standard)
    MetallicRoughness,
    /// Specular-glossiness PBR workflow (legacy)
    SpecularGlossiness,
    /// Unknown or undetected PBR workflow
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
    /// Get base color texture at the specified index
    pub fn base_color_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::BaseColor, index)
    }

    /// Get the metallic-roughness texture (glTF packed format)
    pub fn metallic_roughness_texture(&self) -> Option<TextureInfo> {
        // glTF packed metallic-roughness (one texture, index 0)
        self.texture(TextureType::GltfMetallicRoughness, 0)
    }

    /// Get the emission texture, preferring `EmissionColor` over the glTF `Emissive` slot.
    pub fn emission_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::EmissionColor, index)
            .or_else(|| self.texture(TextureType::Emissive, index))
    }

    /// Get normal map texture at the specified index
    pub fn normal_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::Normals, index)
    }

    /// Get sheen color texture
    pub fn sheen_color_texture(&self) -> Option<TextureInfo> {
        // sheen color texture is TextureType::Sheen, index 0
        self.texture(TextureType::Sheen, 0)
    }

    /// Get sheen roughness texture
    pub fn sheen_roughness_texture(&self) -> Option<TextureInfo> {
        // sheen roughness texture is TextureType::Sheen, index 1
        self.texture(TextureType::Sheen, 1)
    }

    /// Get clearcoat texture
    pub fn clearcoat_texture(&self) -> Option<TextureInfo> {
        self.texture(TextureType::Clearcoat, 0)
    }

    /// Get clearcoat roughness texture
    pub fn clearcoat_roughness_texture(&self) -> Option<TextureInfo> {
        self.texture(TextureType::Clearcoat, 1)
    }

    /// Get clearcoat normal map texture
    pub fn clearcoat_normal_texture(&self) -> Option<TextureInfo> {
        self.texture(TextureType::Clearcoat, 2)
    }

    /// Get transmission texture
    pub fn transmission_texture(&self) -> Option<TextureInfo> {
        self.texture(TextureType::Transmission, 0)
    }

    /// Get volume thickness texture
    pub fn volume_thickness_texture(&self) -> Option<TextureInfo> {
        // Defined to use aiTextureType_TRANSMISSION, index 1
        self.texture(TextureType::Transmission, 1)
    }

    /// Get anisotropy texture
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

    /// glTF occlusion texture.
    ///
    /// Assimp imports glTF `occlusionTexture` as a lightmap texture.
    pub fn occlusion_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::Lightmap, index)
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

    /// Alias of [`Self::emission_texture`], with the same texture slot precedence.
    pub fn emissive_texture(&self, index: usize) -> Option<TextureInfo> {
        self.emission_texture(index)
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
    /// Other mapping mode explicitly selected by Assimp, retaining its raw value.
    Other(u32),
    /// Unknown mapping mode with its raw value.
    Unknown(u32),
}

impl TextureMapping {
    fn from_raw(value: u32) -> Self {
        match value {
            v if v == sys::aiTextureMapping::aiTextureMapping_UV.0 as u32 => Self::UV,
            v if v == sys::aiTextureMapping::aiTextureMapping_SPHERE.0 as u32 => Self::Sphere,
            v if v == sys::aiTextureMapping::aiTextureMapping_CYLINDER.0 as u32 => Self::Cylinder,
            v if v == sys::aiTextureMapping::aiTextureMapping_BOX.0 as u32 => Self::Box,
            v if v == sys::aiTextureMapping::aiTextureMapping_PLANE.0 as u32 => Self::Plane,
            v if v == sys::aiTextureMapping::aiTextureMapping_OTHER.0 as u32 => Self::Other(v),
            other => Self::Unknown(other),
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
    /// Legacy representation of an unrecognized operation.
    Other(u32),
    /// Unknown operation with its raw value.
    Unknown(u32),
}

impl TextureOperation {
    fn from_raw(value: u32) -> Self {
        match value {
            v if v == sys::aiTextureOp::aiTextureOp_Multiply.0 as u32 => Self::Multiply,
            v if v == sys::aiTextureOp::aiTextureOp_Add.0 as u32 => Self::Add,
            v if v == sys::aiTextureOp::aiTextureOp_Subtract.0 as u32 => Self::Subtract,
            v if v == sys::aiTextureOp::aiTextureOp_Divide.0 as u32 => Self::Divide,
            v if v == sys::aiTextureOp::aiTextureOp_SmoothAdd.0 as u32 => Self::SmoothAdd,
            v if v == sys::aiTextureOp::aiTextureOp_SignedAdd.0 as u32 => Self::SignedAdd,
            other => Self::Unknown(other),
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
    /// Legacy representation of an unrecognized texture map mode.
    Other(u32),
    /// Unknown texture map mode with its raw value.
    Unknown(u32),
}

impl TextureMapMode {
    fn from_raw(value: u32) -> Self {
        match value {
            v if v == sys::aiTextureMapMode::aiTextureMapMode_Wrap.0 as u32 => Self::Wrap,
            v if v == sys::aiTextureMapMode::aiTextureMapMode_Clamp.0 as u32 => Self::Clamp,
            v if v == sys::aiTextureMapMode::aiTextureMapMode_Mirror.0 as u32 => Self::Mirror,
            v if v == sys::aiTextureMapMode::aiTextureMapMode_Decal.0 as u32 => Self::Decal,
            other => Self::Unknown(other),
        }
    }
}

/// Information about a texture applied to a material
#[derive(Debug, Clone)]
pub struct TextureInfoRef {
    path: sys::aiString,
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

impl TextureInfoRef {
    /// Texture path as UTF-8 (lossy), without allocation.
    pub fn path_str(&self) -> Cow<'_, str> {
        ai_string_to_str(&self.path)
    }

    /// Raw bytes of the path (without assuming NUL-termination).
    pub fn path_bytes(&self) -> &[u8] {
        let len = (self.path.length as usize).min(self.path.data.len());
        unsafe { ffi::slice_from_ptr_len(self, self.path.data.as_ptr() as *const u8, len) }
    }

    /// Borrow the underlying Assimp `aiString`.
    #[cfg(feature = "raw-sys")]
    pub fn path_raw(&self) -> &sys::aiString {
        &self.path
    }

    /// Convert into an owned `TextureInfo` (allocates for the path string).
    pub fn into_owned(self) -> TextureInfo {
        TextureInfo {
            path: ai_string_to_string(&self.path),
            mapping: self.mapping,
            uv_index: self.uv_index,
            blend_factor: self.blend_factor,
            operation: self.operation,
            map_modes: self.map_modes,
            flags: self.flags,
            uv_transform: self.uv_transform,
            axis: self.axis,
        }
    }

    /// Convert into an owned `TextureInfo` (allocates for the path string).
    pub fn to_owned(&self) -> TextureInfo {
        self.clone().into_owned()
    }
}

/// Owned information about a texture applied to a material.
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
    /// Translation offset for UV coordinates
    pub translation: Vector2D,
    /// Scaling factor for UV coordinates
    pub scaling: Vector2D,
    /// Rotation angle in radians
    pub rotation: f32,
}

bitflags::bitflags! {
    /// Texture flags (material.h: aiTextureFlags)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TextureFlags: u32 {
        /// Invert the texture colors
        const INVERT        = sys::aiTextureFlags::aiTextureFlags_Invert.0 as u32;
        /// Use the alpha channel of the texture
        const USE_ALPHA     = sys::aiTextureFlags::aiTextureFlags_UseAlpha.0 as u32;
        /// Ignore the alpha channel of the texture
        const IGNORE_ALPHA  = sys::aiTextureFlags::aiTextureFlags_IgnoreAlpha.0 as u32;
    }
}

// Auto-traits (Send/Sync) are derived from the contained pointers and lifetimes.
