//! Scene export functionality
//!
//! Prepare scene data during import or with [`Scene::apply_postprocess`] before exporting.
//! Post-processing properties belong to the scene's importer. Export properties configure
//! only the selected format, whose mandatory conversions are applied by Assimp.

use std::ffi::CString;
use std::path::Path;
use std::sync::Arc;

use crate::{
    bridge_properties::build_rust_properties,
    error::{Error, Result},
    ffi,
    importer::{PropertyStore, PropertyValue},
    io::{DefaultFileSystem, FileSystem, OwnedAiFileIO},
    ptr::SharedPtr,
    scene::Scene,
    sys,
    types::ai_string_to_string,
};

/// Common export property keys
///
/// These constants provide convenient access to commonly used Assimp export properties.
pub mod export_properties {
    /// FBX: Whether the exporter should interpret transparency factor as opacity.
    ///
    /// (AI_CONFIG_EXPORT_FBX_TRANSPARENCY_FACTOR_REFER_TO_OPACITY)
    pub const FBX_TRANSPARENCY_FACTOR_REFER_TO_OPACITY: &str =
        "EXPORT_FBX_TRANSPARENCY_FACTOR_REFER_TO_OPACITY";
}

#[cfg(test)]
mod export_properties_tests {
    use super::export_properties;

    fn c_key(bytes: &[u8]) -> &str {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        std::str::from_utf8(&bytes[..end]).expect("Assimp config key should be UTF-8")
    }

    #[test]
    fn export_property_keys_match_assimp_macros() {
        assert_eq!(
            export_properties::FBX_TRANSPARENCY_FACTOR_REFER_TO_OPACITY,
            c_key(crate::sys::AI_CONFIG_EXPORT_FBX_TRANSPARENCY_FACTOR_REFER_TO_OPACITY)
        );
    }
}

/// Builder for configuring and executing scene exports
///
/// Configure scene preparation through [`crate::importer::ImportBuilder`] or apply
/// post-processing to an exclusively owned [`Scene`] before exporting. The export
/// operation does not accept additional post-processing steps.
pub struct ExportBuilder {
    format_id: String,
    file_system: Option<std::sync::Arc<std::sync::Mutex<dyn FileSystem>>>,
    properties: PropertyStore,
}

impl std::fmt::Debug for ExportBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExportBuilder")
            .field("format_id", &self.format_id)
            .field("file_system", &self.file_system.is_some())
            .field("properties", &self.properties.len())
            .finish()
    }
}

impl ExportBuilder {
    /// Create a new export builder for the specified format
    pub fn new<S: Into<String>>(format_id: S) -> Self {
        Self {
            format_id: format_id.into(),
            file_system: None,
            properties: PropertyStore::new(),
        }
    }

    /// Set an exporter property, replacing any value with the same name and native type.
    pub fn with_property<S: Into<String>>(mut self, name: S, value: PropertyValue) -> Self {
        self.properties.set(name, value);
        self
    }

    /// Add properties from a PropertyStore.
    pub fn with_property_store(mut self, store: PropertyStore) -> Self {
        self.properties
            .extend(Vec::<(String, PropertyValue)>::from(store));
        self
    }

    /// Add properties from a PropertyStore by reference.
    pub fn with_property_store_ref(mut self, store: &PropertyStore) -> Self {
        self.properties.extend(store.properties().iter().cloned());
        self
    }

    /// Set an integer property.
    pub fn with_property_int<S: Into<String>>(self, name: S, value: i32) -> Self {
        self.with_property(name, PropertyValue::Integer(value))
    }

    /// Set a float property.
    pub fn with_property_float<S: Into<String>>(self, name: S, value: f32) -> Self {
        self.with_property(name, PropertyValue::Float(value))
    }

    /// Set a string property.
    pub fn with_property_string<S: Into<String>, V: Into<String>>(self, name: S, value: V) -> Self {
        self.with_property(name, PropertyValue::String(value.into()))
    }

