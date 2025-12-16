//! Axis-Aligned Bounding Box (AABB) support
//!
//! This module provides functionality for working with axis-aligned bounding boxes,
//! which are commonly used for collision detection, frustum culling, and spatial
//! optimization in 3D graphics applications.

use crate::{
    sys,
    types::{Matrix4x4, Vector3D},
};

/// An axis-aligned bounding box in 3D space
///
/// An AABB is defined by its minimum and maximum corner points.
/// It's called "axis-aligned" because its faces are parallel to the coordinate axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    /// Minimum corner of the bounding box
    pub min: Vector3D,
    /// Maximum corner of the bounding box
    pub max: Vector3D,
}

impl AABB {
    /// Create a new AABB with the given minimum and maximum points
    pub fn new(min: Vector3D, max: Vector3D) -> Self {
        Self { min, max }
    }

    /// Create an empty AABB (min > max, indicating no volume)
    pub fn empty() -> Self {
        Self {
            min: Vector3D::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vector3D::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }

    /// Create an AABB that encompasses the entire coordinate space
    pub fn infinite() -> Self {
        Self {
            min: Vector3D::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
            max: Vector3D::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        }
    }

    /// Create an AABB from a single point
    pub fn from_point(point: Vector3D) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    /// Create an AABB from a collection of points
    pub fn from_points<I>(points: I) -> Self
    where
        I: IntoIterator<Item = Vector3D>,
    {
        let mut aabb = Self::empty();
        for point in points {
            aabb.expand_to_include_point(point);
        }
        aabb
    }

    /// Check if this AABB is empty (has no volume)
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    /// Check if this AABB is valid (min <= max for all axes)
    pub fn is_valid(&self) -> bool {
        !self.is_empty()
    }

    /// Get the center point of the AABB
    pub fn center(&self) -> Vector3D {
        (self.min + self.max) * 0.5
    }

    /// Get the size (extent) of the AABB along each axis
    pub fn size(&self) -> Vector3D {
        if self.is_empty() {
            Vector3D::ZERO
        } else {
            self.max - self.min
        }
    }

    /// Get the half-size (half-extent) of the AABB
    pub fn half_size(&self) -> Vector3D {
        self.size() * 0.5
    }

    /// Get the volume of the AABB
    pub fn volume(&self) -> f32 {
        if self.is_empty() {
            0.0
        } else {
            let size = self.size();
            size.x * size.y * size.z
        }
    }

    /// Get the surface area of the AABB
    pub fn surface_area(&self) -> f32 {
        if self.is_empty() {
            0.0
        } else {
            let size = self.size();
            2.0 * (size.x * size.y + size.y * size.z + size.z * size.x)
        }
    }

    /// Get the diagonal length of the AABB
    pub fn diagonal_length(&self) -> f32 {
        if self.is_empty() {
            0.0
        } else {
            self.size().length()
        }
    }

    /// Expand the AABB to include a point
    pub fn expand_to_include_point(&mut self, point: Vector3D) {
        if self.is_empty() {
            self.min = point;
            self.max = point;
        } else {
            self.min = self.min.min(point);
            self.max = self.max.max(point);
        }
    }

    /// Expand the AABB to include another AABB
    pub fn expand_to_include_aabb(&mut self, other: &AABB) {
        if other.is_empty() {
            return;
        }
        if self.is_empty() {
            *self = *other;
        } else {
            self.min = self.min.min(other.min);
            self.max = self.max.max(other.max);
        }
    }

    /// Create a new AABB that includes both this AABB and a point
    pub fn expanded_to_include_point(&self, point: Vector3D) -> Self {
        let mut result = *self;
        result.expand_to_include_point(point);
        result
    }

    /// Create a new AABB that includes both this AABB and another AABB
    pub fn expanded_to_include_aabb(&self, other: &AABB) -> Self {
        let mut result = *self;
        result.expand_to_include_aabb(other);
        result
    }

    /// Check if a point is inside this AABB
    pub fn contains_point(&self, point: Vector3D) -> bool {
        !self.is_empty()
            && point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Check if another AABB is completely inside this AABB
    pub fn contains_aabb(&self, other: &AABB) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.min.x <= other.min.x
            && self.max.x >= other.max.x
            && self.min.y <= other.min.y
            && self.max.y >= other.max.y
            && self.min.z <= other.min.z
            && self.max.z >= other.max.z
    }

    /// Check if this AABB intersects with another AABB
    pub fn intersects_aabb(&self, other: &AABB) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Get the intersection of this AABB with another AABB
    pub fn intersection(&self, other: &AABB) -> Self {
        if !self.intersects_aabb(other) {
            return Self::empty();
        }

        Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        }
    }

    /// Transform this AABB by a matrix
    ///
    /// Note: This may result in a larger AABB than necessary for rotated boxes,
    /// as the result is still axis-aligned.
    pub fn transformed(&self, matrix: &Matrix4x4) -> Self {
        if self.is_empty() {
            return *self;
        }

        let mut min = glam::Vec3::splat(f32::INFINITY);
        let mut max = glam::Vec3::splat(f32::NEG_INFINITY);

        for corner in self.corners() {
            let transformed = matrix.transform_point3(corner);

            min = min.min(transformed);
            max = max.max(transformed);
        }

        AABB { min, max }
    }

    /// Expand the AABB by a uniform amount in all directions
    pub fn expanded(&self, amount: f32) -> Self {
        if self.is_empty() {
            return *self;
        }

        let expansion = Vector3D::splat(amount);
        Self {
            min: self.min - expansion,
            max: self.max + expansion,
        }
    }

    /// Get the 8 corner points of the AABB
    pub fn corners(&self) -> [Vector3D; 8] {
        [
            Vector3D::new(self.min.x, self.min.y, self.min.z), // min corner
            Vector3D::new(self.max.x, self.min.y, self.min.z),
            Vector3D::new(self.min.x, self.max.y, self.min.z),
            Vector3D::new(self.max.x, self.max.y, self.min.z),
            Vector3D::new(self.min.x, self.min.y, self.max.z),
            Vector3D::new(self.max.x, self.min.y, self.max.z),
            Vector3D::new(self.min.x, self.max.y, self.max.z),
            Vector3D::new(self.max.x, self.max.y, self.max.z), // max corner
        ]
    }

    /// Get the closest point on the AABB to a given point
    pub fn closest_point(&self, point: Vector3D) -> Vector3D {
        if self.is_empty() {
            return point;
        }

        Vector3D::new(
            point.x.clamp(self.min.x, self.max.x),
            point.y.clamp(self.min.y, self.max.y),
            point.z.clamp(self.min.z, self.max.z),
        )
    }

    /// Get the squared distance from a point to this AABB
    pub fn distance_squared_to_point(&self, point: Vector3D) -> f32 {
        let closest = self.closest_point(point);
        point.distance_squared(closest)
    }

    /// Get the distance from a point to this AABB
    pub fn distance_to_point(&self, point: Vector3D) -> f32 {
        self.distance_squared_to_point(point).sqrt()
    }
}

impl From<&sys::aiAABB> for AABB {
    fn from(aabb: &sys::aiAABB) -> Self {
        Self {
            min: Vector3D::new(aabb.mMin.x, aabb.mMin.y, aabb.mMin.z),
            max: Vector3D::new(aabb.mMax.x, aabb.mMax.y, aabb.mMax.z),
        }
    }
}

impl Default for AABB {
    fn default() -> Self {
        Self::empty()
    }
}
