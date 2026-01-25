//! Metadata support for scenes and nodes
//!
//! This module provides safe Rust wrappers around Assimp's metadata functionality,
//! allowing you to access additional information stored in 3D models.

use crate::{
    error::Result,
    ffi, sys,
    types::{Vector3D, ai_string_to_string},
};

/// Decode `aiMetadataEntry::mData` without assuming alignment.
///
/// Note: `aiMetadataEntry` does not carry a length, so decoding is inherently unsafe: callers must
/// trust Assimp to provide a valid pointer for the requested type. We still centralize reads here
/// to reduce `unsafe` surface and avoid UB-prone patterns (e.g. decoding `bool` directly).
struct MetadataEntryData {
    ptr: *const u8,
}

impl MetadataEntryData {
    /// # Safety
    /// `entry.mData` must be a valid pointer for the corresponding metadata type.
    unsafe fn from_entry(entry: &sys::aiMetadataEntry) -> Result<Self> {
        if entry.mData.is_null() {
            return Err(crate::error::Error::invalid_parameter(
                "Null metadata data".to_string(),
            ));
        }
        Ok(Self {
            ptr: entry.mData.cast::<u8>(),
        })
    }

    /// # Safety
    /// `self.ptr` must be valid for `N` bytes.
    unsafe fn read_bytes<const N: usize>(&self) -> [u8; N] {
        let mut out = [0u8; N];
        unsafe { std::ptr::copy_nonoverlapping(self.ptr, out.as_mut_ptr(), N) };
        out
    }

    /// # Safety
    /// `self.ptr` must point to a valid `T` value in native endianness.
    unsafe fn read_ne_i32(&self) -> i32 {
        i32::from_ne_bytes(unsafe { self.read_bytes::<4>() })
    }

    /// # Safety
    /// `self.ptr` must point to a valid `T` value in native endianness.
    unsafe fn read_ne_u32(&self) -> u32 {
        u32::from_ne_bytes(unsafe { self.read_bytes::<4>() })
    }

    /// # Safety
    /// `self.ptr` must point to a valid `T` value in native endianness.
    unsafe fn read_ne_i64(&self) -> i64 {
        i64::from_ne_bytes(unsafe { self.read_bytes::<8>() })
    }

    /// # Safety
    /// `self.ptr` must point to a valid `T` value in native endianness.
    unsafe fn read_ne_u64(&self) -> u64 {
        u64::from_ne_bytes(unsafe { self.read_bytes::<8>() })
    }

    /// # Safety
    /// `self.ptr` must point to a valid `T` value in native endianness.
    unsafe fn read_ne_f32(&self) -> f32 {
        f32::from_ne_bytes(unsafe { self.read_bytes::<4>() })
    }

    /// # Safety
    /// `self.ptr` must point to a valid `T` value in native endianness.
    unsafe fn read_ne_f64(&self) -> f64 {
        f64::from_ne_bytes(unsafe { self.read_bytes::<8>() })
    }

