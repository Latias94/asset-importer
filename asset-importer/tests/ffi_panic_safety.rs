#![cfg(any(feature = "prebuilt", feature = "build-assimp", feature = "system"))]

// Regression tests: never unwind across FFI callbacks.

use asset_importer::io::{FileStream, FileSystem};
use asset_importer::{Error, Importer};

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
