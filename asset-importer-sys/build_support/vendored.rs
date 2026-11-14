use std::{fs, path::PathBuf};

use crate::build_support::{
    config::BuildConfig,
    plan::{BuildMethod, BuildPlan, LinkKind},
    util, version,
};

pub fn build(cfg: &BuildConfig, link_kind: LinkKind) -> BuildPlan {
    let assimp_src = cfg.assimp_source_dir();
    validate_assimp_source(&assimp_src);
    reset_stale_cmake_output(cfg, &assimp_src, link_kind);

    let dst = build_assimp_with_cmake(cfg, &assimp_src, link_kind);
    let revision_header = dst.join("build/include/assimp/revision.h");
    let actual_version = fs::read_to_string(&revision_header)
        .ok()
        .and_then(|contents| version::parse_header(&contents));
    assert_eq!(
        actual_version.as_deref(),
        Some(version::BUNDLED_VERSION),
        "Assimp source version must match the bundled bindings. Check ASSIMP_DIR and initialize the pinned submodule. Header: {}",
        revision_header.display()
    );
    write_build_stamp(cfg, &assimp_src, link_kind);

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

    let lib_name = detect_assimp_lib(&dst, cfg, link_kind).unwrap_or_else(|| "assimp".to_string());

    // Unix static builds require explicit linkage to the system zlib.
    if !cfg.is_windows() && matches!(link_kind, LinkKind::Static) {
        println!("cargo:rustc-link-lib=z");
    }

    // For Windows static builds, link bundled zlib if present.
    if cfg.is_windows() && matches!(link_kind, LinkKind::Static) {
        link_windows_zlib(&dst, cfg);
    }

    // For Windows shared builds, copy DLLs next to test binaries.
    if cfg.is_windows() && matches!(link_kind, LinkKind::Dynamic) {
        util::copy_windows_dlls(&cfg.out_dir, &dst);
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

fn build_stamp_contents(
    cfg: &BuildConfig,
    assimp_src: &std::path::Path,
    link_kind: LinkKind,
) -> String {
    format!(
        "assimp_version={}\nlink_kind={:?}\ncmake_profile={}\nsource={}\nminizip=bundled\n",
        version::BUNDLED_VERSION,
        link_kind,
        cfg.cmake_profile(),
        assimp_src.display()
    )
}

fn build_stamp_path(cfg: &BuildConfig) -> PathBuf {
    cfg.out_dir.join("asset-importer-assimp-build-stamp.txt")
}

fn reset_stale_cmake_output(cfg: &BuildConfig, assimp_src: &std::path::Path, link_kind: LinkKind) {
    let stamp_path = build_stamp_path(cfg);
    let expected = build_stamp_contents(cfg, assimp_src, link_kind);
    if fs::read_to_string(&stamp_path).ok().as_deref() == Some(expected.as_str()) {
        return;
    }

    if cfg.verbose {
        util::warn(format!(
            "Assimp build cache stamp changed; rebuilding vendored Assimp {}",
            version::BUNDLED_VERSION
        ));
    }

    for name in ["build", "include", "lib", "lib64", "bin", "share"] {
        remove_out_dir_child(cfg, name);
    }
    let _ = fs::remove_file(stamp_path);
}

fn remove_out_dir_child(cfg: &BuildConfig, name: &str) {
    let target = cfg.out_dir.join(name);
    if !target.exists() {
        return;
    }

    let Ok(out_dir) = cfg.out_dir.canonicalize() else {
        return;
    };
    let Ok(target_canonical) = target.canonicalize() else {
        return;
    };
    if !target_canonical.starts_with(&out_dir) {
        util::warn(format!(
            "Refusing to remove stale Assimp build path outside OUT_DIR: {}",
            target_canonical.display()
        ));
        return;
    }

    if target_canonical.is_dir() {
        let _ = fs::remove_dir_all(&target_canonical);
    } else {
        let _ = fs::remove_file(&target_canonical);
    }
}

fn write_build_stamp(cfg: &BuildConfig, assimp_src: &std::path::Path, link_kind: LinkKind) {
    let stamp_path = build_stamp_path(cfg);
    let _ = fs::write(stamp_path, build_stamp_contents(cfg, assimp_src, link_kind));
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
        // Assimp 6.0.6 otherwise probes system minizip when using system zlib.
        .define("ASSIMP_BUILD_MINIZIP", "ON")
        .define("UNZIP_FOUND", "OFF")
        .define("BUILD_SHARED_LIBS", if build_shared { "ON" } else { "OFF" });

    // Keep Assimp's C "export" API enabled even when the Rust `export` feature is off.
    //
    // Assimp gates `aiCopyScene` / `aiFreeScene` behind `ASSIMP_BUILD_NO_EXPORT`.
    // Keep these available for raw scene adoption and a consistent low-level C API.
    cmake_config.define("ASSIMP_NO_EXPORT", "OFF");

    // zlib strategy:
    // - Windows: build bundled zlib (default for Assimp)
    // - Unix: use system zlib (faster and more predictable); link `-lz` for static builds.
    if cfg.is_windows() {
        cmake_config.define("ASSIMP_BUILD_ZLIB", "ON");
    } else {
        cmake_config.define("ASSIMP_BUILD_ZLIB", "OFF");
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

fn detect_assimp_lib(
    dst: &std::path::Path,
    cfg: &BuildConfig,
    link_kind: LinkKind,
) -> Option<String> {
    let search_dirs = [
        dst.join("lib"),
        dst.join("lib64"),
        dst.join("build").join("lib").join(cfg.cmake_profile()),
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
            let Some(stem) = util::library_filename::assimp_link_name(
                name,
                &cfg.target,
                matches!(link_kind, LinkKind::Static),
            ) else {
                continue;
            };

            let prefer_debug = cfg.is_debug() && !cfg.is_msvc();
            if stem.starts_with("assimpd") == prefer_debug {
                return Some(stem.to_string());
            }
            best.get_or_insert_with(|| stem.to_string());
        }
    }

    best
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
            if let Some(stem) = util::library_filename::windows_zlib_link_name(name, &cfg.target) {
                println!("cargo:rustc-link-search=native={}", dir.display());
                println!("cargo:rustc-link-lib=static={}", stem);
                return;
            }
        }
    }
}
