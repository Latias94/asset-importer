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
            if tps != 0.0 {
                tps
            } else {
                25.0
            } // Default to 25 FPS
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
    pub fn position_keys(&self) -> &[VectorKey] {
        unsafe {
            let channel = &*self.channel_ptr;
            std::slice::from_raw_parts(
                channel.mPositionKeys as *const VectorKey,
                channel.mNumPositionKeys as usize,
            )
        }
    }

    /// Get the number of rotation keyframes
    pub fn num_rotation_keys(&self) -> usize {
        unsafe { (*self.channel_ptr).mNumRotationKeys as usize }
    }

    /// Get the rotation keyframes
    pub fn rotation_keys(&self) -> &[QuaternionKey] {
        unsafe {
            let channel = &*self.channel_ptr;
            std::slice::from_raw_parts(
                channel.mRotationKeys as *const QuaternionKey,
                channel.mNumRotationKeys as usize,
            )
        }
    }

    /// Get the number of scaling keyframes
    pub fn num_scaling_keys(&self) -> usize {
        unsafe { (*self.channel_ptr).mNumScalingKeys as usize }
    }

    /// Get the scaling keyframes
    pub fn scaling_keys(&self) -> &[VectorKey] {
        unsafe {
            let channel = &*self.channel_ptr;
            std::slice::from_raw_parts(
                channel.mScalingKeys as *const VectorKey,
                channel.mNumScalingKeys as usize,
            )
        }
    }
}

/// A keyframe containing a time and a 3D vector value
#[repr(C)]
pub struct VectorKey {
    /// Time of the keyframe
    pub time: f64,
    /// Vector value at this time
    pub value: Vector3D,
}

/// A keyframe containing a time and a quaternion value
#[repr(C)]
pub struct QuaternionKey {
    /// Time of the keyframe
    pub time: f64,
    /// Quaternion value at this time
    pub value: Quaternion,
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
