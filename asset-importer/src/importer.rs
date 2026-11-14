//! Scene importer functionality

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{
    error::{Error, Result},
    io::{FileSystem, OwnedAiFileIO, catch_unwind_or},
    postprocess::PostProcessSteps,
    progress::ProgressHandler,
    scene::Scene,
    sys,
};

use crate::bridge_properties::build_rust_properties;

type ProgressMutex = std::sync::Mutex<Box<dyn ProgressHandler>>;

struct ProgressUser {
    ptr: *mut ProgressMutex,
}

impl ProgressUser {
    fn new(handler: Box<dyn ProgressHandler>) -> Self {
        let ptr = Box::into_raw(Box::new(std::sync::Mutex::new(handler)));
        Self { ptr }
    }

    fn as_void_ptr(&self) -> *mut c_void {
        self.ptr.cast::<c_void>()
    }
}

impl Drop for ProgressUser {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            drop(Box::from_raw(self.ptr));
        }
    }
}

unsafe extern "C" fn progress_cb(
    percentage: f32,
    message: *const c_char,
    user: *mut c_void,
) -> bool {
    if user.is_null() {
        return true;
    }

    let user_ptr = user as *const ProgressMutex;
    let align = std::mem::align_of::<ProgressMutex>();
    if align > 1 && (user_ptr as usize) % align != 0 {
        return true;
    }

    let msg_opt = if message.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(message) }.to_str().ok()
    };

    catch_unwind_or(false, || {
        let mutex = unsafe { &*user_ptr };
        let Ok(mut handler) = mutex.lock() else {
            return false;
        };
        handler.update(percentage, msg_opt)
    })
}

/// A property store for configuring import and export behavior.
///
/// Setting a property replaces its previous value in the same native type space.
/// Boolean and integer properties share a type space; float, string, and matrix
/// properties are independent, even when their names match.
/// Only the final values are validated and encoded when an operation executes.
#[derive(Debug, Clone)]
pub struct PropertyStore {
    properties: Vec<(String, PropertyValue)>,
}

impl PropertyStore {
    /// Create a new empty property store
    pub fn new() -> Self {
        Self {
            properties: Vec::new(),
        }
    }

    /// Set a property, replacing any value with the same name and native type.
    pub fn set<S: Into<String>>(&mut self, name: S, value: PropertyValue) -> &mut Self {
        let name = name.into();
        if let Some((_, previous)) = self
            .properties
            .iter_mut()
            .find(|(key, previous)| key == &name && previous.same_native_type(&value))
        {
            *previous = value;
        } else {
            self.properties.push((name, value));
        }
        self
    }

    /// Set an integer property
    pub fn set_int<S: Into<String>>(&mut self, name: S, value: i32) -> &mut Self {
        self.set(name, PropertyValue::Integer(value))
    }

    /// Set a float property
    pub fn set_float<S: Into<String>>(&mut self, name: S, value: f32) -> &mut Self {
        self.set(name, PropertyValue::Float(value))
    }

    /// Set a string property
    pub fn set_string<S: Into<String>, V: Into<String>>(&mut self, name: S, value: V) -> &mut Self {
        self.set(name, PropertyValue::String(value.into()))
    }

    /// Set a boolean property
    pub fn set_bool<S: Into<String>>(&mut self, name: S, value: bool) -> &mut Self {
        self.set(name, PropertyValue::Boolean(value))
    }

    /// Set a matrix property
    pub fn set_matrix<S: Into<String>>(
        &mut self,
        name: S,
        value: crate::types::Matrix4x4,
    ) -> &mut Self {
        self.set(name, PropertyValue::Matrix(value))
    }

    /// Get the effective properties in their original insertion order.
    pub fn properties(&self) -> &[(String, PropertyValue)] {
        &self.properties
    }

    /// Clear all properties
    pub fn clear(&mut self) {
        self.properties.clear();
    }

    /// Check if the property store is empty
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Get the number of effective properties across all native type spaces.
    pub fn len(&self) -> usize {
        self.properties.len()
    }
}

