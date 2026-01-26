//! Bone and skeletal animation support
//!
//! This module provides functionality for working with bones and vertex weights,
//! which are essential for skeletal animation in 3D models.

use crate::{
    error::{Error, Result},
    ffi,
    ptr::SharedPtr,
    raw,
    scene::Scene,
    sys,
    types::{Matrix4x4, ai_string_to_str, ai_string_to_string, from_ai_matrix4x4},
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

#[cfg(feature = "raw-sys")]
impl From<&sys::aiVertexWeight> for VertexWeight {
    fn from(weight: &sys::aiVertexWeight) -> Self {
        Self {
            vertex_id: weight.mVertexId,
            weight: weight.mWeight,
        }
    }
}

impl From<&raw::AiVertexWeight> for VertexWeight {
    fn from(weight: &raw::AiVertexWeight) -> Self {
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
#[derive(Debug, Clone)]
pub struct Bone {
    #[allow(dead_code)]
    scene: Scene,
    bone_ptr: SharedPtr<sys::aiBone>,
}

impl Bone {
    pub(crate) fn from_sys_ptr(scene: Scene, bone_ptr: *mut sys::aiBone) -> Result<Self> {
        let bone_ptr = SharedPtr::new(bone_ptr as *const sys::aiBone)
            .ok_or_else(|| Error::invalid_scene("Bone pointer is null"))?;
        Ok(Self { scene, bone_ptr })
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw_sys(&self) -> *const sys::aiBone {
        self.bone_ptr.as_ptr()
    }

    /// Get the raw bone pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> *const sys::aiBone {
        self.as_raw_sys()
    }

    #[inline]
    fn raw(&self) -> &sys::aiBone {
        self.bone_ptr.as_ref()
    }

    /// Get the name of the bone
    pub fn name(&self) -> String {
        ai_string_to_string(&self.raw().mName)
    }

    /// Get the name of the bone (zero-copy, lossy UTF-8).
    pub fn name_str(&self) -> std::borrow::Cow<'_, str> {
        ai_string_to_str(&self.raw().mName)
    }

    /// Get the number of vertex weights for this bone
    pub fn num_weights(&self) -> usize {
        let bone = self.raw();
        if bone.mWeights.is_null() {
            0
        } else {
            bone.mNumWeights as usize
        }
    }

    /// Get the vertex weights for this bone
    pub fn weights(&self) -> Vec<VertexWeight> {
        self.weights_iter().collect()
    }

    /// Get the raw vertex weight array (zero-copy).
    pub fn weights_raw(&self) -> &[raw::AiVertexWeight] {
        let bone = self.raw();
        debug_assert!(bone.mNumWeights == 0 || !bone.mWeights.is_null());
        ffi::slice_from_ptr_len(
            self,
            bone.mWeights as *const raw::AiVertexWeight,
            bone.mNumWeights as usize,
        )
    }

    /// Get the raw vertex weight array (zero-copy), returning `None` when absent.
    pub fn weights_raw_opt(&self) -> Option<&[raw::AiVertexWeight]> {
        let bone = self.raw();
        ffi::slice_from_ptr_len_opt(
            self,
            bone.mWeights as *const raw::AiVertexWeight,
            bone.mNumWeights as usize,
        )
    }

    /// Iterate vertex weights without allocation.
    pub fn weights_iter(&self) -> impl Iterator<Item = VertexWeight> + '_ {
        self.weights_raw().iter().map(VertexWeight::from)
    }

    /// Get a specific vertex weight by index
    pub fn weight(&self, index: usize) -> Option<VertexWeight> {
        self.weights_raw().get(index).map(VertexWeight::from)
    }

    /// Get the offset matrix for this bone
    ///
    /// The offset matrix transforms vertices from mesh space to bone space.
    /// It's typically the inverse of the bone's transformation matrix in bind pose.
    pub fn offset_matrix(&self) -> Matrix4x4 {
        from_ai_matrix4x4(self.raw().mOffsetMatrix)
    }

    /// Get weights that affect a specific vertex
    pub fn weights_for_vertex(&self, vertex_id: u32) -> Vec<VertexWeight> {
        self.weights_for_vertex_iter(vertex_id).collect()
    }

    /// Iterate weights that affect a specific vertex without allocating.
    pub fn weights_for_vertex_iter(
        &self,
        vertex_id: u32,
    ) -> impl Iterator<Item = VertexWeight> + '_ {
        self.weights_iter()
            .filter(move |w| w.vertex_id == vertex_id)
    }

    /// Get weights above a certain threshold
    pub fn significant_weights(&self, threshold: f32) -> Vec<VertexWeight> {
        self.significant_weights_iter(threshold).collect()
    }

    /// Iterate weights above a threshold without allocating.
    pub fn significant_weights_iter(
        &self,
        threshold: f32,
    ) -> impl Iterator<Item = VertexWeight> + '_ {
        self.weights_iter()
            .filter(move |w| w.is_significant(threshold))
    }

    /// Get the total weight for all vertices (should typically be close to the number of affected vertices)
    pub fn total_weight(&self) -> f32 {
        self.weights_iter().map(|w| w.weight).sum()
    }

    /// Get the maximum weight value
    pub fn max_weight(&self) -> f32 {
        self.weights_iter().map(|w| w.weight).fold(0.0, f32::max)
    }

    /// Get the minimum weight value
    pub fn min_weight(&self) -> f32 {
        self.weights_iter()
            .map(|w| w.weight)
            .fold(f32::INFINITY, f32::min)
    }

    /// Get the average weight value
    pub fn average_weight(&self) -> f32 {
        let ws = self.weights_raw();
        if ws.is_empty() {
            return 0.0;
        }
        let sum: f32 = ws.iter().map(|w| w.mWeight).sum();
        sum / (ws.len() as f32)
    }

    /// Get the list of vertex IDs affected by this bone
    pub fn affected_vertices(&self) -> Vec<u32> {
        self.affected_vertices_iter().collect()
    }

    /// Iterate vertex IDs affected by this bone without allocating.
    pub fn affected_vertices_iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.weights_iter().map(|w| w.vertex_id)
    }

    /// Check if this bone affects a specific vertex
    pub fn affects_vertex(&self, vertex_id: u32) -> bool {
        self.weights_iter().any(|w| w.vertex_id == vertex_id)
    }

    /// Get the weight value for a specific vertex (0.0 if not affected)
    pub fn weight_for_vertex(&self, vertex_id: u32) -> f32 {
        self.weights_iter()
            .find(|w| w.vertex_id == vertex_id)
            .map(|w| w.weight)
            .unwrap_or(0.0)
    }

    /// Create a normalized version of this bone's weights
    ///
    /// This ensures all weights are in the range [0.0, 1.0] and can optionally
    /// normalize the total weight to 1.0 per vertex.
    pub fn normalized_weights(&self) -> Vec<VertexWeight> {
        self.normalized_weights_iter().collect()
    }

    /// Iterate normalized weights without allocating.
    pub fn normalized_weights_iter(&self) -> impl Iterator<Item = VertexWeight> + '_ {
        self.weights_iter().map(|w| w.normalized())
    }
}

