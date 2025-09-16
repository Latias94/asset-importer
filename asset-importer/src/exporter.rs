//! Scene export functionality

use std::ffi::CString;
use std::path::Path;

use crate::{
    error::{Error, Result},
    io::{AssimpFileIO, FileSystem},
    scene::Scene,
    sys,
};

/// Builder for configuring and executing scene exports
#[derive(Debug)]
pub struct ExportBuilder {
    format_id: String,
    preprocessing: u32,
    file_system: Option<std::sync::Arc<std::sync::Mutex<dyn FileSystem>>>,
}

impl ExportBuilder {
    /// Create a new export builder for the specified format
    pub fn new<S: Into<String>>(format_id: S) -> Self {
        Self {
            format_id: format_id.into(),
            preprocessing: 0,
            file_system: None,
        }
    }

    /// Set preprocessing steps to apply before export
    pub fn with_preprocessing(mut self, steps: u32) -> Self {
        self.preprocessing = steps;
        self
    }

    /// Use a custom file system for exporting (uses aiExportSceneEx)
    pub fn with_file_system(
        mut self,
        file_system: std::sync::Arc<std::sync::Mutex<dyn FileSystem>>,
    ) -> Self {
        self.file_system = Some(file_system);
        self
    }

    /// Export the scene to a file
    pub fn export_to_file<P: AsRef<Path>>(self, scene: &Scene, path: P) -> Result<()> {
        let path_str = path.as_ref().to_string_lossy();
        let c_path = CString::new(path_str.as_ref())
            .map_err(|_| Error::invalid_parameter("Invalid file path"))?;
        let c_format = CString::new(self.format_id.as_str())
            .map_err(|_| Error::invalid_parameter("Invalid format ID"))?;

        let result = if let Some(fs) = &self.file_system {
            let file_io = AssimpFileIO::new(fs.clone()).create_ai_file_io();
            unsafe {
                sys::aiExportSceneEx(
                    scene.as_raw(),
                    c_format.as_ptr(),
                    &file_io as *const _ as *mut _,
                    c_path.as_ptr(),
                    self.preprocessing,
                )
            }
        } else {
            unsafe {
                sys::aiExportScene(
                    scene.as_raw(),
                    c_format.as_ptr(),
                    c_path.as_ptr(),
                    self.preprocessing,
                )
            }
        };

        if result == sys::aiReturn::Type::aiReturn_SUCCESS {
            Ok(())
        } else {
            Err(Error::from_assimp())
        }
    }

    /// Export the scene to a blob in memory
    pub fn export_to_blob(self, scene: &Scene) -> Result<ExportBlob> {
        let c_format = CString::new(self.format_id.as_str())
            .map_err(|_| Error::invalid_parameter("Invalid format ID"))?;

        let blob_ptr = unsafe {
            sys::aiExportSceneToBlob(scene.as_raw(), c_format.as_ptr(), self.preprocessing)
        };

        if blob_ptr.is_null() {
            Err(Error::from_assimp())
        } else {
            Ok(ExportBlob::from_raw(blob_ptr))
        }
    }
}

/// A blob containing exported scene data
pub struct ExportBlob {
    blob_ptr: *const sys::aiExportDataBlob,
}

impl ExportBlob {
    /// Create an ExportBlob from a raw Assimp blob pointer
    fn from_raw(blob_ptr: *const sys::aiExportDataBlob) -> Self {
        Self { blob_ptr }
    }

    /// Get the data as a byte slice
    pub fn data(&self) -> &[u8] {
        unsafe {
            let blob = &*self.blob_ptr;
            std::slice::from_raw_parts(blob.data as *const u8, blob.size as usize)
        }
    }

    /// Get the size of the data
    pub fn size(&self) -> usize {
        unsafe { (*self.blob_ptr).size as usize }
    }

    /// Get the name/hint for this blob
    pub fn name(&self) -> String {
        unsafe {
            let blob = &*self.blob_ptr;
            let name_data = blob.name.data.as_ptr() as *const i8;
            if name_data.is_null() || blob.name.length == 0 {
                String::new()
            } else {
                std::ffi::CStr::from_ptr(name_data)
                    .to_string_lossy()
                    .into_owned()
            }
        }
    }

