//! Mesh representation and utilities

#![allow(clippy::unnecessary_cast)]

use crate::{
    aabb::AABB,
    bone::{Bone, BoneIterator},
    ffi,
    ptr::SharedPtr,
    raw,
    scene::Scene,
    sys,
    types::{Color4D, Vector3D, ai_string_to_str, ai_string_to_string},
};

/// A mesh containing vertices, faces, and other geometric data
#[derive(Clone)]
pub struct Mesh {
    scene: Scene,
    mesh_ptr: SharedPtr<sys::aiMesh>,
}

impl Mesh {
    /// Create a Mesh from a raw Assimp mesh pointer
    ///
    /// # Safety
    /// Caller must ensure `mesh_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(scene: Scene, mesh_ptr: *const sys::aiMesh) -> Self {
        debug_assert!(!mesh_ptr.is_null());
        let mesh_ptr = unsafe { SharedPtr::new_unchecked(mesh_ptr) };
        Self { scene, mesh_ptr }
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw_sys(&self) -> *const sys::aiMesh {
        self.mesh_ptr.as_ptr()
    }

    /// Get the raw mesh pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> *const sys::aiMesh {
        self.as_raw_sys()
    }

    /// Get the name of the mesh
    pub fn name(&self) -> String {
        unsafe { ai_string_to_string(&(*self.mesh_ptr.as_ptr()).mName) }
    }

    /// Get the name of the mesh (zero-copy, lossy UTF-8).
    pub fn name_str(&self) -> std::borrow::Cow<'_, str> {
        unsafe { ai_string_to_str(&(*self.mesh_ptr.as_ptr()).mName) }
    }

    /// Get the number of vertices in the mesh
    pub fn num_vertices(&self) -> usize {
        unsafe { (*self.mesh_ptr.as_ptr()).mNumVertices as usize }
    }

    /// Returns `true` if this mesh has a position buffer.
    pub fn has_vertices(&self) -> bool {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            mesh.mNumVertices > 0 && !mesh.mVertices.is_null()
        }
    }

    /// Returns `true` if this mesh has normals.
    pub fn has_normals(&self) -> bool {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            mesh.mNumVertices > 0 && !mesh.mNormals.is_null()
        }
    }

    /// Returns `true` if this mesh has tangents.
    pub fn has_tangents(&self) -> bool {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            mesh.mNumVertices > 0 && !mesh.mTangents.is_null()
        }
    }

    /// Returns `true` if this mesh has bitangents.
    pub fn has_bitangents(&self) -> bool {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            mesh.mNumVertices > 0 && !mesh.mBitangents.is_null()
        }
    }

    /// Returns `true` if this mesh has texture coordinates for `channel`.
    pub fn has_texture_coords(&self, channel: usize) -> bool {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return false;
        }
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            mesh.mNumVertices > 0 && !mesh.mTextureCoords[channel].is_null()
        }
    }

    /// Returns `true` if this mesh has vertex colors for `channel`.
    pub fn has_vertex_colors(&self, channel: usize) -> bool {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return false;
        }
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            mesh.mNumVertices > 0 && !mesh.mColors[channel].is_null()
        }
    }

    /// Get the vertices of the mesh
    pub fn vertices(&self) -> Vec<Vector3D> {
        self.vertices_iter().collect()
    }

    /// Get the raw vertex buffer (zero-copy).
    pub fn vertices_raw(&self) -> &[raw::AiVector3D] {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let n = mesh.mNumVertices as usize;
            debug_assert!(n == 0 || !mesh.mVertices.is_null());
            ffi::slice_from_ptr_len(self, mesh.mVertices as *const raw::AiVector3D, n)
        }
    }

    /// Get the raw vertex buffer (zero-copy), returning `None` when absent.
    pub fn vertices_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let n = mesh.mNumVertices as usize;
            let ptr = mesh.mVertices as *const raw::AiVector3D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(self, ptr, n))
            }
        }
    }

    /// Iterate vertices without allocation.
    pub fn vertices_iter(&self) -> impl Iterator<Item = Vector3D> + '_ {
        self.vertices_raw()
            .iter()
            .map(|v| Vector3D::new(v.x, v.y, v.z))
    }

    /// Get the normals of the mesh
    pub fn normals(&self) -> Option<Vec<Vector3D>> {
        self.normals_raw_opt()
            .map(|ns| ns.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Get the raw normal buffer (zero-copy).
    pub fn normals_raw(&self) -> &[raw::AiVector3D] {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            ffi::slice_from_ptr_len(
                self,
                mesh.mNormals as *const raw::AiVector3D,
                mesh.mNumVertices as usize,
            )
        }
    }

    /// Get the raw normal buffer (zero-copy), returning `None` when absent.
    pub fn normals_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let ptr = mesh.mNormals as *const raw::AiVector3D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(
                    self,
                    ptr,
                    mesh.mNumVertices as usize,
                ))
            }
        }
    }

    /// Iterate normals without allocation.
    pub fn normals_iter(&self) -> impl Iterator<Item = Vector3D> + '_ {
        self.normals_raw()
            .iter()
            .map(|v| Vector3D::new(v.x, v.y, v.z))
    }

    /// Get the tangents of the mesh
    pub fn tangents(&self) -> Option<Vec<Vector3D>> {
        self.tangents_raw_opt()
            .map(|ts| ts.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Get the raw tangent buffer (zero-copy).
    pub fn tangents_raw(&self) -> &[raw::AiVector3D] {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            ffi::slice_from_ptr_len(
                self,
                mesh.mTangents as *const raw::AiVector3D,
                mesh.mNumVertices as usize,
            )
        }
    }

    /// Get the raw tangent buffer (zero-copy), returning `None` when absent.
    pub fn tangents_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let ptr = mesh.mTangents as *const raw::AiVector3D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(
                    self,
                    ptr,
                    mesh.mNumVertices as usize,
                ))
            }
        }
    }

    /// Iterate tangents without allocation.
    pub fn tangents_iter(&self) -> impl Iterator<Item = Vector3D> + '_ {
        self.tangents_raw()
            .iter()
            .map(|v| Vector3D::new(v.x, v.y, v.z))
    }

    /// Get the bitangents of the mesh
    pub fn bitangents(&self) -> Option<Vec<Vector3D>> {
        self.bitangents_raw_opt()
            .map(|bs| bs.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Get the raw bitangent buffer (zero-copy).
    pub fn bitangents_raw(&self) -> &[raw::AiVector3D] {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            ffi::slice_from_ptr_len(
                self,
                mesh.mBitangents as *const raw::AiVector3D,
                mesh.mNumVertices as usize,
            )
        }
    }

    /// Get the raw bitangent buffer (zero-copy), returning `None` when absent.
    pub fn bitangents_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let ptr = mesh.mBitangents as *const raw::AiVector3D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(
                    self,
                    ptr,
                    mesh.mNumVertices as usize,
                ))
            }
        }
    }

    /// Iterate bitangents without allocation.
    pub fn bitangents_iter(&self) -> impl Iterator<Item = Vector3D> + '_ {
        self.bitangents_raw()
            .iter()
            .map(|v| Vector3D::new(v.x, v.y, v.z))
    }

    /// Get texture coordinates for a specific channel
    pub fn texture_coords(&self, channel: usize) -> Option<Vec<Vector3D>> {
        self.texture_coords_raw_opt(channel)
            .map(|uvs| uvs.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Get raw texture coordinates for a specific channel (zero-copy).
    pub fn texture_coords_raw(&self, channel: usize) -> &[raw::AiVector3D] {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return &[];
        }

        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let tex_coords_ptr = mesh.mTextureCoords[channel];
            ffi::slice_from_ptr_len(
                self,
                tex_coords_ptr as *const raw::AiVector3D,
                mesh.mNumVertices as usize,
            )
        }
    }

    /// Get raw texture coordinates for a specific channel (zero-copy), returning `None` when absent.
    pub fn texture_coords_raw_opt(&self, channel: usize) -> Option<&[raw::AiVector3D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return None;
        }
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let ptr = mesh.mTextureCoords[channel] as *const raw::AiVector3D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(
                    self,
                    ptr,
                    mesh.mNumVertices as usize,
                ))
            }
        }
    }

    /// Iterate texture coordinates without allocation.
    pub fn texture_coords_iter(&self, channel: usize) -> impl Iterator<Item = Vector3D> + '_ {
        self.texture_coords_raw(channel)
            .iter()
            .map(|v| Vector3D::new(v.x, v.y, v.z))
    }

    /// Get vertex colors for a specific channel
    pub fn vertex_colors(&self, channel: usize) -> Option<Vec<Color4D>> {
        self.vertex_colors_raw_opt(channel).map(|cs| {
            cs.iter()
                .map(|c| Color4D::new(c.r, c.g, c.b, c.a))
                .collect()
        })
    }

    /// Get raw vertex colors for a specific channel (zero-copy).
    pub fn vertex_colors_raw(&self, channel: usize) -> &[raw::AiColor4D] {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return &[];
        }

        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let colors_ptr = mesh.mColors[channel];
            ffi::slice_from_ptr_len(
                self,
                colors_ptr as *const raw::AiColor4D,
                mesh.mNumVertices as usize,
            )
        }
    }

    /// Get raw vertex colors for a specific channel (zero-copy), returning `None` when absent.
    pub fn vertex_colors_raw_opt(&self, channel: usize) -> Option<&[raw::AiColor4D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return None;
        }
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let ptr = mesh.mColors[channel] as *const raw::AiColor4D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(
                    self,
                    ptr,
                    mesh.mNumVertices as usize,
                ))
            }
        }
    }

    /// Iterate vertex colors without allocation.
    pub fn vertex_colors_iter(&self, channel: usize) -> impl Iterator<Item = Color4D> + '_ {
        self.vertex_colors_raw(channel)
            .iter()
            .map(|c| Color4D::new(c.r, c.g, c.b, c.a))
    }

    /// Get the number of faces in the mesh
    pub fn num_faces(&self) -> usize {
        unsafe { (*self.mesh_ptr.as_ptr()).mNumFaces as usize }
    }

    /// Get the faces of the mesh
    pub fn faces(&self) -> FaceIterator {
        FaceIterator {
            scene: self.scene.clone(),
            mesh_ptr: self.mesh_ptr,
            index: 0,
        }
    }

    /// Get the raw face array (zero-copy).
    pub fn faces_raw(&self) -> &[raw::AiFace] {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let n = mesh.mNumFaces as usize;
            debug_assert!(n == 0 || !mesh.mFaces.is_null());
            ffi::slice_from_ptr_len(self, mesh.mFaces as *const raw::AiFace, n)
        }
    }

    /// Get the raw face array (zero-copy), returning `None` when absent.
    pub fn faces_raw_opt(&self) -> Option<&[raw::AiFace]> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let n = mesh.mNumFaces as usize;
            let ptr = mesh.mFaces as *const raw::AiFace;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(self, ptr, n))
            }
        }
    }

    /// Iterate faces without allocation.
    pub fn faces_iter(&self) -> impl Iterator<Item = Face> + '_ {
        self.faces()
    }

    /// Get the material index for this mesh
    pub fn material_index(&self) -> usize {
        unsafe { (*self.mesh_ptr.as_ptr()).mMaterialIndex as usize }
    }

    /// Get the primitive types present in this mesh
    pub fn primitive_types(&self) -> u32 {
        unsafe { (*self.mesh_ptr.as_ptr()).mPrimitiveTypes }
    }

    /// Check if the mesh contains points
    pub fn has_points(&self) -> bool {
        self.primitive_types() & (sys::aiPrimitiveType::aiPrimitiveType_POINT as u32) != 0
    }

    /// Check if the mesh contains lines
    pub fn has_lines(&self) -> bool {
        self.primitive_types() & (sys::aiPrimitiveType::aiPrimitiveType_LINE as u32) != 0
    }

    /// Check if the mesh contains triangles
    pub fn has_triangles(&self) -> bool {
        self.primitive_types() & (sys::aiPrimitiveType::aiPrimitiveType_TRIANGLE as u32) != 0
    }

    /// Check if the mesh contains polygons
    pub fn has_polygons(&self) -> bool {
        self.primitive_types() & (sys::aiPrimitiveType::aiPrimitiveType_POLYGON as u32) != 0
    }

    /// Get the axis-aligned bounding box of the mesh
    pub fn aabb(&self) -> AABB {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            crate::aabb::from_sys_aabb(&mesh.mAABB)
        }
    }

    /// Get the number of animation meshes (morph targets)
    pub fn num_anim_meshes(&self) -> usize {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            if mesh.mAnimMeshes.is_null() {
                0
            } else {
                mesh.mNumAnimMeshes as usize
            }
        }
    }

    /// Get an animation mesh by index
    pub fn anim_mesh(&self, index: usize) -> Option<AnimMesh> {
        if index >= self.num_anim_meshes() {
            return None;
        }
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            if mesh.mAnimMeshes.is_null() {
                return None;
            }
            let ptr = *mesh.mAnimMeshes.add(index);
            if ptr.is_null() {
                None
            } else {
                let anim_ptr = SharedPtr::new(ptr)?;
                Some(AnimMesh {
                    scene: self.scene.clone(),
                    anim_ptr,
                })
            }
        }
    }

    /// Iterate over animation meshes
    pub fn anim_meshes(&self) -> AnimMeshIterator {
        AnimMeshIterator {
            scene: self.scene.clone(),
            mesh_ptr: self.mesh_ptr,
            index: 0,
        }
    }

    /// Get the number of bones in the mesh
    pub fn num_bones(&self) -> usize {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            if mesh.mBones.is_null() {
                0
            } else {
                mesh.mNumBones as usize
            }
        }
    }

    /// Get a bone by index
    pub fn bone(&self, index: usize) -> Option<Bone> {
        if index >= self.num_bones() {
            return None;
        }

        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            if mesh.mBones.is_null() {
                return None;
            }
            let bone_ptr = *mesh.mBones.add(index);
            if bone_ptr.is_null() {
                None
            } else {
                Bone::from_raw(self.scene.clone(), bone_ptr).ok()
            }
        }
    }

    /// Get an iterator over all bones in the mesh
    pub fn bones(&self) -> BoneIterator {
        unsafe {
            BoneIterator::new(
                self.scene.clone(),
                (*self.mesh_ptr.as_ptr()).mBones,
                self.num_bones(),
            )
        }
    }

    /// Check if the mesh has bones (is rigged for skeletal animation)
    pub fn has_bones(&self) -> bool {
        self.num_bones() > 0
    }

    /// Find a bone by name
    pub fn find_bone_by_name(&self, name: &str) -> Option<Bone> {
        self.bones().find(|bone| bone.name_str().as_ref() == name)
    }

    /// Get all bone names
    pub fn bone_names(&self) -> Vec<String> {
        self.bones().map(|bone| bone.name()).collect()
    }

    /// Get the mesh morphing method (if any)
    pub fn morphing_method(&self) -> MorphingMethod {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            MorphingMethod::from_sys(mesh.mMethod)
        }
    }
}

