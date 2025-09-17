//! Comprehensive tests based on Assimp's original test suite
//! These tests verify compatibility with the original Assimp library

use asset_importer::{
    Importer, enable_verbose_logging, get_import_extensions,
    io::{FileStream, MemoryFileStream},
    postprocess::PostProcessSteps,
    version,
};

/// Simple OBJ cube model for testing (based on assimp/test/models/OBJ/box.obj)
const SIMPLE_OBJ_CUBE: &str = r#"# Simple cube for testing
# Vertices: 8, Faces: 6

o cube

# Vertex list
v -0.5 -0.5  0.5
v -0.5 -0.5 -0.5
v -0.5  0.5 -0.5
v -0.5  0.5  0.5
v  0.5 -0.5  0.5
v  0.5 -0.5 -0.5
v  0.5  0.5 -0.5
v  0.5  0.5  0.5

# Face list
f 4 3 2 1
f 2 6 5 1
f 3 7 6 2
f 8 7 3 4
f 5 8 4 1
f 6 7 8 5
"#;

/// Simple PLY triangle for testing
const SIMPLE_PLY_TRIANGLE: &str = r#"ply
format ascii 1.0
element vertex 3
property float x
property float y
property float z
element face 1
property list uchar int vertex_indices
end_header
0.0 0.0 0.0
1.0 0.0 0.0
0.5 1.0 0.0
3 0 1 2
"#;

#[test]
fn test_version_functions() {
    // Test version functions (based on AssimpAPITest)
    let major = version::assimp_version_major();
    let minor = version::assimp_version_minor();
    let revision = version::assimp_version_revision();

    println!("Assimp Version: {}.{}.{}", major, minor, revision);

    // Assimp should be at least version 5.0
    assert!(
        major >= 5,
        "Expected Assimp major version >= 5, got {}",
        major
    );

    // Version should be reasonable
    assert!(
        major <= 10,
        "Assimp major version seems too high: {}",
        major
    );
    assert!(
        minor <= 99,
        "Assimp minor version seems too high: {}",
        minor
    );
}

#[test]
fn test_extension_list() {
    // Test extension list functionality (based on ImporterTest::testExtensionCheck)
    let extensions = get_import_extensions();

    assert!(!extensions.is_empty(), "Extension list should not be empty");

    // Should contain common formats (with dots)
    assert!(
        extensions.iter().any(|ext| ext == ".obj"),
        "Should support OBJ format"
    );
    assert!(
        extensions.iter().any(|ext| ext == ".ply"),
        "Should support PLY format"
    );
    assert!(
        extensions.iter().any(|ext| ext == ".3ds"),
        "Should support 3DS format"
    );

    println!("Supported extensions: {:?}", extensions);

    // Count extensions (should be many)
    let ext_count = extensions.len();
    assert!(
        ext_count >= 30,
        "Should support at least 30 formats, found {}",
        ext_count
    );
}

#[test]
fn test_verbose_logging() {
    // Test verbose logging functionality
    enable_verbose_logging(true);
    enable_verbose_logging(false);
    // If we get here without crashing, the test passes
}

#[test]
fn test_property_store_functionality() {
    // Test property store functionality through ImportBuilder
    let importer = Importer::new();
    let _builder = importer
        .read_from_memory(SIMPLE_OBJ_CUBE.as_bytes())
        .with_property_int("AI_CONFIG_PP_RVC_FLAGS", 42)
        .with_property_float("AI_CONFIG_PP_CT_MAX_SMOOTHING_ANGLE", std::f32::consts::PI)
        .with_property_string("AI_CONFIG_IMPORT_FBX_READ_ALL_GEOMETRY_LAYERS", "true")
        .with_property_bool("AI_CONFIG_PP_FD_REMOVE", true);

    println!("✅ Property store functionality test passed");
}

#[test]
fn test_memory_import_obj_cube() {
    // Test importing from memory (based on ImporterTest::testMemoryRead)
    let importer = Importer::new();

    let result = importer
        .read_from_memory(SIMPLE_OBJ_CUBE.as_bytes())
        .with_post_process(
            PostProcessSteps::TRIANGULATE | PostProcessSteps::JOIN_IDENTICAL_VERTICES,
        )
        .import_from_memory(SIMPLE_OBJ_CUBE.as_bytes(), Some("obj"));

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
                "OBJ Cube: {} vertices, {} faces",
                mesh.num_vertices(),
                mesh.num_faces()
            );

            // A cube should have 8 vertices and 6 faces (after triangulation, 12 triangles)
            assert_eq!(mesh.num_vertices(), 8, "Cube should have 8 vertices");
            // After triangulation, 6 quad faces become 12 triangular faces
            assert_eq!(
                mesh.num_faces(),
                12,
                "Triangulated cube should have 12 faces"
            );
        }
        Err(e) => {
            panic!("Failed to import OBJ cube: {:?}", e);
        }
    }
}

