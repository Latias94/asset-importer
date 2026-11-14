// Regression tests: never unwind across FFI callbacks.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use asset_importer::io::{FileStream, FileSystem, MemoryFileStream, MemoryFileSystem};
use asset_importer::postprocess::PostProcessSteps;
use asset_importer::{Error, Importer};

const OBJ: &[u8] = b"o tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
const PLY: &[u8] = b"ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n";

#[derive(Debug)]
struct PanicFs;

impl FileSystem for PanicFs {
    fn exists(&self, _path: &str) -> bool {
        true
    }

    fn open(&self, path: &str) -> Result<Box<dyn FileStream>, Error> {
        self.open_with_mode(path, "rb")
    }

    fn open_with_mode(&self, _path: &str, _mode: &str) -> Result<Box<dyn FileStream>, Error> {
        panic!("intentional panic in FileSystem::open_with_mode");
    }
}

#[test]
fn test_file_system_panic_does_not_unwind_over_ffi() {
    let importer = Importer::new();

    // The important property: this must not abort or unwind across the C ABI.
    // A panic inside the FileSystem should be caught and turned into an import error.
    let result = importer
        .read_file("does-not-matter.obj")
        .with_file_system(PanicFs)
        .import();

    assert!(result.is_err());
}

#[test]
fn progress_cancellation_is_latched() {
    let calls = Arc::new(AtomicUsize::new(0));
    let progress_calls = calls.clone();
    let result = Importer::new()
        .read_from_memory(OBJ)
        .with_memory_hint("obj")
        .with_progress_handler_fn(move |_, _| progress_calls.fetch_add(1, Ordering::SeqCst) != 0)
        .import();

    let error = result.expect_err("a cancelled import must not return a scene");
    assert!(error.to_string().contains("cancel"), "{error}");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn postprocess_cancellation_rejects_the_scene() {
    let cancelled = Arc::new(AtomicUsize::new(0));
    let progress_cancelled = cancelled.clone();
    let result = Importer::new()
        .read_from_memory(OBJ)
        .with_memory_hint("obj")
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .with_progress_handler_fn(move |_, message| {
            if message.is_some_and(|message| message.starts_with("post ")) {
                progress_cancelled.fetch_add(1, Ordering::SeqCst);
                false
            } else {
                true
            }
        })
        .import();

    assert!(result.is_err());
    assert_eq!(cancelled.load(Ordering::SeqCst), 1);
}

struct PanickingPayload;

impl Drop for PanickingPayload {
    fn drop(&mut self) {
        panic!("intentional panic while dropping a panic payload");
    }
}

#[test]
fn progress_panic_with_panicking_payload_is_an_import_error() {
    let result = Importer::new()
        .read_from_memory(OBJ)
        .with_memory_hint("obj")
        .with_progress_handler_fn(|_, _| std::panic::panic_any(PanickingPayload))
        .import();

    assert!(result.is_err());
}

#[derive(Clone, Copy, Debug)]
enum StreamFailure {
    None,
    ReadPanic,
    DropPanic,
    #[cfg(feature = "export")]
    WriteError,
    #[cfg(feature = "export")]
    WritePanic,
    #[cfg(feature = "export")]
    WriteZero,
    FlushError,
}

#[derive(Debug)]
struct CountingFs {
    opened: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
    failure: StreamFailure,
}

impl FileSystem for CountingFs {
    fn exists(&self, _path: &str) -> bool {
        true
    }

    fn open(&self, path: &str) -> Result<Box<dyn FileStream>, Error> {
        self.open_with_mode(path, "rb")
    }

    fn open_with_mode(&self, path: &str, mode: &str) -> Result<Box<dyn FileStream>, Error> {
        let data = if path.ends_with(".ply") { PLY } else { OBJ };
        let stream = if mode.starts_with('r') {
            MemoryFileStream::new(data.to_vec())
        } else {
            MemoryFileStream::new_writable(0)
        };
        self.opened.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(CountingStream {
            stream,
            closed: self.closed.clone(),
            failure: self.failure,
        }))
    }
}

struct CountingStream {
    stream: MemoryFileStream,
    closed: Arc<AtomicUsize>,
    failure: StreamFailure,
}

impl FileStream for CountingStream {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Error> {
        if matches!(self.failure, StreamFailure::ReadPanic) {
            panic!("intentional panic in FileStream::read");
        }
        // Exercise byte streams that return less than a full Assimp read request.
        let len = buffer.len().min(7);
        self.stream.read(&mut buffer[..len])
    }

    fn write(&mut self, buffer: &[u8]) -> Result<usize, Error> {
        #[cfg(feature = "export")]
        match self.failure {
            StreamFailure::WriteError => return Err(Error::io_error("intentional write failure")),
            StreamFailure::WritePanic => panic!("intentional panic in FileStream::write"),
            StreamFailure::WriteZero => return Ok(0),
            _ => {}
        }
        self.stream.write(&buffer[..buffer.len().min(7)])
    }

