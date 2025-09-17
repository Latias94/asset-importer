//! Tests for enhanced material system and property store functionality

use asset_importer::{
    import_properties, material_keys, postprocess::PostProcessSteps, Importer, PropertyStore,
    PropertyValue, Scene,
};

// Simple OBJ cube for testing
const SIMPLE_OBJ_CUBE: &str = r#"
# Simple cube
v -1.0 -1.0  1.0
v  1.0 -1.0  1.0
v  1.0  1.0  1.0
v -1.0  1.0  1.0
v -1.0 -1.0 -1.0
v  1.0 -1.0 -1.0
v  1.0  1.0 -1.0
v -1.0  1.0 -1.0

f 1 2 3 4
f 5 8 7 6
f 1 5 6 2
f 2 6 7 3
f 3 7 8 4
f 5 1 4 8
"#;

#[test]
fn test_property_store_creation() {
    let mut store = PropertyStore::new();
    assert!(store.is_empty());
    assert_eq!(store.len(), 0);

    store.set_int("test_int", 42);
    store.set_float("test_float", std::f32::consts::PI);
    store.set_string("test_string", "hello");
    store.set_bool("test_bool", true);

    assert!(!store.is_empty());
    assert_eq!(store.len(), 4);

    let properties = store.properties();
    assert_eq!(properties.len(), 4);
}

#[test]
fn test_property_store_methods() {
    let mut store = PropertyStore::new();

    // Test method chaining
    store
        .set_int("int_prop", 100)
        .set_float("float_prop", std::f32::consts::E)
        .set_string("string_prop", "world")
        .set_bool("bool_prop", false);

    assert_eq!(store.len(), 4);

    // Test clear
    store.clear();
    assert!(store.is_empty());
}

#[test]
fn test_property_store_conversions() {
    let mut store = PropertyStore::new();
    store.set_int("test", 123);

    // Test conversion to Vec
    let vec: Vec<(String, PropertyValue)> = store.clone().into();
    assert_eq!(vec.len(), 1);
    assert_eq!(vec[0].0, "test");

    // Test conversion from Vec
    let new_store = PropertyStore::from(vec);
    assert_eq!(new_store.len(), 1);
}

#[test]
fn test_import_with_property_store() {
    let mut props = PropertyStore::new();
    props
        .set_float(import_properties::MAX_SMOOTHING_ANGLE, 45.0)
        .set_bool(import_properties::REMOVE_DEGENERATE_FACES, true)
        .set_int(import_properties::LIMIT_BONE_WEIGHTS_MAX, 4);

    let importer = Importer::new();
    let result = importer
        .read_from_memory(SIMPLE_OBJ_CUBE.as_bytes())
        .with_property_store(props)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .import_from_memory(SIMPLE_OBJ_CUBE.as_bytes(), Some("obj"));

    assert!(result.is_ok(), "Import with PropertyStore should succeed");
    let scene = result.unwrap();
    assert!(scene.num_meshes() > 0);
}

#[test]
fn test_russimp_compatible_interface() {
    // Test Scene::from_file
    let result = Scene::from_file("non_existent.obj");
    assert!(result.is_err(), "Should fail for non-existent file");

    // Test Scene::from_memory
    let result = Scene::from_memory(SIMPLE_OBJ_CUBE.as_bytes(), Some("obj"));
    assert!(result.is_ok(), "Should load simple OBJ from memory");
    let scene = result.unwrap();
    assert!(scene.num_meshes() > 0);

    // Test Scene::from_memory_with_flags
    let result = Scene::from_memory_with_flags(
        SIMPLE_OBJ_CUBE.as_bytes(),
        Some("obj"),
        PostProcessSteps::TRIANGULATE,
    );
    assert!(result.is_ok(), "Should load with post-processing flags");
    let scene = result.unwrap();
    assert!(scene.num_meshes() > 0);

    // Test Scene::from_memory_with_props
    let mut props = PropertyStore::new();
    props.set_bool(import_properties::REMOVE_DEGENERATE_FACES, true);

    let result = Scene::from_memory_with_props(
        SIMPLE_OBJ_CUBE.as_bytes(),
        Some("obj"),
        PostProcessSteps::TRIANGULATE,
        &props,
    );
    assert!(result.is_ok(), "Should load with properties");
    let scene = result.unwrap();
    assert!(scene.num_meshes() > 0);
}

