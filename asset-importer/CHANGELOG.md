# Changelog

All notable changes to `asset-importer` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Zero-copy accessors**: Added `*_raw()` / `*_iter()` accessors for meshes (vertices/normals/tangents/bitangents/UVs/colors), plus `Texture::data_ref()` to borrow embedded texture bytes/texels without allocation.
- **More zero-allocation views**: Added `Mesh::faces_raw()` and `Face::indices_raw()`, plus `Material::texture_ref()` / `Material::texture_refs()` with `TextureInfoRef` to query texture metadata without allocating the path `String`.
- **More zero-copy iterators**: Added `Bone::weights_raw()` / `Bone::weights_iter()` and `Node::mesh_indices_raw()` / `Node::mesh_indices_iter()` to avoid repeated allocations in common skeletal/scene traversal code.
- **Raw zero-copy views**: Added `asset_importer::raw` with `repr(C)` mirrors so `*_raw()` APIs no longer depend on exposing `asset_importer::sys` by default.
- **Zero-copy material properties**: Added `Material::properties()` yielding `MaterialPropertyRef` (borrowed key + raw bytes), plus raw animation key accessors on `NodeAnimation`.

### Changed
- **Scene ownership model (breaking)**: Scene-backed view types (`Mesh`, `Node`, `Material`, `Texture`, etc.) now own a cheap clone of `Scene` instead of borrowing via lifetimes, making them effectively `'static` and more ergonomic for async/multithreading.
- **View trait changes (breaking)**: Many scene-backed view types are no longer `Copy`; prefer cloning the view handle when needed.
- **Export blob ergonomics (breaking)**: `ExportBlobView` no longer borrows `ExportBlob`; export blob views now keep the underlying blob chain alive internally (Arc-backed), making them easier to pass around.
- **Thread-safety boundaries**: `SharedPtr<T>` is no longer `Send + Sync` for all `T`; cross-thread sharing is now only enabled for explicitly approved, read-only Assimp/FFI target types.
- **Assimp version helpers**: `version::assimp_version()` now reports `major.minor.patch` and new helpers expose patch/branch/legal strings.
- **Bundled Assimp updated**: Vendored Assimp (via `asset-importer-sys`) is now pinned to `v6.0.2`.
- **Thread-safety internals**: Centralized Assimp pointer sharing logic to reduce scattered `unsafe impl Send/Sync` across the codebase.
- **Material property keys (breaking)**: `material_keys::*` are now `&CStr` constants, and `Material::get_*_property` APIs prefer `&CStr` with `*_str` convenience wrappers.
- **Raw sys bindings opt-in (breaking)**: `asset_importer::sys` is now behind the `raw-sys` feature to reduce accidental safety contract violations.
- **Raw pointers opt-in (breaking)**: `Scene::as_raw()` (and similar `as_raw()` accessors on scene-backed view types) now require the `raw-sys` feature; the default API stays sys-free.
- **Zero-copy API types (breaking)**: `Mesh::vertices_raw()`/`normals_raw()`/etc and `Texture::data_ref()` now return `asset_importer::raw::*` or crate-owned types instead of `sys::*`, so users can consume zero-copy data without enabling `raw-sys`.
- **Raw view safety (breaking)**: `asset_importer::raw::AiFace::indices_unchecked()` is now `unsafe` to prevent safe Rust from dereferencing arbitrary user-provided pointers.
- **Postprocess ownership (breaking)**: `Scene::apply_postprocess` now consumes and returns `Scene` to avoid double-free/use-after-free if Assimp invalidates the scene pointer on failure.
- **Postprocess with shared scenes**: `Scene::apply_postprocess` now post-processes a deep copy when the scene is shared, avoiding mutation of shared scene memory.
- **Thread-safe callbacks (breaking)**: `io::FileSystem`/`io::FileStream` and `progress::ProgressHandler` are now `Send` (and `FileSystem` is `Sync`) so import/export configuration can be moved across threads safely.
- **Material string ergonomics**: `MaterialStringRef` now implements `Display` (use `.to_string()` via `ToString`) and exposes `to_string_lossy()` as an explicit allocating conversion helper.

