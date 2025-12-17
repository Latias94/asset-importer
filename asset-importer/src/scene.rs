//! Scene representation and management

use crate::{
    animation::Animation,
    camera::Camera,
    error::{Error, Result},
    importer::{Importer, PropertyStore},
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
/// `Send + Sync` and can be used with `Arc` across threads for *read-only* access.
///
/// This guarantee relies on the safe API treating the imported Assimp scene as immutable.
/// If you call into raw Assimp bindings (`asset_importer::sys` with feature `raw-sys`, or the
/// `asset-importer-sys` crate) and mutate internal pointers yourself, you can
/// violate this contract and cause undefined behavior.
pub struct Scene {
    /// Raw pointer to the Assimp scene
    scene_ptr: SharedPtr<sys::aiScene>,
    /// How to release the scene when dropped
    release_kind: SceneRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneRelease {
    /// Scene returned by aiImportFile* family, free with aiReleaseImport
    ReleaseImport,
    /// Scene created via aiCopyScene, free with aiFreeScene
    FreeScene,
}

impl Scene {
    /// Create a Scene from a raw Assimp scene pointer
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `scene_ptr` is a valid pointer to an aiScene
    /// - The scene was allocated by Assimp and should be freed with aiReleaseImport
    /// - The scene pointer remains valid for the lifetime of this Scene
    pub(crate) unsafe fn from_raw_import_sys(scene_ptr: *const sys::aiScene) -> Result<Self> {
        let scene_ptr = SharedPtr::new(scene_ptr).ok_or(Error::NullPointer)?;

        Ok(Self {
            scene_ptr,
            release_kind: SceneRelease::ReleaseImport,
        })
    }

    /// Create a Scene from a raw Assimp scene pointer (requires `raw-sys`).
    ///
    /// # Safety
    /// Same contract as `from_raw_import_sys`.
    #[cfg(feature = "raw-sys")]
    pub unsafe fn from_raw_import(scene_ptr: *const sys::aiScene) -> Result<Self> {
        unsafe { Self::from_raw_import_sys(scene_ptr) }
    }

    /// Create a Scene from a deep-copied Assimp scene pointer (aiCopyScene)
    /// The scene will be freed with aiFreeScene.
    ///
    /// # Safety
    /// Caller must ensure `scene_ptr` is valid and was allocated by aiCopyScene.
    pub(crate) unsafe fn from_raw_copied_sys(scene_ptr: *const sys::aiScene) -> Result<Self> {
        let scene_ptr = SharedPtr::new(scene_ptr).ok_or(Error::NullPointer)?;
        Ok(Self {
            scene_ptr,
            release_kind: SceneRelease::FreeScene,
        })
    }

    /// Create a Scene from a deep-copied Assimp scene pointer (requires `raw-sys`).
    ///
    /// # Safety
    /// Same contract as `from_raw_copied_sys`.
    #[cfg(feature = "raw-sys")]
    pub unsafe fn from_raw_copied(scene_ptr: *const sys::aiScene) -> Result<Self> {
        unsafe { Self::from_raw_copied_sys(scene_ptr) }
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw_sys(&self) -> *const sys::aiScene {
        self.scene_ptr.as_ptr()
    }

    /// Get the raw scene pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> *const sys::aiScene {
        self.as_raw_sys()
    }

    /// Apply Assimp post-processing to this scene.
    ///
    /// This consumes the scene and returns the updated scene on success:
    /// `scene = scene.apply_postprocess(flags)?;`.
    ///
    /// Assimp documents that post-processing is in-place but may return `NULL` on failure
    /// (notably for `aiProcess_ValidateDataStructure`), potentially invalidating the input
    /// scene pointer. To avoid double-free or use-after-free in safe Rust, this API takes
    /// ownership of the scene and will not drop the original pointer on failure.
    pub fn apply_postprocess(self, flags: crate::postprocess::PostProcessSteps) -> Result<Self> {
        let this = std::mem::ManuallyDrop::new(self);

        let new_ptr = unsafe { sys::aiApplyPostProcessing(this.scene_ptr.as_ptr(), flags.as_raw()) };
        if new_ptr.is_null() {
            return Err(Error::invalid_scene("Post-processing failed"));
        }

        // Assimp promises this is the same scene pointer on success, but treat it as an update anyway.
        let mut scene = std::mem::ManuallyDrop::into_inner(this);
        scene.scene_ptr = SharedPtr::new(new_ptr).ok_or(Error::NullPointer)?;
        Ok(scene)
    }

    /// Load a scene from a file with default settings
    ///
    /// This is a convenience method that provides a russimp-compatible interface.
    /// For more control, use `Importer::new().read_file(path).import_file(path)`.
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
            .import_file(path)
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
            .import_file(path)
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
            .with_post_process(post_process)
            .import_from_memory(data, hint)
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
            .with_post_process(post_process)
            .with_property_store_ref(props)
            .import_from_memory(data, hint)
    }

    /// Get the scene flags
    pub fn flags(&self) -> u32 {
        unsafe { (*self.scene_ptr.as_ptr()).mFlags }
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

        unsafe {
            sys::aiGetMemoryRequirements(self.scene_ptr.as_ptr(), &mut info);
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
    pub fn root_node(&self) -> Option<Node<'_>> {
        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mRootNode.is_null() {
                None
            } else {
                Some(Node::from_raw(scene.mRootNode))
            }
        }
    }

    /// Get the number of meshes in the scene
    pub fn num_meshes(&self) -> usize {
        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mMeshes.is_null() {
                0
            } else {
                scene.mNumMeshes as usize
            }
        }
    }

    /// Get a mesh by index
    pub fn mesh(&self, index: usize) -> Option<Mesh<'_>> {
        if index >= self.num_meshes() {
            return None;
        }

        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mMeshes.is_null() {
                return None;
            }
            let mesh_ptr = *scene.mMeshes.add(index);
            if mesh_ptr.is_null() {
                None
            } else {
                Some(Mesh::from_raw(mesh_ptr))
            }
        }
    }

    /// Get an iterator over all meshes
    pub fn meshes(&self) -> MeshIterator<'_> {
        MeshIterator {
            scene: self,
            index: 0,
        }
    }

    /// Get the number of materials in the scene
    pub fn num_materials(&self) -> usize {
        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mMaterials.is_null() {
                0
            } else {
                scene.mNumMaterials as usize
            }
        }
    }

    /// Get a material by index
    pub fn material(&self, index: usize) -> Option<Material<'_>> {
        if index >= self.num_materials() {
            return None;
        }

        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mMaterials.is_null() {
                return None;
            }
            let material_ptr = *scene.mMaterials.add(index);
            if material_ptr.is_null() {
                None
            } else {
                Some(Material::from_raw(material_ptr))
            }
        }
    }

    /// Get an iterator over all materials
    pub fn materials(&self) -> MaterialIterator<'_> {
        MaterialIterator {
            scene: self,
            index: 0,
        }
    }

    /// Get the number of animations in the scene
    pub fn num_animations(&self) -> usize {
        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mAnimations.is_null() {
                0
            } else {
                scene.mNumAnimations as usize
            }
        }
    }

    /// Get an animation by index
    pub fn animation(&self, index: usize) -> Option<Animation<'_>> {
        if index >= self.num_animations() {
            return None;
        }

        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mAnimations.is_null() {
                return None;
            }
            let animation_ptr = *scene.mAnimations.add(index);
            if animation_ptr.is_null() {
                None
            } else {
                Some(Animation::from_raw(animation_ptr))
            }
        }
    }

    /// Get an iterator over all animations
    pub fn animations(&self) -> AnimationIterator<'_> {
        AnimationIterator {
            scene: self,
            index: 0,
        }
    }

    /// Get the number of cameras in the scene
    pub fn num_cameras(&self) -> usize {
        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mCameras.is_null() {
                0
            } else {
                scene.mNumCameras as usize
            }
        }
    }

    /// Get a camera by index
    pub fn camera(&self, index: usize) -> Option<Camera<'_>> {
        if index >= self.num_cameras() {
            return None;
        }

        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mCameras.is_null() {
                return None;
            }
            let camera_ptr = *scene.mCameras.add(index);
            if camera_ptr.is_null() {
                None
            } else {
                Some(Camera::from_raw(camera_ptr))
            }
        }
    }

    /// Get an iterator over all cameras
    pub fn cameras(&self) -> CameraIterator<'_> {
        CameraIterator {
            scene: self,
            index: 0,
        }
    }

    /// Get the number of lights in the scene
    pub fn num_lights(&self) -> usize {
        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mLights.is_null() {
                0
            } else {
                scene.mNumLights as usize
            }
        }
    }

    /// Get a light by index
    pub fn light(&self, index: usize) -> Option<Light<'_>> {
        if index >= self.num_lights() {
            return None;
        }

        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mLights.is_null() {
                return None;
            }
            let light_ptr = *scene.mLights.add(index);
            if light_ptr.is_null() {
                None
            } else {
                Some(Light::from_raw(light_ptr))
            }
        }
    }

    /// Get an iterator over all lights
    pub fn lights(&self) -> LightIterator<'_> {
        LightIterator {
            scene: self,
            index: 0,
        }
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        unsafe {
            match self.release_kind {
                SceneRelease::ReleaseImport => sys::release_import(self.scene_ptr.as_ptr()),
                SceneRelease::FreeScene => sys::aiFreeScene(self.scene_ptr.as_ptr()),
            }
        }
    }
}

