#![cfg(feature = "export")]

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use asset_importer::exporter::{ExportBuilder, formats};
use asset_importer::io::{FileStream, FileSystem};
use asset_importer::postprocess::PostProcessSteps;
use asset_importer::{Error, Importer, PropertyStore, Scene};

const OBJ: &[u8] = b"o tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
const CHILD_ENV: &str = "ASSET_IMPORTER_3MF_GUARD_CHILD";

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "asset-importer-export-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn triangle() -> Scene {
    Importer::new()
        .import_from_memory(OBJ, Some("obj"))
        .expect("import triangle")
}

fn export_message(error: Error) -> String {
    match error {
        Error::ExportFailed { message } => message,
        error => panic!("expected ExportFailed, got {error}"),
    }
}

#[test]
fn default_exports_write_real_output_with_default_properties() {
    let dir = TempDir::new();
    let path = dir.0.join("triangle.stl");
    let scene = triangle();
    let source_flags = scene.flags();
    let source_mesh = scene.mesh(0).unwrap();
    let source_vertices = source_mesh.vertices_raw();
    assert!(source_mesh.normals_raw().is_empty());

    ExportBuilder::new("stl")
        .export_to_file(&scene, &path)
        .expect("export without custom I/O or properties");
    let file = std::fs::read(&path).unwrap();
    let blob = ExportBuilder::new("stl")
        .export_to_blob(&scene)
        .expect("export blob without properties");

    assert!(file.starts_with(b"solid "));
    assert_eq!(blob.data(), file.as_slice());
    assert_eq!(scene.flags(), source_flags);
    assert!(source_mesh.normals_raw().is_empty());
    assert!(source_vertices.iter().any(|v| v.x == 1.0));
    let round_trip = Importer::new()
        .import_from_memory(blob.data(), Some("stl"))
        .expect("read exported STL");
    let mesh = round_trip.mesh(0).unwrap();
    assert_eq!(mesh.num_vertices(), 3);
    assert_eq!(mesh.num_faces(), 1);
    assert!(mesh.vertices_raw().iter().any(|v| v.x == 1.0));
    assert!(mesh.vertices_raw().iter().any(|v| v.y == 1.0));
    assert!(mesh.vertices_raw().iter().all(|v| v.z == 0.0));
}

#[test]
fn default_exports_report_exporter_errors_without_stale_import_messages() {
    let dir = TempDir::new();
    let scene = triangle();
    let stale = Importer::new()
        .read_from_memory(b"")
        .with_memory_hint("obj")
        .import()
        .expect_err("seed an unrelated import error")
        .to_string();
    assert!(stale.contains("Memory buffer is empty"), "{stale}");

    let format = "missing-regression-exporter";
    let file_error = ExportBuilder::new(format)
        .export_to_file(&scene, dir.0.join("unsupported.out"))
        .expect_err("unsupported file export");
    let blob_error = ExportBuilder::new(format)
        .export_to_blob(&scene)
        .err()
        .expect("unsupported blob export");
    for error in [file_error, blob_error] {
        let message = export_message(error);
        assert!(message.contains("Found no exporter"), "{message}");
        assert!(message.contains(format), "{message}");
        assert!(!message.contains("Memory buffer is empty"), "{message}");
    }
    assert!(!dir.0.join("unsupported.out").exists());

    let path = dir.0.join("missing-directory").join("triangle.stl");
    let error = ExportBuilder::new("stl")
        .export_to_file(&scene, &path)
        .expect_err("cannot open an output under a missing directory");
    let Error::IoError { source } = error else {
        panic!("expected original I/O error, got {error}");
    };
    assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
    assert!(!path.exists());
}

#[test]
fn collada_constant_exports_the_runtime_format() {
    let Some(format) =
        asset_importer::get_export_formats_iter().find(|format| format.id == "collada")
    else {
        return;
    };
    assert_eq!(formats::COLLADA, format.id);
    assert_eq!(format.file_extension, "dae");

    let blob = ExportBuilder::new(formats::COLLADA)
        .export_to_blob(&triangle())
        .expect("export COLLADA using its runtime format ID");
    let xml = std::str::from_utf8(blob.data()).expect("COLLADA XML");
    assert!(xml.contains("<COLLADA"), "{xml}");
}

