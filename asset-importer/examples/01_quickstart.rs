//! Quickstart: load a model and print a concise summary

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

use asset_importer::{postprocess::PostProcessSteps, Scene};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();
    let path = common::resolve_model_path(common::ModelSource::ArgOrExamplesDir, "box.obj");

    let scene: Scene = common::import_scene(&path, PostProcessSteps::empty())?;
    println!("Loaded: {}", path.display());
    println!(
        "Meshes: {}  Materials: {}  Textures: {}  Animations: {}",
        scene.num_meshes(),
        scene.num_materials(),
        scene.num_textures(),
        scene.num_animations()
    );

    if let Some(mesh) = scene.mesh(0) {
        println!(
            "Mesh[0]: vertices={} faces={} bones={}",
            mesh.num_vertices(),
            mesh.num_faces(),
            mesh.num_bones()
        );
    }

    common::shutdown_logging();
    Ok(())
}
