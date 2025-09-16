/*!
 * Example demonstrating model format conversion using asset-importer.
 *
 * This shows how to load a model in one format and export it to another,
 * similar to Assimp's command-line tools but with Rust.
 */

#[cfg(feature = "export")]
use asset_importer::{postprocess::PostProcessSteps, ExportBuilder, Importer};

#[cfg(not(feature = "export"))]
// Note: Importer import removed as it's not used in this example
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "export"))]
    {
        eprintln!("Error: Export functionality not enabled.");
        eprintln!("Please run with: cargo run --example convert_model --features export -- <input> <output>");
        std::process::exit(1);
    }

    #[cfg(feature = "export")]
    convert_main()
}

#[cfg(feature = "export")]
fn convert_main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Usage: {} <input_file> <output_file>", args[0]);
        println!("Example: {} model.fbx model.obj", args[0]);
        println!("         {} scene.dae scene.gltf", args[0]);
        return Ok(());
    }

    let input_file = &args[1];
    let output_file = &args[2];

    // Check if input file exists
    if !Path::new(input_file).exists() {
        eprintln!("Error: Input file '{}' does not exist", input_file);
        return Ok(());
    }

    println!("Converting: {} -> {}", input_file, output_file);

    // Load the input model
    println!("Loading input model...");
    let importer = Importer::new();
    let scene = importer
        .read_file(input_file)
        .with_post_process(
            PostProcessSteps::TRIANGULATE
                | PostProcessSteps::GEN_NORMALS
                | PostProcessSteps::FLIP_UVS
                | PostProcessSteps::JOIN_IDENTICAL_VERTICES
                | PostProcessSteps::IMPROVE_CACHE_LOCALITY
                | PostProcessSteps::REMOVE_REDUNDANT_MATERIALS
                | PostProcessSteps::FIX_INFACING_NORMALS,
        )
        .import_file(input_file)?;

    println!("✓ Loaded successfully!");
    println!("  Meshes: {}", scene.num_meshes());
    println!("  Materials: {}", scene.num_materials());
    println!("  Vertices: {}", count_total_vertices(&scene));
    println!("  Faces: {}", count_total_faces(&scene));

    // Determine output format from file extension
    let output_ext = Path::new(output_file)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    println!("\nExporting to {} format...", output_ext.to_uppercase());

    // Export the model
    let export_builder = ExportBuilder::new(output_ext);
    export_builder.export_to_file(&scene, output_file)?;

    println!("✓ Export completed successfully!");

    // Verify the output file was created
    if Path::new(output_file).exists() {
        let metadata = std::fs::metadata(output_file)?;
        println!("Output file size: {} bytes", metadata.len());
    }

    // Show some conversion statistics
    println!("\n=== Conversion Summary ===");
    println!("Input:  {}", input_file);
    println!("Output: {}", output_file);
    println!(
        "Format: {} -> {}",
        get_file_extension(input_file).to_uppercase(),
        output_ext.to_uppercase()
    );

    // Suggest optimization options
    println!("\n=== Optimization Tips ===");
    println!("For smaller files, try:");
    println!("  - Remove unused materials");
    println!("  - Merge identical vertices");
    println!("  - Use binary formats (like .glb instead of .gltf)");

    println!("\nFor better quality, try:");
    println!("  - Generate smooth normals");
    println!("  - Calculate tangent space");
    println!("  - Preserve original vertex order");

    Ok(())
}

// Utility functions for demonstration purposes
#[allow(dead_code)]
fn count_total_vertices(scene: &asset_importer::Scene) -> u32 {
    scene.meshes().map(|mesh| mesh.num_vertices() as u32).sum()
}

#[allow(dead_code)]
fn count_total_faces(scene: &asset_importer::Scene) -> u32 {
    scene.meshes().map(|mesh| mesh.num_faces() as u32).sum()
}

#[allow(dead_code)]
fn get_file_extension(filename: &str) -> &str {
    Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
}
