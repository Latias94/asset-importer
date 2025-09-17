# asset-importer-sys

Low-level FFI bindings for the [Assimp](https://github.com/assimp/assimp) 3D asset import library.

## Status

⚠️ **Early Development**: These bindings are functional but need more testing across different platforms and use cases.

## Overview

This crate provides unsafe Rust bindings to the Assimp C API. For a safe, high-level API, use the [`asset-importer`](../asset-importer/) crate instead.

## Build Features

**Default**: Builds from source for best compatibility.

### System Library
```toml
[dependencies]
asset-importer-sys = { features = ["system"] }
```
Uses system-installed assimp. Install via package manager:
- **macOS**: `brew install assimp`
- **Ubuntu/Debian**: `sudo apt install libassimp-dev`
- **Windows**: Use vcpkg or manual installation

### Build from Source (Explicit)
```toml
asset-importer-sys = { features = ["build-assimp"] }
```
- Explicitly builds assimp from bundled source
- Requires: CMake, C++ compiler, Git
- Full control over build configuration

### Prebuilt Binaries
```toml
asset-importer-sys = { features = ["prebuilt"] }
```
- Downloads prebuilt libraries from GitHub releases
- No build dependencies required
- Fastest option for development

### Static Linking
```toml
asset-importer-sys = { features = ["static-link", "build-assimp"] }
```
- Creates single executable with fewer external runtime dependencies
- Larger binary size

### Additional Options
```toml
asset-importer-sys = { 
    features = [
        "build-assimp",
        "export",        # Enable export functionality  
        "nozlib",        # Don't link zlib
        "mint",          # Math library interop
        "type-extensions" # Convenience methods
    ]
}
```

## Environment Variables

- `ASSET_IMPORTER_PACKAGE_DIR`: Use local prebuilt packages
- `CMAKE_GENERATOR`: Override CMake generator (e.g., "Ninja")

## Platform Support

### Tested Platforms
- Windows (MSVC, MinGW)
- macOS (Intel, Apple Silicon)
- Linux (x86_64)

### Build Requirements

| Feature | Requirements |
|---------|-------------|
| `prebuilt` | None |
| `system` | System assimp installation |
| `build-assimp` | CMake, C++ compiler |

## Usage

```rust
use asset_importer_sys as sys;
use std::ffi::CString;
use std::ptr;

unsafe {
    let importer = sys::aiCreateImporter();
    let filename = CString::new("model.obj").unwrap();
    
    let scene = sys::aiImportFile(
        filename.as_ptr(),
        sys::aiProcess_Triangulate | sys::aiProcess_FlipUVs
    );
    
    if !scene.is_null() {
        let scene_ref = &*scene;
        println!("Loaded {} meshes", scene_ref.mNumMeshes);
        sys::aiReleaseImporter(importer);
    }
}
```

## Goals

Provides comprehensive FFI bindings for the latest Assimp with flexible build options and optional convenience features.

## Bindings Generation

Bindings are generated using [bindgen](https://github.com/rust-lang/rust-bindgen) from the Assimp headers. The generated bindings include:

- All `ai*` functions
- All `ai*` types and constants
- Proper struct layouts for FFI

## Safety

This crate provides `unsafe` bindings. Memory management, null pointer checks, and proper API usage are the caller's responsibility. Consider using the safe [`asset-importer`](../asset-importer/) wrapper instead.

## Contributing

Help needed with:
- Testing on more platforms
- Improving build reliability
- Documentation for advanced features

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE))
- MIT License ([LICENSE-MIT](../LICENSE-MIT))

at your option.
