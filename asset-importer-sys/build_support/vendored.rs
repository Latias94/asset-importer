use std::{env, fs, path::PathBuf};

use crate::build_support::{
    config::BuildConfig,
    plan::{BuildMethod, BuildPlan, LinkKind},
    util,
};

pub fn build(cfg: &BuildConfig, link_kind: LinkKind) -> BuildPlan {
    let assimp_src = cfg.assimp_source_dir();
    validate_assimp_source(&assimp_src);

    let dst = build_assimp_with_cmake(cfg, &assimp_src, link_kind);

    let include_dir = dst.join("include");
    let include_dirs = vec![include_dir, assimp_src.join("include")];

    let mut link_search = vec![
        dst.join("lib"),
        dst.join("lib64"),
        dst.join("build").join("lib"),
        dst.join("build").join("lib64"),
    ];
    if cfg.is_windows() {
        link_search.push(dst.join("bin"));
    }

    // On Windows, add profile subdirs for MSVC builds.
    if cfg.is_windows() && cfg.is_msvc() {
        let cmake_dir = cfg.cmake_profile();
        link_search.push(dst.join("build").join("lib").join(cmake_dir));
    }

    let lib_name = if cfg.is_windows() && cfg.is_msvc() {
        detect_windows_assimp_lib(&dst, cfg).unwrap_or_else(|| "assimp".to_string())
    } else if let Some(name) = detect_unix_assimp_lib(&dst, cfg) {
        name
    } else {
        "assimp".to_string()
    };

    // For unix static builds, link system zlib explicitly when enabled.
    if !cfg.is_windows() && matches!(link_kind, LinkKind::Static) && !cfg!(feature = "nozlib") {
        println!("cargo:rustc-link-lib=z");
    }

    // For Windows static builds, link bundled zlib if present.
    if cfg.is_windows()
        && cfg.is_msvc()
        && matches!(link_kind, LinkKind::Static)
        && !cfg!(feature = "nozlib")
    {
        link_windows_zlib(&dst, cfg);
    }

    // For Windows shared builds, copy DLLs next to test binaries.
    if cfg.is_windows() && cfg.is_msvc() && matches!(link_kind, LinkKind::Dynamic) {
        copy_windows_dlls(&dst);
    }

    BuildPlan {
        include_dirs,
        link_kind,
        link_lib: Some(lib_name),
        link_search: link_search.into_iter().filter(|p| p.exists()).collect(),
        method: BuildMethod::Vendored,
    }
}

fn validate_assimp_source(assimp_src: &std::path::Path) {
    if !assimp_src.exists() || !assimp_src.join("include").exists() {
        panic!(
            "Assimp source not found at {}.\n\
             Hint: git submodule update --init --recursive\n\
             or set ASSIMP_DIR=/path/to/assimp",
            assimp_src.display()
        );
    }
}

fn build_assimp_with_cmake(
    cfg: &BuildConfig,
    assimp_src: &std::path::Path,
    link_kind: LinkKind,
) -> PathBuf {
    let mut cmake_config = cmake::Config::new(assimp_src);
    cmake_config.profile(cfg.cmake_profile());

    let build_shared = matches!(link_kind, LinkKind::Dynamic);
    cmake_config
        .define("ASSIMP_BUILD_TESTS", "OFF")
        .define("ASSIMP_BUILD_SAMPLES", "OFF")
        .define("ASSIMP_BUILD_ASSIMP_TOOLS", "OFF")
        .define("ASSIMP_WARNINGS_AS_ERRORS", "OFF")
        .define("BUILD_SHARED_LIBS", if build_shared { "ON" } else { "OFF" });

    // Keep Assimp's C "export" API enabled even when the Rust `export` feature is off.
    //
    // Assimp gates `aiCopyScene` / `aiFreeScene` behind `ASSIMP_BUILD_NO_EXPORT`, and this crate
    // relies on them for safe deep-copy/ownership behavior (e.g. postprocess on shared scenes).
    cmake_config.define("ASSIMP_NO_EXPORT", "OFF");

    // zlib strategy:
    // - Windows: build bundled zlib (default for Assimp)
    // - Unix: use system zlib (faster and more predictable); link `-lz` for static builds.
    if cfg!(feature = "nozlib") {
        cmake_config.define("ASSIMP_BUILD_ZLIB", "OFF");
    } else if cfg.is_windows() {
        cmake_config.define("ASSIMP_BUILD_ZLIB", "ON");
    } else {
        cmake_config.define("ASSIMP_BUILD_ZLIB", "OFF");
        cmake_config.define("ASSIMP_BUILD_NO_OWN_ZLIB", "ON");
    }

    // Toolchain/platform knobs
    cmake_config.define("CMAKE_CXX_STANDARD", "17");
    cmake_config.define("CMAKE_CXX_STANDARD_REQUIRED", "ON");

    if cfg.is_windows() && cfg.is_msvc() {
        // Always match Rust's CRT family, but avoid MSVC debug CRT entirely.
        cmake_config.static_crt(cfg.use_static_crt());
        cmake_config.define("CMAKE_POLICY_DEFAULT_CMP0091", "NEW");
        cmake_config.define(
            "CMAKE_MSVC_RUNTIME_LIBRARY",
            if cfg.use_static_crt() {
                "MultiThreaded"
            } else {
                "MultiThreadedDLL"
            },
        );
        cmake_config.define(
            "USE_STATIC_CRT",
            if cfg.use_static_crt() { "ON" } else { "OFF" },
        );

        // Assimp's install rules for MSVC static-library PDBs can be brittle across generators/configs.
        // Disabling PDB installation avoids spurious CI failures while keeping the library build intact.
        // Set `ASSET_IMPORTER_ASSIMP_INSTALL_PDB=1` to re-enable if you need PDBs in the install tree.
        let install_pdb = std::env::var("ASSET_IMPORTER_ASSIMP_INSTALL_PDB")
            .ok()
            .is_some_and(|v| {
                v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on")
            });
        if !install_pdb {
            cmake_config.define("ASSIMP_INSTALL_PDB", "OFF");
        }
    } else if cfg.is_macos() {
        cmake_config.define("CMAKE_OSX_DEPLOYMENT_TARGET", cfg.macos_deployment_target());
    }

    if cfg.verbose {
        util::warn(format!(
            "Building Assimp from source: {}",
            assimp_src.display()
        ));
    }

    cmake_config.build()
}

