use flate2::read::GzDecoder;
use std::{env, fs, path::PathBuf};

/// Build configuration containing all environment and target information
#[derive(Debug, Clone)]
struct BuildConfig {
    manifest_dir: PathBuf,
    out_dir: PathBuf,
    target_os: String,
    target_env: String,
    target_features: String,
    profile: String,
    force_build: bool,
}

impl BuildConfig {
    fn new() -> Self {
        Self {
            manifest_dir: PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()),
            out_dir: PathBuf::from(env::var("OUT_DIR").unwrap()),
            target_os: env::var("CARGO_CFG_TARGET_OS").unwrap_or_default(),
            target_env: env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default(),
            target_features: env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default(),
            profile: env::var("PROFILE").unwrap_or_else(|_| "release".to_string()),
            force_build: env::var("ASSET_IMPORTER_FORCE_BUILD").is_ok(),
        }
    }

    fn is_windows(&self) -> bool {
        self.target_os == "windows"
    }

    fn is_msvc(&self) -> bool {
        self.target_env == "msvc"
    }

    fn is_debug(&self) -> bool {
        self.profile == "debug"
    }

    fn use_static_crt(&self) -> bool {
        self.is_windows()
            && self.is_msvc()
            && self.target_features.split(',').any(|f| f == "crt-static")
    }

    fn cmake_profile(&self) -> &str {
        if self.is_debug() { "Debug" } else { "Release" }
    }

    fn assimp_source_dir(&self) -> PathBuf {
        env::var("ASSIMP_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.manifest_dir.join("assimp"))
    }

    fn assimp_include_dir(&self) -> PathBuf {
        self.assimp_source_dir().join("include")
    }
}

fn main() {
    let config = BuildConfig::new();

    // Re-run when cache/download related env vars change
    println!("cargo:rerun-if-env-changed=ASSET_IMPORTER_PACKAGE_DIR");
    println!("cargo:rerun-if-env-changed=ASSET_IMPORTER_CACHE_DIR");
    println!("cargo:rerun-if-env-changed=ASSET_IMPORTER_OFFLINE");
    println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");
    println!("cargo:rerun-if-env-changed=CARGO_NET_OFFLINE");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=assimp");

    // Check if assimp submodule exists
    validate_assimp_source(&config);

    // Determine build strategy and generate bindings
    let built_include_dir = determine_build_strategy(&config);

    generate_bindings(&config, built_include_dir.as_deref());

    // Build the C++ bridge (progress + IOSystem wrappers)
    compile_bridge_cpp(&config, built_include_dir.as_deref());
}

fn validate_assimp_source(config: &BuildConfig) {
    let assimp_dir = config.assimp_source_dir();
    if !assimp_dir.exists() || !assimp_dir.join("include").exists() {
        panic!(
            "Assimp submodule not found at {}. Please run:\n\
             git submodule update --init --recursive",
            assimp_dir.display()
        );
    }
}

fn determine_build_strategy(config: &BuildConfig) -> Option<PathBuf> {
    if cfg!(feature = "system") {
        if cfg!(feature = "static-link") {
            println!(
                "cargo:warning=feature 'static-link' is ignored with 'system' linking; using dynamic system lib"
            );
        }
        // Explicitly use system assimp
        link_system_assimp(config);
        None
    } else if cfg!(feature = "build-assimp") || cfg!(feature = "static-link") || config.force_build
    {
        // Explicitly build from source
        build_assimp_from_source(config);
        // Use the built include directory which has config.h
        Some(config.out_dir.join("include"))
    } else if cfg!(feature = "prebuilt") {
        // Explicitly use prebuilt binaries, and use their include directory for wrapper/bindgen
        let include = link_prebuilt_assimp(config);
        Some(include)
    } else {
        // Default: build from source for better compatibility
        build_assimp_from_source(config);
        Some(config.out_dir.join("include"))
    }
}

