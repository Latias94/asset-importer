# asset-importer

A comprehensive Rust binding for the latest [Assimp](https://github.com/assimp/assimp) 3D asset import library.

## Status

⚠️ **Early Development**: This library is functional but lacks extensive real-world testing. Use with caution in production environments.

## Features

- **Import Support**: 71+ 3D file formats (OBJ, FBX, glTF, DAE, etc.)
- **Export Support**: 22+ output formats (optional)
- **Memory Safe**: Safe Rust API over unsafe FFI bindings
- **Modern Math**: Integration with glam for vectors and matrices
- **Flexible Building**: Multiple build options for different use cases

## Quick Start

Add to your `Cargo.toml` (choose a build method):

```toml
[dependencies]
# Fastest option - prebuilt binaries
asset-importer = { version = "0.1", features = ["prebuilt"] }

# Or use system-installed assimp
asset-importer = { version = "0.1", features = ["system"] }
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

### Prebuilt Binaries (Recommended)
```toml
asset-importer = { features = ["prebuilt"] }
```
- Fastest builds, no dependencies
- Downloads from GitHub releases
- Good for development and CI

### System Library
```toml
asset-importer = { features = ["system"] }
```
- Uses system-installed assimp
- Install via: `brew install assimp` (macOS), `apt install libassimp-dev` (Ubuntu)

### Build from Source
```toml
asset-importer = { features = ["build-assimp"] }
```
- Requires: CMake, C++ compiler
- Full control over build configuration

### Additional Features
```toml
asset-importer = { 
    features = [
        "build-assimp",
        "export",      # Enable export functionality
        "static",      # Static linking
        "nozlib"       # Disable zlib
    ] 
}
```

## Architecture

This crate provides a high-level safe API. For low-level FFI bindings, see [`asset-importer-sys`](asset-importer-sys/).

## Platform Support

- **Windows**: MSVC, MinGW
- **macOS**: Intel, Apple Silicon  
- **Linux**: x86_64, aarch64

## Goals

This project aims to provide the most comprehensive and up-to-date Rust binding for Assimp, supporting both import and export functionality with modern Rust practices.

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

## Acknowledgments

- [Assimp](https://github.com/assimp/assimp) - The underlying C++ library
- [russimp](https://github.com/jkvargas/russimp) - Inspiration and reference
