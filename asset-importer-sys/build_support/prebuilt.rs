use std::{env, fs, path::PathBuf};

use crate::build_support::{
    config::BuildConfig,
    plan::{BuildMethod, BuildPlan, LinkKind},
    util,
};

use flate2_build::read::GzDecoder;
use tar_build::Archive;

pub fn prepare(cfg: &BuildConfig, link_kind: LinkKind) -> BuildPlan {
    let crate_version = env::var("CARGO_PKG_VERSION").unwrap();
    let target = cfg.target.clone();

    let link_type = match link_kind {
        LinkKind::Static => "static",
        LinkKind::Dynamic => "dylib",
    };

    let crt_suffix = if cfg.is_windows() && cfg.is_msvc() {
        Some(if cfg.use_static_crt() { "mt" } else { "md" })
    } else {
        None
    };

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

    let cache_root = cache_root(cfg);
    let package_root = env::var("ASSET_IMPORTER_PACKAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| cache_root.clone());

    // Ensure archive(s) exist: download into cache_root when not provided locally.
    if env::var("ASSET_IMPORTER_PACKAGE_DIR").is_err() {
        download_if_needed(cfg, &cache_root, &archive_names);
    }

    let extract_dir = extract_dir(cfg, link_type, crt_suffix);
    extract_archive(&package_root, &archive_names, &extract_dir);

    let include_dir = extract_dir.join("include");
    let lib_dir = if extract_dir.join("lib").exists() {
        extract_dir.join("lib")
    } else {
        extract_dir.join("lib64")
    };

    if !include_dir.exists() || !lib_dir.exists() {
        panic!(
            "prebuilt package extraction is missing include/lib: {}\n\
             Hint: rebuild and upload the prebuilt package for this target.",
            extract_dir.display()
        );
    }

    let lib_name = if cfg.is_windows() {
        detect_windows_import_lib(&lib_dir).unwrap_or_else(|| "assimp".to_string())
    } else {
        "assimp".to_string()
    };

    // Prebuilt static libs may require linking zlib explicitly.
    if matches!(link_kind, LinkKind::Static) && !cfg!(feature = "nozlib") {
        if cfg.is_windows() && cfg.is_msvc() {
            link_windows_zlib_if_present(&lib_dir);
        } else {
            println!("cargo:rustc-link-lib=z");
        }
    }

    // Make runtime shared libraries discoverable for tests/binaries.
    if matches!(link_kind, LinkKind::Dynamic) {
        ensure_runtime_libs(cfg, &extract_dir);
    }

    BuildPlan {
        include_dirs: vec![include_dir],
        link_kind,
        link_lib: Some(lib_name),
        link_search: vec![lib_dir, cfg.out_dir.clone()],
        method: BuildMethod::Prebuilt,
    }
}

fn cache_root(cfg: &BuildConfig) -> PathBuf {
    if let Ok(dir) = env::var("ASSET_IMPORTER_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            cfg.manifest_dir
                .parent()
                .map(|p| p.join("target"))
                .unwrap_or_else(|| cfg.manifest_dir.join("target"))
        });
    target_dir.join("asset-importer-prebuilt")
}

fn extract_dir(cfg: &BuildConfig, link_type: &str, crt_suffix: Option<&str>) -> PathBuf {
    let root = cache_root(cfg);
    let crate_version = env::var("CARGO_PKG_VERSION").unwrap();
    let subdir = if let Some(crt) = crt_suffix {
        format!("{}-{}", link_type, crt)
    } else {
        link_type.to_string()
    };
    root.join(crate_version).join(&cfg.target).join(subdir)
}

fn pick_archive_name(root: &std::path::Path, candidates: &[String]) -> PathBuf {
    for name in candidates {
        let p = root.join(name);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "prebuilt package not found in {} with any of {:?}",
        root.display(),
        candidates
    );
}

