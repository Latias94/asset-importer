use std::{env, fs, path::PathBuf};
#[cfg(feature = "prebuilt")]
use flate2::read::GzDecoder;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=assimp");

    // Check if assimp submodule exists
    let assimp_dir = manifest_dir.join("assimp");
    if !assimp_dir.exists() || !assimp_dir.join("include").exists() {
        panic!(
            "Assimp submodule not found at {}. Please run:\n\
             git submodule update --init --recursive",
            assimp_dir.display()
        );
    }

    // Determine build strategy and generate bindings
    let built_include_dir = if cfg!(feature = "system") {
        // Explicitly use system assimp
        link_system_assimp();
        None
    } else if cfg!(feature = "build-assimp") || cfg!(feature = "static") {
        // Explicitly build from source
        build_assimp_from_source(&manifest_dir, &out_dir);
        // Use the built include directory which has config.h
        Some(out_dir.join("build").join("include"))
    } else {
        // Default: use prebuilt binaries for fast, friction-free builds
        link_prebuilt_assimp(&out_dir);
        None
    };

    generate_bindings(&manifest_dir, &out_dir, built_include_dir.as_deref());

    // Build the C++ bridge (progress + IOSystem wrappers)
    compile_bridge_cpp(&manifest_dir, built_include_dir.as_deref());
}

fn link_system_assimp() {
    // Try to find and link system assimp
    if !try_system_assimp() {
        panic!(
            "System assimp library not found!\n\
             \n\
             To fix this issue, you have several options:\n\
             \n\
             1. Install assimp system-wide:\n\
             \n\
             Windows (vcpkg):\n\
               vcpkg install assimp\n\
             \n\
             macOS (Homebrew):\n\
               brew install assimp\n\
             \n\
             Ubuntu/Debian:\n\
               sudo apt install libassimp-dev\n\
             \n\
             2. Or use a different build method:\n\
             \n\
             Build from source (recommended):\n\
               cargo build  # (default behavior)\n\
             \n\
             Use prebuilt binaries (when available):\n\
               cargo build --features prebuilt\n\
             "
        );
    }
}