fn link_system_assimp(config: &BuildConfig) {
    // Try to find and link system assimp
    if !try_system_assimp(config) {
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
    // Ensure platform runtime/system libs are linked consistently
    link_system_dependencies(config);
}

fn try_system_assimp(config: &BuildConfig) -> bool {
    match config.target_os.as_str() {
        "windows" => {
            // Try vcpkg
            if env::var("VCPKG_ROOT").is_ok() {
                println!("cargo:rustc-link-lib=assimp");
                return true;
            }
        }
        "macos" => {
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
        "linux" => {
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
        _ => {}
    }

    false
}

fn link_prebuilt_assimp(config: &BuildConfig) -> PathBuf {
    let target = env::var("TARGET").unwrap();
    let crate_version = env::var("CARGO_PKG_VERSION").unwrap();

    // Determine link type
    let link_type = if cfg!(feature = "static-link") {
        "static"
    } else {
        "dylib"
    };

    // Choose CRT suffix on Windows MSVC to disambiguate MD/MT
    let crt_suffix = if config.is_windows() && config.is_msvc() {
        if config.use_static_crt() {
            Some("mt")
        } else {
            Some("md")
        }
    } else {
        None
    };

    // Try new name (with CRT suffix) first, then fallback to old name (without)
    let mut archive_names: Vec<String> = Vec::new();
    if let Some(crt) = crt_suffix {
        archive_names.push(format!(
            "asset-importer-{}-{}-{}-{}.tar.gz",
            crate_version, target, link_type, crt
        ));
    }
    archive_names.push(format!(
        "asset-importer-{}-{}-{}.tar.gz",
        crate_version, target, link_type
    ));

    // Resolve stable cache root to avoid repeated downloads across builds
    let cache_root = prebuilt_cache_root(config);
    let ar_src_dir = if let Ok(package_dir) = env::var("ASSET_IMPORTER_PACKAGE_DIR") {
        PathBuf::from(package_dir)
    } else {
        // Download archive(s) into cache if not present
        download_prebuilt_packages(&cache_root, &archive_names);
        cache_root.clone()
    };

    // Determine and ensure extraction directory inside cache
    let extract_dir = prebuilt_extract_dir(config, link_type, crt_suffix);
    extract_prebuilt_package(config, &ar_src_dir, &archive_names, &extract_dir, link_type);

    // Return the include directory inside the cached extraction
    extract_dir.join("include")
}

fn download_prebuilt_packages(out_dir: &std::path::Path, archive_names: &[String]) {
    let crate_version = env::var("CARGO_PKG_VERSION").unwrap();

    // Try different tag formats for downloading prebuilt packages
    // 1. First try sys-specific tag: asset-importer-sys-v{version}
    // 2. Fallback to unified tag: v{version}
    let tag_formats = [
        format!("asset-importer-sys-v{}", crate_version),
        format!("v{}", crate_version),
    ];

    let mut last_error = String::new();

    // If any of the candidate archives already exists in cache, skip downloading
    for name in archive_names {
        if out_dir.join(name).exists() {
            return;
        }
    }

    for tag in &tag_formats {
        for archive_name in archive_names {
            let download_url = format!(
                "https://github.com/Latias94/asset-importer/releases/download/{}/{}",
                tag, archive_name
            );

            if env::var("ASSET_IMPORTER_VERBOSE").is_ok() {
                println!("cargo:warning=Trying download URL: {}", download_url);
            }

            // Try to download with this tag format
            if try_download(&download_url, out_dir, archive_name).is_ok() {
                return;
            } else {
                last_error = format!("Failed to download {} from tag: {}", archive_name, tag);
            }
        }
    }

    // If all attempts failed, panic with helpful message
    panic!(
        "Failed to download prebuilt package from any tag format. \
         Tried tags: {:?} and archives: {:?}. Last error: {}. \
         Consider using 'build-assimp' feature to build from source instead.",
        tag_formats, archive_names, last_error
    );
}

fn try_download(
    download_url: &str,
    out_dir: &std::path::Path,
    archive_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let archive_path = out_dir.join(archive_name);

    // Skip download if file already exists
    if archive_path.exists() {
        if env::var("ASSET_IMPORTER_VERBOSE").is_ok() {
            println!(
                "cargo:warning=Using existing prebuilt package: {}",
                archive_path.display()
            );
        }
        return Ok(());
    }

    // Respect offline mode if requested
    let offline = env::var("ASSET_IMPORTER_OFFLINE").is_ok()
        || matches!(
            env::var("CARGO_NET_OFFLINE").ok().as_deref(),
            Some("true") | Some("1") | Some("yes")
        );
    if offline {
        return Err(format!(
            "Offline mode enabled and archive not found in cache: {}",
            archive_path.display()
        )
        .into());
    }

    // This is important info for users to know download is happening
    println!(
        "cargo:warning=Downloading prebuilt package from: {}",
        download_url
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(download_url)
        .send()
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "HTTP error {}: Failed to download from {}",
            response.status(),
            download_url
        )
        .into());
    }

    let bytes = response
        .bytes()
        .map_err(|e| format!("Failed to read response bytes: {}", e))?;

    std::fs::create_dir_all(out_dir).ok();
    fs::write(&archive_path, &bytes)
        .map_err(|e| format!("Failed to write downloaded package: {}", e))?;

    if env::var("ASSET_IMPORTER_VERBOSE").is_ok() {
        println!(
            "cargo:warning=Downloaded prebuilt package: {}",
            archive_path.display()
        );
    }

    Ok(())
}

fn extract_prebuilt_package(
    config: &BuildConfig,
    ar_src_dir: &std::path::Path,
    archive_names: &[String],
    extract_dir: &std::path::Path,
    link_type: &str,
) {
    // Pick the first archive that exists
    let mut archive_path: Option<PathBuf> = None;
    for name in archive_names {
        let candidate = ar_src_dir.join(name);
        if candidate.exists() {
            archive_path = Some(candidate);
            break;
        }
    }
    let archive_path = archive_path.unwrap_or_else(|| {
        panic!(
            "Prebuilt package not found in {} with any of {:?}. \
             Consider using 'build-assimp' feature to build from source instead.",
            ar_src_dir.display(),
            archive_names
        )
    });

    // If we already have a valid extraction (include + lib exist), skip extracting
    let include_ok = extract_dir.join("include").exists();
    let lib_ok = extract_dir.join("lib").exists() || extract_dir.join("lib64").exists();
    if !(include_ok && lib_ok) {
        let file = fs::File::open(&archive_path).expect("Failed to open prebuilt package");
        let mut archive = tar::Archive::new(GzDecoder::new(file));
        if extract_dir.exists() {
            let _ = std::fs::remove_dir_all(extract_dir);
        }
        std::fs::create_dir_all(extract_dir).expect("Failed to create cache extract directory");
        archive
            .unpack(extract_dir)
            .expect("Failed to extract prebuilt package");
    }

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
    let mut lib_name = String::from("assimp");
    if config.is_windows() {
        lib_name = detect_windows_lib_name(&extract_dir.join("lib")).unwrap_or(lib_name);
    }

    // Warn if runtime flavor likely mismatches Rust target feature on MSVC
    validate_crt_compatibility(config, &lib_name);

    if link_type == "static" {
        println!("cargo:rustc-link-lib=static={}", lib_name);
    } else {
        println!("cargo:rustc-link-lib={}", lib_name);
    }

    // For prebuilt static libraries on Windows/MSVC, link bundled zlib if present
    if link_type == "static" && config.is_windows() && config.is_msvc() && !cfg!(feature = "nozlib")
    {
        link_prebuilt_zlib(&extract_dir.join("lib"));
    }

    // Link system dependencies
    link_system_dependencies(config);

    if env::var("ASSET_IMPORTER_VERBOSE").is_ok() {
        println!(
            "cargo:warning=Using prebuilt assimp from: {}",
            extract_dir.display()
        );
    }
}

fn detect_windows_lib_name(lib_dir: &std::path::Path) -> Option<String> {
    if let Ok(read) = fs::read_dir(lib_dir) {
        for entry in read.flatten() {
            let p = entry.path();
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                let lower = name.to_ascii_lowercase();
                if lower.starts_with("assimp") && lower.ends_with(".lib") {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        return Some(stem.to_string());
                    }
                }
            }
        }
    }
    None
}

