//! Advanced Features Demo
//!
//! This example demonstrates the newly implemented advanced features:
//! - Embedded textures
//! - Axis-aligned bounding boxes (AABB)
//! - Bone and skeletal animation data

use asset_importer::{postprocess::PostProcessSteps, Scene};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Advanced Features Demo");
    println!("Demonstrating newly implemented Assimp features\n");

    // Try to load a model with embedded textures and bones
    let model_paths = [
        "examples/models/box.obj",
        "examples/models/simple.fbx",    // If available
        "examples/models/character.dae", // If available
    ];

    for model_path in &model_paths {
        println!("📁 Attempting to load: {}", model_path);

        match load_and_analyze_model(model_path) {
            Ok(_) => {
                println!("✅ Successfully analyzed {}\n", model_path);
                break; // Use the first successful model
            }
            Err(e) => {
                println!("⚠️  Could not load {}: {}\n", model_path, e);
                continue;
            }
        }
    }

    // Demonstrate with in-memory OBJ data
    println!("💾 Demonstrating with in-memory OBJ data");
    demonstrate_with_memory_data()?;

    println!("🎯 Demo completed!");
    Ok(())
}

fn load_and_analyze_model(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Load the scene with comprehensive post-processing
    let scene = Scene::from_file_with_flags(
        path,
        PostProcessSteps::TRIANGULATE
            | PostProcessSteps::GEN_SMOOTH_NORMALS
            | PostProcessSteps::CALC_TANGENT_SPACE
            | PostProcessSteps::GEN_BOUNDING_BOXES,
    )?;

    println!("📊 Scene Statistics:");
    println!("  Meshes: {}", scene.num_meshes());
    println!("  Materials: {}", scene.num_materials());
    println!("  Textures: {}", scene.num_textures());
    println!("  Animations: {}", scene.num_animations());
    println!("  Cameras: {}", scene.num_cameras());
    println!("  Lights: {}", scene.num_lights());

    // Analyze embedded textures
    analyze_textures(&scene);

    // Analyze meshes with AABB and bones
    analyze_meshes(&scene);

    // Analyze materials
    analyze_materials(&scene);

    Ok(())
}

fn analyze_textures(scene: &Scene) {
    println!("\n🎨 Texture Analysis:");

    if !scene.has_textures() {
        println!("  No embedded textures found");
        return;
    }

    for (i, texture) in scene.textures().enumerate() {
        println!("  Texture {}:", i);
        println!("    Dimensions: {}x{}", texture.width(), texture.height());
        println!(
            "    Type: {}",
            if texture.is_compressed() {
                "Compressed"
            } else {
                "Uncompressed"
            }
        );
        println!("    Format hint: '{}'", texture.format_hint());

        if let Some(filename) = texture.filename() {
            println!("    Original filename: {}", filename);
        }

        println!("    Data size: {} bytes", texture.data_size());

        // Check specific formats
        if texture.check_format("jpg") {
            println!("    📷 JPEG format detected");
        } else if texture.check_format("png") {
            println!("    🖼️  PNG format detected");
        }
    }

    // Show texture type breakdown
    let compressed_count = scene.compressed_textures().len();
    let uncompressed_count = scene.uncompressed_textures().len();
    println!(
        "  Summary: {} compressed, {} uncompressed",
        compressed_count, uncompressed_count
    );
}

fn analyze_meshes(scene: &Scene) {
    println!("\n🔺 Mesh Analysis:");

    for (i, mesh) in scene.meshes().enumerate() {
        println!("  Mesh {}:", i);
        println!("    Vertices: {}", mesh.num_vertices());
        println!("    Faces: {}", mesh.num_faces());
        println!("    Bones: {}", mesh.num_bones());

        // Analyze AABB
        let aabb = mesh.aabb();
        if aabb.is_valid() {
            println!("    📦 Bounding Box:");
            println!(
                "      Min: ({:.2}, {:.2}, {:.2})",
                aabb.min.x, aabb.min.y, aabb.min.z
            );
            println!(
                "      Max: ({:.2}, {:.2}, {:.2})",
                aabb.max.x, aabb.max.y, aabb.max.z
            );
            println!(
                "      Size: ({:.2}, {:.2}, {:.2})",
                aabb.size().x,
                aabb.size().y,
                aabb.size().z
            );
            println!(
                "      Center: ({:.2}, {:.2}, {:.2})",
                aabb.center().x,
                aabb.center().y,
                aabb.center().z
            );
            println!("      Volume: {:.2}", aabb.volume());
            println!("      Surface Area: {:.2}", aabb.surface_area());
        } else {
            println!("    📦 Bounding Box: Invalid/Empty");
        }

        // Analyze bones
        if mesh.has_bones() {
            println!("    🦴 Bone Analysis:");
            for (bone_idx, bone) in mesh.bones().enumerate() {
                println!("      Bone {}: '{}'", bone_idx, bone.name());
                println!("        Weights: {}", bone.num_weights());
                println!("        Max weight: {:.3}", bone.max_weight());
                println!("        Min weight: {:.3}", bone.min_weight());
                println!("        Avg weight: {:.3}", bone.average_weight());

                // Show offset matrix (first row only for brevity)
                let matrix = bone.offset_matrix();
                println!(
                    "        Offset matrix (row 1): [{:.3}, {:.3}, {:.3}, {:.3}]",
                    matrix.x_axis.x, matrix.x_axis.y, matrix.x_axis.z, matrix.x_axis.w
                );

                // Show some affected vertices
                let affected = bone.affected_vertices();
                if !affected.is_empty() {
                    let preview: Vec<String> =
                        affected.iter().take(5).map(|v| v.to_string()).collect();
                    println!(
                        "        Affected vertices: {} (showing first 5: {})",
                        affected.len(),
                        preview.join(", ")
                    );
                }
            }

            // Show bone names
            let bone_names = mesh.bone_names();
            println!("    🏷️  Bone names: {}", bone_names.join(", "));
        }

        // Analyze primitive types
        println!("    🔸 Primitive types:");
        if mesh.has_triangles() {
            println!("      ✓ Triangles");
        }
        if mesh.has_lines() {
            println!("      ✓ Lines");
        }
        if mesh.has_points() {
            println!("      ✓ Points");
        }
        if mesh.has_polygons() {
            println!("      ✓ Polygons");
        }
    }
}

