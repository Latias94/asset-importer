//! Mesh representation and utilities

#![allow(clippy::unnecessary_cast)]

use std::marker::PhantomData;

use crate::{
    aabb::AABB,
    bone::{Bone, BoneIterator},
    ptr::SharedPtr,
    sys,
    types::{Color4D, Vector3D, ai_string_to_string},
};

/// A mesh containing vertices, faces, and other geometric data
pub struct Mesh<'a> {
    mesh_ptr: SharedPtr<sys::aiMesh>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Mesh<'a> {
    /// Create a Mesh from a raw Assimp mesh pointer
    ///
    /// # Safety
    /// Caller must ensure `mesh_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(mesh_ptr: *const sys::aiMesh) -> Self {
        let mesh_ptr = SharedPtr::new(mesh_ptr).expect("aiMesh pointer is null");
        Self {
            mesh_ptr,
            _marker: PhantomData,
        }
    }

    /// Get the raw mesh pointer
    pub fn as_raw(&self) -> *const sys::aiMesh {
        self.mesh_ptr.as_ptr()
    }

    /// Get the name of the mesh
    pub fn name(&self) -> String {
        unsafe { ai_string_to_string(&(*self.mesh_ptr.as_ptr()).mName) }
    }

    /// Get the number of vertices in the mesh
    pub fn num_vertices(&self) -> usize {
        unsafe { (*self.mesh_ptr.as_ptr()).mNumVertices as usize }
    }

    /// Get the vertices of the mesh
    pub fn vertices(&self) -> &[Vector3D] {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();

            if mesh.mVertices.is_null() {
                return &[];
            }

            // SAFETY:
            // Safe to reinterpret as ai_real has a defined size matching
            // our Vector3D allowing us to reinterpret the vector to glam types
            // without element-wise mapping.
            std::slice::from_raw_parts(
                mesh.mVertices as *const Vector3D,
                mesh.mNumVertices as usize,
            )
        }
    }

    /// Get the normals of the mesh
    pub fn normals(&self) -> Option<&[Vector3D]> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();

            if mesh.mNormals.is_null() {
                return None;
            }

            // SAFETY: Read vertices.
            Some(std::slice::from_raw_parts(
                mesh.mNormals as *const Vector3D,
                mesh.mNumVertices as usize,
            ))
        }
    }

    /// Get the tangents of the mesh
    pub fn tangents(&self) -> Option<&[Vector3D]> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();

            if mesh.mTangents.is_null() {
                return None;
            }

            // SAFETY: Same explanation as vertices
            Some(std::slice::from_raw_parts(
                mesh.mTangents as *const Vector3D,
                mesh.mNumVertices as usize,
            ))
        }
    }

    /// Get the bitangents of the mesh
    pub fn bitangents(&self) -> Option<&[Vector3D]> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();

            if mesh.mBitangents.is_null() {
                return None;
            }

            // SAFETY: Same explanation as vertices
            Some(std::slice::from_raw_parts(
                mesh.mBitangents as *const Vector3D,
                mesh.mNumVertices as usize,
            ))
        }
    }

    /// Get texture coordinates for a specific channel
    pub fn texture_coords(&self, channel: usize) -> Option<&[Vector3D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return None;
        }

        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let tex_coords_ptr = mesh.mTextureCoords[channel];

            if tex_coords_ptr.is_null() {
                return None;
            }

            // SAFETY: Same explanation as vertices
            Some(std::slice::from_raw_parts(
                tex_coords_ptr as *const Vector3D,
                mesh.mNumVertices as usize,
            ))
        }
    }

    /// Get vertex colors for a specific channel
    pub fn vertex_colors(&self, channel: usize) -> Option<&[Color4D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return None;
        }

        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let colors_ptr = mesh.mColors[channel];

            if colors_ptr.is_null() {
                return None;
            }

            // SAFETY: Same explanation as vertices
            Some(std::slice::from_raw_parts(
                colors_ptr as *const Color4D,
                mesh.mNumVertices as usize,
            ))
        }
    }

    /// Get the number of faces in the mesh
    pub fn num_faces(&self) -> usize {
        unsafe { (*self.mesh_ptr.as_ptr()).mNumFaces as usize }
    }

    /// Get the faces of the mesh
    pub fn faces(&self) -> FaceIterator<'a> {
        FaceIterator {
            mesh_ptr: self.mesh_ptr,
            index: 0,
            _marker: PhantomData,
        }
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
            AABB::from(&mesh.mAABB)
        }
    }

    /// Get the number of animation meshes (morph targets)
    pub fn num_anim_meshes(&self) -> usize {
        unsafe { (*self.mesh_ptr.as_ptr()).mNumAnimMeshes as usize }
    }

    /// Get an animation mesh by index
    pub fn anim_mesh(&self, index: usize) -> Option<AnimMesh<'a>> {
        if index >= self.num_anim_meshes() {
            return None;
        }
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let ptr = *mesh.mAnimMeshes.add(index);
            if ptr.is_null() {
                None
            } else {
                Some(AnimMesh {
                    anim_ptr: SharedPtr::new(ptr).expect("aiAnimMesh pointer is null"),
                    _marker: PhantomData,
                })
            }
        }
    }

    /// Iterate over animation meshes
    pub fn anim_meshes(&self) -> AnimMeshIterator<'a> {
        AnimMeshIterator {
            mesh_ptr: self.mesh_ptr,
            index: 0,
            _marker: PhantomData,
        }
    }

    /// Get the number of bones in the mesh
    pub fn num_bones(&self) -> usize {
        unsafe { (*self.mesh_ptr.as_ptr()).mNumBones as usize }
    }

    /// Get a bone by index
    pub fn bone(&self, index: usize) -> Option<Bone<'a>> {
        if index >= self.num_bones() {
            return None;
        }

        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let bone_ptr = *mesh.mBones.add(index);
            if bone_ptr.is_null() {
                None
            } else {
                Bone::from_raw(bone_ptr).ok()
            }
        }
    }

    /// Get an iterator over all bones in the mesh
    pub fn bones(&self) -> BoneIterator<'a> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            BoneIterator::new(mesh.mBones, self.num_bones())
        }
    }

    /// Check if the mesh has bones (is rigged for skeletal animation)
    pub fn has_bones(&self) -> bool {
        self.num_bones() > 0
    }

    /// Find a bone by name
    pub fn find_bone_by_name(&self, name: &str) -> Option<Bone<'a>> {
        self.bones().find(|bone| bone.name() == name)
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
pub struct Face<'a> {
    face_ptr: SharedPtr<sys::aiFace>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Face<'a> {
    /// Get the number of indices in this face
    pub fn num_indices(&self) -> usize {
        unsafe { (*self.face_ptr.as_ptr()).mNumIndices as usize }
    }

    /// Get the indices of this face
    pub fn indices(&self) -> &[u32] {
        unsafe {
            let face = &*self.face_ptr.as_ptr();
            if face.mIndices.is_null() || face.mNumIndices == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(face.mIndices, face.mNumIndices as usize)
            }
        }
    }
}

