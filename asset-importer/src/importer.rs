//! Scene importer functionality

use std::ffi::CString;
use std::path::Path;

use crate::{
    error::{Error, Result},
    io::{AssimpFileIO, FileSystem},
    postprocess::PostProcessSteps,
    progress::ProgressHandler,
    scene::Scene,
    sys,
};

/// A property store for configuring import behavior
///
/// This provides a more convenient API for setting import properties
/// compared to using the builder methods directly.
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

    /// Set an integer property
    pub fn set_int<S: Into<String>>(&mut self, name: S, value: i32) -> &mut Self {
        self.properties
            .push((name.into(), PropertyValue::Integer(value)));
        self
    }

    /// Set a float property
    pub fn set_float<S: Into<String>>(&mut self, name: S, value: f32) -> &mut Self {
        self.properties
            .push((name.into(), PropertyValue::Float(value)));
        self
    }

    /// Set a string property
    pub fn set_string<S: Into<String>, V: Into<String>>(&mut self, name: S, value: V) -> &mut Self {
        self.properties
            .push((name.into(), PropertyValue::String(value.into())));
        self
    }

    /// Set a boolean property
    pub fn set_bool<S: Into<String>>(&mut self, name: S, value: bool) -> &mut Self {
        self.properties
            .push((name.into(), PropertyValue::Boolean(value)));
        self
    }

    /// Set a matrix property
    pub fn set_matrix<S: Into<String>>(
        &mut self,
        name: S,
        value: crate::types::Matrix4x4,
    ) -> &mut Self {
        self.properties
            .push((name.into(), PropertyValue::Matrix(value)));
        self
    }

    /// Get all properties as a slice
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

    /// Get the number of properties
    pub fn len(&self) -> usize {
        self.properties.len()
    }
}

impl Default for PropertyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<(String, PropertyValue)>> for PropertyStore {
    fn from(properties: Vec<(String, PropertyValue)>) -> Self {
        Self { properties }
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
    pub const REMOVE_VERTEX_COMPONENTS: &str = "AI_CONFIG_PP_RVC_FLAGS";

    /// Maximum smoothing angle for normal generation (AI_CONFIG_PP_CT_MAX_SMOOTHING_ANGLE)
    pub const MAX_SMOOTHING_ANGLE: &str = "AI_CONFIG_PP_CT_MAX_SMOOTHING_ANGLE";

    /// FBX: Read all geometry layers (AI_CONFIG_IMPORT_FBX_READ_ALL_GEOMETRY_LAYERS)
    pub const FBX_READ_ALL_GEOMETRY_LAYERS: &str = "AI_CONFIG_IMPORT_FBX_READ_ALL_GEOMETRY_LAYERS";

    /// FBX: Preserve pivots (AI_CONFIG_IMPORT_FBX_PRESERVE_PIVOTS)
    pub const FBX_PRESERVE_PIVOTS: &str = "AI_CONFIG_IMPORT_FBX_PRESERVE_PIVOTS";

    /// Remove degenerate faces (AI_CONFIG_PP_FD_REMOVE)
    pub const REMOVE_DEGENERATE_FACES: &str = "AI_CONFIG_PP_FD_REMOVE";

    /// Split large meshes (AI_CONFIG_PP_SLM_VERTEX_LIMIT)
    pub const SPLIT_LARGE_MESHES_VERTEX_LIMIT: &str = "AI_CONFIG_PP_SLM_VERTEX_LIMIT";

    /// Split large meshes triangle limit (AI_CONFIG_PP_SLM_TRIANGLE_LIMIT)
    pub const SPLIT_LARGE_MESHES_TRIANGLE_LIMIT: &str = "AI_CONFIG_PP_SLM_TRIANGLE_LIMIT";

    /// Limit bone weights (AI_CONFIG_PP_LBW_MAX_WEIGHTS)
    pub const LIMIT_BONE_WEIGHTS_MAX: &str = "AI_CONFIG_PP_LBW_MAX_WEIGHTS";

    /// Validate data structure (AI_CONFIG_PP_DB_THRESHOLD)
    pub const VALIDATE_DATA_STRUCTURE_THRESHOLD: &str = "AI_CONFIG_PP_DB_THRESHOLD";

    /// IFC: Skip space representations (AI_CONFIG_IMPORT_IFC_SKIP_SPACE_REPRESENTATIONS)
    pub const IFC_SKIP_SPACE_REPRESENTATIONS: &str =
        "AI_CONFIG_IMPORT_IFC_SKIP_SPACE_REPRESENTATIONS";

    /// Global scale factor (AI_CONFIG_GLOBAL_SCALE_FACTOR_KEY)
    pub const GLOBAL_SCALE_FACTOR: &str = "AI_CONFIG_GLOBAL_SCALE_FACTOR_KEY";

    /// Application scale factor (AI_CONFIG_APP_SCALE_KEY)
    pub const APP_SCALE_FACTOR: &str = "AI_CONFIG_APP_SCALE_KEY";
}

/// Builder for configuring and executing scene imports
pub struct ImportBuilder {
    post_process: PostProcessSteps,
    properties: Vec<(String, PropertyValue)>,
    file_system: Option<std::sync::Arc<std::sync::Mutex<dyn FileSystem>>>,
    progress_handler: Option<Box<dyn ProgressHandler>>,
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

impl ImportBuilder {
    /// Create a new import builder
    pub fn new() -> Self {
        Self {
            post_process: PostProcessSteps::default(),
            properties: Vec::new(),
            file_system: None,
            progress_handler: None,
        }
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
        self.properties
            .push((name.into(), PropertyValue::Integer(value)));
        self
    }

