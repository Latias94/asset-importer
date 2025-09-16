//! Example demonstrating russimp-compatible interface
//!
//! This example shows how to use the russimp-style static methods
//! for users migrating from russimp to asset-importer.

use asset_importer::{
    import_properties, node::Node, postprocess::PostProcessSteps, PropertyStore, Scene,
};

fn traverse_nodes(node: &Node, indent: String) {
    println!("{}{}", indent, node.name());
    for child in node.children() {
        traverse_nodes(&child, format!("  {}", indent));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Demonstrating russimp-compatible interface");

    // Example 1: Simple file loading (russimp style)
    println!("\n📁 Example 1: Simple file loading");
    let model_path = "examples/models/box.obj";

    match Scene::from_file(model_path) {
        Ok(scene) => {
            println!("✅ Loaded scene with {} meshes", scene.num_meshes());
        }
        Err(e) => {
            println!("⚠️  Could not load {}: {}", model_path, e);
        }
    }

    // Example 2: File loading with post-processing (russimp style)
    println!("\n🔧 Example 2: File loading with post-processing");
    let post_process = PostProcessSteps::TRIANGULATE
        | PostProcessSteps::GEN_SMOOTH_NORMALS
        | PostProcessSteps::JOIN_IDENTICAL_VERTICES;

    match Scene::from_file_with_flags(model_path, post_process) {
        Ok(scene) => {
            println!(
                "✅ Loaded scene with post-processing: {} meshes",
                scene.num_meshes()
            );
        }
        Err(e) => {
            println!("⚠️  Could not load {}: {}", model_path, e);
        }
    }

    // Example 3: File loading with properties (russimp style)
    println!("\n⚙️  Example 3: File loading with properties");

    // Create property store (similar to russimp)
    let mut props = PropertyStore::new();
    props
        .set_float(import_properties::MAX_SMOOTHING_ANGLE, 80.0)
        .set_bool(import_properties::REMOVE_DEGENERATE_FACES, true)
        .set_int(import_properties::LIMIT_BONE_WEIGHTS_MAX, 4);

    println!("🔧 Properties configured:");
    for (key, value) in props.properties() {
        println!("  {} = {:?}", key, value);
    }

    match Scene::from_file_with_props(model_path, post_process, &props) {
        Ok(scene) => {
            println!(
                "✅ Loaded scene with properties: {} meshes",
                scene.num_meshes()
            );

            // Traverse scene hierarchy (russimp style)
            if let Some(root) = scene.root_node() {
                println!("\n🌳 Scene hierarchy:");
                traverse_nodes(&root, String::new());
            }
        }
        Err(e) => {
            println!("⚠️  Could not load {}: {}", model_path, e);
        }
    }

    // Example 4: Memory loading (russimp style)
    println!("\n💾 Example 4: Memory loading");

    // Simple OBJ cube data
    let obj_data = r#"
# Simple cube
v -1.0 -1.0  1.0
v  1.0 -1.0  1.0
v  1.0  1.0  1.0
v -1.0  1.0  1.0
v -1.0 -1.0 -1.0
v  1.0 -1.0 -1.0
v  1.0  1.0 -1.0
v -1.0  1.0 -1.0

f 1 2 3 4
f 5 8 7 6
f 1 5 6 2
f 2 6 7 3
f 3 7 8 4
f 5 1 4 8
"#;

    match Scene::from_memory(obj_data.as_bytes(), Some("obj")) {
        Ok(scene) => {
            println!("✅ Loaded scene from memory: {} meshes", scene.num_meshes());
        }
        Err(e) => {
            println!("⚠️  Could not load from memory: {}", e);
        }
    }

    // Example 5: Memory loading with properties (russimp style)
    println!("\n🔧 Example 5: Memory loading with properties");

    match Scene::from_memory_with_props(obj_data.as_bytes(), Some("obj"), post_process, &props) {
        Ok(scene) => {
            println!(
                "✅ Loaded scene from memory with properties: {} meshes",
                scene.num_meshes()
            );

            // Show material information
            println!("\n🎨 Materials:");
            for (i, material) in scene.materials().enumerate() {
                println!("  Material {}: {:?}", i, material.name());

                // Show some material properties
                if let Some(diffuse) = material.diffuse_color() {
                    println!(
                        "    Diffuse: ({:.2}, {:.2}, {:.2})",
                        diffuse.x, diffuse.y, diffuse.z
                    );
                }
                if let Some(shininess) = material.shininess() {
                    println!("    Shininess: {:.2}", shininess);
                }
            }
        }
        Err(e) => {
            println!("⚠️  Could not load from memory: {}", e);
        }
    }

    println!("\n🎯 Comparison with our Builder API:");
    println!("   russimp style: Scene::from_file_with_props(path, flags, &props)");
    println!("   Builder style: Importer::new().read_file(path).with_post_process(flags).with_property_store(props).import_file(path)");
    println!("\n✨ Both styles are supported for maximum flexibility!");

    Ok(())
}