/// Iterator over meshes in a scene
pub struct MeshIterator<'a> {
    scene: &'a Scene,
    index: usize,
}

impl<'a> Iterator for MeshIterator<'a> {
    type Item = Mesh<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mesh = self.scene.mesh(self.index)?;
        self.index += 1;
        Some(mesh)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scene.num_meshes().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for MeshIterator<'a> {}

/// Iterator over materials in a scene
pub struct MaterialIterator<'a> {
    scene: &'a Scene,
    index: usize,
}

impl<'a> Iterator for MaterialIterator<'a> {
    type Item = Material<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let material = self.scene.material(self.index)?;
        self.index += 1;
        Some(material)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scene.num_materials().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for MaterialIterator<'a> {}

/// Iterator over animations in a scene
pub struct AnimationIterator<'a> {
    scene: &'a Scene,
    index: usize,
}

impl<'a> Iterator for AnimationIterator<'a> {
    type Item = Animation<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let animation = self.scene.animation(self.index)?;
        self.index += 1;
        Some(animation)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scene.num_animations().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for AnimationIterator<'a> {}

/// Iterator over cameras in a scene
pub struct CameraIterator<'a> {
    scene: &'a Scene,
    index: usize,
}

impl<'a> Iterator for CameraIterator<'a> {
    type Item = Camera<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let camera = self.scene.camera(self.index)?;
        self.index += 1;
        Some(camera)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scene.num_cameras().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for CameraIterator<'a> {}

/// Iterator over lights in a scene
pub struct LightIterator<'a> {
    scene: &'a Scene,
    index: usize,
}