fn detect_windows_assimp_lib(dst: &std::path::Path, cfg: &BuildConfig) -> Option<String> {
    let cmake_dir = cfg.cmake_profile();
    let candidates = [
        dst.join("build").join("lib").join(cmake_dir),
        dst.join("build").join("lib"),
        dst.join("lib"),
        dst.join("lib64"),
    ];
    for dir in candidates.iter() {
        let Ok(read) = fs::read_dir(dir) else {
            continue;
        };
        for entry in read.flatten() {
            let p = entry.path();
            let name = p.file_name().and_then(|s| s.to_str())?;
            if name.to_ascii_lowercase().starts_with("assimp")
                && name.to_ascii_lowercase().ends_with(".lib")
            {
                return p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string());
            }
        }
    }
    None
}

fn detect_unix_assimp_lib(dst: &std::path::Path, cfg: &BuildConfig) -> Option<String> {
    let search_dirs = [
        dst.join("lib"),
        dst.join("lib64"),
        dst.join("build").join("lib"),
        dst.join("build").join("lib64"),
        dst.join("build").join("bin"),
    ];

    let mut best: Option<String> = None;
    for dir in &search_dirs {
        let Ok(read) = fs::read_dir(dir) else {
            continue;
        };
        for entry in read.flatten() {
            let p = entry.path();
            let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let lower = name.to_ascii_lowercase();
            let is_shared = lower.ends_with(".dylib") || lower.contains(".so");
            let is_static = lower.ends_with(".a");
            if !(is_shared || is_static) || !lower.contains("assimp") {
                continue;
            }

            // Turn "libassimpd.6.0.4.dylib" into "assimpd".
            let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let stem = stem.strip_prefix("lib").unwrap_or(stem);

            // Prefer debug-suffixed library when building in debug mode.
            if cfg.is_debug() {
                if stem.contains("assimpd") {
                    return Some("assimpd".to_string());
                }
                best.get_or_insert_with(|| stem.to_string());
            } else {
                if stem == "assimp" || stem.starts_with("assimp.") {
                    return Some("assimp".to_string());
                }
                best.get_or_insert_with(|| stem.to_string());
            }
        }
    }

    best.map(|s| {
        // Strip accidental version suffix in "assimp.6" style stems.
        s.split('.').next().unwrap_or(&s).to_string()
    })
}

fn link_windows_zlib(dst: &std::path::Path, cfg: &BuildConfig) {
    let cmake_dir = cfg.cmake_profile();
    let candidates = [
        dst.join("build")
            .join("contrib")
            .join("zlib")
            .join(cmake_dir),
        dst.join("build").join("contrib").join("zlib"),
        dst.join("build").join("lib").join(cmake_dir),
        dst.join("build").join("lib"),
        dst.join("lib"),
    ];
    for dir in candidates.iter() {
        let Ok(read) = fs::read_dir(dir) else {
            continue;
        };
        for entry in read.flatten() {
            let p = entry.path();
            let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".lib") && (lower.contains("zlibstatic") || lower == "zlib.lib") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    println!("cargo:rustc-link-search=native={}", dir.display());
                    println!("cargo:rustc-link-lib=static={}", stem);
                    return;
                }
            }
        }
    }
}

fn copy_windows_dlls(dst: &std::path::Path) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());
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
                if p.extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("dll"))
                {
                    let fname = p.file_name().unwrap();
                    let _ = fs::create_dir_all(&deps_dir);
                    let _ = fs::copy(&p, deps_dir.join(fname));
                    let _ = fs::copy(&p, profile_dir.join(fname));
                }
            }
        }
    }
}
