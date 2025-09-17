//! Mesh representation and utilities

#![allow(clippy::unnecessary_cast)]

use crate::{
    aabb::AABB,
    bone::{Bone, BoneIterator},
    sys,
    types::{from_ai_color4d, from_ai_vector3d, Color4D, Vector3D},
};

/// A mesh containing vertices, faces, and other geometric data
pub struct Mesh {
    mesh_ptr: *const sys::aiMesh,
}

impl Mesh {
    /// Create a Mesh from a raw Assimp mesh pointer
    pub(crate) fn from_raw(mesh_ptr: *const sys::aiMesh) -> Self {
        Self { mesh_ptr }
    }

    /// Get the raw mesh pointer
    pub fn as_raw(&self) -> *const sys::aiMesh {
        self.mesh_ptr
    }

    /// Get the name of the mesh
    pub fn name(&self) -> String {
        unsafe {
            let mesh = &*self.mesh_ptr;
            let name_ptr = mesh.mName.data.as_ptr();
            std::ffi::CStr::from_ptr(name_ptr)
                .to_string_lossy()
                .into_owned()
        }
    }

    /// Get the number of vertices in the mesh
    pub fn num_vertices(&self) -> usize {
        unsafe { (*self.mesh_ptr).mNumVertices as usize }
    }

    /// Get the vertices of the mesh
    pub fn vertices(&self) -> Vec<Vector3D> {
        unsafe {
            let mesh = &*self.mesh_ptr;
            if mesh.mVertices.is_null() {
                Vec::new()
            } else {
                let ai_vertices =
                    std::slice::from_raw_parts(mesh.mVertices, mesh.mNumVertices as usize);
                ai_vertices.iter().map(|&v| from_ai_vector3d(v)).collect()
            }
        }
    }

    /// Get the normals of the mesh
    pub fn normals(&self) -> Option<Vec<Vector3D>> {
        unsafe {
            let mesh = &*self.mesh_ptr;
            if mesh.mNormals.is_null() {
                None
            } else {
                let ai_normals =
                    std::slice::from_raw_parts(mesh.mNormals, mesh.mNumVertices as usize);
                Some(ai_normals.iter().map(|&v| from_ai_vector3d(v)).collect())
            }
        }
    }

    /// Get the tangents of the mesh
    pub fn tangents(&self) -> Option<Vec<Vector3D>> {
        unsafe {
            let mesh = &*self.mesh_ptr;
            if mesh.mTangents.is_null() {
                None
            } else {
                let ai_tangents =
                    std::slice::from_raw_parts(mesh.mTangents, mesh.mNumVertices as usize);
                Some(ai_tangents.iter().map(|&v| from_ai_vector3d(v)).collect())
            }
        }
    }

    /// Get the bitangents of the mesh
    pub fn bitangents(&self) -> Option<Vec<Vector3D>> {
        unsafe {
            let mesh = &*self.mesh_ptr;
            if mesh.mBitangents.is_null() {
                None
            } else {
                let ai_bitangents =
                    std::slice::from_raw_parts(mesh.mBitangents, mesh.mNumVertices as usize);
                Some(ai_bitangents.iter().map(|&v| from_ai_vector3d(v)).collect())
            }
        }
    }

    /// Get texture coordinates for a specific channel
    pub fn texture_coords(&self, channel: usize) -> Option<Vec<Vector3D>> {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return None;
        }