fn validate_crt_compatibility(config: &BuildConfig, lib_name: &str) {
    if config.is_windows() && config.is_msvc() {
        let lname = lib_name.to_ascii_lowercase();
        if !config.use_static_crt() && lname.contains("-mt") {
            println!(
                "cargo:warning=Prebuilt assimp appears to use static MSVC runtime (-mt) while your Rust target uses dynamic CRT. Consider enabling crt-static or using a MD-built prebuilt."
            );
        }
        if config.use_static_crt() && lname.contains("-md") {
            println!(
                "cargo:warning=Prebuilt assimp appears to use dynamic MSVC runtime (-md) while your Rust target uses static CRT. Consider disabling crt-static or using a MT-built prebuilt."
            );
        }
    }
}

fn link_prebuilt_zlib(lib_dir: &std::path::Path) {
    if let Ok(read) = fs::read_dir(lib_dir) {
        for entry in read.flatten() {
            let p = entry.path();
            if let (Some(name), Some(ext)) = (
                p.file_name().and_then(|s| s.to_str()),
                p.extension().and_then(|s| s.to_str()),
            ) {
                if ext.eq_ignore_ascii_case("lib") && name.to_ascii_lowercase().contains("zlib") {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        println!("cargo:rustc-link-lib=static={}", stem);
                        break;
                    }
                }
            }
        }
    } else {
        println!(
            "cargo:warning=Expected zlib static library with prebuilt static assimp but none found in {}",
            lib_dir.display()
        );
    }
}

