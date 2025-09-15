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
    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> ImportBuilder {
        ImportBuilder::new()
    }

    /// Start building an import operation from memory
    pub fn read_from_memory(&self, data: &[u8]) -> ImportBuilder {
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
