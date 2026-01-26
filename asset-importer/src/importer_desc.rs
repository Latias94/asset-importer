//! Importer description functionality
//!
//! This module provides access to information about available importers,
//! including their capabilities, supported file formats, and metadata.

#![allow(clippy::unnecessary_cast)]

use crate::{error::c_str_to_string_or_empty, ffi, sys};
use std::ffi::CString;

/// Flags indicating features common to many importers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImporterFlags {
    bits: u32,
}

impl ImporterFlags {
    /// Indicates that there is a textual encoding of the file format; and that it is supported.
    pub const SUPPORT_TEXT_FLAVOUR: Self = Self {
        bits: sys::aiImporterFlags::aiImporterFlags_SupportTextFlavour as u32,
    };

    /// Indicates that there is a binary encoding of the file format; and that it is supported.
    pub const SUPPORT_BINARY_FLAVOUR: Self = Self {
        bits: sys::aiImporterFlags::aiImporterFlags_SupportBinaryFlavour as u32,
    };

    /// Indicates that there is a compressed encoding of the file format; and that it is supported.
    pub const SUPPORT_COMPRESSED_FLAVOUR: Self = Self {
        bits: sys::aiImporterFlags::aiImporterFlags_SupportCompressedFlavour as u32,
    };

    /// Indicates that the importer reads only a very particular subset of the file format.
    /// This happens commonly for declarative or procedural formats which cannot easily be mapped to #aiScene
    pub const LIMITED_SUPPORT: Self = Self {
        bits: sys::aiImporterFlags::aiImporterFlags_LimitedSupport as u32,
    };

    /// Indicates that the importer is highly experimental and should be used with care.
    pub const EXPERIMENTAL: Self = Self {
        bits: sys::aiImporterFlags::aiImporterFlags_Experimental as u32,
    };

    /// Create empty flags
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Create flags from raw bits
    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    /// Get the raw bits
    pub const fn bits(&self) -> u32 {
        self.bits
    }

    /// Check if flags contain a specific flag
    pub const fn contains(&self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }

    /// Combine flags
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }
}

impl std::ops::BitOr for ImporterFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl std::ops::BitOrAssign for ImporterFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

/// Meta information about a particular importer
#[derive(Debug, Clone)]
pub struct ImporterDesc {
    /// Full name of the importer (i.e. Blender3D importer)
    pub name: String,

    /// Original author (left blank if unknown or whole assimp team)
    pub author: String,

    /// Current maintainer, left blank if unknown
    pub maintainer: String,

    /// Implementation comments, i.e. unimplemented features
    pub comments: String,

    /// Feature flags
    pub flags: ImporterFlags,

    /// Minimum format version supported by this importer
    pub min_major: u32,

    /// Maximum format version supported by this importer
    pub max_major: u32,

    /// Minimum format version supported by this importer
    pub min_minor: u32,

    /// Maximum format version supported by this importer
    pub max_minor: u32,

    /// List of file extensions this importer can handle
    pub file_extensions: Vec<String>,
}

impl ImporterDesc {
    /// Create from raw Assimp importer description
    pub(crate) fn from_raw(desc: &sys::aiImporterDesc) -> Self {
        let name = c_str_to_string_or_empty(desc.mName);
        let author = c_str_to_string_or_empty(desc.mAuthor);
        let maintainer = c_str_to_string_or_empty(desc.mMaintainer);
        let comments = c_str_to_string_or_empty(desc.mComments);

        let extensions_str = c_str_to_string_or_empty(desc.mFileExtensions);
        let file_extensions = extensions_str
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        Self {
            name,
            author,
            maintainer,
            comments,
            flags: ImporterFlags::from_bits(desc.mFlags),
            min_major: desc.mMinMajor,
            max_major: desc.mMaxMajor,
            min_minor: desc.mMinMinor,
            max_minor: desc.mMaxMinor,
            file_extensions,
        }
    }
}