    /// Set a boolean property.
    pub fn with_property_bool<S: Into<String>>(self, name: S, value: bool) -> Self {
        self.with_property(name, PropertyValue::Boolean(value))
    }

    /// Set a matrix property.
    pub fn with_property_matrix<S: Into<String>>(
        self,
        name: S,
        value: crate::types::Matrix4x4,
    ) -> Self {
        self.with_property(name, PropertyValue::Matrix(value))
    }

    /// Use a custom file system for exporting through the C++ bridge.
    ///
    /// This setting is only valid for file exports. Blob exports reject it.
    pub fn with_file_system<F>(self, file_system: F) -> Self
    where
        F: FileSystem + 'static,
    {
        self.with_file_system_shared(std::sync::Arc::new(std::sync::Mutex::new(file_system)))
    }

    /// Use a custom file system for exporting, from an explicitly shared handle.
    pub fn with_file_system_shared(
        mut self,
        file_system: std::sync::Arc<std::sync::Mutex<dyn FileSystem>>,
    ) -> Self {
        self.file_system = Some(file_system);
        self
    }

    /// Export the scene to a file.
    ///
    /// Uses [`DefaultFileSystem`] when no custom file system is configured, so write
    /// and flush failures are returned to the caller.
    /// Paths must be valid UTF-8 on every platform, including native 3MF output.
    /// 3MF requires the native file system because its ZIP writer bypasses custom I/O.
    /// Morph targets with sparse UV or color channels cannot be copied safely by Assimp.
    /// Empty or unsupported node metadata and mesh animation channels are also rejected
    /// because Assimp cannot deep-copy their ownership safely.
    pub fn export_to_file<P: AsRef<Path>>(self, scene: &Scene, path: P) -> Result<()> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| Error::invalid_parameter("Export file paths must be valid UTF-8"))?;
        let c_path =
            CString::new(path_str).map_err(|_| Error::invalid_parameter("Invalid file path"))?;
        let c_format = CString::new(self.format_id.as_str())
            .map_err(|_| Error::invalid_parameter("Invalid format ID"))?;

        let file_io = match self.file_system {
            Some(file_system) => Some(OwnedAiFileIO::new(file_system)),
            None if self.format_id != "3mf" => Some(OwnedAiFileIO::new(std::sync::Arc::new(
                std::sync::Mutex::new(DefaultFileSystem),
            ))),
            None => None,
        };
        let buffers = build_rust_properties(&self.properties)?;
        let result = unsafe {
            sys::aiExportSceneExWithPropertiesRust(
                scene.as_raw_sys(),
                c_format.as_ptr(),
                c_path.as_ptr(),
                file_io
                    .as_ref()
                    .map_or(std::ptr::null(), |io| io.as_ptr_sys()),
                0,
                buffers.ffi_props.as_ptr(),
                buffers.ffi_props.len(),
            )
        };

        if let Some(error) = file_io.as_ref().and_then(OwnedAiFileIO::take_error) {
            return Err(error);
        }

        match result {
            sys::aiReturn::aiReturn_SUCCESS => Ok(()),
            sys::aiReturn::aiReturn_OUTOFMEMORY => Err(Error::OutOfMemory),
            _ => Err(Error::export_failed(Error::from_bridge().to_string())),
        }
    }

    /// Export the scene to a blob in memory.
    ///
    /// A configured custom file system is rejected because blobs own their output.
    /// 3MF is unsupported because its ZIP writer writes directly to the host file system.
    /// Morph targets with sparse UV or color channels cannot be copied safely by Assimp.
    /// Empty or unsupported node metadata and mesh animation channels are also rejected.
    /// Native blob failures return [`Error::ExportFailed`] because Assimp's blob API
    /// does not expose an `aiReturn` status, including for out-of-memory failures.
    pub fn export_to_blob(self, scene: &Scene) -> Result<ExportBlob> {
        if self.file_system.is_some() {
            return Err(Error::invalid_parameter(
                "Custom file systems are only supported for file exports",
            ));
        }
        let c_format = CString::new(self.format_id.as_str())
            .map_err(|_| Error::invalid_parameter("Invalid format ID"))?;

        let buffers = build_rust_properties(&self.properties)?;
        let blob_ptr = unsafe {
            sys::aiExportSceneToBlobWithPropertiesRust(
                scene.as_raw_sys(),
                c_format.as_ptr(),
                0,
                buffers.ffi_props.as_ptr(),
                buffers.ffi_props.len(),
            )
        };

        if blob_ptr.is_null() {
            Err(Error::export_failed(Error::from_bridge().to_string()))
        } else {
            unsafe { ExportBlob::from_sys_ptr(blob_ptr) }
                .ok_or_else(|| Error::invalid_scene("Invalid export blob pointer"))
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
    ///
    /// # Safety
    /// A non-null, aligned pointer must be the root of an exclusively owned blob chain
    /// returned by Assimp. Ownership transfers here, including all reachable buffers.
    unsafe fn from_sys_ptr(blob_ptr: *const sys::aiExportDataBlob) -> Option<Self> {
        let blob_ptr = unsafe { SharedPtr::new(blob_ptr) }?;
        Some(Self {
            inner: Arc::new(ExportBlobInner { root: blob_ptr }),
        })
    }

    /// Create a view of the root blob in the chain.
    pub fn view(&self) -> ExportBlobView {
        ExportBlobView {
            inner: self.inner.clone(),
            blob_ptr: self.inner.root,
        }
    }

    #[inline]
    fn raw_root(&self) -> &sys::aiExportDataBlob {
        self.inner.root.as_ref()
    }

    /// Get the data as a byte slice
    pub fn data(&self) -> &[u8] {
        let blob = self.raw_root();
        unsafe { ffi::slice_from_ptr_len(self, blob.data as *const u8, blob.size) }
    }

    /// Get the size of the data
    pub fn size(&self) -> usize {
        self.raw_root().size
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
    #[inline]
    fn raw(&self) -> &sys::aiExportDataBlob {
        self.blob_ptr.as_ref()
    }

    /// Get the data as a byte slice.
    pub fn data(&self) -> &[u8] {
        let blob = self.raw();
        unsafe { ffi::slice_from_ptr_len(self, blob.data as *const u8, blob.size) }
    }

    /// Get the size of the data.
    pub fn size(&self) -> usize {
        self.raw().size
    }

    /// Get the name/hint for this blob.
    pub fn name(&self) -> String {
        let blob = self.raw();
        if blob.name.length == 0 {
            String::new()
        } else {
            ai_string_to_string(&blob.name)
        }
    }

    /// Check if this blob has a next blob (for multi-file exports).
    pub fn has_next(&self) -> bool {
        !self.raw().next.is_null()
    }

    /// Get the next blob in the chain.
    pub fn next(&self) -> Option<ExportBlobView> {
        let next = self.raw().next as *const sys::aiExportDataBlob;
        unsafe { SharedPtr::new(next) }.map(|blob_ptr| ExportBlobView {
            inner: self.inner.clone(),
            blob_ptr,
        })
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
        let next = current.as_ref().next as *const sys::aiExportDataBlob;
        self.current = unsafe { SharedPtr::new(next) };
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
    ///
    /// # Safety
    /// Non-null string fields must remain valid NUL-terminated strings during this call.
    pub(crate) unsafe fn from_raw(desc: &sys::aiExportFormatDesc) -> Self {
        Self {
            id: unsafe { crate::error::c_str_to_string_or_empty(desc.id) },
            description: unsafe { crate::error::c_str_to_string_or_empty(desc.description) },
            file_extension: unsafe { crate::error::c_str_to_string_or_empty(desc.fileExtension) },
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
    pub const COLLADA: &str = "collada";
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
    use crate::Importer;

    fn morph_scene(channels: usize) -> Scene {
        // Positions, two-component UVs, RGBA colors, then zero position deltas.
        let data = "AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AACAPwAAAAAAAAAAAACAPwAAAAAAAIA/AAAAAAAAgD8AAAAAAAAAAAAAgD8AAIA/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let mut attributes = String::from("\"POSITION\": 0");
        for channel in 0..channels {
            attributes.push_str(&format!(
                ", \"TEXCOORD_{channel}\": 1, \"COLOR_{channel}\": 2",
            ));
        }
        let gltf = format!(
            r#"{{
              "asset": {{ "version": "2.0" }},
              "buffers": [{{ "uri": "data:application/octet-stream;base64,{data}", "byteLength": 144 }}],
              "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
                {{ "buffer": 0, "byteOffset": 60, "byteLength": 48 }},
                {{ "buffer": 0, "byteOffset": 108, "byteLength": 36 }}
              ],
              "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0] }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC4" }},
                {{ "bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [0,0,0] }}
              ],
              "meshes": [{{ "weights": [0.5, 0.5], "primitives": [{{
                "attributes": {{ {attributes} }},
                "targets": [{{ "POSITION": 3 }}, {{ "POSITION": 3 }}]
              }}] }}],
              "nodes": [{{ "mesh": 0 }}],
              "scenes": [{{ "nodes": [0] }}],
              "scene": 0
            }}"#,
        );
        let import = || {
            Importer::new()
                .read_from_memory(gltf.as_bytes())
                .with_memory_hint("gltf")
                .with_post_process(crate::postprocess::PostProcessSteps::empty())
                .import()
                .expect("import native-owned morph fixture")
        };
        let scene = import();
        if channels > 0 {
            let donor = import();
            // SAFETY: Both scenes are exclusively owned here, with no live views.
            // glTF creates position-only targets. Transfer the donor mesh's native
            // arrays into target 1, keeping exactly one owner of every allocation.
            // The host mesh retains matching dense channels and the same vertex count.
            unsafe {
                let mesh = &mut **(*scene.as_raw_sys()).mMeshes;
                let anim = &mut **mesh.mAnimMeshes.add(1);
                let donor_mesh = &mut **(*donor.as_raw_sys()).mMeshes;
                assert!(anim.mTextureCoords.iter().all(|ptr| ptr.is_null()));
                assert!(anim.mColors.iter().all(|ptr| ptr.is_null()));
                std::mem::swap(&mut anim.mTextureCoords, &mut donor_mesh.mTextureCoords);
                std::mem::swap(&mut anim.mColors, &mut donor_mesh.mColors);
                donor_mesh.mNumUVComponents.fill(0);
            }
        }
        assert_eq!(scene.num_meshes(), 1);
        let mesh = scene.mesh(0).unwrap();
        assert_eq!(mesh.num_anim_meshes(), 2);
        for anim in mesh.anim_meshes() {
            assert_eq!(anim.vertices_raw().len(), 3);
        }
        let anim = mesh.anim_mesh(1).unwrap();
        for channel in 0..channels {
            assert_eq!(anim.texture_coords_raw(channel).len(), 3);
            assert_eq!(anim.vertex_colors_raw(channel).len(), 3);
        }
        scene
    }

    #[test]
    fn sparse_morph_channels_are_rejected_without_freeing_source_buffers() {
        for (colors, from, to) in [(false, 0, 2), (false, 1, 7), (true, 0, 2), (true, 1, 7)] {
            let scene = morph_scene(2);
            // SAFETY: No scene clones or views are alive here. Both the host mesh
            // and its second morph target keep each native allocation exactly once;
            // swapping slots preserves ownership and matching vertex components.
            unsafe {
                let raw_scene = &*scene.as_raw_sys();
                let mesh = &mut **raw_scene.mMeshes;
                let anim = &mut **mesh.mAnimMeshes.add(1);
                if colors {
                    mesh.mColors.swap(from, to);
                    anim.mColors.swap(from, to);
                } else {
                    mesh.mTextureCoords.swap(from, to);
                    mesh.mNumUVComponents.swap(from, to);
                    anim.mTextureCoords.swap(from, to);
                }
            }

            let mesh = scene.mesh(0).unwrap();
            let anim = mesh.anim_mesh(1).unwrap();
            let uv = anim.texture_coords_raw(to);
            let color = anim.vertex_colors_raw(to);
            let expected_uv = uv.to_vec();
            let expected_color = color.to_vec();
            let expected_kind = if colors {
                "vertex color"
            } else {
                "texture coordinate"
            };
            let expected_channel = if from == 0 { 1 } else { to };

            // Cover both entry points and the property/custom-I/O configurations
            // that previously selected different native export implementations.
            let blob_error = ExportBuilder::new("stl")
                .with_property_bool("EXPORT_POINT_CLOUDS", false)
                .export_to_blob(&scene)
                .err()
                .expect("reject sparse morph target before native copy");
            let file_error = ExportBuilder::new("stl")
                .with_file_system(crate::io::MemoryFileSystem::new())
                .export_to_file(&scene, "virtual/morph.stl")
                .expect_err("reject sparse morph target before native copy");
            for error in [blob_error, file_error] {
                let Error::ExportFailed { message } = error else {
                    panic!("expected ExportFailed, got {error}");
                };
                assert!(message.contains("morph target 1"), "{message}");
                assert!(message.contains(expected_kind), "{message}");
                assert!(
                    message.contains(&format!("channel {expected_channel}")),
                    "{message}"
                );
            }
            assert_eq!(uv, expected_uv);
            assert_eq!(color, expected_color);
        }
    }

    #[test]
    fn dense_and_position_only_morph_targets_still_export() {
        for channels in [0, 2] {
            let scene = morph_scene(channels);
            let mesh = scene.mesh(0).unwrap();
            let anim = mesh.anim_mesh(1).unwrap();
            let positions = anim.vertices_raw();
            let expected = positions.to_vec();

            let blob = ExportBuilder::new("stl")
                .export_to_blob(&scene)
                .expect("export morph targets whose channels can be safely copied");

            assert!(blob.data().starts_with(b"solid "));
            assert_eq!(positions, expected);
            for channel in 0..channels {
                assert_eq!(anim.texture_coords_raw(channel).len(), 3);
                assert_eq!(anim.vertex_colors_raw(channel).len(), 3);
            }
        }
    }

    #[test]
    fn test_exporter_creation() {
        let exporter = Exporter::new();
        let _builder = exporter.export_scene(formats::OBJ);
    }

    #[test]
    fn export_blob_rejects_unaligned_pointers() {
        let buf = [0u64; 8];
        let unaligned =
            unsafe { (buf.as_ptr() as *const u8).add(1) } as *const sys::aiExportDataBlob;
        assert!(unsafe { ExportBlob::from_sys_ptr(unaligned) }.is_none());
    }

    #[test]
    fn test_export_builder() {
        let builder = ExportBuilder::new(formats::OBJ);

        assert_eq!(builder.format_id, formats::OBJ);
    }

    #[test]
    fn test_format_constants() {
        assert_eq!(formats::OBJ, "obj");
        assert_eq!(formats::COLLADA, "collada");
        assert_eq!(formats::GLTF2, "gltf2");
    }

    #[cfg(feature = "export")]
    #[test]
    fn test_export_to_blob_with_properties() {
        // Minimal OBJ scene.
        let obj = b"o tri\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let scene = Importer::new()
            .import_from_memory(obj, Some("obj"))
            .expect("import OBJ scene");

        let blob = ExportBuilder::new(formats::OBJ)
            .with_property_bool(
                export_properties::FBX_TRANSPARENCY_FACTOR_REFER_TO_OPACITY,
                true,
            )
            .with_property_string("EXPORT_BLOB_NAME", "superseded\0name")
            .with_property_string("EXPORT_BLOB_NAME", "current.obj")
            .export_to_blob(&scene)
            .expect("export to blob with properties");

        assert!(blob.size() > 0);
        assert_eq!(blob.name(), "current.obj");
    }
}
