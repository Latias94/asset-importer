//! Zero-copy / low-allocation view tests

use asset_importer::{
    Importer,
    material::{PropertyTypeInfo, TextureType, material_keys},
    postprocess::PostProcessSteps,
};
use std::path::Path;

#[test]
fn test_mesh_faces_raw_and_iter() {
    let model_path = Path::new("tests/models/box.obj");
    if !model_path.exists() {
        println!("Skipping test - model file not found: {:?}", model_path);
        return;
    }

    let scene = Importer::new()
        .read_file(model_path)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .import_file(model_path)
        .expect("failed to import box.obj");

    let mesh = scene.meshes().next().expect("scene has no meshes");
    let raw_faces = mesh.faces_raw();
    assert!(!raw_faces.is_empty(), "mesh has no raw faces");
    assert_eq!(raw_faces.len(), mesh.num_faces());
    assert_eq!(mesh.faces_iter().count(), mesh.num_faces());

    for (i, face) in mesh.faces_iter().enumerate() {
        assert_eq!(face.num_indices(), raw_faces[i].mNumIndices as usize);
        assert_eq!(face.indices().len(), face.num_indices());
        assert_eq!(face.indices_raw().len(), face.num_indices());
    }
}

#[test]
fn test_mesh_vertices_raw_type_is_sys_free() {
    let model_path = Path::new("tests/models/box.obj");
    if !model_path.exists() {
        println!("Skipping test - model file not found: {:?}", model_path);
        return;
    }

    let scene = Importer::new()
        .read_file(model_path)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .import_file(model_path)
        .expect("failed to import box.obj");

    let mesh = scene.meshes().next().expect("scene has no meshes");
    let raw = mesh.vertices_raw();
    assert!(!raw.is_empty(), "mesh has no vertices");
}

#[test]
fn test_mesh_has_helpers() {
    let box_path = Path::new("tests/models/box.obj");
    if !box_path.exists() {
        println!("Skipping test - model file not found: {:?}", box_path);
        return;
    }

    let scene = Importer::new()
        .read_file(box_path)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .import_file(box_path)
        .expect("failed to import box.obj");

    let mesh = scene.meshes().next().expect("scene has no meshes");
    assert!(mesh.has_vertices());
    assert!(!mesh.has_normals());
    assert!(!mesh.has_tangents());
    assert!(!mesh.has_bitangents());
    assert!(!mesh.has_texture_coords(0));
    assert!(!mesh.has_vertex_colors(0));

    let tri_path = Path::new("tests/models/textured.obj");
    if !tri_path.exists() {
        println!("Skipping test - model file not found: {:?}", tri_path);
        return;
    }

    let scene = Importer::new()
        .read_file(tri_path)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .import_file(tri_path)
        .expect("failed to import textured.obj");

    let mesh = scene.meshes().next().expect("scene has no meshes");
    assert!(mesh.has_vertices());
    assert!(mesh.has_normals());
    assert!(mesh.has_texture_coords(0));
    assert!(!mesh.has_vertex_colors(0));
}

#[test]
fn test_material_texture_ref_path() {
    let model_path = Path::new("tests/models/textured.obj");
    if !model_path.exists() {
        println!("Skipping test - model file not found: {:?}", model_path);
        return;
    }

    let scene = Importer::new()
        .read_file(model_path)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .import_file(model_path)
        .expect("failed to import textured.obj");

    let material = scene
        .materials()
        .find(|m| m.texture_count(TextureType::Diffuse) > 0)
        .expect("no material with a diffuse texture");

    assert_eq!(material.texture_count(TextureType::Diffuse), 1);

    let tex = material
        .texture_ref(TextureType::Diffuse, 0)
        .expect("missing diffuse texture 0");
    assert_eq!(tex.path_str().as_ref(), "dummy.png");

    let owned = tex.to_owned();
    assert_eq!(owned.path, "dummy.png");

    let owned2 = material
        .texture(TextureType::Diffuse, 0)
        .expect("missing owned diffuse texture 0");
    assert_eq!(owned2.path, "dummy.png");

    // Diffuse color from .mtl should be available as a float property as well.
    let diffuse_prop = material
        .properties()
        .find(|p| p.key_str().as_ref() == material_keys::COLOR_DIFFUSE.to_str().unwrap())
        .expect("missing $clr.diffuse property");
    assert_eq!(diffuse_prop.type_info(), PropertyTypeInfo::Float);
    let c = diffuse_prop
        .as_color3()
        .expect("diffuse should decode as Color3D");
    assert!((c.x - 1.0).abs() < 1e-6);
    assert!((c.y - 1.0).abs() < 1e-6);
    assert!((c.z - 1.0).abs() < 1e-6);

    // If Assimp exposes any string-typed material properties, ensure they can be decoded
    // without additional queries back into the material API.
    for p in material.properties() {
        if p.type_info() == PropertyTypeInfo::String {
            assert!(p.string_ref().is_some());
        }
    }
}