#[test]
fn scene_preparation_uses_importer_properties_before_export() {
    for prepare_after_import in [false, true] {
        let preparation = PostProcessSteps::GLOBAL_SCALE;
        let steps = if prepare_after_import {
            PostProcessSteps::TRIANGULATE
        } else {
            PostProcessSteps::TRIANGULATE | preparation
        };
        let scene = Importer::new()
            .read_from_memory(OBJ)
            .with_memory_hint("obj")
            .with_property_float("GLOBAL_SCALE_FACTOR", 0.25)
            .with_post_process(steps)
            .import()
            .expect("import with scene preparation properties");
        let scene = if prepare_after_import {
            scene
                .apply_postprocess(preparation)
                .expect("prepare an exclusively owned scene with its importer")
        } else {
            scene
        };

        let blob = ExportBuilder::new("assbin")
            .export_to_blob(&scene)
            .expect("export the prepared scene");
        let imported = Importer::new()
            .import_from_memory(blob.data(), Some("assbin"))
            .expect("read back the prepared scene");
        let mesh = imported.mesh(0).unwrap();
        assert!(mesh.vertices_raw().iter().any(|vertex| vertex.x == 0.25));
        assert!(mesh.vertices_raw().iter().any(|vertex| vertex.y == 0.25));
        assert!(mesh.vertices_raw().iter().all(|vertex| vertex.x <= 0.25));
    }
}

#[test]
fn export_properties_encode_only_current_assignments() {
    for merge_by_reference in [false, true] {
        let mut store = PropertyStore::new();
        store.set_string("EXPORT_BLOB_NAME", "current.obj");
        let builder = ExportBuilder::new(formats::OBJ)
            .with_property_string("EXPORT_BLOB_NAME", "superseded\0name");
        let builder = if merge_by_reference {
            builder.with_property_store_ref(&store)
        } else {
            builder.with_property_store(store)
        };
        let blob = builder
            .export_to_blob(&triangle())
            .expect("encode only the current export property value");

        assert_eq!(blob.name(), "current.obj");
        let obj = std::str::from_utf8(blob.data()).unwrap();
        assert!(obj.contains("current.mtl"), "{obj}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn default_file_exports_report_write_failures() {
    let error = ExportBuilder::new("stl")
        .export_to_file(&triangle(), "/dev/full")
        .expect_err("the default output file system must observe a full device");
    assert!(matches!(error, Error::IoError { .. }), "{error}");
}

#[cfg(any(unix, windows))]
#[test]
fn non_utf8_export_paths_preserve_assets_at_the_lossy_path() {
    let directory = TempDir::new();
    let scene = triangle();
    for format in ["stl", "3mf"] {
        #[cfg(unix)]
        let filename = {
            use std::os::unix::ffi::OsStringExt;
            let mut bytes = b"asset-\xff.".to_vec();
            bytes.extend_from_slice(format.as_bytes());
            std::ffi::OsString::from_vec(bytes)
        };
        #[cfg(windows)]
        let filename = {
            use std::os::windows::ffi::OsStringExt;
            let mut wide: Vec<u16> = "asset-".encode_utf16().collect();
            wide.push(0xd800);
            wide.extend(format!(".{format}").encode_utf16());
            std::ffi::OsString::from_wide(&wide)
        };
        let path = directory.0.join(filename);
        let lossy_path = directory.0.join(format!("asset-\u{fffd}.{format}"));
        let sentinel = b"another asset: do not overwrite";
        std::fs::write(&lossy_path, sentinel).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));

        for custom_io in [false, true] {
            let builder = ExportBuilder::new(format);
            let builder = if custom_io {
                builder.with_file_system(CountingFs {
                    calls: calls.clone(),
                })
            } else {
                builder
            };
            let error = builder
                .export_to_file(&scene, &path)
                .expect_err("reject the original path before any lossy conversion");
            assert!(matches!(error, Error::InvalidParameter { .. }), "{error}");
            assert_eq!(std::fs::read(&lossy_path).unwrap(), sentinel);
            assert!(!path.exists());
        }
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }
}

fn triangle_with_node_extras(extras: &str) -> Scene {
    let gltf = format!(
        r#"{{
          "asset": {{ "version": "2.0" }},
          "buffers": [{{
            "uri": "data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAA",
            "byteLength": 36
          }}],
          "bufferViews": [{{ "buffer": 0, "byteLength": 36 }}],
          "accessors": [{{
            "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
            "min": [0,0,0], "max": [1,1,0]
          }}],
          "meshes": [{{ "primitives": [{{ "attributes": {{ "POSITION": 0 }} }}] }}],
          "nodes": [{{ "name": "source", "mesh": 0, "extras": {extras} }}],
          "scenes": [{{ "nodes": [0] }}], "scene": 0
        }}"#
    );
    Scene::from_memory(gltf.as_bytes(), Some("gltf")).unwrap()
}

