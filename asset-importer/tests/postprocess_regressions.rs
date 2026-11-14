#[cfg(feature = "raw-sys")]
use std::process::{Command, Stdio};
#[cfg(feature = "raw-sys")]
use std::time::{Duration, Instant};

use asset_importer::postprocess::PostProcessSteps;
use asset_importer::{Error, Importer};

const TRIANGLE: &[u8] = b"o tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
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
#[cfg(feature = "raw-sys")]
const CHILD_ENV: &str = "ASSET_IMPORTER_SPLIT_GUARD_CHILD";

#[test]
fn unsupported_split_flags_are_retained_and_rejected() {
    let split = PostProcessSteps::from_raw(SPLIT_LARGE_MESHES);
    assert_eq!(split.bits(), SPLIT_LARGE_MESHES);
    assert!(split.validate().unwrap_err().contains("SplitLargeMeshes"));
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

    for preset in [
        PostProcessSteps::default(),
        PostProcessSteps::FAST,
        PostProcessSteps::QUALITY,
        PostProcessSteps::MAX_QUALITY,
        PostProcessSteps::REALTIME,
        PostProcessSteps::TARGET_REALTIME_LEFT_HANDED,
    ] {
        assert_eq!(preset.bits() & SPLIT_LARGE_MESHES, 0);
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
    let key = CString::new("PP_SLM_VERTEX_LIMIT").unwrap();
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
        let import = |flags| unsafe {
            sys::aiImportFileFromMemoryRust(
                data.as_ptr().cast(),
                data.len() as u32,
                flags,
                hint.as_ptr(),
                std::ptr::null(),
                &property,
                1,
                None,
                std::ptr::null_mut(),
            )
        };
        let owner = import(SPLIT_LARGE_MESHES);
        assert!(owner.is_null(), "bridge must reject native splitting");
        let message = unsafe { sys::aiGetLastErrorStringRust() };
        assert!(!message.is_null());
        assert!(
            unsafe { CStr::from_ptr(message) }
                .to_bytes()
                .starts_with(b"SplitLargeMeshes is unsupported")
        );

        let owner = import(0);
        assert!(!owner.is_null());
        let scene = unsafe { sys::aiGetSceneRust(owner) };
        assert!(!scene.is_null());
        let rejected = !unsafe { sys::aiApplyPostProcessingRust(owner, SPLIT_LARGE_MESHES) };
        let intact = unsafe { sys::aiGetSceneRust(owner) } == scene;
        unsafe { sys::aiReleaseSceneRust(owner) };
        assert!(rejected, "later native processing must reject splitting");
        assert!(intact, "rejection must precede scene mutation");
    }
}

#[cfg(feature = "raw-sys")]
#[test]
fn native_split_is_rejected_before_reading_or_mutating_a_scene() {
    if std::env::var_os(CHILD_ENV).is_some() {
        check_native_split_guard();
        return;
    }

    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "native_split_is_rejected_before_reading_or_mutating_a_scene",
            "--nocapture",
        ])
        .env(CHILD_ENV, "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn isolated native splitting regression");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let output = child.wait_with_output().unwrap();
            assert!(
                status.success(),
                "split guard regression failed:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!(
                "native splitting did not terminate:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
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
