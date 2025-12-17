//! Scene export functionality

use std::ffi::CString;
use std::path::Path;
use std::sync::Arc;

use crate::{
    error::{Error, Result},
    ffi,
    io::{AssimpFileIO, FileSystem},
    ptr::SharedPtr,
    scene::Scene,
    sys,
    types::ai_string_to_string,
};

/// Builder for configuring and executing scene exports
pub struct ExportBuilder {
    format_id: String,
    preprocessing: u32,
    file_system: Option<std::sync::Arc<std::sync::Mutex<dyn FileSystem>>>,
}

impl std::fmt::Debug for ExportBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExportBuilder")
            .field("format_id", &self.format_id)
            .field("preprocessing", &self.preprocessing)
            .field("file_system", &self.file_system.is_some())
            .finish()
    }
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
            let mut file_io = AssimpFileIO::new(fs.clone()).create_ai_file_io();
            unsafe {
                sys::aiExportSceneEx(
                    scene.as_raw_sys(),
                    c_format.as_ptr(),
                    c_path.as_ptr(),
                    file_io.as_mut_ptr_sys(),
                    self.preprocessing,
                )
            }
        } else {
            unsafe {
                sys::aiExportScene(
                    scene.as_raw_sys(),
                    c_format.as_ptr(),
                    c_path.as_ptr(),
                    self.preprocessing,
                )
            }
        };

        if result == sys::aiReturn::aiReturn_SUCCESS {
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
            sys::aiExportSceneToBlob(scene.as_raw_sys(), c_format.as_ptr(), self.preprocessing)
        };

        if blob_ptr.is_null() {
            Err(Error::from_assimp())
        } else {
            Ok(ExportBlob::from_raw(blob_ptr))
        }
    }
}

/// A blob containing exported scene data
#[derive(Clone)]
pub struct ExportBlob {
    inner: Arc<ExportBlobInner>,
}

impl ExportBlob {
    /// Create an ExportBlob from a raw Assimp blob pointer
    fn from_raw(blob_ptr: *const sys::aiExportDataBlob) -> Self {
        debug_assert!(!blob_ptr.is_null());
        let blob_ptr = unsafe { SharedPtr::new_unchecked(blob_ptr) };
        Self {
            inner: Arc::new(ExportBlobInner { root: blob_ptr }),
        }
    }

    /// Create a view of the root blob in the chain.
    pub fn view(&self) -> ExportBlobView {
        ExportBlobView {
            inner: self.inner.clone(),
            blob_ptr: self.inner.root,
        }
    }

    /// Get the data as a byte slice
    pub fn data(&self) -> &[u8] {
        unsafe {
            let blob = &*self.inner.root.as_ptr();
            ffi::slice_from_ptr_len(self, blob.data as *const u8, blob.size)
        }
    }

    /// Get the size of the data
    pub fn size(&self) -> usize {
        unsafe { (*self.inner.root.as_ptr()).size }
    }

    /// Get the name/hint for this blob
    pub fn name(&self) -> String {
        self.view().name()
    }

    /// Check if this blob has a next blob (for multi-file exports)
    pub fn has_next(&self) -> bool {
        self.view().has_next()
    }

    /// Get the next blob in the chain
    pub fn next(&self) -> Option<ExportBlobView> {
        self.view().next()
    }

    /// Iterate over all blobs in the chain (primary + auxiliaries).
    pub fn iter(&self) -> ExportBlobIterator {
        ExportBlobIterator {
            inner: self.inner.clone(),
            current: Some(self.inner.root),
        }
    }
}

#[derive(Debug)]
struct ExportBlobInner {
    root: SharedPtr<sys::aiExportDataBlob>,
}

impl Drop for ExportBlobInner {
    fn drop(&mut self) {
        unsafe {
            sys::aiReleaseExportBlob(self.root.as_ptr());
        }
    }
}

/// A non-owning view into an export blob inside a blob chain.
#[derive(Clone)]
pub struct ExportBlobView {
    inner: Arc<ExportBlobInner>,
    blob_ptr: SharedPtr<sys::aiExportDataBlob>,
}

impl ExportBlobView {
    /// Get the data as a byte slice.
    pub fn data(&self) -> &[u8] {
        unsafe {
            let blob = &*self.blob_ptr.as_ptr();
            ffi::slice_from_ptr_len(self, blob.data as *const u8, blob.size)
        }
    }

    /// Get the size of the data.
    pub fn size(&self) -> usize {
        unsafe { (*self.blob_ptr.as_ptr()).size }
    }

    /// Get the name/hint for this blob.
    pub fn name(&self) -> String {
        unsafe {
            let blob = &*self.blob_ptr.as_ptr();
            if blob.name.length == 0 {
                String::new()
            } else {
                ai_string_to_string(&blob.name)
            }
        }
    }

    /// Check if this blob has a next blob (for multi-file exports).
    pub fn has_next(&self) -> bool {
        unsafe { !(*self.blob_ptr.as_ptr()).next.is_null() }
    }

    /// Get the next blob in the chain.
    pub fn next(&self) -> Option<ExportBlobView> {
        unsafe {
            let next = (*self.blob_ptr.as_ptr()).next as *const sys::aiExportDataBlob;
            SharedPtr::new(next).map(|blob_ptr| ExportBlobView {
                inner: self.inner.clone(),
                blob_ptr,
            })
        }
    }
}

/// Iterator over blobs in an export blob chain.
pub struct ExportBlobIterator {
    inner: Arc<ExportBlobInner>,
    current: Option<SharedPtr<sys::aiExportDataBlob>>,
}

impl Iterator for ExportBlobIterator {
    type Item = ExportBlobView;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        unsafe {
            let next = (*current.as_ptr()).next as *const sys::aiExportDataBlob;
            self.current = SharedPtr::new(next);
        }
        Some(ExportBlobView {
            inner: self.inner.clone(),
            blob_ptr: current,
        })
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
        Self {
            id: crate::error::c_str_to_string_or_empty(desc.id),
            description: crate::error::c_str_to_string_or_empty(desc.description),
            file_extension: crate::error::c_str_to_string_or_empty(desc.fileExtension),
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

    /// Iterate all available export formats without allocating a `Vec`.
    pub fn get_export_formats_iter(&self) -> crate::ExportFormatDescIterator {
        crate::get_export_formats_iter()
    }

    /// Check if a format is supported for export
    pub fn is_format_supported<S: AsRef<str>>(&self, format_id: S) -> bool {
        self.get_export_formats_iter()
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
