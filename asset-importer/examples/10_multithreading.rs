//! Multithreading: share a `Scene` behind `Arc` and process meshes in parallel.

#[path = "common/mod.rs"]
mod common;

use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use asset_importer::postprocess::PostProcessSteps;

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();
    let path = common::resolve_model_path(common::ModelSource::ArgOrExamplesDir, "box.obj");
    let scene = Arc::new(common::import_scene(&path, PostProcessSteps::empty())?);

    let mesh_count = scene.num_meshes();
    if mesh_count == 0 {
        eprintln!("No meshes found.");
        return Ok(());
    }

    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(mesh_count.max(1));

    let total_vertices = AtomicUsize::new(0);
    let total_faces = AtomicUsize::new(0);
    let total_triangles = AtomicUsize::new(0);

    std::thread::scope(|s| {
        for worker in 0..workers {
            let scene = scene.clone();
            let total_vertices = &total_vertices;
            let total_faces = &total_faces;
            let total_triangles = &total_triangles;
            s.spawn(move || {
                for i in (worker..mesh_count).step_by(workers) {
                    if let Some(mesh) = scene.mesh(i) {
                        total_vertices.fetch_add(mesh.num_vertices(), Ordering::Relaxed);
                        total_faces.fetch_add(mesh.num_faces(), Ordering::Relaxed);
                        total_triangles.fetch_add(mesh.triangles_iter().count(), Ordering::Relaxed);
                    }
                }
            });
        }
    });

    println!("Loaded: {}", path.display());
    println!("Meshes: {}", mesh_count);
    println!("Workers: {}", workers);
    println!("Total vertices: {}", total_vertices.load(Ordering::Relaxed));
    println!("Total faces: {}", total_faces.load(Ordering::Relaxed));
    println!(
        "Total triangles: {}",
        total_triangles.load(Ordering::Relaxed)
    );

    common::shutdown_logging();
    Ok(())
}
