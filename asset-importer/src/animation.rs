//! Animation data structures and utilities

use std::marker::PhantomData;

use crate::{
    ptr::SharedPtr,
    raw, sys,
    types::{Quaternion, Vector3D, ai_string_to_string},
};

/// An animation containing keyframes for various properties
pub struct Animation<'a> {
    animation_ptr: SharedPtr<sys::aiAnimation>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Animation<'a> {
    /// Create an Animation from a raw Assimp animation pointer
    ///
    /// # Safety
    /// Caller must ensure `animation_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(animation_ptr: *const sys::aiAnimation) -> Self {
        debug_assert!(!animation_ptr.is_null());
        let animation_ptr = unsafe { SharedPtr::new_unchecked(animation_ptr) };
        Self {
            animation_ptr,
            _marker: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw_sys(&self) -> *const sys::aiAnimation {
        self.animation_ptr.as_ptr()
    }

    /// Get the raw animation pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> *const sys::aiAnimation {
        self.as_raw_sys()
    }

    /// Get the name of the animation
    pub fn name(&self) -> String {
        unsafe { ai_string_to_string(&(*self.animation_ptr.as_ptr()).mName) }
    }

    /// Get the duration of the animation in ticks
    pub fn duration(&self) -> f64 {
        unsafe { (*self.animation_ptr.as_ptr()).mDuration }
    }

    /// Get the ticks per second for this animation
    pub fn ticks_per_second(&self) -> f64 {
        unsafe {
            let tps = (*self.animation_ptr.as_ptr()).mTicksPerSecond;
            if tps != 0.0 { tps } else { 25.0 } // Default to 25 FPS
        }
    }

    /// Get the duration in seconds
    pub fn duration_in_seconds(&self) -> f64 {
        self.duration() / self.ticks_per_second()
    }

    /// Get the number of node animation channels
    pub fn num_channels(&self) -> usize {
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            if anim.mChannels.is_null() {
                0
            } else {
                anim.mNumChannels as usize
            }
        }
    }

    /// Get a node animation channel by index
    pub fn channel(&self, index: usize) -> Option<NodeAnimation<'a>> {
        if index >= self.num_channels() {
            return None;
        }

        unsafe {
            let animation = &*self.animation_ptr.as_ptr();
            if animation.mChannels.is_null() {
                return None;
            }
            let channel_ptr = *animation.mChannels.add(index);
            if channel_ptr.is_null() {
                None
            } else {
                Some(NodeAnimation::from_raw(channel_ptr))
            }
        }
    }

    /// Get an iterator over all node animation channels
    pub fn channels(&self) -> NodeAnimationIterator<'a> {
        NodeAnimationIterator {
            animation_ptr: self.animation_ptr,
            index: 0,
            _marker: PhantomData,
        }
    }

    /// Get the number of mesh animation channels (vertex anim via aiAnimMesh)
    pub fn num_mesh_channels(&self) -> usize {
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            if anim.mMeshChannels.is_null() {
                0
            } else {
                anim.mNumMeshChannels as usize
            }
        }
    }

    /// Get a mesh animation channel
    pub fn mesh_channel(&self, index: usize) -> Option<MeshAnimation<'a>> {
        if index >= self.num_mesh_channels() {
            return None;
        }
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            if anim.mMeshChannels.is_null() {
                return None;
            }
            let ptr = *anim.mMeshChannels.add(index);
            if ptr.is_null() {
                None
            } else {
                Some(MeshAnimation::from_raw(ptr))
            }
        }
    }

    /// Iterate mesh animation channels
    pub fn mesh_channels(&self) -> MeshAnimationIterator<'a> {
        MeshAnimationIterator {
            animation_ptr: self.animation_ptr,
            index: 0,
            _marker: PhantomData,
        }
    }

    /// Get the number of morph mesh animation channels
    pub fn num_morph_mesh_channels(&self) -> usize {
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            if anim.mMorphMeshChannels.is_null() {
                0
            } else {
                anim.mNumMorphMeshChannels as usize
            }
        }
    }

    /// Get a morph mesh animation channel
    pub fn morph_mesh_channel(&self, index: usize) -> Option<MorphMeshAnimation<'a>> {
        if index >= self.num_morph_mesh_channels() {
            return None;
        }
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            if anim.mMorphMeshChannels.is_null() {
                return None;
            }
            let ptr = *anim.mMorphMeshChannels.add(index);
            if ptr.is_null() {
                None
            } else {
                Some(MorphMeshAnimation::from_raw(ptr))
            }
        }
    }

    /// Iterate morph mesh animation channels
    pub fn morph_mesh_channels(&self) -> MorphMeshAnimationIterator<'a> {
        MorphMeshAnimationIterator {
            animation_ptr: self.animation_ptr,
            index: 0,
            _marker: PhantomData,
        }
    }
}

