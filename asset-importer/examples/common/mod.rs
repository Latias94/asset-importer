//! Common helpers shared by examples.
//!
//! Keep this file dependency-free (no external crates) for simplicity.

use std::path::{Path, PathBuf};

use asset_importer::{ImportBuilder, Importer, Scene, postprocess::PostProcessSteps};

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
        .import()?;
    Ok(scene)
}

/// Convenience to build an ImportBuilder with optional property store
#[allow(dead_code)]
pub fn builder_with_defaults(path: &Path) -> ImportBuilder {
    Importer::new()
        .read_file(path)
        .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::SORT_BY_PTYPE)
}
