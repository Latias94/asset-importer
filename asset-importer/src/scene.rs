//! Scene representation and management

use std::sync::Arc;

use crate::{
    animation::Animation,
    camera::Camera,
    error::{Error, Result, bridge_error_string_or_default},
    ffi,
    importer::{Importer, PropertyStore},
    io::OwnedAiFileIO,
    light::Light,
    material::Material,
    mesh::Mesh,
    metadata::Metadata,
    node::Node,
    postprocess::PostProcessSteps,
    ptr::SharedPtr,
    sys,
    texture::{Texture, TextureIterator},
};

/// Memory usage information for a scene
///
/// This structure provides detailed information about the memory consumption
/// of different components in an imported scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryInfo {
    /// Storage allocated for texture data (in bytes)
    pub textures: u32,
    /// Storage allocated for material data (in bytes)
    pub materials: u32,
    /// Storage allocated for mesh data (in bytes)
    pub meshes: u32,
    /// Storage allocated for node data (in bytes)
    pub nodes: u32,
    /// Storage allocated for animation data (in bytes)
    pub animations: u32,
    /// Storage allocated for camera data (in bytes)
    pub cameras: u32,
    /// Storage allocated for light data (in bytes)
    pub lights: u32,
    /// Total storage allocated for the full import (in bytes)
    pub total: u32,
}

impl MemoryInfo {
    /// Create a new empty memory info
    pub fn new() -> Self {
        Self {
            textures: 0,
            materials: 0,
            meshes: 0,
            nodes: 0,
            animations: 0,
            cameras: 0,
            lights: 0,
            total: 0,
        }
    }

    /// Get the total memory usage in bytes
    pub fn total_bytes(&self) -> u32 {
        self.total
    }

    /// Get the total memory usage in kilobytes
    pub fn total_kb(&self) -> f64 {
        self.total as f64 / 1024.0
    }

    /// Get the total memory usage in megabytes
    pub fn total_mb(&self) -> f64 {
        self.total as f64 / (1024.0 * 1024.0)
    }

    /// Get a breakdown of memory usage by component
    pub fn breakdown(&self) -> Vec<(&'static str, u32)> {
        vec![
            ("Textures", self.textures),
            ("Materials", self.materials),
            ("Meshes", self.meshes),
            ("Nodes", self.nodes),
            ("Animations", self.animations),
            ("Cameras", self.cameras),
            ("Lights", self.lights),
        ]
    }
}

impl Default for MemoryInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// A 3D scene containing meshes, materials, animations, and other assets.
///
/// ## Thread safety
/// `Scene` and all scene-backed view types (`Mesh`, `Material`, `Node`, `Texture`, etc.) are
/// `Send + Sync` and can be used across threads for *read-only* access.
///
/// Unlike lifetime-tied view types, scene-backed views in this crate keep the owning
/// scene alive by holding a cheap clone of `Scene` internally. This makes views effectively
/// `'static` (as long as you own the view value) and avoids borrow-checker friction in
/// async and multithreaded code.
///
/// This guarantee relies on the safe API treating the imported Assimp scene as immutable.
/// If you call into raw Assimp bindings (`asset_importer::sys` with feature `raw-sys`, or the
/// `asset-importer-sys` crate) and mutate internal pointers yourself, you can
/// violate this contract and cause undefined behavior.
#[derive(Clone, Debug)]
pub struct Scene {
    inner: Arc<SceneInner>,
}

#[derive(Debug)]
pub(crate) struct SceneInner {
    scene_ptr: SharedPtr<sys::aiScene>,
    release_kind: SceneRelease,
    // Dropped after the native importer, including any streams it closes during destruction.
    file_io: Option<OwnedAiFileIO>,
}

#[derive(Debug)]
enum SceneRelease {
    /// Owns the C++ importer and its scene through the Rust bridge.
    Importer(SharedPtr<sys::aiRustScene>),
    /// Scene returned by aiImportFile* family, free with aiReleaseImport
    #[cfg(feature = "raw-sys")]
    ReleaseImport,
    /// Scene created via aiCopyScene, free with aiFreeScene
    #[cfg(feature = "raw-sys")]
    FreeScene,
}