    /// Set a float property
    pub fn with_property_float<S: Into<String>>(mut self, name: S, value: f32) -> Self {
        self.properties
            .push((name.into(), PropertyValue::Float(value)));
        self
    }

    /// Set a string property
    pub fn with_property_string<S: Into<String>, V: Into<String>>(
        mut self,
        name: S,
        value: V,
    ) -> Self {
        self.properties
            .push((name.into(), PropertyValue::String(value.into())));
        self
    }

    /// Set a boolean property
    pub fn with_property_bool<S: Into<String>>(mut self, name: S, value: bool) -> Self {
        self.properties
            .push((name.into(), PropertyValue::Boolean(value)));
        self
    }

    /// Set a matrix property
    pub fn with_property_matrix<S: Into<String>>(
        mut self,
        name: S,
        value: crate::types::Matrix4x4,
    ) -> Self {
        self.properties
            .push((name.into(), PropertyValue::Matrix(value)));
        self
    }

    /// Set properties from a PropertyStore
    pub fn with_property_store(mut self, store: PropertyStore) -> Self {
        self.properties.extend(store.properties);
        self
    }

    /// Set properties from a PropertyStore by reference
    pub fn with_property_store_ref(mut self, store: &PropertyStore) -> Self {
        self.properties.extend(store.properties().iter().cloned());
        self
    }

    /// Set a custom file system
    pub fn with_file_system(
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

    /// Import a scene from a file path
    pub fn import_file<P: AsRef<Path>>(self, path: P) -> Result<Scene> {
        let path_str = path.as_ref().to_string_lossy();
        let c_path = CString::new(path_str.as_ref())
            .map_err(|_| Error::invalid_parameter("Invalid file path"))?;

        // Create property store if we have properties
        let property_store = if self.properties.is_empty() {
            std::ptr::null_mut()
        } else {
            self.create_property_store()
        };

        // Create custom file I/O if specified
        let file_io = self
            .file_system
            .as_ref()
            .map(|fs| AssimpFileIO::new(fs.clone()).create_ai_file_io());
        let file_io_ptr: *mut sys::aiFileIO = file_io
            .as_ref()
            .map_or(std::ptr::null_mut(), |io| io as *const _ as *mut _);

        // Import the scene
        let scene_ptr = unsafe {
            if property_store.is_null() && file_io_ptr.is_null() {
                sys::aiImportFile(c_path.as_ptr(), self.post_process.as_raw())
            } else if file_io_ptr.is_null() {
                sys::aiImportFileExWithProperties(
                    c_path.as_ptr(),
                    self.post_process.as_raw(),
                    std::ptr::null_mut(),
                    property_store,
                )
            } else if property_store.is_null() {
                sys::aiImportFileEx(c_path.as_ptr(), self.post_process.as_raw(), file_io_ptr)
            } else {
                sys::aiImportFileExWithProperties(
                    c_path.as_ptr(),
                    self.post_process.as_raw(),
                    file_io_ptr,
                    property_store,
                )
            }
        };

        // Clean up property store
        if !property_store.is_null() {
            unsafe {
                sys::aiReleasePropertyStore(property_store);
            }
        }

        // Check if import was successful
        if scene_ptr.is_null() {
            return Err(Error::from_assimp());
        }

        // Create safe wrapper
        unsafe { Scene::from_raw(scene_ptr) }
    }

    /// Import a scene from memory buffer
    pub fn import_from_memory(self, data: &[u8], hint: Option<&str>) -> Result<Scene> {
        let hint_cstr = if let Some(h) = hint {
            Some(CString::new(h).map_err(|_| Error::invalid_parameter("Invalid hint"))?)
        } else {
            None
        };

        let hint_ptr = hint_cstr.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());

        // Create property store if we have properties
        let property_store = if self.properties.is_empty() {
            std::ptr::null_mut()
        } else {
            self.create_property_store()
        };

        // Import from memory
        let scene_ptr = unsafe {
            if property_store.is_null() {
                sys::aiImportFileFromMemory(
                    data.as_ptr() as *const std::os::raw::c_char,
                    data.len() as u32,
                    self.post_process.as_raw(),
                    hint_ptr,
                )
            } else {
                sys::aiImportFileFromMemoryWithProperties(
                    data.as_ptr() as *const std::os::raw::c_char,
                    data.len() as u32,
                    self.post_process.as_raw(),
                    hint_ptr,
                    property_store,
                )
            }
        };

        // Clean up property store
        if !property_store.is_null() {
            unsafe {
                sys::aiReleasePropertyStore(property_store);
            }
        }

        // Check if import was successful
        if scene_ptr.is_null() {
            return Err(Error::from_assimp());
        }

        // Create safe wrapper
        unsafe { Scene::from_raw(scene_ptr) }
    }

