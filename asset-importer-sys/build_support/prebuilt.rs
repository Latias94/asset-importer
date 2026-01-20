use std::{env, fs, path::PathBuf};

use crate::build_support::{
    config::BuildConfig,
    plan::{BuildMethod, BuildPlan, LinkKind},
    util,
};

use flate2::read::GzDecoder;
use tar::Archive;

const PACKAGE_PREFIX: &str = "asset-importer";
const VENDORED_ASSIMP_VERSION: &str = "6.0.3";

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
            "{}-{}-{}-{}-{}.tar.gz",
            PACKAGE_PREFIX, crate_version, target, link_type, crt
        ));
    }
    archive_names.push(format!(
        "{}-{}-{}-{}.tar.gz",
        PACKAGE_PREFIX, crate_version, target, link_type
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

    validate_prebuilt_headers(&include_dir);

    let lib_name = if cfg.is_windows() {
        detect_windows_import_lib(&lib_dir).unwrap_or_else(|| "assimp".to_string())
    } else {
        detect_unix_link_name(&lib_dir, cfg.is_debug()).unwrap_or_else(|| "assimp".to_string())
    };

    validate_prebuilt_libs(cfg, &extract_dir, &lib_dir, link_kind, &lib_name);

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

fn validate_prebuilt_headers(include_dir: &std::path::Path) {
    // Prefer revision.h because it contains numeric version defines (VER_MAJOR/MINOR/PATCH)
    // in installed/package headers. Fall back to version.h for older/custom layouts.
    let header = {
        let revision = include_dir.join("assimp").join("revision.h");
        if revision.exists() {
            revision
        } else {
            include_dir.join("assimp").join("version.h")
        }
    };
    let contents = std::fs::read_to_string(&header).unwrap_or_else(|e| {
        panic!(
            "prebuilt package is missing {}: {}\n\
             Hint: rebuild and upload the prebuilt package.",
            header.display(),
            e
        )
    });

    let Some(version) = parse_assimp_version_from_header(&contents) else {
        util::warn(format!(
            "could not parse Assimp version from {}; skipping version check",
            header.display()
        ));
        return;
    };

    if version != VENDORED_ASSIMP_VERSION {
        panic!(
            "prebuilt Assimp headers are version {}, but this crate expects {}.\n\
             Hint: rebuild the prebuilt package for this crate version, or use `--features build-assimp` / `--features system`.",
            version, VENDORED_ASSIMP_VERSION
        );
    }
}

fn parse_assimp_version_from_header(contents: &str) -> Option<String> {
    fn find_num(contents: &str, key: &str) -> Option<u32> {
        for line in contents.lines() {
            let line = line.trim();
            if !line.starts_with("#define") {
                continue;
            }
            let mut it = line.split_whitespace();
            let _define = it.next()?;
            let name = it.next()?;
            let value = it.next()?;
            if name != key {
                continue;
            }
            return value.parse::<u32>().ok();
        }
        None
    }

    let major =
        find_num(contents, "VER_MAJOR").or_else(|| find_num(contents, "ASSIMP_VERSION_MAJOR"))?;
    let minor =
        find_num(contents, "VER_MINOR").or_else(|| find_num(contents, "ASSIMP_VERSION_MINOR"))?;
    let patch =
        find_num(contents, "VER_PATCH").or_else(|| find_num(contents, "ASSIMP_VERSION_PATCH"))?;
    Some(format!("{major}.{minor}.{patch}"))
}

fn detect_unix_link_name(lib_dir: &std::path::Path, prefer_debug: bool) -> Option<String> {
    let Ok(read) = fs::read_dir(lib_dir) else {
        return None;
    };

    let mut has_assimp = false;
    let mut has_assimpd = false;

    for entry in read.flatten() {
        let p = entry.path();
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        let is_lib = lower.ends_with(".a") || lower.ends_with(".dylib") || lower.contains(".so");
        if !is_lib {
            continue;
        }
        if lower.starts_with("libassimpd") {
            has_assimpd = true;
        } else if lower.starts_with("libassimp") {
            has_assimp = true;
        }
    }

    if prefer_debug && has_assimpd {
        Some("assimpd".to_string())
    } else if has_assimp {
        Some("assimp".to_string())
    } else if has_assimpd {
        Some("assimpd".to_string())
    } else {
        None
    }
}

fn validate_prebuilt_libs(
    cfg: &BuildConfig,
    extract_dir: &std::path::Path,
    lib_dir: &std::path::Path,
    link_kind: LinkKind,
    link_name: &str,
) {
    let Ok(read) = fs::read_dir(lib_dir) else {
        panic!(
            "prebuilt package is missing library directory: {}\n\
             Hint: rebuild and upload the prebuilt package.",
            lib_dir.display()
        );
    };

    let mut has_static = false;
    let mut has_dynamic = false;
    let mut has_windows_import_lib = false;

    for entry in read.flatten() {
        let p = entry.path();
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".a") && lower.contains(link_name) {
            has_static = true;
        }
        if (lower.ends_with(".dylib") || lower.contains(".so")) && lower.contains(link_name) {
            has_dynamic = true;
        }
        if cfg.is_windows() && lower.ends_with(".lib") && lower.contains("assimp") {
            has_windows_import_lib = true;
        }
    }

    if cfg.is_windows() {
        match link_kind {
            LinkKind::Static | LinkKind::Dynamic => {
                if !has_windows_import_lib {
                    panic!(
                        "prebuilt package is missing assimp import library (*.lib) under {}.\n\
                         Hint: rebuild and upload the prebuilt package.",
                        lib_dir.display()
                    );
                }
            }
        }

        if matches!(link_kind, LinkKind::Dynamic) {
            let bin_dir = extract_dir.join("bin");
            let has_dll = bin_dir.exists()
                && fs::read_dir(&bin_dir)
                    .ok()
                    .into_iter()
                    .flatten()
                    .flatten()
                    .any(|e| {
                        e.path()
                            .extension()
                            .and_then(|s| s.to_str())
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
                    });
            if !has_dll {
                panic!(
                    "prebuilt package is missing assimp DLLs under {}.\n\
                     Hint: rebuild and upload the prebuilt package.",
                    bin_dir.display()
                );
            }
        }

        return;
    }

    match link_kind {
        LinkKind::Static => {
            if !has_static {
                panic!(
                    "prebuilt package is missing static assimp library under {}.\n\
                     Hint: rebuild and upload the prebuilt package.",
                    lib_dir.display()
                );
            }
        }
        LinkKind::Dynamic => {
            if !has_dynamic {
                panic!(
                    "prebuilt package is missing shared assimp library under {}.\n\
                     Hint: rebuild and upload the prebuilt package.",
                    lib_dir.display()
                );
            }
        }
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

    let config = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(300)))
        .build();
    let client = ureq::Agent::new_with_config(config);
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

            let resp = match client.get(&url).call() {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = Some(format!("{} for {}", e, url));
                    continue;
                }
            };

            let status = resp.status();
            if !status.is_success() {
                last_error = Some(format!("HTTP {} for {}", status, url));
                continue;
            }

            let dst = cache_root.join(archive);
            let tmp = dst.with_extension("tmp");
            let mut reader = resp.into_body().into_reader();
            let mut file = fs::File::create(&tmp)
                .unwrap_or_else(|e| panic!("Failed to create {}: {}", tmp.display(), e));
            std::io::copy(&mut reader, &mut file).unwrap_or_else(|e| {
                panic!("Failed to download {} into {}: {}", url, tmp.display(), e)
            });
            let _ = file.sync_all();
            fs::rename(&tmp, &dst).unwrap_or_else(|e| {
                panic!(
                    "Failed to move downloaded prebuilt package {} -> {}: {}",
                    tmp.display(),
                    dst.display(),
                    e
                )
            });
            return;
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
