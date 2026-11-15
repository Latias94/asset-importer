use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use asset_importer::io::MemoryFileSystem;
use asset_importer::postprocess::PostProcessSteps;
use asset_importer::{Error, ImportBuilder, Importer, Scene};

const TRIANGLE: &[u8] = b"o tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
const NORMAL_ONLY_MORPH_GLTF: &[u8] = br#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AAAAAAAAgD8AAAAAAAAAAAAAgD8AAAAAAAAAAAAAgD8AAAAA", "byteLength": 108 }],
  "bufferViews": [
    { "buffer": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 72, "byteLength": 36 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0] },
    { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,1,0], "max": [0,1,0] }
  ],
  "meshes": [{ "weights": [0.5], "primitives": [{
    "attributes": { "POSITION": 0, "NORMAL": 1 }, "targets": [{ "NORMAL": 2 }]
  }] }],
  "nodes": [{ "mesh": 0 }], "scenes": [{ "nodes": [0] }], "scene": 0
}"#;
#[cfg(feature = "raw-sys")]
const UNUSED_VERTEX_PLY: &[u8] = b"ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar uint vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n9 9 9\n3 0 1 2\n";
#[cfg(feature = "raw-sys")]
const SPARSE_UV_GLTF: &[u8] = br#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAAAQAAAAAAAAAAAAABAQAAAAAAAAAAAAAAAQAAAgD8AAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/", "byteLength": 120 }],
  "bufferViews": [{ "buffer": 0, "byteLength": 72 }, { "buffer": 0, "byteOffset": 72, "byteLength": 48 }],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 6, "type": "VEC3", "min": [0,0,0], "max": [3,1,0] },
    { "bufferView": 1, "componentType": 5126, "count": 6, "type": "VEC2" },
    { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }
  ],
  "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0, "TEXCOORD_0": 2, "TEXCOORD_1": 1 } }] }],
  "nodes": [{ "mesh": 0 }], "scenes": [{ "nodes": [0] }], "scene": 0
}"#;
const SPLIT_LARGE_MESHES: u32 = 0x80;
const SPLIT_BY_BONE_COUNT: u32 = 0x0200_0000;
const CHILD_ENV: &str = "ASSET_IMPORTER_POSTPROCESS_GUARD_CHILD";