impl Default for PropertyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Extend<(String, PropertyValue)> for PropertyStore {
    fn extend<T: IntoIterator<Item = (String, PropertyValue)>>(&mut self, properties: T) {
        for (name, value) in properties {
            self.set(name, value);
        }
    }
}

impl From<Vec<(String, PropertyValue)>> for PropertyStore {
    fn from(properties: Vec<(String, PropertyValue)>) -> Self {
        let mut store = Self::new();
        store.extend(properties);
        store
    }
}

impl From<PropertyStore> for Vec<(String, PropertyValue)> {
    fn from(store: PropertyStore) -> Self {
        store.properties
    }
}

/// Common import property keys
///
/// These constants provide convenient access to commonly used Assimp import properties.
pub mod import_properties {
    /// Remove vertex components (AI_CONFIG_PP_RVC_FLAGS)
    pub const REMOVE_VERTEX_COMPONENTS: &str = "PP_RVC_FLAGS";

    /// Maximum normal smoothing angle in degrees (AI_CONFIG_PP_GSN_MAX_SMOOTHING_ANGLE).
    pub const NORMAL_MAX_SMOOTHING_ANGLE: &str = "PP_GSN_MAX_SMOOTHING_ANGLE";

    /// Maximum tangent smoothing angle in degrees (AI_CONFIG_PP_CT_MAX_SMOOTHING_ANGLE).
    pub const TANGENT_MAX_SMOOTHING_ANGLE: &str = "PP_CT_MAX_SMOOTHING_ANGLE";

    /// FBX: Read all geometry layers (AI_CONFIG_IMPORT_FBX_READ_ALL_GEOMETRY_LAYERS)
    pub const FBX_READ_ALL_GEOMETRY_LAYERS: &str = "IMPORT_FBX_READ_ALL_GEOMETRY_LAYERS";

    /// FBX: Read all materials (AI_CONFIG_IMPORT_FBX_READ_ALL_MATERIALS)
    pub const FBX_READ_ALL_MATERIALS: &str = "IMPORT_FBX_READ_ALL_MATERIALS";

    /// FBX: Read materials (AI_CONFIG_IMPORT_FBX_READ_MATERIALS)
    pub const FBX_READ_MATERIALS: &str = "IMPORT_FBX_READ_MATERIALS";

    /// FBX: Read textures (AI_CONFIG_IMPORT_FBX_READ_TEXTURES)
    pub const FBX_READ_TEXTURES: &str = "IMPORT_FBX_READ_TEXTURES";

    /// FBX: Read cameras (AI_CONFIG_IMPORT_FBX_READ_CAMERAS)
    pub const FBX_READ_CAMERAS: &str = "IMPORT_FBX_READ_CAMERAS";

    /// FBX: Read lights (AI_CONFIG_IMPORT_FBX_READ_LIGHTS)
    pub const FBX_READ_LIGHTS: &str = "IMPORT_FBX_READ_LIGHTS";

    /// FBX: Read animations (AI_CONFIG_IMPORT_FBX_READ_ANIMATIONS)
    pub const FBX_READ_ANIMATIONS: &str = "IMPORT_FBX_READ_ANIMATIONS";

    /// FBX: Read weights (AI_CONFIG_IMPORT_FBX_READ_WEIGHTS)
    pub const FBX_READ_WEIGHTS: &str = "IMPORT_FBX_READ_WEIGHTS";

    /// FBX: Strict mode (AI_CONFIG_IMPORT_FBX_STRICT_MODE)
    pub const FBX_STRICT_MODE: &str = "IMPORT_FBX_STRICT_MODE";

    /// FBX: Preserve pivots (AI_CONFIG_IMPORT_FBX_PRESERVE_PIVOTS)
    pub const FBX_PRESERVE_PIVOTS: &str = "IMPORT_FBX_PRESERVE_PIVOTS";

    /// FBX: Optimize empty animation curves (AI_CONFIG_IMPORT_FBX_OPTIMIZE_EMPTY_ANIMATION_CURVES)
    pub const FBX_OPTIMIZE_EMPTY_ANIMATION_CURVES: &str =
        "IMPORT_FBX_OPTIMIZE_EMPTY_ANIMATION_CURVES";

