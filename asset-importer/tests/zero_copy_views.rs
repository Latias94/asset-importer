//! Zero-copy / low-allocation view tests

use asset_importer::{
    Importer,
    material::{PropertyTypeInfo, TextureType, material_keys},
    postprocess::PostProcessSteps,
};
use std::path::Path;

const QUAD_OBJ: &[u8] = b"o quad\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n";

fn import_quad() -> asset_importer::Scene {
    Importer::new()
        .read_from_memory(QUAD_OBJ)
        .with_memory_hint("obj")
        .with_post_process(PostProcessSteps::empty())
        .import()
        .expect("failed to import quad")
}

#[test]
fn test_postprocess_unique_scene() {
    let scene = import_quad();
    assert_eq!(scene.mesh(0).expect("missing mesh").num_faces(), 1);

    let scene = scene
        .apply_postprocess(PostProcessSteps::TRIANGULATE)
        .expect("failed to triangulate uniquely owned scene");
    let mesh = scene.mesh(0).expect("missing processed mesh");
    assert_eq!(mesh.num_faces(), 2);
    assert!(mesh.faces().all(|face| face.indices_raw().len() == 3));
}

#[test]
fn test_postprocess_rejects_shared_scene() {
    let scene = import_quad();
    let shared = scene.clone();

    let error = scene
        .apply_postprocess(PostProcessSteps::TRIANGULATE)
        .expect_err("shared scene should be rejected before calling Assimp");
    assert!(matches!(
        error,
        asset_importer::Error::InvalidParameter { ref message }
            if message.contains("exclusive scene ownership")
    ));
    assert_eq!(
        shared.mesh(0).expect("missing original mesh").num_faces(),
        1
    );
}

#[test]
fn test_postprocess_preserves_borrowed_views() {
    let scene = import_quad();
    let mesh = scene.mesh(0).expect("missing mesh");
    let face = mesh.faces().next().expect("missing face");
    let node = scene.root_node().expect("missing root node");
    let vertices = mesh.vertices_raw();
    let indices = face.indices_raw();
    let name = node.name_str();
    let expected_vertices = vertices.to_vec();
    let expected_indices = indices.to_vec();
    let expected_name = name.to_string();

    let error = scene
        .apply_postprocess(PostProcessSteps::TRIANGULATE)
        .expect_err("live scene-backed views should prevent post-processing");
    assert!(matches!(
        error,
        asset_importer::Error::InvalidParameter { ref message }
            if message.contains("exclusive scene ownership")
    ));
    assert_eq!(vertices, expected_vertices.as_slice());
    assert_eq!(indices, expected_indices.as_slice());
    assert_eq!(name.as_ref(), expected_name);
    assert_eq!(mesh.num_faces(), 1);
}

#[test]
fn test_postprocess_rejects_owning_iterator() {
    let scene = import_quad();
    let mesh = scene.mesh(0).expect("missing mesh");
    let mut faces = mesh.faces();
    drop(mesh);

    let error = scene
        .apply_postprocess(PostProcessSteps::TRIANGULATE)
        .expect_err("an owning iterator should prevent post-processing");
    assert!(matches!(
        error,
        asset_importer::Error::InvalidParameter { ref message }
            if message.contains("exclusive scene ownership")
    ));

    let face = faces.next().expect("iterator lost its scene");
    drop(faces);
    assert_eq!(face.indices_raw(), &[0, 1, 2, 3]);
}

#[test]
fn test_progress_import_retains_importer_for_queries_and_postprocessing() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let progress_calls = calls.clone();
    let scene = Importer::new()
        .read_from_memory(QUAD_OBJ)
        .with_memory_hint("obj")
        .with_post_process(PostProcessSteps::empty())
        .with_progress_handler_fn(move |_, _| {
            progress_calls.fetch_add(1, Ordering::SeqCst);
            true
        })
        .import()
        .expect("failed to import quad with progress handler");

    let info = scene
        .memory_requirements()
        .expect("progress imports retain their importer for memory queries");
    assert!(info.total_bytes() > 0);
    assert_eq!(scene.mesh(0).expect("missing mesh").num_faces(), 1);
    assert_eq!(
        Arc::strong_count(&calls),
        1,
        "progress state was released after import"
    );
    let import_calls = calls.load(Ordering::SeqCst);
    assert!(import_calls > 0);

    let scene = scene
        .apply_postprocess(PostProcessSteps::TRIANGULATE)
        .expect("progress imports support later post-processing");
    assert_eq!(
        scene.mesh(0).expect("missing processed mesh").num_faces(),
        2
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        import_calls,
        "post-processing must not reuse released progress state"
    );
}