#[test]
fn empty_node_metadata_is_rejected_without_freeing_source() {
    let scene = triangle_with_node_extras(r#"{"unused": null}"#);
    let root = scene.root_node().unwrap();
    let source = if root.name() == "source" {
        root
    } else {
        root.children()
            .find(|node| node.name() == "source")
            .unwrap()
    };
    assert!(source.metadata().unwrap().is_empty());
    let flags = scene.flags();
    let directory = TempDir::new();
    for format in ["assbin", "missing-regression-exporter"] {
        let error = ExportBuilder::new(format)
            .export_to_blob(&scene)
            .err()
            .unwrap();
        assert!(export_message(error).contains("empty metadata"));
        let path = directory.0.join(format!("{format}.out"));
        let error = ExportBuilder::new(format)
            .export_to_file(&scene, &path)
            .unwrap_err();
        assert!(export_message(error).contains("empty metadata"));
        assert!(!path.exists());
        assert!(source.metadata().unwrap().is_empty());
        assert_eq!(source.name(), "source");
        assert_eq!(scene.flags(), flags);
    }
}

#[test]
fn assigned_node_metadata_survives_export_and_source_drop() {
    let scene = triangle_with_node_extras(r#"{"label": "retained"}"#);
    let blob = ExportBuilder::new("assbin").export_to_blob(&scene).unwrap();
    let imported = Importer::new()
        .import_from_memory(blob.data(), Some("assbin"))
        .unwrap();
    assert!(imported.root_node().is_some());
    assert!(ExportBuilder::new("assbin").export_to_blob(&scene).is_ok());
    drop(scene);
    assert_eq!(imported.num_meshes(), 1);
}

#[derive(Debug)]
struct CountingFs {
    calls: Arc<AtomicUsize>,
}

impl FileSystem for CountingFs {
    fn exists(&self, _path: &str) -> bool {
        self.calls.fetch_add(1, Ordering::Relaxed);
        true
    }

    fn open(&self, _path: &str) -> Result<Box<dyn FileStream>, Error> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Err(Error::io_error("unexpected custom I/O callback"))
    }

    fn open_with_mode(&self, path: &str, _mode: &str) -> Result<Box<dyn FileStream>, Error> {
        self.open(path)
    }
}

#[test]
fn blob_exports_reject_a_configured_file_system_without_using_it() {
    let calls = Arc::new(AtomicUsize::new(0));
    let error = ExportBuilder::new(formats::OBJ)
        .with_file_system(CountingFs {
            calls: calls.clone(),
        })
        .export_to_blob(&triangle())
        .err()
        .expect("blob exports cannot use a custom file system");

    assert!(matches!(error, Error::InvalidParameter { .. }), "{error}");
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[derive(Debug)]
struct FailingOutputFs {
    fail_write: bool,
    bytes_written: Arc<AtomicUsize>,
    flushes: Arc<AtomicUsize>,
}

impl FileSystem for FailingOutputFs {
    fn exists(&self, _path: &str) -> bool {
        false
    }

    fn open(&self, _path: &str) -> Result<Box<dyn FileStream>, Error> {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound).into())
    }

    fn open_with_mode(&self, _path: &str, _mode: &str) -> Result<Box<dyn FileStream>, Error> {
        Ok(Box::new(FailingOutputStream {
            fail_write: self.fail_write,
            position: 0,
            bytes_written: self.bytes_written.clone(),
            flushes: self.flushes.clone(),
        }))
    }
}

struct FailingOutputStream {
    fail_write: bool,
    position: usize,
    bytes_written: Arc<AtomicUsize>,
    flushes: Arc<AtomicUsize>,
}

impl FileStream for FailingOutputStream {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, Error> {
        Err(std::io::Error::from(std::io::ErrorKind::Unsupported).into())
    }

    fn write(&mut self, buffer: &[u8]) -> Result<usize, Error> {
        if self.fail_write && self.position > 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "original export write failure",
            )
            .into());
        }
        let written = buffer.len().min(7);
        self.position += written;
        self.bytes_written.fetch_add(written, Ordering::Relaxed);
        Ok(written)
    }

    fn tell(&self) -> Result<u64, Error> {
        Ok(self.position as u64)
    }

    fn seek(&mut self, position: u64) -> Result<(), Error> {
        self.position = usize::try_from(position)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        Ok(())
    }

    fn size(&self) -> Result<u64, Error> {
        Ok(self.position as u64)
    }

    fn flush(&mut self) -> Result<(), Error> {
        self.flushes.fetch_add(1, Ordering::Relaxed);
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "original export flush failure",
        )
        .into())
    }
}

