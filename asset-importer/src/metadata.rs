//! Metadata support for scenes and nodes
//!
//! This module provides safe Rust wrappers around Assimp's metadata functionality,
//! allowing you to access additional information stored in 3D models.

use crate::{
    error::Result,
    sys,
    types::Vector3D,
};

/// Common metadata keys used across different file formats
pub mod common_metadata {
    /// Scene metadata holding the name of the importer which loaded the source asset.
    /// This is always present if the scene was created from an imported asset.
    pub const SOURCE_FORMAT: &str = "SourceAsset_Format";

    /// Scene metadata holding the version of the source asset as a string, if available.
    /// Not all formats add this metadata.
    pub const SOURCE_FORMAT_VERSION: &str = "SourceAsset_FormatVersion";

    /// Scene metadata holding the name of the software which generated the source asset, if available.
    /// Not all formats add this metadata.
    pub const SOURCE_GENERATOR: &str = "SourceAsset_Generator";

    /// Scene metadata holding the source asset copyright statement, if available.
    /// Not all formats add this metadata.
    pub const SOURCE_COPYRIGHT: &str = "SourceAsset_Copyright";
}

/// Collada-specific metadata keys
pub mod collada_metadata {
    /// Collada ID attribute
    pub const ID: &str = "Collada_id";

    /// Collada SID attribute
    pub const SID: &str = "Collada_sid";
}

/// Metadata type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataType {
    /// Boolean value
    Bool,
    /// 32-bit signed integer
    Int32,
    /// 64-bit unsigned integer
    UInt64,
    /// 32-bit floating point
    Float,
    /// 64-bit floating point
    Double,
    /// String value
    String,
    /// 3D vector
    Vector3D,
    /// Nested metadata
    Metadata,
    /// 64-bit signed integer
    Int64,
    /// 32-bit unsigned integer
    UInt32,
}

impl From<sys::aiMetadataType::Type> for MetadataType {
    fn from(value: sys::aiMetadataType::Type) -> Self {
        match value {
            sys::aiMetadataType::AI_BOOL => MetadataType::Bool,
            sys::aiMetadataType::AI_INT32 => MetadataType::Int32,
            sys::aiMetadataType::AI_UINT64 => MetadataType::UInt64,
            sys::aiMetadataType::AI_FLOAT => MetadataType::Float,
            sys::aiMetadataType::AI_DOUBLE => MetadataType::Double,
            sys::aiMetadataType::AI_AISTRING => MetadataType::String,
            sys::aiMetadataType::AI_AIVECTOR3D => MetadataType::Vector3D,
            sys::aiMetadataType::AI_AIMETADATA => MetadataType::Metadata,
            sys::aiMetadataType::AI_INT64 => MetadataType::Int64,
            sys::aiMetadataType::AI_UINT32 => MetadataType::UInt32,
            _ => MetadataType::String, // Default fallback
        }
    }
}

/// A metadata entry containing a typed value
#[derive(Debug, Clone)]
pub enum MetadataEntry {
    /// Boolean value
    Bool(bool),
    /// 32-bit signed integer
    Int32(i32),
    /// 64-bit unsigned integer
    UInt64(u64),
    /// 32-bit floating point
    Float(f32),
    /// 64-bit floating point
    Double(f64),
    /// String value
    String(String),
    /// 3D vector
    Vector3D(Vector3D),
    /// Nested metadata
    Metadata(Metadata),
    /// 64-bit signed integer
    Int64(i64),
    /// 32-bit unsigned integer
    UInt32(u32),
}

impl MetadataEntry {
    /// Get the type of this metadata entry
    pub fn metadata_type(&self) -> MetadataType {
        match self {
            MetadataEntry::Bool(_) => MetadataType::Bool,
            MetadataEntry::Int32(_) => MetadataType::Int32,
            MetadataEntry::UInt64(_) => MetadataType::UInt64,
            MetadataEntry::Float(_) => MetadataType::Float,
            MetadataEntry::Double(_) => MetadataType::Double,
            MetadataEntry::String(_) => MetadataType::String,
            MetadataEntry::Vector3D(_) => MetadataType::Vector3D,
            MetadataEntry::Metadata(_) => MetadataType::Metadata,
            MetadataEntry::Int64(_) => MetadataType::Int64,
            MetadataEntry::UInt32(_) => MetadataType::UInt32,
        }
    }

    /// Try to get this entry as a boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            MetadataEntry::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get this entry as an i32
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            MetadataEntry::Int32(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get this entry as a u64
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            MetadataEntry::UInt64(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get this entry as an f32
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            MetadataEntry::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get this entry as an f64
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            MetadataEntry::Double(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get this entry as a string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            MetadataEntry::String(v) => Some(v),
            _ => None,
        }
    }

    /// Try to get this entry as a Vector3D
    pub fn as_vector3d(&self) -> Option<&Vector3D> {
        match self {
            MetadataEntry::Vector3D(v) => Some(v),
            _ => None,
        }
    }

    /// Try to get this entry as nested metadata
    pub fn as_metadata(&self) -> Option<&Metadata> {
        match self {
            MetadataEntry::Metadata(v) => Some(v),
            _ => None,
        }
    }

    /// Try to get this entry as an i64
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            MetadataEntry::Int64(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get this entry as a u32
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            MetadataEntry::UInt32(v) => Some(*v),
            _ => None,
        }
    }
}

/// A collection of metadata entries
#[derive(Debug, Clone)]
pub struct Metadata {
    entries: std::collections::HashMap<String, MetadataEntry>,
}