/// Iterator over bones in a mesh
pub struct BoneIterator {
    scene: Scene,
    bones: Option<SharedPtr<*const sys::aiBone>>,
    count: usize,
    index: usize,
}

impl BoneIterator {
    /// Create a new bone iterator
    pub(crate) fn new(scene: Scene, bones: *mut *mut sys::aiBone, count: usize) -> Self {
        let bones_ptr = SharedPtr::new(bones as *const *const sys::aiBone);
        let count = if bones_ptr.is_some() { count } else { 0 };
        Self {
            scene,
            bones: bones_ptr,
            count,
            index: 0,
        }
    }
}

impl Iterator for BoneIterator {
    type Item = Bone;

    fn next(&mut self) -> Option<Self::Item> {
        let bones = self.bones?;
        let slice = crate::ffi::slice_from_ptr_len_opt(&(), bones.as_ptr(), self.count)?;
        while self.index < slice.len() {
            let bone_ptr = slice[self.index];
            self.index += 1;
            if bone_ptr.is_null() {
                continue;
            }
            if let Ok(bone) = Bone::from_sys_ptr(self.scene.clone(), bone_ptr as *mut sys::aiBone) {
                return Some(bone);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count.saturating_sub(self.index);
        (0, Some(remaining))
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
            for weight in bone.weights_iter() {
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
        bones
            .iter()
            .filter(|bone| bone.name_str().as_ref() == name)
            .collect()
    }

    /// Get the maximum number of bones affecting any single vertex
    pub fn max_bones_per_vertex(bones: &[Bone]) -> usize {
        let mut vertex_bone_count: HashMap<u32, usize> = HashMap::new();

        for bone in bones {
            for weight in bone.weights_iter() {
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