fn link_system_dependencies(config: &BuildConfig) {
    match config.target_os.as_str() {
        "windows" => {
            if config.is_msvc() {
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

fn build_assimp_from_source(config: &BuildConfig) {
    let assimp_src = config.assimp_source_dir();

    // Check if assimp source exists
    if !assimp_src.exists() {
        panic!(
            "Assimp source not found. Set ASSIMP_DIR to a valid Assimp source, or init submodule at {}.\nHint:\n  git submodule update --init --recursive\n  or set environment variable ASSIMP_DIR=/path/to/assimp",
            config.manifest_dir.join("assimp").display()
        );
    }

    let mut cmake_config = cmake::Config::new(&assimp_src);
    configure_cmake_basic_options(&mut cmake_config);
    configure_cmake_zlib(&mut cmake_config, config);
    configure_cmake_export(&mut cmake_config);
    configure_cmake_profile(&mut cmake_config, config);
    configure_cmake_crt(&mut cmake_config, config);
    configure_cmake_platform_specific(&mut cmake_config, config);

    let dst = cmake_config.build();
    setup_library_linking(&dst, config);
}

fn configure_cmake_basic_options(cmake_config: &mut cmake::Config) {
    // Shared vs static library build
    let build_shared = if cfg!(feature = "static-link") {
        "OFF"
    } else {
        "ON"
    };

    cmake_config
        .define("ASSIMP_BUILD_TESTS", "OFF")
        .define("ASSIMP_BUILD_SAMPLES", "OFF")
        .define("ASSIMP_BUILD_ASSIMP_TOOLS", "OFF")
        .define("BUILD_SHARED_LIBS", build_shared)
        // Disable being overly strict with warnings, which can cause build issues
        .define("ASSIMP_WARNINGS_AS_ERRORS", "OFF");
}

fn configure_cmake_zlib(cmake_config: &mut cmake::Config, config: &BuildConfig) {
    // Configure zlib based on nozlib feature, platform, and shared/static build
    if cfg!(feature = "nozlib") {
        cmake_config.define("ASSIMP_BUILD_ZLIB", "OFF");
    } else if config.is_windows() {
        // Use bundled zlib on Windows (assimp default)
        cmake_config.define("ASSIMP_BUILD_ZLIB", "ON");
    } else {
        // Unix (Linux/macOS):
        // - macOS: always use system zlib (shared), system libz.dylib is universally available and avoids
        //   issues in contrib zlib headers conflicting with SDK headers.
        // - Linux: for shared builds, bundle zlib to avoid picking up non-PIC libz.a; for static builds,
        //   prefer system zlib if available.
        if config.target_os == "macos" {
            cmake_config.define("ASSIMP_BUILD_ZLIB", "OFF");
        } else {
            let building_shared = !cfg!(feature = "static-link");
            if building_shared {
                cmake_config.define("ASSIMP_BUILD_ZLIB", "ON");
            } else {
                let use_system = has_system_zlib_any();
                cmake_config.define("ASSIMP_BUILD_ZLIB", if use_system { "OFF" } else { "ON" });
            }
        }
    }
}

fn configure_cmake_export(cmake_config: &mut cmake::Config) {
    // Enable export functionality if requested
    #[cfg(feature = "export")]
    cmake_config.define("ASSIMP_BUILD_NO_EXPORT", "OFF");

    #[cfg(not(feature = "export"))]
    cmake_config.define("ASSIMP_BUILD_NO_EXPORT", "ON");
}

fn configure_cmake_profile(cmake_config: &mut cmake::Config, config: &BuildConfig) {
    // Use a CMake profile that matches Cargo's profile to avoid MD/MDd mismatches
    cmake_config.profile(config.cmake_profile());
}

fn configure_cmake_crt(cmake_config: &mut cmake::Config, config: &BuildConfig) {
    // Decide MSVC runtime based on Rust target feature `crt-static`.
    // Important: Rust on MSVC always links the release CRT, even in debug profile.
    // To avoid LNK4098/LNK2019 mismatches, we build Assimp and our bridge with the
    // non-debug CRT as well.
    if config.is_windows() && config.is_msvc() {
        // Ensure cmake crate sets the expected CRT flavor for MSVC projects
        cmake_config.static_crt(config.use_static_crt());

        // Explicitly tell Assimp's CMake logic which CRT family to use.
        // - When Rust target has `+crt-static`, use MultiThreaded (MT) for all configs
        // - Otherwise use MultiThreadedDLL (MD) for all configs
        let msvc_rt = if config.use_static_crt() {
            "MultiThreaded"
        } else {
            "MultiThreadedDLL"
        };
        cmake_config.define("CMAKE_MSVC_RUNTIME_LIBRARY", msvc_rt);

        // Some Assimp trees support opting into static CRT via this toggle; set it
        // to match our target selection to avoid project-local overrides to /MTd.
        cmake_config.define(
            "USE_STATIC_CRT",
            if config.use_static_crt() { "ON" } else { "OFF" },
        );
    }
}

fn prebuilt_cache_root(config: &BuildConfig) -> PathBuf {
    if let Ok(dir) = env::var("ASSET_IMPORTER_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Prefer workspace root/target if likely in a workspace, else crate-local target
            config
                .manifest_dir
                .parent()
                .map(|p| p.join("target"))
                .unwrap_or_else(|| config.manifest_dir.join("target"))
        });
    target_dir.join("asset-importer-prebuilt")
}

fn prebuilt_extract_dir(
    config: &BuildConfig,
    link_type: &str,
    crt_suffix: Option<&str>,
) -> PathBuf {
    let cache_root = prebuilt_cache_root(config);
    let crate_version = env::var("CARGO_PKG_VERSION").unwrap();
    let target = env::var("TARGET").unwrap();
    let subdir = if let Some(crt) = crt_suffix {
        format!("{}-{}", link_type, crt)
    } else {
        link_type.to_string()
    };
    cache_root.join(crate_version).join(target).join(subdir)
}

fn configure_cmake_platform_specific(cmake_config: &mut cmake::Config, config: &BuildConfig) {
    // Platform-specific configurations
    // Set C++ standard to C++17 (required by Assimp)
    cmake_config.define("CMAKE_CXX_STANDARD", "17");
    cmake_config.define("CMAKE_CXX_STANDARD_REQUIRED", "ON");

    match config.target_os.as_str() {
        "windows" => {
            if config.is_msvc() {
                // Match MSVC runtime with `crt-static` and Debug/Release configuration
                // Use generator expressions to select *Debug variants in Debug builds.
                let msvc_rt = if config.use_static_crt() {
                    "MultiThreaded$<$<CONFIG:Debug>:Debug>"
                } else {
                    "MultiThreaded$<$<CONFIG:Debug>:Debug>DLL"
                };
                cmake_config.define("CMAKE_MSVC_RUNTIME_LIBRARY", msvc_rt);
                // Enable C++ exception handling for MSVC
                cmake_config.define("CMAKE_CXX_FLAGS", "/EHsc");
                cmake_config.define("CMAKE_CXX_FLAGS_DEBUG", "/EHsc");
                cmake_config.define("CMAKE_CXX_FLAGS_RELEASE", "/EHsc");
            }
        }
        "macos" => {
            cmake_config.define("CMAKE_OSX_DEPLOYMENT_TARGET", "10.12");
            // Ensure C++17 standard for macOS
            cmake_config.define("CMAKE_CXX_FLAGS", "-std=c++17");
        }
        _ => {
            // For other Unix-like systems, ensure C++17
            cmake_config.define("CMAKE_CXX_FLAGS", "-std=c++17");
        }
    }
}

fn setup_library_linking(dst: &std::path::Path, config: &BuildConfig) {
    // Link the built library
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-search=native={}/lib64", dst.display());

    // Also search in the build directory structure
    println!("cargo:rustc-link-search=native={}/build/lib", dst.display());
    println!(
        "cargo:rustc-link-search=native={}/build/lib64",
        dst.display()
    );

    // Add the out directory itself as a search path
    println!("cargo:rustc-link-search=native={}", dst.display());

    // Add the CMake profile subdirectory for MSVC builds
    if config.is_msvc() {
        let cmake_dir = config.cmake_profile();
        println!(
            "cargo:rustc-link-search=native={}/build/lib/{}",
            dst.display(),
            cmake_dir
        );
        println!(
            "cargo:rustc-link-search=native={}/build/contrib/zlib/{}",
            dst.display(),
            cmake_dir
        );
    }

    // Link to the built assimp library
    if config.is_msvc() {
        link_msvc_libraries(dst, config);
    } else {
        link_unix_libraries(dst, config);
    }

    // Handle Windows DLL copying for shared builds
    if config.is_windows() && config.is_msvc() && !cfg!(feature = "static-link") {
        copy_windows_dlls(dst);
    }

    // Link system dependencies
    link_system_dependencies(config);

    // For static linking on Windows MSVC, ensure we link the correct debug CRT libraries
    if config.is_windows() && config.is_msvc() && cfg!(feature = "static-link") && config.is_debug()
    {
        // Link debug CRT libraries explicitly for static linking
        println!("cargo:rustc-link-lib=msvcrtd");
        println!("cargo:rustc-link-lib=msvcprtd");
    }

    // Export include path for bindgen
    let include_path = dst.join("include");
    println!("cargo:include={}", include_path.display());
}

fn link_msvc_libraries(dst: &std::path::Path, config: &BuildConfig) {
    let cmake_dir = config.cmake_profile();
    let search_dirs = [
        dst.join("build").join("lib").join(cmake_dir),
        dst.join("build").join("lib"),
        dst.join("lib"),
        dst.join("lib64"),
    ];

    // Auto-detect the assimp .lib file produced by CMake
    let assimp_lib =
        find_library_in_dirs(&search_dirs, "assimp", "lib").unwrap_or_else(|| "assimp".to_string());

    if cfg!(feature = "static-link") {
        println!("cargo:rustc-link-lib=static={}", assimp_lib);
    } else {
        println!("cargo:rustc-link-lib={}", assimp_lib);
    }

    // If building from source and zlib is enabled, link zlib explicitly.
    if !cfg!(feature = "nozlib")
        && (cfg!(feature = "build-assimp") || env::var("ASSET_IMPORTER_FORCE_BUILD").is_ok())
    {
        let zlib_lib = find_library_in_dirs(&search_dirs, "zlib", "lib")
            .unwrap_or_else(|| "zlibstatic".to_string());
        println!("cargo:rustc-link-lib=static={}", zlib_lib);
    }
}

fn link_unix_libraries(dst: &std::path::Path, config: &BuildConfig) {
    let search_dirs = [
        dst.join("lib"),
        dst.join("lib64"),
        dst.join("build").join("lib"),
        dst.join("build").join("lib64"),
    ];

    // Try to find debug version first if in debug mode
    let assimp_lib = if config.is_debug() {
        find_unix_debug_library(&search_dirs).or_else(|| find_unix_release_library(&search_dirs))
    } else {
        find_unix_release_library(&search_dirs)
    }
    .unwrap_or_else(|| "assimp".to_string());

    if cfg!(feature = "static-link") {
        println!("cargo:rustc-link-lib=static={}", assimp_lib);
    } else {
        println!("cargo:rustc-link-lib={}", assimp_lib);
    }

    // On non-Windows, when building from source and zlib is enabled, link against system zlib
    if !cfg!(feature = "nozlib")
        && (cfg!(feature = "build-assimp") || env::var("ASSET_IMPORTER_FORCE_BUILD").is_ok())
    {
        println!("cargo:rustc-link-lib=z");
    }
}

fn find_library_in_dirs(
    search_dirs: &[PathBuf],
    lib_prefix: &str,
    extension: &str,
) -> Option<String> {
    for dir in search_dirs {
        if let Ok(read) = fs::read_dir(dir) {
            for entry in read.flatten() {
                let p = entry.path();
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    let lower = name.to_ascii_lowercase();
                    if lower.starts_with(lib_prefix) && lower.ends_with(&format!(".{}", extension))
                    {
                        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                            return Some(stem.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn find_unix_debug_library(search_dirs: &[PathBuf]) -> Option<String> {
    for dir in search_dirs {
        if let Ok(read) = fs::read_dir(dir) {
            for entry in read.flatten() {
                let p = entry.path();
                if let (Some(name), Some(ext)) = (
                    p.file_name().and_then(|s| s.to_str()),
                    p.extension().and_then(|s| s.to_str()),
                ) {
                    let lower_name = name.to_ascii_lowercase();
                    if (ext.eq_ignore_ascii_case("a")
                        || ext.eq_ignore_ascii_case("so")
                        || ext.eq_ignore_ascii_case("dylib"))
                        && lower_name.contains("assimp")
                        && (lower_name.contains("assimpd")
                            || lower_name.ends_with("d.a")
                            || lower_name.ends_with("d.so")
                            || lower_name.ends_with("d.dylib"))
                    {
                        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                            // Remove lib prefix for Unix libraries
                            let lib_name = stem.strip_prefix("lib").unwrap_or(stem);
                            return Some(lib_name.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn find_unix_release_library(search_dirs: &[PathBuf]) -> Option<String> {
    for dir in search_dirs {
        if let Ok(read) = fs::read_dir(dir) {
            for entry in read.flatten() {
                let p = entry.path();
                if let (Some(name), Some(ext)) = (
                    p.file_name().and_then(|s| s.to_str()),
                    p.extension().and_then(|s| s.to_str()),
                ) {
                    let lower_name = name.to_ascii_lowercase();
                    if (ext.eq_ignore_ascii_case("a")
                        || ext.eq_ignore_ascii_case("so")
                        || ext.eq_ignore_ascii_case("dylib"))
                        && lower_name.contains("assimp")
                        && !lower_name.contains("assimpd")
                        && !lower_name.ends_with("d.a")
                        && !lower_name.ends_with("d.so")
                        && !lower_name.ends_with("d.dylib")
                    {
                        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                            // Remove lib prefix for Unix libraries
                            let lib_name = stem.strip_prefix("lib").unwrap_or(stem);
                            return Some(lib_name.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn has_system_zlib_any() -> bool {
    // Quick checks for pkg-config and common headers/libs
    // Return true if zlib seems available on the system
    use std::process::Command;
    // Try pkg-config first
    if let Ok(status) = Command::new("pkg-config")
        .arg("--exists")
        .arg("zlib")
        .status()
    {
        if status.success() {
            return true;
        }
    }
    // Fallback: check for common library/header locations
    let header_paths = [
        "/usr/include/zlib.h",
        "/usr/local/include/zlib.h",
        "/opt/homebrew/include/zlib.h",
    ];
    if header_paths
        .iter()
        .any(|p| std::path::Path::new(p).exists())
    {
        return true;
    }
    let lib_paths = [
        "/usr/lib/libz.so",
        "/usr/local/lib/libz.so",
        "/usr/lib/x86_64-linux-gnu/libz.so",
        "/usr/lib/aarch64-linux-gnu/libz.so",
        "/opt/homebrew/lib/libz.dylib",
        "/usr/lib/libz.a",
        "/usr/local/lib/libz.a",
        "/usr/lib/x86_64-linux-gnu/libz.a",
        "/usr/lib/aarch64-linux-gnu/libz.a",
    ];
    lib_paths.iter().any(|p| std::path::Path::new(p).exists())
}

fn copy_windows_dlls(dst: &std::path::Path) {
    use std::fs;
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());
    // OUT_DIR: target\<profile>\build\<crate-hash>\out
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| out_dir.clone());
    let deps_dir = profile_dir.join("deps");

    let candidates = [dst.join("bin"), dst.join("build").join("bin")];
    for bin_dir in candidates.iter() {
        if !bin_dir.exists() {
            continue;
        }
        if let Ok(read) = fs::read_dir(bin_dir) {
            for entry in read.flatten() {
                let p = entry.path();
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    if ext.eq_ignore_ascii_case("dll") {
                        let fname = p.file_name().unwrap();
                        let _ = fs::create_dir_all(&deps_dir);
                        let _ = fs::copy(&p, deps_dir.join(fname));
                        let _ = fs::copy(&p, profile_dir.join(fname));
                        // Only show copy info if verbose build is requested
                        if env::var("ASSET_IMPORTER_VERBOSE").is_ok() {
                            println!(
                                "cargo:warning=Copied {} to {} and {}",
                                p.display(),
                                deps_dir.display(),
                                profile_dir.display()
                            );
                        }
                    }
                }
            }
        }
    }
}

fn generate_bindings(config: &BuildConfig, built_include_dir: Option<&std::path::Path>) {
    let wrapper_h = config.manifest_dir.join("wrapper.h");
    let assimp_include = config.assimp_include_dir();

    // Create empty config.h if it doesn't exist (needed for system builds) only when using submodule path
    let submodule_include = config.manifest_dir.join("assimp").join("include");
    let use_submodule = assimp_include == submodule_include;
    let config_file = assimp_include.join("assimp").join("config.h");
    if use_submodule && !config_file.exists() {
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
        // Use Rust enums by default (non-exhaustive off for stable pattern matching)
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        // Prefer rustified enums for specific API enums we read/write directly
        .rustified_enum("aiTextureMapping")
        .rustified_enum("aiTextureOp")
        .rustified_enum("aiTextureMapMode")
        .rustified_enum("aiLightSourceType")
        .rustified_enum("aiReturn")
        .rustified_enum("aiOrigin")
        .rustified_enum("aiMetadataType")
        .rustified_enum("aiShadingMode")
        .rustified_enum("aiBlendMode")
        .rustified_enum("aiTextureType")
        .rustified_enum("aiMorphingMethod")
        .rustified_enum("aiAnimInterpolation")
        .rustified_enum("aiAnimBehaviour")
        .rustified_enum("aiPropertyTypeInfo")
        // Constify only known bitmask/flags enums
        .constified_enum_module("aiPostProcessSteps")
        .constified_enum_module("aiPrimitiveType")
        .constified_enum_module("aiTextureFlags")
        .constified_enum_module("aiImporterFlags")
        .constified_enum_module("aiDefaultLogStream");

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

    let out_file = config.out_dir.join("bindings.rs");
    bindings
        .write_to_file(&out_file)
        .expect("Couldn't write bindings!");

    // Keep a temporary config.h for wrapper.cpp compilation in prebuilt/system modes
}

fn compile_bridge_cpp(config: &BuildConfig, built_include_dir: Option<&std::path::Path>) {
    let mut build = cc::Build::new();
    build.cpp(true);

    // Set C++17 standard (required by Assimp)
    build.std("c++17");

    build.file(config.manifest_dir.join("wrapper.cpp"));

    // Include paths for Assimp headers: prefer built include (has config.h), then submodule include
    if let Some(dir) = built_include_dir {
        build.include(dir);
    }
    build.include(config.assimp_include_dir());

    // Platform-specific compiler flags
    configure_cpp_build_flags(&mut build, config);

    build.compile("assimp_rust_bridge");
}

fn configure_cpp_build_flags(build: &mut cc::Build, config: &BuildConfig) {
    match config.target_os.as_str() {
        "windows" => {
            if config.is_msvc() {
                build.flag("/EHsc");

                // Always match Rust's CRT family, but never use the MSVC debug CRT
                // (Rust doesn't link it, which caused the unresolved __imp__CrtDbgReport).
                // Use /MD (dynamic) or /MT (static) for both Debug and Release.
                let use_static = config.use_static_crt();
                build.static_crt(use_static);
                if use_static {
                    build.flag("/MT");
                } else {
                    build.flag("/MD");
                }

                // Preserve debug info/optimizations without toggling CRT flavor.
                if config.is_debug() {
                    build.debug(true);
                    build.opt_level(0);
                } else {
                    build.debug(false);
                    build.opt_level(2);
                }

                // Use the non-debug iterator level to be consistent with non-debug CRT.
                build.flag("/D_ITERATOR_DEBUG_LEVEL=0");
                // Avoid defining _DEBUG which can drag debug-CRT-only references.
            }
        }
        "macos" => {
            // Ensure compatibility with macOS deployment target
            build.flag("-mmacosx-version-min=10.12");
        }
        _ => {
            // For other Unix-like systems
        }
    }
}