/// Animation data for a single node
pub struct NodeAnimation<'a> {
    channel_ptr: SharedPtr<sys::aiNodeAnim>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> NodeAnimation<'a> {
    /// Create a NodeAnimation from a raw Assimp node animation pointer
    ///
    /// # Safety
    /// Caller must ensure `channel_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(channel_ptr: *const sys::aiNodeAnim) -> Self {
        debug_assert!(!channel_ptr.is_null());
        let channel_ptr = unsafe { SharedPtr::new_unchecked(channel_ptr) };
        Self {
            channel_ptr,
            _marker: PhantomData,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn as_raw_sys(&self) -> *const sys::aiNodeAnim {
        self.channel_ptr.as_ptr()
    }

    /// Get the raw channel pointer (requires `raw-sys`).
    #[cfg(feature = "raw-sys")]
    pub fn as_raw(&self) -> *const sys::aiNodeAnim {
        self.as_raw_sys()
    }

    /// Get the name of the node this animation affects
    pub fn node_name(&self) -> String {
        unsafe { ai_string_to_string(&(*self.channel_ptr.as_ptr()).mNodeName) }
    }

    /// Get the number of position keyframes
    pub fn num_position_keys(&self) -> usize {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mPositionKeys.is_null() {
                0
            } else {
                ch.mNumPositionKeys as usize
            }
        }
    }

    /// Get the raw position keyframes (zero-copy).
    pub fn position_keys_raw(&self) -> &'a [raw::AiVectorKey] {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mPositionKeys.is_null() || ch.mNumPositionKeys == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(
                    ch.mPositionKeys as *const raw::AiVectorKey,
                    ch.mNumPositionKeys as usize,
                )
            }
        }
    }

    /// Iterate position keyframes without allocation.
    pub fn position_keys_iter(&self) -> impl Iterator<Item = VectorKey> + '_ {
        self.position_keys_raw()
            .iter()
            .copied()
            .map(VectorKey::from_raw)
    }

    /// Get the position keyframes (allocates).
    pub fn position_keys(&self) -> Vec<VectorKey> {
        self.position_keys_iter().collect()
    }

    /// Get the number of rotation keyframes
    pub fn num_rotation_keys(&self) -> usize {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mRotationKeys.is_null() {
                0
            } else {
                ch.mNumRotationKeys as usize
            }
        }
    }

    /// Get the raw rotation keyframes (zero-copy).
    pub fn rotation_keys_raw(&self) -> &'a [raw::AiQuatKey] {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mRotationKeys.is_null() || ch.mNumRotationKeys == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(
                    ch.mRotationKeys as *const raw::AiQuatKey,
                    ch.mNumRotationKeys as usize,
                )
            }
        }
    }

    /// Iterate rotation keyframes without allocation.
    pub fn rotation_keys_iter(&self) -> impl Iterator<Item = QuaternionKey> + '_ {
        self.rotation_keys_raw()
            .iter()
            .copied()
            .map(QuaternionKey::from_raw)
    }

    /// Get the rotation keyframes (allocates).
    pub fn rotation_keys(&self) -> Vec<QuaternionKey> {
        self.rotation_keys_iter().collect()
    }

    /// Get the number of scaling keyframes
    pub fn num_scaling_keys(&self) -> usize {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mScalingKeys.is_null() {
                0
            } else {
                ch.mNumScalingKeys as usize
            }
        }
    }

    /// Get the raw scaling keyframes (zero-copy).
    pub fn scaling_keys_raw(&self) -> &'a [raw::AiVectorKey] {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mScalingKeys.is_null() || ch.mNumScalingKeys == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(
                    ch.mScalingKeys as *const raw::AiVectorKey,
                    ch.mNumScalingKeys as usize,
                )
            }
        }
    }

    /// Iterate scaling keyframes without allocation.
    pub fn scaling_keys_iter(&self) -> impl Iterator<Item = VectorKey> + '_ {
        self.scaling_keys_raw()
            .iter()
            .copied()
            .map(VectorKey::from_raw)
    }

    /// Get the scaling keyframes (allocates).
    pub fn scaling_keys(&self) -> Vec<VectorKey> {
        self.scaling_keys_iter().collect()
    }
    /// Behaviour before the first key
    pub fn pre_state(&self) -> AnimBehaviour {
        unsafe { AnimBehaviour::from_sys((*self.channel_ptr.as_ptr()).mPreState) }
    }
    /// Behaviour after the last key
    pub fn post_state(&self) -> AnimBehaviour {
        unsafe { AnimBehaviour::from_sys((*self.channel_ptr.as_ptr()).mPostState) }
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
    fn from_raw(v: i32) -> Self {
        match v as u32 {
            x if x == sys::aiAnimInterpolation::aiAnimInterpolation_Step as u32 => Self::Step,
            x if x == sys::aiAnimInterpolation::aiAnimInterpolation_Linear as u32 => Self::Linear,
            x if x == sys::aiAnimInterpolation::aiAnimInterpolation_Spherical_Linear as u32 => {
                Self::SphericalLinear
            }
            x if x == sys::aiAnimInterpolation::aiAnimInterpolation_Cubic_Spline as u32 => {
                Self::CubicSpline
            }
            other => Self::Unknown(other),
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
    fn from_raw(k: raw::AiVectorKey) -> Self {
        Self {
            time: k.mTime,
            value: Vector3D::new(k.mValue.x, k.mValue.y, k.mValue.z),
            interpolation: AnimInterpolation::from_raw(k.mInterpolation),
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
    fn from_raw(k: raw::AiQuatKey) -> Self {
        Self {
            time: k.mTime,
            value: Quaternion::from_xyzw(k.mValue.x, k.mValue.y, k.mValue.z, k.mValue.w),
            interpolation: AnimInterpolation::from_raw(k.mInterpolation),
        }
    }
}

/// Iterator over node animation channels
pub struct NodeAnimationIterator<'a> {
    animation_ptr: SharedPtr<sys::aiAnimation>,
    index: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Iterator for NodeAnimationIterator<'a> {
    type Item = NodeAnimation<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let animation = &*self.animation_ptr.as_ptr();
            if animation.mChannels.is_null() || animation.mNumChannels == 0 {
                return None;
            }
            while self.index < animation.mNumChannels as usize {
                let channel_ptr = *animation.mChannels.add(self.index);
                self.index += 1;
                if channel_ptr.is_null() {
                    continue;
                }
                return Some(NodeAnimation::from_raw(channel_ptr));
            }
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let animation = &*self.animation_ptr.as_ptr();
            if animation.mChannels.is_null() {
                (0, Some(0))
            } else {
                let remaining = (animation.mNumChannels as usize).saturating_sub(self.index);
                (0, Some(remaining))
            }
        }
    }
}

/// Mesh animation key
#[repr(C)]
pub struct MeshKey {
    /// Time of this key in the animation
    pub time: f64,
    /// Index into aiMesh::mAnimMeshes array
    pub value: u32, // index into aiMesh::mAnimMeshes
}

/// Mesh animation of a specific mesh (aiMeshAnim)
pub struct MeshAnimation<'a> {
    channel_ptr: SharedPtr<sys::aiMeshAnim>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> MeshAnimation<'a> {
    /// # Safety
    /// Caller must ensure `channel_ptr` is non-null and points into a live `aiScene`.
    unsafe fn from_raw(channel_ptr: *const sys::aiMeshAnim) -> Self {
        debug_assert!(!channel_ptr.is_null());
        let channel_ptr = unsafe { SharedPtr::new_unchecked(channel_ptr) };
        Self {
            channel_ptr,
            _marker: PhantomData,
        }
    }

    /// Get the name of this mesh animation channel
    pub fn name(&self) -> String {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            ai_string_to_string(&ch.mName)
        }
    }

    /// Get the number of animation keys
    pub fn num_keys(&self) -> usize {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mKeys.is_null() {
                0
            } else {
                ch.mNumKeys as usize
            }
        }
    }

    /// Get the array of animation keys
    pub fn keys(&self) -> &[MeshKey] {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mKeys.is_null() || ch.mNumKeys == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(ch.mKeys as *const MeshKey, ch.mNumKeys as usize)
            }
        }
    }
}

