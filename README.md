# asset-importer

[![Crates.io](https://img.shields.io/crates/v/asset-importer.svg)](https://crates.io/crates/asset-importer)
[![Documentation](https://docs.rs/asset-importer/badge.svg)](https://docs.rs/asset-importer)
[![Rust Version](https://img.shields.io/badge/rust-1.85+-blue.svg)](https://www.rust-lang.org)
[![Crates.io Downloads](https://img.shields.io/crates/d/asset-importer.svg)](https://crates.io/crates/asset-importer)
[![Made with Rust](https://img.shields.io/badge/made%20with-Rust-orange.svg)](https://www.rust-lang.org)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD%203--Clause-orange.svg)](https://opensource.org/licenses/BSD-3-Clause)

A comprehensive Rust binding for the latest [Assimp](https://github.com/assimp/assimp) 3D asset import library.

This crate provides safe, high-level Rust bindings for **Assimp v6.0.2**, implementing the vast majority of the C API with idiomatic Rust interfaces.

## Example Application

See **[Model Viewer](https://github.com/Latias94/model-viewer)** (asset-importer + wgpu) - A practical model viewer built on top of this Assimp Rust binding. It serves as a real-world sample and testbed to validate asset-importer across common 3D formats.

## Status

⚠️ **Early Development**: This library is functional but lacks extensive real-world testing. Use with caution in production environments.

## Features

- **Comprehensive API Coverage**: Implements the vast majority of [Assimp v6.0.2](https://github.com/assimp/assimp/releases/tag/v6.0.2) C API
- **Import Support**: 71+ 3D file formats (OBJ, FBX, glTF, DAE, etc.)
- **Export Support**: 22+ output formats (optional)
- **Memory Safe**: Safe Rust API over unsafe FFI bindings
- **Modern Math**: Integration with glam for vectors and matrices
- **Flexible Building**: Multiple build options for different use cases
- **Cross-Platform**: Supports Windows, macOS, and Linux

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
# Default – Use prebuilt binaries (fastest)
asset-importer = "0.4"

# Or build from source (best compatibility)
asset-importer = { version = "0.4", features = ["build-assimp"] }

# Or use system-installed assimp
asset-importer = { version = "0.4", features = ["system"] }
```

Basic usage:

```rust
use asset_importer::Importer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let importer = Importer::new();
    let scene = importer.import_file("model.obj")?;
    
    println!("Loaded {} meshes", scene.num_meshes());
    
    for mesh in scene.meshes() {
        let vertices = mesh.vertices(); // Returns Vec<Vector3D>
        println!("Mesh has {} vertices", vertices.len());
    }
    
    Ok(())
}
```

## Build Options

### Default: Prebuilt Binaries (Recommended)

```toml
asset-importer = "0.4"
```

- **Fastest**: No compilation time
- **Convenient**: No native toolchain required
- **Requires**: Available release artifacts from GitHub releases
- **Note**: Only available for released versions

### Build from Source

```toml
asset-importer = { version = "0.4", features = ["build-assimp"] }
```

- **Best compatibility**: Works on all platforms
- **Full control**: Latest Assimp version with all features
- **Requires**: CMake, C++ compiler (automatically handled by Cargo)

### System Library

```toml
asset-importer = { version = "0.4", features = ["system"] }
```

- **Lightweight**: Uses existing system installation
- **Setup required**: Install via `brew install assimp` (macOS), `apt install libassimp-dev` (Ubuntu)
- **Version dependent**: Behavior may vary based on system library version

### Additional Features

```toml
asset-importer = {
    version = "0.4",
    features = [
        "export",          # Enable export functionality
        "type-extensions", # Enable convenience methods on types
        "mint",            # Enable mint math library integration
        "static-link",     # Prefer static linking (source/prebuilt)
        "nozlib"           # Disable zlib compression support
    ]
}
```

## Build Requirements

### Source Build Dependencies

When building from source (`build-assimp` feature), you'll need:

- **CMake** (3.10 or later)
- **C++ Compiler** (MSVC on Windows, GCC/Clang on Linux/macOS)
- **Git** (for submodules)

### Platform-Specific Setup

- **Windows**: Install Visual Studio Build Tools or Visual Studio
- **macOS**: Install Xcode Command Line Tools (`xcode-select --install`)
- **Linux**: Install build essentials (`sudo apt install build-essential cmake` on Ubuntu)

For detailed platform-specific instructions and troubleshooting, see the [asset-importer-sys README](https://github.com/Latias94/asset-importer/blob/main/asset-importer-sys/README.md#build-requirements).

## Windows Notes (MSVC CRT)

- Default Rust (MSVC) uses dynamic CRT (/MD). Our build follows Rust’s choice automatically.
- If you need static CRT (/MT), set a target-wide flag so all crates agree:

```
# .cargo/config.toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

or per-build:

```
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release
```

- Prebuilt binaries: we publish both MD and MT variants on Windows. Filenames include a `-md` or `-mt` suffix, for example:
  `asset-importer-<version>-x86_64-pc-windows-msvc-<static|dylib>-md.tar.gz`.
  The build script auto-selects the variant matching your current CRT and falls back to old names if needed.

- vcpkg (with `features=["system"]`):
  - Choose triplet to match CRT:
    - `x64-windows` → /MD (dynamic)
    - `x64-windows-static` → /MT (static)
  - Install: `vcpkg install assimp:x64-windows` or `assimp:x64-windows-static`.
  - If using the static triplet, also enable `crt-static` to avoid LNK2038.

## Why prebuilt binaries by default?

- **Fast builds**: No compilation time for Assimp
- **Easy setup**: No native toolchain required
- **CI/CD friendly**: Faster builds in continuous integration
- **Consistent**: Same binary across environments

If you need more control or compatibility, use:

- `--features build-assimp` to build from source (best compatibility)
- `--features system` to link an existing installation

## Development and Testing

For development work or when prebuilt binaries are not available:

```toml
# Use this for development
asset-importer = { version = "0.4", features = ["build-assimp"] }
```

This ensures you can always build from source regardless of release availability.

## Architecture

This crate provides a high-level safe API. For low-level FFI bindings, see [`asset-importer-sys`](asset-importer-sys/).

## Platform Support

- **Windows**: MSVC, MinGW
- **macOS**: Intel, Apple Silicon  
- **Linux**: x86_64, aarch64

## Goals

This project aims to provide the most comprehensive and up-to-date Rust binding for Assimp, supporting both import and export functionality with modern Rust practices.

## Versioning

This workspace uses independent versioning for each crate:

- **`asset-importer-sys`**: Tracks Assimp versions and FFI binding changes
- **`asset-importer`**: Tracks high-level API changes and features

See [VERSIONING.md](VERSIONING.md) for detailed versioning strategy and release process.

## Limitations

- **Limited Testing**: Needs more real-world usage validation
- **API Stability**: May change before 1.0 release

## Contributing

Contributions welcome! Areas needing help:

- Real-world testing with various file formats
- Performance benchmarking
- Documentation improvements
- Platform-specific testing

## Related Projects

If you're working with graphics and UI in Rust, you might also be interested in:

- **[dear-imgui](https://github.com/Latias94/dear-imgui)** - Comprehensive Dear ImGui bindings for Rust using C++ bindgen, providing immediate mode GUI capabilities for graphics applications

## Acknowledgments

- [Assimp](https://github.com/assimp/assimp) - The underlying C++ library
- [russimp](https://github.com/jkvargas/russimp) - Inspiration and reference