    /// Create a property store with the configured properties
    fn create_property_store(&self) -> *mut sys::aiPropertyStore {
        let store = unsafe { sys::aiCreatePropertyStore() };
        if store.is_null() {
            return std::ptr::null_mut();
        }

        for (name, value) in &self.properties {
            let c_name = match CString::new(name.as_str()) {
                Ok(name) => name,
                Err(_) => continue, // Skip invalid property names
            };

            unsafe {
                match value {
                    PropertyValue::Integer(v) => {
                        sys::aiSetImportPropertyInteger(store, c_name.as_ptr(), *v);
                    }
                    PropertyValue::Float(v) => {
                        sys::aiSetImportPropertyFloat(store, c_name.as_ptr(), *v);
                    }
                    PropertyValue::String(v) => {
                        if let Ok(c_value) = CString::new(v.as_str()) {
                            // Create aiString from the string value
                            let mut ai_string = sys::aiString {
                                length: v.len() as u32,
                                data: [0; 1024],
                            };

                            // Copy string data to aiString, ensuring we don't exceed buffer size
                            let bytes = c_value.as_bytes();
                            let copy_len = std::cmp::min(bytes.len(), 1023); // Leave space for null terminator

                            // Convert u8 bytes to c_char (i8 on Windows)
                            for (i, &byte) in bytes[..copy_len].iter().enumerate() {
                                ai_string.data[i] = byte as std::os::raw::c_char;
                            }
                            ai_string.data[copy_len] = 0; // Null terminator
                            ai_string.length = copy_len as u32;

                            sys::aiSetImportPropertyString(store, c_name.as_ptr(), &ai_string);
                        }
                    }
                    PropertyValue::Boolean(v) => {
                        sys::aiSetImportPropertyInteger(
                            store,
                            c_name.as_ptr(),
                            if *v { 1 } else { 0 },
                        );
                    }
                    PropertyValue::Matrix(v) => {
                        // Convert glam Mat4 to aiMatrix4x4
                        let ai_matrix = sys::aiMatrix4x4 {
                            a1: v.x_axis.x,
                            a2: v.y_axis.x,
                            a3: v.z_axis.x,
                            a4: v.w_axis.x,
                            b1: v.x_axis.y,
                            b2: v.y_axis.y,
                            b3: v.z_axis.y,
                            b4: v.w_axis.y,
                            c1: v.x_axis.z,
                            c2: v.y_axis.z,
                            c3: v.z_axis.z,
                            c4: v.w_axis.z,
                            d1: v.x_axis.w,
                            d2: v.y_axis.w,
                            d3: v.z_axis.w,
                            d4: v.w_axis.w,
                        };
                        sys::aiSetImportPropertyMatrix(store, c_name.as_ptr(), &ai_matrix);
                    }
                }
            }
        }

        store
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
    pub fn read_file<P: AsRef<Path>>(&self, _path: P) -> ImportBuilder {
        ImportBuilder::new()
    }

    /// Start building an import operation from memory
    pub fn read_from_memory(&self, _data: &[u8]) -> ImportBuilder {
        ImportBuilder::new()
    }

    /// Quick import with default settings
    pub fn import_file<P: AsRef<Path>>(&self, path: P) -> Result<Scene> {
        ImportBuilder::new().import_file(path)
    }

    /// Quick import from memory with default settings
    pub fn import_from_memory(&self, data: &[u8], hint: Option<&str>) -> Result<Scene> {
        ImportBuilder::new().import_from_memory(data, hint)
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
