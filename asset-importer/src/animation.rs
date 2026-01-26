//! Animation data structures and utilities

use crate::{
    ffi,
    ptr::SharedPtr,
    raw,
    scene::Scene,
    sys,
    types::{Quaternion, Vector3D, ai_string_to_string},
};

/// An animation containing keyframes for various properties
#[derive(Clone)]
pub struct Animation {
    scene: Scene,
    animation_ptr: SharedPtr<sys::aiAnimation>,
}

impl Animation {
    pub(crate) fn from_sys_ptr(scene: Scene, animation_ptr: *mut sys::aiAnimation) -> Option<Self> {
        let animation_ptr = SharedPtr::new(animation_ptr as *const sys::aiAnimation)?;
        Some(Self {
            scene,
            animation_ptr,
        })
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

    #[inline]
    fn raw(&self) -> &sys::aiAnimation {
        self.animation_ptr.as_ref()
    }

    #[inline]
    fn channel_ptr(&self, index: usize) -> Option<*const sys::aiNodeAnim> {
        let anim = self.raw();
        ffi::ptr_array_get(self, anim.mChannels, anim.mNumChannels as usize, index)
            .map(|p| p as *const sys::aiNodeAnim)
    }

    #[inline]
    fn mesh_channel_ptr(&self, index: usize) -> Option<*const sys::aiMeshAnim> {
        let anim = self.raw();
        ffi::ptr_array_get(
            self,
            anim.mMeshChannels,
            anim.mNumMeshChannels as usize,
            index,
        )
        .map(|p| p as *const sys::aiMeshAnim)
    }

    #[inline]
    fn morph_mesh_channel_ptr(&self, index: usize) -> Option<*const sys::aiMeshMorphAnim> {
        let anim = self.raw();
        ffi::ptr_array_get(
            self,
            anim.mMorphMeshChannels,
            anim.mNumMorphMeshChannels as usize,
            index,
        )
        .map(|p| p as *const sys::aiMeshMorphAnim)
    }

    /// Get the name of the animation
    pub fn name(&self) -> String {
        ai_string_to_string(&self.raw().mName)
    }

    /// Get the duration of the animation in ticks
    pub fn duration(&self) -> f64 {
        self.raw().mDuration
    }

    /// Get the ticks per second for this animation
    pub fn ticks_per_second(&self) -> f64 {
        let tps = self.raw().mTicksPerSecond;
        if tps != 0.0 { tps } else { 25.0 } // Default to 25 FPS
    }

    /// Get the duration in seconds
    pub fn duration_in_seconds(&self) -> f64 {
        self.duration() / self.ticks_per_second()
    }

    /// Get the number of node animation channels
    pub fn num_channels(&self) -> usize {
        let anim = self.raw();
        if anim.mChannels.is_null() {
            0
        } else {
            anim.mNumChannels as usize
        }
    }

    /// Get a node animation channel by index
    pub fn channel(&self, index: usize) -> Option<NodeAnimation> {
        if index >= self.num_channels() {
            return None;
        }

        let channel_ptr = self.channel_ptr(index)?;
        NodeAnimation::from_ptr(self.scene.clone(), channel_ptr)
    }

    /// Get an iterator over all node animation channels
    pub fn channels(&self) -> NodeAnimationIterator {
        NodeAnimationIterator {
            scene: self.scene.clone(),
            animation_ptr: self.animation_ptr,
            index: 0,
        }
    }

    /// Get the number of mesh animation channels (vertex anim via aiAnimMesh)
    pub fn num_mesh_channels(&self) -> usize {
        let anim = self.raw();
        if anim.mMeshChannels.is_null() {
            0
        } else {
            anim.mNumMeshChannels as usize
        }
    }

    /// Get a mesh animation channel
    pub fn mesh_channel(&self, index: usize) -> Option<MeshAnimation> {
        if index >= self.num_mesh_channels() {
            return None;
        }
        let ptr = self.mesh_channel_ptr(index)?;
        MeshAnimation::from_ptr(self.scene.clone(), ptr)
    }

    /// Iterate mesh animation channels
    pub fn mesh_channels(&self) -> MeshAnimationIterator {
        MeshAnimationIterator {
            scene: self.scene.clone(),
            animation_ptr: self.animation_ptr,
            index: 0,
        }
    }

    /// Get the number of morph mesh animation channels
    pub fn num_morph_mesh_channels(&self) -> usize {
        let anim = self.raw();
        if anim.mMorphMeshChannels.is_null() {
            0
        } else {
            anim.mNumMorphMeshChannels as usize
        }
    }

    /// Get a morph mesh animation channel
    pub fn morph_mesh_channel(&self, index: usize) -> Option<MorphMeshAnimation> {
        if index >= self.num_morph_mesh_channels() {
            return None;
        }
        let ptr = self.morph_mesh_channel_ptr(index)?;
        MorphMeshAnimation::from_ptr(self.scene.clone(), ptr)
    }

    /// Iterate morph mesh animation channels
    pub fn morph_mesh_channels(&self) -> MorphMeshAnimationIterator {
        MorphMeshAnimationIterator {
            scene: self.scene.clone(),
            animation_ptr: self.animation_ptr,
            index: 0,
        }
    }
}

/// Animation data for a single node
#[derive(Clone)]
pub struct NodeAnimation {
    #[allow(dead_code)]
    scene: Scene,
    channel_ptr: SharedPtr<sys::aiNodeAnim>,
}

impl NodeAnimation {
    pub(crate) fn from_ptr(scene: Scene, channel_ptr: *const sys::aiNodeAnim) -> Option<Self> {
        let channel_ptr = SharedPtr::new(channel_ptr)?;
        Some(Self { scene, channel_ptr })
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

    #[inline]
    fn raw(&self) -> &sys::aiNodeAnim {
        self.channel_ptr.as_ref()
    }

    /// Get the name of the node this animation affects
    pub fn node_name(&self) -> String {
        ai_string_to_string(&self.raw().mNodeName)
    }

    /// Get the number of position keyframes
    pub fn num_position_keys(&self) -> usize {
        let ch = self.raw();
        if ch.mPositionKeys.is_null() {
            0
        } else {
            ch.mNumPositionKeys as usize
        }
    }

    /// Get the raw position keyframes (zero-copy).
    pub fn position_keys_raw(&self) -> &[raw::AiVectorKey] {
        let ch = self.raw();
        let n = ch.mNumPositionKeys as usize;
        debug_assert!(n == 0 || !ch.mPositionKeys.is_null());
        ffi::slice_from_ptr_len(self, ch.mPositionKeys as *const raw::AiVectorKey, n)
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
        let ch = self.raw();
        if ch.mRotationKeys.is_null() {
            0
        } else {
            ch.mNumRotationKeys as usize
        }
    }

    /// Get the raw rotation keyframes (zero-copy).
    pub fn rotation_keys_raw(&self) -> &[raw::AiQuatKey] {
        let ch = self.raw();
        let n = ch.mNumRotationKeys as usize;
        debug_assert!(n == 0 || !ch.mRotationKeys.is_null());
        ffi::slice_from_ptr_len(self, ch.mRotationKeys as *const raw::AiQuatKey, n)
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
        let ch = self.raw();
        if ch.mScalingKeys.is_null() {
            0
        } else {
            ch.mNumScalingKeys as usize
        }
    }

    /// Get the raw scaling keyframes (zero-copy).
    pub fn scaling_keys_raw(&self) -> &[raw::AiVectorKey] {
        let ch = self.raw();
        let n = ch.mNumScalingKeys as usize;
        debug_assert!(n == 0 || !ch.mScalingKeys.is_null());
        ffi::slice_from_ptr_len(self, ch.mScalingKeys as *const raw::AiVectorKey, n)
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
        AnimBehaviour::from_sys(self.raw().mPreState)
    }
    /// Behaviour after the last key
    pub fn post_state(&self) -> AnimBehaviour {
        AnimBehaviour::from_sys(self.raw().mPostState)
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
pub struct NodeAnimationIterator {
    scene: Scene,
    animation_ptr: SharedPtr<sys::aiAnimation>,
    index: usize,
}

impl NodeAnimationIterator {
    #[inline]
    fn raw(&self) -> &sys::aiAnimation {
        self.animation_ptr.as_ref()
    }

    #[inline]
    fn channel_array(&self) -> (*mut *mut sys::aiNodeAnim, usize) {
        let animation = self.raw();
        (animation.mChannels, animation.mNumChannels as usize)
    }
}

impl Iterator for NodeAnimationIterator {
    type Item = NodeAnimation;

    fn next(&mut self) -> Option<Self::Item> {
        let (base, len) = self.channel_array();
        let channels: &[*mut sys::aiNodeAnim] =
            ffi::slice_from_ptr_len_opt(&(), base as *const *mut sys::aiNodeAnim, len)?;
        while self.index < channels.len() {
            let channel_ptr = channels[self.index];
            self.index += 1;
            if channel_ptr.is_null() {
                continue;
            }
            return NodeAnimation::from_ptr(
                self.scene.clone(),
                channel_ptr as *const sys::aiNodeAnim,
            );
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let animation = self.raw();
        if animation.mChannels.is_null() {
            (0, Some(0))
        } else {
            let remaining = (animation.mNumChannels as usize).saturating_sub(self.index);
            (0, Some(remaining))
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
#[derive(Clone)]
pub struct MeshAnimation {
    #[allow(dead_code)]
    scene: Scene,
    channel_ptr: SharedPtr<sys::aiMeshAnim>,
}

impl MeshAnimation {
    fn from_ptr(scene: Scene, channel_ptr: *const sys::aiMeshAnim) -> Option<Self> {
        let channel_ptr = SharedPtr::new(channel_ptr)?;
        Some(Self { scene, channel_ptr })
    }

    #[inline]
    fn raw(&self) -> &sys::aiMeshAnim {
        self.channel_ptr.as_ref()
    }

    /// Get the name of this mesh animation channel
    pub fn name(&self) -> String {
        ai_string_to_string(&self.raw().mName)
    }

    /// Get the number of animation keys
    pub fn num_keys(&self) -> usize {
        let ch = self.raw();
        if ch.mKeys.is_null() {
            0
        } else {
            ch.mNumKeys as usize
        }
    }

    /// Get the array of animation keys
    pub fn keys(&self) -> &[MeshKey] {
        let ch = self.raw();
        let n = ch.mNumKeys as usize;
        debug_assert!(n == 0 || !ch.mKeys.is_null());
        ffi::slice_from_ptr_len(self, ch.mKeys as *const MeshKey, n)
    }
}

/// Iterator over mesh animation channels
pub struct MeshAnimationIterator {
    scene: Scene,
    animation_ptr: SharedPtr<sys::aiAnimation>,
    index: usize,
}

impl MeshAnimationIterator {
    #[inline]
    fn raw(&self) -> &sys::aiAnimation {
        self.animation_ptr.as_ref()
    }

    #[inline]
    fn channel_array(&self) -> (*mut *mut sys::aiMeshAnim, usize) {
        let anim = self.raw();
        (anim.mMeshChannels, anim.mNumMeshChannels as usize)
    }
}

impl Iterator for MeshAnimationIterator {
    type Item = MeshAnimation;
    fn next(&mut self) -> Option<Self::Item> {
        let (base, len) = self.channel_array();
        let chans: &[*mut sys::aiMeshAnim] =
            ffi::slice_from_ptr_len_opt(&(), base as *const *mut sys::aiMeshAnim, len)?;
        while self.index < chans.len() {
            let ptr = chans[self.index];
            self.index += 1;
            if ptr.is_null() {
                continue;
            }
            return MeshAnimation::from_ptr(self.scene.clone(), ptr as *const sys::aiMeshAnim);
        }
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let anim = self.raw();
        if anim.mMeshChannels.is_null() {
            (0, Some(0))
        } else {
            let remaining = (anim.mNumMeshChannels as usize).saturating_sub(self.index);
            (0, Some(remaining))
        }
    }
}

/// Morph mesh key (weights for multiple targets)
#[derive(Clone)]
pub struct MorphMeshKey {
    #[allow(dead_code)]
    scene: Scene,
    key_ptr: SharedPtr<sys::aiMeshMorphKey>,
}

impl MorphMeshKey {
    #[inline]
    fn raw(&self) -> &sys::aiMeshMorphKey {
        self.key_ptr.as_ref()
    }

    /// Key time in ticks.
    pub fn time(&self) -> f64 {
        self.raw().mTime
    }

    /// Number of values/weights in this key.
    pub fn len(&self) -> usize {
        self.raw().mNumValuesAndWeights as usize
    }

    /// Returns `true` if this key has no values/weights.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Morph target indices (zero-copy).
    pub fn values(&self) -> &[u32] {
        let k = self.raw();
        let n = k.mNumValuesAndWeights as usize;
        debug_assert!(n == 0 || !k.mValues.is_null());
        ffi::slice_from_ptr_len(self, k.mValues as *const u32, n)
    }

    /// Morph target weights (zero-copy).
    pub fn weights(&self) -> &[f64] {
        let k = self.raw();
        let n = k.mNumValuesAndWeights as usize;
        debug_assert!(n == 0 || !k.mWeights.is_null());
        ffi::slice_from_ptr_len(self, k.mWeights as *const f64, n)
    }
}

/// Morph mesh animation channel (aiMeshMorphAnim)
#[derive(Clone)]
pub struct MorphMeshAnimation {
    scene: Scene,
    channel_ptr: SharedPtr<sys::aiMeshMorphAnim>,
}

impl MorphMeshAnimation {
    fn from_ptr(scene: Scene, channel_ptr: *const sys::aiMeshMorphAnim) -> Option<Self> {
        let channel_ptr = SharedPtr::new(channel_ptr)?;
        Some(Self { scene, channel_ptr })
    }

    #[inline]
    fn raw(&self) -> &sys::aiMeshMorphAnim {
        self.channel_ptr.as_ref()
    }

    #[inline]
    fn keys_raw(&self) -> Option<&[sys::aiMeshMorphKey]> {
        let ch = self.raw();
        ffi::slice_from_ptr_len_opt(ch, ch.mKeys, ch.mNumKeys as usize)
    }

    /// Get the name of this morph mesh animation channel
    pub fn name(&self) -> String {
        ai_string_to_string(&self.raw().mName)
    }

    /// Get the number of animation keys
    pub fn num_keys(&self) -> usize {
        let ch = self.raw();
        if ch.mKeys.is_null() {
            0
        } else {
            ch.mNumKeys as usize
        }
    }

    /// Get a specific animation key by index
    pub fn key(&self, index: usize) -> Option<MorphMeshKey> {
        if index >= self.num_keys() {
            return None;
        }
        let keys = self.keys_raw()?;
        let key_ref = keys.get(index)?;
        let n = key_ref.mNumValuesAndWeights as usize;
        if n > 0 && (key_ref.mValues.is_null() || key_ref.mWeights.is_null()) {
            return None;
        }
        let key_ptr = SharedPtr::new(std::ptr::from_ref(key_ref))?;
        Some(MorphMeshKey {
            scene: self.scene.clone(),
            key_ptr,
        })
    }
}

/// Iterator over morph mesh animation channels
pub struct MorphMeshAnimationIterator {
    scene: Scene,
    animation_ptr: SharedPtr<sys::aiAnimation>,
    index: usize,
}

impl MorphMeshAnimationIterator {
    #[inline]
    fn raw(&self) -> &sys::aiAnimation {
        self.animation_ptr.as_ref()
    }

    #[inline]
    fn channel_array(&self) -> (*mut *mut sys::aiMeshMorphAnim, usize) {
        let anim = self.raw();
        (anim.mMorphMeshChannels, anim.mNumMorphMeshChannels as usize)
    }
}

impl Iterator for MorphMeshAnimationIterator {
    type Item = MorphMeshAnimation;
    fn next(&mut self) -> Option<Self::Item> {
        let (base, len) = self.channel_array();
        let chans: &[*mut sys::aiMeshMorphAnim] =
            ffi::slice_from_ptr_len_opt(&(), base as *const *mut sys::aiMeshMorphAnim, len)?;
        while self.index < chans.len() {
            let ptr = chans[self.index];
            self.index += 1;
            if ptr.is_null() {
                continue;
            }
            return MorphMeshAnimation::from_ptr(
                self.scene.clone(),
                ptr as *const sys::aiMeshMorphAnim,
            );
        }
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let anim = self.raw();
        if anim.mMorphMeshChannels.is_null() {
            (0, Some(0))
        } else {
            let remaining = (anim.mNumMorphMeshChannels as usize).saturating_sub(self.index);
            (0, Some(remaining))
        }
    }
}

// Auto-traits (Send/Sync) are derived from the contained pointers and lifetimes.

#[cfg(test)]
mod layout_tests {
    use super::MeshKey;
    use crate::sys;

    #[test]
    fn test_mesh_key_layout_matches_sys() {
        assert_eq!(
            std::mem::size_of::<MeshKey>(),
            std::mem::size_of::<sys::aiMeshKey>()
        );
        assert_eq!(
            std::mem::align_of::<MeshKey>(),
            std::mem::align_of::<sys::aiMeshKey>()
        );
    }
}