    /// FBX: Use legacy naming for embedded textures (AI_CONFIG_IMPORT_FBX_EMBEDDED_TEXTURES_LEGACY_NAMING)
    pub const FBX_EMBEDDED_TEXTURES_LEGACY_NAMING: &str =
        "AI_CONFIG_IMPORT_FBX_EMBEDDED_TEXTURES_LEGACY_NAMING";

    /// FBX: Ignore up direction (AI_CONFIG_IMPORT_FBX_IGNORE_UP_DIRECTION)
    ///
    /// This can be useful when importing files that define custom axes.
    pub const FBX_IGNORE_UP_DIRECTION: &str = "AI_CONFIG_IMPORT_FBX_IGNORE_UP_DIRECTION";

    /// Remove degenerate faces (AI_CONFIG_PP_FD_REMOVE)
    pub const REMOVE_DEGENERATE_FACES: &str = "PP_FD_REMOVE";

    /// Limit bone weights (AI_CONFIG_PP_LBW_MAX_WEIGHTS)
    pub const LIMIT_BONE_WEIGHTS_MAX: &str = "PP_LBW_MAX_WEIGHTS";

    /// Bone removal threshold for the Debone step (AI_CONFIG_PP_DB_THRESHOLD).
    pub const DEBONE_THRESHOLD: &str = "PP_DB_THRESHOLD";

    /// IFC: Skip space representations (AI_CONFIG_IMPORT_IFC_SKIP_SPACE_REPRESENTATIONS)
    pub const IFC_SKIP_SPACE_REPRESENTATIONS: &str = "IMPORT_IFC_SKIP_SPACE_REPRESENTATIONS";

    /// User scale factor for the GlobalScale step (AI_CONFIG_GLOBAL_SCALE_FACTOR_KEY).
    pub const GLOBAL_SCALE_FACTOR: &str = "GLOBAL_SCALE_FACTOR";
}

#[cfg(test)]
mod import_properties_tests {
    use super::import_properties;

    fn c_key(bytes: &[u8]) -> &str {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        std::str::from_utf8(&bytes[..end]).expect("Assimp config key should be UTF-8")
    }

    #[test]
    fn import_property_keys_match_assimp_macros() {
        assert_eq!(
            import_properties::REMOVE_VERTEX_COMPONENTS,
            c_key(crate::sys::AI_CONFIG_PP_RVC_FLAGS)
        );
        assert_eq!(
            import_properties::NORMAL_MAX_SMOOTHING_ANGLE,
            c_key(crate::sys::AI_CONFIG_PP_GSN_MAX_SMOOTHING_ANGLE)
        );
        assert_eq!(
            import_properties::TANGENT_MAX_SMOOTHING_ANGLE,
            c_key(crate::sys::AI_CONFIG_PP_CT_MAX_SMOOTHING_ANGLE)
        );
        assert_eq!(
            import_properties::FBX_READ_ALL_GEOMETRY_LAYERS,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_READ_ALL_GEOMETRY_LAYERS)
        );
        assert_eq!(
            import_properties::FBX_READ_ALL_MATERIALS,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_READ_ALL_MATERIALS)
        );
        assert_eq!(
            import_properties::FBX_READ_MATERIALS,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_READ_MATERIALS)
        );
        assert_eq!(
            import_properties::FBX_READ_TEXTURES,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_READ_TEXTURES)
        );
        assert_eq!(
            import_properties::FBX_READ_CAMERAS,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_READ_CAMERAS)
        );
        assert_eq!(
            import_properties::FBX_READ_LIGHTS,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_READ_LIGHTS)
        );
        assert_eq!(
            import_properties::FBX_READ_ANIMATIONS,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_READ_ANIMATIONS)
        );
        assert_eq!(
            import_properties::FBX_READ_WEIGHTS,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_READ_WEIGHTS)
        );
        assert_eq!(
            import_properties::FBX_STRICT_MODE,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_STRICT_MODE)
        );
        assert_eq!(
            import_properties::FBX_PRESERVE_PIVOTS,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_PRESERVE_PIVOTS)
        );
        assert_eq!(
            import_properties::FBX_OPTIMIZE_EMPTY_ANIMATION_CURVES,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_OPTIMIZE_EMPTY_ANIMATION_CURVES)
        );
        assert_eq!(
            import_properties::FBX_EMBEDDED_TEXTURES_LEGACY_NAMING,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_EMBEDDED_TEXTURES_LEGACY_NAMING)
        );
        assert_eq!(
            import_properties::FBX_IGNORE_UP_DIRECTION,
            c_key(crate::sys::AI_CONFIG_IMPORT_FBX_IGNORE_UP_DIRECTION)
        );
        assert_eq!(
            import_properties::REMOVE_DEGENERATE_FACES,
            c_key(crate::sys::AI_CONFIG_PP_FD_REMOVE)
        );
        assert_eq!(
            import_properties::LIMIT_BONE_WEIGHTS_MAX,
            c_key(crate::sys::AI_CONFIG_PP_LBW_MAX_WEIGHTS)
        );
        assert_eq!(
            import_properties::DEBONE_THRESHOLD,
            c_key(crate::sys::AI_CONFIG_PP_DB_THRESHOLD)
        );
        assert_eq!(
            import_properties::IFC_SKIP_SPACE_REPRESENTATIONS,
            c_key(crate::sys::AI_CONFIG_IMPORT_IFC_SKIP_SPACE_REPRESENTATIONS)
        );
        assert_eq!(
            import_properties::GLOBAL_SCALE_FACTOR,
            c_key(crate::sys::AI_CONFIG_GLOBAL_SCALE_FACTOR_KEY)
        );
    }
}

