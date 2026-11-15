# asset-importer-sys

[![asset-importer-sys (crates.io)](https://img.shields.io/crates/v/asset-importer-sys.svg)](https://crates.io/crates/asset-importer-sys)
[![asset-importer-sys Downloads](https://img.shields.io/crates/d/asset-importer-sys.svg)](https://crates.io/crates/asset-importer-sys)

Low-level FFI bindings for the [Assimp](https://github.com/assimp/assimp) 3D asset import library.

This crate provides unsafe Rust FFI bindings for **Assimp v6.0.6**, implementing the vast majority of the C API functions, types, and constants.

## Status

⚠️ **Early Development**: These bindings are functional but need more testing across different platforms and use cases.

## Overview

This crate provides unsafe Rust bindings to the [Assimp v6.0.6](https://github.com/assimp/assimp/releases/tag/v6.0.6) C API, implementing the vast majority of functions, types, and constants. For a safe, high-level API, use the [`asset-importer`](../asset-importer/) crate instead.

### API Coverage

The bindings include:
- **Core Import/Export Functions**: All major scene loading and saving functions
- **Data Structures**: Complete type definitions for scenes, meshes, materials, animations, etc.
- **Post-Processing**: All available post-processing steps and configurations
- **Metadata Access**: Scene and node metadata querying
- **Memory Management**: Proper resource cleanup and memory handling
- **Logging System**: Integration with Assimp's logging infrastructure

## Build Features

**Default**: Builds Assimp from the bundled source (vendored) for maximum compatibility.

The high-level [`asset-importer`](../asset-importer/) crate follows the same default source-build
policy. Enable `prebuilt` only when you explicitly want matching release artifacts.

### Build from Source
```toml
[dependencies]
asset-importer-sys = "0.9"

# Equivalent explicit source-build mode:
asset-importer-sys = { version = "0.9", features = ["build-assimp"] }
```
- Builds Assimp from bundled source
- Requires: CMake and a C++17 compiler; Git only for repository submodules
- Uses shipped bindings without LLVM/libclang or bindgen
- Best compatibility across platforms

### Prebuilt Binaries
```toml
[dependencies]
asset-importer-sys = { version = "0.9", features = ["prebuilt"] }
```
- Downloads prebuilt libraries from GitHub releases
- Requires a C++17 compiler for the bridge; skips CMake and the Assimp source build
- Uses shipped bindings without LLVM/libclang or bindgen
- Fastest option for development
- Requires a package whose manifest matches the bundled Assimp version

### System Library
```toml
asset-importer-sys = { version = "0.9", features = ["system", "generate-bindings"] }
```
Uses system-installed assimp. Install via package manager:
- **macOS**: `brew install assimp`
- **Ubuntu/Debian**: `sudo apt install libassimp-dev`
- **Windows**: Use vcpkg or manual installation

This mode requires libclang to regenerate bindings from the installed headers. The high-level
crate's `system` feature enables `generate-bindings` automatically; direct sys consumers enable both.

On Windows with vcpkg, pick a triplet that matches your desired CRT:
- `x64-windows` → dynamic CRT (/MD)
- `x64-windows-static` → static CRT (/MT)

Note: vcpkg-rs requires `VCPKGRS_DYNAMIC=1` for dynamic triplets like `x64-windows`. The build
script sets this automatically for `system` builds, but you may still want to set it explicitly in
custom environments.

If you use the static triplet, also enable Rust `crt-static` so all crates agree:

```
# .cargo/config.toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

### Static Linking
```toml
asset-importer-sys = { version = "0.9", features = ["static-link", "build-assimp"] }
```
- Creates single executable with fewer external runtime dependencies
- Larger binary size

### Additional Options
```toml
asset-importer-sys = {
    version = "0.9",
    features = [
        "build-assimp",
        "export",        # Enable export functionality
        "mint",          # Math library interop
        "type-extensions" # Convenience methods
    ]
}
```

## Environment Variables

- `ASSIMP_DIR`: Path to an Assimp source tree to use when the `assimp` submodule is not present (builds from this directory; must match the bundled Assimp 6.0.6 version).
- `ASSET_IMPORTER_PACKAGE_DIR`: Use local prebuilt packages
  - On Windows we publish both `-md` and `-mt` suffixed archives; the build script selects the right one automatically and falls back to the old name if not found.
- `ASSET_IMPORTER_FORCE_GENERATE_BINDINGS=1`: Maintainer override to regenerate vendored bindings when `generate-bindings` is enabled.
- `CMAKE_GENERATOR`: Override CMake generator (e.g., "Ninja")

## Platform Support

### Tested Platforms
- Windows (MSVC, MinGW)
- macOS (Intel, Apple Silicon)
- Linux (x86_64)

### Build Requirements

| Feature | Requirements |
|---------|-------------|
| Default vendored source | CMake (≥3.22), C++17 compiler |
| `prebuilt` | C++17 compiler for the Rust bridge; no CMake or Assimp source build |
| `system` | System assimp installation, libclang/bindgen, C++17 compiler for the bridge |
| `build-assimp` | CMake (≥3.22), C++17 compiler |

Source and prebuilt builds use pregenerated bindings and do not require libclang. Git is only needed
to initialize submodules in a repository checkout; crates.io source packages include Assimp.

#### Detailed Requirements for Source Build

**Essential Tools:**
- **CMake 3.22+**: Build system generator
- **C++ Compiler**:
  - Windows: MSVC 2019+ (Visual Studio Build Tools)
  - macOS: Clang (Xcode Command Line Tools)
  - Linux: GCC 7+ or Clang 6+
- **Git**: For submodule management in repository checkouts

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

unsafe {
    let filename = CString::new("model.obj").unwrap();
    
    let scene = sys::aiImportFile(
        filename.as_ptr(),
        (sys::aiPostProcessSteps::aiProcess_Triangulate.0)
            | (sys::aiPostProcessSteps::aiProcess_FlipUVs.0)
    );
    
    if !scene.is_null() {
        let scene_ref = &*scene;
        println!("Loaded {} meshes", scene_ref.mNumMeshes);
        sys::aiReleaseImport(scene);
    }
}
```

## Goals

Provides comprehensive FFI bindings for the latest Assimp with flexible build options and optional convenience features.

## Native enums

Native enum types are transparent integer newtypes with associated constants. Read the underlying value through `.0` (for example, `aiTextureType::aiTextureType_DIFFUSE.0`) and construct unknown values through the tuple constructor. This preserves file-provided values and combined flags without creating invalid Rust enum discriminants.

## Bindings Generation

`src/bindings_pregenerated.rs` is generated with [bindgen](https://github.com/rust-lang/rust-bindgen)
from the bundled Assimp headers and is included in the published crate. Default source builds,
explicit `build-assimp` builds, prebuilt consumers, and docs.rs copy this file without running
bindgen. The generated bindings include:

- All `ai*` functions
- All `ai*` types and constants
- Proper struct layouts for FFI

Only `system` builds regenerate bindings automatically, because their installed headers can differ
from the bundled version. For non-system builds, enabling `generate-bindings` makes bindgen available
but still prefers the shipped file. Maintainers also set `ASSET_IMPORTER_FORCE_GENERATE_BINDINGS=1`
to regenerate it.

After updating the Assimp submodule and `assimp-version.txt`, install libclang and the Rust `rustfmt`
component, then run from the repository root on native Linux or macOS:

```console
python3 tools/update_bindings.py
python3 tools/update_bindings.py --check
```

Set `LIBCLANG_PATH` if libclang is not discoverable. The update tool invokes the existing Cargo build
with `build-assimp,generate-bindings`, locates the exact output from Cargo's JSON messages, and updates
only the pregenerated file.
`--check` reports differences without changing the checked-in file. Review generated changes and
commit them with the Assimp upgrade.

MSVC gives some native enums a different signedness. Its regenerated output is compiled separately
in CI and must not replace the shared pregenerated file. On Windows, validate fresh bindings with
`ASSET_IMPORTER_FORCE_GENERATE_BINDINGS=1`, feature `asset-importer-sys/generate-bindings`, and
`LIBCLANG_PATH` pointing to the LLVM `bin` directory.

CI checks regeneration on Linux and compiles the safe API against fresh MSVC bindings. Source and
prebuilt test jobs separately assert that `bindgen` and `clang-sys` are absent from their dependency
graphs, then run the tests with the shipped bindings. The minimum Rust version also uses the shipped
bindings, so a bindgen upgrade cannot hide incompatible pregenerated output.

## Safety

This crate provides `unsafe` bindings. Memory management, null pointer checks, and proper API usage are the caller's responsibility. Consider using the safe [`asset-importer`](../asset-importer/) wrapper instead.

The Rust bridge import functions return an opaque `aiRustScene` owner. Borrow its scene with `aiGetSceneRust` and release the owner with `aiReleaseSceneRust`; the borrowed scene must not be released through Assimp's C API. Custom `aiFileIO` tables and their user data must remain alive until the owner is released, including any later post-processing. Progress callbacks are detached before the import returns. Use `aiApplyPostProcessingRust` and `aiGetMemoryRequirementsRust` with bridge-owned scenes.

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