impl Scene {
    /// Adopt ownership of a bridge import and prioritize custom I/O errors.
    ///
    /// # Safety
    /// `owner` must be returned by a bridge import and not yet owned or released elsewhere.
    /// `file_io` must own the callback table supplied to that import, if any.
    pub(crate) unsafe fn from_importer(
        owner: *mut sys::aiRustScene,
        file_io: Option<OwnedAiFileIO>,
    ) -> Result<Self> {
        if owner.is_null() {
            return Err(file_io
                .as_ref()
                .and_then(OwnedAiFileIO::take_error)
                .unwrap_or_else(Error::import_failed_from_bridge));
        }
        let owner = unsafe { SharedPtr::new(owner) }.ok_or(Error::NullPointer)?;
        let scene_ptr = unsafe { sys::aiGetSceneRust(owner.as_ptr()) };
        let Some(scene_ptr) = (unsafe { SharedPtr::new(scene_ptr) }) else {
            unsafe { sys::aiReleaseSceneRust(owner.as_ptr().cast_mut()) };
            return Err(file_io
                .as_ref()
                .and_then(OwnedAiFileIO::take_error)
                .unwrap_or(Error::NullPointer));
        };
        let scene = Self {
            inner: Arc::new(SceneInner {
                scene_ptr,
                release_kind: SceneRelease::Importer(owner),
                file_io,
            }),
        };
        if let Some(error) = scene
            .inner
            .file_io
            .as_ref()
            .and_then(OwnedAiFileIO::take_error)
        {
            return Err(error);
        }
        Ok(scene)
    }

    /// Create a Scene from a raw Assimp scene pointer
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `scene_ptr` is a valid pointer to an aiScene
    /// - The scene was returned by an aiImportFile* C API function and still has its
    ///   original importer, which will be freed with aiReleaseImport
    /// - Exclusive ownership of the scene and its importer is transferred to this Scene;
    ///   no external code may release or mutate the scene or its reachable data
    #[cfg(feature = "raw-sys")]
    pub unsafe fn from_raw_import(scene_ptr: *const sys::aiScene) -> Result<Self> {
        let scene_ptr = unsafe { SharedPtr::new(scene_ptr) }.ok_or(Error::NullPointer)?;

        Ok(Self {
            inner: Arc::new(SceneInner {
                scene_ptr,
                release_kind: SceneRelease::ReleaseImport,
                file_io: None,
            }),
        })
    }

    /// Create a Scene from a deep-copied Assimp scene pointer (aiCopyScene)
    /// The scene will be freed with aiFreeScene.
    ///
    /// # Safety
    /// Caller must ensure `scene_ptr` is valid and was allocated by aiCopyScene.
    /// Exclusive ownership is transferred to this Scene; no external code may release
    /// or mutate the scene or its reachable data.
    #[cfg(feature = "raw-sys")]
    pub unsafe fn from_raw_copied(scene_ptr: *const sys::aiScene) -> Result<Self> {
        let scene_ptr = unsafe { SharedPtr::new(scene_ptr) }.ok_or(Error::NullPointer)?;
        Ok(Self {
            inner: Arc::new(SceneInner {
                scene_ptr,
                release_kind: SceneRelease::FreeScene,
                file_io: None,
            }),
        })
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw_sys(&self) -> *const sys::aiScene {
        self.inner.scene_ptr.as_ptr()
    }

    /// Get the raw scene pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> *const sys::aiScene {
        self.as_raw_sys()
    }

    #[inline]
    fn raw(&self) -> &sys::aiScene {
        self.inner.scene_ptr.as_ref()
    }

