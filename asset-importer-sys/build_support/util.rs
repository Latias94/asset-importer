use std::path::{Path, PathBuf};

pub fn warn(message: impl AsRef<str>) {
    println!("cargo:warning={}", message.as_ref());
}

pub fn join_paths_for_env(paths: &[PathBuf]) -> Option<String> {
    std::env::join_paths(paths)
        .ok()
        .map(|v| v.to_string_lossy().into_owned())
}

pub fn cargo_profile_dir(out_dir: &Path) -> &Path {
    out_dir
        .ancestors()
        .find(|path| path.file_name().is_some_and(|name| name == "build"))
        .and_then(Path::parent)
        .expect("Cargo OUT_DIR must be inside the profile's build directory")
}

pub fn copy_windows_dlls(out_dir: &Path, source_root: &Path) {
    let profile_dir = cargo_profile_dir(out_dir);
    let deps_dir = profile_dir.join("deps");
    std::fs::create_dir_all(&deps_dir).expect("Failed to create Cargo deps directory");
    for bin_dir in [source_root.join("bin"), source_root.join("build/bin")] {
        let Ok(entries) = std::fs::read_dir(&bin_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
            {
                let name = path.file_name().expect("DLL path has a file name");
                for destination in [profile_dir.join(name), deps_dir.join(name)] {
                    std::fs::copy(&path, &destination).unwrap_or_else(|error| {
                        panic!(
                            "Failed to copy {} to {}: {error}",
                            path.display(),
                            destination.display()
                        )
                    });
                }
            }
        }
    }
}