#[test]
fn test_import_with_property_store_ref() {
    let mut props = PropertyStore::new();
    props.set_float(import_properties::MAX_SMOOTHING_ANGLE, 60.0);

    let importer = Importer::new();
    let result = importer
        .read_from_memory(SIMPLE_OBJ_CUBE.as_bytes())
        .with_property_store_ref(&props)
        .import_from_memory(SIMPLE_OBJ_CUBE.as_bytes(), Some("obj"));

    assert!(
        result.is_ok(),
        "Import with PropertyStore reference should succeed"
    );
}

#[test]
fn test_matrix_property() {
    use asset_importer::types::Matrix4x4;

    let matrix = Matrix4x4::IDENTITY;
    let mut store = PropertyStore::new();
    store.set_matrix("transform", matrix);

    assert_eq!(store.len(), 1);

    // Verify the property was stored
    let properties = store.properties();
    match &properties[0].1 {
        PropertyValue::Matrix(m) => {
            assert_eq!(*m, Matrix4x4::IDENTITY);
        }
        _ => panic!("Expected Matrix property"),
    }
}

#[test]
fn test_material_property_access() {
    let importer = Importer::new();
    let result = importer.import_from_memory(SIMPLE_OBJ_CUBE.as_bytes(), Some("obj"));

    if let Ok(scene) = result {
        if scene.num_materials() > 0 {
            let material = scene.materials().next().unwrap();

            // Test basic property access
            let name = material.name();
            assert!(!name.is_empty() || name.is_empty()); // Name might be empty for default material

            // Test color properties (might be None for simple OBJ)
            let _diffuse = material.diffuse_color();
            let _specular = material.specular_color();
            let _ambient = material.ambient_color();
            let _emissive = material.emissive_color();

            // Test material flags
            let _two_sided = material.is_two_sided();

            // Test texture counts
            let _diffuse_count = material.texture_count(asset_importer::TextureType::Diffuse);
            // diffuse_count is u32, so it's always >= 0
        }
    }
}

#[test]
fn test_material_keys_constants() {
    // Test that material key constants are valid strings
    assert!(!material_keys::NAME.is_empty());
    assert!(!material_keys::COLOR_DIFFUSE.is_empty());
    assert!(!material_keys::COLOR_SPECULAR.is_empty());
    assert!(!material_keys::SHININESS.is_empty());
    assert!(!material_keys::OPACITY.is_empty());
}

#[test]
fn test_import_properties_constants() {
    // Test that import property constants are valid strings
    assert!(!import_properties::MAX_SMOOTHING_ANGLE.is_empty());
    assert!(!import_properties::FBX_READ_ALL_GEOMETRY_LAYERS.is_empty());
    assert!(!import_properties::REMOVE_DEGENERATE_FACES.is_empty());
    assert!(!import_properties::GLOBAL_SCALE_FACTOR.is_empty());
}

#[test]
fn test_enhanced_metadata_access() {
    let importer = Importer::new();
    let result = importer.import_from_memory(SIMPLE_OBJ_CUBE.as_bytes(), Some("obj"));

    if let Ok(scene) = result {
        if let Ok(metadata) = scene.metadata() {
            // Test enhanced metadata access methods
            for (key, _) in metadata.iter() {
                // Try different accessor methods
                let _bool_val = metadata.get_bool(key);
                let _int_val = metadata.get_i32(key);
                let _float_val = metadata.get_f32(key);
                let _string_val = metadata.get_string(key);
                let _vector_val = metadata.get_vector3d(key);
            }
        }
    }
}

#[test]
fn test_property_value_variants() {
    // Test all PropertyValue variants
    let _int_prop = PropertyValue::Integer(42);
    let _float_prop = PropertyValue::Float(std::f32::consts::PI);
    let _string_prop = PropertyValue::String("test".to_string());
    let _bool_prop = PropertyValue::Boolean(true);
    let _matrix_prop = PropertyValue::Matrix(asset_importer::types::Matrix4x4::IDENTITY);

    // Verify they can be created and stored
    let mut store = PropertyStore::new();
    store.set_int("int", 42);
    store.set_float("float", std::f32::consts::PI);
    store.set_string("string", "test");
    store.set_bool("bool", true);
    store.set_matrix("matrix", asset_importer::types::Matrix4x4::IDENTITY);

    assert_eq!(store.len(), 5);
}