/// A face in a mesh
#[derive(Clone)]
pub struct Face {
    #[allow(dead_code)]
    scene: Scene,
    face_ptr: SharedPtr<raw::AiFace>,
}

impl Face {
    /// Get the number of indices in this face
    pub fn num_indices(&self) -> usize {
        unsafe { (*self.face_ptr.as_ptr()).mNumIndices as usize }
    }

    /// Get the raw index slice (zero-copy).
    pub fn indices_raw(&self) -> &[u32] {
        unsafe {
            let face = &*self.face_ptr.as_ptr();
            debug_assert!(face.mNumIndices == 0 || !face.mIndices.is_null());
            ffi::slice_from_ptr_len(self, face.mIndices as *const u32, face.mNumIndices as usize)
        }
    }

    /// Get the raw index slice (zero-copy), returning `None` when absent.
    pub fn indices_raw_opt(&self) -> Option<&[u32]> {
        unsafe {
            let face = &*self.face_ptr.as_ptr();
            ffi::slice_from_ptr_len_opt(
                self,
                face.mIndices as *const u32,
                face.mNumIndices as usize,
            )
        }
    }

    /// Get the indices of this face.
    pub fn indices(&self) -> &[u32] {
        self.indices_raw()
    }
}

/// Iterator over faces in a mesh
pub struct FaceIterator {
    scene: Scene,
    mesh_ptr: SharedPtr<sys::aiMesh>,
    index: usize,
}

