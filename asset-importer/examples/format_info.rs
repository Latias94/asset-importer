/*!
 * Example showing how to query supported file formats and Assimp version information.
 *
 * This demonstrates the library's capability discovery features.
 */

use asset_importer::{enable_verbose_logging, get_import_extensions, version};

#[cfg(feature = "export")]
use asset_importer::get_export_formats;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Asset Importer Format Information ===\n");

    // Show version information
    println!(
        "Assimp Version: {}.{}.{}",
        version::assimp_version_major(),
        version::assimp_version_minor(),
        version::assimp_version_revision()
    );

    // Enable verbose logging to see more details
    enable_verbose_logging(true);

    // Show supported import formats
    println!("\n=== Supported Import Formats ===");
    let import_extensions = get_import_extensions();

    println!("Total import formats: {}", import_extensions.len());
    println!("Extensions:");

    // Group extensions for better display
    let mut current_line = String::new();
    for (i, ext) in import_extensions.iter().enumerate() {
        if current_line.len() + ext.len() + 2 > 80 {
            println!("  {}", current_line);
            current_line.clear();
        }

        if !current_line.is_empty() {
            current_line.push_str(", ");
        }
        current_line.push_str(ext);

        if i == import_extensions.len() - 1 {
            println!("  {}", current_line);
        }
    }

    // Show supported export formats
    #[cfg(feature = "export")]
    {
        println!("\n=== Supported Export Formats ===");
        let export_formats = get_export_formats();
        println!("Total export formats: {}", export_formats.len());
        println!("Formats:");

        for format in &export_formats {
            println!("  .{} - {}", format.file_extension, format.description);
        }
    }

    #[cfg(not(feature = "export"))]
    {
        println!("\n=== Export Formats ===");
        println!("Export functionality not enabled. Use --features export to enable.");
    }

    // Show some popular format categories
    println!("\n=== Popular Format Categories ===");

    let popular_3d = ["obj", "fbx", "dae", "3ds", "blend", "x3d"];
    let popular_cad = ["step", "iges", "ifc", "ply", "stl"];
    let popular_game = ["md2", "md3", "md5mesh", "ms3d", "x"];
    let popular_modern = ["gltf", "glb", "usd", "usda", "usdc"];

    print_format_category("3D Graphics", &popular_3d, &import_extensions);
    print_format_category("CAD/Engineering", &popular_cad, &import_extensions);
    print_format_category("Game Engines", &popular_game, &import_extensions);
    print_format_category("Modern/PBR", &popular_modern, &import_extensions);

    println!("\n=== Usage Examples ===");
    println!("Import a model:");
    println!("  cargo run --example simple_load --features prebuilt -- model.obj");
    println!("\nConvert between formats:");
    println!("  cargo run --example convert --features prebuilt -- input.fbx output.obj");
    println!("\nBatch process:");
    println!("  cargo run --example batch_process --features prebuilt -- *.dae");

    Ok(())
}

fn print_format_category(category: &str, formats: &[&str], supported: &[String]) {
    println!("\n{}:", category);
    for format in formats {
        let is_supported = supported.iter().any(|s| s.contains(format));
        let status = if is_supported { "✓" } else { "✗" };
        println!("  {} .{}", status, format);
    }
}
