//! Low-level FFI bindings for the Assimp 3D asset import library
//!
//! This crate provides raw, unsafe bindings to the Assimp C API.
//! For safe, idiomatic Rust bindings, use the `asset-importer` crate instead.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::all)]

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
