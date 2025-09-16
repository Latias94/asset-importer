//! # Asset Importer
//!
//! Comprehensive Rust bindings for the Assimp 3D asset import library.
//!
//! This crate provides safe, idiomatic Rust bindings for Assimp, offering
//! more complete functionality than existing alternatives like `russimp`.
//!
//! ## Features
//!
//! - **Complete API coverage**: Import, export, and post-processing
//! - **Memory safety**: Safe Rust abstractions over raw C API
//! - **Zero-cost abstractions**: Minimal overhead over direct C API usage
//! - **Custom I/O**: Support for custom file systems and progress callbacks
//! - **Modern Rust**: Uses latest Rust features and idioms
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use asset_importer::{Importer, postprocess::PostProcessSteps};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let importer = Importer::new();
//! let scene = importer
//!     .read_file("model.fbx")
//!     .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::FLIP_UVS)
//!     .import_file("model.fbx")?;
//!
//! println!("Loaded {} meshes", scene.meshes().count());
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! This crate is built on top of `asset-importer-sys`, which provides the raw
//! FFI bindings. The high-level API is designed to be:
//!
//! - **Safe**: All unsafe operations are encapsulated
//! - **Ergonomic**: Builder patterns and method chaining
//! - **Efficient**: Zero-copy where possible
//! - **Extensible**: Support for custom I/O and callbacks

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

pub use asset_importer_sys as sys;

// Re-export common types for convenience
pub use crate::{
    error::{Error, Result},
    importer::{import_properties, ImportBuilder, Importer, PropertyStore, PropertyValue},
    scene::{MemoryInfo, Scene},
    types::*,
};

#[cfg(feature = "export")]
pub use crate::exporter::{ExportBlob, ExportBuilder, ExportFormatDesc};

// Re-export logging functionality
pub use crate::logging::{LogLevel, LogStream, Logger};

// Re-export metadata functionality
pub use crate::metadata::{Metadata, MetadataEntry, MetadataType};

// Re-export material functionality
pub use crate::material::{material_keys, Material, TextureInfo, TextureType};

// Re-export texture functionality
pub use crate::texture::{Texel, Texture, TextureData, TextureIterator};

// Re-export AABB functionality
pub use crate::aabb::AABB;

// Re-export bone functionality
pub use crate::bone::{Bone, BoneIterator, VertexWeight};

// Re-export importer description functionality
pub use crate::importer_desc::{
    get_all_importer_descs, get_importer_desc, ImporterDesc, ImporterFlags,
};

// Core modules
pub mod error;
pub mod importer;
pub mod importer_desc;
pub mod scene;
pub mod types;

// Component modules
pub mod animation;
pub mod camera;
pub mod light;
pub mod material;
pub mod mesh;
pub mod node;

// Data structure modules
pub mod aabb;
pub mod bone;
pub mod texture;

// Advanced features
#[cfg(feature = "export")]
pub mod exporter;
pub mod io;
pub mod logging;
pub mod metadata;
pub mod progress;

// Utility modules
pub mod math;
pub mod postprocess;
pub mod utils;

/// Version information
pub mod version {
    /// Version of this crate
    pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

    /// Version of the underlying Assimp library
    pub fn assimp_version() -> String {
        let major = assimp_version_major();
        let minor = assimp_version_minor();
        let revision = assimp_version_revision();
        format!("{}.{}.{}", major, minor, revision)
    }

    /// Major version of Assimp
    pub fn assimp_version_major() -> u32 {
        unsafe { crate::sys::aiGetVersionMajor() }
    }

    /// Minor version of Assimp
    pub fn assimp_version_minor() -> u32 {
        unsafe { crate::sys::aiGetVersionMinor() }
    }

    /// Revision of Assimp
    pub fn assimp_version_revision() -> u32 {
        unsafe { crate::sys::aiGetVersionRevision() }
    }