        unsafe {
            let mesh = &*self.mesh_ptr;
            let tex_coords_ptr = mesh.mTextureCoords[channel];
            if tex_coords_ptr.is_null() {
                None
            } else {
                let ai_tex_coords =
                    std::slice::from_raw_parts(tex_coords_ptr, mesh.mNumVertices as usize);
                Some(ai_tex_coords.iter().map(|&v| from_ai_vector3d(v)).collect())
            }
        }
    }

    /// Get vertex colors for a specific channel
    pub fn vertex_colors(&self, channel: usize) -> Option<Vec<Color4D>> {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return None;
        }

        unsafe {
            let mesh = &*self.mesh_ptr;
            let colors_ptr = mesh.mColors[channel];
            if colors_ptr.is_null() {
                None
            } else {
                let ai_colors = std::slice::from_raw_parts(colors_ptr, mesh.mNumVertices as usize);
                Some(ai_colors.iter().map(|&c| from_ai_color4d(c)).collect())
            }
        }
    }

    /// Get the number of faces in the mesh
    pub fn num_faces(&self) -> usize {
        unsafe { (*self.mesh_ptr).mNumFaces as usize }
    }

    /// Get the faces of the mesh
    pub fn faces(&self) -> FaceIterator {
        FaceIterator {
            mesh_ptr: self.mesh_ptr,
            index: 0,
        }
    }

    /// Get the material index for this mesh
    pub fn material_index(&self) -> usize {
        unsafe { (*self.mesh_ptr).mMaterialIndex as usize }
    }

    /// Get the primitive types present in this mesh
    pub fn primitive_types(&self) -> u32 {
        unsafe { (*self.mesh_ptr).mPrimitiveTypes }
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
            let mesh = &*self.mesh_ptr;
            AABB::from(&mesh.mAABB)
        }
    }

    /// Get the number of animation meshes (morph targets)
    pub fn num_anim_meshes(&self) -> usize {
        unsafe { (*self.mesh_ptr).mNumAnimMeshes as usize }
    }

    /// Get an animation mesh by index
    pub fn anim_mesh(&self, index: usize) -> Option<AnimMesh> {
        if index >= self.num_anim_meshes() {
            return None;
        }
        unsafe {
            let mesh = &*self.mesh_ptr;
            let ptr = *mesh.mAnimMeshes.add(index);
            if ptr.is_null() {
                None
            } else {
                Some(AnimMesh { anim_ptr: ptr })
            }
        }
    }

    /// Iterate over animation meshes
    pub fn anim_meshes(&self) -> AnimMeshIterator {
        AnimMeshIterator {
            mesh_ptr: self.mesh_ptr,
            index: 0,
        }
    }

    /// Get the number of bones in the mesh
    pub fn num_bones(&self) -> usize {
        unsafe { (*self.mesh_ptr).mNumBones as usize }
    }

    /// Get a bone by index
    pub fn bone(&self, index: usize) -> Option<Bone> {
        if index >= self.num_bones() {
            return None;
        }

        unsafe {
            let mesh = &*self.mesh_ptr;
            let bone_ptr = *mesh.mBones.add(index);
            if bone_ptr.is_null() {
                None
            } else {
                Bone::from_raw(bone_ptr).ok()
            }
        }
    }

    /// Get an iterator over all bones in the mesh
    pub fn bones(&self) -> BoneIterator {
        unsafe {
            let mesh = &*self.mesh_ptr;
            BoneIterator::new(mesh.mBones, self.num_bones())
        }
    }

    /// Check if the mesh has bones (is rigged for skeletal animation)
    pub fn has_bones(&self) -> bool {
        self.num_bones() > 0
    }

    /// Find a bone by name
    pub fn find_bone_by_name(&self, name: &str) -> Option<Bone> {
        self.bones().find(|bone| bone.name() == name)
    }

    /// Get all bone names
    pub fn bone_names(&self) -> Vec<String> {
        self.bones().map(|bone| bone.name()).collect()
    }

    /// Get the mesh morphing method (if any)
    pub fn morphing_method(&self) -> MorphingMethod {
        unsafe {
            let mesh = &*self.mesh_ptr;
            MorphingMethod::from_sys(mesh.mMethod)
        }
    }
}

/// A face in a mesh
pub struct Face {
    face_ptr: *const sys::aiFace,
}

impl Face {
    /// Get the number of indices in this face
    pub fn num_indices(&self) -> usize {
        unsafe { (*self.face_ptr).mNumIndices as usize }
    }

    /// Get the indices of this face
    pub fn indices(&self) -> &[u32] {
        unsafe {
            let face = &*self.face_ptr;
            std::slice::from_raw_parts(face.mIndices, face.mNumIndices as usize)
        }
    }
}

/// Iterator over faces in a mesh
pub struct FaceIterator {
    mesh_ptr: *const sys::aiMesh,
    index: usize,
}

impl Iterator for FaceIterator {
    type Item = Face;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mesh = &*self.mesh_ptr;
            if self.index >= mesh.mNumFaces as usize {
                None
            } else {
                let face_ptr = mesh.mFaces.add(self.index);
                self.index += 1;
                Some(Face { face_ptr })
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let mesh = &*self.mesh_ptr;
            let remaining = (mesh.mNumFaces as usize).saturating_sub(self.index);
            (remaining, Some(remaining))
        }
    }
}

impl ExactSizeIterator for FaceIterator {}

/// An animation mesh (morph target) that replaces certain vertex streams
pub struct AnimMesh {
    anim_ptr: *const sys::aiAnimMesh,
}