    /// Check if this blob has a next blob (for multi-file exports)
    pub fn has_next(&self) -> bool {
        unsafe { !(*self.blob_ptr).next.is_null() }
    }

    /// Get the next blob in the chain
    pub fn next(&self) -> Option<ExportBlob> {
        unsafe {
            let blob = &*self.blob_ptr;
            if blob.next.is_null() {
                None
            } else {
                Some(ExportBlob::from_raw(blob.next))
            }
        }
    }
}

impl Drop for ExportBlob {
    fn drop(&mut self) {
        unsafe {
            sys::aiReleaseExportBlob(self.blob_ptr);
        }
    }
}

/// Description of an export format
#[derive(Debug, Clone)]
pub struct ExportFormatDesc {
    /// Format identifier
    pub id: String,
    /// Human-readable description
    pub description: String,
    /// File extension
    pub file_extension: String,
}

impl ExportFormatDesc {
    /// Create from raw Assimp export format description
    pub(crate) fn from_raw(desc: &sys::aiExportFormatDesc) -> Self {
        unsafe {
            let id = std::ffi::CStr::from_ptr(desc.id)
                .to_string_lossy()
                .into_owned();
            let description = std::ffi::CStr::from_ptr(desc.description)
                .to_string_lossy()
                .into_owned();
            let file_extension = std::ffi::CStr::from_ptr(desc.fileExtension)
                .to_string_lossy()
                .into_owned();

            Self {
                id,
                description,
                file_extension,
            }
        }
    }
}

/// Main exporter interface
#[derive(Debug)]
pub struct Exporter;

impl Exporter {
    /// Create a new exporter
    pub fn new() -> Self {
        Self
    }

    /// Start building an export operation for the specified format
    pub fn export_scene<S: Into<String>>(&self, format_id: S) -> ExportBuilder {
        ExportBuilder::new(format_id)
    }

    /// Quick export with default settings
    pub fn export_to_file<P: AsRef<Path>, S: Into<String>>(
        &self,
        scene: &Scene,
        format_id: S,
        path: P,
    ) -> Result<()> {
        ExportBuilder::new(format_id).export_to_file(scene, path)
    }

    /// Quick export to blob with default settings
    pub fn export_to_blob<S: Into<String>>(
        &self,
        scene: &Scene,
        format_id: S,
    ) -> Result<ExportBlob> {
        ExportBuilder::new(format_id).export_to_blob(scene)
    }

    /// Get all available export formats
    pub fn get_export_formats(&self) -> Vec<ExportFormatDesc> {
        crate::get_export_formats()
    }

    /// Check if a format is supported for export
    pub fn is_format_supported<S: AsRef<str>>(&self, format_id: S) -> bool {
        self.get_export_formats()
            .iter()
            .any(|desc| desc.id == format_id.as_ref())
    }
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Common export format identifiers
pub mod formats {
    /// Wavefront OBJ format
    pub const OBJ: &str = "obj";
    /// COLLADA format
    pub const COLLADA: &str = "dae";
    /// Stanford PLY format
    pub const PLY: &str = "ply";
    /// STL format
    pub const STL: &str = "stl";
    /// glTF 2.0 format
    pub const GLTF2: &str = "gltf2";
    /// glTF 2.0 binary format
    pub const GLB2: &str = "glb2";
    /// Autodesk FBX format (if supported)
    pub const FBX: &str = "fbx";
    /// 3D Studio Max 3DS format
    pub const _3DS: &str = "3ds";
    /// X3D format
    pub const X3D: &str = "x3d";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exporter_creation() {
        let exporter = Exporter::new();
        let _builder = exporter.export_scene(formats::OBJ);
    }

    #[test]
    fn test_export_builder() {
        let builder = ExportBuilder::new(formats::OBJ).with_preprocessing(0);

        assert_eq!(builder.format_id, formats::OBJ);
        assert_eq!(builder.preprocessing, 0);
    }

    #[test]
    fn test_format_constants() {
        assert_eq!(formats::OBJ, "obj");
        assert_eq!(formats::COLLADA, "dae");
        assert_eq!(formats::GLTF2, "gltf2");
    }
}