fn analyze_materials(scene: &Scene) {
    println!("\n🎭 Material Analysis:");

    for (i, material) in scene.materials().enumerate() {
        println!("  Material {}:", i);

        let name = material.name();
        if !name.is_empty() {
            println!("    Name: {}", name);
        }

        // Show material properties
        if let Some(diffuse) = material.diffuse_color() {
            println!(
                "    Diffuse: ({:.2}, {:.2}, {:.2})",
                diffuse.x, diffuse.y, diffuse.z
            );
        }

        if let Some(specular) = material.specular_color() {
            println!(
                "    Specular: ({:.2}, {:.2}, {:.2})",
                specular.x, specular.y, specular.z
            );
        }

        if let Some(shininess) = material.shininess() {
            println!("    Shininess: {:.2}", shininess);
        }

        if let Some(opacity) = material.opacity() {
            println!("    Opacity: {:.2}", opacity);
        }

        // Show texture count for diffuse textures
        let texture_count = material.texture_count(asset_importer::TextureType::Diffuse);
        if texture_count > 0 {
            println!("    🖼️  Diffuse Textures: {}", texture_count);
        }
    }
}

fn demonstrate_with_memory_data() -> Result<(), Box<dyn std::error::Error>> {
    // Simple OBJ cube with some basic data
    let obj_data = r#"
# Simple cube with normals
v -1.0 -1.0  1.0
v  1.0 -1.0  1.0
v  1.0  1.0  1.0
v -1.0  1.0  1.0
v -1.0 -1.0 -1.0
v  1.0 -1.0 -1.0
v  1.0  1.0 -1.0
v -1.0  1.0 -1.0

vn 0.0 0.0 1.0
vn 0.0 0.0 -1.0
vn 0.0 1.0 0.0
vn 0.0 -1.0 0.0
vn 1.0 0.0 0.0
vn -1.0 0.0 0.0

f 1//1 2//1 3//1 4//1
f 5//2 8//2 7//2 6//2
f 1//6 5//6 6//6 2//6
f 2//5 6//5 7//5 3//5
f 3//3 7//3 8//3 4//3
f 5//4 1//4 4//4 8//4
"#;

    let scene = Scene::from_memory_with_flags(
        obj_data.as_bytes(),
        Some("obj"),
        PostProcessSteps::TRIANGULATE | PostProcessSteps::GEN_BOUNDING_BOXES,
    )?;

    println!("📊 Memory Scene Statistics:");
    println!("  Meshes: {}", scene.num_meshes());

    if let Some(mesh) = scene.meshes().next() {
        let aabb = mesh.aabb();
        println!("  📦 Cube AABB:");
        println!(
            "    Min: ({:.1}, {:.1}, {:.1})",
            aabb.min.x, aabb.min.y, aabb.min.z
        );
        println!(
            "    Max: ({:.1}, {:.1}, {:.1})",
            aabb.max.x, aabb.max.y, aabb.max.z
        );
        println!("    Expected: Min(-1, -1, -1), Max(1, 1, 1)");

        // Verify the AABB is correct for a unit cube
        let expected_min = asset_importer::types::Vector3D::new(-1.0, -1.0, -1.0);
        let expected_max = asset_importer::types::Vector3D::new(1.0, 1.0, 1.0);

        let tolerance = 0.01;
        let min_correct = (aabb.min - expected_min).length() < tolerance;
        let max_correct = (aabb.max - expected_max).length() < tolerance;

        if min_correct && max_correct {
            println!("    ✅ AABB calculation is correct!");
        } else {
            println!("    ⚠️  AABB calculation may be incorrect");
        }
    }

    Ok(())
}