/// Iterator over mesh animation channels
pub struct MeshAnimationIterator<'a> {
    animation_ptr: SharedPtr<sys::aiAnimation>,
    index: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Iterator for MeshAnimationIterator<'a> {
    type Item = MeshAnimation<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            if anim.mMeshChannels.is_null() || anim.mNumMeshChannels == 0 {
                return None;
            }
            while self.index < anim.mNumMeshChannels as usize {
                let ptr = *anim.mMeshChannels.add(self.index);
                self.index += 1;
                if ptr.is_null() {
                    continue;
                }
                return Some(MeshAnimation::from_raw(ptr));
            }
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            if anim.mMeshChannels.is_null() {
                (0, Some(0))
            } else {
                let remaining = (anim.mNumMeshChannels as usize).saturating_sub(self.index);
                (0, Some(remaining))
            }
        }
    }
}

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
pub struct MorphMeshAnimation<'a> {
    channel_ptr: SharedPtr<sys::aiMeshMorphAnim>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> MorphMeshAnimation<'a> {
    /// # Safety
    /// Caller must ensure `channel_ptr` is non-null and points into a live `aiScene`.
    unsafe fn from_raw(channel_ptr: *const sys::aiMeshMorphAnim) -> Self {
        debug_assert!(!channel_ptr.is_null());
        let channel_ptr = unsafe { SharedPtr::new_unchecked(channel_ptr) };
        Self {
            channel_ptr,
            _marker: PhantomData,
        }
    }

    /// Get the name of this morph mesh animation channel
    pub fn name(&self) -> String {
        unsafe { ai_string_to_string(&(*self.channel_ptr.as_ptr()).mName) }
    }

    /// Get the number of animation keys
    pub fn num_keys(&self) -> usize {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mKeys.is_null() {
                0
            } else {
                ch.mNumKeys as usize
            }
        }
    }

    /// Get a specific animation key by index
    pub fn key(&self, index: usize) -> Option<MorphMeshKey<'_>> {
        if index >= self.num_keys() {
            return None;
        }
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mKeys.is_null() {
                return None;
            }
            let key = &*ch.mKeys.add(index);
            let n = key.mNumValuesAndWeights as usize;
            if n == 0 {
                return Some(MorphMeshKey {
                    time: key.mTime,
                    values: &[],
                    weights: &[],
                });
            }
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