#[cfg(feature = "raw-sys")]
#[test]
fn test_raw_scene_owners_use_their_matching_release_api() {
    use asset_importer::{Scene, sys};

    let raw = unsafe {
        sys::aiImportFileFromMemory(
            QUAD_OBJ.as_ptr().cast(),
            QUAD_OBJ.len() as u32,
            0,
            c"obj".as_ptr(),
        )
    };
    let scene = unsafe { Scene::from_raw_import(raw) }.expect("adopt C API scene");
    assert!(
        scene
            .memory_requirements()
            .expect("C API memory query")
            .total
            > 0
    );
    assert!(matches!(
        scene.apply_postprocess(PostProcessSteps::TRIANGULATE),
        Err(asset_importer::Error::InvalidParameter { .. })
    ));

    let source = import_quad();
    let mut copied = std::ptr::null_mut();
    unsafe { sys::aiCopyScene(source.as_raw(), &mut copied) };
    let copied = unsafe { Scene::from_raw_copied(copied) }.expect("adopt deep copy");
    drop(source);
    assert_eq!(copied.mesh(0).expect("copied mesh").num_faces(), 1);
    assert!(matches!(
        copied.memory_requirements(),
        Err(asset_importer::Error::InvalidParameter { .. })
    ));
    assert!(matches!(
        copied.apply_postprocess(PostProcessSteps::TRIANGULATE),
        Err(asset_importer::Error::InvalidParameter { .. })
    ));
}

#[test]
fn test_postprocess_rejects_incompatible_flags() {
    let scene = import_quad();
    let error = scene
        .apply_postprocess(PostProcessSteps::GEN_NORMALS | PostProcessSteps::GEN_SMOOTH_NORMALS)
        .expect_err("incompatible flags should be rejected before calling Assimp");
    assert!(matches!(
        error,
        asset_importer::Error::InvalidParameter { .. }
    ));
}

#[test]
fn test_postprocess_failure_after_indexing() {
    let scene = Importer::new()
        .read_from_memory(QUAD_OBJ)
        .with_memory_hint("obj")
        .with_post_process(PostProcessSteps::JOIN_IDENTICAL_VERTICES)
        .import()
        .expect("failed to import indexed quad");
    assert!(scene.is_non_verbose());

    // Assimp rejects normal generation after indexing and deletes its scene on failure.
    // The bridge must release the importer without dereferencing the deleted scene again.
    let error = scene
        .apply_postprocess(PostProcessSteps::GEN_NORMALS)
        .expect_err("normal generation should reject a non-verbose scene");
    assert!(matches!(error, asset_importer::Error::InvalidScene { .. }));
}

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
        .import()
        .expect("failed to import box.obj");

    let mesh = scene.meshes().next().expect("scene has no meshes");
    let raw_faces = mesh.faces_raw();
    assert!(!raw_faces.is_empty(), "mesh has no raw faces");
    assert_eq!(raw_faces.len(), mesh.num_faces());
    assert_eq!(mesh.faces_iter().count(), mesh.num_faces());
    assert_eq!(mesh.triangles_iter().count(), mesh.num_faces());

    for (i, face) in mesh.faces_iter().enumerate() {
        assert_eq!(face.num_indices(), raw_faces[i].mNumIndices as usize);
        assert_eq!(face.indices().len(), face.num_indices());
        assert_eq!(face.indices_raw().len(), face.num_indices());
    }

    // Flat triangle index stream should have 3 indices per face under triangulation.
    assert_eq!(mesh.triangle_indices_iter().count(), mesh.num_faces() * 3);
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
        .import()
        .expect("failed to import box.obj");

    let mesh = scene.meshes().next().expect("scene has no meshes");
    let raw = mesh.vertices_raw();
    assert!(!raw.is_empty(), "mesh has no vertices");
}

#[cfg(feature = "bytemuck")]
#[test]
fn test_bytemuck_mesh_bytes_views() {
    let model_path = Path::new("tests/models/textured.obj");
    if !model_path.exists() {
        println!("Skipping test - model file not found: {:?}", model_path);
        return;
    }

    let scene = Importer::new()
        .read_file(model_path)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .import()
        .expect("failed to import textured.obj");

    let mesh = scene.meshes().next().expect("scene has no meshes");
    assert_eq!(mesh.vertices_bytes().len(), mesh.vertices_raw().len() * 12);
    assert_eq!(mesh.vertices_f32().len(), mesh.vertices_raw().len() * 3);

    assert_eq!(mesh.normals_bytes().len(), mesh.normals_raw().len() * 12);
    assert_eq!(mesh.normals_f32().len(), mesh.normals_raw().len() * 3);

    assert_eq!(
        mesh.texture_coords_bytes(0).len(),
        mesh.texture_coords_raw(0).len() * 12
    );
    assert_eq!(
        mesh.texture_coords_f32(0).len(),
        mesh.texture_coords_raw(0).len() * 3
    );
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
        .import()
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
        .import()
        .expect("failed to import textured.obj");

    let mesh = scene.meshes().next().expect("scene has no meshes");
    assert!(mesh.has_vertices());
    assert!(mesh.has_normals());
    assert!(mesh.has_texture_coords(0));
    assert!(!mesh.has_vertex_colors(0));

    // 2D UV convenience API should match the first two components of the raw buffer.
    let raw_uv = mesh.texture_coords_raw(0);
    assert!(!raw_uv.is_empty());
    let uv2 = mesh.texture_coords2(0).expect("expected UVs");
    assert_eq!(uv2.len(), raw_uv.len());
    assert!((uv2[0].x - raw_uv[0].x).abs() < 1e-6);
    assert!((uv2[0].y - raw_uv[0].y).abs() < 1e-6);
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
        .import()
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
