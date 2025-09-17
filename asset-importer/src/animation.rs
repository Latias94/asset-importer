//! Animation data structures and utilities

use crate::{
    error::c_str_to_string_or_empty,
    sys,
    types::{Quaternion, Vector3D},
};

/// An animation containing keyframes for various properties
pub struct Animation {
    animation_ptr: *const sys::aiAnimation,
}

impl Animation {
    /// Create an Animation from a raw Assimp animation pointer
    pub(crate) fn from_raw(animation_ptr: *const sys::aiAnimation) -> Self {
        Self { animation_ptr }
    }

    /// Get the raw animation pointer
    pub fn as_raw(&self) -> *const sys::aiAnimation {
        self.animation_ptr
    }

    /// Get the name of the animation
    pub fn name(&self) -> String {
        unsafe {
            let animation = &*self.animation_ptr;
            c_str_to_string_or_empty(animation.mName.data.as_ptr())
        }
    }

    /// Get the duration of the animation in ticks
    pub fn duration(&self) -> f64 {
        unsafe { (*self.animation_ptr).mDuration }
    }

    /// Get the ticks per second for this animation
    pub fn ticks_per_second(&self) -> f64 {
        unsafe {
            let tps = (*self.animation_ptr).mTicksPerSecond;
            if tps != 0.0 { tps } else { 25.0 } // Default to 25 FPS
        }
    }

    /// Get the duration in seconds
    pub fn duration_in_seconds(&self) -> f64 {
        self.duration() / self.ticks_per_second()
    }

    /// Get the number of node animation channels
    pub fn num_channels(&self) -> usize {
        unsafe { (*self.animation_ptr).mNumChannels as usize }
    }

    /// Get a node animation channel by index
    pub fn channel(&self, index: usize) -> Option<NodeAnimation> {
        if index >= self.num_channels() {
            return None;
        }

        unsafe {
            let animation = &*self.animation_ptr;
            let channel_ptr = *animation.mChannels.add(index);
            if channel_ptr.is_null() {
                None
            } else {
                Some(NodeAnimation::from_raw(channel_ptr))
            }
        }
    }

    /// Get an iterator over all node animation channels
    pub fn channels(&self) -> NodeAnimationIterator {
        NodeAnimationIterator {
            animation_ptr: self.animation_ptr,
            index: 0,
        }
    }

    /// Get the number of mesh animation channels (vertex anim via aiAnimMesh)
    pub fn num_mesh_channels(&self) -> usize {
        unsafe { (*self.animation_ptr).mNumMeshChannels as usize }
    }

    /// Get a mesh animation channel
    pub fn mesh_channel(&self, index: usize) -> Option<MeshAnimation> {
        if index >= self.num_mesh_channels() {
            return None;
        }
        unsafe {
            let ptr = *(*self.animation_ptr).mMeshChannels.add(index);
            if ptr.is_null() {
                None
            } else {
                Some(MeshAnimation { channel_ptr: ptr })
            }
        }
    }

    /// Iterate mesh animation channels
    pub fn mesh_channels(&self) -> MeshAnimationIterator {
        MeshAnimationIterator {
            animation_ptr: self.animation_ptr,
            index: 0,
        }
    }

    /// Get the number of morph mesh animation channels
    pub fn num_morph_mesh_channels(&self) -> usize {
        unsafe { (*self.animation_ptr).mNumMorphMeshChannels as usize }
    }

    /// Get a morph mesh animation channel
    pub fn morph_mesh_channel(&self, index: usize) -> Option<MorphMeshAnimation> {
        if index >= self.num_morph_mesh_channels() {
            return None;
        }
        unsafe {
            let ptr = *(*self.animation_ptr).mMorphMeshChannels.add(index);
            if ptr.is_null() {
                None
            } else {
                Some(MorphMeshAnimation { channel_ptr: ptr })
            }
        }
    }

    /// Iterate morph mesh animation channels
    pub fn morph_mesh_channels(&self) -> MorphMeshAnimationIterator {
        MorphMeshAnimationIterator {
            animation_ptr: self.animation_ptr,
            index: 0,
        }
    }
}

/// Animation data for a single node
pub struct NodeAnimation {
    channel_ptr: *const sys::aiNodeAnim,
}

impl NodeAnimation {
    /// Create a NodeAnimation from a raw Assimp node animation pointer
    pub(crate) fn from_raw(channel_ptr: *const sys::aiNodeAnim) -> Self {
        Self { channel_ptr }
    }

    /// Get the raw channel pointer
    pub fn as_raw(&self) -> *const sys::aiNodeAnim {
        self.channel_ptr
    }

    /// Get the name of the node this animation affects
    pub fn node_name(&self) -> String {
        unsafe {
            let channel = &*self.channel_ptr;
            c_str_to_string_or_empty(channel.mNodeName.data.as_ptr())
        }
    }

    /// Get the number of position keyframes
    pub fn num_position_keys(&self) -> usize {
        unsafe { (*self.channel_ptr).mNumPositionKeys as usize }
    }