/// Builder for configuring and executing scene imports
pub struct ImportBuilder {
    source: Option<ImportSource>,
    source_memory_hint: Option<String>,
    post_process: PostProcessSteps,
    properties: PropertyStore,
    file_system: Option<std::sync::Arc<std::sync::Mutex<dyn FileSystem>>>,
    progress_handler: Option<Box<dyn ProgressHandler>>,
}

enum ImportSource {
    File(PathBuf),
    OwnedMemory(Vec<u8>),
    SharedMemory(Arc<[u8]>),
}

enum ImportInput<'a> {
    File(CString),
    Memory {
        data: &'a [u8],
        length: u32,
        hint: Option<CString>,
    },
}

/// Property values that can be set for import configuration
#[derive(Debug, Clone)]
pub enum PropertyValue {
    /// Integer property
    Integer(i32),
    /// Float property
    Float(f32),
    /// String property
    String(String),
    /// Boolean property (stored as integer)
    Boolean(bool),
    /// Matrix property (4x4 transformation matrix)
    Matrix(crate::types::Matrix4x4),
}

impl PropertyValue {
    fn same_native_type(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (
                Self::Integer(_) | Self::Boolean(_),
                Self::Integer(_) | Self::Boolean(_)
            ) | (Self::Float(_), Self::Float(_))
                | (Self::String(_), Self::String(_))
                | (Self::Matrix(_), Self::Matrix(_))
        )
    }
}

impl ImportBuilder {
    /// Create a new import builder
    pub fn new() -> Self {
        Self {
            source: None,
            source_memory_hint: None,
            post_process: PostProcessSteps::default(),
            properties: PropertyStore::new(),
            file_system: None,
            progress_handler: None,
        }
    }