    /// Apply Assimp post-processing to this scene.
    ///
    /// This consumes the scene and returns the updated scene on success:
    /// `scene = scene.apply_postprocess(flags)?;`.
    ///
    /// Requires exclusive ownership of a scene created by this crate's import builder.
    /// Returns an error if any scene clones, scene-backed views, or owning iterators
    /// are still alive, or if the scene was adopted from a raw pointer. Set post-processing
    /// steps before importing when you need to keep existing views alive.
    ///
    /// The importer owns cleanup even when Assimp invalidates its scene on failure.
    pub fn apply_postprocess(self, flags: crate::postprocess::PostProcessSteps) -> Result<Self> {
        flags.validate().map_err(Error::invalid_parameter)?;
        let mut inner = Arc::try_unwrap(self.inner).map_err(|_| {
            Error::invalid_parameter(
                "Post-processing requires exclusive scene ownership; drop all scene clones, views, and iterators first",
            )
        })?;
        #[allow(irrefutable_let_patterns)]
        let SceneRelease::Importer(owner) = &inner.release_kind else {
            return Err(Error::invalid_parameter(
                "Post-processing requires a scene created by the import builder; raw scenes have no bridge-owned importer",
            ));
        };
        let ok =
            unsafe { sys::aiApplyPostProcessingRust(owner.as_ptr().cast_mut(), flags.as_raw()) };
        if let Some(error) = inner.file_io.as_ref().and_then(OwnedAiFileIO::take_error) {
            return Err(error);
        }
        if !ok {
            let error = Error::invalid_scene(bridge_error_string_or_default());
            let file_io = inner.file_io.take();
            drop(inner);
            return Err(file_io
                .as_ref()
                .and_then(OwnedAiFileIO::take_error)
                .unwrap_or(error));
        }

        let new_ptr = unsafe { sys::aiGetSceneRust(owner.as_ptr()) };
        inner.scene_ptr = unsafe { SharedPtr::new(new_ptr) }.ok_or(Error::NullPointer)?;
        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Load a scene from a file with default settings
    ///
    /// This is a convenience method that provides a russimp-compatible interface.
    /// For more control, use `Importer::new().read_file(path).import()`.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Importer::new().import_file(path)
    }

    /// Load a scene from a file with post-processing steps
    ///
    /// This is a convenience method that provides a russimp-compatible interface.
    /// For more control, use the `Importer` and `ImportBuilder` APIs.
    pub fn from_file_with_flags<P: AsRef<std::path::Path>>(
        path: P,
        post_process: PostProcessSteps,
    ) -> Result<Self> {
        Importer::new()
            .read_file(&path)
            .with_post_process(post_process)
            .import()
    }

    /// Load a scene from a file with properties and post-processing steps
    ///
    /// This is a convenience method that provides a russimp-compatible interface.
    /// For more control, use the `Importer` and `ImportBuilder` APIs.
    ///
    /// # Example
    /// ```rust,no_run
    /// use asset_importer::{Scene, PropertyStore, postprocess::PostProcessSteps, import_properties};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut props = PropertyStore::new();
    /// props.set_int(import_properties::FBX_PRESERVE_PIVOTS, 0);
    ///
    /// let scene = Scene::from_file_with_props(
    ///     "model.fbx",
    ///     PostProcessSteps::TRIANGULATE | PostProcessSteps::GEN_SMOOTH_NORMALS,
    ///     &props
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_file_with_props<P: AsRef<std::path::Path>>(
        path: P,
        post_process: PostProcessSteps,
        props: &PropertyStore,
    ) -> Result<Self> {
        Importer::new()
            .read_file(&path)
            .with_post_process(post_process)
            .with_property_store_ref(props)
            .import()
    }

    /// Load a scene from memory with default settings
    ///
    /// This is a convenience method that provides a russimp-compatible interface.
    pub fn from_memory(data: &[u8], hint: Option<&str>) -> Result<Self> {
        Importer::new().import_from_memory(data, hint)
    }

    /// Load a scene from memory with post-processing steps
    ///
    /// This is a convenience method that provides a russimp-compatible interface.
    pub fn from_memory_with_flags(
        data: &[u8],
        hint: Option<&str>,
        post_process: PostProcessSteps,
    ) -> Result<Self> {
        Importer::new()
            .read_from_memory(data)
            .with_memory_hint_opt(hint)
            .with_post_process(post_process)
            .import()
    }

    /// Load a scene from memory with properties and post-processing steps
    ///
    /// This is a convenience method that provides a russimp-compatible interface.
    pub fn from_memory_with_props(
        data: &[u8],
        hint: Option<&str>,
        post_process: PostProcessSteps,
        props: &PropertyStore,
    ) -> Result<Self> {
        Importer::new()
            .read_from_memory(data)
            .with_memory_hint_opt(hint)
            .with_post_process(post_process)
            .with_property_store_ref(props)
            .import()
    }

