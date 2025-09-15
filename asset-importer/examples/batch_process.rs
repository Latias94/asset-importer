/*!
 * Example demonstrating batch processing of multiple 3D models.
 *
 * This shows how to process multiple files efficiently and gather
 * statistics about model collections.
 */

use asset_importer::{postprocess::PostProcessSteps, Importer};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct ModelStats {
    total_files: usize,
    successful_loads: usize,
    failed_loads: usize,
    total_meshes: u32,
    total_vertices: u32,
    total_faces: u32,
    total_materials: u32,
    formats: std::collections::HashMap<String, usize>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <directory_or_pattern>", args[0]);
        println!("Examples:");
        println!("  {} examples/models/", args[0]);
        println!("  {} *.obj", args[0]);
        println!("  {} models/*.fbx", args[0]);
        return Ok(());
    }

    let pattern = &args[1];
    let files = find_model_files(pattern)?;

    if files.is_empty() {
        println!("No model files found matching: {}", pattern);
        return Ok(());
    }

    println!("Found {} model files to process", files.len());
    println!("Processing...\n");

    let mut stats = ModelStats::default();
    let importer = Importer::new();

    for (i, file_path) in files.iter().enumerate() {
        print!(
            "[{}/{}] Processing: {} ... ",
            i + 1,
            files.len(),
            file_path.display()
        );

        stats.total_files += 1;

        // Track file format
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            *stats.formats.entry(ext.to_lowercase()).or_insert(0) += 1;
        }

        match process_single_file(&importer, file_path) {
            Ok(file_stats) => {
                stats.successful_loads += 1;
                stats.total_meshes += file_stats.meshes;
                stats.total_vertices += file_stats.vertices;
                stats.total_faces += file_stats.faces;
                stats.total_materials += file_stats.materials;

                println!(
                    "✓ ({} meshes, {} vertices)",
                    file_stats.meshes, file_stats.vertices
                );
            }
            Err(e) => {
                stats.failed_loads += 1;
                println!("✗ Error: {}", e);
            }
        }
    }

    // Print summary statistics
    print_summary(&stats);

    Ok(())
}

#[derive(Default)]
struct FileStats {
    meshes: u32,
    vertices: u32,
    faces: u32,
    materials: u32,
}

fn process_single_file(
    importer: &Importer,
    file_path: &Path,
) -> Result<FileStats, Box<dyn std::error::Error>> {
    let scene = importer
        .read_file(file_path.to_str().unwrap())
        .with_post_process(
            PostProcessSteps::TRIANGULATE
                | PostProcessSteps::GEN_NORMALS
                | PostProcessSteps::JOIN_IDENTICAL_VERTICES,
        )
        .import_file(file_path.to_str().unwrap())?;

    let mut stats = FileStats::default();
    stats.meshes = scene.num_meshes() as u32;
    stats.materials = scene.num_materials() as u32;

    for mesh in scene.meshes() {
        stats.vertices += mesh.num_vertices() as u32;
        stats.faces += mesh.num_faces() as u32;
    }

    Ok(stats)
}

fn find_model_files(pattern: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();

    let path = Path::new(pattern);

    if path.is_dir() {
        // Process directory
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() && is_model_file(&file_path) {
                files.push(file_path);
            }
        }
    } else if path.exists() {
        // Single file
        files.push(path.to_path_buf());
    } else {
        // Try glob pattern (simplified)
        let parent = path.parent().unwrap_or(Path::new("."));
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("*");

        if parent.is_dir() {
            for entry in fs::read_dir(parent)? {
                let entry = entry?;
                let file_path = entry.path();

                if file_path.is_file()
                    && is_model_file(&file_path)
                    && matches_pattern(
                        file_path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                        filename,
                    )
                {
                    files.push(file_path);
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

fn is_model_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            "obj"
                | "fbx"
                | "dae"
                | "3ds"
                | "blend"
                | "ply"
                | "stl"
                | "gltf"
                | "glb"
                | "x3d"
                | "md2"
                | "md3"
                | "md5mesh"
                | "ms3d"
                | "x"
                | "ac"
                | "ase"
                | "lwo"
                | "lws"
                | "off"
        )
    } else {
        false
    }
}

fn matches_pattern(filename: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if pattern.contains('*') {
        // Simple wildcard matching
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            filename.starts_with(parts[0]) && filename.ends_with(parts[1])
        } else {
            filename.contains(&pattern.replace('*', ""))
        }
    } else {
        filename == pattern
    }
}

fn print_summary(stats: &ModelStats) {
    println!("\n=== Batch Processing Summary ===");
    println!("Total files processed: {}", stats.total_files);
    println!(
        "Successful loads: {} ({:.1}%)",
        stats.successful_loads,
        100.0 * stats.successful_loads as f64 / stats.total_files as f64
    );
    println!("Failed loads: {}", stats.failed_loads);

    if stats.successful_loads > 0 {
        println!("\n=== Aggregate Statistics ===");
        println!("Total meshes: {}", stats.total_meshes);
        println!("Total vertices: {}", stats.total_vertices);
        println!("Total faces: {}", stats.total_faces);
        println!("Total materials: {}", stats.total_materials);

        println!("Average per file:");
        println!(
            "  Meshes: {:.1}",
            stats.total_meshes as f64 / stats.successful_loads as f64
        );
        println!(
            "  Vertices: {:.1}",
            stats.total_vertices as f64 / stats.successful_loads as f64
        );
        println!(
            "  Faces: {:.1}",
            stats.total_faces as f64 / stats.successful_loads as f64
        );

        println!("\n=== File Formats ===");
        let mut format_vec: Vec<_> = stats.formats.iter().collect();
        format_vec.sort_by(|a, b| b.1.cmp(a.1));

        for (format, count) in format_vec {
            println!("  .{}: {} files", format, count);
        }
    }
}