fn download_if_needed(cfg: &BuildConfig, cache_root: &std::path::Path, archive_names: &[String]) {
    fs::create_dir_all(cache_root).expect("Failed to create prebuilt cache directory");

    // Skip download if any candidate archive is already present.
    if archive_names.iter().any(|n| cache_root.join(n).exists()) {
        return;
    }

    if cfg.offline {
        panic!(
            "ASSET_IMPORTER_OFFLINE/CARGO_NET_OFFLINE is set but no prebuilt archive exists in {}\n\
             Hint: set ASSET_IMPORTER_PACKAGE_DIR to a directory containing the prebuilt .tar.gz, or disable offline mode.",
            cache_root.display()
        );
    }

    let crate_version = env::var("CARGO_PKG_VERSION").unwrap();
    let tag_formats = [
        format!("asset-importer-sys-v{}", crate_version),
        format!("v{}", crate_version),
    ];

    let client = reqwest::blocking::Client::new();
    let mut last_error = None;

    for tag in &tag_formats {
        for archive in archive_names {
            let url = format!(
                "https://github.com/Latias94/asset-importer/releases/download/{}/{}",
                tag, archive
            );

            if cfg.verbose {
                util::warn(format!("Downloading prebuilt package: {}", url));
            }

            match client.get(&url).send() {
                Ok(resp) if resp.status().is_success() => {
                    let bytes = resp.bytes().expect("Failed to read response body");
                    let dst = cache_root.join(archive);
                    fs::write(&dst, &bytes).expect("Failed to write downloaded prebuilt package");
                    return;
                }
                Ok(resp) => last_error = Some(format!("HTTP {} for {}", resp.status(), url)),
                Err(e) => last_error = Some(format!("{} for {}", e, url)),
            }
        }
    }

    panic!(
        "Failed to download prebuilt package for {:?}; last error: {:?}",
        archive_names, last_error
    );
}

fn extract_archive(root: &std::path::Path, candidates: &[String], dst: &std::path::Path) {
    let archive_path = pick_archive_name(root, candidates);
    let include_ok = dst.join("include").exists();
    let lib_ok = dst.join("lib").exists() || dst.join("lib64").exists();
    if include_ok && lib_ok {
        return;
    }

    if dst.exists() {
        let _ = fs::remove_dir_all(dst);
    }
    fs::create_dir_all(dst).expect("Failed to create extract directory");

    let file = fs::File::open(&archive_path).expect("Failed to open prebuilt archive");
    let mut archive = Archive::new(GzDecoder::new(file));
    archive
        .unpack(dst)
        .expect("Failed to extract prebuilt archive");
}

fn detect_windows_import_lib(lib_dir: &std::path::Path) -> Option<String> {
    let Ok(read) = fs::read_dir(lib_dir) else {
        return None;
    };
    for entry in read.flatten() {
        let p = entry.path();
        let name = p.file_name().and_then(|s| s.to_str())?;
        let lower = name.to_ascii_lowercase();
        if lower.starts_with("assimp") && lower.ends_with(".lib") {
            return p
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
        }
    }
    None
}

fn ensure_runtime_libs(cfg: &BuildConfig, extract_dir: &std::path::Path) {
    if cfg.is_windows() && cfg.is_msvc() {
        copy_windows_dlls(extract_dir);
        return;
    }

    // On Unix-like platforms, copy libassimp.* into OUT_DIR and add OUT_DIR as a link-search path.
    let candidates = [
        extract_dir.join("lib"),
        extract_dir.join("lib64"),
        extract_dir.join("bin"),
    ];
    for dir in &candidates {
        if !dir.exists() {
            continue;
        }
        let Ok(read) = fs::read_dir(dir) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let lower = name.to_ascii_lowercase();
            let is_shared = lower.ends_with(".dylib") || lower.contains(".so");
            if !lower.contains("assimp") || !is_shared {
                continue;
            }
            let _ = fs::copy(&path, cfg.out_dir.join(name));
        }
    }
}

fn link_windows_zlib_if_present(lib_dir: &std::path::Path) {
    let Ok(read) = fs::read_dir(lib_dir) else {
        return;
    };
    for entry in read.flatten() {
        let p = entry.path();
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if !lower.ends_with(".lib") || !lower.contains("zlib") {
            continue;
        }
        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            println!("cargo:rustc-link-lib=static={}", stem);
            return;
        }
    }
}

fn copy_windows_dlls(src_root: &std::path::Path) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| out_dir.clone());
    let deps_dir = profile_dir.join("deps");

    let candidates = [src_root.join("bin"), src_root.join("build").join("bin")];
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