### Fixed
- **Send/Sync on scene-backed views**: `Texture` and other scene-backed view types now implement `Send + Sync`, matching the multithreading guarantees promised by the crate.
- **`export` feature build**: Fixed `ExportBlob::data()` lifetime issue and ensured `ExportBlob` is `Send + Sync` when `export` is enabled.
- **FFI property memory leak**: Matrix properties passed through the C++ bridge no longer leak memory.
- **Custom IO leaks**: Assimp `aiFileIO` user-data is now released via RAII, fixing leaks when using a custom `FileSystem`.
- **Custom IO Windows path handling**: The C++ IOSystem bridge now reports the correct OS path separator on Windows.
- **Export blob ownership**: Export blob iteration no longer risks double-free by mixing owned and borrowed nodes.
- **aiString conversion correctness**: `aiString` is decoded using its explicit length (no longer assuming NUL-termination) for names and metadata.
- **Metadata/data alignment safety**: `aiMetadataEntry::mData` and material property blobs are now read with unaligned-safe loads to avoid UB on misaligned payloads.
- **Panic surface reduction**: Scene-backed `from_raw` constructors no longer `expect()` on null pointers; internal invariants are checked via `debug_assert!`.
- **FFI panic safety**: Panics from custom `FileSystem`/`FileStream` and progress callbacks are caught to prevent unwinding across the C ABI.
- **Progress callback thread-safety**: Progress handlers are invoked under a mutex to avoid `&mut` aliasing if Assimp calls the callback from multiple threads.
- **Custom IO callback thread-safety**: File stream callbacks now lock the per-file stream to avoid `&mut` aliasing UB if Assimp calls IO procs concurrently.
- **Memory import length safety**: `import_from_memory` now rejects buffers larger than `u32::MAX` to avoid length truncation in the Assimp C API.
- **Material typed-slice safety**: `MaterialPropertyRef::{data_i32,data_f32,data_f64}` now reject null payload pointers when length is non-zero to avoid UB.
- **Iterator robustness**: Iterators over scene-backed pointer arrays now skip null entries instead of ending iteration early.
- **Enum conversion safety**: Removed `unsafe transmute` from `TextureType::to_sys()` to eliminate a potential UB footgun.

## [0.4.0] - 2025-09-20

### Added
- **Async/Multithreading Support**: Implemented `Send` and `Sync` traits for all core types to enable async/await and multithreading usage
  - All core types (`Scene`, `Node`, `Mesh`, `Material`, `Light`, `Camera`, `Bone`, `Animation`, `Texture`, etc.) now implement `Send + Sync`
  - All iterator types now support multithreading
  - Zero-copy performance maintained in multithreaded contexts

## [0.3.0] - 2025-09-19

### Changed
- **License Documentation**: Enhanced license information in README with dedicated license badges
  - Added MIT License badge
  - Added Apache 2.0 License badge
  - Added BSD 3-Clause License badge for Assimp dependency

## [0.2.1] - 2025-09-19

### Fixed
- **Export functionality**: Fixed compilation errors in exporter module
  - Added `Debug` trait constraint to `FileSystem` trait
  - Implemented manual `Debug` for `ExportBuilder` to handle `dyn FileSystem`
  - Fixed FFI parameter order in `aiExportSceneEx` calls
- **Mint integration**: Fixed orphan rule violations and indexing errors
  - Replaced direct `From` trait implementations with new `FromMint`/`ToMint` traits
  - Fixed matrix conversion using `to_cols_array_2d()` instead of `to_cols_array()`
  - All mint conversions now work correctly for Vector2D, Vector3D, Matrix4x4, and Quaternion
- **Prebuilt binaries**: Fixed DLL dependency issues on Windows
  - Added missing DLL copying logic for prebuilt binaries
  - Prebuilt feature now works correctly without STATUS_DLL_NOT_FOUND errors
- **Logging system**: Removed unsafe callback-based logging to prevent access violations
  - Removed custom log streams (stdout, stderr, file) due to FFI safety issues
  - Kept verbose logging functionality which is safe to use
  - Added deprecation warnings for removed functionality
  - Updated examples to use safe logging methods only

### Added
- New `FromMint` and `ToMint` traits for safe mint library integration
- Comprehensive test coverage for all fixed functionality
- **Mint Integration Example**: Added `07_mint_integration.rs` example demonstrating conversions between asset-importer and mint types

### Removed
- **Custom Log Streams**: Removed callback-based logging streams due to STATUS_ACCESS_VIOLATION errors
  - `attach_stdout_stream()`, `attach_stderr_stream()`, `attach_file_stream()` now return errors
  - `Logger::attach_stream()`, `Logger::detach_stream()` methods deprecated
  - Use `enable_verbose_logging()` and `get_last_error_message()` for safe logging instead

## [0.2.0] - 2025-09-18

### Fixed
- Logging could cause an access violation in some cases.

### Added
- Expose the missing `prebuilt` feature for parity with `asset-importer-sys`.

## [0.1.0] - 2025-09-17

### Added
- Initial release of asset-importer
- High-level Rust API for Assimp 3D asset import library
- Safe and ergonomic wrappers around asset-importer-sys
- Support for loading various 3D model formats
- Integration with popular Rust math libraries (glam, mint)
- Comprehensive error handling with thiserror
- Type-safe scene graph representation
- Animation and material support
- Export functionality for supported formats

### Features
- Scene loading and parsing
- Mesh data extraction
- Material and texture handling
- Animation data access
- Node hierarchy traversal
- Memory-safe API design
- Optional mint integration for math interoperability

---

## How to update this changelog

When making changes to `asset-importer`, please:

1. Add your changes under the `[Unreleased]` section
2. Use the appropriate category: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`
3. Write clear, user-focused descriptions
4. When release-plz creates a release PR, it will automatically move unreleased changes to a new version section

Example:

```markdown
## [Unreleased]

### Added
- New API method for loading animations

### Fixed
- Memory leak in scene parsing
```
