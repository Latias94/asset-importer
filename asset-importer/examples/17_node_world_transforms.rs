//! Scene graph transforms: compute global transforms by traversing the node hierarchy.
//!
//! This example demonstrates:
//! - how to walk `Scene::root_node()` recursively
//! - how to compute `global = parent * local` (column-major)
//! - how to map node mesh indices to `Scene::mesh(i)`
//! - how to transform a mesh AABB into world space (8-corner transform)
//!
//! Usage:
//!   cargo run -p asset-importer --example 17_node_world_transforms --no-default-features --features build-assimp -- <model>

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

use asset_importer::{
    aabb::AABB,
    node::Node,
    postprocess::PostProcessSteps,
    types::{Matrix4x4, Vector4D},
};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();
    let path = common::resolve_model_path(
        common::ModelSource::ArgOrExamplesDir,
        "cube_with_materials.obj",
    );
    let scene = common::import_scene(&path, PostProcessSteps::empty())?;

    let Some(root) = scene.root_node() else {
        eprintln!("Scene has no root node.");
        return Ok(());
    };

    println!("Loaded: {}", path.display());
    println!("Nodes: {}", count_nodes(&root));
    println!("Meshes: {}", scene.num_meshes());
    println!("---");

    walk_node(&scene, &root, Matrix4x4::IDENTITY, 0);

    common::shutdown_logging();
    Ok(())
}

fn count_nodes(node: &Node) -> usize {
    1 + node.children().map(|c| count_nodes(&c)).sum::<usize>()
}

fn walk_node(scene: &asset_importer::Scene, node: &Node, parent_world: Matrix4x4, depth: usize) {
    let local = node.transformation();
    let world = mul_mat4(parent_world, local);

    let (scale, _rot, translation) = world.to_scale_rotation_translation();
    println!(
        "{:indent$}node: {} meshes={} children={} T=({:.3},{:.3},{:.3}) S=({:.3},{:.3},{:.3})",
        "",
        node.name_str(),
        node.num_meshes(),
        node.num_children(),
        translation.x,
        translation.y,
        translation.z,
        scale.x,
        scale.y,
        scale.z,
        indent = depth * 2
    );

    // Print (and transform) up to a few meshes per node to keep output readable.
    let mut printed = 0usize;
    for mesh_index in node.mesh_indices_iter() {
        let Some(mesh) = scene.mesh(mesh_index) else {
            continue;
        };
        let local_aabb = mesh.aabb();
        let world_aabb = local_aabb.transformed(&world);
        println!(
            "{:indent$}mesh[{mesh_index}]: name={} local_aabb={} world_aabb={}",
            "",
            mesh.name_str(),
            fmt_aabb(local_aabb),
            fmt_aabb(world_aabb),
            indent = depth * 2 + 2
        );
        printed += 1;
        if printed >= 3 {
            if node.num_meshes() > printed {
                println!(
                    "{:indent$}... ({} more)",
                    "",
                    node.num_meshes() - printed,
                    indent = depth * 2 + 2
                );
            }
            break;
        }
    }

    for child in node.children() {
        walk_node(scene, &child, world, depth + 1);
    }
}

fn fmt_aabb(aabb: AABB) -> String {
    if aabb.is_empty() {
        return "<empty>".to_string();
    }
    format!(
        "([{:>7.3},{:>7.3},{:>7.3}]..[{:>7.3},{:>7.3},{:>7.3}])",
        aabb.min.x, aabb.min.y, aabb.min.z, aabb.max.x, aabb.max.y, aabb.max.z
    )
}

#[inline]
fn mul_mat4(a: Matrix4x4, b: Matrix4x4) -> Matrix4x4 {
    // Column-major: columns of (A*B) are A * columns(B).
    Matrix4x4::from_cols(
        a.mul_vec4(b.x_axis),
        a.mul_vec4(b.y_axis),
        a.mul_vec4(b.z_axis),
        a.mul_vec4(b.w_axis),
    )
}

// Keep this function here as a learning aid for column-major math:
// it shows that `mul_vec4` is equivalent to a linear combination of columns.
#[allow(dead_code)]
#[inline]
fn mul_vec4_by_cols(m: Matrix4x4, v: Vector4D) -> Vector4D {
    m.x_axis * v.x + m.y_axis * v.y + m.z_axis * v.z + m.w_axis * v.w
}