/// Iterator over faces in a mesh
pub struct FaceIterator<'a> {
    mesh_ptr: SharedPtr<sys::aiMesh>,
    index: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Iterator for FaceIterator<'a> {
    type Item = Face<'a>;

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
                Some(Face {
                    face_ptr: SharedPtr::new(face_ptr).expect("aiFace pointer is null"),
                    _marker: PhantomData,
                })
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let remaining = (mesh.mNumFaces as usize).saturating_sub(self.index);
            (remaining, Some(remaining))
        }
    }
}

impl<'a> ExactSizeIterator for FaceIterator<'a> {}

/// An animation mesh (morph target) that replaces certain vertex streams
pub struct AnimMesh<'a> {
    anim_ptr: SharedPtr<sys::aiAnimMesh>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> AnimMesh<'a> {
    /// Name of this anim mesh (if present)
    pub fn name(&self) -> String {
        unsafe { crate::types::ai_string_to_string(&(*self.anim_ptr.as_ptr()).mName) }
    }
    /// Number of vertices in this anim mesh
    pub fn num_vertices(&self) -> usize {
        unsafe { (*self.anim_ptr.as_ptr()).mNumVertices as usize }
    }

    /// Replacement positions (if present)
    pub fn vertices(&self) -> Option<&[Vector3D]> {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();

            if m.mVertices.is_null() {
                return None;
            }

            // SAFETY: Same explanation as vertices
            Some(std::slice::from_raw_parts(
                m.mVertices as *const Vector3D,
                m.mNumVertices as usize,
            ))
        }
    }

    /// Replacement normals (if present)
    pub fn normals(&self) -> Option<&[Vector3D]> {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();

            if m.mNormals.is_null() {
                return None;
            }

            // SAFETY: Same explanation as vertices
            Some(std::slice::from_raw_parts(
                m.mNormals as *const Vector3D,
                m.mNumVertices as usize,
            ))
        }
    }

    /// Replacement tangents (if present)
    pub fn tangents(&self) -> Option<&[Vector3D]> {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();

            if m.mTangents.is_null() {
                return None;
            }

            // SAFETY: Same explanation as vertices
            Some(std::slice::from_raw_parts(
                m.mTangents as *const Vector3D,
                m.mNumVertices as usize,
            ))
        }
    }

    /// Replacement bitangents (if present)
    pub fn bitangents(&self) -> Option<&[Vector3D]> {
        unsafe {
            let m = &*self.anim_ptr.as_ptr();

            if m.mBitangents.is_null() {
                return None;
            }

            // SAFETY: Same explanation as vertices
            Some(std::slice::from_raw_parts(
                m.mBitangents as *const Vector3D,
                m.mNumVertices as usize,
            ))
        }
    }

    /// Replacement vertex colors for a specific channel
    pub fn vertex_colors(&self, channel: usize) -> Option<&[Color4D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return None;
        }

        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            let ptr = m.mColors[channel];

            if ptr.is_null() {
                return None;
            }

            // SAFETY: Same explanation as vertices
            Some(std::slice::from_raw_parts(
                ptr as *const Color4D,
                m.mNumVertices as usize,
            ))
        }
    }

    /// Replacement texture coordinates for a specific channel
    pub fn texture_coords(&self, channel: usize) -> Option<&[Vector3D]> {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return None;
        }
        unsafe {
            let m = &*self.anim_ptr.as_ptr();
            let ptr = m.mTextureCoords[channel];

            if ptr.is_null() {
                return None;
            }

            Some(std::slice::from_raw_parts(
                ptr as *const Vector3D,
                m.mNumVertices as usize,
            ))
        }
    }

    /// Weight of this anim mesh
    pub fn weight(&self) -> f32 {
        unsafe { (*self.anim_ptr.as_ptr()).mWeight }
    }
}

/// Iterator over anim meshes
pub struct AnimMeshIterator<'a> {
    mesh_ptr: SharedPtr<sys::aiMesh>,
    index: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Iterator for AnimMeshIterator<'a> {
    type Item = AnimMesh<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            if mesh.mAnimMeshes.is_null() || mesh.mNumAnimMeshes == 0 {
                return None;
            }
            if self.index >= mesh.mNumAnimMeshes as usize {
                None
            } else {
                let ptr = *mesh.mAnimMeshes.add(self.index);
                self.index += 1;
                if ptr.is_null() {
                    None
                } else {
                    Some(AnimMesh {
                        anim_ptr: SharedPtr::new(ptr).expect("aiAnimMesh pointer is null"),
                        _marker: PhantomData,
                    })
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let mesh = &*self.mesh_ptr.as_ptr();
            let remaining = (mesh.mNumAnimMeshes as usize).saturating_sub(self.index);
            (remaining, Some(remaining))
        }
    }
}

impl<'a> ExactSizeIterator for AnimMeshIterator<'a> {}

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