    /// Set the import source to a file path.
    ///
    /// This enables [`ImportBuilder::import`] without passing the path again.
    pub fn with_source_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.source = Some(ImportSource::File(path.as_ref().to_path_buf()));
        self.source_memory_hint = None;
        self
    }

    /// Set the import source to a memory buffer by copying `data`.
    ///
    /// Prefer [`ImportBuilder::with_source_memory_owned`] or [`ImportBuilder::with_source_memory_shared`]
    /// when you already have an owned buffer.
    pub fn with_source_memory_copy(self, data: &[u8]) -> Self {
        self.with_source_memory_owned(data.to_vec())
    }

    /// Set the import source to an owned memory buffer without copying its allocation.
    pub fn with_source_memory_owned(mut self, data: Vec<u8>) -> Self {
        self.source = Some(ImportSource::OwnedMemory(data));
        self
    }

    /// Set the import source to a shared memory buffer.
    pub fn with_source_memory_shared(mut self, data: Arc<[u8]>) -> Self {
        self.source = Some(ImportSource::SharedMemory(data));
        self
    }

    /// Set the optional file format hint for memory imports (e.g. `"obj"`, `"fbx"`).
    pub fn with_memory_hint<S: Into<String>>(mut self, hint: S) -> Self {
        self.source_memory_hint = Some(hint.into());
        self
    }

    /// Set the optional file format hint for memory imports.
    pub fn with_memory_hint_opt(mut self, hint: Option<&str>) -> Self {
        self.source_memory_hint = hint.map(|s| s.to_string());
        self
    }

    /// Set the post-processing steps to apply
    pub fn with_post_process(mut self, steps: PostProcessSteps) -> Self {
        self.post_process = steps;
        self
    }

    /// Add post-processing steps to the current set
    pub fn add_post_process(mut self, steps: PostProcessSteps) -> Self {
        self.post_process |= steps;
        self
    }

    /// Set an integer property
    pub fn with_property_int<S: Into<String>>(mut self, name: S, value: i32) -> Self {
        self.properties.set_int(name, value);
        self
    }

    /// Set a float property
    pub fn with_property_float<S: Into<String>>(mut self, name: S, value: f32) -> Self {
        self.properties.set_float(name, value);
        self
    }

    /// Set a string property
    pub fn with_property_string<S: Into<String>, V: Into<String>>(
        mut self,
        name: S,
        value: V,
    ) -> Self {
        self.properties.set_string(name, value);
        self
    }

    /// Set a boolean property
    pub fn with_property_bool<S: Into<String>>(mut self, name: S, value: bool) -> Self {
        self.properties.set_bool(name, value);
        self
    }

    /// Set a matrix property
    pub fn with_property_matrix<S: Into<String>>(
        mut self,
        name: S,
        value: crate::types::Matrix4x4,
    ) -> Self {
        self.properties.set_matrix(name, value);
        self
    }

    /// Merge a property store, overriding existing values in matching native type spaces.
    pub fn with_property_store(mut self, store: PropertyStore) -> Self {
        self.properties
            .extend(Vec::<(String, PropertyValue)>::from(store));
        self
    }

    /// Merge a borrowed property store with the same override semantics as an owned store.
    pub fn with_property_store_ref(mut self, store: &PropertyStore) -> Self {
        self.properties.extend(store.properties().iter().cloned());
        self
    }

    /// Set a custom file system (ergonomic wrapper).
    ///
    /// Prefer this over [`ImportBuilder::with_file_system_shared`] unless you need to share a
    /// single file system instance across multiple importers/builders.
    /// The imported scene retains this file system for later post-processing and
    /// releases it after the native importer and its streams have been destroyed.
    pub fn with_file_system<F>(self, file_system: F) -> Self
    where
        F: FileSystem + 'static,
    {
        self.with_file_system_shared(std::sync::Arc::new(std::sync::Mutex::new(file_system)))
    }

    /// Set a custom file system from an explicitly shared handle.
    pub fn with_file_system_shared(
        mut self,
        file_system: std::sync::Arc<std::sync::Mutex<dyn FileSystem>>,
    ) -> Self {
        self.file_system = Some(file_system);
        self
    }

    /// Set a progress handler
    pub fn with_progress_handler(mut self, handler: Box<dyn ProgressHandler>) -> Self {
        self.progress_handler = Some(handler);
        self
    }

    /// Set a progress handler from a closure.
    pub fn with_progress_handler_fn<F>(self, f: F) -> Self
    where
        F: FnMut(f32, Option<&str>) -> bool + Send + 'static,
    {
        self.with_progress_handler(Box::new(crate::progress::ClosureProgressHandler::new(f)))
    }

    /// Import using the configured source.
    ///
    /// This is the preferred ergonomic entry point when the source was set via
    /// [`Importer::read_file`], [`Importer::read_from_memory`], or the `with_source_*` methods.
    pub fn import(mut self) -> Result<Scene> {
        let source = self.source.take().ok_or_else(|| {
            Error::invalid_parameter(
                "Import source not set (use Importer::read_file/read_from_memory or ImportBuilder::with_source_*)",
            )
        })?;
        let hint = self.source_memory_hint.take();
        match source {
            ImportSource::File(path) => self.import_file(path),
            ImportSource::OwnedMemory(data) => self.import_from_memory(&data, hint.as_deref()),
            ImportSource::SharedMemory(data) => self.import_from_memory(&data, hint.as_deref()),
        }
    }

    /// Import a scene from a file path.
    ///
    /// Native Unix imports preserve the path's original bytes. Custom file systems
    /// require UTF-8 paths; other platforms reject paths that cannot be encoded as UTF-8.
    pub fn import_file<P: AsRef<Path>>(self, path: P) -> Result<Scene> {
        let path = path.as_ref();
        let path_bytes = if self.file_system.is_some() {
            path.to_str()
                .ok_or_else(|| Error::invalid_parameter("Custom file systems require UTF-8 paths"))?
                .as_bytes()
        } else {
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStrExt;
                path.as_os_str().as_bytes()
            }
            #[cfg(not(unix))]
            {
                path.to_str()
                    .ok_or_else(|| {
                        Error::invalid_parameter("Native file paths must be valid UTF-8")
                    })?
                    .as_bytes()
            }
        };
        let c_path =
            CString::new(path_bytes).map_err(|_| Error::invalid_parameter("Invalid file path"))?;
        self.execute(ImportInput::File(c_path))
    }

    /// Import a scene directly from a borrowed memory buffer without copying the source.
    pub fn import_from_memory(self, data: &[u8], hint: Option<&str>) -> Result<Scene> {
        let length = u32::try_from(data.len()).map_err(|_| {
            Error::invalid_parameter("Memory buffer is too large (assimp C API takes u32 length)")
        })?;
        let hint = hint
            .map(CString::new)
            .transpose()
            .map_err(|_| Error::invalid_parameter("Invalid hint"))?;
        self.execute(ImportInput::Memory { data, length, hint })
    }

    fn execute(self, input: ImportInput<'_>) -> Result<Scene> {
        self.post_process
            .validate()
            .map_err(Error::invalid_parameter)?;
        let buffers = build_rust_properties(&self.properties)?;
        let file_io = self
            .file_system
            .as_ref()
            .map(|fs| OwnedAiFileIO::new(fs.clone()));
        let user = self.progress_handler.map(ProgressUser::new);
        let callback: sys::aiRustProgressCallback = user.as_ref().map(|_| progress_cb as _);
        let file_io_ptr = file_io
            .as_ref()
            .map_or(std::ptr::null(), |io| io.as_ptr_sys());
        let user_ptr = user
            .as_ref()
            .map_or(std::ptr::null_mut(), ProgressUser::as_void_ptr);
        let owner = unsafe {
            match input {
                ImportInput::File(path) => sys::aiImportFileRust(
                    path.as_ptr(),
                    self.post_process.as_raw(),
                    file_io_ptr,
                    buffers.ffi_props.as_ptr(),
                    buffers.ffi_props.len(),
                    callback,
                    user_ptr,
                ),
                ImportInput::Memory { data, length, hint } => sys::aiImportFileFromMemoryRust(
                    data.as_ptr().cast::<c_char>(),
                    length,
                    self.post_process.as_raw(),
                    hint.as_ref().map_or(std::ptr::null(), |hint| hint.as_ptr()),
                    file_io_ptr,
                    buffers.ffi_props.as_ptr(),
                    buffers.ffi_props.len(),
                    callback,
                    user_ptr,
                ),
            }
        };
        let scene = unsafe { Scene::from_importer(owner, file_io) }?;
        drop(user);
        drop(self.file_system);
        Ok(scene)
    }
}

