#![cfg(feature = "export")]

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use asset_importer::exporter::ExportBuilder;
use asset_importer::io::{FileStream, FileSystem};
use asset_importer::{Error, Importer, Scene};

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
    let message = export_message(
        ExportBuilder::new("stl")
            .export_to_file(&scene, &path)
            .expect_err("cannot open an output under a missing directory"),
    );
    assert!(
        message.contains("could not open output .stl file"),
        "{message}"
    );
    assert!(message.contains("triangle.stl"), "{message}");
    assert!(!message.contains(format), "{message}");
    assert!(!path.exists());
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
