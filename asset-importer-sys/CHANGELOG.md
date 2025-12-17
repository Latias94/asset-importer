# Changelog

All notable changes to `asset-importer-sys` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Bundled Assimp updated**: Updated the `assimp` submodule to `v6.0.2` (`fb375dd8c66f1c47dd000548f44d81b47f4466df`).

### Fixed
- **System Assimp version mismatch**: `system` builds now fail fast when discovered headers are older than Assimp 6 (including when pkg-config omits include paths), avoiding confusing missing-symbol errors during compilation.
- **macOS prebuilt runtime loading**: Prebuilt builds now copy Assimp shared libraries into `OUT_DIR` to avoid `dyld: Library not loaded @rpath/libassimp*.dylib` at runtime.

## [0.3.0] - 2025-09-19

### Changed
- **Package Distribution**: Enhanced license file inclusion in binary packages
  - Updated packaging tool to include LICENSE-MIT, LICENSE-APACHE, and LICENSE-assimp.txt
  - All license files are now properly included in prebuilt binary distributions

## [0.2.1] - 2025-09-19

### Fixed
- **Prebuilt binaries**: Fixed DLL dependency issues on Windows
  - Added missing DLL copying logic in `link_prebuilt_assimp` function
  - Prebuilt shared libraries now correctly copy required DLLs to target directories
  - Resolves STATUS_DLL_NOT_FOUND errors when using prebuilt feature

## [0.2.0] - 2025-09-18

### Changed
- Refactored `build.rs` and aligned MSVC runtime selection to avoid Debug CRT across all targets (use RelWithDebInfo on MSVC when Cargo is in debug).
- C++ bridge now avoids `/MDd` and `/MTd` and disables iterator debug to match Rust’s runtime.

### Improved
- Added a stable prebuilt cache under `target/asset-importer-prebuilt/{version}/{target}/{link_type}[-{crt}]`.
- Offline support via `ASSET_IMPORTER_OFFLINE=1` or `CARGO_NET_OFFLINE=true`, with graceful fallback to cache.
- Quieter downloads: only prints when an actual network fetch occurs; reuses cached archives and extractions.
- Refined CMake configuration: set `CMAKE_MSVC_RUNTIME_LIBRARY`, enable `CMP0091`, and disable PDB install on Windows.

## [0.1.0] - 2025-09-17

### Added
- Initial release of asset-importer-sys
- FFI bindings for Assimp 3D asset import library
- Support for multiple build modes:
  - Build from source with `build-assimp` feature
  - Use system-installed assimp with `system` feature
  - Use prebuilt binaries with `prebuilt` feature
- Static and dynamic linking support
- Cross-platform support (Windows, macOS, Linux)
- Package tool for creating prebuilt binary distributions
- Comprehensive CI/CD pipeline with release automation

### Features
- Complete Assimp API bindings
- Type-safe Rust wrappers for C structures
- Optional mint math library integration
- Export functionality support
- Configurable zlib linking

---

## How to update this changelog

When making changes to `asset-importer-sys`, please:

1. Add your changes under the `[Unreleased]` section
2. Use the appropriate category: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`
3. Write clear, user-focused descriptions
4. When release-plz creates a release PR, it will automatically move unreleased changes to a new version section

Example:
```markdown
## [Unreleased]

### Added
- New feature X that allows Y

### Fixed
- Bug where Z would cause W
```
