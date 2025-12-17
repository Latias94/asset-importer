//! Material representation and properties

#![allow(clippy::unnecessary_cast)]

use std::borrow::Cow;
use std::ffi::CStr;
use std::ffi::CString;
use std::marker::PhantomData;

use crate::raw;
use crate::{
    ptr::SharedPtr,
    sys,
    types::{Color3D, Color4D, Vector2D, Vector3D, ai_string_to_str, ai_string_to_string},
};

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
pub struct Material<'a> {
    material_ptr: SharedPtr<sys::aiMaterial>,
    _marker: PhantomData<&'a ()>,
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
        unsafe { std::slice::from_raw_parts(self.value.data.as_ptr() as *const u8, len) }
    }

    /// Borrow the underlying Assimp `aiString`.
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> &sys::aiString {
        &self.value
    }

    /// Convert to an owned `String` (allocates).
    pub fn to_string(&self) -> String {
        ai_string_to_string(&self.value)
    }
}

impl<'a> Material<'a> {
    /// Create a Material from a raw Assimp material pointer
    ///
    /// # Safety
    /// Caller must ensure `material_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(material_ptr: *const sys::aiMaterial) -> Self {
        debug_assert!(!material_ptr.is_null());
        let material_ptr = unsafe { SharedPtr::new_unchecked(material_ptr) };
        Self {
            material_ptr,
            _marker: PhantomData,
        }
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
        let mut ai_string = sys::aiString::default();

        let result = unsafe {
            sys::aiGetMaterialString(
                self.material_ptr.as_ptr(),
                key.as_ptr(),
                0, // type
                0, // index
                &mut ai_string,
            )
        };

