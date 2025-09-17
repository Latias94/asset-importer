//! Inspect scene graph, meshes and memory info

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

use asset_importer::{node::Node, postprocess::PostProcessSteps, scene::MemoryInfo};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();
    let path = common::resolve_model_path(common::ModelSource::ArgOrExamplesDir, "box.obj");
    let scene = common::import_scene(&path, PostProcessSteps::empty())?;

    println!(
        "Root: {}",
        scene.root_node().map(|n| n.name()).unwrap_or_default()
    );
    if let Some(root) = scene.root_node() {
        print_tree(root, 0, 3);
    }

    println!("\nMeshes: {}", scene.num_meshes());
    for (i, mesh) in scene.meshes().enumerate().take(5) {
        println!(
            "  [{}] verts={} faces={} bones={} morphs={}",
            i,
            mesh.num_vertices(),
            mesh.num_faces(),
            mesh.num_bones(),
            mesh.num_anim_meshes()
        );
    }

    if let Ok(mem) = scene.memory_requirements() {
        let MemoryInfo {
            textures,
            materials,
            meshes,
            nodes,
            animations,
            cameras,
            lights,
            total,
        } = mem;
        println!(
            "\nMemory: total={} bytes (meshes={}, materials={}, nodes={})",
            total, meshes, materials, nodes
        );
        println!(
            "         textures={}, animations={}, cameras={}, lights={}",
            textures, animations, cameras, lights
        );
    }

    common::shutdown_logging();
    Ok(())
}

fn print_tree(node: Node, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }
    let indent = "  ".repeat(depth);
    println!("{}- {} (meshes={})", indent, node.name(), node.num_meshes());
    for child in node.children() {
        print_tree(child, depth + 1, max_depth);
    }
}