    /// Get the position keyframes
    pub fn position_keys(&self) -> Vec<VectorKey> {
        unsafe {
            let ch = &*self.channel_ptr;
            std::slice::from_raw_parts(ch.mPositionKeys, ch.mNumPositionKeys as usize)
                .iter()
                .map(|k| VectorKey::from_sys(*k))
                .collect()
        }
    }

    /// Get the number of rotation keyframes
    pub fn num_rotation_keys(&self) -> usize {
        unsafe { (*self.channel_ptr).mNumRotationKeys as usize }
    }

    /// Get the rotation keyframes
    pub fn rotation_keys(&self) -> Vec<QuaternionKey> {
        unsafe {
            let ch = &*self.channel_ptr;
            std::slice::from_raw_parts(ch.mRotationKeys, ch.mNumRotationKeys as usize)
                .iter()
                .map(|k| QuaternionKey::from_sys(*k))
                .collect()
        }
    }

    /// Get the number of scaling keyframes
    pub fn num_scaling_keys(&self) -> usize {
        unsafe { (*self.channel_ptr).mNumScalingKeys as usize }
    }

    /// Get the scaling keyframes
    pub fn scaling_keys(&self) -> Vec<VectorKey> {
        unsafe {
            let ch = &*self.channel_ptr;
            std::slice::from_raw_parts(ch.mScalingKeys, ch.mNumScalingKeys as usize)
                .iter()
                .map(|k| VectorKey::from_sys(*k))
                .collect()
        }
    }
    /// Behaviour before the first key
    pub fn pre_state(&self) -> AnimBehaviour {
        unsafe { AnimBehaviour::from_sys((*self.channel_ptr).mPreState) }
    }
    /// Behaviour after the last key
    pub fn post_state(&self) -> AnimBehaviour {
        unsafe { AnimBehaviour::from_sys((*self.channel_ptr).mPostState) }
    }
}

/// Interpolation method for animation keys
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimInterpolation {
    /// Step interpolation - no interpolation, use the value of the previous key
    Step,
    /// Linear interpolation between keys
    Linear,
    /// Spherical linear interpolation (for quaternions)
    SphericalLinear,
    /// Cubic spline interpolation
    CubicSpline,
    /// Unknown interpolation method with raw value
    Unknown(u32),
}

impl AnimInterpolation {
    fn from_sys(v: sys::aiAnimInterpolation) -> Self {
        match v {
            sys::aiAnimInterpolation::aiAnimInterpolation_Step => Self::Step,
            sys::aiAnimInterpolation::aiAnimInterpolation_Linear => Self::Linear,
            sys::aiAnimInterpolation::aiAnimInterpolation_Spherical_Linear => Self::SphericalLinear,
            sys::aiAnimInterpolation::aiAnimInterpolation_Cubic_Spline => Self::CubicSpline,
            _ => Self::Unknown(v as u32),
        }
    }
}

/// Behaviour outside key range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimBehaviour {
    /// Use the default behavior (usually constant)
    Default,
    /// Keep the value constant at the boundary
    Constant,
    /// Linear extrapolation beyond the key range
    Linear,
    /// Repeat the animation cyclically
    Repeat,
}

impl AnimBehaviour {
    fn from_sys(v: sys::aiAnimBehaviour) -> Self {
        match v {
            sys::aiAnimBehaviour::aiAnimBehaviour_DEFAULT => Self::Default,
            sys::aiAnimBehaviour::aiAnimBehaviour_CONSTANT => Self::Constant,
            sys::aiAnimBehaviour::aiAnimBehaviour_LINEAR => Self::Linear,
            sys::aiAnimBehaviour::aiAnimBehaviour_REPEAT => Self::Repeat,
            _ => Self::Default,
        }
    }
}

/// A keyframe containing a time and a 3D vector value
pub struct VectorKey {
    /// Time of the keyframe
    pub time: f64,
    /// Vector value at this time
    pub value: Vector3D,
    /// Interpolation method
    pub interpolation: AnimInterpolation,
}

impl VectorKey {
    fn from_sys(k: sys::aiVectorKey) -> Self {
        Self {
            time: k.mTime,
            value: Vector3D::new(k.mValue.x, k.mValue.y, k.mValue.z),
            interpolation: AnimInterpolation::from_sys(k.mInterpolation),
        }
    }
}

/// A keyframe containing a time and a quaternion value
pub struct QuaternionKey {
    /// Time of the keyframe
    pub time: f64,
    /// Quaternion value at this time
    pub value: Quaternion,
    /// Interpolation method
    pub interpolation: AnimInterpolation,
}

impl QuaternionKey {
    fn from_sys(k: sys::aiQuatKey) -> Self {
        Self {
            time: k.mTime,
            value: Quaternion::from_xyzw(k.mValue.x, k.mValue.y, k.mValue.z, k.mValue.w),
            interpolation: AnimInterpolation::from_sys(k.mInterpolation),
        }
    }
}

/// Iterator over node animation channels
pub struct NodeAnimationIterator {
    animation_ptr: *const sys::aiAnimation,
    index: usize,
}