fn run_isolated(test_name: &str, check: impl FnOnce()) {
    if std::env::var(CHILD_ENV).ok().as_deref() == Some(test_name) {
        check();
        return;
    }

    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test_name, "--nocapture"])
        .env(CHILD_ENV, test_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn isolated post-processing regression");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let output = child.wait_with_output().unwrap();
            assert!(
                status.success(),
                "{test_name} failed:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!(
                "{test_name} did not terminate:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn unsupported_split_flags_are_retained_and_rejected() {
    for (bits, name, other) in [
        (SPLIT_LARGE_MESHES, "SplitLargeMeshes", "SplitByBoneCount"),
        (SPLIT_BY_BONE_COUNT, "SplitByBoneCount", "SplitLargeMeshes"),
    ] {
        let split = PostProcessSteps::from_raw(bits);
        assert_eq!(split.bits(), bits);
        let message = split.validate().unwrap_err();
        assert!(message.contains(name), "{message}");
        assert!(!message.contains(other), "{message}");
        let error = Importer::new()
            .read_from_memory(TRIANGLE)
            .with_memory_hint("obj")
            .with_post_process(split)
            .import()
            .expect_err("unsupported bits must not be silently dropped");
        assert!(matches!(error, Error::InvalidParameter { .. }));
        let error = Importer::new()
            .import_from_memory(TRIANGLE, Some("obj"))
            .unwrap()
            .apply_postprocess(split)
            .expect_err("later processing must also reject unsupported bits");
        assert!(matches!(error, Error::InvalidParameter { .. }));
    }
    let combined = SPLIT_LARGE_MESHES | SPLIT_BY_BONE_COUNT | PostProcessSteps::TRIANGULATE.bits();
    assert_eq!(PostProcessSteps::from_raw(combined).bits(), combined);
    assert!(PostProcessSteps::from_raw(combined).validate().is_err());

    for preset in [
        PostProcessSteps::default(),
        PostProcessSteps::FAST,
        PostProcessSteps::QUALITY,
        PostProcessSteps::MAX_QUALITY,
        PostProcessSteps::REALTIME,
        PostProcessSteps::TARGET_REALTIME_LEFT_HANDED,
    ] {
        assert_eq!(
            preset.bits() & (SPLIT_LARGE_MESHES | SPLIT_BY_BONE_COUNT),
            0
        );
        let scene = Importer::new()
            .read_from_memory(TRIANGLE)
            .with_memory_hint("obj")
            .with_post_process(preset)
            .import()
            .expect("supported presets continue to import");
        assert_eq!(
            scene.meshes().map(|mesh| mesh.num_faces()).sum::<usize>(),
            1
        );
    }
}

#[cfg(feature = "raw-sys")]
#[allow(clippy::unnecessary_cast)]
fn check_native_split_guard() {
    use std::ffi::{CStr, CString};

    use asset_importer::sys;

    assert_eq!(
        SPLIT_LARGE_MESHES,
        sys::aiPostProcessSteps::aiProcess_SplitLargeMeshes.0 as u32
    );
    assert_eq!(
        SPLIT_BY_BONE_COUNT,
        sys::aiPostProcessSteps::aiProcess_SplitByBoneCount.0 as u32
    );
    let source = Importer::new()
        .read_from_memory(UNUSED_VERTEX_PLY)
        .with_memory_hint("ply")
        .with_post_process(PostProcessSteps::empty())
        .import()
        .unwrap();
    assert_eq!(source.mesh(0).unwrap().num_vertices(), 4);
    assert_eq!(source.mesh(0).unwrap().num_faces(), 1);
    let sparse = Importer::new()
        .read_from_memory(SPARSE_UV_GLTF)
        .with_memory_hint("gltf")
        .with_post_process(PostProcessSteps::empty())
        .import()
        .unwrap();
    let sparse_mesh = sparse.mesh(0).unwrap();
    assert_eq!(sparse_mesh.num_vertices(), 6);
    assert!(!sparse_mesh.has_texture_coords(0));
    assert!(sparse_mesh.has_texture_coords(1));
    for (flags, key, name) in [
        (
            SPLIT_LARGE_MESHES,
            "PP_SLM_VERTEX_LIMIT",
            "SplitLargeMeshes",
        ),
        (SPLIT_BY_BONE_COUNT, "PP_SBBC_MAX_BONES", "SplitByBoneCount"),
    ] {
        let key = CString::new(key).unwrap();
        let expected = format!("{name} is unsupported");
        let owner = unsafe {
            sys::aiImportFileRust(
                c"missing-split-guard-input.gltf".as_ptr(),
                flags,
                std::ptr::null(),
                std::ptr::null(),
                0,
                None,
                std::ptr::null_mut(),
            )
        };
        assert!(owner.is_null());
        let message = unsafe { sys::aiGetLastErrorStringRust() };
        assert!(!message.is_null());
        assert!(
            unsafe { CStr::from_ptr(message) }
                .to_string_lossy()
                .starts_with(&expected),
            "split rejection must precede file access"
        );

        for (data, hint, limit) in [
            (UNUSED_VERTEX_PLY, "ply", 0),
            (UNUSED_VERTEX_PLY, "ply", 1),
            (UNUSED_VERTEX_PLY, "ply", 2),
            (UNUSED_VERTEX_PLY, "ply", 3),
            (UNUSED_VERTEX_PLY, "ply", -1),
            (SPARSE_UV_GLTF, "gltf", 3),
        ] {
            let hint = CString::new(hint).unwrap();
            let property = sys::aiRustProperty {
                name: key.as_ptr(),
                kind: sys::aiRustPropertyKind::aiRustPropertyKind_Integer,
                int_value: limit,
                ..Default::default()
            };
            let import = |steps| unsafe {
                sys::aiImportFileFromMemoryRust(
                    data.as_ptr().cast(),
                    data.len() as u32,
                    steps,
                    hint.as_ptr(),
                    std::ptr::null(),
                    &property,
                    1,
                    None,
                    std::ptr::null_mut(),
                )
            };
            let owner = import(flags);
            assert!(owner.is_null(), "bridge must reject native splitting");
            let message = unsafe { sys::aiGetLastErrorStringRust() };
            assert!(!message.is_null());
            assert!(
                unsafe { CStr::from_ptr(message) }
                    .to_string_lossy()
                    .starts_with(&expected)
            );

            let owner = import(0);
            assert!(!owner.is_null());
            let scene = unsafe { sys::aiGetSceneRust(owner) };
            assert!(!scene.is_null());
            let rejected = !unsafe { sys::aiApplyPostProcessingRust(owner, flags) };
            let message = unsafe { sys::aiGetLastErrorStringRust() };
            assert!(!message.is_null());
            let message = unsafe { CStr::from_ptr(message) }
                .to_string_lossy()
                .into_owned();
            let intact = unsafe { sys::aiGetSceneRust(owner) } == scene;
            unsafe { sys::aiReleaseSceneRust(owner) };
            assert!(rejected, "later native processing must reject splitting");
            assert!(message.starts_with(&expected), "{message}");
            assert!(intact, "rejection must precede scene mutation");
        }
    }
}

#[cfg(feature = "raw-sys")]
#[test]
fn native_split_is_rejected_before_reading_or_mutating_a_scene() {
    run_isolated(
        "native_split_is_rejected_before_reading_or_mutating_a_scene",
        check_native_split_guard,
    );
}

fn normal_only_morph_import(from_memory: bool) -> ImportBuilder {
    if from_memory {
        Importer::new()
            .read_from_memory(NORMAL_ONLY_MORPH_GLTF)
            .with_memory_hint("gltf")
    } else {
        let mut file_system = MemoryFileSystem::new();
        file_system.add_file("normal-only-morph.gltf", NORMAL_ONLY_MORPH_GLTF.to_vec());
        Importer::new()
            .read_file("normal-only-morph.gltf")
            .with_file_system(file_system)
    }
}

fn assert_normal_only_morph(scene: &Scene) {
    let mesh = scene.mesh(0).expect("morph fixture mesh");
    assert_eq!(mesh.num_vertices(), 3);
    assert_eq!(mesh.vertices_raw()[1].x, 1.0);
    assert_eq!(mesh.vertices_raw()[2].y, 1.0);
    assert_eq!(mesh.normals_raw()[0].z, 1.0);
    let target = mesh.anim_mesh(0).expect("normal-only morph target");
    assert_eq!(target.num_vertices(), 3);
    assert!(target.vertices_raw_opt().is_none());
    assert_eq!(target.normals_raw().len(), 3);
    assert_eq!(target.normals_raw()[0].y, 1.0);
    assert_eq!(target.normals_raw()[0].z, 1.0);
}

fn assert_scale_error(error: Error, apply_later: bool, expected: &str) {
    let message = match (apply_later, error) {
        (false, Error::ImportFailed { message }) | (true, Error::InvalidScene { message }) => {
            message
        }
        (_, error) => panic!("expected a native scaling error, got {error:?}"),
    };
    assert!(message.contains(expected), "{message}");
}

#[test]
fn global_scale_preserves_normal_only_morphs_when_inactive() {
    run_isolated(
        "global_scale_preserves_normal_only_morphs_when_inactive",
        || {
            for from_memory in [false, true] {
                for flags in [
                    PostProcessSteps::empty(),
                    PostProcessSteps::GLOBAL_SCALE,
                    PostProcessSteps::GLOBAL_SCALE | PostProcessSteps::VALIDATE_DATA_STRUCTURE,
                ] {
                    let scene = normal_only_morph_import(from_memory)
                        .with_property_float("GLOBAL_SCALE_FACTOR", 1.0)
                        .with_post_process(flags)
                        .import()
                        .expect("normal-only morphs support imports without effective scaling");
                    assert_normal_only_morph(&scene);
                    let scene = scene
                        .apply_postprocess(PostProcessSteps::GLOBAL_SCALE)
                        .expect("unit scaling must preserve normal-only morphs");
                    assert_normal_only_morph(&scene);
                }
            }
        },
    );
}

#[test]
fn global_scale_rejects_normal_only_morphs_before_initial_and_later_processing() {
    run_isolated(
        "global_scale_rejects_normal_only_morphs_before_initial_and_later_processing",
        || {
            for from_memory in [false, true] {
                for apply_later in [false, true] {
                    let flags = if apply_later {
                        PostProcessSteps::empty()
                    } else {
                        PostProcessSteps::GLOBAL_SCALE | PostProcessSteps::VALIDATE_DATA_STRUCTURE
                    };
                    let imported = normal_only_morph_import(from_memory)
                        .with_property_float("GLOBAL_SCALE_FACTOR", 2.0)
                        .with_post_process(flags)
                        .import();
                    let error = if apply_later {
                        let scene =
                            imported.expect("the scale property alone does not modify morphs");
                        assert_normal_only_morph(&scene);
                        scene
                            .apply_postprocess(PostProcessSteps::GLOBAL_SCALE)
                            .expect_err("reject scaling a morph without positions")
                    } else {
                        imported.expect_err("reject scaling before initial post-processing")
                    };
                    assert_scale_error(error, apply_later, "mesh 0, morph target 0");
                }
            }
        },
    );
}

#[test]
fn global_scale_rejects_nonfinite_and_zero_final_values() {
    run_isolated(
        "global_scale_rejects_nonfinite_and_zero_final_values",
        || {
            for scale in [0.0, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                for apply_later in [false, true] {
                    let flags = if apply_later {
                        PostProcessSteps::empty()
                    } else {
                        PostProcessSteps::GLOBAL_SCALE
                    };
                    let imported = ImportBuilder::new()
                        .with_post_process(flags)
                        .with_property_float("GLOBAL_SCALE_FACTOR", scale)
                        .import_from_memory(TRIANGLE, Some("obj"));
                    let error = if apply_later {
                        imported
                            .expect("unused scale properties do not invalidate an import")
                            .apply_postprocess(PostProcessSteps::GLOBAL_SCALE)
                            .expect_err("reject invalid effective scaling before later processing")
                    } else {
                        imported.expect_err("reject invalid effective scaling during import")
                    };
                    assert_scale_error(error, apply_later, "finite, nonzero effective scale");

                    let imported = ImportBuilder::new()
                        .with_post_process(flags)
                        .with_property_float("GLOBAL_SCALE_FACTOR", scale)
                        .with_property_float("GLOBAL_SCALE_FACTOR", 2.0)
                        .with_property_float("APP_SCALE_FACTOR", 0.0)
                        .import_from_memory(TRIANGLE, Some("obj"))
                        .expect(
                            "use the final user value and importer-populated application scale",
                        );
                    let scene = if apply_later {
                        imported
                            .apply_postprocess(PostProcessSteps::GLOBAL_SCALE)
                            .expect("later scaling uses the retained final properties")
                    } else {
                        imported
                    };
                    let mesh = scene.mesh(0).unwrap();
                    assert_eq!(mesh.vertices_raw()[1].x, 2.0);
                    assert_eq!(mesh.vertices_raw()[2].y, 2.0);
                }
            }
        },
    );
}

#[test]
fn global_scale_preserves_position_morph_support() {
    run_isolated("global_scale_preserves_position_morph_support", || {
        let gltf = std::str::from_utf8(NORMAL_ONLY_MORPH_GLTF)
            .unwrap()
            .replace("\"NORMAL\": 2", "\"POSITION\": 2");
        for apply_later in [false, true] {
            let flags = if apply_later {
                PostProcessSteps::empty()
            } else {
                PostProcessSteps::GLOBAL_SCALE
            };
            let imported = ImportBuilder::new()
                .with_post_process(flags)
                .with_property_float("GLOBAL_SCALE_FACTOR", 2.0)
                .import_from_memory(gltf.as_bytes(), Some("gltf"))
                .expect("position morphs still support scaling");
            let scene = if apply_later {
                imported
                    .apply_postprocess(PostProcessSteps::GLOBAL_SCALE)
                    .unwrap()
            } else {
                imported
            };
            let mesh = scene.mesh(0).unwrap();
            assert_eq!(mesh.vertices_raw()[1].x, 2.0);
            assert_eq!(mesh.vertices_raw()[2].y, 2.0);
            let target = mesh.anim_mesh(0).unwrap();
            assert_eq!(target.vertices_raw()[1].x, 2.0);
            assert_eq!(target.vertices_raw()[1].y, 2.0);
        }
    });
}

#[test]
fn deferred_global_scale_preserves_cancellation_and_io_errors() {
    use asset_importer::io::{FileStream, FileSystem};

    #[derive(Debug)]
    struct DeniedSidecar(MemoryFileSystem);

    impl FileSystem for DeniedSidecar {
        fn exists(&self, path: &str) -> bool {
            self.0.exists(path)
        }

        fn open(&self, path: &str) -> asset_importer::Result<Box<dyn FileStream>> {
            if path.ends_with(".mtl") {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "original sidecar error before scaling",
                )
                .into());
            }
            self.0.open(path)
        }
    }

    run_isolated(
        "deferred_global_scale_preserves_cancellation_and_io_errors",
        || {
            let error = ImportBuilder::new()
                .with_post_process(PostProcessSteps::GLOBAL_SCALE)
                .with_property_float("GLOBAL_SCALE_FACTOR", 0.0)
                .with_progress_handler_fn(|_, _| false)
                .import_from_memory(TRIANGLE, Some("obj"))
                .expect_err("cancellation must precede deferred scale rejection");
            assert_scale_error(error, false, "cancelled by progress handler");

            let mut files = MemoryFileSystem::new();
            files.add_file(
                "triangle.obj",
                [b"mtllib denied.mtl\n".as_slice(), TRIANGLE].concat(),
            );
            let error = Importer::new()
                .read_file("triangle.obj")
                .with_file_system(DeniedSidecar(files))
                .with_post_process(PostProcessSteps::GLOBAL_SCALE)
                .with_property_float("GLOBAL_SCALE_FACTOR", 0.0)
                .import()
                .expect_err("the first actual I/O error must precede scale rejection");
            let Error::IoError { source } = error else {
                panic!("expected the original I/O error, got {error:?}");
            };
            assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
            assert_eq!(source.to_string(), "original sidecar error before scaling");
        },
    );
}

