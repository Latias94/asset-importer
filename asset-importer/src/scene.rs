//! Scene representation and management

use std::ptr::NonNull;

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

/// A 3D scene containing meshes, materials, animations, and other assets
pub struct Scene {
    /// Raw pointer to the Assimp scene
    scene_ptr: NonNull<sys::aiScene>,
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
    pub unsafe fn from_raw(scene_ptr: *const sys::aiScene) -> Result<Self> {
        let scene_ptr = NonNull::new(scene_ptr as *mut sys::aiScene).ok_or(Error::NullPointer)?;

        Ok(Self { scene_ptr })
    }

    /// Get the raw scene pointer
    pub fn as_raw(&self) -> *const sys::aiScene {
        self.scene_ptr.as_ptr()
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
        unsafe { self.scene_ptr.as_ref().mFlags }
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
    pub fn root_node(&self) -> Option<Node> {
        unsafe {
            let scene = self.scene_ptr.as_ref();
            if scene.mRootNode.is_null() {
                None
            } else {
                Some(Node::from_raw(scene.mRootNode))
            }
        }
    }

    /// Get the number of meshes in the scene
    pub fn num_meshes(&self) -> usize {
        unsafe { self.scene_ptr.as_ref().mNumMeshes as usize }
    }

    /// Get a mesh by index
    pub fn mesh(&self, index: usize) -> Option<Mesh> {
        if index >= self.num_meshes() {
            return None;
        }

        unsafe {
            let scene = self.scene_ptr.as_ref();
            let mesh_ptr = *scene.mMeshes.add(index);
            if mesh_ptr.is_null() {
                None
            } else {
                Some(Mesh::from_raw(mesh_ptr))
            }
        }
    }

    /// Get an iterator over all meshes
    pub fn meshes(&self) -> MeshIterator {
        MeshIterator {
            scene: self,
            index: 0,
        }
    }

    /// Get the number of materials in the scene
    pub fn num_materials(&self) -> usize {
        unsafe { self.scene_ptr.as_ref().mNumMaterials as usize }
    }

    /// Get a material by index
    pub fn material(&self, index: usize) -> Option<Material> {
        if index >= self.num_materials() {
            return None;
        }

        unsafe {
            let scene = self.scene_ptr.as_ref();
            let material_ptr = *scene.mMaterials.add(index);
            if material_ptr.is_null() {
                None
            } else {
                Some(Material::from_raw(material_ptr))
            }
        }
    }

    /// Get an iterator over all materials
    pub fn materials(&self) -> MaterialIterator {
        MaterialIterator {
            scene: self,
            index: 0,
        }
    }

    /// Get the number of animations in the scene
    pub fn num_animations(&self) -> usize {
        unsafe { self.scene_ptr.as_ref().mNumAnimations as usize }
    }

    /// Get an animation by index
    pub fn animation(&self, index: usize) -> Option<Animation> {
        if index >= self.num_animations() {
            return None;
        }

        unsafe {
            let scene = self.scene_ptr.as_ref();
            let animation_ptr = *scene.mAnimations.add(index);
            if animation_ptr.is_null() {
                None
            } else {
                Some(Animation::from_raw(animation_ptr))
            }
        }
    }

    /// Get an iterator over all animations
    pub fn animations(&self) -> AnimationIterator {
        AnimationIterator {
            scene: self,
            index: 0,
        }
    }

    /// Get the number of cameras in the scene
    pub fn num_cameras(&self) -> usize {
        unsafe { self.scene_ptr.as_ref().mNumCameras as usize }
    }

    /// Get a camera by index
    pub fn camera(&self, index: usize) -> Option<Camera> {
        if index >= self.num_cameras() {
            return None;
        }

        unsafe {
            let scene = self.scene_ptr.as_ref();
            let camera_ptr = *scene.mCameras.add(index);
            if camera_ptr.is_null() {
                None
            } else {
                Some(Camera::from_raw(camera_ptr))
            }
        }
    }

    /// Get an iterator over all cameras
    pub fn cameras(&self) -> CameraIterator {
        CameraIterator {
            scene: self,
            index: 0,
        }
    }

    /// Get the number of lights in the scene
    pub fn num_lights(&self) -> usize {
        unsafe { self.scene_ptr.as_ref().mNumLights as usize }
    }

    /// Get a light by index
    pub fn light(&self, index: usize) -> Option<Light> {
        if index >= self.num_lights() {
            return None;
        }

        unsafe {
            let scene = self.scene_ptr.as_ref();
            let light_ptr = *scene.mLights.add(index);
            if light_ptr.is_null() {
                None
            } else {
                Some(Light::from_raw(light_ptr))
            }
        }
    }

    /// Get an iterator over all lights
    pub fn lights(&self) -> LightIterator {
        LightIterator {
            scene: self,
            index: 0,
        }
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        unsafe {
            sys::release_import(self.scene_ptr.as_ptr());
        }
    }
}

// Send and Sync are safe because we own the scene and Assimp doesn't use global state
unsafe impl Send for Scene {}
unsafe impl Sync for Scene {}

/// Iterator over meshes in a scene
pub struct MeshIterator<'a> {
    scene: &'a Scene,
    index: usize,
}

impl<'a> Iterator for MeshIterator<'a> {
    type Item = Mesh;

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
    type Item = Material;

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
    type Item = Animation;

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
    type Item = Camera;

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
    type Item = Light;

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
        let scene = unsafe { self.scene_ptr.as_ref() };
        unsafe { Metadata::from_raw(scene.mMetaData) }
    }

    /// Get the number of textures in the scene
    pub fn num_textures(&self) -> usize {
        unsafe { (*self.scene_ptr.as_ptr()).mNumTextures as usize }
    }

    /// Get a texture by index
    pub fn texture(&self, index: usize) -> Option<Texture> {
        if index >= self.num_textures() {
            return None;
        }

        unsafe {
            let scene = self.scene_ptr.as_ref();
            let texture_ptr = *scene.mTextures.add(index);
            if texture_ptr.is_null() {
                None
            } else {
                Texture::from_raw(texture_ptr).ok()
            }
        }
    }

    /// Get an iterator over all textures in the scene
    pub fn textures(&self) -> TextureIterator {
        unsafe {
            let scene = self.scene_ptr.as_ref();
            TextureIterator::new(scene.mTextures, self.num_textures())
        }
    }

    /// Check if the scene has embedded textures
    pub fn has_textures(&self) -> bool {
        self.num_textures() > 0
    }

    /// Find a texture by filename
    pub fn find_texture_by_filename(&self, filename: &str) -> Option<Texture> {
        self.textures().find(|texture| {
            texture
                .filename()
                .map(|name| name == filename)
                .unwrap_or(false)
        })
    }

    /// Get all compressed textures
    pub fn compressed_textures(&self) -> Vec<Texture> {
        self.textures()
            .filter(|texture| texture.is_compressed())
            .collect()
    }

    /// Get all uncompressed textures
    pub fn uncompressed_textures(&self) -> Vec<Texture> {
        self.textures()
            .filter(|texture| texture.is_uncompressed())
            .collect()
    }
}
