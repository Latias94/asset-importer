use std::{env, fs, path::PathBuf};

use flate2::{write::GzEncoder, Compression};

const LICENSE_FILEPATH: &str = "LICENSE";

const fn static_lib() -> &'static str {
    if cfg!(feature = "static") {
        "static"
    } else {
        "dylib"
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap();
    let ar_dst_dir = PathBuf::from(
        env::var("ASSET_IMPORTER_PACKAGE_DIR").unwrap_or_else(|_| env::var("OUT_DIR").unwrap()),
    );

    let target = env::var("TARGET").unwrap();
    let crate_version = env::var("CARGO_PKG_VERSION").unwrap();
    let link_type = static_lib();

    let ar_filename = format!(
        "asset-importer-{}-{}-{}.tar.gz",
        crate_version, target, link_type
    );

    // Determine the source directory based on build type
    let from_dir = if cfg!(feature = "build-assimp") || cfg!(feature = "static") {
        // Built from source - use the cmake output directory
        out_dir.join("build")
    } else {
        // This shouldn't happen in package mode, but fallback
        return Err("Package tool requires build-assimp or static feature".into());
    };

    // Find license file in workspace root
    let license_path = workspace_root.join("LICENSE");
    if !license_path.exists() {
        return Err(format!("License file not found at {}", license_path.display()).into());
    }

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

    // On Windows, also check for lib64 and bin directories
    if cfg!(target_os = "windows") {
        let bin_dir = from_dir.join("bin");
        if bin_dir.exists() {
            archive.append_dir_all("bin", &bin_dir)?;
            println!("Added bin directory: {}", bin_dir.display());
        }
    }

    // Check for lib64 directory (common on Linux)
    let lib64_dir = from_dir.join("lib64");
    if lib64_dir.exists() {
        archive.append_dir_all("lib64", &lib64_dir)?;
        println!("Added lib64 directory: {}", lib64_dir.display());
    }

    // Add license file
    let mut license_file = fs::File::open(&license_path)?;
    archive.append_file(LICENSE_FILEPATH, &mut license_file)?;
    println!("Added license file: {}", license_path.display());

    archive.finish()?;

    println!(
        "Package created at: {}\nTarget: {}\nLink type: {}",
        ar_dst_dir.join(&ar_filename).display(),
        target,
        link_type,
    );

    Ok(())
}
