//! Common helpers shared by examples.
//!
//! Keep this file dependency-free (no external crates) for simplicity.

use std::path::{Path, PathBuf};

use asset_importer::{
    ImportBuilder, Importer, Scene,
    logging::{attach_file_stream, attach_stderr_stream, attach_stdout_stream, detach_all_streams},
    postprocess::PostProcessSteps,
};

/// How to pick a model path for examples
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum ModelSource {
    /// Only accept CLI arg; print usage if missing
    ArgOnly,
    /// Accept CLI arg, else fallback to examples/models/<name>
    ArgOrExamplesDir,
}

/// Resolve a model path from CLI args or examples/models directory
#[allow(dead_code)]
pub fn resolve_model_path(source: ModelSource, fallback_name: &str) -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 {
        return PathBuf::from(&args[1]);
    }
    match source {
        ModelSource::ArgOnly => {
            eprintln!(
                "Usage: {} <model_file>",
                args.first().map(|s| s.as_str()).unwrap_or("example")
            );
            std::process::exit(1);
        }
        ModelSource::ArgOrExamplesDir => {
            let mut p = PathBuf::from("asset-importer/examples/models");
            p.push(fallback_name);
            p
        }
    }
}

/// Attach default logging streams if AI_EX_VERBOSE=1 or AI_EX_STDERR=1
pub fn init_logging_from_env() {
    // Only enable logging if explicitly requested to avoid potential issues
    let verbose = std::env::var("AI_EX_VERBOSE").ok().as_deref() == Some("1");
    let log_to_stderr = std::env::var("AI_EX_STDERR").ok().as_deref() == Some("1");
    let has_log_file = std::env::var("AI_EX_LOGFILE").is_ok();

    // Only attach streams if logging is explicitly requested
    if verbose || log_to_stderr || has_log_file {
        // Use safe logging functions instead of Assimp's predefined streams
        if log_to_stderr {
            let _ = attach_stderr_stream();
        } else {
            let _ = attach_stdout_stream();
        }

        // Optional FILE stream via AI_EX_LOGFILE
        if let Ok(file_path) = std::env::var("AI_EX_LOGFILE") {
            let _ = attach_file_stream(PathBuf::from(file_path));
        }

        // verbose logging is global
        asset_importer::enable_verbose_logging(verbose);
    }
}

/// Clean up logging on exit
pub fn shutdown_logging() {
    detach_all_streams();
}

/// Import a scene using a set of default post-process steps. Accepts extra steps to combine.
#[allow(dead_code)]
pub fn import_scene(
    path: &Path,
    extra: PostProcessSteps,
) -> Result<Scene, Box<dyn std::error::Error>> {
    let mut steps = PostProcessSteps::TRIANGULATE | PostProcessSteps::SORT_BY_PTYPE;
    steps |= extra;
    let scene = Importer::new()
        .read_file(path)
        .with_post_process(steps)
        .import_file(path)?;
    Ok(scene)
}

/// Convenience to build an ImportBuilder with optional property store
#[allow(dead_code)]
pub fn builder_with_defaults(path: &Path) -> ImportBuilder {
    Importer::new()
        .read_file(path)
        .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::SORT_BY_PTYPE)
}
