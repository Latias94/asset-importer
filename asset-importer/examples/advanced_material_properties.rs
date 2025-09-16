//! Advanced material properties example
//!
//! This example demonstrates the enhanced material system with complete
//! property access and the new PropertyStore API.

use asset_importer::{import_properties, postprocess::PostProcessSteps, Importer, PropertyStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the model path from command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <model_file>", args[0]);
        std::process::exit(1);
    }
    let model_path = &args[1];

    // Create a property store with advanced import settings
    let mut props = PropertyStore::new();
    props
        .set_float(import_properties::MAX_SMOOTHING_ANGLE, 80.0)
        .set_bool(import_properties::FBX_READ_ALL_GEOMETRY_LAYERS, true)
        .set_bool(import_properties::FBX_PRESERVE_PIVOTS, false)
        .set_int(import_properties::LIMIT_BONE_WEIGHTS_MAX, 4)
        .set_float(import_properties::GLOBAL_SCALE_FACTOR, 1.0);

    println!("🔧 Import properties configured:");
    for (key, value) in props.properties() {
        println!("  {} = {:?}", key, value);
    }

    // Import the scene with advanced settings
    let importer = Importer::new();
    let scene = importer
        .read_file(model_path)
        .with_property_store(props)
        .with_post_process(
            PostProcessSteps::TRIANGULATE
                | PostProcessSteps::GEN_SMOOTH_NORMALS
                | PostProcessSteps::JOIN_IDENTICAL_VERTICES
                | PostProcessSteps::IMPROVE_CACHE_LOCALITY,
        )
        .import_file(model_path)?;

    println!("\n📦 Scene loaded successfully!");
    println!("  Meshes: {}", scene.num_meshes());
    println!("  Materials: {}", scene.num_materials());
    println!("  Animations: {}", scene.num_animations());

    // Analyze materials with enhanced property access
    println!("\n🎨 Material Analysis:");
    for (i, material) in scene.materials().enumerate() {
        println!("\n  Material {}: {}", i, material.name());

        // Basic color properties
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
        if let Some(ambient) = material.ambient_color() {
            println!(
                "    Ambient: ({:.2}, {:.2}, {:.2})",
                ambient.x, ambient.y, ambient.z
            );
        }
        if let Some(emissive) = material.emissive_color() {
            println!(
                "    Emissive: ({:.2}, {:.2}, {:.2})",
                emissive.x, emissive.y, emissive.z
            );
        }

        // Advanced properties
        if let Some(shininess) = material.shininess() {
            println!("    Shininess: {:.2}", shininess);
        }
        if let Some(shininess_strength) = material.shininess_strength() {
            println!("    Shininess Strength: {:.2}", shininess_strength);
        }
        if let Some(opacity) = material.opacity() {
            println!("    Opacity: {:.2}", opacity);
        }
        if let Some(transparency) = material.transparency_factor() {
            println!("    Transparency: {:.2}", transparency);
        }
        if let Some(refraction) = material.refraction_index() {
            println!("    Refraction Index: {:.2}", refraction);
        }
        if let Some(reflectivity) = material.reflectivity() {
            println!("    Reflectivity: {:.2}", reflectivity);
        }

        // Material flags
        if material.is_two_sided() {
            println!("    ✓ Two-sided material");
        }

        // Texture analysis
        let texture_types = [
            (asset_importer::TextureType::Diffuse, "Diffuse"),
            (asset_importer::TextureType::Specular, "Specular"),
            (asset_importer::TextureType::Ambient, "Ambient"),
            (asset_importer::TextureType::Emissive, "Emissive"),
            (asset_importer::TextureType::Height, "Height/Normal"),
            (asset_importer::TextureType::Normals, "Normal"),
            (asset_importer::TextureType::Shininess, "Shininess"),
            (asset_importer::TextureType::Opacity, "Opacity"),
            (asset_importer::TextureType::Displacement, "Displacement"),
            (asset_importer::TextureType::Lightmap, "Lightmap"),
            (asset_importer::TextureType::Reflection, "Reflection"),
        ];

        for (tex_type, name) in texture_types.iter() {
            let count = material.texture_count(*tex_type);
            if count > 0 {
                println!("    {} textures: {}", name, count);
                for j in 0..count {
                    if let Some(texture_info) = material.texture(*tex_type, j) {
                        println!("      [{}] {}", j, texture_info.path);
                        if texture_info.blend_factor != 1.0 {
                            println!("          Blend: {:.2}", texture_info.blend_factor);
                        }
                        if texture_info.uv_index != 0 {
                            println!("          UV Index: {}", texture_info.uv_index);
                        }
                    }
                }
            }
        }

        // Custom property access using material keys
        if let Some(custom_prop) = material.get_string_property("?mat.name") {
            println!("    Custom Name Property: {}", custom_prop);
        }
    }

    // Metadata analysis
    if let Ok(metadata) = scene.metadata() {
        println!("\n📋 Scene Metadata:");
        for (key, entry) in metadata.iter() {
            match entry {
                asset_importer::MetadataEntry::String(s) => {
                    println!("  {}: \"{}\"", key, s);
                }
                asset_importer::MetadataEntry::Int32(i) => {
                    println!("  {}: {}", key, i);
                }
                asset_importer::MetadataEntry::Float(f) => {
                    println!("  {}: {:.2}", key, f);
                }
                asset_importer::MetadataEntry::Bool(b) => {
                    println!("  {}: {}", key, b);
                }
                asset_importer::MetadataEntry::Vector3D(v) => {
                    println!("  {}: ({:.2}, {:.2}, {:.2})", key, v.x, v.y, v.z);
                }
                _ => {
                    println!("  {}: {:?}", key, entry.metadata_type());
                }
            }
        }
    }

    println!("\n✅ Material analysis complete!");
    Ok(())
}
