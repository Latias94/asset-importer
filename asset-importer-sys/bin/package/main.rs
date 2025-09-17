use std::{env, fs, path::PathBuf};

use flate2::{Compression, write::GzEncoder};

const LICENSE_APACHE_FILEPATH: &str = "LICENSE-APACHE";
const LICENSE_MIT_FILEPATH: &str = "LICENSE-MIT";

const fn static_lib() -> &'static str {
    if cfg!(feature = "static-link") {
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
    let from_dir = if cfg!(feature = "build-assimp") {
        // Built from source - use the cmake output directory
        out_dir.join("build")
    } else {
        // This shouldn't happen in package mode, but fallback
        return Err("Package tool requires build-assimp feature".into());
    };

    // Find license files in workspace root
    let license_apache_path = workspace_root.join("LICENSE-APACHE");
    let license_mit_path = workspace_root.join("LICENSE-MIT");

    if !license_apache_path.exists() {
        return Err(format!(
            "Apache license file not found at {}",
            license_apache_path.display()
        )
        .into());
    }
    if !license_mit_path.exists() {
        return Err(format!(
            "MIT license file not found at {}",
            license_mit_path.display()
        )
        .into());
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

    // Add license files
    let mut license_apache_file = fs::File::open(&license_apache_path)?;
    archive.append_file(LICENSE_APACHE_FILEPATH, &mut license_apache_file)?;
    println!(
        "Added Apache license file: {}",
        license_apache_path.display()
    );

    let mut license_mit_file = fs::File::open(&license_mit_path)?;
    archive.append_file(LICENSE_MIT_FILEPATH, &mut license_mit_file)?;
    println!("Added MIT license file: {}", license_mit_path.display());

    archive.finish()?;

    println!(
        "Package created at: {}\nTarget: {}\nLink type: {}",
        ar_dst_dir.join(&ar_filename).display(),
        target,
        link_type,
    );

    Ok(())
}