    fn tell(&self) -> Result<u64, Error> {
        self.stream.tell()
    }

    fn seek(&mut self, position: u64) -> Result<(), Error> {
        self.stream.seek(position)
    }

    fn size(&self) -> Result<u64, Error> {
        self.stream.size()
    }

    fn flush(&mut self) -> Result<(), Error> {
        if matches!(self.failure, StreamFailure::FlushError) {
            return Err(Error::io_error("intentional flush failure"));
        }
        Ok(())
    }
}

impl Drop for CountingStream {
    fn drop(&mut self) {
        self.closed.fetch_add(1, Ordering::SeqCst);
        if matches!(self.failure, StreamFailure::DropPanic) {
            std::panic::panic_any(PanickingPayload);
        }
    }
}

#[test]
fn bridge_closes_streams_deleted_by_assimp() {
    let opened = Arc::new(AtomicUsize::new(0));
    let closed = Arc::new(AtomicUsize::new(0));
    let scene = Importer::new()
        .read_file("triangle.ply")
        .with_file_system(CountingFs {
            opened: opened.clone(),
            closed: closed.clone(),
            failure: StreamFailure::None,
        })
        .with_progress_handler_fn(|_, _| true)
        .import()
        .expect("import PLY through the bridge");

    assert_eq!(scene.num_meshes(), 1);
    assert!(opened.load(Ordering::SeqCst) > 0);
    assert_eq!(opened.load(Ordering::SeqCst), closed.load(Ordering::SeqCst));
}

#[test]
fn stream_panics_are_errors_and_close_all_files() {
    for failure in [StreamFailure::ReadPanic, StreamFailure::DropPanic] {
        let opened = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicUsize::new(0));
        let result = Importer::new()
            .read_file("triangle.ply")
            .with_file_system(CountingFs {
                opened: opened.clone(),
                closed: closed.clone(),
                failure,
            })
            .import();

        assert!(result.is_err(), "{failure:?}");
        assert!(opened.load(Ordering::SeqCst) > 0);
        assert_eq!(opened.load(Ordering::SeqCst), closed.load(Ordering::SeqCst));
    }
}

#[test]
fn cancellation_closes_custom_io_streams() {
    let opened = Arc::new(AtomicUsize::new(0));
    let closed = Arc::new(AtomicUsize::new(0));
    let result = Importer::new()
        .read_file("triangle.ply")
        .with_file_system(CountingFs {
            opened: opened.clone(),
            closed: closed.clone(),
            failure: StreamFailure::None,
        })
        .with_progress_handler_fn(|_, _| false)
        .import();

    assert!(result.is_err());
    assert!(opened.load(Ordering::SeqCst) > 0);
    assert_eq!(opened.load(Ordering::SeqCst), closed.load(Ordering::SeqCst));
}

