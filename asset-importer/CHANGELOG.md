# Changelog

All notable changes to `asset-importer` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Stronger lifetime safety (breaking)**: Most scene-backed view types now borrow the owning `Scene` via lifetimes to prevent use-after-free in safe code.
- **Assimp version helpers**: `version::assimp_version()` now reports `major.minor.patch` and new helpers expose patch/branch/legal strings.

### Fixed
- **FFI property memory leak**: Matrix properties passed through the C++ bridge no longer leak memory.
- **Custom IO leaks**: Assimp `aiFileIO` user-data is now released via RAII, fixing leaks when using a custom `FileSystem`.
- **Export blob ownership**: Export blob iteration no longer risks double-free by mixing owned and borrowed nodes.
- **aiString conversion correctness**: `aiString` is decoded using its explicit length (no longer assuming NUL-termination) for names and metadata.

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