#[test]
fn file_exports_preserve_the_first_write_or_flush_error() {
    for (fail_write, expected_kind, expected_message) in [
        (
            true,
            std::io::ErrorKind::PermissionDenied,
            "original export write failure",
        ),
        (
            false,
            std::io::ErrorKind::BrokenPipe,
            "original export flush failure",
        ),
    ] {
        let bytes_written = Arc::new(AtomicUsize::new(0));
        let flushes = Arc::new(AtomicUsize::new(0));
        let error = ExportBuilder::new("stl")
            .with_file_system(FailingOutputFs {
                fail_write,
                bytes_written: bytes_written.clone(),
                flushes: flushes.clone(),
            })
            .export_to_file(&triangle(), "virtual/triangle.stl")
            .expect_err("write and flush failures must override native success");
        let Error::IoError { source } = error else {
            panic!("expected original I/O error, got {error}");
        };

        assert_eq!(source.kind(), expected_kind);
        assert_eq!(source.to_string(), expected_message);
        assert!(flushes.load(Ordering::Relaxed) > 0);
        let written = bytes_written.load(Ordering::Relaxed);
        if fail_write {
            assert_eq!(written, 7);
        } else {
            assert!(written > 7);
        }
    }
}

#[test]
fn three_mf_guards_preserve_host_sentinels_in_an_isolated_process() {
    let dir = TempDir::new();
    let sentinels = ["$blobfile", "named-blob.3mf", "existing.3mf"];
    for name in sentinels {
        std::fs::write(dir.0.join(name), b"host sentinel: do not overwrite").unwrap();
    }

    // Only this subprocess uses a different working directory. Parallel tests in
    // the parent keep their original directory and cannot share these sentinels.
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "three_mf_guard_child", "--nocapture"])
        .env(CHILD_ENV, "1")
        .current_dir(&dir.0)
        .output()
        .expect("run isolated 3MF regression");
    assert!(
        output.status.success(),
        "3MF child failed: {}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    for name in sentinels {
        assert_eq!(
            std::fs::read(dir.0.join(name)).unwrap(),
            b"host sentinel: do not overwrite",
            "3MF changed {name}",
        );
    }
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), sentinels.len());
}

#[test]
fn three_mf_guard_child() {
    if std::env::var_os(CHILD_ENV).is_none() {
        return;
    }
    let scene = triangle();
    let default_blob_error = ExportBuilder::new("3mf")
        .export_to_blob(&scene)
        .err()
        .expect("3MF blob export must be rejected");
    let named_blob_error = ExportBuilder::new("3mf")
        .with_property_string("EXPORT_BLOB_NAME", "named-blob.3mf")
        .export_to_blob(&scene)
        .err()
        .expect("3MF named blob export must be rejected");
    for error in [default_blob_error, named_blob_error] {
        let message = export_message(error);
        assert!(message.contains("3MF"), "{message}");
        assert!(message.contains("blob"), "{message}");
        assert!(message.contains("host file system"), "{message}");
    }

    let calls = Arc::new(AtomicUsize::new(0));
    for path in ["existing.3mf", "new-output.3mf"] {
        let message = export_message(
            ExportBuilder::new("3mf")
                .with_file_system(CountingFs {
                    calls: calls.clone(),
                })
                .export_to_file(&scene, path)
                .expect_err("3MF must reject custom I/O before native export"),
        );
        assert!(message.contains("3MF"), "{message}");
        assert!(message.contains("custom file system"), "{message}");
        assert!(message.contains("host file system"), "{message}");
    }
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert!(!std::path::Path::new("new-output.3mf").exists());
}

#[test]
fn three_mf_native_file_export_remains_available() {
    if !asset_importer::exporter::Exporter::new().is_format_supported("3mf") {
        return;
    }
    let dir = TempDir::new();
    let path = dir.0.join("triangle.3mf");
    ExportBuilder::new("3mf")
        .export_to_file(&triangle(), &path)
        .expect("3MF can still export to an explicit native path");
    assert!(std::fs::read(path).unwrap().starts_with(b"PK\x03\x04"));
}
