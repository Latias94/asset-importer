/*!
 * Importer Discovery Demo
 *
 * This example demonstrates how the importer_desc and importer modules work together,
 * showcasing smart file format detection, importer capability queries, and adaptive import strategies.
 */

use asset_importer::{
    get_all_importer_descs, get_importer_desc, import_properties, postprocess::PostProcessSteps,
    Importer, ImporterFlags, PropertyStore, Scene,
};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Importer Discovery and Smart Import Demo\n");

    // 1. Demonstrate all available importers
    demonstrate_available_importers()?;

    // 2. Smart file format detection
    demonstrate_smart_format_detection()?;

    // 3. Adaptive import based on importer capabilities
    demonstrate_adaptive_import_strategy()?;

    // 4. Format-specific optimized import
    demonstrate_format_specific_optimization()?;

    Ok(())
}

/// Demonstrates all available importers and their capabilities
fn demonstrate_available_importers() -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 Available Importers:");

    let importers = get_all_importer_descs();
    println!("   Found {} importers\n", importers.len());

    for (i, desc) in importers.iter().enumerate().take(10) {
        // Only show the first 10
        println!("{}. {} ({})", i + 1, desc.name, desc.author);
        println!("   📁 Extensions: {}", desc.file_extensions.join(", "));

        // Analyze importer capabilities
        let mut capabilities = Vec::new();
        if desc.flags.contains(ImporterFlags::SUPPORT_TEXT_FLAVOUR) {
            capabilities.push("📄 Text");
        }
        if desc.flags.contains(ImporterFlags::SUPPORT_BINARY_FLAVOUR) {
            capabilities.push("🔢 Binary");
        }
        if desc
            .flags
            .contains(ImporterFlags::SUPPORT_COMPRESSED_FLAVOUR)
        {
            capabilities.push("🗜️ Compressed");
        }
        if desc.flags.contains(ImporterFlags::LIMITED_SUPPORT) {
            capabilities.push("⚠️ Limited");
        }
        if desc.flags.contains(ImporterFlags::EXPERIMENTAL) {
            capabilities.push("🧪 Experimental");
        }

        if !capabilities.is_empty() {
            println!("   🎯 Capabilities: {}", capabilities.join(", "));
        }

        if !desc.comments.is_empty() {
            println!("   💬 Notes: {}", desc.comments);
        }
        println!();
    }

    Ok(())
}

/// Smart file format detection
fn demonstrate_smart_format_detection() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 Smart Format Detection:");

    let test_files = [
        "model.obj",
        "scene.fbx",
        "mesh.dae",
        "animation.bvh",
        "texture.gltf",
        "unknown.xyz",
    ];

    for file_path in &test_files {
        let extension = Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match get_importer_desc(extension) {
            Some(desc) => {
                println!("✅ {}: Supported by {}", file_path, desc.name);

                // Provide suggestions based on importer capabilities
                if desc.flags.contains(ImporterFlags::EXPERIMENTAL) {
                    println!("   ⚠️  Warning: This importer is experimental");
                }
                if desc.flags.contains(ImporterFlags::LIMITED_SUPPORT) {
                    println!("   ℹ️  Note: Limited format support");
                }

                // Recommended post-processing steps
                let recommended_steps = recommend_post_processing(&desc);
                if !recommended_steps.is_empty() {
                    println!("   🔧 Recommended post-processing: {}", recommended_steps);
                }
            }
            None => {
                println!(
                    "❌ {}: No importer found for '.{}' extension",
                    file_path, extension
                );
            }
        }
    }
    println!();

    Ok(())
}

/// Adaptive import strategy based on importer capabilities
fn demonstrate_adaptive_import_strategy() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Adaptive Import Strategy:");

    // Simulate different types of files
    let scenarios = [
        ("model.obj", "Simple geometry file"),
        ("scene.fbx", "Complex scene with animations"),
        ("mesh.dae", "COLLADA scene file"),
        ("simple.ply", "Point cloud data"),
    ];

    for (file_path, description) in &scenarios {
        let extension = Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        if let Some(desc) = get_importer_desc(extension) {
            println!("📁 {}: {}", file_path, description);
            println!("   🔧 Importer: {}", desc.name);

            // Create an adaptive import configuration
            let (post_process, properties) = create_adaptive_import_config(&desc, file_path);

            println!(
                "   ⚙️  Post-processing: {:?}",
                format_post_process_steps(post_process)
            );
            if !properties.is_empty() {
                println!("   🔧 Properties: {} custom settings", properties.len());
            }

            // Actual import can be executed here (if the file exists)
            // let scene = execute_adaptive_import(file_path, post_process, properties)?;

            println!();
        }
    }

    Ok(())
}