#[test]
fn custom_io_remains_alive_for_postprocessing() {
    use std::io::Write;
    struct HostFile(std::path::PathBuf);
    impl Drop for HostFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let texture_path = format!(
        "asset-importer-io-probe-{}-{}.png",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut host = std::fs::File::create_new(&texture_path).expect("create unique host texture");
    let _host_file = HostFile(texture_path.clone().into());
    host.write_all(b"host-only data must not be embedded")
        .unwrap();
    drop(host);

    let mut file_system = MemoryFileSystem::new();
    file_system.add_file(
        "triangle.obj",
        b"mtllib triangle.mtl\no tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl surface\nf 1 2 3\n"
            .to_vec(),
    );
    file_system.add_file(
        "triangle.mtl",
        format!("newmtl surface\nKd 1 1 1\nmap_Kd {texture_path}\n").into_bytes(),
    );
    file_system.add_file(texture_path, b"opaque texture data".to_vec());
    let file_system = Arc::new(std::sync::Mutex::new(file_system));
    let weak = Arc::downgrade(&file_system);
    let scene = Importer::new()
        .read_file("triangle.obj")
        .with_file_system_shared(file_system)
        .import()
        .expect("import textured OBJ with custom IO");

    assert!(weak.upgrade().is_some());
    assert_eq!(scene.num_meshes(), 1);
    assert_eq!(scene.num_textures(), 0);
    // The virtual file system must remain authoritative for later texture reads.
    let scene = scene
        .apply_postprocess(PostProcessSteps::EMBED_TEXTURES)
        .expect("post-processing uses the retained custom IO");
    assert_eq!(scene.num_textures(), 1);
    let texture = scene.texture(0).expect("embedded virtual texture");
    let asset_importer::texture::TextureDataRef::Compressed(bytes) = texture.data_ref().unwrap()
    else {
        panic!("embedded PNG should retain its compressed bytes");
    };
    assert_eq!(bytes, b"opaque texture data");
    drop(texture);
    drop(scene);
    assert!(weak.upgrade().is_none());
}

#[test]
fn closing_read_only_streams_does_not_flush_them() {
    let opened = Arc::new(AtomicUsize::new(0));
    let closed = Arc::new(AtomicUsize::new(0));
    let scene = Importer::new()
        .read_file("triangle.ply")
        .with_file_system(CountingFs {
            opened: opened.clone(),
            closed: closed.clone(),
            failure: StreamFailure::FlushError,
        })
        .import()
        .expect("read-only import does not require flushing");
    assert_eq!(scene.num_meshes(), 1);
    drop(scene);
    assert!(opened.load(Ordering::SeqCst) > 0);
    assert_eq!(opened.load(Ordering::SeqCst), closed.load(Ordering::SeqCst));
}

#[test]
fn memory_import_reads_external_buffers_from_custom_io() {
    let gltf = br#"{
      "asset": {"version": "2.0"},
      "buffers": [{"uri": "positions.bin", "byteLength": 36}],
      "bufferViews": [{"buffer": 0, "byteLength": 36}],
      "accessors": [{"bufferView": 0, "componentType": 5126, "count": 3,
                     "type": "VEC3", "min": [0,0,0], "max": [1,1,0]}],
      "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
      "nodes": [{"mesh": 0}], "scenes": [{"nodes": [0]}], "scene": 0
    }"#;
    let positions: Vec<u8> = [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect();
    let mut fs = MemoryFileSystem::new();
    fs.add_file("positions.bin", positions);
    let scene = Importer::new()
        .read_from_memory(gltf)
        .with_memory_hint("gltf")
        .with_file_system(fs)
        .import()
        .expect("in-memory glTF resolves its buffer from custom IO");
    let mesh = scene.mesh(0).expect("triangle mesh");
    assert_eq!(mesh.num_vertices(), 3);
    assert_eq!(mesh.vertices_raw()[1].x, 1.0);
}

#[derive(Debug)]
struct FailingSidecarFs {
    files: MemoryFileSystem,
    error_kind: std::io::ErrorKind,
}

impl FileSystem for FailingSidecarFs {
    fn exists(&self, path: &str) -> bool {
        self.files.exists(path)
    }

    fn open(&self, path: &str) -> Result<Box<dyn FileStream>, Error> {
        if path.ends_with(".mtl") {
            return Err(std::io::Error::new(self.error_kind, "sidecar open failure").into());
        }
        self.files.open(path)
    }
}

#[test]
fn optional_sidecars_only_ignore_not_found_errors() {
    for kind in [
        std::io::ErrorKind::NotFound,
        std::io::ErrorKind::PermissionDenied,
    ] {
        let mut files = MemoryFileSystem::new();
        files.add_file(
            "triangle.obj",
            [b"mtllib triangle.mtl\n".as_slice(), OBJ].concat(),
        );
        let result = Importer::new()
            .read_file("triangle.obj")
            .with_file_system(FailingSidecarFs {
                files,
                error_kind: kind,
            })
            .import();
        assert_eq!(
            result.is_ok(),
            kind == std::io::ErrorKind::NotFound,
            "{result:?}"
        );
    }
}

#[cfg(feature = "export")]
#[test]
fn custom_export_reports_callback_failures_and_closes_files() {
    use asset_importer::exporter::ExportBuilder;

    let scene = Importer::new()
        .import_from_memory(OBJ, Some("obj"))
        .expect("import OBJ for export");

    for format in ["obj", "x3d"] {
        for with_properties in [false, true] {
            for failure in [
                StreamFailure::None,
                StreamFailure::WriteError,
                StreamFailure::WritePanic,
                StreamFailure::WriteZero,
                StreamFailure::FlushError,
                StreamFailure::DropPanic,
            ] {
                let opened = Arc::new(AtomicUsize::new(0));
                let closed = Arc::new(AtomicUsize::new(0));
                let mut builder = ExportBuilder::new(format).with_file_system(CountingFs {
                    opened: opened.clone(),
                    closed: closed.clone(),
                    failure,
                });
                if with_properties {
                    builder = builder.with_property_bool("EXPORT_POINT_CLOUDS", false);
                }

                let result = builder.export_to_file(&scene, format!("triangle.{format}"));
                assert_eq!(result.is_ok(), matches!(failure, StreamFailure::None));
                assert!(opened.load(Ordering::SeqCst) > 0);
                assert_eq!(opened.load(Ordering::SeqCst), closed.load(Ordering::SeqCst));
            }
        }
    }
}
