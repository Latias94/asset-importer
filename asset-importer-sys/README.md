# asset-importer-sys

Low-level FFI bindings for the [Assimp](https://github.com/assimp/assimp) 3D asset import library.

This crate provides unsafe Rust FFI bindings for **Assimp v6.0.2**, implementing the vast majority of the C API functions, types, and constants.

## Status

⚠️ **Early Development**: These bindings are functional but need more testing across different platforms and use cases.

## Overview

This crate provides unsafe Rust bindings to the [Assimp v6.0.2](https://github.com/assimp/assimp/releases/tag/v6.0.2) C API, implementing the vast majority of functions, types, and constants. For a safe, high-level API, use the [`asset-importer`](../asset-importer/) crate instead.

### API Coverage

The bindings include:
- **Core Import/Export Functions**: All major scene loading and saving functions
- **Data Structures**: Complete type definitions for scenes, meshes, materials, animations, etc.
- **Post-Processing**: All available post-processing steps and configurations
- **Metadata Access**: Scene and node metadata querying
- **Memory Management**: Proper resource cleanup and memory handling
- **Logging System**: Integration with Assimp's logging infrastructure

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

- `ASSIMP_DIR`: Path to an Assimp source tree to use when the `assimp` submodule is not present (builds from this directory).
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
| `build-assimp` | CMake (≥3.10), C++ compiler, Git |

#### Detailed Requirements for Source Build

**Essential Tools:**
- **CMake 3.10+**: Build system generator
- **C++ Compiler**:
  - Windows: MSVC 2019+ (Visual Studio Build Tools)
  - macOS: Clang (Xcode Command Line Tools)
  - Linux: GCC 7+ or Clang 6+
- **Git**: For submodule management

**Platform Setup:**

**Windows:**
```cmd
# Install Visual Studio Build Tools or Visual Studio Community
# Or use chocolatey:
choco install cmake visualstudio2022buildtools
```

**macOS:**
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install CMake via Homebrew (recommended)
brew install cmake
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt update
sudo apt install build-essential cmake git
```

**Linux (CentOS/RHEL):**
```bash
sudo yum groupinstall "Development Tools"
sudo yum install cmake3 git
```

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