impl AnimMesh {
    /// Name of this anim mesh (if present)
    pub fn name(&self) -> String {
        unsafe {
            let n = &(*self.anim_ptr).mName;
            if n.length == 0 {
                return String::new();
            }
            let slice = std::slice::from_raw_parts(n.data.as_ptr() as *const u8, n.length as usize);
            String::from_utf8_lossy(slice).into_owned()
        }
    }
    /// Number of vertices in this anim mesh
    pub fn num_vertices(&self) -> usize {
        unsafe { (*self.anim_ptr).mNumVertices as usize }
    }

    /// Replacement positions (if present)
    pub fn vertices(&self) -> Option<Vec<Vector3D>> {
        unsafe {
            let m = &*self.anim_ptr;
            if m.mVertices.is_null() {
                None
            } else {
                let slice = std::slice::from_raw_parts(m.mVertices, m.mNumVertices as usize);
                Some(slice.iter().map(|&v| from_ai_vector3d(v)).collect())
            }
        }
    }

    /// Replacement normals (if present)
    pub fn normals(&self) -> Option<Vec<Vector3D>> {
        unsafe {
            let m = &*self.anim_ptr;
            if m.mNormals.is_null() {
                None
            } else {
                let slice = std::slice::from_raw_parts(m.mNormals, m.mNumVertices as usize);
                Some(slice.iter().map(|&v| from_ai_vector3d(v)).collect())
            }
        }
    }

    /// Replacement tangents (if present)
    pub fn tangents(&self) -> Option<Vec<Vector3D>> {
        unsafe {
            let m = &*self.anim_ptr;
            if m.mTangents.is_null() {
                None
            } else {
                let slice = std::slice::from_raw_parts(m.mTangents, m.mNumVertices as usize);
                Some(slice.iter().map(|&v| from_ai_vector3d(v)).collect())
            }
        }
    }

    /// Replacement bitangents (if present)
    pub fn bitangents(&self) -> Option<Vec<Vector3D>> {
        unsafe {
            let m = &*self.anim_ptr;
            if m.mBitangents.is_null() {
                None
            } else {
                let slice = std::slice::from_raw_parts(m.mBitangents, m.mNumVertices as usize);
                Some(slice.iter().map(|&v| from_ai_vector3d(v)).collect())
            }
        }
    }

    /// Replacement vertex colors for a specific channel
    pub fn vertex_colors(&self, channel: usize) -> Option<Vec<Color4D>> {
        if channel >= sys::AI_MAX_NUMBER_OF_COLOR_SETS as usize {
            return None;
        }
        unsafe {
            let m = &*self.anim_ptr;
            let ptr = m.mColors[channel];
            if ptr.is_null() {
                None
            } else {
                let slice = std::slice::from_raw_parts(ptr, m.mNumVertices as usize);
                Some(slice.iter().map(|&c| from_ai_color4d(c)).collect())
            }
        }
    }

    /// Replacement texture coordinates for a specific channel
    pub fn texture_coords(&self, channel: usize) -> Option<Vec<Vector3D>> {
        if channel >= sys::AI_MAX_NUMBER_OF_TEXTURECOORDS as usize {
            return None;
        }
        unsafe {
            let m = &*self.anim_ptr;
            let ptr = m.mTextureCoords[channel];
            if ptr.is_null() {
                None
            } else {
                let slice = std::slice::from_raw_parts(ptr, m.mNumVertices as usize);
                Some(slice.iter().map(|&v| from_ai_vector3d(v)).collect())
            }
        }
    }

    /// Weight of this anim mesh
    pub fn weight(&self) -> f32 {
        unsafe { (*self.anim_ptr).mWeight }
    }
}

/// Iterator over anim meshes
pub struct AnimMeshIterator {
    mesh_ptr: *const sys::aiMesh,
    index: usize,
}

impl Iterator for AnimMeshIterator {
    type Item = AnimMesh;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mesh = &*self.mesh_ptr;
            if self.index >= mesh.mNumAnimMeshes as usize {
                None
            } else {
                let ptr = *mesh.mAnimMeshes.add(self.index);
                self.index += 1;
                if ptr.is_null() {
                    None
                } else {
                    Some(AnimMesh { anim_ptr: ptr })
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let mesh = &*self.mesh_ptr;
            let remaining = (mesh.mNumAnimMeshes as usize).saturating_sub(self.index);
            (remaining, Some(remaining))
        }
    }
}

impl ExactSizeIterator for AnimMeshIterator {}

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