    /// # Safety
    /// `self.ptr` must point to a valid `aiString` value.
    unsafe fn read_ai_string(&self) -> sys::aiString {
        let mut out = std::mem::MaybeUninit::<sys::aiString>::uninit();
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.ptr,
                out.as_mut_ptr().cast::<u8>(),
                std::mem::size_of::<sys::aiString>(),
            );
            out.assume_init()
        }
    }

    /// # Safety
    /// `self.ptr` must point to a valid `aiVector3D`-compatible layout.
    unsafe fn read_vector3d(&self) -> Vector3D {
        let bytes = unsafe { self.read_bytes::<12>() };
        let x = f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let y = f32::from_ne_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let z = f32::from_ne_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        Vector3D::new(x, y, z)
    }

    /// # Safety
    /// `self.ptr` must point to a boolean value stored as a byte.
    unsafe fn read_bool_byte(&self) -> bool {
        unsafe { *self.ptr != 0 }
    }
}

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
    pub(crate) unsafe fn from_raw_sys(metadata_ptr: *const sys::aiMetadata) -> Result<Self> {
        if metadata_ptr.is_null() {
            return Ok(Self::new());
        }

        let metadata = unsafe { &*metadata_ptr };
        if metadata.mNumProperties > 0 && (metadata.mKeys.is_null() || metadata.mValues.is_null()) {
            return Ok(Self::new());
        }
        let mut entries = std::collections::HashMap::new();

        // Parse each metadata entry.
        let n = metadata.mNumProperties as usize;
        let keys = ffi::slice_from_ptr_len(metadata, metadata.mKeys, n);
        let values = ffi::slice_from_ptr_len(metadata, metadata.mValues, n);
        for (key_ai_string, entry) in keys.iter().zip(values.iter()) {
            let key = ai_string_to_string(key_ai_string);
            if key.is_empty() {
                continue;
            }
            if let Ok(entry) = unsafe { Self::parse_metadata_entry(entry) } {
                entries.insert(key, entry);
            }
        }

        Ok(Self { entries })
    }

    /// Create metadata from a raw Assimp metadata pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub unsafe fn from_raw(metadata_ptr: *const sys::aiMetadata) -> Result<Self> {
        unsafe { Self::from_raw_sys(metadata_ptr) }
    }

    /// Parse a single metadata entry
    unsafe fn parse_metadata_entry(entry: &sys::aiMetadataEntry) -> Result<MetadataEntry> {
        let data = unsafe { MetadataEntryData::from_entry(entry) }?;

        match entry.mType {
            sys::aiMetadataType::AI_BOOL => {
                // `aiMetadataEntry::mData` is an untyped pointer; do not assume alignment.
                //
                // Don't read a Rust `bool` directly: if a corrupted/malicious scene stores a value
                // other than 0/1, materializing it as `bool` is UB. Decode as a byte instead.
                Ok(MetadataEntry::Bool(unsafe { data.read_bool_byte() }))
            }
            sys::aiMetadataType::AI_INT32 => {
                Ok(MetadataEntry::Int32(unsafe { data.read_ne_i32() }))
            }
            sys::aiMetadataType::AI_UINT64 => {
                Ok(MetadataEntry::UInt64(unsafe { data.read_ne_u64() }))
            }
            sys::aiMetadataType::AI_FLOAT => {
                Ok(MetadataEntry::Float(unsafe { data.read_ne_f32() }))
            }
            sys::aiMetadataType::AI_DOUBLE => {
                Ok(MetadataEntry::Double(unsafe { data.read_ne_f64() }))
            }
            sys::aiMetadataType::AI_AISTRING => {
                let ai_string = unsafe { data.read_ai_string() };
                Ok(MetadataEntry::String(ai_string_to_string(&ai_string)))
            }
            sys::aiMetadataType::AI_AIVECTOR3D => {
                Ok(MetadataEntry::Vector3D(unsafe { data.read_vector3d() }))
            }
            sys::aiMetadataType::AI_AIMETADATA => {
                let nested_metadata =
                    unsafe { Self::from_raw_sys(entry.mData as *const sys::aiMetadata)? };
                Ok(MetadataEntry::Metadata(nested_metadata))
            }
            sys::aiMetadataType::AI_INT64 => {
                Ok(MetadataEntry::Int64(unsafe { data.read_ne_i64() }))
            }
            sys::aiMetadataType::AI_UINT32 => {
                Ok(MetadataEntry::UInt32(unsafe { data.read_ne_u32() }))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bool_is_robust_to_noncanonical_values() {
        let mut b0: u8 = 0;
        let entry0 = sys::aiMetadataEntry {
            mType: sys::aiMetadataType::AI_BOOL,
            mData: std::ptr::from_mut(&mut b0).cast::<std::ffi::c_void>(),
        };
        assert!(matches!(
            unsafe { Metadata::parse_metadata_entry(&entry0) }.unwrap(),
            MetadataEntry::Bool(false)
        ));

        let mut b1: u8 = 1;
        let entry1 = sys::aiMetadataEntry {
            mType: sys::aiMetadataType::AI_BOOL,
            mData: std::ptr::from_mut(&mut b1).cast::<std::ffi::c_void>(),
        };
        assert!(matches!(
            unsafe { Metadata::parse_metadata_entry(&entry1) }.unwrap(),
            MetadataEntry::Bool(true)
        ));

        // A corrupted scene could technically store other non-zero values; treat as true without UB.
        let mut b2: u8 = 2;
        let entry2 = sys::aiMetadataEntry {
            mType: sys::aiMetadataType::AI_BOOL,
            mData: std::ptr::from_mut(&mut b2).cast::<std::ffi::c_void>(),
        };
        assert!(matches!(
            unsafe { Metadata::parse_metadata_entry(&entry2) }.unwrap(),
            MetadataEntry::Bool(true)
        ));
    }

    #[test]
    fn parse_int32_allows_unaligned_data() {
        let mut buf = vec![0u8; 8];
        let offset = 1usize;
        buf[offset..offset + 4].copy_from_slice(&(-42i32).to_ne_bytes());

        let entry = sys::aiMetadataEntry {
            mType: sys::aiMetadataType::AI_INT32,
            mData: unsafe { buf.as_mut_ptr().add(offset) }.cast::<std::ffi::c_void>(),
        };
        assert!(matches!(
            unsafe { Metadata::parse_metadata_entry(&entry) }.unwrap(),
            MetadataEntry::Int32(-42)
        ));
    }

    #[test]
    fn parse_metadata_entry_rejects_null_data() {
        let entry = sys::aiMetadataEntry {
            mType: sys::aiMetadataType::AI_UINT32,
            mData: std::ptr::null_mut(),
        };
        assert!(unsafe { Metadata::parse_metadata_entry(&entry) }.is_err());
    }

    #[test]
    fn parse_vector3d_allows_unaligned_data() {
        let mut buf = vec![0u8; 32];
        let offset = 1usize;

        let x = 1.25f32.to_ne_bytes();
        let y = (-2.0f32).to_ne_bytes();
        let z = 3.5f32.to_ne_bytes();
        buf[offset..offset + 4].copy_from_slice(&x);
        buf[offset + 4..offset + 8].copy_from_slice(&y);
        buf[offset + 8..offset + 12].copy_from_slice(&z);

        let entry = sys::aiMetadataEntry {
            mType: sys::aiMetadataType::AI_AIVECTOR3D,
            mData: unsafe { buf.as_mut_ptr().add(offset) }.cast::<std::ffi::c_void>(),
        };
        assert!(matches!(
            unsafe { Metadata::parse_metadata_entry(&entry) }.unwrap(),
            MetadataEntry::Vector3D(v) if v == Vector3D::new(1.25, -2.0, 3.5)
        ));
    }

    #[test]
    fn parse_ai_string_allows_unaligned_data() {
        let mut s = sys::aiString {
            length: 3,
            data: [0; sys::AI_MAXLEN as usize],
        };
        s.data[0] = b'a' as std::os::raw::c_char;
        s.data[1] = b'b' as std::os::raw::c_char;
        s.data[2] = b'c' as std::os::raw::c_char;

        let size = std::mem::size_of::<sys::aiString>();
        let mut buf = vec![0u8; size + 8];
        let offset = 1usize;
        unsafe {
            std::ptr::copy_nonoverlapping(
                std::ptr::from_ref(&s).cast::<u8>(),
                buf.as_mut_ptr().add(offset),
                size,
            );
        }

        let entry = sys::aiMetadataEntry {
            mType: sys::aiMetadataType::AI_AISTRING,
            mData: unsafe { buf.as_mut_ptr().add(offset) }.cast::<std::ffi::c_void>(),
        };
        assert!(matches!(
            unsafe { Metadata::parse_metadata_entry(&entry) }.unwrap(),
            MetadataEntry::String(v) if v == "abc"
        ));
    }
}