impl Iterator for NodeAnimationIterator {
    type Item = NodeAnimation;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let animation = &*self.animation_ptr;
            if self.index >= animation.mNumChannels as usize {
                None
            } else {
                let channel_ptr = *animation.mChannels.add(self.index);
                self.index += 1;
                if channel_ptr.is_null() {
                    None
                } else {
                    Some(NodeAnimation::from_raw(channel_ptr))
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let animation = &*self.animation_ptr;
            let remaining = (animation.mNumChannels as usize).saturating_sub(self.index);
            (remaining, Some(remaining))
        }
    }
}

impl ExactSizeIterator for NodeAnimationIterator {}

/// Mesh animation key
#[repr(C)]
pub struct MeshKey {
    /// Time of this key in the animation
    pub time: f64,
    /// Index into aiMesh::mAnimMeshes array
    pub value: u32, // index into aiMesh::mAnimMeshes
}

/// Mesh animation of a specific mesh (aiMeshAnim)
pub struct MeshAnimation {
    channel_ptr: *const sys::aiMeshAnim,
}

impl MeshAnimation {
    /// Get the name of this mesh animation channel
    pub fn name(&self) -> String {
        unsafe {
            let ch = &*self.channel_ptr;
            c_str_to_string_or_empty(ch.mName.data.as_ptr())
        }
    }

    /// Get the number of animation keys
    pub fn num_keys(&self) -> usize {
        unsafe { (*self.channel_ptr).mNumKeys as usize }
    }

    /// Get the array of animation keys
    pub fn keys(&self) -> &[MeshKey] {
        unsafe {
            let ch = &*self.channel_ptr;
            std::slice::from_raw_parts(ch.mKeys as *const MeshKey, ch.mNumKeys as usize)
        }
    }
}

/// Iterator over mesh animation channels
pub struct MeshAnimationIterator {
    animation_ptr: *const sys::aiAnimation,
    index: usize,
}

impl Iterator for MeshAnimationIterator {
    type Item = MeshAnimation;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let anim = &*self.animation_ptr;
            if self.index >= anim.mNumMeshChannels as usize {
                None
            } else {
                let ptr = *anim.mMeshChannels.add(self.index);
                self.index += 1;
                if ptr.is_null() {
                    None
                } else {
                    Some(MeshAnimation { channel_ptr: ptr })
                }
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let anim = &*self.animation_ptr;
            let remaining = (anim.mNumMeshChannels as usize).saturating_sub(self.index);
            (remaining, Some(remaining))
        }
    }
}

impl ExactSizeIterator for MeshAnimationIterator {}

/// Morph mesh key (weights for multiple targets)
pub struct MorphMeshKey<'a> {
    /// Time of this key in the animation
    pub time: f64,
    /// Indices of the morph targets
    pub values: &'a [u32],
    /// Weights for each morph target
    pub weights: &'a [f64],
}

/// Morph mesh animation channel (aiMeshMorphAnim)
pub struct MorphMeshAnimation {
    channel_ptr: *const sys::aiMeshMorphAnim,
}

impl MorphMeshAnimation {
    /// Get the name of this morph mesh animation channel
    pub fn name(&self) -> String {
        unsafe { c_str_to_string_or_empty((*self.channel_ptr).mName.data.as_ptr()) }
    }

    /// Get the number of animation keys
    pub fn num_keys(&self) -> usize {
        unsafe { (*self.channel_ptr).mNumKeys as usize }
    }

    /// Get a specific animation key by index
    pub fn key(&self, index: usize) -> Option<MorphMeshKey<'_>> {
        if index >= self.num_keys() {
            return None;
        }
        unsafe {
            let ch = &*self.channel_ptr;
            let key = &*ch.mKeys.add(index);
            let n = key.mNumValuesAndWeights as usize;
            if key.mValues.is_null() || key.mWeights.is_null() {
                return None;
            }
            let values = std::slice::from_raw_parts(key.mValues, n);
            let weights = std::slice::from_raw_parts(key.mWeights, n);
            Some(MorphMeshKey {
                time: key.mTime,
                values,
                weights,
            })
        }
    }
}

/// Iterator over morph mesh animation channels
pub struct MorphMeshAnimationIterator {
    animation_ptr: *const sys::aiAnimation,
    index: usize,
}

impl Iterator for MorphMeshAnimationIterator {
    type Item = MorphMeshAnimation;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let anim = &*self.animation_ptr;
            if self.index >= anim.mNumMorphMeshChannels as usize {
                None
            } else {
                let ptr = *anim.mMorphMeshChannels.add(self.index);
                self.index += 1;
                if ptr.is_null() {
                    None
                } else {
                    Some(MorphMeshAnimation { channel_ptr: ptr })
                }
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let anim = &*self.animation_ptr;
            let remaining = (anim.mNumMorphMeshChannels as usize).saturating_sub(self.index);
            (remaining, Some(remaining))
        }
    }
}

impl ExactSizeIterator for MorphMeshAnimationIterator {}
