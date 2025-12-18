//! Multithreading (practical): process meshes in parallel and compute stats + AABB (zero-copy).
//!
//! Usage:
//!   cargo run -p asset-importer --example 12_parallel_mesh_stats --no-default-features --features build-assimp -- <model> [workers]

#[path = "common/mod.rs"]
mod common;

use std::{
    error::Error,
    sync::mpsc,
    time::{Duration, Instant},
};

use asset_importer::{Scene, postprocess::PostProcessSteps, raw};

#[derive(Debug, Clone)]
struct MeshStats {
    index: usize,
    name: String,
    vertices: usize,
    faces: usize,
    triangles: usize,
    aabb: Option<([f32; 3], [f32; 3])>,
    elapsed: Duration,
}

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();

    let args: Vec<String> = std::env::args().collect();
    let path = common::resolve_model_path(
        common::ModelSource::ArgOrExamplesDir,
        "cube_with_materials.obj",
    );
    let workers = args
        .get(2)
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&n| n > 0);

    let scene = import_scene_for_stats(&path)?;
    let meshes: Vec<_> = scene.meshes().collect();
    if meshes.is_empty() {
        eprintln!("No meshes found: {}", path.display());
        return Ok(());
    }

    let default_workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let workers = workers.unwrap_or(default_workers).min(meshes.len());

    let started = Instant::now();
    let (tx, rx) = mpsc::channel::<MeshStats>();

    std::thread::scope(|s| {
        for worker in 0..workers {
            let tx = tx.clone();
            let chunk: Vec<(usize, _)> = meshes
                .iter()
                .cloned()
                .enumerate()
                .skip(worker)
                .step_by(workers)
                .collect();

            s.spawn(move || {
                for (index, mesh) in chunk {
                    let t0 = Instant::now();
                    let vertices = mesh.num_vertices();
                    let faces = mesh.num_faces();
                    let triangles = mesh.triangles_iter().count();
                    let aabb = aabb_from_positions(mesh.vertices_raw());
                    let elapsed = t0.elapsed();
                    let _ = tx.send(MeshStats {
                        index,
                        name: mesh.name(),
                        vertices,
                        faces,
                        triangles,
                        aabb,
                        elapsed,
                    });
                }
            });
        }
        drop(tx);
    });

    let mut results: Vec<MeshStats> = rx.into_iter().collect();
    results.sort_by_key(|r| r.index);

    let total_vertices: usize = results.iter().map(|r| r.vertices).sum();
    let total_faces: usize = results.iter().map(|r| r.faces).sum();
    let total_triangles: usize = results.iter().map(|r| r.triangles).sum();

    println!("Loaded: {}", path.display());
    println!("Meshes: {} (workers={})", results.len(), workers);
    println!(
        "Totals: vertices={}, faces={}, triangles={}",
        total_vertices, total_faces, total_triangles
    );

    println!("Per-mesh:");
    for r in &results {
        if let Some((min, max)) = r.aabb {
            println!(
                "  #{:02} {:<24} vtx={:<6} tri={:<6} aabb=([{:.3},{:.3},{:.3}]..[{:.3},{:.3},{:.3}]) ({:?})",
                r.index,
                r.name,
                r.vertices,
                r.triangles,
                min[0],
                min[1],
                min[2],
                max[0],
                max[1],
                max[2],
                r.elapsed
            );
        } else {
            println!(
                "  #{:02} {:<24} vtx={:<6} tri={:<6} aabb=<none> ({:?})",
                r.index, r.name, r.vertices, r.triangles, r.elapsed
            );
        }
    }

    println!("Total elapsed: {:?}", started.elapsed());
    common::shutdown_logging();
    Ok(())
}

fn import_scene_for_stats(path: &std::path::Path) -> Result<Scene, Box<dyn Error>> {
    let steps = PostProcessSteps::TRIANGULATE | PostProcessSteps::SORT_BY_PTYPE;
    common::import_scene(path, steps)
}

fn aabb_from_positions(positions: &[raw::AiVector3D]) -> Option<([f32; 3], [f32; 3])> {
    if positions.is_empty() {
        return None;
    }

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in positions {
        min[0] = min[0].min(v.x);
        min[1] = min[1].min(v.y);
        min[2] = min[2].min(v.z);
        max[0] = max[0].max(v.x);
        max[1] = max[1].max(v.y);
        max[2] = max[2].max(v.z);
    }

    Some((min, max))
}
