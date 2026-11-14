use std::io::ErrorKind;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use asset_importer::io::{FileStream, FileSystem, MemoryFileStream, MemoryFileSystem};
use asset_importer::postprocess::PostProcessSteps;
use asset_importer::{Error, Importer};

const OBJ: &[u8] = b"o tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
const PLY: &[u8] = b"ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n";
const FIRST_OPEN_ERROR: &str = "first custom open failure";
const LATER_OPEN_ERROR: &str = "later custom open failure";
const READ_ERROR: &str = "original custom read failure";

fn assert_io_error(error: Error, kind: ErrorKind, message: &str) {
    match error {
        Error::IoError { source } => {
            assert_eq!(source.kind(), kind);
            assert_eq!(source.to_string(), message);
        }
        error => panic!("expected the original I/O error, got {error:?}"),
    }
}

#[derive(Debug)]
struct OpenFailureFs {
    files: MemoryFileSystem,
    suffix: &'static str,
    kind: ErrorKind,
    attempts: Arc<AtomicUsize>,
}

impl FileSystem for OpenFailureFs {
    fn exists(&self, path: &str) -> bool {
        self.files.exists(path)
    }

    fn open(&self, path: &str) -> Result<Box<dyn FileStream>, Error> {
        if path.ends_with(self.suffix) {
            let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
            let message = if attempt == 0 {
                FIRST_OPEN_ERROR
            } else {
                LATER_OPEN_ERROR
            };
            return Err(std::io::Error::new(self.kind, message).into());
        }
        self.files.open(path)
    }

    fn open_with_mode(&self, path: &str, _mode: &str) -> Result<Box<dyn FileStream>, Error> {
        self.open(path)
    }
}

#[test]
fn main_file_permission_denied_precedes_native_import_failure() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let error = Importer::new()
        .read_file("triangle.obj")
        .with_file_system(OpenFailureFs {
            files: MemoryFileSystem::new(),
            suffix: ".obj",
            kind: ErrorKind::PermissionDenied,
            attempts: attempts.clone(),
        })
        .import()
        .expect_err("opening the main file must fail");

    assert!(attempts.load(Ordering::SeqCst) > 0);
    assert_io_error(error, ErrorKind::PermissionDenied, FIRST_OPEN_ERROR);
}

#[derive(Debug)]
struct ReadFailureFs {
    opened: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
}

impl FileSystem for ReadFailureFs {
    fn exists(&self, path: &str) -> bool {
        path.ends_with(".ply")
    }

    fn open(&self, path: &str) -> Result<Box<dyn FileStream>, Error> {
        if !self.exists(path) {
            return Err(std::io::Error::new(ErrorKind::NotFound, path.to_owned()).into());
        }
        self.opened.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(ReadFailureStream {
            stream: MemoryFileStream::new(PLY.to_vec()),
            closed: self.closed.clone(),
        }))
    }
}

struct ReadFailureStream {
    stream: MemoryFileStream,
    closed: Arc<AtomicUsize>,
}

impl FileStream for ReadFailureStream {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, Error> {
        Err(std::io::Error::new(ErrorKind::InvalidData, READ_ERROR).into())
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
}

impl Drop for ReadFailureStream {
    fn drop(&mut self) {
        self.closed.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn read_failure_preserves_source_and_releases_streams() {
    let opened = Arc::new(AtomicUsize::new(0));
    let closed = Arc::new(AtomicUsize::new(0));
    let file_system = Arc::new(Mutex::new(ReadFailureFs {
        opened: opened.clone(),
        closed: closed.clone(),
    }));
    let weak = Arc::downgrade(&file_system);
    let error = Importer::new()
        .read_file("triangle.ply")
        .with_file_system_shared(file_system)
        .import()
        .expect_err("reading the main file must fail");

    assert_io_error(error, ErrorKind::InvalidData, READ_ERROR);
    assert!(opened.load(Ordering::SeqCst) > 0);
    assert_eq!(opened.load(Ordering::SeqCst), closed.load(Ordering::SeqCst));
    assert!(weak.upgrade().is_none());
}

#[test]
fn not_found_sidecars_do_not_fail_a_valid_import() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let mut files = MemoryFileSystem::new();
    files.add_file(
        "triangle.obj",
        [b"mtllib missing.mtl\n".as_slice(), OBJ].concat(),
    );
    let scene = Importer::new()
        .read_file("triangle.obj")
        .with_file_system(OpenFailureFs {
            files,
            suffix: ".mtl",
            kind: ErrorKind::NotFound,
            attempts: attempts.clone(),
        })
        .import()
        .expect("missing optional materials must not reject the triangle");

    assert_eq!(scene.num_meshes(), 1);
    assert!(attempts.load(Ordering::SeqCst) >= 2);
}

#[test]
fn first_sidecar_error_survives_later_failures_and_native_success() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let mut files = MemoryFileSystem::new();
    files.add_file(
        "triangle.obj",
        [b"mtllib missing.mtl\n".as_slice(), OBJ].concat(),
    );
    let file_system = Arc::new(Mutex::new(OpenFailureFs {
        files,
        suffix: ".mtl",
        kind: ErrorKind::PermissionDenied,
        attempts: attempts.clone(),
    }));
    let weak = Arc::downgrade(&file_system);
    let error = Importer::new()
        .read_file("triangle.obj")
        .with_file_system_shared(file_system)
        .import()
        .expect_err("a native scene must not hide material I/O failures");

