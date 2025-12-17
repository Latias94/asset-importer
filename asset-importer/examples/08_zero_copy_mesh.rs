//! Zero-copy mesh access: raw views + allocation-free iterators.

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

use asset_importer::{postprocess::PostProcessSteps, raw};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();
    let path = common::resolve_model_path(common::ModelSource::ArgOrExamplesDir, "box.obj");
    let scene = common::import_scene(&path, PostProcessSteps::empty())?;

    let Some(mesh) = scene.mesh(0) else {
        eprintln!("No meshes found.");
        return Ok(());
    };

    println!("Loaded: {}", path.display());
    println!(
        "Mesh[0]: verts={} faces={} bones={}",
        mesh.num_vertices(),
        mesh.num_faces(),
        mesh.num_bones()
    );

    let verts: &[raw::AiVector3D] = mesh.vertices_raw();
    if let Some(first) = verts.first() {
        println!(
            "First vertex (raw): [{:.6}, {:.6}, {:.6}]",
            first.x, first.y, first.z
        );
    }

    // Compute an AABB without allocating a Vec<Vector3D>.
    if let Some(first) = verts.first() {
        let mut min = *first;
        let mut max = *first;
        for v in verts.iter().skip(1) {
            min.x = min.x.min(v.x);
            min.y = min.y.min(v.y);
            min.z = min.z.min(v.z);
            max.x = max.x.max(v.x);
            max.y = max.y.max(v.y);
            max.z = max.z.max(v.z);
        }
        println!(
            "AABB (raw): min=[{:.6},{:.6},{:.6}] max=[{:.6},{:.6},{:.6}]",
            min.x, min.y, min.z, max.x, max.y, max.z
        );
    }

    let tri_count = mesh.triangles_iter().count();
    println!("Triangles (triangles_iter): {}", tri_count);

    #[cfg(feature = "bytemuck")]
    {
        println!("Vertex bytes: {} bytes", mesh.vertices_bytes().len());
        println!("Index bytes: {} bytes", mesh.indices_bytes().len());
    }

    common::shutdown_logging();
    Ok(())
}
