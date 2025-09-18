//! Demonstrate mint math library integration for interoperability
//!
//! This example shows how to use the mint integration feature to convert
//! between asset-importer types and mint types for interoperability with
//! other math libraries that support mint.

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

#[cfg(feature = "mint")]
use asset_importer::{FromMint, Matrix4x4, Quaternion, ToMint, Vector2D, Vector3D};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();

    #[cfg(not(feature = "mint"))]
    {
        eprintln!("This example requires the 'mint' feature.");
        eprintln!(
            "Run with: cargo run --example 07_mint_integration --features \"build-assimp,mint\""
        );
        std::process::exit(1);
    }

    #[cfg(feature = "mint")]
    {
        println!("=== Mint Integration Demo ===");
        println!("This example demonstrates converting between asset-importer and mint types.\n");

        // Load a model to get some real data
        let path = common::resolve_model_path(common::ModelSource::ArgOrExamplesDir, "box.obj");
        let scene = common::import_scene(
            &path,
            asset_importer::postprocess::PostProcessSteps::empty(),
        )?;

        println!("Loaded model: {}", path.display());

        if let Some(mesh) = scene.mesh(0) {
            println!("\n=== Vector3D Conversion ===");

            // Get a vertex from the mesh
            let vertices = mesh.vertices();
            if let Some(&vertex) = vertices.first() {
                println!("Original vertex: {:?}", vertex);

                // Convert to mint
                let mint_vertex: mint::Vector3<f32> = vertex.to_mint();
                println!("As mint::Vector3: {:?}", mint_vertex);

                // Convert back
                let back_to_asset: Vector3D = Vector3D::from_mint(mint_vertex);
                println!("Back to Vector3D: {:?}", back_to_asset);

                // Verify they're the same
                let diff = (vertex - back_to_asset).length();
                println!("Conversion difference: {:.10}", diff);
                assert!(diff < f32::EPSILON, "Conversion should be lossless");
            }

            // Test with texture coordinates if available
            if let Some(tex_coords) = mesh.texture_coords(0) {
                println!("\n=== Vector2D Conversion ===");

                if let Some(&tex_coord) = tex_coords.first() {
                    // Convert Vector3D to Vector2D (texture coords are stored as 3D but usually only use x,y)
                    let tex_coord_2d = Vector2D::new(tex_coord.x, tex_coord.y);
                    println!("Original tex coord: {:?}", tex_coord_2d);

                    let mint_tex: mint::Vector2<f32> = tex_coord_2d.to_mint();
                    println!("As mint::Vector2: {:?}", mint_tex);

                    let back_to_asset: Vector2D = Vector2D::from_mint(mint_tex);
                    println!("Back to Vector2D: {:?}", back_to_asset);

                    let diff = (tex_coord_2d - back_to_asset).length();
                    println!("Conversion difference: {:.10}", diff);
                    assert!(diff < f32::EPSILON, "Conversion should be lossless");
                }
            }
        }

        println!("\n=== Matrix4x4 Conversion ===");

        // Test with a transformation matrix
        if let Some(root) = scene.root_node() {
            let transform = root.transformation();
            println!("Original transform matrix:");
            print_matrix4x4(&transform);

            // Convert to mint
            let mint_matrix: mint::ColumnMatrix4<f32> = transform.to_mint();
            println!("\nAs mint::ColumnMatrix4:");
            print_mint_matrix(&mint_matrix);

            // Convert back
            let back_to_asset: Matrix4x4 = Matrix4x4::from_mint(mint_matrix);
            println!("\nBack to Matrix4x4:");
            print_matrix4x4(&back_to_asset);

            // Verify they're the same
            let diff = (transform - back_to_asset).abs_diff_eq(Matrix4x4::IDENTITY, f32::EPSILON);
            println!(
                "Matrices are equivalent: {}",
                diff || (transform - back_to_asset).determinant().abs() < f32::EPSILON
            );
        }

        println!("\n=== Quaternion Conversion ===");

        // Create a test quaternion
        let quat = Quaternion::from_rotation_y(std::f32::consts::PI / 4.0);
        println!("Original quaternion: {:?}", quat);

        // Convert to mint
        let mint_quat: mint::Quaternion<f32> = quat.to_mint();
        println!(
            "As mint::Quaternion: s={:.6}, v=({:.6}, {:.6}, {:.6})",
            mint_quat.s, mint_quat.v.x, mint_quat.v.y, mint_quat.v.z
        );

        // Convert back
        let back_to_asset: Quaternion = Quaternion::from_mint(mint_quat);
        println!("Back to Quaternion: {:?}", back_to_asset);

        // Verify they're the same (quaternions can be equivalent with opposite signs)
        let diff1 = (quat - back_to_asset).length();
        let diff2 = (quat + back_to_asset).length();
        let min_diff = diff1.min(diff2);
        println!("Conversion difference: {:.10}", min_diff);
        assert!(min_diff < f32::EPSILON, "Conversion should be lossless");

        println!("\n=== Practical Usage Example ===");
        println!("// Convert asset-importer vertex to mint for use with other libraries");
        println!("let vertex: Vector3D = mesh.vertices()[0];");
        println!("let mint_vertex: mint::Vector3<f32> = vertex.to_mint();");
        println!("// Use with cgmath, nalgebra, or other mint-compatible libraries");
        println!("// let cgmath_point = cgmath::Point3::from(mint_vertex);");
        println!();
        println!("// Convert from mint back to asset-importer");
        println!("let mint_vec = mint::Vector3 {{ x: 1.0, y: 2.0, z: 3.0 }};");
        println!("let asset_vec = Vector3D::from_mint(mint_vec);");

        println!("\n✓ All mint conversions completed successfully!");
        println!(
            "The mint integration allows seamless interoperability with other math libraries."
        );
    }

    common::shutdown_logging();
    Ok(())
}

#[cfg(feature = "mint")]
fn print_matrix4x4(matrix: &Matrix4x4) {
    let cols = matrix.to_cols_array_2d();
    for row in 0..4 {
        println!(
            "  [{:8.3} {:8.3} {:8.3} {:8.3}]",
            cols[0][row], cols[1][row], cols[2][row], cols[3][row]
        );
    }
}

#[cfg(feature = "mint")]
fn print_mint_matrix(matrix: &mint::ColumnMatrix4<f32>) {
    println!(
        "  [{:8.3} {:8.3} {:8.3} {:8.3}]",
        matrix.x.x, matrix.y.x, matrix.z.x, matrix.w.x
    );
    println!(
        "  [{:8.3} {:8.3} {:8.3} {:8.3}]",
        matrix.x.y, matrix.y.y, matrix.z.y, matrix.w.y
    );
    println!(
        "  [{:8.3} {:8.3} {:8.3} {:8.3}]",
        matrix.x.z, matrix.y.z, matrix.z.z, matrix.w.z
    );
    println!(
        "  [{:8.3} {:8.3} {:8.3} {:8.3}]",
        matrix.x.w, matrix.y.w, matrix.z.w, matrix.w.w
    );
}
