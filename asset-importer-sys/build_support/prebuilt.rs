use std::{
    collections::hash_map::DefaultHasher,
    env, fs,
    hash::Hasher,
    io::{Read, Seek},
    path::{Path, PathBuf},
};

use crate::build_support::{
    config::BuildConfig,
    plan::{BuildMethod, BuildPlan, LinkKind},
    util, version,
};

use flate2::read::GzDecoder;
use tar::Archive;

const PACKAGE_PREFIX: &str = "asset-importer";

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

    // Watch candidate additions too, e.g. a preferred MSVC CRT package replacing a fallback.
    println!("cargo:rerun-if-changed={}", package_root.display());
    let selected_archive = pick_archive_name(&package_root, &archive_names);
    println!("cargo:rerun-if-changed={}", selected_archive.display());
    let archive_path = selected_archive
        .canonicalize()
        .expect("Failed to resolve prebuilt archive path");
    if archive_path != selected_archive {
        println!("cargo:rerun-if-changed={}", archive_path.display());
    }
    let extract_dir = extract_archive(cfg, &archive_path);

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

    validate_prebuilt_package(&extract_dir, &include_dir);

    let lib_name =
        detect_assimp_link_name(&lib_dir, cfg, link_kind).unwrap_or_else(|| "assimp".to_string());

    validate_prebuilt_libs(cfg, &extract_dir, &lib_dir, link_kind, &lib_name);

    // Prebuilt static libs may require linking zlib explicitly.
    if matches!(link_kind, LinkKind::Static) {
        if cfg.is_windows() {
            link_windows_zlib(&lib_dir, cfg);
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

fn validate_prebuilt_package(extract_dir: &std::path::Path, include_dir: &std::path::Path) {
    let manifest_path = extract_dir.join("manifest.txt");
    if manifest_path.exists() {
        let contents = std::fs::read_to_string(&manifest_path).unwrap_or_else(|e| {
            panic!(
                "failed to read prebuilt package manifest {}: {}\n\
                 Hint: rebuild and upload the prebuilt package.",
                manifest_path.display(),
                e
            )
        });

        if let Some(version) = parse_manifest_value(&contents, "assimp_version") {
            require_prebuilt_assimp_version(version);
            return;
        }

        util::warn(format!(
            "prebuilt package manifest {} has no assimp_version entry; falling back to header version check",
            manifest_path.display()
        ));
    }

    validate_prebuilt_headers(include_dir);
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

    let version = version::parse_header(&contents).unwrap_or_else(|| {
        panic!(
            "could not determine Assimp version from {}; refusing an unverified prebuilt package. Rebuild the package with the current package tool.",
            header.display()
        )
    });

    require_prebuilt_assimp_version(&version);
}

fn parse_manifest_value<'a>(contents: &'a str, key: &str) -> Option<&'a str> {
    contents.lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        (k.trim() == key).then(|| v.trim())
    })
}

fn require_prebuilt_assimp_version(version: &str) {
    let expected = version::BUNDLED_VERSION;
    if version == expected {
        return;
    }

    panic!(
        "prebuilt Assimp package is version {}, but this crate expects {}.\n\
         Hint: rebuild the prebuilt package for this crate version, remove `prebuilt` to use the default vendored build, or use `--features system`.",
        version, expected
    );
}

fn detect_assimp_link_name(
    lib_dir: &Path,
    cfg: &BuildConfig,
    link_kind: LinkKind,
) -> Option<String> {
    let read = fs::read_dir(lib_dir).ok()?;
    let mut fallback = None;
    for entry in read.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
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
        fallback.get_or_insert_with(|| stem.to_string());
    }
    fallback
}