    assert!(attempts.load(Ordering::SeqCst) >= 2);
    assert_io_error(error, ErrorKind::PermissionDenied, FIRST_OPEN_ERROR);
    assert!(weak.upgrade().is_none());
}

#[test]
fn missing_main_file_is_a_native_import_failure() {
    let error = Importer::new()
        .read_file("missing.obj")
        .with_file_system(MemoryFileSystem::new())
        .import()
        .expect_err("a missing main file must fail the native import");

    match error {
        Error::ImportFailed { message } => assert!(!message.is_empty()),
        error => panic!("expected ImportFailed, got {error:?}"),
    }
}

#[test]
fn native_memory_import_failure_is_typed() {
    let error = Importer::new()
        .read_from_memory(b"")
        .with_memory_hint("obj")
        .import()
        .expect_err("an empty memory import must fail");

    match error {
        Error::ImportFailed { message } => assert_eq!(message, "Memory buffer is empty"),
        error => panic!("expected ImportFailed, got {error:?}"),
    }
}

#[test]
fn poisoned_file_system_is_latched_as_an_io_failure() {
    let file_system = Arc::new(Mutex::new(MemoryFileSystem::new()));
    let poisoned = file_system.clone();
    assert!(
        std::thread::spawn(move || {
            let _guard = poisoned.lock().unwrap();
            panic!("poison the custom file system");
        })
        .join()
        .is_err()
    );

    let error = Importer::new()
        .read_file("triangle.obj")
        .with_file_system_shared(file_system)
        .import()
        .expect_err("a poisoned file system must fail the import");

    assert_io_error(
        error,
        ErrorKind::Other,
        "Custom IO file system mutex poisoned",
    );
}

#[test]
fn postprocess_preserves_io_errors_even_when_native_processing_succeeds() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let mut files = MemoryFileSystem::new();
    files.add_file(
        "triangle.obj",
        b"mtllib triangle.mtl\no tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl surface\nf 1 2 3\n"
            .to_vec(),
    );
    files.add_file(
        "triangle.mtl",
        b"newmtl surface\nKd 1 1 1\nmap_Kd texture.png\n".to_vec(),
    );
    let file_system = Arc::new(Mutex::new(OpenFailureFs {
        files,
        suffix: ".png",
        kind: ErrorKind::PermissionDenied,
        attempts: attempts.clone(),
    }));
    let weak = Arc::downgrade(&file_system);
    let scene = Importer::new()
        .read_file("triangle.obj")
        .with_file_system_shared(file_system)
        .import()
        .expect("loading the OBJ must not read its texture yet");
    assert_eq!(attempts.load(Ordering::SeqCst), 0);

    let error = scene
        .apply_postprocess(PostProcessSteps::EMBED_TEXTURES)
        .expect_err("texture I/O failures must take precedence over native success");

    assert!(attempts.load(Ordering::SeqCst) > 0);
    assert_io_error(error, ErrorKind::PermissionDenied, FIRST_OPEN_ERROR);
    assert!(weak.upgrade().is_none());
}

#[test]
fn native_postprocess_failure_remains_invalid_scene() {
    let scene = Importer::new()
        .read_from_memory(OBJ)
        .with_memory_hint("obj")
        .with_post_process(PostProcessSteps::JOIN_IDENTICAL_VERTICES)
        .import()
        .expect("import an indexed triangle");
    assert!(scene.is_non_verbose());

    let error = scene
        .apply_postprocess(PostProcessSteps::GEN_NORMALS)
        .expect_err("normal generation must reject an indexed scene");

    match error {
        Error::InvalidScene { message } => assert!(
            message.starts_with("Post-processing order mismatch:"),
            "{message}"
        ),
        error => panic!("expected InvalidScene, got {error:?}"),
    }
}

