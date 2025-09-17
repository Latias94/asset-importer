//! File import tests using real model files
//! These tests verify file-based import functionality

use asset_importer::{Importer, postprocess::PostProcessSteps};
use std::path::Path;

#[test]
fn test_file_import_obj_box() {
    // Test importing from file (based on real assimp test model)
    let model_path = Path::new("tests/models/box.obj");

    // Skip test if model file doesn't exist
    if !model_path.exists() {
        println!(
            "Skipping file import test - model file not found: {:?}",
            model_path
        );
        return;
    }

    let importer = Importer::new();

    let result = importer
        .read_file(model_path)
        .with_post_process(
            PostProcessSteps::TRIANGULATE | PostProcessSteps::JOIN_IDENTICAL_VERTICES,
        )
        .import_file(model_path);

    match result {
        Ok(scene) => {
            assert!(
                scene.num_meshes() > 0,
                "Scene should have at least one mesh"
            );

            let mesh = scene
                .meshes()
                .next()
                .expect("Should have at least one mesh");
            assert!(mesh.num_vertices() > 0, "Mesh should have vertices");
            assert!(mesh.num_faces() > 0, "Mesh should have faces");

            println!(
                "File OBJ Box: {} vertices, {} faces",
                mesh.num_vertices(),
                mesh.num_faces()
            );

            // A cube should have 8 vertices
            assert_eq!(mesh.num_vertices(), 8, "Box should have 8 vertices");
            // After triangulation, 6 quad faces become 12 triangular faces
            assert_eq!(
                mesh.num_faces(),
                12,
                "Triangulated box should have 12 faces"
            );

            // Test scene structure
            assert!(scene.root_node().is_some(), "Scene should have a root node");

            if let Some(root) = scene.root_node() {
                println!("Root node name: {:?}", root.name());
                // root.num_children() is u32, so it's always >= 0
                // Just verify we can call the method without panicking
                let _children_count = root.num_children();
            }
        }
        Err(e) => {
            panic!("Failed to import OBJ box from file: {:?}", e);
        }
    }
}

#[test]
fn test_file_import_nonexistent() {
    // Test error handling for non-existent files
    let importer = Importer::new();

    let result = importer
        .read_file("nonexistent_file.obj")
        .import_file("nonexistent_file.obj");

    assert!(result.is_err(), "Import of non-existent file should fail");

    if let Err(e) = result {
        println!("Expected error for non-existent file: {:?}", e);
    }
}

#[test]
fn test_import_builder_chaining() {
    // Test ImportBuilder method chaining
    let model_path = Path::new("tests/models/box.obj");

    // Skip test if model file doesn't exist
    if !model_path.exists() {
        println!("Skipping builder chaining test - model file not found");
        return;
    }

    let importer = Importer::new();

    // Test method chaining with multiple configurations
    let result = importer
        .read_file(model_path)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .with_post_process(PostProcessSteps::JOIN_IDENTICAL_VERTICES) // Should override previous
        .import_file(model_path);

    assert!(result.is_ok(), "Chained import should succeed");
}

#[test]
fn test_multiple_imports_same_importer() {
    // Test multiple imports using the same importer instance
    // (based on ImporterTest::testMultipleReads)
    let model_path = Path::new("tests/models/box.obj");

    // Skip test if model file doesn't exist
    if !model_path.exists() {
        println!("Skipping multiple imports test - model file not found");
        return;
    }

    let importer = Importer::new();

    // First import
    let result1 = importer
        .read_file(model_path)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .import_file(model_path);

    assert!(result1.is_ok(), "First import should succeed");

    // Second import with different settings
    let result2 = importer
        .read_file(model_path)
        .with_post_process(PostProcessSteps::GEN_NORMALS)
        .import_file(model_path);

    assert!(result2.is_ok(), "Second import should succeed");

    // Verify both imports worked
    if let (Ok(scene1), Ok(scene2)) = (result1, result2) {
        assert_eq!(
            scene1.num_meshes(),
            scene2.num_meshes(),
            "Both scenes should have same mesh count"
        );

        let mesh1 = scene1.meshes().next().unwrap();
        let mesh2 = scene2.meshes().next().unwrap();

        assert_eq!(
            mesh1.num_vertices(),
            mesh2.num_vertices(),
            "Both meshes should have same vertex count"
        );

        // Second mesh should have normals due to GENERATE_NORMALS
        assert!(mesh2.normals().is_some(), "Second mesh should have normals");
    }
}

#[test]
fn test_scene_iteration() {
    // Test scene data structure iteration
    let model_path = Path::new("tests/models/box.obj");

    // Skip test if model file doesn't exist
    if !model_path.exists() {
        println!("Skipping scene iteration test - model file not found");
        return;
    }

    let importer = Importer::new();

    let result = importer
        .read_file(model_path)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .import_file(model_path);

    if let Ok(scene) = result {
        // Test mesh iteration
        let mut mesh_count = 0;
        for mesh in scene.meshes() {
            mesh_count += 1;
            assert!(mesh.num_vertices() > 0, "Each mesh should have vertices");
            assert!(mesh.num_faces() > 0, "Each mesh should have faces");

            println!(
                "Mesh {}: {} vertices, {} faces",
                mesh_count,
                mesh.num_vertices(),
                mesh.num_faces()
            );
        }

        assert_eq!(
            mesh_count,
            scene.num_meshes(),
            "Iterator should visit all meshes"
        );

        // Test material iteration
        let mut material_count = 0;
        for material in scene.materials() {
            material_count += 1;
            println!("Material {}: {:?}", material_count, material.name());
        }

        assert_eq!(
            material_count,
            scene.num_materials(),
            "Iterator should visit all materials"
        );

        // Test node hierarchy
        if let Some(root) = scene.root_node() {
            println!(
                "Root node: {:?}, children: {}",
                root.name(),
                root.num_children()
            );

            // Test node iteration (if there are children)
            for (i, child) in root.children().enumerate() {
                println!("Child {}: {:?}", i, child.name());
            }
        }
    }
}

#[test]
fn test_mesh_data_access() {
    // Test accessing mesh vertex data
    let model_path = Path::new("tests/models/box.obj");

    // Skip test if model file doesn't exist
    if !model_path.exists() {
        println!("Skipping mesh data access test - model file not found");
        return;
    }

    let importer = Importer::new();

    let result = importer
        .read_file(model_path)
        .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::GEN_NORMALS)
        .import_file(model_path);

    if let Ok(scene) = result {
        if let Some(mesh) = scene.meshes().next() {
            // Test vertex access
            assert!(
                !mesh.vertices().is_empty(),
                "Mesh should have vertex positions"
            );

            if let Some(normals) = mesh.normals() {
                println!("Mesh has normals: {} normals", normals.len());
            }

            if let Some(tex_coords) = mesh.texture_coords(0) {
                println!("Mesh has texture coordinates: {} coords", tex_coords.len());
            }

            // Test face access
            for (i, face) in mesh.faces().enumerate() {
                if i < 3 {
                    // Only print first few faces
                    println!("Face {}: {} indices", i, face.num_indices());
                }
            }

            // Verify face structure
            for face in mesh.faces() {
                assert!(
                    face.num_indices() >= 3,
                    "Each face should have at least 3 indices (triangle)"
                );
                assert!(
                    face.num_indices() <= 3,
                    "After triangulation, faces should have exactly 3 indices"
                );
            }
        }
    }
}