fn validate_prebuilt_libs(
    cfg: &BuildConfig,
    extract_dir: &Path,
    lib_dir: &Path,
    link_kind: LinkKind,
    link_name: &str,
) {
    let read = fs::read_dir(lib_dir).unwrap_or_else(|error| {
        panic!(
            "prebuilt package is missing library directory {}: {error}",
            lib_dir.display()
        )
    });
    let has_library = read.flatten().any(|entry| {
        let path = entry.path();
        path.file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| {
                util::library_filename::assimp_link_name(
                    name,
                    &cfg.target,
                    matches!(link_kind, LinkKind::Static),
                )
            })
            == Some(link_name)
    });
    if !has_library {
        panic!(
            "prebuilt package is missing a {:?} Assimp library for {} under {}.\n\
             Hint: rebuild and upload the prebuilt package for this target ABI.",
            link_kind,
            cfg.target,
            lib_dir.display()
        );
    }

    if cfg.is_windows() && matches!(link_kind, LinkKind::Dynamic) {
        let bin_dir = extract_dir.join("bin");
        let has_dll = fs::read_dir(&bin_dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| {
                entry.file_name().to_str().is_some_and(|name| {
                    let lower = name.to_ascii_lowercase();
                    lower.contains("assimp") && lower.ends_with(".dll")
                })
            });
        if !has_dll {
            panic!(
                "prebuilt package is missing assimp DLLs under {}.\n\
                 Hint: rebuild and upload the prebuilt package.",
                bin_dir.display()
            );
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

fn extract_archive(cfg: &BuildConfig, archive_path: &Path) -> PathBuf {
    let dst = cfg.out_dir.join("prebuilt");
    let stamp_path = dst.join(".asset-importer-archive");
    let mut file = fs::File::open(archive_path).expect("Failed to open prebuilt archive");
    // This digest only invalidates a local cache; it is not an authenticity check.
    let mut digest = DefaultHasher::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .expect("Failed to read prebuilt archive");
        if count == 0 {
            break;
        }
        digest.write(&buffer[..count]);
    }
    let identity = format!(
        "archive={archive_path:?}\ncontent={:016x}\n",
        digest.finish()
    );
    let include_ok = dst.join("include").is_dir();
    let lib_ok = dst.join("lib").is_dir() || dst.join("lib64").is_dir();
    if include_ok
        && lib_ok
        && fs::read_to_string(&stamp_path).ok().as_deref() == Some(identity.as_str())
    {
        return dst;
    }

    if let Ok(metadata) = fs::symlink_metadata(&dst) {
        let out_dir = cfg
            .out_dir
            .canonicalize()
            .expect("Failed to resolve OUT_DIR");
        let canonical_dst = dst
            .canonicalize()
            .expect("Failed to resolve extract directory");
        assert!(
            !metadata.file_type().is_symlink() && canonical_dst.parent() == Some(out_dir.as_path()),
            "Refusing to remove a prebuilt extract directory outside OUT_DIR: {}",
            dst.display()
        );
        fs::remove_dir_all(&dst).expect("Failed to clear stale prebuilt extract directory");
    }
    fs::create_dir_all(&dst).expect("Failed to create extract directory");

    file.rewind().expect("Failed to rewind prebuilt archive");
    let mut archive = Archive::new(GzDecoder::new(file));
    archive
        .unpack(&dst)
        .expect("Failed to extract prebuilt archive");
    fs::write(stamp_path, identity).expect("Failed to record prebuilt archive identity");
    dst
}

fn ensure_runtime_libs(cfg: &BuildConfig, extract_dir: &std::path::Path) {
    if cfg.is_windows() {
        util::copy_windows_dlls(&cfg.out_dir, extract_dir);
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

fn link_windows_zlib(lib_dir: &Path, cfg: &BuildConfig) {
    let read = fs::read_dir(lib_dir).expect("Failed to read prebuilt library directory");
    for entry in read.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if let Some(stem) = util::library_filename::windows_zlib_link_name(name, &cfg.target) {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            println!("cargo:rustc-link-lib=static={}", stem);
            return;
        }
    }
    panic!(
        "prebuilt static package is missing bundled zlib for {} under {}. Rebuild the package with the current package tool.",
        cfg.target,
        lib_dir.display()
    );
}
