//! Low-level FFI bindings for the Assimp 3D asset import library
//!
//! This crate provides raw, unsafe bindings to the Assimp C API.
//! For safe, idiomatic Rust bindings, use the `asset-importer` crate instead.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::all)]
#![allow(unpredictable_function_pointer_comparisons)]

#[cfg(any(
    all(feature = "system", feature = "prebuilt"),
    all(feature = "system", feature = "build-assimp"),
    all(feature = "prebuilt", feature = "build-assimp"),
))]
compile_error!(
    "Build mode features are mutually exclusive. Use at most one of: `system`, `prebuilt`, `build-assimp`.\n\
     Hint: use `system` to link against a system-installed Assimp, `prebuilt` to download prebuilt binaries, or `build-assimp` to build from source."
);

#[cfg(all(feature = "system", not(feature = "generate-bindings")))]
compile_error!(
    "feature `system` requires `generate-bindings` so Rust bindings match the system Assimp headers.\n\
     Hint: enable `asset-importer-sys/generate-bindings` (or use `build-assimp` / `prebuilt`)."
);

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// Re-export commonly used types for convenience
pub use aiImporterDesc as ImporterDesc;
pub use aiScene as Scene;

// Import/Export function aliases for clarity
pub use aiImportFile as import_file;
pub use aiReleaseImport as release_import;

/// Version information for this crate
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

// Include tests
mod test;

// Include type extensions (optional convenience implementations)
#[cfg(feature = "type-extensions")]
pub mod types;
