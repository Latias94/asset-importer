//! Progress + cancellation: use a progress handler and abort the import.
//!
//! Notes:
//! - Assimp may not always emit progress callbacks for tiny files, so use a larger model if you
//!   want to reliably observe multiple updates.
//!
//! Usage:
//!   cargo run -p asset-importer --example 13_progress_cancel --no-default-features --features build-assimp -- <model>

#[path = "common/mod.rs"]
mod common;

use std::{
    error::Error,
    sync::atomic::{AtomicUsize, Ordering},
};

use asset_importer::{Importer, postprocess::PostProcessSteps};

static CALLS: AtomicUsize = AtomicUsize::new(0);

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();

    let path = common::resolve_model_path(
        common::ModelSource::ArgOrExamplesDir,
        "cube_with_materials.obj",
    );

    let cancel_after_calls = 3usize;
    let cancel_after_percent = 0.20f32;

    let result = Importer::new()
        .read_file(&path)
        .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::SORT_BY_PTYPE)
        .with_progress_handler_fn(move |p, msg| {
            let n = CALLS.fetch_add(1, Ordering::Relaxed) + 1;
            println!(
                "progress: {:>5.1}%  calls={}  msg={}",
                p * 100.0,
                n,
                msg.unwrap_or("<none>")
            );

            // Abort either after N callbacks or after reaching a threshold percentage.
            n < cancel_after_calls && p < cancel_after_percent
        })
        .import();

    match result {
        Ok(scene) => {
            println!(
                "Import succeeded (progress callbacks: {}). Meshes={}",
                CALLS.load(Ordering::Relaxed),
                scene.num_meshes()
            );
            println!(
                "If you expected cancellation, try a larger model (some importers may not emit progress for tiny files)."
            );
        }
        Err(e) => {
            println!(
                "Import cancelled/failed as expected (progress callbacks: {}). Error: {}",
                CALLS.load(Ordering::Relaxed),
                e
            );
        }
    }

    common::shutdown_logging();
    Ok(())
}