#[cfg(feature = "export")]
mod export {
    use super::*;
    use asset_importer::exporter::ExportBuilder;

    const WRITE_ERROR: &str = "original custom write failure";
    const FLUSH_ERROR: &str = "original custom close flush failure";

    #[derive(Debug)]
    struct ExportFs {
        fail_write: bool,
        opened: Arc<AtomicUsize>,
        closed: Arc<AtomicUsize>,
        flushed: Arc<AtomicUsize>,
    }

    impl FileSystem for ExportFs {
        fn exists(&self, _path: &str) -> bool {
            false
        }

        fn open(&self, path: &str) -> Result<Box<dyn FileStream>, Error> {
            Err(std::io::Error::new(ErrorKind::NotFound, path.to_owned()).into())
        }

        fn open_with_mode(&self, _path: &str, _mode: &str) -> Result<Box<dyn FileStream>, Error> {
            self.opened.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(ExportStream {
                stream: MemoryFileStream::new_writable(0),
                fail_write: self.fail_write,
                closed: self.closed.clone(),
                flushed: self.flushed.clone(),
            }))
        }
    }

    struct ExportStream {
        stream: MemoryFileStream,
        fail_write: bool,
        closed: Arc<AtomicUsize>,
        flushed: Arc<AtomicUsize>,
    }

    impl FileStream for ExportStream {
        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Error> {
            self.stream.read(buffer)
        }

        fn write(&mut self, buffer: &[u8]) -> Result<usize, Error> {
            if self.fail_write {
                return Err(std::io::Error::new(ErrorKind::PermissionDenied, WRITE_ERROR).into());
            }
            self.stream.write(buffer)
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
            self.flushed.fetch_add(1, Ordering::SeqCst);
            Err(std::io::Error::new(ErrorKind::BrokenPipe, FLUSH_ERROR).into())
        }
    }

    impl Drop for ExportStream {
        fn drop(&mut self) {
            self.closed.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn close_flush_errors_are_available_when_export_returns() {
        let scene = Importer::new()
            .import_from_memory(OBJ, Some("obj"))
            .expect("import the export source");
        let opened = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicUsize::new(0));
        let flushed = Arc::new(AtomicUsize::new(0));
        let error = ExportBuilder::new("x3d")
            .with_file_system(ExportFs {
                fail_write: false,
                opened: opened.clone(),
                closed: closed.clone(),
                flushed: flushed.clone(),
            })
            .export_to_file(&scene, "triangle.x3d")
            .expect_err("closing a writable stream must report flush failures");

        assert_io_error(error, ErrorKind::BrokenPipe, FLUSH_ERROR);
        assert!(opened.load(Ordering::SeqCst) > 0);
        assert_eq!(opened.load(Ordering::SeqCst), closed.load(Ordering::SeqCst));
        assert!(flushed.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn write_failure_precedes_later_close_flush_failure() {
        let scene = Importer::new()
            .import_from_memory(OBJ, Some("obj"))
            .expect("import the export source");
        let opened = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicUsize::new(0));
        let flushed = Arc::new(AtomicUsize::new(0));
        let error = ExportBuilder::new("obj")
            .with_file_system(ExportFs {
                fail_write: true,
                opened: opened.clone(),
                closed: closed.clone(),
                flushed: flushed.clone(),
            })
            .export_to_file(&scene, "triangle.obj")
            .expect_err("the first write failure must survive stream closing");

        assert_io_error(error, ErrorKind::PermissionDenied, WRITE_ERROR);
        assert!(opened.load(Ordering::SeqCst) > 0);
        assert_eq!(opened.load(Ordering::SeqCst), closed.load(Ordering::SeqCst));
        assert!(flushed.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn output_open_not_found_is_not_an_optional_probe() {
        let scene = Importer::new()
            .import_from_memory(OBJ, Some("obj"))
            .expect("import the export source");
        let attempts = Arc::new(AtomicUsize::new(0));
        let error = ExportBuilder::new("x3d")
            .with_file_system(OpenFailureFs {
                files: MemoryFileSystem::new(),
                suffix: ".x3d",
                kind: ErrorKind::NotFound,
                attempts: attempts.clone(),
            })
            .export_to_file(&scene, "triangle.x3d")
            .expect_err("missing output directories must retain their I/O errors");

        assert!(attempts.load(Ordering::SeqCst) > 0);
        assert_io_error(error, ErrorKind::NotFound, FIRST_OPEN_ERROR);
    }
}