impl Iterator for FaceIterator {
    type Item = Face;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            if mesh.mFaces.is_null() || mesh.mNumFaces == 0 {
                return None;
            }
            if self.index >= mesh.mNumFaces as usize {
                None
            } else {
                let face_ptr = mesh.mFaces.add(self.index);
                self.index += 1;
                let face_ptr = SharedPtr::new(face_ptr as *const raw::AiFace)?;
                Some(Face {
                    scene: self.scene.clone(),
                    face_ptr,
                })
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            if mesh.mFaces.is_null() {
                (0, Some(0))
            } else {
                let remaining = (mesh.mNumFaces as usize).saturating_sub(self.index);
                (remaining, Some(remaining))
            }
        }
    }
}

impl ExactSizeIterator for FaceIterator {}

/// An animation mesh (morph target) that replaces certain vertex streams
#[derive(Clone)]
pub struct AnimMesh {
    #[allow(dead_code)]
    scene: Scene,
    anim_ptr: SharedPtr<sys::aiAnimMesh>,
}

impl AnimMesh {
    /// Name of this anim mesh (if present)
    pub fn name(&self) -> String {
        unsafe { crate::types::ai_string_to_string(&(*self.anim_ptr.as_ptr()).mName) }
    }
    /// Number of vertices in this anim mesh
    pub fn num_vertices(&self) -> usize {
        unsafe { (*self.anim_ptr.as_ptr()).mNumVertices as usize }
    }

