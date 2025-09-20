//! Bone and skeletal animation support
//!
//! This module provides functionality for working with bones and vertex weights,
//! which are essential for skeletal animation in 3D models.

use crate::{
    error::{Error, Result},
    sys,
    types::Matrix4x4,
};

/// A vertex weight that associates a vertex with a bone
///
/// Each vertex can be influenced by multiple bones with different weights.
/// The sum of all weights for a vertex should typically equal 1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VertexWeight {
    /// The ID of the vertex this weight applies to
    pub vertex_id: u32,
    /// The weight value (typically 0.0 to 1.0)
    pub weight: f32,
}

impl VertexWeight {
    /// Create a new vertex weight
    pub fn new(vertex_id: u32, weight: f32) -> Self {
        Self { vertex_id, weight }
    }

    /// Check if this weight is significant (above a threshold)
    pub fn is_significant(&self, threshold: f32) -> bool {
        self.weight >= threshold
    }

    /// Normalize the weight to ensure it's in the range [0.0, 1.0]
    pub fn normalized(&self) -> Self {
        Self {
            vertex_id: self.vertex_id,
            weight: self.weight.clamp(0.0, 1.0),
        }
    }
}

impl From<&sys::aiVertexWeight> for VertexWeight {
    fn from(weight: &sys::aiVertexWeight) -> Self {
        Self {
            vertex_id: weight.mVertexId,
            weight: weight.mWeight,
        }
    }
}

/// A bone in a skeletal animation system
///
/// Bones define how vertices are transformed during animation.
/// Each bone has a name, an offset matrix, and a list of vertex weights.
#[derive(Debug)]
pub struct Bone {
    bone_ptr: *const sys::aiBone,
}

impl Bone {
    /// Create a bone wrapper from a raw Assimp bone pointer
    ///
    /// # Safety
    /// The caller must ensure that the pointer is valid and that the bone
    /// will not be freed while this Bone instance exists.
    pub(crate) unsafe fn from_raw(bone_ptr: *const sys::aiBone) -> Result<Self> {
        if bone_ptr.is_null() {
            return Err(Error::invalid_scene("Bone pointer is null"));
        }

        Ok(Self { bone_ptr })
    }

    /// Get the raw bone pointer
    pub fn as_raw(&self) -> *const sys::aiBone {
        self.bone_ptr
    }

    /// Get the name of the bone
    pub fn name(&self) -> String {
        unsafe {
            let ai_string = &(*self.bone_ptr).mName;
            if ai_string.length == 0 {
                return String::new();
            }

            let slice = std::slice::from_raw_parts(
                ai_string.data.as_ptr() as *const u8,
                ai_string.length as usize,
            );
            String::from_utf8_lossy(slice).to_string()
        }
    }

    /// Get the number of vertex weights for this bone
    pub fn num_weights(&self) -> usize {
        unsafe { (*self.bone_ptr).mNumWeights as usize }
    }

    /// Get the vertex weights for this bone
    pub fn weights(&self) -> Vec<VertexWeight> {
        unsafe {
            let bone = &*self.bone_ptr;
            if bone.mWeights.is_null() || bone.mNumWeights == 0 {
                return Vec::new();
            }

            let weights_slice =
                std::slice::from_raw_parts(bone.mWeights, bone.mNumWeights as usize);
            weights_slice.iter().map(VertexWeight::from).collect()
        }
    }

    /// Get a specific vertex weight by index
    pub fn weight(&self, index: usize) -> Option<VertexWeight> {
        if index >= self.num_weights() {
            return None;
        }

        unsafe {
            let bone = &*self.bone_ptr;
            let weight = &*bone.mWeights.add(index);
            Some(VertexWeight::from(weight))
        }
    }

    /// Get the offset matrix for this bone
    ///
    /// The offset matrix transforms vertices from mesh space to bone space.
    /// It's typically the inverse of the bone's transformation matrix in bind pose.
    pub fn offset_matrix(&self) -> Matrix4x4 {
        unsafe {
            let matrix = &(*self.bone_ptr).mOffsetMatrix;
            Matrix4x4::from_cols_array_2d(&[
                [matrix.a1, matrix.a2, matrix.a3, matrix.a4],
                [matrix.b1, matrix.b2, matrix.b3, matrix.b4],
                [matrix.c1, matrix.c2, matrix.c3, matrix.c4],
                [matrix.d1, matrix.d2, matrix.d3, matrix.d4],
            ])
        }
    }

    /// Get weights that affect a specific vertex
    pub fn weights_for_vertex(&self, vertex_id: u32) -> Vec<VertexWeight> {
        self.weights()
            .into_iter()
            .filter(|w| w.vertex_id == vertex_id)
            .collect()
    }

    /// Get weights above a certain threshold
    pub fn significant_weights(&self, threshold: f32) -> Vec<VertexWeight> {
        self.weights()
            .into_iter()
            .filter(|w| w.is_significant(threshold))
            .collect()
    }

    /// Get the total weight for all vertices (should typically be close to the number of affected vertices)
    pub fn total_weight(&self) -> f32 {
        self.weights().iter().map(|w| w.weight).sum()
    }

