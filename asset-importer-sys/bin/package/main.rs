use std::{
    env, fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use flate2::{Compression, write::GzEncoder};

const VENDORED_ASSIMP_VERSION: &str = "6.0.3";

fn target_os_from_triple(target: &str) -> &'static str {
    if target.contains("-windows-") {
        "windows"
    } else if target.contains("-apple-darwin") {
        "macos"
    } else if target.contains("-linux-") {
        "linux"
    } else {
        "unknown"
    }
}

const fn static_lib() -> &'static str {
    if cfg!(feature = "static-link") {
        "static"
    } else {
        "dylib"
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap();
    let ar_dst_dir = PathBuf::from(
        env::var("ASSET_IMPORTER_PACKAGE_DIR").unwrap_or_else(|_| env::var("OUT_DIR").unwrap()),
    );

    // Get target from environment variable or use built-in target
    let target = env::var("TARGET").unwrap_or_else(|_| {
        // Fallback to built-in target if TARGET env var is not set
        env::var("CARGO_CFG_TARGET_TRIPLE").unwrap_or_else(|_| {
            // Last resort: construct a reasonable default target
            let arch = std::env::consts::ARCH;
            let os = std::env::consts::OS;
            match os {
                "windows" => format!("{}-pc-windows-msvc", arch),
                "macos" => format!("{}-apple-darwin", arch),
                "linux" => format!("{}-unknown-linux-gnu", arch),
                _ => format!("{}-unknown-{}", arch, os),
            }
        })
    });
    let crate_version = env::var("CARGO_PKG_VERSION").unwrap();
    let link_type = static_lib();
    // Note: Cargo does not guarantee that `CARGO_CFG_TARGET_*` env vars are present when running
    // `cargo run` binaries. Prefer deriving the OS from the target triple.
    let target_os = env::var("CARGO_CFG_TARGET_OS")
        .unwrap_or_else(|_| target_os_from_triple(&target).to_string());
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();

    // Packaging for Windows needs to distinguish CRT variants (md/mt). Allow explicit override
    // from CI, because `CARGO_CFG_TARGET_FEATURE` may be missing at runtime.
    let crt = if target_os == "windows" {
        if let Ok(v) = env::var("ASSET_IMPORTER_PKG_CRT") {
            if !v.trim().is_empty() {
                Box::leak(v.trim().to_string().into_boxed_str())
            } else {
                ""
            }
        } else if target_env == "msvc" && target_features.split(',').any(|f| f == "crt-static") {
            "mt"
        } else if target_env == "msvc" {
            "md"
        } else {
            ""
        }
    } else {
        ""
    };

    println!("Package configuration:");
    println!("  Target: {}", target);
    println!("  Version: {}", crate_version);
    println!("  Link type: {}", link_type);
    println!("  Package dir: {}", ar_dst_dir.display());
    if !crt.is_empty() {
        println!("  CRT: {}", crt);
    }

    let ar_filename = if crt.is_empty() {
        format!(
            "asset-importer-{}-{}-{}.tar.gz",
            crate_version, target, link_type
        )
    } else {
        format!(
            "asset-importer-{}-{}-{}-{}.tar.gz",
            crate_version, target, link_type, crt
        )
    };

    // Determine the source directory based on build type
    let from_dir = if cfg!(feature = "build-assimp") {
        // We need the CMake install prefix produced by build.rs (cmake::Config::build())
        // That path is target/<triple>/<profile>/build/asset-importer-sys-*/out
        // Prefer environment override if set (useful in CI)
        if let Ok(dir) = env::var("ASSET_IMPORTER_BUILD_OUT_DIR") {
            PathBuf::from(dir)
        } else {
            locate_build_out_dir(workspace_root, &target, link_type, &target_os)?
        }
    } else {
        // This shouldn't happen in package mode, but fallback
        return Err("Package tool requires build-assimp feature".into());
    };

    fs::create_dir_all(&ar_dst_dir)?;
    println!("Packaging at: {}", ar_dst_dir.display());

    validate_from_dir(&from_dir, link_type, &target_os)?;

    let tar_file = fs::File::create(ar_dst_dir.join(&ar_filename))?;
    let mut archive = tar::Builder::new(GzEncoder::new(tar_file, Compression::best()));

    let include_dir = from_dir.join("include");
    append_assimp_headers(&mut archive, &include_dir)?;

    let lib_root = pick_lib_root(&from_dir)?;
    append_assimp_libs(&mut archive, &lib_root, link_type, &target_os)?;

    if target_os == "windows" && link_type == "dylib" {
        append_windows_runtime_dlls(&mut archive, &from_dir.join("bin"))?;
    }

    append_manifest(
        &mut archive,
        &target,
        &crate_version,
        link_type,
        crt,
        VENDORED_ASSIMP_VERSION,
    )?;

    // Add license files
    let license_files = [
        ("LICENSE-MIT", "LICENSE-MIT"),
        ("LICENSE-APACHE", "LICENSE-APACHE"),
        ("LICENSE-assimp.txt", "LICENSE-assimp.txt"),
    ];

    for (archive_name, file_name) in license_files {
        let license_path = workspace_root.join(file_name);
        if license_path.exists() {
            let mut f = fs::File::open(&license_path)?;
            archive.append_file(archive_name, &mut f)?;
            println!("Added license file: {}", license_path.display());
        } else {
            println!("License file not found: {}", license_path.display());
        }
    }

    archive.finish()?;

    println!(
        "Package created at: {}\nTarget: {}\nLink type: {}",
        ar_dst_dir.join(&ar_filename).display(),
        target,
        link_type,
    );

    Ok(())
}