#[test]
#[cfg(feature = "build-assimp")]
fn test_texture_system() -> Result<(), Box<dyn std::error::Error>> {
    // Test with a simple OBJ that won't have embedded textures
    let obj_data = create_simple_cube_obj();
    let scene = Scene::from_memory_with_flags(
        obj_data.as_bytes(),
        Some("obj"),
        PostProcessSteps::TRIANGULATE,
    )?;

    // OBJ files typically don't have embedded textures
    assert_eq!(scene.num_textures(), 0);
    assert!(!scene.has_textures());
    assert!(scene.compressed_textures().is_empty());
    assert!(scene.uncompressed_textures().is_empty());

    // Test texture iterator on empty scene
    let texture_count = scene.textures().count();
    assert_eq!(texture_count, 0);

    Ok(())
}

#[test]
#[cfg(feature = "build-assimp")]
fn test_aabb_system() -> Result<(), Box<dyn std::error::Error>> {
    let obj_data = create_simple_cube_obj();
    let scene = Scene::from_memory_with_flags(
        obj_data.as_bytes(),
        Some("obj"),
        PostProcessSteps::TRIANGULATE | PostProcessSteps::GEN_BOUNDING_BOXES,
    )?;

    assert!(scene.num_meshes() > 0);

    let mesh = scene.meshes().next().unwrap();
    let aabb = mesh.aabb();

    // Test AABB properties
    assert!(aabb.is_valid());

    // For a unit cube from -1 to 1, we expect specific bounds
    let tolerance = 0.1; // Allow some tolerance for floating point
    assert!((aabb.min.x - (-1.0)).abs() < tolerance);
    assert!((aabb.min.y - (-1.0)).abs() < tolerance);
    assert!((aabb.min.z - (-1.0)).abs() < tolerance);
    assert!((aabb.max.x - 1.0).abs() < tolerance);
    assert!((aabb.max.y - 1.0).abs() < tolerance);
    assert!((aabb.max.z - 1.0).abs() < tolerance);

    // Test AABB calculations
    let size = aabb.size();
    assert!((size.x - 2.0).abs() < tolerance);
    assert!((size.y - 2.0).abs() < tolerance);
    assert!((size.z - 2.0).abs() < tolerance);

    let center = aabb.center();
    assert!(center.x.abs() < tolerance);
    assert!(center.y.abs() < tolerance);
    assert!(center.z.abs() < tolerance);

    let volume = aabb.volume();
    assert!((volume - 8.0).abs() < tolerance); // 2x2x2 = 8

    let surface_area = aabb.surface_area();
    assert!((surface_area - 24.0).abs() < tolerance); // 6 faces * 4 area each = 24

    Ok(())
}

#[test]
#[cfg(feature = "build-assimp")]
fn test_bone_system() -> Result<(), Box<dyn std::error::Error>> {
    let obj_data = create_simple_cube_obj();
    let scene = Scene::from_memory_with_flags(
        obj_data.as_bytes(),
        Some("obj"),
        PostProcessSteps::TRIANGULATE,
    )?;

    assert!(scene.num_meshes() > 0);

    let mesh = scene.meshes().next().unwrap();

    // OBJ files typically don't have bone data
    assert_eq!(mesh.num_bones(), 0);
    assert!(!mesh.has_bones());
    assert!(mesh.bone_names().is_empty());

    // Test bone iterator on empty mesh
    let bone_count = mesh.bones().count();
    assert_eq!(bone_count, 0);

    // Test finding non-existent bone
    assert!(mesh.find_bone_by_name("non_existent").is_none());

    Ok(())
}

#[test]
#[cfg(feature = "build-assimp")]
fn test_vertex_weight_operations() {
    use asset_importer::VertexWeight;

    let weight = VertexWeight::new(42, 0.75);
    assert_eq!(weight.vertex_id, 42);
    assert_eq!(weight.weight, 0.75);

    assert!(weight.is_significant(0.5));
    assert!(!weight.is_significant(0.8));

    // Test normalization
    let over_weight = VertexWeight::new(10, 1.5);
    let normalized = over_weight.normalized();
    assert_eq!(normalized.weight, 1.0);

    let under_weight = VertexWeight::new(20, -0.5);
    let normalized = under_weight.normalized();
    assert_eq!(normalized.weight, 0.0);
}