fn try_system_assimp() -> bool {
    // Try common system paths
    #[cfg(target_os = "windows")]
    {
        // Try vcpkg
        if env::var("VCPKG_ROOT").is_ok() {
            println!("cargo:rustc-link-lib=assimp");
            return true;
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Try homebrew paths
        let homebrew_paths = [
            "/opt/homebrew/lib", // Apple Silicon
            "/usr/local/lib",    // Intel
        ];

        for path in &homebrew_paths {
            let lib_path = PathBuf::from(path).join("libassimp.dylib");
            if lib_path.exists() {
                println!("cargo:rustc-link-search=native={}", path);
                println!("cargo:rustc-link-lib=assimp");
                return true;
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try common Linux paths
        let linux_paths = ["/usr/lib", "/usr/local/lib", "/usr/lib/x86_64-linux-gnu"];

        for path in &linux_paths {
            let lib_path = PathBuf::from(path).join("libassimp.so");
            if lib_path.exists() {
                println!("cargo:rustc-link-search=native={}", path);
                println!("cargo:rustc-link-lib=assimp");
                return true;
            }
        }
    }

    false
}

fn link_prebuilt_assimp(out_dir: &std::path::Path) {
    let target = env::var("TARGET").unwrap();
    let crate_version = env::var("CARGO_PKG_VERSION").unwrap();

    // Determine link type
    let link_type = if cfg!(feature = "static") {
        "static"
    } else {
        "dylib"
    };

    let archive_name = format!(
        "asset-importer-{}-{}-{}.tar.gz",
        crate_version, target, link_type
    );

    let ar_src_dir = if let Ok(package_dir) = env::var("ASSET_IMPORTER_PACKAGE_DIR") {
        // Use local package directory if specified
        PathBuf::from(package_dir)
    } else {
        // Download from GitHub releases
        download_prebuilt_package(out_dir, &archive_name);
        out_dir.to_path_buf()
    };

    // Extract the archive
    extract_prebuilt_package(&ar_src_dir, &archive_name, out_dir, link_type);
}

fn download_prebuilt_package(out_dir: &std::path::Path, archive_name: &str) {
    let crate_version = env::var("CARGO_PKG_VERSION").unwrap();
    let download_url = format!(
        "https://github.com/Latias94/asset-importer/releases/download/v{}/{}",
        crate_version, archive_name
    );

    let archive_path = out_dir.join(archive_name);

    // Skip download if file already exists
    if archive_path.exists() {
        println!(
            "cargo:warning=Using existing prebuilt package: {}",
            archive_path.display()
        );
        return;
    }

    println!(
        "cargo:warning=Downloading prebuilt package from: {}",
        download_url
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .expect("Failed to create HTTP client");

    let response = client
        .get(&download_url)
        .send()
        .expect("Failed to download prebuilt package");

    if !response.status().is_success() {
        panic!(
            "Failed to download prebuilt package from {}. Status: {}. \
             Consider using 'build-assimp' feature to build from source instead.",
            download_url,
            response.status()
        );
    }

    let bytes = response.bytes().expect("Failed to read response bytes");
    fs::write(&archive_path, &bytes).expect("Failed to write downloaded package");

    println!(
        "cargo:warning=Downloaded prebuilt package: {}",
        archive_path.display()
    );
}

fn extract_prebuilt_package(
    ar_src_dir: &std::path::Path,
    archive_name: &str,
    out_dir: &std::path::Path,
    link_type: &str,
) {
    let archive_path = ar_src_dir.join(archive_name);

    if !archive_path.exists() {
        panic!(
            "Prebuilt package not found at {}. \
             Consider using 'build-assimp' feature to build from source instead.",
            archive_path.display()
        );
    }

    let file = fs::File::open(&archive_path).expect("Failed to open prebuilt package");

    let mut archive = tar::Archive::new(GzDecoder::new(file));
    let extract_dir = out_dir.join(link_type);

    archive
        .unpack(&extract_dir)
        .expect("Failed to extract prebuilt package");

    // Set up library search paths
    println!(
        "cargo:rustc-link-search=native={}",
        extract_dir.join("lib").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        extract_dir.join("bin").display()
    );

    // Link the library (auto-detect Windows import lib name like assimp-vc143-mt.lib)
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let mut lib_name = String::from("assimp");
    if target_os == "windows" {
        let lib_dir = extract_dir.join("lib");
        if let Ok(read) = fs::read_dir(&lib_dir) {
            for entry in read.flatten() {
                let p = entry.path();
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    let lower = name.to_ascii_lowercase();
                    if lower.starts_with("assimp") && lower.ends_with(".lib") {
                        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                            lib_name = stem.to_string();
                            break;
                        }
                    }
                }
            }
        }
    }
    if link_type == "static" {
        println!("cargo:rustc-link-lib=static={}", lib_name);
    } else {
        println!("cargo:rustc-link-lib={}", lib_name);
    }

    // Link system dependencies
    link_system_dependencies();

    println!(
        "cargo:warning=Using prebuilt assimp from: {}",
        extract_dir.display()
    );
}

#[allow(dead_code)]
fn link_system_dependencies() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    match target_os.as_str() {
        "windows" => {
            if target_env == "msvc" {
                println!("cargo:rustc-link-lib=user32");
                println!("cargo:rustc-link-lib=gdi32");
                println!("cargo:rustc-link-lib=shell32");
                println!("cargo:rustc-link-lib=ole32");
                println!("cargo:rustc-link-lib=oleaut32");
                println!("cargo:rustc-link-lib=uuid");
                println!("cargo:rustc-link-lib=advapi32");
            } else {
                println!("cargo:rustc-link-lib=stdc++");
            }
        }
        "macos" => {
            println!("cargo:rustc-link-lib=c++");
            println!("cargo:rustc-link-lib=framework=Foundation");
        }
        _ => {
            println!("cargo:rustc-link-lib=stdc++");
        }
    }

    // Link zlib unless disabled
    if !cfg!(feature = "nozlib") {
        if target_os == "windows" {
            println!("cargo:rustc-link-lib=static=zlibstatic");
        } else {
            println!("cargo:rustc-link-lib=z");
        }
    }
}

#[allow(dead_code)]
fn generate_bindings_from_system(manifest_dir: &std::path::Path, out_dir: &std::path::Path) {
    let wrapper_h = manifest_dir.join("wrapper.h");

    let bindings = bindgen::Builder::default()
        .header(wrapper_h.to_str().unwrap())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_type("ai.*")
        .allowlist_function("ai.*")
        .allowlist_var("ai.*")
        .allowlist_var("AI_.*")
        .derive_partialeq(true)
        .derive_eq(true)
        .derive_hash(true)
        .derive_debug(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = out_dir.join("bindings.rs");
    bindings
        .write_to_file(out_path)
        .expect("Couldn't write bindings!");
}

fn build_assimp_from_source(manifest_dir: &std::path::Path, _out_path: &std::path::Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let assimp_src = manifest_dir.join("assimp");

    // Check if assimp source exists
    if !assimp_src.exists() {
        panic!(
            "Assimp source not found at {}. Please run: git submodule update --init --recursive",
            assimp_src.display()
        );
    }

    let mut config = cmake::Config::new(&assimp_src);

    // Configure CMake options
    config
        .define("ASSIMP_BUILD_TESTS", "OFF")
        .define("ASSIMP_BUILD_SAMPLES", "OFF")
        .define("ASSIMP_BUILD_ASSIMP_TOOLS", "OFF")
        .define("BUILD_SHARED_LIBS", "OFF")
        // Disable being overly strict with warnings, which can cause build issues
        .define("ASSIMP_WARNINGS_AS_ERRORS", "OFF");

    // Configure zlib based on nozlib feature
    if cfg!(feature = "nozlib") {
        config.define("ASSIMP_BUILD_ZLIB", "OFF");
    } else {
        config.define("ASSIMP_BUILD_ZLIB", "ON");
    }

    // Enable export functionality if requested
    #[cfg(feature = "export")]
    config.define("ASSIMP_BUILD_NO_EXPORT", "OFF");

    #[cfg(not(feature = "export"))]
    config.define("ASSIMP_BUILD_NO_EXPORT", "ON");

    // Force Release build to avoid runtime library conflicts
    config.profile("Release");

    // Platform-specific configurations
    match target_os.as_str() {
        "windows" => {
            if target_env == "msvc" {
                // Use MultiThreaded runtime library (Release version)
                config.define("CMAKE_MSVC_RUNTIME_LIBRARY", "MultiThreaded");
                // Enable C++ exception handling for MSVC
                config.define("CMAKE_CXX_FLAGS", "/EHsc");
                config.define("CMAKE_CXX_FLAGS_DEBUG", "/EHsc");
                config.define("CMAKE_CXX_FLAGS_RELEASE", "/EHsc");
            }
        }
        "macos" => {
            config.define("CMAKE_OSX_DEPLOYMENT_TARGET", "10.12");
        }
        _ => {}
    }

    let dst = config.build();

    // Link the built library
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-search=native={}/lib64", dst.display());

    // Also search in the build directory structure
    println!("cargo:rustc-link-search=native={}/build/lib", dst.display());
    println!(
        "cargo:rustc-link-search=native={}/build/lib64",
        dst.display()
    );

    // Add the Release subdirectory for MSVC builds (we force Release mode)
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env == "msvc" {
        println!(
            "cargo:rustc-link-search=native={}/build/lib/Release",
            dst.display()
        );
        println!(
            "cargo:rustc-link-search=native={}/build/contrib/zlib/Release",
            dst.display()
        );
    }

    // Link to the built assimp library
    // The library name depends on the build configuration
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env == "msvc" {
        // MSVC Release builds use "assimp-vc143-mt" (no 'd' suffix)
        #[cfg(feature = "static")]
        println!("cargo:rustc-link-lib=static=assimp-vc143-mt");

        #[cfg(not(feature = "static"))]
        println!("cargo:rustc-link-lib=assimp-vc143-mt");

        // Link to zlib (built by assimp) unless disabled
        if !cfg!(feature = "nozlib") {
            println!("cargo:rustc-link-lib=static=zlibstatic");
        }
    } else {
        #[cfg(feature = "static")]
        println!("cargo:rustc-link-lib=static=assimp");

        #[cfg(not(feature = "static"))]
        println!("cargo:rustc-link-lib=assimp");

        // Link to zlib unless disabled
        if !cfg!(feature = "nozlib") {
            println!("cargo:rustc-link-lib=z");
        }
    }

    // Link system dependencies
    match target_os.as_str() {
        "windows" => {
            if target_env == "msvc" {
                println!("cargo:rustc-link-lib=user32");
                println!("cargo:rustc-link-lib=gdi32");
                println!("cargo:rustc-link-lib=shell32");
                println!("cargo:rustc-link-lib=ole32");
                println!("cargo:rustc-link-lib=oleaut32");
                println!("cargo:rustc-link-lib=uuid");
                println!("cargo:rustc-link-lib=advapi32");
            } else {
                println!("cargo:rustc-link-lib=stdc++");
            }
        }
        "macos" => {
            println!("cargo:rustc-link-lib=c++");
            println!("cargo:rustc-link-lib=framework=Foundation");
        }
        _ => {
            println!("cargo:rustc-link-lib=stdc++");
        }
    }

    // Export include path for bindgen
    let include_path = dst.join("include");
    println!("cargo:include={}", include_path.display());
}

fn generate_bindings(
    manifest_dir: &std::path::Path,
    out_path: &std::path::Path,
    built_include_dir: Option<&std::path::Path>,
) {
    let wrapper_h = manifest_dir.join("wrapper.h");
    let assimp_include = manifest_dir.join("assimp").join("include");

    // Create empty config.h if it doesn't exist (needed for system builds)
    let config_file = assimp_include.join("assimp").join("config.h");
    let config_exists = config_file.exists();
    if !config_exists {
        if let Some(parent) = config_file.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create assimp include directory");
        }
        std::fs::write(&config_file, "")
            .expect("Unable to write config.h to assimp/include/assimp/");
    }

    let mut builder = bindgen::Builder::default()
        .header(wrapper_h.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    // Add include paths based on build type
    if let Some(built_include) = built_include_dir {
        // Use built include directory first (has config.h), then submodule include
        builder = builder
            .clang_arg(format!("-I{}", built_include.display()))
            .clang_arg(format!("-I{}", assimp_include.display()));
    } else {
        // Use submodule include directory only
        builder = builder.clang_arg(format!("-I{}", assimp_include.display()));
    }

    builder = builder
        .allowlist_function("ai.*")
        .allowlist_type("ai.*")
        .allowlist_var("ai.*")
        .allowlist_var("AI_.*")
        // Derive common traits
        .derive_default(true)
        .derive_debug(true)
        .derive_copy(true)
        // Don't derive PartialEq and Hash for types with function pointers
        // to avoid clippy warnings about function pointer comparisons
        .no_partialeq("aiLogStream")
        .no_hash("aiLogStream")
        .no_partialeq("aiFileIO")
        .no_hash("aiFileIO")
        .no_partialeq("aiFile")
        .no_hash("aiFile")
        // Layout tests can be flaky across platforms
        .layout_tests(false)
        // Generate comments but disable doctests to avoid C-style code examples
        .generate_comments(true)
        .disable_untagged_union()
        // Force enums to be i32 to ensure consistency across platforms
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .constified_enum_module(".*");

    // Add include paths from environment (for system builds)
    if let Ok(include_path) = env::var("DEP_ASSIMP_INCLUDE") {
        builder = builder.clang_arg(format!("-I{}", include_path));
    }

    // Add system include paths if using system assimp
    if let Ok(include_paths) = env::var("DEP_ASSIMP_INCLUDE_PATHS") {
        for path in include_paths.split(':') {
            if !path.is_empty() {
                builder = builder.clang_arg(format!("-I{}", path));
            }
        }
    }

    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_file = out_path.join("bindings.rs");
    bindings
        .write_to_file(&out_file)
        .expect("Couldn't write bindings!");

    // Clean up temporary config.h if we created it
    if !config_exists {
        let _ = std::fs::remove_file(config_file);
    }
}

fn compile_bridge_cpp(manifest_dir: &std::path::Path, built_include_dir: Option<&std::path::Path>) {
    let mut build = cc::Build::new();
    build.cpp(true);
    build.file(manifest_dir.join("wrapper.cpp"));

    // Include paths for Assimp headers
    build.include(manifest_dir.join("assimp").join("include"));
    if let Some(dir) = built_include_dir {
        build.include(dir);
    }

    // On Windows, ensure exceptions for MSVC (consistent with Assimp build)
    #[cfg(target_env = "msvc")]
    {
        build.flag("/EHsc");
    }

    build.compile("assimp_rust_bridge");
}
