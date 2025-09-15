/*!
 * Simple example demonstrating basic model loading with asset-importer.
 *
 * Based on Assimp's SimpleOpenGL sample, but focused on just loading and
 * inspecting the model data without OpenGL rendering.
 */

use asset_importer::{node::Node, postprocess::PostProcessSteps, Importer};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: {} <model_file>", args[0]);
        println!("Example: {} examples/models/box.obj", args[0]);
        return Ok(());
    }

    let filename = &args[1];
    println!("Loading model: {}", filename);

    // Create importer and load the model
    let importer = Importer::new();
    let scene = importer
        .read_file(filename)
        .with_post_process(
            PostProcessSteps::TRIANGULATE
                | PostProcessSteps::GEN_NORMALS
                | PostProcessSteps::FLIP_UVS,
        )
        .import_file(filename)?;

    // Print basic scene information
    println!("\n=== Scene Information ===");
    println!("Meshes: {}", scene.num_meshes());
    println!("Materials: {}", scene.num_materials());
    println!("Animations: {}", scene.num_animations());
    println!("Cameras: {}", scene.num_cameras());
    println!("Lights: {}", scene.num_lights());

    // Examine each mesh
    println!("\n=== Mesh Details ===");
    for (i, mesh) in scene.meshes().enumerate() {
        println!("Mesh {}: '{}'", i, mesh.name());
        println!("  Vertices: {}", mesh.num_vertices());
        println!("  Faces: {}", mesh.num_faces());
        println!("  Has normals: {}", mesh.normals().is_some());
        println!("  Has texture coords: {}", mesh.texture_coords(0).is_some());
        println!("  Has vertex colors: {}", mesh.vertex_colors(0).is_some());

        // Show first few vertices as example
        let vertices = mesh.vertices();
        if !vertices.is_empty() {
            println!("  First vertex: {:?}", vertices[0]);
            if vertices.len() > 1 {
                println!("  Last vertex: {:?}", vertices[vertices.len() - 1]);
            }
        }

        // Calculate bounding box
        if !vertices.is_empty() {
            let mut min = vertices[0];
            let mut max = vertices[0];

            for vertex in &vertices {
                min = min.min(*vertex);
                max = max.max(*vertex);
            }

            println!("  Bounding box: min={:?}, max={:?}", min, max);
            println!("  Size: {:?}", max - min);
        }

        println!();
    }

    // Examine materials
    if scene.num_materials() > 0 {
        println!("=== Material Details ===");
        for (i, material) in scene.materials().enumerate() {
            println!("Material {}: '{}'", i, material.name());

            // Try to get some common properties
            if let Some(diffuse) = material.get_color_property("$clr.diffuse") {
                println!("  Diffuse color: {:?}", diffuse);
            }
            if let Some(opacity) = material.get_float_property("$mat.opacity") {
                println!("  Opacity: {}", opacity);
            }
            if let Some(shininess) = material.get_float_property("$mat.shininess") {
                println!("  Shininess: {}", shininess);
            }

            println!();
        }
    }

    // Show scene graph structure
    if let Some(root) = scene.root_node() {
        println!("=== Scene Graph ===");
        print_node_hierarchy(&root, 0);
    }

    println!("Model loaded successfully!");
    Ok(())
}

fn print_node_hierarchy(node: &Node, depth: usize) {
    let indent = "  ".repeat(depth);
    println!(
        "{}Node: '{}' (meshes: {})",
        indent,
        node.name(),
        node.num_meshes()
    );

    // Print transformation matrix
    let transform = node.transformation();
    println!("{}  Transform: {:?}", indent, transform);

    // Recursively print children
    for child in node.children() {
        print_node_hierarchy(&child, depth + 1);
    }
}