impl Default for ImportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Main importer interface
#[derive(Debug)]
pub struct Importer;

impl Importer {
    /// Create a new importer
    pub fn new() -> Self {
        Self
    }

    /// Start building an import operation
    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> ImportBuilder {
        ImportBuilder::new().with_source_file(path)
    }

    /// Start building an import operation from memory
    ///
    /// Note: this copies `data` into an owned buffer so the builder can be `'static`.
    pub fn read_from_memory(&self, data: &[u8]) -> ImportBuilder {
        ImportBuilder::new().with_source_memory_copy(data)
    }

    /// Start building an import operation from an owned memory buffer (no extra copy).
    pub fn read_from_memory_owned(&self, data: Vec<u8>) -> ImportBuilder {
        ImportBuilder::new().with_source_memory_owned(data)
    }

    /// Start building an import operation from a shared memory buffer (no extra copy).
    pub fn read_from_memory_shared(&self, data: Arc<[u8]>) -> ImportBuilder {
        ImportBuilder::new().with_source_memory_shared(data)
    }

    /// Quick import with default settings
    pub fn import_file<P: AsRef<Path>>(&self, path: P) -> Result<Scene> {
        self.read_file(path).import()
    }

    /// Quick import from borrowed memory with default settings and no source copy.
    pub fn import_from_memory(&self, data: &[u8], hint: Option<&str>) -> Result<Scene> {
        ImportBuilder::new().import_from_memory(data, hint)
    }

