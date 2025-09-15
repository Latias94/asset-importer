//! Utility functions and helpers for working with 3D assets
//!
//! This module provides convenience functions for common operations.
//! For advanced mathematical operations, use the glam methods directly
//! on Vector3D, Matrix4x4, Quaternion, etc.
//!
//! # Examples
//!
//! ```rust
//! use asset_importer::types::*;
//! use asset_importer::utils::*;
//!
//! // Use glam directly for math operations
//! let v1 = Vector3D::new(1.0, 2.0, 3.0);
//! let v2 = Vector3D::new(4.0, 5.0, 6.0);
//! let dot_product = v1.dot(v2);
//! let cross_product = v1.cross(v2);
//! let normalized = v1.normalize();
//!
//! // Use utilities for convenience
//! let bounds = calculate_bounding_box(&[v1, v2]);
//! ```

use crate::types::*;

/// Calculate the bounding box of a set of points
pub fn calculate_bounding_box(points: &[Vector3D]) -> (Vector3D, Vector3D) {
    if points.is_empty() {
        return (Vector3D::ZERO, Vector3D::ZERO);
    }

    let mut min = points[0];
    let mut max = points[0];

    for &point in points.iter().skip(1) {
        min = min.min(point);
        max = max.max(point);
    }

    (min, max)
}

/// Calculate the center point of a bounding box
pub fn bounding_box_center(min: Vector3D, max: Vector3D) -> Vector3D {
    (min + max) * 0.5
}

/// Calculate the size of a bounding box
pub fn bounding_box_size(min: Vector3D, max: Vector3D) -> Vector3D {
    max - min
}

/// Convert degrees to radians
pub fn degrees_to_radians(degrees: f32) -> f32 {
    degrees * std::f32::consts::PI / 180.0
}

/// Convert radians to degrees
pub fn radians_to_degrees(radians: f32) -> f32 {
    radians * 180.0 / std::f32::consts::PI
}

/// Clamp a value between min and max
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

/// Linear interpolation between two values
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Check if two floating point values are approximately equal
pub fn approximately_equal(a: f32, b: f32, epsilon: f32) -> bool {
    (a - b).abs() < epsilon
}

/// Color utilities
pub mod color {
    use super::*;

    /// Convert RGB to HSV color space
    pub fn rgb_to_hsv(rgb: Color3D) -> Vector3D {
        let r = rgb.x;
        let g = rgb.y;
        let b = rgb.z;

        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let delta = max - min;

        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };

        let s = if max == 0.0 { 0.0 } else { delta / max };
        let v = max;

        Vector3D::new(h, s, v)
    }

    /// Convert HSV to RGB color space
    pub fn hsv_to_rgb(hsv: Vector3D) -> Color3D {
        let h = hsv.x;
        let s = hsv.y;
        let v = hsv.z;

        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Color3D::new(r + m, g + m, b + m)
    }

    /// Apply gamma correction to a color
    pub fn gamma_correct(color: Color3D, gamma: f32) -> Color3D {
        Color3D::new(
            color.x.powf(1.0 / gamma),
            color.y.powf(1.0 / gamma),
            color.z.powf(1.0 / gamma),
        )
    }

    /// Convert linear RGB to sRGB
    pub fn linear_to_srgb(linear: Color3D) -> Color3D {
        fn linear_to_srgb_component(c: f32) -> f32 {
            if c <= 0.0031308 {
                12.92 * c
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        }

        Color3D::new(
            linear_to_srgb_component(linear.x),
            linear_to_srgb_component(linear.y),
            linear_to_srgb_component(linear.z),
        )
    }

    /// Convert sRGB to linear RGB
    pub fn srgb_to_linear(srgb: Color3D) -> Color3D {
        fn srgb_to_linear_component(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }

        Color3D::new(
            srgb_to_linear_component(srgb.x),
            srgb_to_linear_component(srgb.y),
            srgb_to_linear_component(srgb.z),
        )
    }
}

/// Transformation utilities
pub mod transform {
    use super::*;

    /// Create a look-at matrix (using glam)
    pub fn look_at(eye: Vector3D, target: Vector3D, up: Vector3D) -> Matrix4x4 {
        Matrix4x4::look_at_rh(eye, target, up)
    }

    /// Create a perspective projection matrix (using glam)
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Matrix4x4 {
        Matrix4x4::perspective_rh(fov_y, aspect, near, far)
    }

    /// Create an orthographic projection matrix (using glam)
    pub fn orthographic(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> Matrix4x4 {
        Matrix4x4::orthographic_rh(left, right, bottom, top, near, far)
    }

    /// Decompose a transformation matrix (using glam)
    pub fn decompose_matrix(matrix: Matrix4x4) -> (Vector3D, Quaternion, Vector3D) {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        (translation, rotation, scale)
    }

    /// Compose a transformation matrix (using glam)
    pub fn compose_matrix(
        translation: Vector3D,
        rotation: Quaternion,
        scale: Vector3D,
    ) -> Matrix4x4 {
        Matrix4x4::from_scale_rotation_translation(scale, rotation, translation)
    }
}

/// Mesh utilities
pub mod mesh {
    use super::*;

    /// Calculate face normal from three vertices
    pub fn calculate_face_normal(v0: Vector3D, v1: Vector3D, v2: Vector3D) -> Vector3D {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        edge1.cross(edge2).normalize()
    }

    /// Calculate the area of a triangle
    pub fn triangle_area(v0: Vector3D, v1: Vector3D, v2: Vector3D) -> f32 {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        edge1.cross(edge2).length() * 0.5
    }

    /// Check if a point is inside a triangle (2D)
    pub fn point_in_triangle_2d(point: Vector2D, v0: Vector2D, v1: Vector2D, v2: Vector2D) -> bool {
        fn sign(p1: Vector2D, p2: Vector2D, p3: Vector2D) -> f32 {
            (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
        }

        let d1 = sign(point, v0, v1);
        let d2 = sign(point, v1, v2);
        let d3 = sign(point, v2, v0);

        let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
        let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);

        !(has_neg && has_pos)
    }
}

/// Animation utilities
pub mod animation {
    use super::*;

    /// Interpolate between two quaternions using spherical linear interpolation
    pub fn slerp_quaternion(q1: Quaternion, q2: Quaternion, t: f32) -> Quaternion {
        q1.slerp(q2, t)
    }

    /// Interpolate between two vectors using linear interpolation
    pub fn lerp_vector3(v1: Vector3D, v2: Vector3D, t: f32) -> Vector3D {
        v1.lerp(v2, t)
    }

    /// Smooth step interpolation
    pub fn smooth_step(t: f32) -> f32 {
        t * t * (3.0 - 2.0 * t)
    }

    /// Smoother step interpolation
    pub fn smoother_step(t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }
}
