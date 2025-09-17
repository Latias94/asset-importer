# asset-importer

[![Crates.io](https://img.shields.io/crates/v/asset-importer.svg)](https://crates.io/crates/asset-importer)
[![Documentation](https://docs.rs/asset-importer/badge.svg)](https://docs.rs/asset-importer)
[![Rust Version](https://img.shields.io/badge/rust-1.85+-blue.svg)](https://www.rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/asset-importer.svg)](https://crates.io/crates/asset-importer)
[![Made with Rust](https://img.shields.io/badge/made%20with-Rust-orange.svg)](https://www.rust-lang.org)

A comprehensive Rust binding for the latest [Assimp](https://github.com/assimp/assimp) 3D asset import library.

This crate provides safe, high-level Rust bindings for **Assimp v6.0.2**, implementing the vast majority of the C API with idiomatic Rust interfaces.

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
# Default – Build from source (best compatibility)
asset-importer = "0.1"

# Or use system-installed assimp
asset-importer = { version = "0.1", features = ["system"] }

# Or use prebuilt binaries (fastest, requires release)
asset-importer = { version = "0.1", features = ["prebuilt"] }
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

### Default: Build from Source

```toml
asset-importer = "0.1"
```

- **Best compatibility**: Works on all platforms
- **Full control**: Latest Assimp version with all features
- **Requires**: CMake, C++ compiler (automatically handled by Cargo)

### System Library

```toml
asset-importer = { version = "0.1", features = ["system"] }
```

- **Lightweight**: Uses existing system installation
- **Setup required**: Install via `brew install assimp` (macOS), `apt install libassimp-dev` (Ubuntu)
- **Version dependent**: Behavior may vary based on system library version

### Prebuilt Binaries

```toml
asset-importer = { version = "0.1", features = ["prebuilt"] }
```

- **Fastest**: No compilation time
- **Convenient**: No native toolchain required
- **Requires**: Available release artifacts

### Additional Features

```toml
asset-importer = {
    features = [
        "export",        # Enable export functionality
        "static-link",   # Prefer static linking (source/prebuilt)
        "nozlib"         # Disable zlib
    ]
}
```

## Build Requirements

### Source Build Dependencies

When building from source (default), you'll need:

- **CMake** (3.10 or later)
- **C++ Compiler** (MSVC on Windows, GCC/Clang on Linux/macOS)
- **Git** (for submodules)

### Platform-Specific Setup

- **Windows**: Install Visual Studio Build Tools or Visual Studio
- **macOS**: Install Xcode Command Line Tools (`xcode-select --install`)
- **Linux**: Install build essentials (`sudo apt install build-essential cmake` on Ubuntu)

For detailed platform-specific instructions and troubleshooting, see the [asset-importer-sys README](https://github.com/Latias94/asset-importer/blob/main/asset-importer-sys/README.md#build-requirements).

## Why build from source by default?

- **Best compatibility**: Works reliably across all platforms and environments
- **Latest features**: Always uses the most recent Assimp version
- **No external dependencies**: Self-contained build process
- **CI/CD friendly**: No need for prebuilt artifacts during development

If you prefer faster builds or have constraints, use:

- `--features prebuilt` to use prebuilt binaries (when available)
- `--features system` to link an existing installation

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
- **Documentation**: Some advanced features need better docs
- **Performance**: Not yet benchmarked against russimp

## Contributing

Contributions welcome! Areas needing help:

- Real-world testing with various file formats
- Performance benchmarking
- Documentation improvements
- Platform-specific testing

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Related Projects

If you're working with graphics and UI in Rust, you might also be interested in:

- **[dear-imgui](https://github.com/Latias94/dear-imgui)** - Comprehensive Dear ImGui bindings for Rust using C++ bindgen, providing immediate mode GUI capabilities for graphics applications

## Acknowledgments

- [Assimp](https://github.com/assimp/assimp) - The underlying C++ library
- [russimp](https://github.com/jkvargas/russimp) - Inspiration and reference
