# Changelog

All notable changes to `asset-importer-sys` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