    /// Quick import from an owned memory buffer (no extra copy).
    pub fn import_from_memory_owned(&self, data: Vec<u8>, hint: Option<&str>) -> Result<Scene> {
        self.read_from_memory_owned(data)
            .with_memory_hint_opt(hint)
            .import()
    }

    /// Import a file with a builder configuration closure.
    ///
    /// This avoids repeating the path and keeps call sites compact.
    ///
    /// # Example
    /// ```rust,no_run
    /// use asset_importer::{Importer, postprocess::PostProcessSteps};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let scene = Importer::new().import_file_with("model.fbx", |b| {
    ///     b.with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::FLIP_UVS)
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn import_file_with<P, F>(&self, path: P, f: F) -> Result<Scene>
    where
        P: AsRef<Path>,
        F: FnOnce(ImportBuilder) -> ImportBuilder,
    {
        f(self.read_file(path)).import()
    }

    /// Import from memory with a builder configuration closure.
    ///
    /// This copies `data` into an owned buffer so the builder can be `'static`.
    pub fn import_from_memory_with<F>(&self, data: &[u8], hint: Option<&str>, f: F) -> Result<Scene>
    where
        F: FnOnce(ImportBuilder) -> ImportBuilder,
    {
        f(self.read_from_memory(data).with_memory_hint_opt(hint)).import()
    }

    /// Import from an owned memory buffer with a builder configuration closure (no extra copy).
    pub fn import_from_memory_owned_with<F>(
        &self,
        data: Vec<u8>,
        hint: Option<&str>,
        f: F,
    ) -> Result<Scene>
    where
        F: FnOnce(ImportBuilder) -> ImportBuilder,
    {
        f(self.read_from_memory_owned(data).with_memory_hint_opt(hint)).import()
    }
}

impl Default for Importer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_cb_rejects_unaligned_user_pointers() {
        let buf = [0u64; 8];
        let unaligned = unsafe { (buf.as_ptr() as *const u8).add(1) } as *mut c_void;
        assert!(unsafe { progress_cb(0.5, std::ptr::null(), unaligned) });
    }

    #[test]
    fn test_importer_creation() {
        let importer = Importer::new();
        let _builder = importer.read_file("test.obj");
    }

    #[test]
    fn test_import_builder() {
        let builder = ImportBuilder::new()
            .with_post_process(PostProcessSteps::TRIANGULATE)
            .with_property_int("test", 42)
            .with_property_bool("flag", true);

        assert!(builder.post_process.contains(PostProcessSteps::TRIANGULATE));
        assert_eq!(builder.properties.len(), 2);
    }
}