#[test]
fn test_memory_import_ply_triangle() {
    // Test importing PLY format
    let importer = Importer::new();

    let result = importer
        .read_from_memory(SIMPLE_PLY_TRIANGLE.as_bytes())
        .with_post_process(PostProcessSteps::VALIDATE_DATA_STRUCTURE)
        .import_from_memory(SIMPLE_PLY_TRIANGLE.as_bytes(), Some("ply"));

    match result {
        Ok(scene) => {
            assert_eq!(scene.num_meshes(), 1, "Scene should have exactly one mesh");

            let mesh = scene.meshes().next().expect("Should have one mesh");
            assert_eq!(mesh.num_vertices(), 3, "Triangle should have 3 vertices");
            assert_eq!(mesh.num_faces(), 1, "Triangle should have 1 face");

            println!(
                "PLY Triangle: {} vertices, {} faces",
                mesh.num_vertices(),
                mesh.num_faces()
            );
        }
        Err(e) => {
            panic!("Failed to import PLY triangle: {:?}", e);
        }
    }
}

#[test]
fn test_import_builder_functionality() {
    // Test ImportBuilder functionality
    let importer = Importer::new();

    // Test with properties through ImportBuilder

    let result = importer
        .read_from_memory(SIMPLE_OBJ_CUBE.as_bytes())
        .with_property_int("AI_CONFIG_PP_RVC_FLAGS", 1)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .import_from_memory(SIMPLE_OBJ_CUBE.as_bytes(), Some("obj"));

    assert!(result.is_ok(), "Import with properties should succeed");
}

#[test]
fn test_invalid_data_handling() {
    // Test error handling with invalid data (based on AssimpAPITest::aiImportFileFromMemoryTest)
    let importer = Importer::new();

    // Test with empty data
    let result = importer
        .read_from_memory(&[])
        .import_from_memory(&[], Some("obj"));

    assert!(result.is_err(), "Import of empty data should fail");

    // Test with invalid data
    let invalid_data = b"This is not a valid 3D model file";
    let result = importer
        .read_from_memory(invalid_data)
        .import_from_memory(invalid_data, Some("obj"));

    // Note: Assimp might not always fail on invalid data immediately,
    // it depends on the format and validation settings
    // For now, we just check that the function doesn't panic
    match result {
        Ok(_) => println!("Assimp accepted the data (possibly with warnings)"),
        Err(e) => println!("Assimp rejected the data: {}", e),
    }
}

#[test]
fn test_post_processing_steps() {
    // Test various post-processing steps
    let importer = Importer::new();

    let steps = PostProcessSteps::TRIANGULATE
        | PostProcessSteps::JOIN_IDENTICAL_VERTICES
        | PostProcessSteps::GEN_NORMALS
        | PostProcessSteps::VALIDATE_DATA_STRUCTURE;

    let result = importer
        .read_from_memory(SIMPLE_OBJ_CUBE.as_bytes())
        .with_post_process(steps)
        .import_from_memory(SIMPLE_OBJ_CUBE.as_bytes(), Some("obj"));

    assert!(
        result.is_ok(),
        "Import with multiple post-processing steps should succeed"
    );

    if let Ok(scene) = result {
        let mesh = scene.meshes().next().expect("Should have a mesh");

        // After JOIN_IDENTICAL_VERTICES, cube should have fewer vertices than the original 24
        // (OBJ format with quads creates 4 vertices per face * 6 faces = 24 vertices)
        // After joining identical vertices, we should have 8 unique vertices
        // However, the exact number depends on how Assimp processes the data
        println!(
            "Mesh has {} vertices after post-processing",
            mesh.num_vertices()
        );
        assert!(
            mesh.num_vertices() <= 24,
            "Mesh should have at most 24 vertices"
        );
        assert!(
            mesh.num_vertices() >= 8,
            "Mesh should have at least 8 vertices"
        );

        // Should have normals after GENERATE_NORMALS
        assert!(
            mesh.normals().is_some(),
            "Mesh should have normals after post-processing"
        );
    }
}

#[test]
fn test_memory_file_stream_integration() {
    // Test memory file stream with actual model data
    let mut stream = MemoryFileStream::new_writable(1024);

    // Write OBJ data to stream
    let written = stream
        .write(SIMPLE_OBJ_CUBE.as_bytes())
        .expect("Should write data");
    assert_eq!(written, SIMPLE_OBJ_CUBE.len());

    // Reset position for reading
    stream.seek(0).expect("Should seek to beginning");

    // Read back the data
    let mut buffer = vec![0u8; SIMPLE_OBJ_CUBE.len()];
    let read = stream.read(&mut buffer).expect("Should read data");
    assert_eq!(read, SIMPLE_OBJ_CUBE.len());

    let read_string = String::from_utf8(buffer).expect("Should be valid UTF-8");
    assert_eq!(read_string, SIMPLE_OBJ_CUBE);
}
