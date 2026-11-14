use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

use asset_importer::{
    Error, ImportBuilder, Importer, PropertyStore, PropertyValue, Scene, import_properties,
    postprocess::PostProcessSteps, types::Matrix4x4,
};

const TRIANGLE: &[u8] = b"o tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
const CORNER: &[u8] = b"o edge\nv 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nf 1 2 3\nf 1 4 2\n";
const EXCLUDE_MATERIALS: &str = "PP_RRM_EXCLUDE_LIST";

thread_local! {
    static ALLOCATIONS: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
}

struct CountingAllocator;

fn record_allocation(size: usize) {
    let _ = ALLOCATIONS.try_with(|allocations| {
        if let Some((minimum_size, count)) = allocations.get() {
            if size >= minimum_size {
                allocations.set(Some((minimum_size, count + 1)));
            }
        }
    });
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let allocation = unsafe { System.alloc(layout) };
        record_allocation(layout.size());
        allocation
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let allocation = unsafe { System.alloc_zeroed(layout) };
        record_allocation(layout.size());
        allocation
    }

    unsafe fn dealloc(&self, allocation: *mut u8, layout: Layout) {
        unsafe { System.dealloc(allocation, layout) };
    }

    unsafe fn realloc(&self, allocation: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let allocation = unsafe { System.realloc(allocation, layout, new_size) };
        record_allocation(new_size);
        allocation
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn source_allocations<T>(minimum_size: usize, operation: impl FnOnce() -> T) -> (T, usize) {
    struct Reset(Option<(usize, usize)>);

    impl Drop for Reset {
        fn drop(&mut self) {
            ALLOCATIONS.with(|allocations| allocations.set(self.0));
        }
    }

    let reset = Reset(ALLOCATIONS.with(|allocations| allocations.replace(Some((minimum_size, 0)))));
    let result = operation();
    let count = ALLOCATIONS.with(|allocations| allocations.get().unwrap().1);
    drop(reset);
    (result, count)
}

fn padded_triangle() -> Vec<u8> {
    let mut data = Vec::with_capacity(128 * 1024);
    data.extend_from_slice(TRIANGLE);
    data.resize(128 * 1024, b'\n');
    data
}

fn assert_max_x(scene: &Scene, expected: f32) {
    let mesh = scene.mesh(0).expect("imported mesh");
    let maximum = mesh
        .vertices_raw()
        .iter()
        .map(|vertex| vertex.x)
        .reduce(f32::max)
        .expect("imported vertices");
    assert!((maximum - expected).abs() < 1e-5, "{maximum} != {expected}");
}

#[test]
fn normal_and_tangent_smoothing_angles_are_independent() {
    for angle in [80.0, 175.0] {
        let scene = Importer::new()
            .read_from_memory(CORNER)
            .with_memory_hint("obj")
            .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::GEN_SMOOTH_NORMALS)
            .with_property_float(import_properties::NORMAL_MAX_SMOOTHING_ANGLE, angle)
            .with_property_float(import_properties::TANGENT_MAX_SMOOTHING_ANGLE, 0.0)
            .import()
            .expect("import perpendicular faces with generated normals");
        let mesh = scene.mesh(0).unwrap();
        let normals: Vec<_> = mesh
            .vertices_raw()
            .iter()
            .zip(mesh.normals_raw())
            .filter(|(vertex, _)| vertex.x == 0.0 && vertex.y == 0.0 && vertex.z == 0.0)
            .map(|(_, normal)| normal)
            .collect();
        assert_eq!(normals.len(), 2);
        if angle == 80.0 {
            assert!(
                normals
                    .iter()
                    .any(|normal| normal.y > 0.99 && normal.z.abs() < 1e-5)
            );
            assert!(
                normals
                    .iter()
                    .any(|normal| normal.z > 0.99 && normal.y.abs() < 1e-5)
            );
        } else {
            for normal in normals {
                assert!((normal.y - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5);
                assert!((normal.z - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5);
            }
        }
    }
}

#[test]
fn property_store_replaces_values_in_the_native_type_space() {
    let mut store = PropertyStore::new();
    store
        .set_int("shared", 7)
        .set_float("shared", 2.0)
        .set_string("shared", "old")
        .set_matrix("shared", Matrix4x4::IDENTITY)
        .set_bool("shared", false);
    assert_eq!(store.len(), 4);
    assert!(
        store
            .properties()
            .iter()
            .any(|(_, value)| { matches!(value, PropertyValue::Boolean(false)) })
    );
    store
        .set_int("shared", 9)
        .set_float("shared", 3.0)
        .set_string("shared", "new");
    assert_eq!(store.len(), 4);
    assert!(
        store
            .properties()
            .iter()
            .any(|(_, value)| { matches!(value, PropertyValue::Integer(9)) })
    );
    assert!(
        store
            .properties()
            .iter()
            .any(|(_, value)| { matches!(value, PropertyValue::Float(value) if *value == 3.0) })
    );
    assert!(
        store
            .properties()
            .iter()
            .any(|(_, value)| { matches!(value, PropertyValue::String(value) if value == "new") })
    );
}

#[test]
fn overridden_strings_are_discarded_before_bridge_validation() {
    let store = PropertyStore::from(vec![
        (
            EXCLUDE_MATERIALS.to_owned(),
            PropertyValue::String("discarded\0value".to_owned()),
        ),
        (
            EXCLUDE_MATERIALS.to_owned(),
            PropertyValue::String("DefaultMaterial".to_owned()),
        ),
    ]);
    let scene = Importer::new()
        .read_from_memory(TRIANGLE)
        .with_memory_hint("obj")
        .with_property_string(EXCLUDE_MATERIALS, "initial\0value")
        .with_property_store_ref(&store)
        .with_property_string(EXCLUDE_MATERIALS, "intermediate\0value")
        .with_property_store(store)
        .add_post_process(PostProcessSteps::REMOVE_REDUNDANT_MATERIALS)
        .import()
        .expect("only the last string value reaches the bridge");
    assert_eq!(scene.num_meshes(), 1);

    let error = ImportBuilder::new()
        .with_property_string(EXCLUDE_MATERIALS, "DefaultMaterial")
        .with_property_string(EXCLUDE_MATERIALS, "final\0value")
        .import_from_memory(TRIANGLE, Some("obj"))
        .expect_err("a final invalid string must still fail");
    assert!(matches!(error, Error::InvalidParameter { .. }));
}

#[test]
fn property_merges_and_builder_overrides_control_native_scaling() {
    let mut base = PropertyStore::new();
    base.set_float(import_properties::GLOBAL_SCALE_FACTOR, 2.0);
    let mut overrides = PropertyStore::new();
    overrides
        .set_float(import_properties::GLOBAL_SCALE_FACTOR, 3.0)
        .set_int(import_properties::GLOBAL_SCALE_FACTOR, 99);

    let cases = [
        (
            ImportBuilder::new()
                .with_property_float(import_properties::GLOBAL_SCALE_FACTOR, 1.0)
                .with_property_store(base.clone()),
            2.0,
        ),
        (
            ImportBuilder::new()
                .with_property_float(import_properties::GLOBAL_SCALE_FACTOR, 1.0)
                .with_property_store_ref(&base),
            2.0,
        ),
        (
            ImportBuilder::new()
                .with_property_store(base.clone())
                .with_property_store_ref(&overrides),
            3.0,
        ),
        (
            ImportBuilder::new()
                .with_property_store_ref(&base)
                .with_property_store(overrides)
                .with_property_float(import_properties::GLOBAL_SCALE_FACTOR, 4.0),
            4.0,
        ),
    ];
    for (builder, expected) in cases {
        let scene = builder
            .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::GLOBAL_SCALE)
            .import_from_memory(TRIANGLE, Some("obj"))
            .expect("import using the effective scale");
        assert_max_x(&scene, expected);
    }
}

#[test]
fn memory_imports_obey_the_source_allocation_contract() {
    let importer = Importer::new();
    let mut data = padded_triangle();
    let (result, allocations) = source_allocations(data.len(), || {
        importer.import_from_memory(&data, Some("obj"))
    });
    assert_eq!(allocations, 0, "immediate imports borrow the source");
    assert_eq!(result.unwrap().num_meshes(), 1);

    let (builder, allocations) =
        source_allocations(data.len(), || importer.read_from_memory(&data));
    assert_eq!(allocations, 1, "a borrowed builder copies the source once");
    data.fill(0);
    drop(data);
    let scene = builder.with_memory_hint("obj").import().unwrap();
    assert_eq!(scene.num_meshes(), 1);

    let data = padded_triangle();
    let (builder, allocations) =
        source_allocations(data.len(), || importer.read_from_memory_owned(data));
    assert_eq!(
        allocations, 0,
        "an owned builder retains the Vec allocation"
    );
    assert_eq!(
        builder
            .with_memory_hint("obj")
            .import()
            .unwrap()
            .num_meshes(),
        1
    );

    let data: Arc<[u8]> = Arc::from(padded_triangle());
    let (builder, allocations) = source_allocations(data.len(), || {
        importer.read_from_memory_shared(data.clone())
    });
    assert_eq!(
        allocations, 0,
        "a shared builder retains the Arc allocation"
    );
    drop(data);
    assert_eq!(
        builder
            .with_memory_hint("obj")
            .import()
            .unwrap()
            .num_meshes(),
        1
    );
}

#[cfg(unix)]
mod unix_paths {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;

    use asset_importer::io::MemoryFileSystem;

    use super::*;

    #[cfg(target_os = "linux")]
    struct TempDir(PathBuf);

    #[cfg(target_os = "linux")]
    impl TempDir {
        fn new() -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "asset-importer-config-{}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    #[cfg(target_os = "linux")]
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn native_paths_preserve_non_utf8_bytes_without_reading_the_lossy_alias() {
        let directory = TempDir::new();
        let path = directory
            .0
            .join(OsString::from_vec(b"model-\xff.obj".to_vec()));
        let alias = directory.0.join("model-\u{fffd}.obj");
        std::fs::write(&path, TRIANGLE).unwrap();
        std::fs::write(&alias, b"o other\nv 0 0 0\nv 9 0 0\nv 0 9 0\nf 1 2 3\n").unwrap();

        let importer = Importer::new();
        let scene = importer.import_file(&path).unwrap();
        assert_max_x(&scene, 1.0);
        let scene = importer.import_file(&alias).unwrap();
        assert_max_x(&scene, 9.0);
    }

    #[test]
    fn custom_file_systems_reject_non_utf8_paths_before_lossy_lookup() {
        let mut file_system = MemoryFileSystem::new();
        file_system.add_file("model-\u{fffd}.obj", TRIANGLE.to_vec());
        let path = PathBuf::from(OsString::from_vec(b"model-\xff.obj".to_vec()));
        let error = Importer::new()
            .read_file(path)
            .with_file_system(file_system)
            .import()
            .expect_err("virtual paths must not be rewritten");
        assert!(
            matches!(error, Error::InvalidParameter { ref message } if message.contains("UTF-8")),
            "{error}"
        );
    }
}
