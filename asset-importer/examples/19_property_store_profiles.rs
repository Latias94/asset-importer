//! PropertyStore profiles: show a few common import-property recipes.
//!
//! Usage:
//!   cargo run -p asset-importer --example 19_property_store_profiles --no-default-features --features build-assimp -- <model> [profile]
//!
//! Profiles:
//! - `preview` (default): faster, minimal work
//! - `quality`: generate normals with a smoothing angle
//! - `skinning`: limit max weights per vertex

#[path = "common/mod.rs"]
mod common;

use std::{error::Error, path::PathBuf};

use asset_importer::{
    Importer,
    importer::{PropertyStore, import_properties},
    postprocess::PostProcessSteps,
};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();

    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() >= 2 {
        PathBuf::from(&args[1])
    } else {
        common::resolve_model_path(
            common::ModelSource::ArgOrExamplesDir,
            "cube_with_materials.obj",
        )
    };

    let profile = args.get(2).map(|s| s.as_str()).unwrap_or("preview");

    let mut store = PropertyStore::new();
    let steps = match profile {
        "preview" => profile_preview(&mut store),
        "quality" => profile_quality(&mut store),
        "skinning" => profile_skinning(&mut store),
        other => {
            eprintln!("Unknown profile: {other}. Use: preview | quality | skinning");
            profile_preview(&mut store)
        }
    };

    println!("Loaded: {}", path.display());
    println!("Profile: {}", profile);
    println!("Properties: {}", store.len());
    println!("PostProcess: {:?}", steps);

    let scene = Importer::new()
        .read_file(&path)
        .with_post_process(steps)
        .with_property_store(store)
        .import()?;

    println!(
        "Result: meshes={} materials={} textures={} animations={}",
        scene.num_meshes(),
        scene.num_materials(),
        scene.num_textures(),
        scene.num_animations()
    );

    common::shutdown_logging();
    Ok(())
}

fn profile_preview(store: &mut PropertyStore) -> PostProcessSteps {
    // Keep it fast. (Still triangulate so examples that iterate triangles work.)
    store
        .set_bool(import_properties::FBX_PRESERVE_PIVOTS, false)
        .set_bool(import_properties::FBX_READ_ALL_GEOMETRY_LAYERS, false);

    PostProcessSteps::TRIANGULATE | PostProcessSteps::SORT_BY_PTYPE
}

fn profile_quality(store: &mut PropertyStore) -> PostProcessSteps {
    // Demonstrate smoothing angle + other postprocess knobs.
    store
        .set_float(import_properties::MAX_SMOOTHING_ANGLE, 80.0)
        .set_bool(import_properties::REMOVE_DEGENERATE_FACES, true);

    PostProcessSteps::TRIANGULATE
        | PostProcessSteps::SORT_BY_PTYPE
        | PostProcessSteps::JOIN_IDENTICAL_VERTICES
        | PostProcessSteps::GEN_SMOOTH_NORMALS
        | PostProcessSteps::FIND_DEGENERATES
        | PostProcessSteps::FIND_INVALID_DATA
}

fn profile_skinning(store: &mut PropertyStore) -> PostProcessSteps {
    // Common engine requirement: clamp to 4 influences per vertex.
    store.set_int(import_properties::LIMIT_BONE_WEIGHTS_MAX, 4);

    PostProcessSteps::TRIANGULATE
        | PostProcessSteps::SORT_BY_PTYPE
        | PostProcessSteps::LIMIT_BONE_WEIGHTS
}