/// Get importer description for a given file extension
///
/// # Arguments
/// * `extension` - File extension to look for (e.g., "obj", "fbx")
///
/// # Returns
/// * `Some(ImporterDesc)` if an importer is found for the extension
/// * `None` if no importer supports the extension
///
/// # Example
/// ```rust,no_run
/// use asset_importer::get_importer_desc;
///
/// if let Some(desc) = get_importer_desc("obj")? {
///     println!("OBJ files are supported by: {}", desc.name);
///     println!("Author: {}", desc.author);
///     println!("Supported extensions: {:?}", desc.file_extensions);
/// }
/// # Ok::<(), asset_importer::Error>(())
/// ```
pub fn get_importer_desc(extension: &str) -> crate::Result<Option<ImporterDesc>> {
    let c_extension = CString::new(extension).map_err(|_| {
        crate::Error::invalid_parameter("file extension contains NUL byte".to_string())
    })?;

    unsafe {
        let desc_ptr = sys::aiGetImporterDesc(c_extension.as_ptr());
        Ok(ffi::ref_from_ptr(&c_extension, desc_ptr).map(ImporterDesc::from_raw))
    }
}

/// Get importer description for a given file extension (zero allocation).
pub fn get_importer_desc_cstr(extension: &std::ffi::CStr) -> Option<ImporterDesc> {
    unsafe {
        let desc_ptr = sys::aiGetImporterDesc(extension.as_ptr());
        ffi::ref_from_ptr(extension, desc_ptr).map(ImporterDesc::from_raw)
    }
}

/// Get descriptions of all available importers
///
/// This function returns information about all importers compiled into Assimp.
/// Note: This is a convenience function that iterates through common file extensions.
/// For complete coverage, you may need to check specific extensions you're interested in.
///
/// # Returns
/// A vector of `ImporterDesc` for all available importers
///
/// # Example
/// ```rust,no_run
/// use asset_importer::get_all_importer_descs;
///
/// let importers = get_all_importer_descs();
/// println!("Available importers:");
/// for desc in importers {
///     println!("  {} - Extensions: {:?}", desc.name, desc.file_extensions);
/// }
/// ```
pub fn get_all_importer_descs() -> Vec<ImporterDesc> {
    get_all_importer_descs_iter().collect()
}

/// Iterate descriptions of all available importers without allocating a `Vec`.
pub fn get_all_importer_descs_iter() -> ImporterDescIterator {
    ImporterDescIterator {
        index: 0,
        count: unsafe { sys::aiGetImportFormatCount() },
    }
}

/// Iterator over importer descriptions.
pub struct ImporterDescIterator {
    index: usize,
    count: usize,
}

impl Iterator for ImporterDescIterator {
    type Item = ImporterDesc;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.count {
            let i = self.index;
            self.index += 1;
            unsafe {
                let ptr = sys::aiGetImportFormatDescription(i);
                if let Some(desc) = ffi::ref_from_ptr(self, ptr).map(ImporterDesc::from_raw) {
                    return Some(desc);
                }
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count.saturating_sub(self.index);
        (0, Some(remaining))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_importer_flags() {
        let flags = ImporterFlags::SUPPORT_TEXT_FLAVOUR | ImporterFlags::SUPPORT_BINARY_FLAVOUR;

        assert!(flags.contains(ImporterFlags::SUPPORT_TEXT_FLAVOUR));
        assert!(flags.contains(ImporterFlags::SUPPORT_BINARY_FLAVOUR));
        assert!(!flags.contains(ImporterFlags::EXPERIMENTAL));
    }

    #[test]
    fn test_get_importer_desc() {
        // Test with a common format that should be supported
        let desc = get_importer_desc("obj").unwrap();
        assert!(desc.is_some(), "OBJ format should be supported");

        if let Some(desc) = desc {
            assert!(!desc.name.is_empty());
            assert!(desc.file_extensions.contains(&"obj".to_string()));
        }
    }

    #[test]
    fn test_get_importer_desc_invalid() {
        // Test with an invalid extension
        let desc = get_importer_desc("invalid_extension_xyz").unwrap();
        assert!(desc.is_none());
    }

    #[test]
    fn test_get_all_importer_descs() {
        let importers = get_all_importer_descs();
        assert!(!importers.is_empty(), "Should have at least some importers");

        // Check that we have some common importers
        let names: Vec<_> = importers.iter().map(|d| d.name.as_str()).collect();
        println!("Available importers: {:?}", names);
    }
}