    /// Get the scene flags
    pub fn flags(&self) -> u32 {
        self.raw().mFlags
    }

    /// Check if the scene is incomplete
    pub fn is_incomplete(&self) -> bool {
        self.flags() & sys::AI_SCENE_FLAGS_INCOMPLETE != 0
    }

    /// Check if the scene was validated
    pub fn is_validated(&self) -> bool {
        self.flags() & sys::AI_SCENE_FLAGS_VALIDATED != 0
    }

    /// Check if the scene contains validation warnings
    pub fn has_validation_warnings(&self) -> bool {
        self.flags() & sys::AI_SCENE_FLAGS_VALIDATION_WARNING != 0
    }

    /// Get memory requirements for this scene
    ///
    /// Returns information about the memory consumption of different components
    /// of the imported scene.
    ///
    /// Returns an error for scenes adopted from deep-copied raw pointers, which have no importer.
    pub fn memory_requirements(&self) -> Result<MemoryInfo> {
        let mut info = sys::aiMemoryInfo {
            textures: 0,
            materials: 0,
            meshes: 0,
            nodes: 0,
            animations: 0,
            cameras: 0,
            lights: 0,
            total: 0,
        };

        match &self.inner.release_kind {
            SceneRelease::Importer(owner) => {
                if !unsafe { sys::aiGetMemoryRequirementsRust(owner.as_ptr(), &mut info) } {
                    return Err(Error::from_bridge());
                }
            }
            #[cfg(feature = "raw-sys")]
            SceneRelease::ReleaseImport => unsafe {
                sys::aiGetMemoryRequirements(self.as_raw_sys(), &mut info);
            },
            #[cfg(feature = "raw-sys")]
            SceneRelease::FreeScene => {
                return Err(Error::invalid_parameter(
                    "Memory requirements are unavailable for copied raw scenes without an importer",
                ));
            }
        }

        Ok(MemoryInfo {
            textures: info.textures,
            materials: info.materials,
            meshes: info.meshes,
            nodes: info.nodes,
            animations: info.animations,
            cameras: info.cameras,
            lights: info.lights,
            total: info.total,
        })
    }

    /// Check if the scene is non-verbose
    pub fn is_non_verbose(&self) -> bool {
        self.flags() & sys::AI_SCENE_FLAGS_NON_VERBOSE_FORMAT != 0
    }

    /// Check if terrain patches are present
    pub fn has_terrain(&self) -> bool {
        self.flags() & sys::AI_SCENE_FLAGS_TERRAIN != 0
    }

    /// Get the root node of the scene
    pub fn root_node(&self) -> Option<Node> {
        unsafe { Node::from_sys_ptr(self.clone(), self.raw().mRootNode) }
    }

    /// Get the number of meshes in the scene
    pub fn num_meshes(&self) -> usize {
        let scene = self.raw();
        if scene.mMeshes.is_null() {
            0
        } else {
            scene.mNumMeshes as usize
        }
    }

    /// Get a mesh by index
    pub fn mesh(&self, index: usize) -> Option<Mesh> {
        if index >= self.num_meshes() {
            return None;
        }

        let scene = self.raw();
        let mesh_ptr =
            unsafe { ffi::ptr_array_get(self, scene.mMeshes, scene.mNumMeshes as usize, index) }?;
        unsafe { Mesh::from_sys_ptr(self.clone(), mesh_ptr) }
    }

    /// Get an iterator over all meshes
    pub fn meshes(&self) -> MeshIterator {
        MeshIterator {
            scene: self.clone(),
            index: 0,
        }
    }

    /// Get the number of materials in the scene
    pub fn num_materials(&self) -> usize {
        let scene = self.raw();
        if scene.mMaterials.is_null() {
            0
        } else {
            scene.mNumMaterials as usize
        }
    }

    /// Get a material by index
    pub fn material(&self, index: usize) -> Option<Material> {
        if index >= self.num_materials() {
            return None;
        }

        let scene = self.raw();
        let material_ptr = unsafe {
            ffi::ptr_array_get(self, scene.mMaterials, scene.mNumMaterials as usize, index)
        }?;
        unsafe { Material::from_sys_ptr(self.clone(), material_ptr) }
    }

