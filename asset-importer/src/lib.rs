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
//! let scene = Importer::new().import_file_with("model.fbx", |b| {
//!     b.with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::FLIP_UVS)
//! })?;
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
//!
//! ## Build features
//! This crate supports three mutually exclusive build modes:
//! - `prebuilt` (default): download/use prebuilt Assimp binaries via `asset-importer-sys`
//! - `build-assimp`: build Assimp from source via CMake
//! - `system`: link against a system-installed Assimp (requires libclang/bindgen)
//!
//! For `system`, use `--no-default-features --features system`.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

#[cfg(any(
    all(feature = "prebuilt", feature = "build-assimp"),
    all(feature = "prebuilt", feature = "system"),
    all(feature = "build-assimp", feature = "system"),
))]
compile_error!(
    "Build mode features are mutually exclusive. Use exactly one of: `prebuilt` (default), `build-assimp`, or `system`.\n\
     Hint: for system Assimp use `--no-default-features --features system`."
);

#[cfg(feature = "raw-sys")]
pub use asset_importer_sys as sys;

#[cfg(not(feature = "raw-sys"))]
pub(crate) use asset_importer_sys as sys;

// Re-export common types for convenience
pub use crate::{
    error::{Error, Result},
    importer::{ImportBuilder, Importer, PropertyStore, PropertyValue, import_properties},
    scene::{MemoryInfo, Scene},
    types::*,
};

/// Zero-copy raw view types for Assimp-owned data.
pub mod raw;

#[cfg(feature = "export")]
pub use crate::exporter::{ExportBlob, ExportBuilder, ExportFormatDesc, export_properties};

// Re-export logging functionality
#[allow(deprecated)]
pub use crate::logging::{LogLevel, LogStream, Logger};

// Re-export metadata functionality
pub use crate::metadata::{Metadata, MetadataEntry, MetadataType};

// Re-export material functionality
pub use crate::material::{
    Material, MaterialPropertyInfo, MaterialPropertyIterator, MaterialPropertyRef,
    MaterialStringRef, PropertyTypeInfo, TextureInfo, TextureInfoRef, TextureType, material_keys,
};

// Re-export texture functionality
pub use crate::texture::{Texel, Texture, TextureData, TextureIterator};

// Re-export AABB functionality
pub use crate::aabb::AABB;

// Re-export bone functionality
pub use crate::bone::{Bone, BoneIterator, VertexWeight};

// Re-export animation type for convenience (used by examples)
pub use crate::animation::Animation;

// Re-export importer description functionality
pub use crate::importer_desc::{
    ImporterDesc, ImporterDescIterator, ImporterFlags, get_all_importer_descs,
    get_all_importer_descs_iter, get_importer_desc, get_importer_desc_cstr,
};

// Core modules
mod bridge_properties;
pub mod error;
pub(crate) mod ffi;
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

mod ptr;

/// Version information
pub mod version {
    /// Version of this crate
    pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

    /// Version of the underlying Assimp library
    pub fn assimp_version() -> String {
        format!(
            "{}.{}.{}",
            assimp_version_major(),
            assimp_version_minor(),
            assimp_version_patch()
        )
    }

    /// Major version of Assimp
    pub fn assimp_version_major() -> u32 {
        unsafe { crate::sys::aiGetVersionMajor() }
    }

    /// Minor version of Assimp
    pub fn assimp_version_minor() -> u32 {
        unsafe { crate::sys::aiGetVersionMinor() }
    }

    /// Patch version of Assimp
    pub fn assimp_version_patch() -> u32 {
        unsafe { crate::sys::aiGetVersionPatch() }
    }

    /// Revision of Assimp
    pub fn assimp_version_revision() -> u32 {
        unsafe { crate::sys::aiGetVersionRevision() }
    }

    /// Version string reported by Assimp
    pub fn assimp_version_string() -> String {
        assimp_version()
    }

    /// Compile flags used to build Assimp
    pub fn assimp_compile_flags() -> u32 {
        unsafe { crate::sys::aiGetCompileFlags() }
    }

    /// Branch name of the Assimp runtime
    pub fn assimp_branch_name() -> String {
        unsafe { crate::error::c_str_to_string_or_empty(crate::sys::aiGetBranchName()) }
    }

    /// Legal/license string for the Assimp runtime
    pub fn assimp_legal_string() -> String {
        unsafe { crate::error::c_str_to_string_or_empty(crate::sys::aiGetLegalString()) }
    }
}

/// Check if a file extension is supported for import.
pub fn is_extension_supported(extension: &str) -> crate::Result<bool> {
    let c_extension = std::ffi::CString::new(extension).map_err(|_| {
        crate::Error::invalid_parameter("file extension contains NUL byte".to_string())
    })?;
    Ok(unsafe { crate::sys::aiIsExtensionSupported(c_extension.as_ptr()) != 0 })
}

const FALLBACK_IMPORT_EXTENSIONS: [&str; 15] = [
    ".obj", ".fbx", ".dae", ".gltf", ".glb", ".3ds", ".blend", ".x", ".ply", ".stl", ".md2",
    ".md3", ".md5", ".ase", ".ifc",
];

