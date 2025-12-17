//! Demonstrate mint math library integration for interoperability
//!
//! This example shows how to use the mint integration feature to convert
//! between asset-importer types and mint types for interoperability with
//! other math libraries that support mint.

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

use asset_importer::{Matrix4x4, Quaternion, Vector2D, Vector3D};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();

    run_mint_demo()?;
    Ok(())
}

fn run_mint_demo() -> Result<(), Box<dyn Error>> {
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
            let mint_vertex: mint::Vector3<f32> = vertex.into();
            println!("As mint::Vector3: {:?}", mint_vertex);

            // Convert back
            let back_to_asset: Vector3D = mint_vertex.into();
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

                let mint_tex: mint::Vector2<f32> = tex_coord_2d.into();
                println!("As mint::Vector2: {:?}", mint_tex);

                let back_to_asset: Vector2D = mint_tex.into();
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
        let mint_matrix: mint::ColumnMatrix4<f32> = transform.into();
        println!("\nAs mint::ColumnMatrix4:");
        print_mint_matrix(&mint_matrix);

        // Convert back
        let back_to_asset: Matrix4x4 = mint_matrix.into();
        println!("\nBack to Matrix4x4:");
        print_matrix4x4(&back_to_asset);

        // Verify they're the same (element-wise)
        let a = transform.to_cols_array_2d();
        let b = back_to_asset.to_cols_array_2d();
        let mut max_abs = 0.0f32;
        for c in 0..4 {
            for r in 0..4 {
                max_abs = max_abs.max((a[c][r] - b[c][r]).abs());
            }
        }
        println!("Max element-wise abs diff: {:.10}", max_abs);
        assert!(max_abs < f32::EPSILON, "Conversion should be lossless");
    }

    println!("\n=== Quaternion Conversion ===");

    // Create a test quaternion (y-axis, 45 degrees)
    let quat = Quaternion::from_xyzw(0.0, 0.38268343, 0.0, 0.9238795);
    println!("Original quaternion: {:?}", quat);

    // Convert to mint
    let mint_quat: mint::Quaternion<f32> = quat.into();
    println!(
        "As mint::Quaternion: s={:.6}, v=({:.6}, {:.6}, {:.6})",
        mint_quat.s, mint_quat.v.x, mint_quat.v.y, mint_quat.v.z
    );

    // Convert back
    let back_to_asset: Quaternion = mint_quat.into();
    println!("Back to Quaternion: {:?}", back_to_asset);

    // Verify they're the same (quaternions can be equivalent with opposite signs)
    let min_diff = quat_equiv_max_abs_component_diff(quat, back_to_asset);
    println!(
        "Max component abs diff (with sign ambiguity): {:.10}",
        min_diff
    );
    assert!(min_diff < f32::EPSILON, "Conversion should be lossless");

    println!("\n=== Practical Usage Example ===");
    println!("// Convert asset-importer vertex to mint for use with other libraries");
    println!("let vertex: Vector3D = mesh.vertices()[0];");
    println!("let mint_vertex: mint::Vector3<f32> = vertex.into();");
    println!("// Use with cgmath, nalgebra, or other mint-compatible libraries");
    println!("// let cgmath_point = cgmath::Point3::from(mint_vertex);");
    println!();
    println!("// Convert from mint back to asset-importer");
    println!("let mint_vec = mint::Vector3 {{ x: 1.0, y: 2.0, z: 3.0 }};");
    println!("let asset_vec: Vector3D = mint_vec.into();");

    println!("\n✓ All mint conversions completed successfully!");
    println!("The mint integration allows seamless interoperability with other math libraries.");

    common::shutdown_logging();

    Ok(())
}

fn print_matrix4x4(matrix: &Matrix4x4) {
    let cols = matrix.to_cols_array_2d();
    for row in 0..4 {
        println!(
            "  [{:8.3} {:8.3} {:8.3} {:8.3}]",
            cols[0][row], cols[1][row], cols[2][row], cols[3][row]
        );
    }
}

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

fn quat_equiv_max_abs_component_diff(a: Quaternion, b: Quaternion) -> f32 {
    fn max_abs_component_diff(a: Quaternion, b: Quaternion) -> f32 {
        let dx = (a.x - b.x).abs();
        let dy = (a.y - b.y).abs();
        let dz = (a.z - b.z).abs();
        let dw = (a.w - b.w).abs();
        dx.max(dy).max(dz).max(dw)
    }
    let neg_b = Quaternion::from_xyzw(-b.x, -b.y, -b.z, -b.w);
    max_abs_component_diff(a, b).min(max_abs_component_diff(a, neg_b))
}