    /// Get an iterator over all materials
    pub fn materials(&self) -> MaterialIterator {
        MaterialIterator {
            scene: self.clone(),
            index: 0,
        }
    }

    /// Get the number of animations in the scene
    pub fn num_animations(&self) -> usize {
        let scene = self.raw();
        if scene.mAnimations.is_null() {
            0
        } else {
            scene.mNumAnimations as usize
        }
    }

    /// Get an animation by index
    pub fn animation(&self, index: usize) -> Option<Animation> {
        if index >= self.num_animations() {
            return None;
        }

        let scene = self.raw();
        let animation_ptr = unsafe {
            ffi::ptr_array_get(
                self,
                scene.mAnimations,
                scene.mNumAnimations as usize,
                index,
            )
        }?;
        unsafe { Animation::from_sys_ptr(self.clone(), animation_ptr) }
    }

    /// Get an iterator over all animations
    pub fn animations(&self) -> AnimationIterator {
        AnimationIterator {
            scene: self.clone(),
            index: 0,
        }
    }

    /// Get the number of cameras in the scene
    pub fn num_cameras(&self) -> usize {
        let scene = self.raw();
        if scene.mCameras.is_null() {
            0
        } else {
            scene.mNumCameras as usize
        }
    }

    /// Get a camera by index
    pub fn camera(&self, index: usize) -> Option<Camera> {
        if index >= self.num_cameras() {
            return None;
        }

        let scene = self.raw();
        let camera_ptr =
            unsafe { ffi::ptr_array_get(self, scene.mCameras, scene.mNumCameras as usize, index) }?;
        unsafe { Camera::from_sys_ptr(self.clone(), camera_ptr) }
    }

    /// Get an iterator over all cameras
    pub fn cameras(&self) -> CameraIterator {
        CameraIterator {
            scene: self.clone(),
            index: 0,
        }
    }

    /// Get the number of lights in the scene
    pub fn num_lights(&self) -> usize {
        let scene = self.raw();
        if scene.mLights.is_null() {
            0
        } else {
            scene.mNumLights as usize
        }
    }

    /// Get a light by index
    pub fn light(&self, index: usize) -> Option<Light> {
        if index >= self.num_lights() {
            return None;
        }

        let scene = self.raw();
        let light_ptr =
            unsafe { ffi::ptr_array_get(self, scene.mLights, scene.mNumLights as usize, index) }?;
        unsafe { Light::from_sys_ptr(self.clone(), light_ptr) }
    }

    /// Get an iterator over all lights
    pub fn lights(&self) -> LightIterator {
        LightIterator {
            scene: self.clone(),
            index: 0,
        }
    }
}

impl Drop for SceneInner {
    fn drop(&mut self) {
        unsafe {
            match &self.release_kind {
                SceneRelease::Importer(owner) => {
                    sys::aiReleaseSceneRust(owner.as_ptr().cast_mut());
                }
                #[cfg(feature = "raw-sys")]
                SceneRelease::ReleaseImport => sys::release_import(self.scene_ptr.as_ptr()),
                #[cfg(feature = "raw-sys")]
                SceneRelease::FreeScene => sys::aiFreeScene(self.scene_ptr.as_ptr()),
            }
        }
    }
}

/// Iterator over meshes in a scene
pub struct MeshIterator {
    scene: Scene,
    index: usize,
}