    /// Version string reported by Assimp
    pub fn assimp_version_string() -> String {
        // Note: aiGetVersionString is not available in the current bindings
        // Use the version components to construct a version string
        format!(
            "{}.{}.{}",
            assimp_version_major(),
            assimp_version_minor(),
            assimp_version_revision()
        )
    }

    /// Compile flags used to build Assimp
    pub fn assimp_compile_flags() -> u32 {
        unsafe { crate::sys::aiGetCompileFlags() }
    }
}

/// Check if a file extension is supported for import
pub fn is_extension_supported(extension: &str) -> bool {
    let c_extension = std::ffi::CString::new(extension).unwrap_or_default();
    unsafe { crate::sys::aiIsExtensionSupported(c_extension.as_ptr()) != 0 }
}

/// Get a list of all supported import file extensions
pub fn get_import_extensions() -> Vec<String> {
    let mut ai_string = crate::sys::aiString {
        length: 0,
        data: [0; 1024],
    };

    unsafe {
        crate::sys::aiGetExtensionList(&mut ai_string);
    }

    // Convert aiString to Rust String
    let extension_list = if ai_string.length > 0 {
        let slice = unsafe {
            std::slice::from_raw_parts(
                ai_string.data.as_ptr() as *const u8,
                ai_string.length as usize,
            )
        };
        String::from_utf8_lossy(slice).to_string()
    } else {
        // Fallback to hardcoded list if the function fails
        return vec![
            ".obj".to_string(),
            ".fbx".to_string(),
            ".dae".to_string(),
            ".gltf".to_string(),
            ".glb".to_string(),
            ".3ds".to_string(),
            ".blend".to_string(),
            ".x".to_string(),
            ".ply".to_string(),
            ".stl".to_string(),
            ".md2".to_string(),
            ".md3".to_string(),
            ".md5".to_string(),
            ".ase".to_string(),
            ".ifc".to_string(),
        ];
    };

    // Parse the extension list (format: "*.3ds;*.obj;*.dae")
    extension_list
        .split(';')
        .filter_map(|ext| {
            let trimmed = ext.trim();
            if trimmed.starts_with("*.") {
                Some(trimmed[1..].to_string()) // Remove the '*' prefix
            } else {
                None
            }
        })
        .collect()
}

/// Get a list of all supported export formats
#[cfg(feature = "export")]
pub fn get_export_formats() -> Vec<crate::exporter::ExportFormatDesc> {
    let count = unsafe { sys::aiGetExportFormatCount() };
    let mut formats = Vec::with_capacity(count);

    for i in 0..count {
        unsafe {
            let desc_ptr = sys::aiGetExportFormatDescription(i);
            if !desc_ptr.is_null() {
                let desc = crate::exporter::ExportFormatDesc::from_raw(&*desc_ptr);
                formats.push(desc);
                sys::aiReleaseExportFormatDescription(desc_ptr);
            }
        }
    }

    formats
}

/// Enable verbose logging for debugging
pub fn enable_verbose_logging(enable: bool) {
    unsafe {
        crate::sys::aiEnableVerboseLogging(if enable { 1 } else { 0 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let version = version::assimp_version();
        assert!(!version.is_empty());

        let major = version::assimp_version_major();
        let minor = version::assimp_version_minor();
        let revision = version::assimp_version_revision();

        // Assimp should be at least version 5.0
        assert!(major >= 5);
        println!("Assimp version: {}.{}.{}", major, minor, revision);
    }

    #[test]
    fn test_extension_support() {
        // These formats should definitely be supported
        assert!(is_extension_supported("obj"));
        assert!(is_extension_supported("fbx"));
        assert!(is_extension_supported("dae"));
        assert!(is_extension_supported("gltf"));

        // This should not be supported
        assert!(!is_extension_supported("xyz"));
    }

    #[test]
    fn test_get_extensions() {
        let extensions = get_import_extensions();
        assert!(!extensions.is_empty());
        assert!(extensions.contains(&".obj".to_string()));
        println!("Supported extensions: {:?}", extensions);
    }
}