#[cfg(test)]
mod layout_tests {
    use super::MeshKey;
    use crate::sys;

    #[test]
    fn test_mesh_key_layout_matches_sys() {
        assert_eq!(std::mem::size_of::<MeshKey>(), std::mem::size_of::<sys::aiMeshKey>());
        assert_eq!(std::mem::align_of::<MeshKey>(), std::mem::align_of::<sys::aiMeshKey>());
    }
}

/// Iterator over morph mesh animation channels
pub struct MorphMeshAnimationIterator<'a> {
    animation_ptr: SharedPtr<sys::aiAnimation>,
    index: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Iterator for MorphMeshAnimationIterator<'a> {
    type Item = MorphMeshAnimation<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            if anim.mMorphMeshChannels.is_null() || anim.mNumMorphMeshChannels == 0 {
                return None;
            }
            while self.index < anim.mNumMorphMeshChannels as usize {
                let ptr = *anim.mMorphMeshChannels.add(self.index);
                self.index += 1;
                if ptr.is_null() {
                    continue;
                }
                return Some(MorphMeshAnimation::from_raw(ptr));
            }
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            if anim.mMorphMeshChannels.is_null() {
                (0, Some(0))
            } else {
                let remaining = (anim.mNumMorphMeshChannels as usize).saturating_sub(self.index);
                (0, Some(remaining))
            }
        }
    }
}

// Auto-traits (Send/Sync) are derived from the contained pointers and lifetimes.
