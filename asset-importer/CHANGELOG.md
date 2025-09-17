# Changelog

All notable changes to `asset-importer` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
