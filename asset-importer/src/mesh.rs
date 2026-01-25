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
    types::{Color4D, Vector2D, Vector3D, ai_string_to_str, ai_string_to_string},
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

    #[inline]
    fn raw(&self) -> &sys::aiMesh {
        unsafe { &*self.mesh_ptr.as_ptr() }
    }

    /// Get the name of the mesh
    pub fn name(&self) -> String {
        ai_string_to_string(&self.raw().mName)
    }

    /// Get the name of the mesh (zero-copy, lossy UTF-8).
    pub fn name_str(&self) -> std::borrow::Cow<'_, str> {
        ai_string_to_str(&self.raw().mName)
    }

    /// Get the number of vertices in the mesh
    pub fn num_vertices(&self) -> usize {
        self.raw().mNumVertices as usize
    }

    /// Returns `true` if this mesh has a position buffer.
    pub fn has_vertices(&self) -> bool {
        let mesh = self.raw();
        mesh.mNumVertices > 0 && !mesh.mVertices.is_null()
    }

    /// Returns `true` if this mesh has normals.
    pub fn has_normals(&self) -> bool {
        let mesh = self.raw();
        mesh.mNumVertices > 0 && !mesh.mNormals.is_null()
    }

    /// Returns `true` if this mesh has tangents.
    pub fn has_tangents(&self) -> bool {
        let mesh = self.raw();
        mesh.mNumVertices > 0 && !mesh.mTangents.is_null()
    }

    /// Returns `true` if this mesh has bitangents.
    pub fn has_bitangents(&self) -> bool {
        let mesh = self.raw();
        mesh.mNumVertices > 0 && !mesh.mBitangents.is_null()
    }

    /// Returns `true` if this mesh has texture coordinates for `channel`.
    pub fn has_texture_coords(&self, channel: usize) -> bool {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return false;
        }
        let mesh = self.raw();
        mesh.mNumVertices > 0 && !mesh.mTextureCoords[channel].is_null()
    }

    /// Returns `true` if this mesh has vertex colors for `channel`.
    pub fn has_vertex_colors(&self, channel: usize) -> bool {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return false;
        }
        let mesh = self.raw();
        mesh.mNumVertices > 0 && !mesh.mColors[channel].is_null()
    }

    /// Get the vertices of the mesh
    pub fn vertices(&self) -> Vec<Vector3D> {
        self.vertices_iter().collect()
    }

    /// Get the raw vertex buffer (zero-copy).
    pub fn vertices_raw(&self) -> &[raw::AiVector3D] {
        let mesh = self.raw();
        let n = mesh.mNumVertices as usize;
        debug_assert!(n == 0 || !mesh.mVertices.is_null());
        unsafe { ffi::slice_from_ptr_len(self, mesh.mVertices as *const raw::AiVector3D, n) }
    }

    /// Get the raw vertex buffer as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn vertices_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.vertices_raw())
    }

    /// Get the raw vertex buffer as a flat `f32` slice (x,y,z interleaved).
    #[cfg(feature = "bytemuck")]
    pub fn vertices_f32(&self) -> &[f32] {
        bytemuck::cast_slice(self.vertices_raw())
    }

    /// Get the raw vertex buffer (zero-copy), returning `None` when absent.
    pub fn vertices_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        let mesh = self.raw();
        let n = mesh.mNumVertices as usize;
        let ptr = mesh.mVertices as *const raw::AiVector3D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, n) })
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
        let mesh = self.raw();
        unsafe {
            ffi::slice_from_ptr_len(
                self,
                mesh.mNormals as *const raw::AiVector3D,
                mesh.mNumVertices as usize,
            )
        }
    }

    /// Get the raw normal buffer as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn normals_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.normals_raw())
    }

    /// Get the raw normal buffer as a flat `f32` slice (x,y,z interleaved).
    #[cfg(feature = "bytemuck")]
    pub fn normals_f32(&self) -> &[f32] {
        bytemuck::cast_slice(self.normals_raw())
    }

    /// Get the raw normal buffer (zero-copy), returning `None` when absent.
    pub fn normals_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        let mesh = self.raw();
        let ptr = mesh.mNormals as *const raw::AiVector3D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, mesh.mNumVertices as usize) })
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
        let mesh = self.raw();
        unsafe {
            ffi::slice_from_ptr_len(
                self,
                mesh.mTangents as *const raw::AiVector3D,
                mesh.mNumVertices as usize,
            )
        }
    }

    /// Get the raw tangent buffer as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn tangents_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.tangents_raw())
    }

    /// Get the raw tangent buffer as a flat `f32` slice (x,y,z interleaved).
    #[cfg(feature = "bytemuck")]
    pub fn tangents_f32(&self) -> &[f32] {
        bytemuck::cast_slice(self.tangents_raw())
    }

    /// Get the raw tangent buffer (zero-copy), returning `None` when absent.
    pub fn tangents_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        let mesh = self.raw();
        let ptr = mesh.mTangents as *const raw::AiVector3D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, mesh.mNumVertices as usize) })
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
        let mesh = self.raw();
        unsafe {
            ffi::slice_from_ptr_len(
                self,
                mesh.mBitangents as *const raw::AiVector3D,
                mesh.mNumVertices as usize,
            )
        }
    }

    /// Get the raw bitangent buffer as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn bitangents_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.bitangents_raw())
    }

    /// Get the raw bitangent buffer as a flat `f32` slice (x,y,z interleaved).
    #[cfg(feature = "bytemuck")]
    pub fn bitangents_f32(&self) -> &[f32] {
        bytemuck::cast_slice(self.bitangents_raw())
    }

    /// Get the raw bitangent buffer (zero-copy), returning `None` when absent.
    pub fn bitangents_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        let mesh = self.raw();
        let ptr = mesh.mBitangents as *const raw::AiVector3D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, mesh.mNumVertices as usize) })
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

    /// Get texture coordinates (Vec2) for a specific channel.
    ///
    /// This is a convenience for the common case where UVs are 2D; it discards the third component.
    pub fn texture_coords2(&self, channel: usize) -> Option<Vec<Vector2D>> {
        self.texture_coords_raw_opt(channel)
            .map(|uvs| uvs.iter().map(|v| Vector2D::new(v.x, v.y)).collect())
    }

    /// Get raw texture coordinates for a specific channel (zero-copy).
    pub fn texture_coords_raw(&self, channel: usize) -> &[raw::AiVector3D] {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return &[];
        }

        let mesh = self.raw();
        let tex_coords_ptr = mesh.mTextureCoords[channel];
        unsafe {
            ffi::slice_from_ptr_len(
                self,
                tex_coords_ptr as *const raw::AiVector3D,
                mesh.mNumVertices as usize,
            )
        }
    }

    /// Get raw texture coordinates for a specific channel as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn texture_coords_bytes(&self, channel: usize) -> &[u8] {
        bytemuck::cast_slice(self.texture_coords_raw(channel))
    }

    /// Get raw texture coordinates for a specific channel as a flat `f32` slice (x,y,z interleaved).
    #[cfg(feature = "bytemuck")]
    pub fn texture_coords_f32(&self, channel: usize) -> &[f32] {
        bytemuck::cast_slice(self.texture_coords_raw(channel))
    }

    /// Get raw texture coordinates for a specific channel (zero-copy), returning `None` when absent.
    pub fn texture_coords_raw_opt(&self, channel: usize) -> Option<&[raw::AiVector3D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return None;
        }
        let mesh = self.raw();
        let ptr = mesh.mTextureCoords[channel] as *const raw::AiVector3D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, mesh.mNumVertices as usize) })
        }
    }

    /// Iterate texture coordinates without allocation.
    pub fn texture_coords_iter(&self, channel: usize) -> impl Iterator<Item = Vector3D> + '_ {
        self.texture_coords_raw(channel)
            .iter()
            .map(|v| Vector3D::new(v.x, v.y, v.z))
    }

    /// Iterate texture coordinates (Vec2) without allocation.
    ///
    /// This is a convenience for the common case where UVs are 2D; it discards the third component.
    pub fn texture_coords_iter2(&self, channel: usize) -> impl Iterator<Item = Vector2D> + '_ {
        self.texture_coords_raw(channel)
            .iter()
            .map(|v| Vector2D::new(v.x, v.y))
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

        let mesh = self.raw();
        let colors_ptr = mesh.mColors[channel];
        unsafe {
            ffi::slice_from_ptr_len(
                self,
                colors_ptr as *const raw::AiColor4D,
                mesh.mNumVertices as usize,
            )
        }
    }

    /// Get raw vertex colors for a specific channel as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn vertex_colors_bytes(&self, channel: usize) -> &[u8] {
        bytemuck::cast_slice(self.vertex_colors_raw(channel))
    }

    /// Get raw vertex colors for a specific channel as a flat `f32` slice (r,g,b,a interleaved).
    #[cfg(feature = "bytemuck")]
    pub fn vertex_colors_f32(&self, channel: usize) -> &[f32] {
        bytemuck::cast_slice(self.vertex_colors_raw(channel))
    }

    /// Get raw vertex colors for a specific channel (zero-copy), returning `None` when absent.
    pub fn vertex_colors_raw_opt(&self, channel: usize) -> Option<&[raw::AiColor4D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return None;
        }
        let mesh = self.raw();
        let ptr = mesh.mColors[channel] as *const raw::AiColor4D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, mesh.mNumVertices as usize) })
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
        self.raw().mNumFaces as usize
    }

    /// Iterate triangle index triplets (`[u32; 3]`) without allocation.
    ///
    /// This yields only faces whose index count is exactly 3. If you run Assimp with
    /// `PostProcessSteps::TRIANGULATE`, this will typically yield one item per face.
    pub fn triangles_iter(&self) -> impl Iterator<Item = [u32; 3]> + '_ {
        self.faces_iter().filter_map(|face| {
            let idx = face.indices_raw();
            (idx.len() == 3).then(|| [idx[0], idx[1], idx[2]])
        })
    }

    /// Collect triangle index triplets (`[u32; 3]`) into a `Vec`.
    ///
    /// Prefer [`Mesh::triangles_iter`] for a zero-allocation option.
    pub fn triangles(&self) -> Vec<[u32; 3]> {
        self.triangles_iter().collect()
    }

    /// Iterate triangle indices as a flat stream (`u32`) without allocation.
    ///
    /// This is convenient for feeding graphics APIs expecting a contiguous index buffer.
    pub fn triangle_indices_iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.triangles_iter().flatten()
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
        let mesh = self.raw();
        let n = mesh.mNumFaces as usize;
        debug_assert!(n == 0 || !mesh.mFaces.is_null());
        unsafe { ffi::slice_from_ptr_len(self, mesh.mFaces as *const raw::AiFace, n) }
    }

    /// Get the raw face array (zero-copy), returning `None` when absent.
    pub fn faces_raw_opt(&self) -> Option<&[raw::AiFace]> {
        let mesh = self.raw();
        let n = mesh.mNumFaces as usize;
        let ptr = mesh.mFaces as *const raw::AiFace;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, n) })
        }
    }

    /// Iterate faces without allocation.
    pub fn faces_iter(&self) -> impl Iterator<Item = Face> + '_ {
        self.faces()
    }

    /// Get the material index for this mesh
    pub fn material_index(&self) -> usize {
        self.raw().mMaterialIndex as usize
    }

    /// Get the primitive types present in this mesh
    pub fn primitive_types(&self) -> u32 {
        self.raw().mPrimitiveTypes
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
        crate::aabb::from_sys_aabb(&self.raw().mAABB)
    }

    /// Get the number of animation meshes (morph targets)
    pub fn num_anim_meshes(&self) -> usize {
        let mesh = self.raw();
        if mesh.mAnimMeshes.is_null() {
            0
        } else {
            mesh.mNumAnimMeshes as usize
        }
    }

    /// Get an animation mesh by index
    pub fn anim_mesh(&self, index: usize) -> Option<AnimMesh> {
        if index >= self.num_anim_meshes() {
            return None;
        }
        let mesh = self.raw();
        let ptr = unsafe {
            ffi::ptr_array_get(self, mesh.mAnimMeshes, mesh.mNumAnimMeshes as usize, index)
        }?;
        let anim_ptr = SharedPtr::new(ptr as *const sys::aiAnimMesh)?;
        Some(AnimMesh {
            scene: self.scene.clone(),
            anim_ptr,
        })
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
        let mesh = self.raw();
        if mesh.mBones.is_null() {
            0
        } else {
            mesh.mNumBones as usize
        }
    }

    /// Get a bone by index
    pub fn bone(&self, index: usize) -> Option<Bone> {
        if index >= self.num_bones() {
            return None;
        }

        let mesh = self.raw();
        let bone_ptr =
            unsafe { ffi::ptr_array_get(self, mesh.mBones, mesh.mNumBones as usize, index) }?;
        unsafe { Bone::from_raw(self.scene.clone(), bone_ptr as *const sys::aiBone) }.ok()
    }

    /// Get an iterator over all bones in the mesh
    pub fn bones(&self) -> BoneIterator {
        let mesh = self.raw();
        unsafe { BoneIterator::new(self.scene.clone(), mesh.mBones, self.num_bones()) }
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
        self.bone_names_iter().collect()
    }

    /// Iterate bone names (allocates per item, but avoids allocating a `Vec`).
    pub fn bone_names_iter(&self) -> impl Iterator<Item = String> + '_ {
        self.bones().map(|bone| bone.name())
    }

    /// Get the mesh morphing method (if any)
    pub fn morphing_method(&self) -> MorphingMethod {
        MorphingMethod::from_sys(self.raw().mMethod)
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
    #[inline]
    fn raw(&self) -> &raw::AiFace {
        unsafe { &*self.face_ptr.as_ptr() }
    }

    /// Get the number of indices in this face
    pub fn num_indices(&self) -> usize {
        self.raw().mNumIndices as usize
    }

    /// Get the raw index slice (zero-copy).
    pub fn indices_raw(&self) -> &[u32] {
        let face = self.raw();
        debug_assert!(face.mNumIndices == 0 || !face.mIndices.is_null());
        unsafe {
            ffi::slice_from_ptr_len(self, face.mIndices as *const u32, face.mNumIndices as usize)
        }
    }

    /// Get the raw index slice as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn indices_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.indices_raw())
    }

    /// Get the raw index slice (zero-copy), returning `None` when absent.
    pub fn indices_raw_opt(&self) -> Option<&[u32]> {
        let face = self.raw();
        unsafe {
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
            let faces = ffi::slice_from_ptr_len_opt(mesh, mesh.mFaces, mesh.mNumFaces as usize)?;
            let face_ref = faces.get(self.index)?;
            self.index += 1;
            let face_ptr = SharedPtr::new(std::ptr::from_ref(face_ref).cast::<raw::AiFace>())?;
            Some(Face {
                scene: self.scene.clone(),
                face_ptr,
            })
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
    #[inline]
    fn raw(&self) -> &sys::aiAnimMesh {
        unsafe { &*self.anim_ptr.as_ptr() }
    }

    /// Name of this anim mesh (if present)
    pub fn name(&self) -> String {
        crate::types::ai_string_to_string(&self.raw().mName)
    }
    /// Number of vertices in this anim mesh
    pub fn num_vertices(&self) -> usize {
        self.raw().mNumVertices as usize
    }

    /// Returns `true` if this anim mesh has replacement positions.
    pub fn has_vertices(&self) -> bool {
        let m = self.raw();
        m.mNumVertices > 0 && !m.mVertices.is_null()
    }

    /// Returns `true` if this anim mesh has replacement normals.
    pub fn has_normals(&self) -> bool {
        let m = self.raw();
        m.mNumVertices > 0 && !m.mNormals.is_null()
    }

    /// Returns `true` if this anim mesh has replacement tangents.
    pub fn has_tangents(&self) -> bool {
        let m = self.raw();
        m.mNumVertices > 0 && !m.mTangents.is_null()
    }

    /// Returns `true` if this anim mesh has replacement bitangents.
    pub fn has_bitangents(&self) -> bool {
        let m = self.raw();
        m.mNumVertices > 0 && !m.mBitangents.is_null()
    }

    /// Returns `true` if this anim mesh has replacement texture coordinates for `channel`.
    pub fn has_texture_coords(&self, channel: usize) -> bool {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return false;
        }
        let m = self.raw();
        m.mNumVertices > 0 && !m.mTextureCoords[channel].is_null()
    }

    /// Returns `true` if this anim mesh has replacement vertex colors for `channel`.
    pub fn has_vertex_colors(&self, channel: usize) -> bool {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return false;
        }
        let m = self.raw();
        m.mNumVertices > 0 && !m.mColors[channel].is_null()
    }

    /// Replacement positions (if present)
    pub fn vertices(&self) -> Option<Vec<Vector3D>> {
        self.vertices_raw_opt()
            .map(|vs| vs.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Raw replacement positions (zero-copy).
    pub fn vertices_raw(&self) -> &[raw::AiVector3D] {
        let m = self.raw();
        unsafe {
            ffi::slice_from_ptr_len(
                self,
                m.mVertices as *const raw::AiVector3D,
                m.mNumVertices as usize,
            )
        }
    }

    /// Raw replacement positions as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn vertices_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.vertices_raw())
    }

    /// Raw replacement positions as a flat `f32` slice (x,y,z interleaved).
    #[cfg(feature = "bytemuck")]
    pub fn vertices_f32(&self) -> &[f32] {
        bytemuck::cast_slice(self.vertices_raw())
    }

    /// Raw replacement positions (zero-copy), returning `None` when absent.
    pub fn vertices_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        let m = self.raw();
        let ptr = m.mVertices as *const raw::AiVector3D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize) })
        }
    }

    /// Replacement normals (if present)
    pub fn normals(&self) -> Option<Vec<Vector3D>> {
        self.normals_raw_opt()
            .map(|ns| ns.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Raw replacement normals (zero-copy).
    pub fn normals_raw(&self) -> &[raw::AiVector3D] {
        let m = self.raw();
        unsafe {
            ffi::slice_from_ptr_len(
                self,
                m.mNormals as *const raw::AiVector3D,
                m.mNumVertices as usize,
            )
        }
    }

    /// Raw replacement normals as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn normals_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.normals_raw())
    }

    /// Raw replacement normals as a flat `f32` slice (x,y,z interleaved).
    #[cfg(feature = "bytemuck")]
    pub fn normals_f32(&self) -> &[f32] {
        bytemuck::cast_slice(self.normals_raw())
    }

    /// Raw replacement normals (zero-copy), returning `None` when absent.
    pub fn normals_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        let m = self.raw();
        let ptr = m.mNormals as *const raw::AiVector3D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize) })
        }
    }

    /// Replacement tangents (if present)
    pub fn tangents(&self) -> Option<Vec<Vector3D>> {
        self.tangents_raw_opt()
            .map(|ts| ts.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Raw replacement tangents (zero-copy).
    pub fn tangents_raw(&self) -> &[raw::AiVector3D] {
        let m = self.raw();
        unsafe {
            ffi::slice_from_ptr_len(
                self,
                m.mTangents as *const raw::AiVector3D,
                m.mNumVertices as usize,
            )
        }
    }

    /// Raw replacement tangents as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn tangents_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.tangents_raw())
    }

    /// Raw replacement tangents as a flat `f32` slice (x,y,z interleaved).
    #[cfg(feature = "bytemuck")]
    pub fn tangents_f32(&self) -> &[f32] {
        bytemuck::cast_slice(self.tangents_raw())
    }

    /// Raw replacement tangents (zero-copy), returning `None` when absent.
    pub fn tangents_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        let m = self.raw();
        let ptr = m.mTangents as *const raw::AiVector3D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize) })
        }
    }

    /// Replacement bitangents (if present)
    pub fn bitangents(&self) -> Option<Vec<Vector3D>> {
        self.bitangents_raw_opt()
            .map(|bs| bs.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Raw replacement bitangents (zero-copy).
    pub fn bitangents_raw(&self) -> &[raw::AiVector3D] {
        let m = self.raw();
        unsafe {
            ffi::slice_from_ptr_len(
                self,
                m.mBitangents as *const raw::AiVector3D,
                m.mNumVertices as usize,
            )
        }
    }

    /// Raw replacement bitangents as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn bitangents_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.bitangents_raw())
    }

    /// Raw replacement bitangents as a flat `f32` slice (x,y,z interleaved).
    #[cfg(feature = "bytemuck")]
    pub fn bitangents_f32(&self) -> &[f32] {
        bytemuck::cast_slice(self.bitangents_raw())
    }

    /// Raw replacement bitangents (zero-copy), returning `None` when absent.
    pub fn bitangents_raw_opt(&self) -> Option<&[raw::AiVector3D]> {
        let m = self.raw();
        let ptr = m.mBitangents as *const raw::AiVector3D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize) })
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
        let m = self.raw();
        let ptr = m.mColors[channel];
        unsafe {
            ffi::slice_from_ptr_len(self, ptr as *const raw::AiColor4D, m.mNumVertices as usize)
        }
    }

    /// Raw replacement vertex colors for a specific channel (zero-copy), returning `None` when absent.
    pub fn vertex_colors_raw_opt(&self, channel: usize) -> Option<&[raw::AiColor4D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return None;
        }
        let m = self.raw();
        let ptr = m.mColors[channel] as *const raw::AiColor4D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize) })
        }
    }

    /// Replacement texture coordinates for a specific channel
    pub fn texture_coords(&self, channel: usize) -> Option<Vec<Vector3D>> {
        self.texture_coords_raw_opt(channel)
            .map(|uvs| uvs.iter().map(|v| Vector3D::new(v.x, v.y, v.z)).collect())
    }

    /// Replacement texture coordinates (Vec2) for a specific channel.
    ///
    /// This is a convenience for the common case where UVs are 2D; it discards the third component.
    pub fn texture_coords2(&self, channel: usize) -> Option<Vec<Vector2D>> {
        self.texture_coords_raw_opt(channel)
            .map(|uvs| uvs.iter().map(|v| Vector2D::new(v.x, v.y)).collect())
    }

    /// Raw replacement texture coordinates for a specific channel (zero-copy).
    pub fn texture_coords_raw(&self, channel: usize) -> &[raw::AiVector3D] {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return &[];
        }
        let m = self.raw();
        let ptr = m.mTextureCoords[channel];
        unsafe {
            ffi::slice_from_ptr_len(self, ptr as *const raw::AiVector3D, m.mNumVertices as usize)
        }
    }

    /// Raw replacement texture coordinates for a specific channel as bytes (zero-copy).
    #[cfg(feature = "bytemuck")]
    pub fn texture_coords_bytes(&self, channel: usize) -> &[u8] {
        bytemuck::cast_slice(self.texture_coords_raw(channel))
    }

    /// Raw replacement texture coordinates for a specific channel as a flat `f32` slice (x,y,z interleaved).
    #[cfg(feature = "bytemuck")]
    pub fn texture_coords_f32(&self, channel: usize) -> &[f32] {
        bytemuck::cast_slice(self.texture_coords_raw(channel))
    }

    /// Raw replacement texture coordinates for a specific channel (zero-copy), returning `None` when absent.
    pub fn texture_coords_raw_opt(&self, channel: usize) -> Option<&[raw::AiVector3D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return None;
        }
        let m = self.raw();
        let ptr = m.mTextureCoords[channel] as *const raw::AiVector3D;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { ffi::slice_from_ptr_len(self, ptr, m.mNumVertices as usize) })
        }
    }

    /// Iterate replacement texture coordinates (Vec2) without allocation.
    ///
    /// This is a convenience for the common case where UVs are 2D; it discards the third component.
    pub fn texture_coords_iter2(&self, channel: usize) -> impl Iterator<Item = Vector2D> + '_ {
        self.texture_coords_raw(channel)
            .iter()
            .map(|v| Vector2D::new(v.x, v.y))
    }

    /// Weight of this anim mesh
    pub fn weight(&self) -> f32 {
        self.raw().mWeight
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
            let meshes =
                ffi::slice_from_ptr_len_opt(mesh, mesh.mAnimMeshes, mesh.mNumAnimMeshes as usize)?;
            while self.index < meshes.len() {
                let ptr = meshes[self.index];
                self.index += 1;
                if ptr.is_null() {
                    continue;
                }
                let anim_ptr = SharedPtr::new(ptr as *const sys::aiAnimMesh)?;
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
