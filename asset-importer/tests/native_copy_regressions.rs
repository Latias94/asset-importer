#![cfg(feature = "raw-sys")]

use asset_importer::io::MemoryFileSystem;
use asset_importer::postprocess::PostProcessSteps;
use asset_importer::{Error, Importer, Scene};

fn shared_morph_gltf(channels: usize) -> String {
    // Positions, two-component UVs, RGBA colors, then zero position deltas.
    let data = "AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AACAPwAAAAAAAAAAAACAPwAAAAAAAIA/AAAAAAAAgD8AAAAAAAAAAAAAgD8AAIA/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let mut attributes = String::from("\"POSITION\": 0");
    for channel in 0..channels {
        attributes.push_str(&format!(
            ", \"TEXCOORD_{channel}\": 1, \"COLOR_{channel}\": 2",
        ));
    }
    format!(
        r#"{{
          "asset": {{ "version": "2.0" }},
          "buffers": [{{ "uri": "data:application/octet-stream;base64,{data}", "byteLength": 144 }}],
          "bufferViews": [
            {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
            {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
            {{ "buffer": 0, "byteOffset": 60, "byteLength": 48 }},
            {{ "buffer": 0, "byteOffset": 108, "byteLength": 36 }}
          ],
          "accessors": [
            {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0] }},
            {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
            {{ "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC4" }},
            {{ "bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [0,0,0] }}
          ],
          "meshes": [{{ "weights": [0.5, 0.5], "primitives": [{{
            "attributes": {{ {attributes} }},
            "targets": [{{ "POSITION": 3 }}, {{ "POSITION": 3 }}]
          }}] }}],
          "nodes": [
            {{ "name": "left", "mesh": 0, "translation": [-2,0,0] }},
            {{ "name": "right", "mesh": 0, "translation": [3,0,0] }}
          ],
          "scenes": [{{ "nodes": [0,1] }}],
          "scene": 0
        }}"#,
    )
}

fn morph_scene(channels: usize, with_morph_channels: bool) -> Scene {
    let gltf = shared_morph_gltf(channels);
    let import = || {
        Importer::new()
            .read_from_memory(gltf.as_bytes())
            .with_memory_hint("gltf")
            .with_property_bool("PP_PTV_KEEP_HIERARCHY", true)
            .with_post_process(PostProcessSteps::empty())
            .import()
            .expect("import native-owned shared morph fixture")
    };
    let scene = import();
    assert_eq!(scene.num_meshes(), 1);
    {
        let root = scene.root_node().expect("fixture root");
        assert_eq!(root.num_children(), 2);
        let left = root.child(0).unwrap();
        let right = root.child(1).unwrap();
        assert_eq!(left.mesh_indices_raw(), &[0]);
        assert_eq!(right.mesh_indices_raw(), &[0]);
        assert_ne!(left.transformation(), right.transformation());
        assert_eq!(scene.mesh(0).unwrap().num_anim_meshes(), 2);
    }
    if channels > 0 && with_morph_channels {
        let donor = import();
        // SAFETY: The scopes above released all views; both scenes have unique
        // ownership. glTF creates position-only morph targets. Move the donor's
        // C++ arrays to target 1, leaving null slots in the donor. The host and
        // target have matching channels and vertex counts. Each native buffer
        // remains owned by exactly one C++ mesh and is freed with delete[].
        unsafe {
            let mesh = &mut **(*scene.as_raw()).mMeshes;
            let anim = &mut **mesh.mAnimMeshes.add(1);
            let donor_mesh = &mut **(*donor.as_raw()).mMeshes;
            assert_eq!(anim.mNumVertices, donor_mesh.mNumVertices);
            assert!(anim.mTextureCoords.iter().all(|ptr| ptr.is_null()));
            assert!(anim.mColors.iter().all(|ptr| ptr.is_null()));
            std::mem::swap(&mut anim.mTextureCoords, &mut donor_mesh.mTextureCoords);
            std::mem::swap(&mut anim.mColors, &mut donor_mesh.mColors);
            donor_mesh.mNumUVComponents.fill(0);
        }
    }
    scene
}