#[cfg(feature = "raw-sys")]
#[test]
fn native_global_scale_rejection_preserves_scene_data() {
    use std::ffi::CStr;

    use asset_importer::sys;

    run_isolated("native_global_scale_rejection_preserves_scene_data", || {
        let flags = (PostProcessSteps::GLOBAL_SCALE | PostProcessSteps::MAKE_LEFT_HANDED).bits();
        for scale in [0.0, f32::NAN, f32::INFINITY, 2.0] {
            let property = sys::aiRustProperty {
                name: c"GLOBAL_SCALE_FACTOR".as_ptr(),
                kind: sys::aiRustPropertyKind::aiRustPropertyKind_Float,
                float_value: scale,
                ..Default::default()
            };
            let import = |steps| unsafe {
                sys::aiImportFileFromMemoryRust(
                    NORMAL_ONLY_MORPH_GLTF.as_ptr().cast(),
                    NORMAL_ONLY_MORPH_GLTF.len() as u32,
                    steps,
                    c"gltf".as_ptr(),
                    std::ptr::null(),
                    &property,
                    1,
                    None,
                    std::ptr::null_mut(),
                )
            };
            let expected = if scale == 2.0 {
                "no position channel"
            } else {
                "finite, nonzero effective scale"
            };
            let rejected = import(flags);
            assert!(rejected.is_null(), "reject scaling during native import");
            let message = unsafe { sys::aiGetLastErrorStringRust() };
            assert!(!message.is_null());
            assert!(
                unsafe { CStr::from_ptr(message) }
                    .to_string_lossy()
                    .contains(expected)
            );

            let owner = import(0);
            assert!(!owner.is_null());
            let scene = unsafe { sys::aiGetSceneRust(owner) };
            assert!(!scene.is_null());
            let snapshot = || unsafe {
                let mesh = &**(*scene).mMeshes;
                let target = &**mesh.mAnimMeshes;
                (
                    (*scene).mFlags,
                    mesh.mNumVertices,
                    target.mNumVertices,
                    target.mVertices,
                    std::slice::from_raw_parts(mesh.mVertices, mesh.mNumVertices as usize)
                        .iter()
                        .map(|vertex| [vertex.x, vertex.y, vertex.z])
                        .collect::<Vec<_>>(),
                    std::slice::from_raw_parts(mesh.mNormals, mesh.mNumVertices as usize)
                        .iter()
                        .map(|normal| [normal.x, normal.y, normal.z])
                        .collect::<Vec<_>>(),
                    std::slice::from_raw_parts(target.mNormals, target.mNumVertices as usize)
                        .iter()
                        .map(|normal| [normal.x, normal.y, normal.z])
                        .collect::<Vec<_>>(),
                )
            };
            let before = snapshot();
            assert!(!unsafe { sys::aiApplyPostProcessingRust(owner, flags) });
            let message = unsafe { sys::aiGetLastErrorStringRust() };
            assert!(!message.is_null());
            assert!(
                unsafe { CStr::from_ptr(message) }
                    .to_string_lossy()
                    .contains(expected)
            );
            assert_eq!(unsafe { sys::aiGetSceneRust(owner) }, scene);
            assert_eq!(
                snapshot(),
                before,
                "rejection must precede all scene mutation"
            );
            unsafe { sys::aiReleaseSceneRust(owner) };
        }
    });
}