fn append_manifest(
    archive: &mut tar::Builder<GzEncoder<fs::File>>,
    target: &str,
    crate_version: &str,
    link_type: &str,
    crt: &str,
    assimp_version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut body = String::new();
    body.push_str("crate=asset-importer-sys\n");
    body.push_str(&format!("crate_version={}\n", crate_version));
    body.push_str(&format!("assimp_version={}\n", assimp_version));
    body.push_str(&format!("target={}\n", target));
    body.push_str(&format!("link_type={}\n", link_type));
    if !crt.is_empty() {
        body.push_str(&format!("crt={}\n", crt));
    }

    let bytes = body.into_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o644);
    header.set_size(bytes.len() as u64);
    header.set_cksum();
    archive.append_data(&mut header, "manifest.txt", Cursor::new(bytes))?;
    Ok(())
}

fn append_assimp_headers(
    archive: &mut tar::Builder<GzEncoder<fs::File>>,
    include_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let assimp_dir = include_dir.join("assimp");
    if !assimp_dir.exists() {
        return Err(format!(
            "Assimp include directory not found at {}",
            assimp_dir.display()
        )
        .into());
    }

    append_dir_all_files(archive, &assimp_dir, Path::new("include").join("assimp"))?;
    println!(
        "Added include/assimp headers from: {}",
        assimp_dir.display()
    );
    Ok(())
}

fn append_windows_runtime_dlls(
    archive: &mut tar::Builder<GzEncoder<fs::File>>,
    bin_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if !bin_dir.exists() {
        return Err(format!(
            "bin directory not found at {}; expected runtime DLLs for Windows dylib packaging",
            bin_dir.display()
        )
        .into());
    }

    let mut added = 0usize;
    for entry in fs::read_dir(bin_dir)?.flatten() {
        let p = entry.path();
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.to_ascii_lowercase().ends_with(".dll") {
            continue;
        }
        archive.append_path_with_name(&p, Path::new("bin").join(name))?;
        added += 1;
    }

    if added == 0 {
        return Err(format!(
            "no DLLs found under {}; refusing to package an unusable Windows dylib archive",
            bin_dir.display()
        )
        .into());
    }

    println!("Added bin/*.dll from: {}", bin_dir.display());
    Ok(())
}

fn pick_lib_root(from_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let lib = from_dir.join("lib");
    if lib.exists() {
        return Ok(lib);
    }
    let lib64 = from_dir.join("lib64");
    if lib64.exists() {
        return Ok(lib64);
    }
    Err(format!(
        "No lib directory found under {} (expected lib/ or lib64/)",
        from_dir.display()
    )
    .into())
}