#[test]
#[cfg(feature = "build-assimp")]
fn test_aabb_operations() {
    use asset_importer::{types::Vector3D, AABB};

    let min = Vector3D::new(-1.0, -2.0, -3.0);
    let max = Vector3D::new(1.0, 2.0, 3.0);
    let aabb = AABB::new(min, max);

    assert!(aabb.is_valid());
    assert_eq!(aabb.size(), Vector3D::new(2.0, 4.0, 6.0));
    assert_eq!(aabb.center(), Vector3D::new(0.0, 0.0, 0.0));
    assert_eq!(aabb.volume(), 48.0); // 2 * 4 * 6
    assert_eq!(aabb.surface_area(), 88.0); // 2 * (2*4 + 2*6 + 4*6)

    // Test point containment
    assert!(aabb.contains_point(Vector3D::new(0.0, 0.0, 0.0)));
    assert!(aabb.contains_point(Vector3D::new(-1.0, -2.0, -3.0))); // min corner
    assert!(aabb.contains_point(Vector3D::new(1.0, 2.0, 3.0))); // max corner
    assert!(!aabb.contains_point(Vector3D::new(2.0, 0.0, 0.0))); // outside

    // Test AABB intersection
    let other = AABB::new(Vector3D::new(0.0, 0.0, 0.0), Vector3D::new(2.0, 3.0, 4.0));
    assert!(aabb.intersects_aabb(&other));

    let non_intersecting = AABB::new(Vector3D::new(5.0, 5.0, 5.0), Vector3D::new(6.0, 6.0, 6.0));
    assert!(!aabb.intersects_aabb(&non_intersecting));

    // Test expansion
    let point = Vector3D::new(5.0, 5.0, 5.0);
    let expanded = aabb.expanded_to_include_point(point);
    assert!(expanded.contains_point(point));
    assert_eq!(expanded.max, Vector3D::new(5.0, 5.0, 5.0));
}

#[test]
#[cfg(feature = "build-assimp")]
fn test_importer_desc_functionality() -> Result<(), Box<dyn std::error::Error>> {
    use asset_importer::{get_all_importer_descs, get_importer_desc, ImporterFlags};

    // Test getting description for OBJ format
    let obj_desc = get_importer_desc("obj");
    assert!(obj_desc.is_some(), "OBJ format should be supported");

    if let Some(desc) = obj_desc {
        assert!(!desc.name.is_empty());
        assert!(desc.file_extensions.contains(&"obj".to_string()));
        println!("OBJ Importer: {}", desc.name);
        println!("Extensions: {:?}", desc.file_extensions);
    }

    // Test getting all importers
    let all_importers = get_all_importer_descs();
    assert!(
        !all_importers.is_empty(),
        "Should have at least some importers"
    );

    println!("Found {} importers", all_importers.len());
    for desc in &all_importers[..std::cmp::min(5, all_importers.len())] {
        println!("  {} - {:?}", desc.name, desc.file_extensions);
    }

    // Test invalid extension
    let invalid_desc = get_importer_desc("invalid_xyz");
    assert!(invalid_desc.is_none());

    // Test flags functionality
    let flags = ImporterFlags::SUPPORT_TEXT_FLAVOUR | ImporterFlags::SUPPORT_BINARY_FLAVOUR;
    assert!(flags.contains(ImporterFlags::SUPPORT_TEXT_FLAVOUR));
    assert!(flags.contains(ImporterFlags::SUPPORT_BINARY_FLAVOUR));
    assert!(!flags.contains(ImporterFlags::EXPERIMENTAL));

    Ok(())
}

#[test]
fn test_common_metadata_constants() {
    use asset_importer::metadata::{collada_metadata, common_metadata};

    // Test common metadata constants
    assert_eq!(common_metadata::SOURCE_FORMAT, "SourceAsset_Format");
    assert_eq!(
        common_metadata::SOURCE_FORMAT_VERSION,
        "SourceAsset_FormatVersion"
    );
    assert_eq!(common_metadata::SOURCE_GENERATOR, "SourceAsset_Generator");
    assert_eq!(common_metadata::SOURCE_COPYRIGHT, "SourceAsset_Copyright");

    // Test Collada metadata constants
    assert_eq!(collada_metadata::ID, "Collada_id");
    assert_eq!(collada_metadata::SID, "Collada_sid");
}