    /// Get the maximum weight value
    pub fn max_weight(&self) -> f32 {
        self.weights().iter().map(|w| w.weight).fold(0.0, f32::max)
    }

    /// Get the minimum weight value
    pub fn min_weight(&self) -> f32 {
        self.weights()
            .iter()
            .map(|w| w.weight)
            .fold(f32::INFINITY, f32::min)
    }

    /// Get the average weight value
    pub fn average_weight(&self) -> f32 {
        let weights = self.weights();
        if weights.is_empty() {
            0.0
        } else {
            weights.iter().map(|w| w.weight).sum::<f32>() / weights.len() as f32
        }
    }

    /// Get the list of vertex IDs affected by this bone
    pub fn affected_vertices(&self) -> Vec<u32> {
        self.weights().into_iter().map(|w| w.vertex_id).collect()
    }

    /// Check if this bone affects a specific vertex
    pub fn affects_vertex(&self, vertex_id: u32) -> bool {
        self.weights().iter().any(|w| w.vertex_id == vertex_id)
    }

    /// Get the weight value for a specific vertex (0.0 if not affected)
    pub fn weight_for_vertex(&self, vertex_id: u32) -> f32 {
        self.weights()
            .iter()
            .find(|w| w.vertex_id == vertex_id)
            .map(|w| w.weight)
            .unwrap_or(0.0)
    }

    /// Create a normalized version of this bone's weights
    ///
    /// This ensures all weights are in the range [0.0, 1.0] and can optionally
    /// normalize the total weight to 1.0 per vertex.
    pub fn normalized_weights(&self) -> Vec<VertexWeight> {
        self.weights().into_iter().map(|w| w.normalized()).collect()
    }
}

/// Iterator over bones in a mesh
pub struct BoneIterator {
    bones: *mut *mut sys::aiBone,
    count: usize,
    index: usize,
}

impl BoneIterator {
    /// Create a new bone iterator
    ///
    /// # Safety
    /// The caller must ensure that the bones pointer and count are valid.
    pub(crate) unsafe fn new(bones: *mut *mut sys::aiBone, count: usize) -> Self {
        Self {
            bones,
            count,
            index: 0,
        }
    }
}

impl Iterator for BoneIterator {
    type Item = Bone;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }

        unsafe {
            let bone_ptr = *self.bones.add(self.index);
            self.index += 1;

            Bone::from_raw(bone_ptr).ok()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count.saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for BoneIterator {
    fn len(&self) -> usize {
        self.count.saturating_sub(self.index)
    }
}

/// Utility functions for working with bones and weights
pub mod utils {
    use super::*;
    use std::collections::HashMap;

    /// Normalize vertex weights so that the total weight per vertex equals 1.0
    pub fn normalize_vertex_weights(bones: &[Bone]) -> HashMap<u32, Vec<(usize, f32)>> {
        let mut vertex_weights: HashMap<u32, Vec<(usize, f32)>> = HashMap::new();

        // Collect all weights per vertex
        for (bone_index, bone) in bones.iter().enumerate() {
            for weight in bone.weights() {
                vertex_weights
                    .entry(weight.vertex_id)
                    .or_default()
                    .push((bone_index, weight.weight));
            }
        }

        // Normalize weights per vertex
        for weights in vertex_weights.values_mut() {
            let total_weight: f32 = weights.iter().map(|(_, w)| *w).sum();
            if total_weight > 0.0 {
                for (_, weight) in weights.iter_mut() {
                    *weight /= total_weight;
                }
            }
        }

        vertex_weights
    }

    /// Find bones by name
    pub fn find_bones_by_name<'a>(bones: &'a [Bone], name: &str) -> Vec<&'a Bone> {
        bones.iter().filter(|bone| bone.name() == name).collect()
    }

    /// Get the maximum number of bones affecting any single vertex
    pub fn max_bones_per_vertex(bones: &[Bone]) -> usize {
        let mut vertex_bone_count: HashMap<u32, usize> = HashMap::new();

        for bone in bones {
            for weight in bone.weights() {
                *vertex_bone_count.entry(weight.vertex_id).or_insert(0) += 1;
            }
        }

        vertex_bone_count.values().copied().max().unwrap_or(0)
    }

    /// Filter out bones with weights below a threshold
    pub fn filter_significant_bones(bones: &[Bone], threshold: f32) -> Vec<usize> {
        bones
            .iter()
            .enumerate()
            .filter(|(_, bone)| bone.max_weight() >= threshold)
            .map(|(index, _)| index)
            .collect()
    }
}

// Send and Sync are safe because:
// 1. Bone only holds a pointer to data owned by the Scene
// 2. The Scene manages the lifetime of all Assimp data
// 3. Assimp doesn't use global state and is thread-safe for read operations
// 4. The pointer remains valid as long as the Scene exists
unsafe impl Send for Bone {}
unsafe impl Sync for Bone {}

// BoneIterator is also safe for the same reasons
unsafe impl Send for BoneIterator {}
unsafe impl Sync for BoneIterator {}