fn append_assimp_libs(
    archive: &mut tar::Builder<GzEncoder<fs::File>>,
    lib_root: &Path,
    link_type: &str,
    target_os: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !lib_root.exists() {
        return Err(format!("lib root not found at {}", lib_root.display()).into());
    }

    let mut added = 0usize;

    for entry in fs::read_dir(lib_root)?.flatten() {
        let p = entry.path();
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();

        let keep = if target_os == "windows" {
            lower.ends_with(".lib") && (lower.starts_with("assimp") || lower.contains("zlib"))
        } else if link_type == "static" {
            lower.starts_with("libassimp") && lower.ends_with(".a")
        } else {
            lower.starts_with("libassimp") && (lower.ends_with(".dylib") || lower.contains(".so"))
        };

        if !keep {
            continue;
        }

        archive.append_path_with_name(&p, Path::new("lib").join(name))?;
        added += 1;
    }

    if added == 0 {
        return Err(format!(
            "No Assimp libraries selected from {} for link_type={} target_os={}",
            lib_root.display(),
            link_type,
            target_os
        )
        .into());
    }

    println!(
        "Added {} library file(s) from: {}",
        added,
        lib_root.display()
    );
    Ok(())
}

fn append_dir_all_files(
    archive: &mut tar::Builder<GzEncoder<fs::File>>,
    src: &Path,
    dst: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(src)?.flatten() {
        let p = entry.path();
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let dst_path = dst.join(name);
        if p.is_dir() {
            append_dir_all_files(archive, &p, dst_path)?;
        } else {
            archive.append_path_with_name(&p, &dst_path)?;
        }
    }
    Ok(())
}

fn validate_from_dir(
    from_dir: &Path,
    link_type: &str,
    target_os: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let include_version = from_dir.join("include").join("assimp").join("version.h");
    if !include_version.exists() {
        return Err(format!(
            "Assimp headers not found (missing {}); refusing to package an invalid archive",
            include_version.display()
        )
        .into());
    }

    let lib_dir = from_dir.join("lib");
    let lib64_dir = from_dir.join("lib64");
    let bin_dir = from_dir.join("bin");

    let lib_roots = [lib_dir.as_path(), lib64_dir.as_path()];

    let mut has_static = false;
    let mut has_shared = false;
    let mut has_windows_lib = false;

    for root in lib_roots {
        if !root.exists() {
            continue;
        }
        for entry in fs::read_dir(root)?.flatten() {
            let p = entry.path();
            let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let lower = name.to_ascii_lowercase();

            if target_os == "windows" {
                if lower.starts_with("assimp") && lower.ends_with(".lib") {
                    has_windows_lib = true;
                }
                continue;
            }

            if lower.starts_with("libassimp") && lower.ends_with(".a") {
                has_static = true;
            }
            if lower.starts_with("libassimp")
                && (lower.ends_with(".dylib") || lower.contains(".so"))
            {
                has_shared = true;
            }
        }
    }

    let has_windows_dll = if target_os == "windows" && bin_dir.exists() {
        fs::read_dir(&bin_dir)?.flatten().any(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
        })
    } else {
        false
    };

    match (target_os, link_type) {
        ("windows", "static") => {
            if !has_windows_lib {
                return Err(format!(
                    "Windows static package is missing assimp *.lib under {} (or {}).",
                    lib_dir.display(),
                    lib64_dir.display()
                )
                .into());
            }
        }
        ("windows", "dylib") => {
            if !has_windows_lib || !has_windows_dll {
                return Err(format!(
                    "Windows dylib package is missing assimp import lib (*.lib) and/or runtime DLLs (bin/*.dll). lib={}, bin={}",
                    lib_dir.display(),
                    bin_dir.display()
                )
                .into());
            }
        }
        (_, "static") => {
            if !has_static || has_shared {
                return Err(format!(
                    "Static package content mismatch: expected static assimp library only, found static={}, shared={}. from_dir={}",
                    has_static,
                    has_shared,
                    from_dir.display()
                )
                .into());
            }
        }
        (_, "dylib") => {
            if !has_shared || has_static {
                return Err(format!(
                    "Dylib package content mismatch: expected shared assimp library only, found shared={}, static={}. from_dir={}",
                    has_shared,
                    has_static,
                    from_dir.display()
                )
                .into());
            }
        }
        _ => {}
    }

    Ok(())
}

