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
    /// Create an Animation from a raw Assimp animation pointer
    ///
    /// # Safety
    /// Caller must ensure `animation_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(scene: Scene, animation_ptr: *const sys::aiAnimation) -> Self {
        debug_assert!(!animation_ptr.is_null());
        let animation_ptr = unsafe { SharedPtr::new_unchecked(animation_ptr) };
        Self {
            scene,
            animation_ptr,
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
    pub fn channel(&self, index: usize) -> Option<NodeAnimation> {
        if index >= self.num_channels() {
            return None;
        }

        unsafe {
            let animation = &*self.animation_ptr.as_ptr();
            let channel_ptr = ffi::ptr_array_get(
                self,
                animation.mChannels,
                animation.mNumChannels as usize,
                index,
            )?;
            Some(NodeAnimation::from_raw(
                self.scene.clone(),
                channel_ptr as *const sys::aiNodeAnim,
            ))
        }
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
    pub fn mesh_channel(&self, index: usize) -> Option<MeshAnimation> {
        if index >= self.num_mesh_channels() {
            return None;
        }
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            let ptr = ffi::ptr_array_get(
                self,
                anim.mMeshChannels,
                anim.mNumMeshChannels as usize,
                index,
            )?;
            Some(MeshAnimation::from_raw(
                self.scene.clone(),
                ptr as *const sys::aiMeshAnim,
            ))
        }
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
    pub fn morph_mesh_channel(&self, index: usize) -> Option<MorphMeshAnimation> {
        if index >= self.num_morph_mesh_channels() {
            return None;
        }
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            let ptr = ffi::ptr_array_get(
                self,
                anim.mMorphMeshChannels,
                anim.mNumMorphMeshChannels as usize,
                index,
            )?;
            Some(MorphMeshAnimation::from_raw(
                self.scene.clone(),
                ptr as *const sys::aiMeshMorphAnim,
            ))
        }
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
    /// Create a NodeAnimation from a raw Assimp node animation pointer
    ///
    /// # Safety
    /// Caller must ensure `channel_ptr` is non-null and points into a live `aiScene`.
    pub(crate) unsafe fn from_raw(scene: Scene, channel_ptr: *const sys::aiNodeAnim) -> Self {
        debug_assert!(!channel_ptr.is_null());
        let channel_ptr = unsafe { SharedPtr::new_unchecked(channel_ptr) };
        Self { scene, channel_ptr }
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
    pub fn position_keys_raw(&self) -> &[raw::AiVectorKey] {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mPositionKeys.is_null() || ch.mNumPositionKeys == 0 {
                &[]
            } else {
                ffi::slice_from_ptr_len(
                    self,
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
    pub fn rotation_keys_raw(&self) -> &[raw::AiQuatKey] {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mRotationKeys.is_null() || ch.mNumRotationKeys == 0 {
                &[]
            } else {
                ffi::slice_from_ptr_len(
                    self,
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
    pub fn scaling_keys_raw(&self) -> &[raw::AiVectorKey] {
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            if ch.mScalingKeys.is_null() || ch.mNumScalingKeys == 0 {
                &[]
            } else {
                ffi::slice_from_ptr_len(
                    self,
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
pub struct NodeAnimationIterator {
    scene: Scene,
    animation_ptr: SharedPtr<sys::aiAnimation>,
    index: usize,
}

impl Iterator for NodeAnimationIterator {
    type Item = NodeAnimation;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let animation = &*self.animation_ptr.as_ptr();
            let channels = ffi::slice_from_ptr_len_opt(
                animation,
                animation.mChannels,
                animation.mNumChannels as usize,
            )?;
            while self.index < channels.len() {
                let channel_ptr = channels[self.index];
                self.index += 1;
                if channel_ptr.is_null() {
                    continue;
                }
                return Some(NodeAnimation::from_raw(
                    self.scene.clone(),
                    channel_ptr as *const sys::aiNodeAnim,
                ));
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
#[derive(Clone)]
pub struct MeshAnimation {
    #[allow(dead_code)]
    scene: Scene,
    channel_ptr: SharedPtr<sys::aiMeshAnim>,
}

impl MeshAnimation {
    /// # Safety
    /// Caller must ensure `channel_ptr` is non-null and points into a live `aiScene`.
    unsafe fn from_raw(scene: Scene, channel_ptr: *const sys::aiMeshAnim) -> Self {
        debug_assert!(!channel_ptr.is_null());
        let channel_ptr = unsafe { SharedPtr::new_unchecked(channel_ptr) };
        Self { scene, channel_ptr }
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
            let n = ch.mNumKeys as usize;
            debug_assert!(n == 0 || !ch.mKeys.is_null());
            ffi::slice_from_ptr_len(self, ch.mKeys as *const MeshKey, n)
        }
    }
}

/// Iterator over mesh animation channels
pub struct MeshAnimationIterator {
    scene: Scene,
    animation_ptr: SharedPtr<sys::aiAnimation>,
    index: usize,
}

impl Iterator for MeshAnimationIterator {
    type Item = MeshAnimation;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            let chans = ffi::slice_from_ptr_len_opt(
                anim,
                anim.mMeshChannels,
                anim.mNumMeshChannels as usize,
            )?;
            while self.index < chans.len() {
                let ptr = chans[self.index];
                self.index += 1;
                if ptr.is_null() {
                    continue;
                }
                return Some(MeshAnimation::from_raw(
                    self.scene.clone(),
                    ptr as *const sys::aiMeshAnim,
                ));
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
#[derive(Clone)]
pub struct MorphMeshKey {
    #[allow(dead_code)]
    scene: Scene,
    key_ptr: SharedPtr<sys::aiMeshMorphKey>,
}

impl MorphMeshKey {
    /// Key time in ticks.
    pub fn time(&self) -> f64 {
        unsafe { (*self.key_ptr.as_ptr()).mTime }
    }

    /// Number of values/weights in this key.
    pub fn len(&self) -> usize {
        unsafe { (*self.key_ptr.as_ptr()).mNumValuesAndWeights as usize }
    }

    /// Returns `true` if this key has no values/weights.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Morph target indices (zero-copy).
    pub fn values(&self) -> &[u32] {
        unsafe {
            let k = &*self.key_ptr.as_ptr();
            let n = k.mNumValuesAndWeights as usize;
            debug_assert!(n == 0 || !k.mValues.is_null());
            ffi::slice_from_ptr_len(self, k.mValues as *const u32, n)
        }
    }

    /// Morph target weights (zero-copy).
    pub fn weights(&self) -> &[f64] {
        unsafe {
            let k = &*self.key_ptr.as_ptr();
            let n = k.mNumValuesAndWeights as usize;
            debug_assert!(n == 0 || !k.mWeights.is_null());
            ffi::slice_from_ptr_len(self, k.mWeights as *const f64, n)
        }
    }
}

/// Morph mesh animation channel (aiMeshMorphAnim)
#[derive(Clone)]
pub struct MorphMeshAnimation {
    scene: Scene,
    channel_ptr: SharedPtr<sys::aiMeshMorphAnim>,
}

impl MorphMeshAnimation {
    /// # Safety
    /// Caller must ensure `channel_ptr` is non-null and points into a live `aiScene`.
    unsafe fn from_raw(scene: Scene, channel_ptr: *const sys::aiMeshMorphAnim) -> Self {
        debug_assert!(!channel_ptr.is_null());
        let channel_ptr = unsafe { SharedPtr::new_unchecked(channel_ptr) };
        Self { scene, channel_ptr }
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
    pub fn key(&self, index: usize) -> Option<MorphMeshKey> {
        if index >= self.num_keys() {
            return None;
        }
        unsafe {
            let ch = &*self.channel_ptr.as_ptr();
            let keys = ffi::slice_from_ptr_len_opt(ch, ch.mKeys, ch.mNumKeys as usize)?;
            let key_ref = keys.get(index)?;
            let key_ptr = SharedPtr::new(std::ptr::from_ref(key_ref))?;
            let k = &*key_ptr.as_ptr();
            let n = k.mNumValuesAndWeights as usize;
            if n > 0 && (k.mValues.is_null() || k.mWeights.is_null()) {
                return None;
            }
            Some(MorphMeshKey {
                scene: self.scene.clone(),
                key_ptr,
            })
        }
    }
}

/// Iterator over morph mesh animation channels
pub struct MorphMeshAnimationIterator {
    scene: Scene,
    animation_ptr: SharedPtr<sys::aiAnimation>,
    index: usize,
}

impl Iterator for MorphMeshAnimationIterator {
    type Item = MorphMeshAnimation;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let anim = &*self.animation_ptr.as_ptr();
            let chans = ffi::slice_from_ptr_len_opt(
                anim,
                anim.mMorphMeshChannels,
                anim.mNumMorphMeshChannels as usize,
            )?;
            while self.index < chans.len() {
                let ptr = chans[self.index];
                self.index += 1;
                if ptr.is_null() {
                    continue;
                }
                return Some(MorphMeshAnimation::from_raw(
                    self.scene.clone(),
                    ptr as *const sys::aiMeshMorphAnim,
                ));
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