fn assert_pretransformed_morphs(scene: &Scene) {
    assert_eq!(scene.num_meshes(), 2, "different transforms require a copy");
    let root = scene.root_node().expect("preserved root");
    assert_eq!(root.num_children(), 2);
    assert_ne!(
        root.child(0).unwrap().mesh_index(0),
        root.child(1).unwrap().mesh_index(0),
    );
    let mut minimum_x = Vec::new();
    for mesh in scene.meshes() {
        assert_eq!(mesh.num_anim_meshes(), 2);
        assert_eq!(mesh.num_vertices(), 3);
        assert_eq!(mesh.num_faces(), 1);
        for anim in mesh.anim_meshes() {
            assert_eq!(anim.vertices_raw().len(), 3);
        }
        minimum_x.push(
            mesh.vertices_raw()
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::INFINITY, f32::min),
        );
    }
    minimum_x.sort_by(f32::total_cmp);
    assert_eq!(minimum_x, [-2.0, 3.0]);
}

#[test]
fn sparse_channels_are_rejected_before_pretransform_copy_and_cleanup() {
    for morph in [false, true] {
        for (colors, from, to) in [(false, 0, 2), (false, 1, 7), (true, 0, 2), (true, 1, 7)] {
            let scene = morph_scene(2, morph);
            // SAFETY: No scene clones, views or iterators are alive. Swap native
            // pointers instead of allocating Rust buffers or duplicating owners.
            // In the morph case, swap matching host channels too. Otherwise both
            // targets remain position-only. Every array keeps one native owner.
            unsafe {
                let mesh = &mut **(*scene.as_raw()).mMeshes;
                let anim = &mut **mesh.mAnimMeshes.add(1);
                if colors {
                    mesh.mColors.swap(from, to);
                    if morph {
                        anim.mColors.swap(from, to);
                    }
                } else {
                    mesh.mTextureCoords.swap(from, to);
                    mesh.mNumUVComponents.swap(from, to);
                    if morph {
                        anim.mTextureCoords.swap(from, to);
                    }
                }
            }

            // The safe API consumes the scene. Rejection also destroys its
            // importer, so a successful test exercises cleanup of every buffer.
            let error = scene
                .apply_postprocess(PostProcessSteps::PRE_TRANSFORM_VERTICES)
                .expect_err("reject sparse channels before BuildWCSMeshes copies the mesh");
            let Error::InvalidScene { message } = error else {
                panic!("expected InvalidScene from native guard, got {error}");
            };
            assert!(message.contains("mesh 0"), "{message}");
            if morph {
                assert!(message.contains("morph target 1"), "{message}");
            } else {
                assert!(!message.contains("morph target"), "{message}");
            }
            let kind = if colors {
                "vertex color"
            } else {
                "texture coordinate"
            };
            let channel = if from == 0 { 1 } else { to };
            assert!(message.contains(kind), "{message}");
            assert!(message.contains(&format!("channel {channel}")), "{message}");
        }
    }
}

#[test]
fn dense_and_position_only_morphs_still_support_later_pretransform() {
    for channels in [0, 2] {
        let scene = morph_scene(channels, true)
            .apply_postprocess(PostProcessSteps::PRE_TRANSFORM_VERTICES)
            .expect("copy dense morph targets through safe post-processing");
        assert_pretransformed_morphs(&scene);
        for mesh in scene.meshes() {
            let anim = mesh.anim_mesh(1).unwrap();
            for channel in 0..2 {
                let expected_len = if channel < channels { 3 } else { 0 };
                assert_eq!(anim.texture_coords_raw(channel).len(), expected_len);
                assert_eq!(anim.vertex_colors_raw(channel).len(), expected_len);
            }
        }
    }
}

#[test]
fn dense_pretransform_imports_support_memory_and_custom_io_with_validation() {
    // These imports cover the dense success path only. glTF does not populate
    // morph UV/color channels, so this is not a sparse import rejection test.
    for validate in [false, true] {
        for from_memory in [false, true] {
            let gltf = shared_morph_gltf(2);
            let builder = if from_memory {
                Importer::new()
                    .read_from_memory(gltf.as_bytes())
                    .with_memory_hint("gltf")
            } else {
                let mut file_system = MemoryFileSystem::new();
                file_system.add_file("shared-morph.gltf", gltf.into_bytes());
                Importer::new()
                    .read_file("shared-morph.gltf")
                    .with_file_system(file_system)
            };
            let mut flags = PostProcessSteps::PRE_TRANSFORM_VERTICES;
            if validate {
                flags |= PostProcessSteps::VALIDATE_DATA_STRUCTURE;
            }
            let scene = builder
                .with_property_bool("PP_PTV_KEEP_HIERARCHY", true)
                .with_post_process(flags)
                .import()
                .expect("guarded import copies a dense shared morph mesh");
            assert_pretransformed_morphs(&scene);
        }
    }
}