impl Iterator for MeshIterator {
    type Item = Mesh;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.scene.num_meshes() {
            let idx = self.index;
            self.index += 1;
            if let Some(mesh) = self.scene.mesh(idx) {
                return Some(mesh);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scene.num_meshes().saturating_sub(self.index);
        (0, Some(remaining))
    }
}

/// Iterator over materials in a scene
pub struct MaterialIterator {
    scene: Scene,
    index: usize,
}

impl Iterator for MaterialIterator {
    type Item = Material;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.scene.num_materials() {
            let idx = self.index;
            self.index += 1;
            if let Some(material) = self.scene.material(idx) {
                return Some(material);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scene.num_materials().saturating_sub(self.index);
        (0, Some(remaining))
    }
}

/// Iterator over animations in a scene
pub struct AnimationIterator {
    scene: Scene,
    index: usize,
}

impl Iterator for AnimationIterator {
    type Item = Animation;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.scene.num_animations() {
            let idx = self.index;
            self.index += 1;
            if let Some(animation) = self.scene.animation(idx) {
                return Some(animation);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scene.num_animations().saturating_sub(self.index);
        (0, Some(remaining))
    }
}

/// Iterator over cameras in a scene
pub struct CameraIterator {
    scene: Scene,
    index: usize,
}

impl Iterator for CameraIterator {
    type Item = Camera;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.scene.num_cameras() {
            let idx = self.index;
            self.index += 1;
            if let Some(camera) = self.scene.camera(idx) {
                return Some(camera);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scene.num_cameras().saturating_sub(self.index);
        (0, Some(remaining))
    }
}

/// Iterator over lights in a scene
pub struct LightIterator {
    scene: Scene,
    index: usize,
}

impl Iterator for LightIterator {
    type Item = Light;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.scene.num_lights() {
            let idx = self.index;
            self.index += 1;
            if let Some(light) = self.scene.light(idx) {
                return Some(light);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scene.num_lights().saturating_sub(self.index);
        (0, Some(remaining))
    }
}

impl Scene {
    /// Get scene metadata
    pub fn metadata(&self) -> Result<Metadata> {
        unsafe { Metadata::from_raw_sys(self.raw().mMetaData) }
    }

    /// Get the number of textures in the scene
    pub fn num_textures(&self) -> usize {
        let scene = self.raw();
        if scene.mTextures.is_null() {
            0
        } else {
            scene.mNumTextures as usize
        }
    }

    /// Get a texture by index
    pub fn texture(&self, index: usize) -> Option<Texture> {
        if index >= self.num_textures() {
            return None;
        }

        let scene = self.raw();
        let texture_ptr = unsafe {
            ffi::ptr_array_get(self, scene.mTextures, scene.mNumTextures as usize, index)
        }?;
        unsafe { Texture::from_sys_ptr(self.clone(), texture_ptr as *const sys::aiTexture) }.ok()
    }

    /// Get an iterator over all textures in the scene
    pub fn textures(&self) -> TextureIterator {
        let scene = self.raw();
        unsafe { TextureIterator::new(self.clone(), scene.mTextures, self.num_textures()) }
    }

    /// Check if the scene has embedded textures
    pub fn has_textures(&self) -> bool {
        self.num_textures() > 0
    }

    /// Find a texture by filename
    pub fn find_texture_by_filename(&self, filename: &str) -> Option<Texture> {
        self.textures()
            .find(|texture| texture.filename_str().is_some_and(|name| name == filename))
    }

    /// Iterate over compressed textures.
    pub fn compressed_textures_iter(&self) -> impl Iterator<Item = Texture> + '_ {
        self.textures().filter(|t| t.is_compressed())
    }

    /// Get all compressed textures
    pub fn compressed_textures(&self) -> Vec<Texture> {
        self.compressed_textures_iter().collect()
    }

    /// Iterate over uncompressed textures.
    pub fn uncompressed_textures_iter(&self) -> impl Iterator<Item = Texture> + '_ {
        self.textures().filter(|t| t.is_uncompressed())
    }

    /// Get all uncompressed textures
    pub fn uncompressed_textures(&self) -> Vec<Texture> {
        self.uncompressed_textures_iter().collect()
    }

    /// Get embedded texture by filename hint (e.g. "*0", "*1")
    pub fn embedded_texture_by_name(&self, name: &str) -> Result<Option<Texture>> {
        let c = std::ffi::CString::new(name).map_err(|_| {
            Error::invalid_parameter("embedded texture name contains NUL byte".to_string())
        })?;
        Ok(self.embedded_texture_by_cstr(c.as_c_str()))
    }

    /// Get embedded texture by filename hint (e.g. "*0", "*1") without allocating.
    pub fn embedded_texture_by_cstr(&self, name: &std::ffi::CStr) -> Option<Texture> {
        unsafe {
            let tex = sys::aiGetEmbeddedTexture(self.as_raw_sys(), name.as_ptr());
            if tex.is_null() {
                None
            } else {
                Texture::from_sys_ptr(self.clone(), tex).ok()
            }
        }
    }
}