    /// Returns `true` if this anim mesh has replacement positions.
    pub fn has_vertices(&self) -> bool {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            m.mNumVertices > 0 && !m.mVertices.is_null()
        }
    }

    /// Returns `true` if this anim mesh has replacement normals.
    pub fn has_normals(&self) -> bool {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            m.mNumVertices > 0 && !m.mNormals.is_null()
        }
    }

    /// Returns `true` if this anim mesh has replacement tangents.
    pub fn has_tangents(&self) -> bool {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            m.mNumVertices > 0 && !m.mTangents.is_null()
        }
    }

    /// Returns `true` if this anim mesh has replacement bitangents.
    pub fn has_bitangents(&self) -> bool {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            m.mNumVertices > 0 && !m.mBitangents.is_null()
        }
    }

    /// Returns `true` if this anim mesh has replacement texture coordinates for `channel`.
    pub fn has_texture_coords(&self, channel: usize) -> bool {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return false;
        }
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            m.mNumVertices > 0 && !m.mTextureCoords[channel].is_null()
        }
    }

    /// Returns `true` if this anim mesh has replacement vertex colors for `channel`.
    pub fn has_vertex_colors(&self, channel: usize) -> bool {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return false;
        }
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            m.mNumVertices > 0 && !m.mColors[channel].is_null()
        }
    }

    /// Replacement positions (if present)
    pub fn vertices(&self) -> Option<Vec<Vector3D>> {
        self.vertices_raw_opt()
            .map(|vs| vs.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Raw replacement positions (zero-copy).
    pub fn vertices_raw(&self) -> &[raw::AiVector3D] {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            ffi::slice_from_ptr_len(
                self,
                m.mVertices as *const raw::AiVector3D,
                m.mNumVertices as usize,
            )
        }
    }

    /// Raw replacement positions (zero-copy), returning `None` when absent.
    pub fn vertices_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            let ptr = m.mVertices as *const raw::AiVector3D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize))
            }
        }
    }

    /// Replacement normals (if present)
    pub fn normals(&self) -> Option<Vec<Vector3D>> {
        self.normals_raw_opt()
            .map(|ns| ns.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Raw replacement normals (zero-copy).
    pub fn normals_raw(&self) -> &[raw::AiVector3D] {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            ffi::slice_from_ptr_len(
                self,
                m.mNormals as *const raw::AiVector3D,
                m.mNumVertices as usize,
            )
        }
    }

    /// Raw replacement normals (zero-copy), returning `None` when absent.
    pub fn normals_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            let ptr = m.mNormals as *const raw::AiVector3D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize))
            }
        }
    }

    /// Replacement tangents (if present)
    pub fn tangents(&self) -> Option<Vec<Vector3D>> {
        self.tangents_raw_opt()
            .map(|ts| ts.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Raw replacement tangents (zero-copy).
    pub fn tangents_raw(&self) -> &[raw::AiVector3D] {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            ffi::slice_from_ptr_len(
                self,
                m.mTangents as *const raw::AiVector3D,
                m.mNumVertices as usize,
            )
        }
    }

    /// Raw replacement tangents (zero-copy), returning `None` when absent.
    pub fn tangents_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            let ptr = m.mTangents as *const raw::AiVector3D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize))
            }
        }
    }

    /// Replacement bitangents (if present)
    pub fn bitangents(&self) -> Option<Vec<Vector3D>> {
        self.bitangents_raw_opt()
            .map(|bs| bs.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Raw replacement bitangents (zero-copy).
    pub fn bitangents_raw(&self) -> &[raw::AiVector3D] {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            ffi::slice_from_ptr_len(
                self,
                m.mBitangents as *const raw::AiVector3D,
                m.mNumVertices as usize,
            )
        }
    }

    /// Raw replacement bitangents (zero-copy), returning `None` when absent.
    pub fn bitangents_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            let ptr = m.mBitangents as *const raw::AiVector3D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize))
            }
        }
    }

    /// Replacement vertex colors for a specific channel
    pub fn vertex_colors(&self, channel: usize) -> Option<Vec<Color4D>> {
        self.vertex_colors_raw_opt(channel).map(|cs| {
            cs.iter()
                .map(|c| Color4D::new(c.r, c.g, c.b, c.a))
                .collect()
        })
    }

    /// Raw replacement vertex colors for a specific channel (zero-copy).
    pub fn vertex_colors_raw(&self, channel: usize) -> &[raw::AiColor4D] {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return &[];
        }
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            let ptr = m.mColors[channel];
            ffi::slice_from_ptr_len(self, ptr as *const raw::AiColor4D, m.mNumVertices as usize)
        }
    }

    /// Raw replacement vertex colors for a specific channel (zero-copy), returning `None` when absent.
    pub fn vertex_colors_raw_opt(&self, channel: usize) -> Option<&[raw::AiColor4D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return None;
        }
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            let ptr = m.mColors[channel] as *const raw::AiColor4D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize))
            }
        }
    }

    /// Replacement texture coordinates for a specific channel
    pub fn texture_coords(&self, channel: usize) -> Option<Vec<Vector3D>> {
        self.texture_coords_raw_opt(channel)
            .map(|uvs| uvs.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Raw replacement texture coordinates for a specific channel (zero-copy).
    pub fn texture_coords_raw(&self, channel: usize) -> &[raw::AiVector3D] {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return &[];
        }
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            let ptr = m.mTextureCoords[channel];
            ffi::slice_from_ptr_len(self, ptr as *const raw::AiVector3D, m.mNumVertices as usize)
        }
    }

    /// Raw replacement texture coordinates for a specific channel (zero-copy), returning `None` when absent.
    pub fn texture_coords_raw_opt(&self, channel: usize) -> Option<&[raw::AiVector3D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return None;
        }
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            let ptr = m.mTextureCoords[channel] as *const raw::AiVector3D;
            if ptr.is_null() {
                None
            } else {
                Some(ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize))
            }
        }
    }

    /// Weight of this anim mesh
    pub fn weight(&self) -> f32 {
        unsafe { (*self.anim_ptr.as_ptr()).mWeight }
    }
}