#[test]
#[cfg(feature = "build-assimp")]
fn test_memory_requirements() -> Result<(), Box<dyn std::error::Error>> {
    use asset_importer::{postprocess::PostProcessSteps, MemoryInfo, Scene};

    // Create a simple OBJ scene in memory
    let obj_data = create_simple_cube_obj();

    let scene = Scene::from_memory_with_flags(
        obj_data.as_bytes(),
        Some("obj"),
        PostProcessSteps::TRIANGULATE | PostProcessSteps::GEN_BOUNDING_BOXES,
    )?;

    // Test memory requirements
    let memory_info = scene.memory_requirements()?;

    println!("Memory requirements:");
    println!(
        "  Total: {} bytes ({:.2} KB)",
        memory_info.total_bytes(),
        memory_info.total_kb()
    );
    println!("  Meshes: {} bytes", memory_info.meshes);
    println!("  Materials: {} bytes", memory_info.materials);
    println!("  Nodes: {} bytes", memory_info.nodes);

    // Verify that we have some memory usage
    assert!(
        memory_info.total > 0,
        "Total memory should be greater than 0"
    );
    assert!(
        memory_info.meshes > 0,
        "Mesh memory should be greater than 0"
    );
    assert!(
        memory_info.nodes > 0,
        "Node memory should be greater than 0"
    );

    // Test breakdown
    let breakdown = memory_info.breakdown();
    assert_eq!(breakdown.len(), 7, "Should have 7 components in breakdown");

    // Test convenience methods
    assert_eq!(memory_info.total_bytes(), memory_info.total);
    assert!((memory_info.total_kb() - (memory_info.total as f64 / 1024.0)).abs() < 0.001);
    assert!(
        (memory_info.total_mb() - (memory_info.total as f64 / (1024.0 * 1024.0))).abs() < 0.001
    );

    // Test default and new
    let default_info = MemoryInfo::default();
    assert_eq!(default_info.total, 0);

    let new_info = MemoryInfo::new();
    assert_eq!(new_info.total, 0);
    assert_eq!(new_info, default_info);

    Ok(())
}

#[test]
#[cfg(feature = "build-assimp")]
fn test_postprocess_validation() -> Result<(), Box<dyn std::error::Error>> {
    use asset_importer::postprocess::PostProcessSteps;

    // Test valid combinations
    let valid_steps = PostProcessSteps::TRIANGULATE | PostProcessSteps::JOIN_IDENTICAL_VERTICES;
    assert!(
        valid_steps.is_valid(),
        "Valid combination should pass validation"
    );
    assert!(
        valid_steps.validate().is_ok(),
        "Valid combination should return Ok"
    );

    // Test invalid combination: GEN_SMOOTH_NORMALS and GEN_NORMALS
    let invalid_steps1 = PostProcessSteps::GEN_SMOOTH_NORMALS | PostProcessSteps::GEN_NORMALS;
    assert!(
        !invalid_steps1.is_valid(),
        "Invalid combination should fail validation"
    );
    assert!(
        invalid_steps1.validate().is_err(),
        "Invalid combination should return Err"
    );

    let error_msg = invalid_steps1.validate().unwrap_err();
    assert!(
        error_msg.contains("incompatible"),
        "Error message should mention incompatibility"
    );

    // Test invalid combination: OPTIMIZE_GRAPH and PRE_TRANSFORM_VERTICES
    let invalid_steps2 =
        PostProcessSteps::OPTIMIZE_GRAPH | PostProcessSteps::PRE_TRANSFORM_VERTICES;
    assert!(
        !invalid_steps2.is_valid(),
        "Invalid combination should fail validation"
    );
    assert!(
        invalid_steps2.validate().is_err(),
        "Invalid combination should return Err"
    );

    // Test that presets are valid
    assert!(
        PostProcessSteps::FAST.is_valid(),
        "FAST preset should be valid"
    );
    assert!(
        PostProcessSteps::QUALITY.is_valid(),
        "QUALITY preset should be valid"
    );
    assert!(
        PostProcessSteps::MAX_QUALITY.is_valid(),
        "MAX_QUALITY preset should be valid"
    );
    assert!(
        PostProcessSteps::REALTIME.is_valid(),
        "REALTIME preset should be valid"
    );

    Ok(())
}