        if result == sys::aiReturn::aiReturn_SUCCESS {
            Some(MaterialStringRef { value: ai_string })
        } else {
            None
        }
    }

    /// Get a string property from the material (allocates).
    pub fn get_string_property(&self, key: &CStr) -> Option<String> {
        self.get_string_property_ref(key).map(|s| s.to_string())
    }

    /// Get a string property from the material (allocates, convenience).
    pub fn get_string_property_str(&self, key: &str) -> Option<String> {
        let c_key = CString::new(key).ok()?;
        self.get_string_property(c_key.as_c_str())
    }

    /// Get a float property from the material
    pub fn get_float_property(&self, key: &CStr) -> Option<f32> {
        let mut value = 0.0f32;
        let mut max = 1u32;

        let result = unsafe {
            sys::aiGetMaterialFloatArray(
                self.material_ptr.as_ptr(),
                key.as_ptr(),
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

    /// Get a float property from the material (allocates, convenience).
    pub fn get_float_property_str(&self, key: &str) -> Option<f32> {
        let c_key = CString::new(key).ok()?;
        self.get_float_property(c_key.as_c_str())
    }

    /// Get an integer property from the material
    pub fn get_integer_property(&self, key: &CStr) -> Option<i32> {
        let mut value = 0i32;
        let mut max = 1u32;

        let result = unsafe {
            sys::aiGetMaterialIntegerArray(
                self.material_ptr.as_ptr(),
                key.as_ptr(),
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

    /// Get an integer property from the material (allocates, convenience).
    pub fn get_integer_property_str(&self, key: &str) -> Option<i32> {
        let c_key = CString::new(key).ok()?;
        self.get_integer_property(c_key.as_c_str())
    }

    /// Get a color property from the material
    pub fn get_color_property(&self, key: &CStr) -> Option<Color4D> {
        let mut color = sys::aiColor4D {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };

        let result = unsafe {
            sys::aiGetMaterialColor(
                self.material_ptr.as_ptr(),
                key.as_ptr(),
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

    /// Get a color property from the material (allocates, convenience).
    pub fn get_color_property_str(&self, key: &str) -> Option<Color4D> {
        let c_key = CString::new(key).ok()?;
        self.get_color_property(c_key.as_c_str())
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
        Some(MaterialPropertyRef::from_ptr(prop_ptr).into_info())
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
                self.material_ptr.as_ptr(),
                key.as_ptr(),
                semantic.map(|t| t.to_sys() as u32).unwrap_or(0),
                index,
                &mut prop_ptr,
            ) == sys::aiReturn::aiReturn_SUCCESS
        };
        (ok && !prop_ptr.is_null()).then_some(prop_ptr)
    }

    /// Get the raw bytes of a property (as stored by Assimp)
    pub fn get_property_raw_ref(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<&'a [u8]> {
        let prop_ptr = self.property_ptr(key, semantic, index)?;
        unsafe {
            let prop = &*prop_ptr;
            if prop.mData.is_null() || prop.mDataLength == 0 {
                Some(&[])
            } else {
                Some(std::slice::from_raw_parts(
                    prop.mData as *const u8,
                    prop.mDataLength as usize,
                ))
            }
        }
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

    /// Get an integer array property (converts from floats if necessary)
    pub fn get_property_i32_array(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<Vec<i32>> {
        let info = self.property_info(key, semantic, index)?;
        // Determine element count using the stored type size
        let elem_size = match info.type_info {
            PropertyTypeInfo::Integer => std::mem::size_of::<i32>(),
            PropertyTypeInfo::Float => std::mem::size_of::<f32>(),
            PropertyTypeInfo::Double => std::mem::size_of::<f64>(),
            _ => return None,
        };
        if elem_size == 0 {
            return None;
        }
        let count = (info.data_length as usize) / elem_size;
        let mut out = vec![0i32; count];
        let mut max = count as u32;
        let result = unsafe {
            sys::aiGetMaterialIntegerArray(
                self.material_ptr.as_ptr(),
                key.as_ptr(),
                semantic.map(|t| t.to_sys() as u32).unwrap_or(0),
                index,
                out.as_mut_ptr(),
                &mut max,
            )
        };
        if result == sys::aiReturn::aiReturn_SUCCESS {
            out.truncate(max as usize);
            Some(out)
        } else {
            None
        }
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

    /// Get a 32-bit float array property. If the property is stored as doubles, it is converted.
    pub fn get_property_f32_array(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<Vec<f32>> {
        let info = self.property_info(key, semantic, index)?;
        match info.type_info {
            PropertyTypeInfo::Float | PropertyTypeInfo::Double | PropertyTypeInfo::Integer => {
                // Use Assimp conversion for Float/Integer; for Double, we can also do a manual conversion for more precision.
                // First, try the API path using aiGetMaterialFloatArray.
                let elem_size = match info.type_info {
                    PropertyTypeInfo::Float => std::mem::size_of::<f32>(),
                    PropertyTypeInfo::Double => std::mem::size_of::<f64>(),
                    PropertyTypeInfo::Integer => std::mem::size_of::<i32>(),
                    _ => unreachable!(),
                };
                let count = (info.data_length as usize) / elem_size;
                let mut out = vec![0f32; count];
                let mut max = count as u32;
                let result = unsafe {
                    sys::aiGetMaterialFloatArray(
                        self.material_ptr.as_ptr(),
                        key.as_ptr(),
                        semantic.map(|t| t.to_sys() as u32).unwrap_or(0),
                        index,
                        out.as_mut_ptr(),
                        &mut max,
                    )
                };
                if result == sys::aiReturn::aiReturn_SUCCESS {
                    out.truncate(max as usize);
                    return Some(out);
                }
                // Fallback: manual conversion from raw data
                self.get_property_f64_array(key, semantic, index)
                    .map(|v| v.into_iter().map(|x| x as f32).collect())
            }
            _ => None,
        }
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
    /// If stored as f32, it will be widened; if stored as i32, it will be cast.
    pub fn get_property_f64_array(
        &self,
        key: &CStr,
        semantic: Option<TextureType>,
        index: u32,
    ) -> Option<Vec<f64>> {
        let info = self.property_info(key, semantic, index)?;
        let raw = self.get_property_raw_ref(key, semantic, index)?;
        match info.type_info {
            PropertyTypeInfo::Double => {
                let sz = std::mem::size_of::<f64>();
                if sz == 0 || raw.len() % sz != 0 {
                    return None;
                }
                let mut out = Vec::with_capacity(raw.len() / sz);
                for chunk in raw.chunks_exact(sz) {
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(chunk);
                    out.push(f64::from_ne_bytes(arr));
                }
                Some(out)
            }
            PropertyTypeInfo::Float => {
                let sz = std::mem::size_of::<f32>();
                if sz == 0 || raw.len() % sz != 0 {
                    return None;
                }
                let mut out = Vec::with_capacity(raw.len() / sz);
                for chunk in raw.chunks_exact(sz) {
                    let mut arr = [0u8; 4];
                    arr.copy_from_slice(chunk);
                    out.push(f32::from_ne_bytes(arr) as f64);
                }
                Some(out)
            }
            PropertyTypeInfo::Integer => {
                let sz = std::mem::size_of::<i32>();
                if sz == 0 || raw.len() % sz != 0 {
                    return None;
                }
                let mut out = Vec::with_capacity(raw.len() / sz);
                for chunk in raw.chunks_exact(sz) {
                    let mut arr = [0u8; 4];
                    arr.copy_from_slice(chunk);
                    out.push(i32::from_ne_bytes(arr) as f64);
                }
                Some(out)
            }
            _ => None,
        }
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
        self.properties()
            .map(MaterialPropertyRef::into_info)
            .collect()
    }

    /// Iterate all material properties (zero allocation for keys and raw data).
    pub fn properties(&self) -> MaterialPropertyIterator<'a> {
        unsafe {
            let m = &*self.material_ptr.as_ptr();
            MaterialPropertyIterator {
                props: SharedPtr::new(m.mProperties as *const *mut sys::aiMaterialProperty),
                count: m.mNumProperties as usize,
                index: 0,
                _marker: PhantomData,
            }
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

    /// Get the number of textures for a specific type
    pub fn texture_count(&self, texture_type: TextureType) -> usize {
        unsafe {
            sys::aiGetMaterialTextureCount(self.material_ptr.as_ptr(), texture_type.to_sys())
                as usize
        }
    }

    /// Get texture information for a specific type and index (no heap allocation).
    pub fn texture_ref(&self, texture_type: TextureType, index: usize) -> Option<TextureInfoRef> {
        if index >= self.texture_count(texture_type) {
            return None;
        }

        unsafe {
            let mut path = sys::aiString::default();
            let mut mapping = std::mem::MaybeUninit::<sys::aiTextureMapping>::uninit();
            let mut uv_index = std::mem::MaybeUninit::<u32>::uninit();
            let mut blend = std::mem::MaybeUninit::<f32>::uninit();
            let mut op = std::mem::MaybeUninit::<sys::aiTextureOp>::uninit();
            // Use the exact sys enum type to avoid platform-dependent
            // signedness mismatches across compilers.
            let mut map_mode: [sys::aiTextureMapMode; 3] =
                [sys::aiTextureMapMode::aiTextureMapMode_Wrap; 3];
            let mut tex_flags: u32 = 0;

            let result = sys::aiGetMaterialTexture(
                self.material_ptr.as_ptr(),
                texture_type.to_sys(),
                index as u32,
                &mut path,
                mapping.as_mut_ptr(),
                uv_index.as_mut_ptr(),
                blend.as_mut_ptr(),
                op.as_mut_ptr(),
                map_mode.as_mut_ptr() as *mut _,
                &mut tex_flags as *mut u32,
            );

            if result != sys::aiReturn::aiReturn_SUCCESS {
                return None;
            }

            let mapping_val = mapping.assume_init();
            let uv_index_val = uv_index.assume_init();
            let blend_val = blend.assume_init();
            let op_val = op.assume_init();

            // Try read UV transform
            let mut uv_transform = std::mem::MaybeUninit::<sys::aiUVTransform>::uninit();
            let uv_key = CStr::from_bytes_with_nul_unchecked(b"$tex.uvtrafo\0");
            let uv_ok = sys::aiGetMaterialUVTransform(
                self.material_ptr.as_ptr(),
                uv_key.as_ptr(),
                texture_type.to_sys() as u32,
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
                let key = CStr::from_bytes_with_nul_unchecked(b"$tex.mapaxis\0");
                let mut prop_ptr: *const sys::aiMaterialProperty = std::ptr::null();
                let ok = sys::aiGetMaterialProperty(
                    self.material_ptr.as_ptr(),
                    key.as_ptr(),
                    texture_type.to_sys() as u32,
                    index as u32,
                    &mut prop_ptr,
                ) == sys::aiReturn::aiReturn_SUCCESS;
                if ok && !prop_ptr.is_null() {
                    let prop = &*prop_ptr;
                    if prop.mData.is_null()
                        || prop.mDataLength < std::mem::size_of::<raw::AiVector3D>() as u32
                    {
                        None
                    } else {
                        // `aiMaterialProperty::mData` is a byte blob; do not assume alignment.
                        let v = std::ptr::read_unaligned(prop.mData as *const raw::AiVector3D);
                        Some(Vector3D::new(v.x, v.y, v.z))
                    }
                } else {
                    None
                }
            };

            Some(TextureInfoRef {
                path,
                mapping: TextureMapping::from_raw(mapping_val),
                uv_index: uv_index_val,
                blend_factor: blend_val,
                operation: TextureOperation::from_raw(op_val),
                map_modes: [
                    TextureMapMode::from_raw(map_mode[0]),
                    TextureMapMode::from_raw(map_mode[1]),
                    TextureMapMode::from_raw(map_mode[2]),
                ],
                flags: TextureFlags::from_bits_truncate(tex_flags),
                uv_transform,
                axis,
            })
        }
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
    /// PBR specular-glossiness workflow
    PbrSpecularGlossiness,
    /// PBR metallic-roughness workflow
    PbrMetallicRoughness,
    /// Unknown shading model with raw value
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
            other => Self::Unknown(other as u32),
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
    fn from_ref(p: MaterialPropertyRef<'_>) -> Self {
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
#[derive(Debug, Clone, Copy)]
pub struct MaterialPropertyRef<'a> {
    prop_ptr: SharedPtr<sys::aiMaterialProperty>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> MaterialPropertyRef<'a> {
    fn from_ptr(prop_ptr: *const sys::aiMaterialProperty) -> Self {
        debug_assert!(!prop_ptr.is_null());
        let prop_ptr = unsafe { SharedPtr::new_unchecked(prop_ptr) };
        Self {
            prop_ptr,
            _marker: PhantomData,
        }
    }

    /// Property key as UTF-8 (lossy), without allocation.
    pub fn key_str(&self) -> Cow<'_, str> {
        unsafe { ai_string_to_str(&(*self.prop_ptr.as_ptr()).mKey) }
    }

    /// Raw bytes of the key (without assuming NUL-termination).
    pub fn key_bytes(&self) -> &[u8] {
        unsafe {
            let s = &(*self.prop_ptr.as_ptr()).mKey;
            let len = (s.length as usize).min(s.data.len());
            std::slice::from_raw_parts(s.data.as_ptr() as *const u8, len)
        }
    }

    /// Property key as owned `String` (allocates).
    pub fn key_string(&self) -> String {
        unsafe { ai_string_to_string(&(*self.prop_ptr.as_ptr()).mKey) }
    }

    /// Semantic (texture type) if texture-related.
    pub fn semantic(&self) -> Option<TextureType> {
        unsafe { TextureType::from_u32((*self.prop_ptr.as_ptr()).mSemantic) }
    }

    /// Texture index (0 for non-texture properties).
    pub fn index(&self) -> u32 {
        unsafe { (*self.prop_ptr.as_ptr()).mIndex }
    }

    /// Property type info.
    pub fn type_info(&self) -> PropertyTypeInfo {
        unsafe { PropertyTypeInfo::from_sys((*self.prop_ptr.as_ptr()).mType) }
    }

    /// Raw property bytes as stored by Assimp (zero-copy).
    pub fn data(&self) -> &'a [u8] {
        unsafe {
            let p = &*self.prop_ptr.as_ptr();
            if p.mData.is_null() || p.mDataLength == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(p.mData as *const u8, p.mDataLength as usize)
            }
        }
    }

    /// Interpret the property payload as an `i32` slice when stored as `Integer` (zero-copy).
    pub fn data_i32(&self) -> Option<&'a [i32]> {
        (self.type_info() == PropertyTypeInfo::Integer)
            .then(|| self.data_cast_slice_opt())
            .flatten()
    }

    /// Interpret the property payload as an `f32` slice when stored as `Float` (zero-copy).
    pub fn data_f32(&self) -> Option<&'a [f32]> {
        (self.type_info() == PropertyTypeInfo::Float)
            .then(|| self.data_cast_slice_opt())
            .flatten()
    }

    /// Interpret the property payload as an `f64` slice when stored as `Double` (zero-copy).
    pub fn data_f64(&self) -> Option<&'a [f64]> {
        (self.type_info() == PropertyTypeInfo::Double)
            .then(|| self.data_cast_slice_opt())
            .flatten()
    }

    fn data_cast_slice_opt<T>(&self) -> Option<&'a [T]> {
        unsafe {
            let p = &*self.prop_ptr.as_ptr();
            let len = p.mDataLength as usize;
            let size = std::mem::size_of::<T>();
            let align = std::mem::align_of::<T>();

            if len == 0 {
                return Some(&[]);
            }
            if p.mData.is_null() {
                return None;
            }

            let ptr = p.mData as *const u8;
            if size == 0 {
                return Some(&[]);
            }
            if (ptr as usize) % align != 0 || len % size != 0 {
                return None;
            }
            Some(std::slice::from_raw_parts(ptr as *const T, len / size))
        }
    }

    fn into_info(self) -> MaterialPropertyInfo {
        MaterialPropertyInfo::from_ref(self)
    }
}

/// Iterator over material properties (skips null entries).
pub struct MaterialPropertyIterator<'a> {
    props: Option<SharedPtr<*mut sys::aiMaterialProperty>>,
    count: usize,
    index: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Iterator for MaterialPropertyIterator<'a> {
    type Item = MaterialPropertyRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let props = self.props?;
        while self.index < self.count {
            unsafe {
                let ptr = *props.as_ptr().add(self.index);
                self.index += 1;
                if ptr.is_null() {
                    continue;
                }
                return Some(MaterialPropertyRef::from_ptr(ptr));
            }
        }
        None
    }
}

impl TextureType {
    #[inline]
    fn to_sys(self) -> sys::aiTextureType {
        // Our discriminants are defined from sys::aiTextureType constants,
        // so this cast is safe for all valid variants of TextureType.
        unsafe { std::mem::transmute(self as u32) }
    }

    /// Try convert a raw u32 (aiTextureType) into TextureType safely
    pub fn from_u32(v: u32) -> Option<Self> {
        Some(match v {
            x if x == sys::aiTextureType::aiTextureType_DIFFUSE as u32 => Self::Diffuse,
            x if x == sys::aiTextureType::aiTextureType_SPECULAR as u32 => Self::Specular,
            x if x == sys::aiTextureType::aiTextureType_AMBIENT as u32 => Self::Ambient,
            x if x == sys::aiTextureType::aiTextureType_EMISSIVE as u32 => Self::Emissive,
            x if x == sys::aiTextureType::aiTextureType_HEIGHT as u32 => Self::Height,
            x if x == sys::aiTextureType::aiTextureType_NORMALS as u32 => Self::Normals,
            x if x == sys::aiTextureType::aiTextureType_SHININESS as u32 => Self::Shininess,
            x if x == sys::aiTextureType::aiTextureType_OPACITY as u32 => Self::Opacity,
            x if x == sys::aiTextureType::aiTextureType_DISPLACEMENT as u32 => Self::Displacement,
            x if x == sys::aiTextureType::aiTextureType_LIGHTMAP as u32 => Self::Lightmap,
            x if x == sys::aiTextureType::aiTextureType_REFLECTION as u32 => Self::Reflection,
            x if x == sys::aiTextureType::aiTextureType_BASE_COLOR as u32 => Self::BaseColor,
            x if x == sys::aiTextureType::aiTextureType_NORMAL_CAMERA as u32 => Self::NormalCamera,
            x if x == sys::aiTextureType::aiTextureType_EMISSION_COLOR as u32 => {
                Self::EmissionColor
            }
            x if x == sys::aiTextureType::aiTextureType_METALNESS as u32 => Self::Metalness,
            x if x == sys::aiTextureType::aiTextureType_DIFFUSE_ROUGHNESS as u32 => {
                Self::DiffuseRoughness
            }
            x if x == sys::aiTextureType::aiTextureType_AMBIENT_OCCLUSION as u32 => {
                Self::AmbientOcclusion
            }
            x if x == sys::aiTextureType::aiTextureType_UNKNOWN as u32 => Self::Unknown,
            x if x == sys::aiTextureType::aiTextureType_SHEEN as u32 => Self::Sheen,
            x if x == sys::aiTextureType::aiTextureType_CLEARCOAT as u32 => Self::Clearcoat,
            x if x == sys::aiTextureType::aiTextureType_TRANSMISSION as u32 => Self::Transmission,
            x if x == sys::aiTextureType::aiTextureType_MAYA_BASE as u32 => Self::MayaBase,
            x if x == sys::aiTextureType::aiTextureType_MAYA_SPECULAR as u32 => Self::MayaSpecular,
            x if x == sys::aiTextureType::aiTextureType_MAYA_SPECULAR_COLOR as u32 => {
                Self::MayaSpecularColor
            }
            x if x == sys::aiTextureType::aiTextureType_MAYA_SPECULAR_ROUGHNESS as u32 => {
                Self::MayaSpecularRoughness
            }
            x if x == sys::aiTextureType::aiTextureType_ANISOTROPY as u32 => Self::Anisotropy,
            x if x == sys::aiTextureType::aiTextureType_GLTF_METALLIC_ROUGHNESS as u32 => {
                Self::GltfMetallicRoughness
            }
            _ => return None,
        })
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
            x if x == sys::aiBlendMode::aiBlendMode_Default as u32 => BlendMode::Default,
            x if x == sys::aiBlendMode::aiBlendMode_Additive as u32 => BlendMode::Additive,
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

impl<'a> Material<'a> {
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

    /// Get emission texture at the specified index
    pub fn emission_texture(&self, index: usize) -> Option<TextureInfo> {
        self.texture(TextureType::EmissionColor, index)
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
    fn from_raw(value: sys::aiTextureMapping) -> Self {
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
    fn from_raw(value: sys::aiTextureOp) -> Self {
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
    fn from_raw(value: sys::aiTextureMapMode) -> Self {
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
        unsafe { std::slice::from_raw_parts(self.path.data.as_ptr() as *const u8, len) }
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
        const INVERT        = sys::aiTextureFlags::aiTextureFlags_Invert as u32;
        /// Use the alpha channel of the texture
        const USE_ALPHA     = sys::aiTextureFlags::aiTextureFlags_UseAlpha as u32;
        /// Ignore the alpha channel of the texture
        const IGNORE_ALPHA  = sys::aiTextureFlags::aiTextureFlags_IgnoreAlpha as u32;
    }
}

// Auto-traits (Send/Sync) are derived from the contained pointers and lifetimes.
