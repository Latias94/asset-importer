/*!
 * Complete Feature Showcase
 *
 * This example demonstrates all the advanced features of the asset-importer library,
 * showcasing why it's the most comprehensive Assimp Rust binding available.
 */

use asset_importer::{
    enable_verbose_logging, get_all_importer_descs, get_importer_desc, import_properties,
    material_keys,
    metadata::{collada_metadata, common_metadata},
    postprocess::PostProcessSteps,
    progress::ClosureProgressHandler,
    version, Importer, ImporterFlags, PropertyStore, Scene,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Asset-Importer Complete Feature Showcase");
    println!("============================================\n");

    // Enable verbose logging to see what's happening
    enable_verbose_logging(true);

    // Show version information
    show_version_info();

    // Demonstrate importer discovery
    demonstrate_importer_discovery()?;

    // Show metadata constants
    demonstrate_metadata_constants();

    // Demonstrate advanced import with all features
    demonstrate_advanced_import()?;

    // Show russimp compatibility
    demonstrate_russimp_compatibility()?;

    println!("\n🎉 All features demonstrated successfully!");
    println!("Asset-Importer is the most comprehensive Assimp Rust binding! 🚀");

    Ok(())
}

fn show_version_info() {
    println!("📋 Version Information:");
    println!(
        "  Assimp Version: {}.{}.{}",
        version::assimp_version_major(),
        version::assimp_version_minor(),
        version::assimp_version_revision()
    );
    println!("  Compile Flags: Available");
    println!();
}

fn demonstrate_importer_discovery() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Importer Discovery:");

    // Check specific format support
    if let Some(obj_desc) = get_importer_desc("obj") {
        println!("  ✅ OBJ Support: {}", obj_desc.name);
        println!("     Author: {}", obj_desc.author);
        println!("     Extensions: {:?}", obj_desc.file_extensions);

        // Check capabilities
        if obj_desc.flags.contains(ImporterFlags::SUPPORT_TEXT_FLAVOUR) {
            println!("     📄 Supports text format");
        }
        if obj_desc
            .flags
            .contains(ImporterFlags::SUPPORT_BINARY_FLAVOUR)
        {
            println!("     🔢 Supports binary format");
        }
    }

    // Show all available importers
    let all_importers = get_all_importer_descs();
    println!("  📊 Total Importers Available: {}", all_importers.len());

    // Show first few importers
    println!("  🎯 Sample Importers:");
    for (i, desc) in all_importers.iter().take(5).enumerate() {
        println!("     {}. {} - {:?}", i + 1, desc.name, desc.file_extensions);
    }

    println!();
    Ok(())
}

fn demonstrate_metadata_constants() {
    println!("📊 Metadata Constants:");
    println!("  Common Metadata Keys:");
    println!("    📁 Source Format: {}", common_metadata::SOURCE_FORMAT);
    println!(
        "    📝 Source Version: {}",
        common_metadata::SOURCE_FORMAT_VERSION
    );
    println!(
        "    👤 Source Generator: {}",
        common_metadata::SOURCE_GENERATOR
    );
    println!(
        "    ©️  Source Copyright: {}",
        common_metadata::SOURCE_COPYRIGHT
    );

    println!("  Collada-Specific Keys:");
    println!("    🆔 ID: {}", collada_metadata::ID);
    println!("    🔗 SID: {}", collada_metadata::SID);

    println!();
}

fn demonstrate_advanced_import() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Advanced Import Features:");

    // Create a simple OBJ model in memory
    let obj_data = create_advanced_test_model();

    // Set up advanced properties
    let mut props = PropertyStore::new();
    props.set_int(import_properties::MAX_SMOOTHING_ANGLE, 80);
    props.set_bool(import_properties::FBX_PRESERVE_PIVOTS, false);

    // Advanced import with all features
    let scene = Importer::new()
        .read_from_memory(obj_data.as_bytes())
        .with_post_process(
            PostProcessSteps::TRIANGULATE
                | PostProcessSteps::GEN_SMOOTH_NORMALS
                | PostProcessSteps::CALC_TANGENT_SPACE
                | PostProcessSteps::GEN_BOUNDING_BOXES,
        )
        .with_property_store_ref(&props)
        .with_progress_handler(Box::new(ClosureProgressHandler::new(
            |progress, _percentage| {
                if progress > 0.0 {
                    println!("    📈 Import Progress: {:.1}%", progress * 100.0);
                }
                true
            },
        )))
        .import_from_memory(obj_data.as_bytes(), Some("obj"))?;

    // Analyze the imported scene
    analyze_scene_comprehensive(&scene)?;

    println!();
    Ok(())
}