#[cfg(all(feature = "export", feature = "raw-sys"))]
#[test]
fn native_bridge_rejects_export_preprocessing_before_opening_output() {
    use std::ffi::{CStr, CString};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use asset_importer::io::{AssimpFileIO, FileStream, FileSystem};
    use asset_importer::sys;

    #[derive(Debug)]
    struct UnusedOutput(Arc<AtomicUsize>);

    impl FileSystem for UnusedOutput {
        fn exists(&self, _: &str) -> bool {
            false
        }

        fn open(&self, _: &str) -> asset_importer::Result<Box<dyn FileStream>> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Err(std::io::Error::other("output must not be opened").into())
        }

        fn open_with_mode(
            &self,
            path: &str,
            _: &str,
        ) -> asset_importer::Result<Box<dyn FileStream>> {
            self.open(path)
        }
    }

    let opens = Arc::new(AtomicUsize::new(0));
    let file_system = Arc::new(Mutex::new(UnusedOutput(opens.clone())));
    let io = AssimpFileIO::new(file_system).create_ai_file_io();
    let scene = Importer::new()
        .import_from_memory(TRIANGLE, Some("obj"))
        .unwrap();
    let format = CString::new("obj").unwrap();
    let path = CString::new("unused-output.obj").unwrap();
    let flags = PostProcessSteps::GLOBAL_SCALE.bits();
    let status = unsafe {
        sys::aiExportSceneExWithPropertiesRust(
            scene.as_raw(),
            format.as_ptr(),
            path.as_ptr(),
            io.as_ptr(),
            flags,
            std::ptr::null(),
            0,
        )
    };
    assert_eq!(status, sys::aiReturn::aiReturn_FAILURE);
    assert_eq!(opens.load(Ordering::Relaxed), 0);
    let message_ptr = unsafe { sys::aiGetLastErrorStringRust() };
    assert!(
        !message_ptr.is_null(),
        "bridge failure must include a diagnostic"
    );
    let message = unsafe { CStr::from_ptr(message_ptr) }.to_string_lossy();
    assert!(
        message.contains("Export preprocessing is unsupported"),
        "{message}"
    );

    let blob = unsafe {
        sys::aiExportSceneToBlobWithPropertiesRust(
            scene.as_raw(),
            format.as_ptr(),
            flags,
            std::ptr::null(),
            0,
        )
    };
    assert!(blob.is_null());
    let message_ptr = unsafe { sys::aiGetLastErrorStringRust() };
    assert!(
        !message_ptr.is_null(),
        "bridge failure must include a diagnostic"
    );
    let message = unsafe { CStr::from_ptr(message_ptr) }.to_string_lossy();
    assert!(
        message.contains("Export preprocessing is unsupported"),
        "{message}"
    );
    assert_eq!(scene.mesh(0).unwrap().vertices_raw()[1].x, 1.0);
}
