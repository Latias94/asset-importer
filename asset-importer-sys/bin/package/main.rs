use std::{env, fs, path::PathBuf};

use flate2::{Compression, write::GzEncoder};

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
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    let crt = if target_os == "windows" && target_env == "msvc" {
        if target_features.split(',').any(|f| f == "crt-static") {
            "mt"
        } else {
            "md"
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
            locate_build_out_dir(workspace_root, &target)?
        }
    } else {
        // This shouldn't happen in package mode, but fallback
        return Err("Package tool requires build-assimp feature".into());
    };

    fs::create_dir_all(&ar_dst_dir)?;
    println!("Packaging at: {}", ar_dst_dir.display());

    let tar_file = fs::File::create(ar_dst_dir.join(&ar_filename))?;
    let mut archive = tar::Builder::new(GzEncoder::new(tar_file, Compression::best()));

    // Add include directory
    let include_dir = from_dir.join("include");
    if include_dir.exists() {
        archive.append_dir_all("include", &include_dir)?;
        println!("Added include directory: {}", include_dir.display());
    } else {
        return Err(format!("Include directory not found at {}", include_dir.display()).into());
    }

    // Add lib directory
    let lib_dir = from_dir.join("lib");
    if lib_dir.exists() {
        archive.append_dir_all("lib", &lib_dir)?;
        println!("Added lib directory: {}", lib_dir.display());
    }

    // Also include lib64 (Linux) and bin (Windows/macOS install sometimes uses bin for dylibs)
    for (name, dir) in [
        ("lib64", from_dir.join("lib64")),
        ("bin", from_dir.join("bin")),
    ] {
        if dir.exists() {
            archive.append_dir_all(name, &dir)?;
            println!("Added {} directory: {}", name, dir.display());
        }
    }

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

fn locate_build_out_dir(
    workspace_root: &std::path::Path,
    target: &str,
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
    // Choose the most recently modified
    candidates.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
    let from_dir = candidates.pop().unwrap();
    println!("Using build out dir: {}", from_dir.display());
    Ok(from_dir)
}