impl<'a> Iterator for LightIterator<'a> {
    type Item = Light<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let light = self.scene.light(self.index)?;
        self.index += 1;
        Some(light)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scene.num_lights().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for LightIterator<'a> {}

impl Scene {
    /// Get scene metadata
    pub fn metadata(&self) -> Result<Metadata> {
        let scene = unsafe { &*self.scene_ptr.as_ptr() };
        unsafe { Metadata::from_raw_sys(scene.mMetaData) }
    }

    /// Get the number of textures in the scene
    pub fn num_textures(&self) -> usize {
        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mTextures.is_null() {
                0
            } else {
                scene.mNumTextures as usize
            }
        }
    }

    /// Get a texture by index
    pub fn texture(&self, index: usize) -> Option<Texture<'_>> {
        if index >= self.num_textures() {
            return None;
        }

        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            if scene.mTextures.is_null() {
                return None;
            }
            let texture_ptr = *scene.mTextures.add(index);
            if texture_ptr.is_null() {
                None
            } else {
                Texture::from_raw(texture_ptr).ok()
            }
        }
    }

    /// Get an iterator over all textures in the scene
    pub fn textures(&self) -> TextureIterator<'_> {
        unsafe {
            let scene = &*self.scene_ptr.as_ptr();
            TextureIterator::new(scene.mTextures, self.num_textures())
        }
    }

    /// Check if the scene has embedded textures
    pub fn has_textures(&self) -> bool {
        self.num_textures() > 0
    }

    /// Find a texture by filename
    pub fn find_texture_by_filename(&self, filename: &str) -> Option<Texture<'_>> {
        self.textures().find(|texture| {
            texture
                .filename()
                .map(|name| name == filename)
                .unwrap_or(false)
        })
    }

    /// Get all compressed textures
    pub fn compressed_textures(&self) -> Vec<Texture<'_>> {
        self.textures()
            .filter(|texture| texture.is_compressed())
            .collect()
    }

    /// Get all uncompressed textures
    pub fn uncompressed_textures(&self) -> Vec<Texture<'_>> {
        self.textures()
            .filter(|texture| texture.is_uncompressed())
            .collect()
    }

    /// Get embedded texture by filename hint (e.g. "*0", "*1")
    pub fn embedded_texture_by_name(&self, name: &str) -> Option<Texture<'_>> {
        let c = std::ffi::CString::new(name).ok()?;
        unsafe {
            let tex = sys::aiGetEmbeddedTexture(self.scene_ptr.as_ptr(), c.as_ptr());
            if tex.is_null() {
                None
            } else {
                Texture::from_raw(tex).ok()
            }
        }
    }
}