fn locate_build_out_dir(
    workspace_root: &std::path::Path,
    target: &str,
    link_type: &str,
    target_os: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Determine profile used for this run
    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".into());
    // Prefer CARGO_TARGET_DIR, else workspace_root/target
    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("target"));

    // Cargo uses different target directory layouts depending on whether `--target` is passed:
    // - with `--target <triple>`: target/<triple>/<profile>/build
    // - without `--target` (native): target/<profile>/build
    let build_roots = [
        target_dir.join(target).join(&profile).join("build"),
        target_dir.join(&profile).join("build"),
    ];

    let mut candidates: Vec<PathBuf> = Vec::new();
    for build_root in build_roots.iter().filter(|p| p.exists()) {
        if let Ok(rd) = std::fs::read_dir(build_root) {
            candidates.extend(rd.filter_map(|e| e.ok()).filter_map(|e| {
                let p = e.path();
                let name = p.file_name()?.to_string_lossy().to_string();
                if !name.starts_with("asset-importer-sys-") {
                    return None;
                }
                let out = p.join("out");
                out.join("include").exists().then_some(out)
            }));
        }
    }

    if candidates.is_empty() {
        let attempted = build_roots
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "No asset-importer-sys build out directories found under any of: {}",
            attempted
        )
        .into());
    }

    let desired = match link_type {
        "static" => PackageKind::Static,
        "dylib" => PackageKind::Dylib,
        _ => PackageKind::Unknown,
    };

    let mut matching: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|p| detect_package_kind(p, target_os) == desired)
        .collect();

    if matching.is_empty() {
        return Err(format!(
            "No asset-importer-sys build out directories match requested link type {link_type} for {target}.\n\
             Hint: build the sys crate with matching features before packaging (static: enable `static-link`, dylib: disable it)."
        )
        .into());
    }

    // Choose the most recently modified among the matching candidates.
    matching.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
    let from_dir = matching.pop().unwrap();
    println!("Using build out dir: {}", from_dir.display());
    Ok(from_dir)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PackageKind {
    Static,
    Dylib,
    Unknown,
}

fn detect_package_kind(from_dir: &Path, target_os: &str) -> PackageKind {
    let include_version = from_dir.join("include").join("assimp").join("version.h");
    if !include_version.exists() {
        return PackageKind::Unknown;
    }

    let lib_dir = from_dir.join("lib");
    let lib64_dir = from_dir.join("lib64");
    let bin_dir = from_dir.join("bin");

    if target_os == "windows" {
        let has_lib = [lib_dir.as_path(), lib64_dir.as_path()]
            .into_iter()
            .filter(|p| p.exists())
            .flat_map(|p| fs::read_dir(p).ok().into_iter().flatten().flatten())
            .any(|e| {
                e.path()
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| {
                        n.to_ascii_lowercase().starts_with("assimp")
                            && n.to_ascii_lowercase().ends_with(".lib")
                    })
            });

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

        if has_lib && has_dll {
            return PackageKind::Dylib;
        }
        if has_lib {
            return PackageKind::Static;
        }
        return PackageKind::Unknown;
    }

    let mut has_static = false;
    let mut has_shared = false;
    for root in [lib_dir.as_path(), lib64_dir.as_path()] {
        if !root.exists() {
            continue;
        }
        if let Ok(rd) = fs::read_dir(root) {
            for entry in rd.flatten() {
                let p = entry.path();
                let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
                    continue;
                };
                let lower = name.to_ascii_lowercase();
                if lower.starts_with("libassimp") && lower.ends_with(".a") {
                    has_static = true;
                }
                if lower.starts_with("libassimp")
                    && (lower.ends_with(".dylib") || lower.contains(".so"))
                {
                    has_shared = true;
                }
            }
        }
    }

    if has_shared {
        PackageKind::Dylib
    } else if has_static {
        PackageKind::Static
    } else {
        PackageKind::Unknown
    }
}