/// Format-specific optimized import
fn demonstrate_format_specific_optimization() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Format-Specific Optimization:");

    // Show how to optimize import settings for different formats
    let optimizations = [
        ("obj", "OBJ files: Enable smooth normals, optimize meshes"),
        (
            "fbx",
            "FBX files: Preserve animations, handle embedded textures",
        ),
        (
            "dae",
            "COLLADA files: Fix coordinate system, process materials",
        ),
        (
            "gltf",
            "glTF files: Preserve PBR materials, handle extensions",
        ),
    ];

    for (ext, description) in &optimizations {
        if let Some(desc) = get_importer_desc(ext) {
            println!("🎯 {}: {}", ext.to_uppercase(), description);

            // Create a format-specific optimized configuration
            let optimized_config = create_format_optimized_config(&desc, ext);
            println!("   📋 Optimizations applied: {}", optimized_config);

            // Demonstrate how to use these configurations
            demonstrate_optimized_import_usage(ext, &desc)?;
            println!();
        }
    }

    Ok(())
}

/// Recommend post-processing steps
fn recommend_post_processing(desc: &asset_importer::ImporterDesc) -> String {
    let mut recommendations = Vec::new();

    // Recommend post-processing based on importer features
    if desc.file_extensions.contains(&"obj".to_string()) {
        recommendations.push("Generate smooth normals");
    }
    if desc.file_extensions.contains(&"fbx".to_string()) {
        recommendations.push("Optimize meshes, Calculate tangents");
    }
    if desc.file_extensions.contains(&"dae".to_string()) {
        recommendations.push("Fix winding order, Convert to left-handed");
    }

    recommendations.join(", ")
}

/// Create an adaptive import configuration
fn create_adaptive_import_config(
    desc: &asset_importer::ImporterDesc,
    file_path: &str,
) -> (PostProcessSteps, Vec<(String, String)>) {
    let mut post_process = PostProcessSteps::empty();
    let mut properties = Vec::new();

    // Adjust configuration based on file type and importer capabilities
    if desc.file_extensions.contains(&"obj".to_string()) {
        post_process |= PostProcessSteps::TRIANGULATE | PostProcessSteps::GEN_SMOOTH_NORMALS;
    } else if desc.file_extensions.contains(&"fbx".to_string()) {
        post_process |= PostProcessSteps::TRIANGULATE | PostProcessSteps::CALC_TANGENT_SPACE;
        properties.push((
            import_properties::FBX_PRESERVE_PIVOTS.to_string(),
            "true".to_string(),
        ));
    } else if desc.file_extensions.contains(&"dae".to_string()) {
        post_process |= PostProcessSteps::TRIANGULATE | PostProcessSteps::FLIP_WINDING_ORDER;
        properties.push((
            "AI_CONFIG_IMPORT_COLLADA_IGNORE_UP_DIRECTION".to_string(),
            "true".to_string(),
        ));
    }

    // If it's an experimental importer, use more conservative settings
    if desc.flags.contains(ImporterFlags::EXPERIMENTAL) {
        post_process = PostProcessSteps::TRIANGULATE; // Use only basic processing
    }

    (post_process, properties)
}

/// Create a format-optimized configuration
fn create_format_optimized_config(desc: &asset_importer::ImporterDesc, format: &str) -> String {
    match format {
        "obj" => "Smooth normals + mesh optimization".to_string(),
        "fbx" => "Animation preservation + embedded texture handling".to_string(),
        "dae" => "Coordinate system fix + material processing".to_string(),
        "gltf" => "PBR material preservation + extension support".to_string(),
        _ => "Standard optimization".to_string(),
    }
}

/// Demonstrate the usage of optimized import
fn demonstrate_optimized_import_usage(
    format: &str,
    desc: &asset_importer::ImporterDesc,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("   💻 Usage example:");

    match format {
        "obj" => {
            println!("       let scene = Importer::new()");
            println!("           .read_file(\"model.obj\")");
            println!("           .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::GEN_SMOOTH_NORMALS)");
            println!("           .import_file(\"model.obj\")?;");
        }
        "fbx" => {
            println!("       let mut props = PropertyStore::new();");
            println!("       props.set_bool(import_properties::FBX_PRESERVE_PIVOTS, true);");
            println!("       let scene = Importer::new()");
            println!("           .read_file(\"scene.fbx\")");
            println!("           .with_property_store_ref(&props)");
            println!("           .with_post_process(PostProcessSteps::CALC_TANGENT_SPACE)");
            println!("           .import_file(\"scene.fbx\")?;");
        }
        _ => {
            println!(
                "       let scene = Importer::new().import_file(\"file.{}\")?;",
                format
            );
        }
    }

    Ok(())
}

/// Format post-processing steps into a readable string
fn format_post_process_steps(steps: PostProcessSteps) -> String {
    let mut step_names = Vec::new();

    if steps.contains(PostProcessSteps::TRIANGULATE) {
        step_names.push("Triangulate");
    }
    if steps.contains(PostProcessSteps::GEN_SMOOTH_NORMALS) {
        step_names.push("Smooth Normals");
    }
    if steps.contains(PostProcessSteps::CALC_TANGENT_SPACE) {
        step_names.push("Tangent Space");
    }
    if steps.contains(PostProcessSteps::FLIP_WINDING_ORDER) {
        step_names.push("Flip Winding");
    }

    if step_names.is_empty() {
        "None".to_string()
    } else {
        step_names.join(" + ")
    }
}