impl Metadata {
    /// Create a new empty metadata collection
    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    /// Create metadata from a raw Assimp metadata pointer
    ///
    /// # Safety
    ///
    /// The caller must ensure that `metadata_ptr` is a valid pointer to an aiMetadata
    pub unsafe fn from_raw(metadata_ptr: *const sys::aiMetadata) -> Result<Self> {
        if metadata_ptr.is_null() {
            return Ok(Self::new());
        }

        let metadata = unsafe { &*metadata_ptr };
        let mut entries = std::collections::HashMap::new();

        // Parse each metadata entry
        for i in 0..metadata.mNumProperties {
            let key_ai_string = unsafe { &*metadata.mKeys.add(i as usize) };
            let entry_ptr = unsafe { metadata.mValues.add(i as usize) };

            if !entry_ptr.is_null() {
                let key_cstr = unsafe { std::ffi::CStr::from_ptr(key_ai_string.data.as_ptr()) };
                if let Ok(key) = key_cstr.to_str() {
                    if let Ok(entry) = unsafe { Self::parse_metadata_entry(&*entry_ptr) } {
                        entries.insert(key.to_string(), entry);
                    }
                }
            }
        }

        Ok(Self { entries })
    }

    /// Parse a single metadata entry
    unsafe fn parse_metadata_entry(entry: &sys::aiMetadataEntry) -> Result<MetadataEntry> {
        if entry.mData.is_null() {
            return Err(crate::error::Error::invalid_parameter(
                "Null metadata data".to_string(),
            ));
        }

        match entry.mType {
            sys::aiMetadataType::AI_BOOL => {
                let value = unsafe { *(entry.mData as *const bool) };
                Ok(MetadataEntry::Bool(value))
            }
            sys::aiMetadataType::AI_INT32 => {
                let value = unsafe { *(entry.mData as *const i32) };
                Ok(MetadataEntry::Int32(value))
            }
            sys::aiMetadataType::AI_UINT64 => {
                let value = unsafe { *(entry.mData as *const u64) };
                Ok(MetadataEntry::UInt64(value))
            }
            sys::aiMetadataType::AI_FLOAT => {
                let value = unsafe { *(entry.mData as *const f32) };
                Ok(MetadataEntry::Float(value))
            }
            sys::aiMetadataType::AI_DOUBLE => {
                let value = unsafe { *(entry.mData as *const f64) };
                Ok(MetadataEntry::Double(value))
            }
            sys::aiMetadataType::AI_AISTRING => {
                let ai_string = unsafe { &*(entry.mData as *const sys::aiString) };
                let c_str = unsafe { std::ffi::CStr::from_ptr(ai_string.data.as_ptr()) };
                let string = c_str.to_str().map_err(|_| {
                    crate::error::Error::invalid_parameter(
                        "Invalid UTF-8 in metadata string".to_string(),
                    )
                })?;
                Ok(MetadataEntry::String(string.to_string()))
            }
            sys::aiMetadataType::AI_AIVECTOR3D => {
                let vector = unsafe { &*(entry.mData as *const sys::aiVector3D) };
                Ok(MetadataEntry::Vector3D(crate::types::Vector3D::new(
                    vector.x, vector.y, vector.z,
                )))
            }
            sys::aiMetadataType::AI_AIMETADATA => {
                let nested_metadata =
                    unsafe { Self::from_raw(entry.mData as *const sys::aiMetadata)? };
                Ok(MetadataEntry::Metadata(nested_metadata))
            }
            sys::aiMetadataType::AI_INT64 => {
                let value = unsafe { *(entry.mData as *const i64) };
                Ok(MetadataEntry::Int64(value))
            }
            sys::aiMetadataType::AI_UINT32 => {
                let value = unsafe { *(entry.mData as *const u32) };
                Ok(MetadataEntry::UInt32(value))
            }
            _ => Err(crate::error::Error::invalid_parameter(
                "Unknown metadata type".to_string(),
            )),
        }
    }

    /// Get the number of metadata entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the metadata is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get a metadata entry by key
    pub fn get(&self, key: &str) -> Option<&MetadataEntry> {
        self.entries.get(key)
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// Get all values
    pub fn values(&self) -> impl Iterator<Item = &MetadataEntry> {
        self.entries.values()
    }

    /// Iterate over all key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&String, &MetadataEntry)> {
        self.entries.iter()
    }

    /// Get a boolean value by key
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key)?.as_bool()
    }

    /// Get an i32 value by key
    pub fn get_i32(&self, key: &str) -> Option<i32> {
        self.get(key)?.as_i32()
    }

    /// Get a u64 value by key
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.get(key)?.as_u64()
    }

    /// Get an f32 value by key
    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get(key)?.as_f32()
    }

    /// Get an f64 value by key
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key)?.as_f64()
    }

    /// Get a string value by key
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.get(key)?.as_string()
    }

    /// Get a Vector3D value by key
    pub fn get_vector3d(&self, key: &str) -> Option<&Vector3D> {
        self.get(key)?.as_vector3d()
    }

    /// Get nested metadata by key
    pub fn get_metadata(&self, key: &str) -> Option<&Metadata> {
        self.get(key)?.as_metadata()
    }

    /// Get an i64 value by key
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key)?.as_i64()
    }

    /// Get a u32 value by key
    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.get(key)?.as_u32()
    }

    /// Insert a new metadata entry
    pub fn insert<S: Into<String>>(&mut self, key: S, entry: MetadataEntry) {
        self.entries.insert(key.into(), entry);
    }

    /// Remove a metadata entry by key
    pub fn remove(&mut self, key: &str) -> Option<MetadataEntry> {
        self.entries.remove(key)
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new()
    }
}
