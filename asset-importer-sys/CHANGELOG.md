# Changelog

All notable changes to `asset-importer-sys` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Breaking:** All native enum types use bindgen integer newtypes with associated constants. Unknown file values and flag combinations remain valid at the FFI boundary; use `.0` to read their integer representation.
- Source builds explicitly bundle minizip while retaining system zlib on Unix. Windows GNU source/prebuilt/package paths distinguish static archives from DLL import archives and retain bundled zlib.
- Local prebuilt extraction caches track the archive's canonical path and contents, and Cargo watches the selected archive for replacement.
- Bundled Assimp is now [6.0.6](https://github.com/assimp/assimp/releases/tag/v6.0.6) (`a79ff6bbce54d3e01186a918dc6374ed55317426`). Bindings are regenerated with bindgen 0.73.2; source overrides and prebuilt packages must match this version.
- **Breaking:** The custom import bridge returns an opaque `aiRustScene` owner. Use `aiGetSceneRust` to borrow its scene, `aiApplyPostProcessingRust` for exclusive post-processing, and `aiReleaseSceneRust` to release it. Upstream C API bindings remain available.
- **Breaking:** Bridge export functions reject nonzero preprocessing arguments because Assimp's exporter skips property initialization for those steps. Prepare the scene with its importer before exporting.
- The package tool uses Cargo's current `OUT_DIR`, target, and CRT settings, removing directory scans and runtime target guesses. Archives include `revision.h` and are checked against their actual build version before packaging.
- Updated build/download dependencies and added `BUNDLED_ASSIMP_VERSION`. macOS fallback deployment targets now match Rust's supported baseline: 10.13 for Intel and 11.0 for ARM64.

### Fixed
- The C++ bridge guards all scene-copy operations it exposes against sparse morph channel aliasing and blocks 3MF blob/custom I/O writes to the host file system.
- Export copies reject empty/unsupported node metadata and mesh animation channels that Assimp would leave shared with the source. Already unique indices with a stale non-verbose flag are normalized on an independent private scene.
- Native Doxygen code blocks render as text in generated Rust documentation, so diagrams and C/C++ examples are not compiled as Rust doctests.
- System header version parsing now skips unrelated defines and include guards. Prebuilt packages whose version cannot be determined are rejected.
- The bridge retains importer ownership after post-processing failures, releases custom I/O streams on every destruction path, and preserves progress cancellation and C++ exception boundaries.
- The bridge rejects `aiProcess_SplitLargeMeshes` before import or scene mutation: Assimp 6.0.6 can retain freed meshes, write through null sparse-channel pointers, or fail to advance. This guard applies to source, prebuilt, and system providers; raw upstream bindings remain available.
- File export maps bridge allocation failures to `aiReturn_OUTOFMEMORY`.
- `aiMaterialProperty::default()` initializes `aiPTI_Float`, matching the C++ constructor; generated integer newtypes also eliminate invalid native enum discriminants.
- Windows GNU source static builds now link bundled `zlibstatic` archives, including debug-suffixed libraries.
- Windows runtime DLL placement supports both Cargo build directory layouts instead of assuming a fixed `OUT_DIR` depth, and reports copy failures.
- Changes to pregenerated bindings now retrigger the build script.

### Removed
- **Breaking:** Removed the ineffective `nozlib` feature. Assimp requires zlib; static builds always link the platform's required library.

## [0.8.0] - 2026-05-04

### Added
- **Maintainer binding regeneration override**: Added `ASSET_IMPORTER_FORCE_GENERATE_BINDINGS=1` so vendored binding refreshes can run bindgen without temporarily removing pregenerated bindings.

### Changed
- **Bundled Assimp updated**: Updated the `assimp` submodule to `v6.0.5` (`392a658f9c271be965271f45e7521a1b80ea4392`).
- **Prebuilt/package version gate**: Updated expected prebuilt Assimp metadata to `6.0.5`.
- **Source-build-first docs**: Clarified that the default sys crate path builds bundled Assimp from source and that `prebuilt` is an explicit opt-in mode requiring matching release artifacts.
- **Assimp version metadata**: Centralized the vendored Assimp version used by the build script and prebuilt package tool in `assimp-version.txt`.

### Fixed
- **C++ bridge ownership**: The bridge now respects Assimp's ownership contract for custom `IOSystem` and `ProgressHandler` objects, avoiding double-free risk.
- **C++ bridge exception boundary**: Bridge entrypoints now catch C++ exceptions and convert them into the thread-local bridge error string instead of allowing exceptions to cross the C ABI boundary.
- **Vendored build cache invalidation**: Source builds now stamp the Assimp version/link profile in `OUT_DIR` and rebuild cached CMake output when it changes, avoiding stale 6.0.4 libraries after upgrading to 6.0.5.
- **Prebuilt version validation**: Prebuilt packages are now validated against `manifest.txt` before falling back to headers, preventing stale packages whose installed headers do not expose version macros from being accepted silently.

## [0.7.0] - 2026-01-25

### Changed
- **Bundled Assimp updated**: Updated the `assimp` submodule to `v6.0.4` (`e0b52347c6e52de2827ec957a9ebf00ce3c54f79`).
- **Pregenerated bindings updated**: Regenerated `src/bindings_pregenerated.rs` for Assimp `v6.0.4`.

## [0.6.0] - 2026-01-20

### Changed
- **Bundled Assimp updated**: Updated the `assimp` submodule to `v6.0.3` (`cf7b652190827878a4726709f0998ce170e55ce8`).
- **Prebuilt downloads**: Switched the prebuilt downloader from `reqwest` to `ureq` to reduce dependency footprint.
- **Crates.io package contents**: Examples are now included in the published crate package.

### Added
- **Exporter property bridge**: Added `aiExportSceneExWithPropertiesRust` and `aiExportSceneToBlobWithPropertiesRust` to support exporter properties with custom IO.

### Fixed
- **Windows system (vcpkg) robustness**: Normalize vcpkg root discovery (including `VCPKG_INSTALLED_DIR`) and opt-in dynamic linking for dynamic triplets.

## [0.5.0] - 2025-12-18

### Added
- **Deterministic bindings by default**: Non-`system` builds now use the shipped pregenerated bindings by default (no libclang required), avoiding header/version skew and speeding up builds.
- **Opt-in bindgen**: Added feature `generate-bindings` to regenerate bindings at build time when needed.
- **Mutually exclusive build modes**: `system`, `prebuilt`, and `build-assimp` are now enforced as mutually exclusive to reduce surprising dependency/build behavior.

### Changed
- **Bundled Assimp updated**: Updated the `assimp` submodule to `v6.0.2` (`fb375dd8c66f1c47dd000548f44d81b47f4466df`).
- **Windows vcpkg ergonomics**: When building with MSVC + `crt-static`, system linking now defaults to a `*-windows-static` vcpkg triplet (unless `VCPKGRS_TRIPLET` is set).
- **System + static-link**: `static-link` is now respected for `system` builds where supported (pkg-config `--static`; vcpkg via `*-windows-static` triplets).

### Fixed
- **System Assimp version mismatch**: `system` builds now fail fast when discovered headers are older than Assimp 6 (including when pkg-config omits include paths), and validate the generated bindings for required symbols to avoid confusing missing-item errors during compilation.
- **macOS prebuilt runtime loading**: Prebuilt builds now copy Assimp shared libraries into `OUT_DIR` to avoid `dyld: Library not loaded @rpath/libassimp*.dylib` at runtime.
- **docs.rs/offline bindings skew**: Updated pregenerated bindings to match the vendored Assimp headers, avoiding missing symbol/type errors when bindgen cannot run.
- **Prebuilt release CI robustness**: Fixed archive verification and caching so Windows MD/MT variants and Linux tar listing checks don't fail spuriously.
- **Windows system (vcpkg) robustness**: Treat `VCPKG_INSTALLATION_ROOT` as a fallback for `VCPKG_ROOT`, and create missing `installed/vcpkg/updates` metadata dir to avoid vcpkg-rs probe failures in CI.

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