/// An allocation-minimized import extension list.
///
/// This keeps the raw Assimp extension list string and provides an iterator over `&str` views
/// (e.g. `".obj"`), avoiding per-extension allocations.
#[derive(Debug, Clone)]
pub struct ImportExtensions {
    raw: Option<String>,
}

#[derive(Debug)]
enum ImportExtensionsIterInner<'a> {
    Assimp(std::str::Split<'a, char>),
    Fallback(std::slice::Iter<'a, &'static str>),
}

/// Iterator over supported import extensions.
#[derive(Debug)]
pub struct ImportExtensionsIter<'a> {
    inner: ImportExtensionsIterInner<'a>,
}

impl<'a> Iterator for ImportExtensionsIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.inner {
                ImportExtensionsIterInner::Assimp(split) => {
                    let ext = split.next()?;
                    let trimmed = ext.trim();
                    if trimmed.starts_with("*.") && trimmed.len() > 1 {
                        return Some(&trimmed[1..]);
                    }
                }
                ImportExtensionsIterInner::Fallback(iter) => {
                    let s: &'static str = iter.next()?;
                    return Some(s);
                }
            }
        }
    }
}

impl ImportExtensions {
    /// Raw Assimp extension list, if available (e.g. `"*.3ds;*.obj;*.dae"`).
    pub fn raw_assimp_list(&self) -> Option<&str> {
        self.raw.as_deref()
    }

    /// Iterate extensions as `".ext"` strings (without allocation).
    pub fn iter(&self) -> ImportExtensionsIter<'_> {
        if let Some(s) = self.raw.as_deref() {
            ImportExtensionsIter {
                inner: ImportExtensionsIterInner::Assimp(s.split(';')),
            }
        } else {
            ImportExtensionsIter {
                inner: ImportExtensionsIterInner::Fallback(FALLBACK_IMPORT_EXTENSIONS.iter()),
            }
        }
    }

    /// Collect into owned `String`s.
    pub fn to_vec(&self) -> Vec<String> {
        self.iter().map(str::to_string).collect()
    }
}

/// Get all supported import file extensions (allocation-minimized).
pub fn get_import_extensions_list() -> ImportExtensions {
    let mut ai_string = crate::sys::aiString {
        length: 0,
        data: [0; 1024],
    };

    unsafe {
        crate::sys::aiGetExtensionList(&mut ai_string);
    }

    if ai_string.length > 0 {
        ImportExtensions {
            raw: Some(crate::types::ai_string_to_string(&ai_string)),
        }
    } else {
        ImportExtensions { raw: None }
    }
}

/// Get a list of all supported import file extensions (allocates).
pub fn get_import_extensions() -> Vec<String> {
    get_import_extensions_list().to_vec()
}

/// Get a list of all supported export formats
#[cfg(feature = "export")]
pub fn get_export_formats() -> Vec<crate::exporter::ExportFormatDesc> {
    get_export_formats_iter().collect()
}

/// Iterate supported export formats without allocating a `Vec`.
///
/// Each yielded item is still an owned `ExportFormatDesc` (it contains copied strings).
#[cfg(feature = "export")]
pub fn get_export_formats_iter() -> ExportFormatDescIterator {
    ExportFormatDescIterator {
        index: 0,
        count: unsafe { sys::aiGetExportFormatCount() },
    }
}

/// Iterator over Assimp export format descriptions.
#[cfg(feature = "export")]
pub struct ExportFormatDescIterator {
    index: usize,
    count: usize,
}

#[cfg(feature = "export")]
impl Iterator for ExportFormatDescIterator {
    type Item = crate::exporter::ExportFormatDesc;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.count {
            let i = self.index;
            self.index += 1;
            unsafe {
                let desc_ptr = sys::aiGetExportFormatDescription(i);
                if desc_ptr.is_null() {
                    continue;
                }
                let desc = crate::exporter::ExportFormatDesc::from_raw(&*desc_ptr);
                sys::aiReleaseExportFormatDescription(desc_ptr);
                return Some(desc);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count.saturating_sub(self.index);
        (0, Some(remaining))
    }
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
        let patch = version::assimp_version_patch();
        let revision = version::assimp_version_revision();

        // Assimp should be at least version 5.0
        assert!(major >= 5);
        println!(
            "Assimp version: {}.{}.{} (revision {})",
            major, minor, patch, revision
        );
    }

    #[test]
    fn test_extension_support() {
        // These formats should definitely be supported
        assert!(is_extension_supported("obj").unwrap());
        assert!(is_extension_supported("fbx").unwrap());
        assert!(is_extension_supported("dae").unwrap());
        assert!(is_extension_supported("gltf").unwrap());

        // This should not be supported
        assert!(!is_extension_supported("xyz").unwrap());
    }

    #[test]
    fn test_get_extensions() {
        let extensions = get_import_extensions();
        assert!(!extensions.is_empty());
        assert!(extensions.contains(&".obj".to_string()));
        println!("Supported extensions: {:?}", extensions);
    }

    #[test]
    fn test_send_sync_traits() {
        // This test verifies that our core types implement Send + Sync
        // If this compiles, our unsafe implementations are working

        fn assert_send_sync<T: Send + Sync>() {}

        // Test core types - if these compile, Send + Sync are implemented
        assert_send_sync::<Scene>();

        // The test passes if it compiles
        println!("✅ Core types implement Send + Sync!");
    }
}
