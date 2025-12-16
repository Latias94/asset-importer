use std::path::PathBuf;

pub fn warn(message: impl AsRef<str>) {
    println!("cargo:warning={}", message.as_ref());
}

pub fn join_paths_for_env(paths: &[PathBuf]) -> Option<String> {
    std::env::join_paths(paths)
        .ok()
        .map(|v| v.to_string_lossy().into_owned())
}