fn analyze_scene_comprehensive(scene: &Scene) -> Result<(), Box<dyn std::error::Error>> {
    println!("  📊 Scene Analysis:");
    println!("    🔺 Meshes: {}", scene.num_meshes());
    println!("    🎨 Materials: {}", scene.num_materials());
    println!("    🎬 Animations: {}", scene.num_animations());
    println!("    📷 Cameras: {}", scene.num_cameras());
    println!("    💡 Lights: {}", scene.num_lights());
    println!("    🖼️  Textures: {}", scene.num_textures());

    // Analyze meshes with all new features
    for (i, mesh) in scene.meshes().enumerate() {
        println!(
            "    Mesh {}: {} vertices, {} faces",
            i,
            mesh.num_vertices(),
            mesh.num_faces()
        );

        // Show AABB information
        let aabb = mesh.aabb();
        if aabb.is_valid() {
            println!("      📦 AABB: min={:?}, max={:?}", aabb.min, aabb.max);
            println!(
                "      📏 Size: {:?}, Volume: {:.2}",
                aabb.size(),
                aabb.volume()
            );
        }

        // Show bone information
        if mesh.has_bones() {
            println!("      🦴 Bones: {}", mesh.num_bones());
            for bone_name in mesh.bone_names().iter().take(3) {
                println!("        - {}", bone_name);
            }
        }
    }

    // Analyze materials with enhanced properties
    for (i, material) in scene.materials().enumerate() {
        println!("    Material {}: {}", i, material.name());

        // Use material key constants
        if let Some(diffuse) = material.get_color_property(material_keys::COLOR_DIFFUSE) {
            println!(
                "      🎨 Diffuse: ({:.2}, {:.2}, {:.2})",
                diffuse.x, diffuse.y, diffuse.z
            );
        }

        if let Some(shininess) = material.get_float_property(material_keys::SHININESS) {
            println!("      ✨ Shininess: {:.2}", shininess);
        }
    }

    // Show metadata if available
    if let Ok(_metadata) = scene.metadata() {
        println!("    📋 Scene Metadata:");
        println!("      Metadata available (use specific keys to access values)");
    }

    Ok(())
}

fn demonstrate_russimp_compatibility() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 russimp Compatibility:");

    let obj_data = create_simple_test_model();

    // russimp-style static methods
    let scene1 = Scene::from_memory(obj_data.as_bytes(), Some("obj"))?;
    println!("  ✅ Scene::from_memory() - {} meshes", scene1.num_meshes());

    let scene2 = Scene::from_memory_with_flags(
        obj_data.as_bytes(),
        Some("obj"),
        PostProcessSteps::TRIANGULATE,
    )?;
    println!(
        "  ✅ Scene::from_memory_with_flags() - {} meshes",
        scene2.num_meshes()
    );

    let props = PropertyStore::new();
    let scene3 = Scene::from_memory_with_props(
        obj_data.as_bytes(),
        Some("obj"),
        PostProcessSteps::TRIANGULATE,
        &props,
    )?;
    println!(
        "  ✅ Scene::from_memory_with_props() - {} meshes",
        scene3.num_meshes()
    );

    println!("  🎯 Perfect russimp compatibility with zero migration cost!");

    Ok(())
}

fn create_advanced_test_model() -> String {
    r#"
# Advanced test model with materials
mtllib test.mtl
usemtl Material1

# Vertices
v -1.0 -1.0  1.0
v  1.0 -1.0  1.0
v  1.0  1.0  1.0
v -1.0  1.0  1.0
v -1.0 -1.0 -1.0
v  1.0 -1.0 -1.0
v  1.0  1.0 -1.0
v -1.0  1.0 -1.0

# Texture coordinates
vt 0.0 0.0
vt 1.0 0.0
vt 1.0 1.0
vt 0.0 1.0

# Normals
vn 0.0 0.0 1.0
vn 0.0 0.0 -1.0
vn 0.0 1.0 0.0
vn 0.0 -1.0 0.0
vn 1.0 0.0 0.0
vn -1.0 0.0 0.0

# Faces with texture coordinates and normals
f 1/1/1 2/2/1 3/3/1 4/4/1
f 5/1/2 8/4/2 7/3/2 6/2/2
f 1/1/4 5/2/4 6/3/4 2/4/4
f 2/1/5 6/2/5 7/3/5 3/4/5
f 3/1/3 7/2/3 8/3/3 4/4/3
f 5/1/6 1/2/6 4/3/6 8/4/6
"#
    .to_string()
}

fn create_simple_test_model() -> String {
    r#"
# Simple cube
v -1.0 -1.0  1.0
v  1.0 -1.0  1.0
v  1.0  1.0  1.0
v -1.0  1.0  1.0

f 1 2 3 4
"#
    .to_string()
}
