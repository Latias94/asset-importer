//! Zero-copy / low-allocation view tests

use asset_importer::{Importer, material::TextureType, postprocess::PostProcessSteps};
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
    let raw_faces = mesh.faces_raw().expect("mesh has no raw faces");
    assert_eq!(raw_faces.len(), mesh.num_faces());
    assert_eq!(mesh.faces_iter().count(), mesh.num_faces());

    for (i, face) in mesh.faces_iter().enumerate() {
        assert_eq!(face.num_indices(), raw_faces[i].mNumIndices as usize);
        assert_eq!(face.indices().len(), face.num_indices());
        assert_eq!(
            face.indices_raw().map(|s| s.len()).unwrap_or(0),
            face.num_indices()
        );
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
    let raw = mesh.vertices_raw().expect("mesh has no vertices");
    assert!(!raw.is_empty());
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
}