/// Iterator over anim meshes
pub struct AnimMeshIterator {
    scene: Scene,
    mesh_ptr: SharedPtr<sys::aiMesh>,
    index: usize,
}

impl Iterator for AnimMeshIterator {
    type Item = AnimMesh;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            if mesh.mAnimMeshes.is_null() || mesh.mNumAnimMeshes == 0 {
                return None;
            }
            while self.index < mesh.mNumAnimMeshes as usize {
                let ptr = *mesh.mAnimMeshes.add(self.index);
                self.index += 1;
                if ptr.is_null() {
                    continue;
                }
                let anim_ptr = SharedPtr::new(ptr)?;
                return Some(AnimMesh {
                    scene: self.scene.clone(),
                    anim_ptr,
                });
            }
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            if mesh.mAnimMeshes.is_null() {
                (0, Some(0))
            } else {
                let remaining = (mesh.mNumAnimMeshes as usize).saturating_sub(self.index);
                (0, Some(remaining))
            }
        }
    }
}

/// Methods of mesh morphing supported by Assimp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MorphingMethod {
    /// Unknown morphing method
    Unknown,
    /// Vertex blending morphing
    VertexBlend,
    /// Normalized morph targets (weights sum to 1.0)
    MorphNormalized,
    /// Relative morph targets (additive)
    MorphRelative,
}

impl MorphingMethod {
    fn from_sys(v: sys::aiMorphingMethod) -> Self {
        match v {
            sys::aiMorphingMethod::aiMorphingMethod_UNKNOWN => MorphingMethod::Unknown,
            sys::aiMorphingMethod::aiMorphingMethod_VERTEX_BLEND => MorphingMethod::VertexBlend,
            sys::aiMorphingMethod::aiMorphingMethod_MORPH_NORMALIZED => {
                MorphingMethod::MorphNormalized
            }
            sys::aiMorphingMethod::aiMorphingMethod_MORPH_RELATIVE => MorphingMethod::MorphRelative,
            _ => MorphingMethod::Unknown,
        }
    }
}

// Auto-traits (Send/Sync) are derived from the contained pointers and lifetimes.
