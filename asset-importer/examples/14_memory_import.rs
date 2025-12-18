//! Import from memory (owned/shared) with a format hint.
//!
//! This is useful for pipelines that download/decompress assets into memory before importing.
//!
//! Usage:
//!   cargo run -p asset-importer --example 14_memory_import --no-default-features --features build-assimp -- <model> [hint]
//!
//! Examples:
//!   cargo run -p asset-importer --example 14_memory_import --no-default-features --features build-assimp -- asset-importer/examples/models/box.obj obj

#[path = "common/mod.rs"]
mod common;

use std::{error::Error, path::PathBuf, sync::Arc};

use asset_importer::{Importer, postprocess::PostProcessSteps};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();

    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() >= 2 {
        PathBuf::from(&args[1])
    } else {
        common::resolve_model_path(common::ModelSource::ArgOrExamplesDir, "box.obj")
    };

    let hint = args
        .get(2)
        .map(|s| s.as_str())
        .or_else(|| path.extension().and_then(|e| e.to_str()))
        .unwrap_or("");
    let hint = (!hint.is_empty()).then_some(hint);

    let bytes = std::fs::read(&path)?;

    // Owned path: no extra copy inside the builder.
    let scene_owned = Importer::new()
        .read_from_memory_owned(bytes.clone())
        .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::SORT_BY_PTYPE)
        .with_memory_hint_opt(hint)
        .import()?;

    // Shared path: share the same buffer across multiple imports/builders without cloning bytes.
    let shared: Arc<[u8]> = bytes.into();
    let scene_shared = Importer::new()
        .read_from_memory_shared(shared)
        .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::SORT_BY_PTYPE)
        .with_memory_hint_opt(hint)
        .import()?;

    println!("Loaded from file bytes: {}", path.display());
    println!("Hint: {}", hint.unwrap_or("<none>"));
    println!(
        "Owned import: meshes={} materials={} textures={}",
        scene_owned.num_meshes(),
        scene_owned.num_materials(),
        scene_owned.num_textures()
    );
    println!(
        "Shared import: meshes={} materials={} textures={}",
        scene_shared.num_meshes(),
        scene_shared.num_materials(),
        scene_shared.num_textures()
    );

    common::shutdown_logging();
    Ok(())
}
