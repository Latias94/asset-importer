//! Thin wrappers over Assimp C math helpers (optional).
//! These functions use the C API for operations on matrices, quaternions, and vectors,
//! returning glam types for ergonomics. Prefer glam for general math; use these to match
//! Assimp's exact semantics.

use crate::{
    sys,
    types::{
        from_ai_matrix3x3, from_ai_matrix4x4, from_ai_quaternion, from_ai_vector2d,
        from_ai_vector3d, to_ai_matrix3x3, to_ai_matrix4x4, to_ai_vector2d, to_ai_vector3d,
        Matrix3x3, Matrix4x4, Quaternion, Vector2D, Vector3D,
    },
};

// 4x4 matrices

/// 4x4 identity matrix using Assimp C API
pub fn identity_matrix4() -> Matrix4x4 {
    let mut m = sys::aiMatrix4x4::default();
    unsafe { sys::aiIdentityMatrix4(&mut m) };
    from_ai_matrix4x4(m)
}

/// Transpose a 4x4 matrix via Assimp
pub fn transpose_matrix4(m: Matrix4x4) -> Matrix4x4 {
    let mut a = to_ai_matrix4x4(m);
    unsafe { sys::aiTransposeMatrix4(&mut a) };
    from_ai_matrix4x4(a)
}

/// Multiply two 4x4 matrices using Assimp semantics: returns a * b
pub fn multiply_matrix4(a: Matrix4x4, b: Matrix4x4) -> Matrix4x4 {
    let mut dst = to_ai_matrix4x4(a);
    let src = to_ai_matrix4x4(b);
    unsafe { sys::aiMultiplyMatrix4(&mut dst, &src) };
    from_ai_matrix4x4(dst)
}

/// Transform a 3D vector by a 4x4 matrix via Assimp
pub fn transform_vec3_by_matrix4(v: Vector3D, m: Matrix4x4) -> Vector3D {
    let mut av = to_ai_vector3d(v);
    let am = to_ai_matrix4x4(m);
    unsafe { sys::aiTransformVecByMatrix4(&mut av, &am) };
    from_ai_vector3d(av)
}

/// Decompose a matrix into translation, rotation, scale using Assimp
pub fn decompose_matrix(m: Matrix4x4) -> (Vector3D, Quaternion, Vector3D) {
    let am = to_ai_matrix4x4(m);
    let mut scaling = sys::aiVector3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut rotation = sys::aiQuaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut position = sys::aiVector3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    unsafe { sys::aiDecomposeMatrix(&am, &mut scaling, &mut rotation, &mut position) };
    (
        from_ai_vector3d(position),
        from_ai_quaternion(rotation),
        from_ai_vector3d(scaling),
    )
}

// 3x3 matrices

/// 3x3 identity matrix via Assimp
pub fn identity_matrix3() -> Matrix3x3 {
    let mut m = sys::aiMatrix3x3::default();
    unsafe { sys::aiIdentityMatrix3(&mut m) };
    from_ai_matrix3x3(m)
}

/// Transpose a 3x3 matrix via Assimp
pub fn transpose_matrix3(m: Matrix3x3) -> Matrix3x3 {
    let mut a = to_ai_matrix3x3(m);
    unsafe { sys::aiTransposeMatrix3(&mut a) };
    from_ai_matrix3x3(a)
}

/// Multiply two 3x3 matrices using Assimp semantics: returns a * b
pub fn multiply_matrix3(a: Matrix3x3, b: Matrix3x3) -> Matrix3x3 {
    let mut dst = to_ai_matrix3x3(a);
    let src = to_ai_matrix3x3(b);
    unsafe { sys::aiMultiplyMatrix3(&mut dst, &src) };
    from_ai_matrix3x3(dst)
}

/// Transform a 3D vector by a 3x3 matrix via Assimp
pub fn transform_vec3_by_matrix3(v: Vector3D, m: Matrix3x3) -> Vector3D {
    let mut av = to_ai_vector3d(v);
    let am = to_ai_matrix3x3(m);
    unsafe { sys::aiTransformVecByMatrix3(&mut av, &am) };
    from_ai_vector3d(av)
}

// Quaternion helpers

/// Create a quaternion from the rotation part of a 4x4 matrix (via Assimp)
pub fn quaternion_from_matrix4(m: Matrix4x4) -> Quaternion {
    // Convert Mat4 -> aiMatrix3x3 using Assimp
    let am4 = to_ai_matrix4x4(m);
    let mut am3 = sys::aiMatrix3x3::default();
    unsafe { sys::aiMatrix3FromMatrix4(&mut am3, &am4) };

    let mut q = sys::aiQuaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    unsafe { sys::aiCreateQuaternionFromMatrix(&mut q, &am3) };
    from_ai_quaternion(q)
}

/// Normalize quaternion via Assimp
pub fn quaternion_normalize(q: Quaternion) -> Quaternion {
    let mut aq = sys::aiQuaternion {
        w: q.w,
        x: q.x,
        y: q.y,
        z: q.z,
    };
    unsafe { sys::aiQuaternionNormalize(&mut aq) };
    from_ai_quaternion(aq)
}

/// Interpolate two quaternions via Assimp (slerp)
pub fn quaternion_interpolate(a: Quaternion, b: Quaternion, d: f32) -> Quaternion {
    let aq = sys::aiQuaternion {
        w: a.w,
        x: a.x,
        y: a.y,
        z: a.z,
    };
    let bq = sys::aiQuaternion {
        w: b.w,
        x: b.x,
        y: b.y,
        z: b.z,
    };
    let mut out = sys::aiQuaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    unsafe { sys::aiQuaternionInterpolate(&mut out, &aq, &bq, d) };
    from_ai_quaternion(out)
}

// ===================== Vector2 helpers =====================

pub fn vector2_equal(a: Vector2D, b: Vector2D) -> bool {
    let aa = sys::aiVector2D { x: a.x, y: a.y };
    let bb = sys::aiVector2D { x: b.x, y: b.y };
    unsafe { sys::aiVector2AreEqual(&aa, &bb) != 0 }
}

pub fn vector2_equal_epsilon(a: Vector2D, b: Vector2D, eps: f32) -> bool {
    let aa = sys::aiVector2D { x: a.x, y: a.y };
    let bb = sys::aiVector2D { x: b.x, y: b.y };
    unsafe { sys::aiVector2AreEqualEpsilon(&aa, &bb, eps) != 0 }
}

pub fn vector2_add(a: Vector2D, b: Vector2D) -> Vector2D {
    let mut dst = to_ai_vector2d(a);
    let src = to_ai_vector2d(b);
    unsafe { sys::aiVector2Add(&mut dst, &src) };
    from_ai_vector2d(dst)
}

pub fn vector2_sub(a: Vector2D, b: Vector2D) -> Vector2D {
    let mut dst = to_ai_vector2d(a);
    let src = to_ai_vector2d(b);
    unsafe { sys::aiVector2Subtract(&mut dst, &src) };
    from_ai_vector2d(dst)
}

pub fn vector2_scale(v: Vector2D, s: f32) -> Vector2D {
    let mut dst = to_ai_vector2d(v);
    unsafe { sys::aiVector2Scale(&mut dst, s) };
    from_ai_vector2d(dst)
}

pub fn vector2_sym_mul(a: Vector2D, b: Vector2D) -> Vector2D {
    let mut dst = to_ai_vector2d(a);
    let other = to_ai_vector2d(b);
    unsafe { sys::aiVector2SymMul(&mut dst, &other) };
    from_ai_vector2d(dst)
}

pub fn vector2_div_scalar(v: Vector2D, s: f32) -> Vector2D {
    let mut dst = to_ai_vector2d(v);
    unsafe { sys::aiVector2DivideByScalar(&mut dst, s) };
    from_ai_vector2d(dst)
}

pub fn vector2_div_vector(a: Vector2D, b: Vector2D) -> Vector2D {
    let mut dst = to_ai_vector2d(a);
    let mut v = to_ai_vector2d(b);
    unsafe { sys::aiVector2DivideByVector(&mut dst, &mut v) };
    from_ai_vector2d(dst)
}

pub fn vector2_length(v: Vector2D) -> f32 {
    let vv = to_ai_vector2d(v);
    unsafe { sys::aiVector2Length(&vv) as f32 }
}

pub fn vector2_length_squared(v: Vector2D) -> f32 {
    let vv = to_ai_vector2d(v);
    unsafe { sys::aiVector2SquareLength(&vv) as f32 }
}

pub fn vector2_negate(v: Vector2D) -> Vector2D {
    let mut dst = to_ai_vector2d(v);
    unsafe { sys::aiVector2Negate(&mut dst) };
    from_ai_vector2d(dst)
}

pub fn vector2_dot(a: Vector2D, b: Vector2D) -> f32 {
    let aa = to_ai_vector2d(a);
    let bb = to_ai_vector2d(b);
    unsafe { sys::aiVector2DotProduct(&aa, &bb) as f32 }
}

// ===================== Vector3 helpers =====================

pub fn vector3_equal(a: Vector3D, b: Vector3D) -> bool {
    let aa = to_ai_vector3d(a);
    let bb = to_ai_vector3d(b);
    unsafe { sys::aiVector3AreEqual(&aa, &bb) != 0 }
}

pub fn vector3_equal_epsilon(a: Vector3D, b: Vector3D, eps: f32) -> bool {
    let aa = to_ai_vector3d(a);
    let bb = to_ai_vector3d(b);
    unsafe { sys::aiVector3AreEqualEpsilon(&aa, &bb, eps) != 0 }
}

pub fn vector3_less_than(a: Vector3D, b: Vector3D) -> bool {
    let aa = to_ai_vector3d(a);
    let bb = to_ai_vector3d(b);
    unsafe { sys::aiVector3LessThan(&aa, &bb) != 0 }
}

pub fn vector3_add(a: Vector3D, b: Vector3D) -> Vector3D {
    let mut dst = to_ai_vector3d(a);
    let src = to_ai_vector3d(b);
    unsafe { sys::aiVector3Add(&mut dst, &src) };
    from_ai_vector3d(dst)
}

pub fn vector3_sub(a: Vector3D, b: Vector3D) -> Vector3D {
    let mut dst = to_ai_vector3d(a);
    let src = to_ai_vector3d(b);
    unsafe { sys::aiVector3Subtract(&mut dst, &src) };
    from_ai_vector3d(dst)
}

pub fn vector3_scale(v: Vector3D, s: f32) -> Vector3D {
    let mut dst = to_ai_vector3d(v);
    unsafe { sys::aiVector3Scale(&mut dst, s) };
    from_ai_vector3d(dst)
}

pub fn vector3_sym_mul(a: Vector3D, b: Vector3D) -> Vector3D {
    let mut dst = to_ai_vector3d(a);
    let other = to_ai_vector3d(b);
    unsafe { sys::aiVector3SymMul(&mut dst, &other) };
    from_ai_vector3d(dst)
}

pub fn vector3_div_scalar(v: Vector3D, s: f32) -> Vector3D {
    let mut dst = to_ai_vector3d(v);
    unsafe { sys::aiVector3DivideByScalar(&mut dst, s) };
    from_ai_vector3d(dst)
}

pub fn vector3_div_vector(a: Vector3D, b: Vector3D) -> Vector3D {
    let mut dst = to_ai_vector3d(a);
    let mut other = to_ai_vector3d(b);
    unsafe { sys::aiVector3DivideByVector(&mut dst, &mut other) };
    from_ai_vector3d(dst)
}

pub fn vector3_length(v: Vector3D) -> f32 {
    let vv = to_ai_vector3d(v);
    unsafe { sys::aiVector3Length(&vv) as f32 }
}

pub fn vector3_length_squared(v: Vector3D) -> f32 {
    let vv = to_ai_vector3d(v);
    unsafe { sys::aiVector3SquareLength(&vv) as f32 }
}

pub fn vector3_negate(v: Vector3D) -> Vector3D {
    let mut vv = to_ai_vector3d(v);
    unsafe { sys::aiVector3Negate(&mut vv) };
    from_ai_vector3d(vv)
}

pub fn vector3_dot(a: Vector3D, b: Vector3D) -> f32 {
    let aa = to_ai_vector3d(a);
    let bb = to_ai_vector3d(b);
    unsafe { sys::aiVector3DotProduct(&aa, &bb) as f32 }
}

pub fn vector3_cross(a: Vector3D, b: Vector3D) -> Vector3D {
    let mut dst = sys::aiVector3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let aa = to_ai_vector3d(a);
    let bb = to_ai_vector3d(b);
    unsafe { sys::aiVector3CrossProduct(&mut dst, &aa, &bb) };
    from_ai_vector3d(dst)
}

pub fn vector3_normalize(v: Vector3D) -> Vector3D {
    let mut vv = to_ai_vector3d(v);
    unsafe { sys::aiVector3Normalize(&mut vv) };
    from_ai_vector3d(vv)
}

pub fn vector3_normalize_safe(v: Vector3D) -> Vector3D {
    let mut vv = to_ai_vector3d(v);
    unsafe { sys::aiVector3NormalizeSafe(&mut vv) };
    from_ai_vector3d(vv)
}

pub fn vector3_rotate_by_quaternion(v: Vector3D, q: Quaternion) -> Vector3D {
    let mut vv = to_ai_vector3d(v);
    let qq = sys::aiQuaternion {
        w: q.w,
        x: q.x,
        y: q.y,
        z: q.z,
    };
    unsafe { sys::aiVector3RotateByQuaternion(&mut vv, &qq) };
    from_ai_vector3d(vv)
}

// ===================== Matrix3 extra =====================

pub fn matrix3_from_matrix4(m: Matrix4x4) -> Matrix3x3 {
    let am = to_ai_matrix4x4(m);
    let mut out = sys::aiMatrix3x3::default();
    unsafe { sys::aiMatrix3FromMatrix4(&mut out, &am) };
    from_ai_matrix3x3(out)
}

pub fn matrix3_from_quaternion(q: Quaternion) -> Matrix3x3 {
    let mut out = sys::aiMatrix3x3::default();
    let qq = sys::aiQuaternion {
        w: q.w,
        x: q.x,
        y: q.y,
        z: q.z,
    };
    unsafe { sys::aiMatrix3FromQuaternion(&mut out, &qq) };
    from_ai_matrix3x3(out)
}

pub fn matrix3_are_equal(a: Matrix3x3, b: Matrix3x3) -> bool {
    let aa = to_ai_matrix3x3(a);
    let bb = to_ai_matrix3x3(b);
    unsafe { sys::aiMatrix3AreEqual(&aa, &bb) != 0 }
}

pub fn matrix3_are_equal_epsilon(a: Matrix3x3, b: Matrix3x3, eps: f32) -> bool {
    let aa = to_ai_matrix3x3(a);
    let bb = to_ai_matrix3x3(b);
    unsafe { sys::aiMatrix3AreEqualEpsilon(&aa, &bb, eps) != 0 }
}

pub fn matrix3_inverse(m: Matrix3x3) -> Matrix3x3 {
    let mut am = to_ai_matrix3x3(m);
    unsafe { sys::aiMatrix3Inverse(&mut am) };
    from_ai_matrix3x3(am)
}

pub fn matrix3_determinant(m: Matrix3x3) -> f32 {
    let am = to_ai_matrix3x3(m);
    unsafe { sys::aiMatrix3Determinant(&am) as f32 }
}

pub fn matrix3_rotation_z(angle: f32) -> Matrix3x3 {
    let mut out = sys::aiMatrix3x3::default();
    unsafe { sys::aiMatrix3RotationZ(&mut out, angle) };
    from_ai_matrix3x3(out)
}

pub fn matrix3_from_rotation_axis(axis: Vector3D, angle: f32) -> Matrix3x3 {
    let mut out = sys::aiMatrix3x3::default();
    let ax = to_ai_vector3d(axis);
    unsafe { sys::aiMatrix3FromRotationAroundAxis(&mut out, &ax, angle) };
    from_ai_matrix3x3(out)
}

pub fn matrix3_translation(t: Vector2D) -> Matrix3x3 {
    let mut out = sys::aiMatrix3x3::default();
    let tv = to_ai_vector2d(t);
    unsafe { sys::aiMatrix3Translation(&mut out, &tv) };
    from_ai_matrix3x3(out)
}

pub fn matrix3_from_to(from: Vector3D, to: Vector3D) -> Matrix3x3 {
    let mut out = sys::aiMatrix3x3::default();
    let f = to_ai_vector3d(from);
    let t = to_ai_vector3d(to);
    unsafe { sys::aiMatrix3FromTo(&mut out, &f, &t) };
    from_ai_matrix3x3(out)
}

// ===================== Matrix4 extra =====================

pub fn matrix4_from_matrix3(m: Matrix3x3) -> Matrix4x4 {
    let am = to_ai_matrix3x3(m);
    let mut out = sys::aiMatrix4x4::default();
    unsafe { sys::aiMatrix4FromMatrix3(&mut out, &am) };
    from_ai_matrix4x4(out)
}

pub fn matrix4_from_s_q_t(scale: Vector3D, rot: Quaternion, pos: Vector3D) -> Matrix4x4 {
    let mut out = sys::aiMatrix4x4::default();
    let s = to_ai_vector3d(scale);
    let q = sys::aiQuaternion {
        w: rot.w,
        x: rot.x,
        y: rot.y,
        z: rot.z,
    };
    let p = to_ai_vector3d(pos);
    unsafe { sys::aiMatrix4FromScalingQuaternionPosition(&mut out, &s, &q, &p) };
    from_ai_matrix4x4(out)
}

pub fn matrix4_add(a: Matrix4x4, b: Matrix4x4) -> Matrix4x4 {
    let mut aa = to_ai_matrix4x4(a);
    let bb = to_ai_matrix4x4(b);
    unsafe { sys::aiMatrix4Add(&mut aa, &bb) };
    from_ai_matrix4x4(aa)
}

pub fn matrix4_are_equal(a: Matrix4x4, b: Matrix4x4) -> bool {
    let aa = to_ai_matrix4x4(a);
    let bb = to_ai_matrix4x4(b);
    unsafe { sys::aiMatrix4AreEqual(&aa, &bb) != 0 }
}

pub fn matrix4_are_equal_epsilon(a: Matrix4x4, b: Matrix4x4, eps: f32) -> bool {
    let aa = to_ai_matrix4x4(a);
    let bb = to_ai_matrix4x4(b);
    unsafe { sys::aiMatrix4AreEqualEpsilon(&aa, &bb, eps) != 0 }
}

pub fn matrix4_inverse(m: Matrix4x4) -> Matrix4x4 {
    let mut am = to_ai_matrix4x4(m);
    unsafe { sys::aiMatrix4Inverse(&mut am) };
    from_ai_matrix4x4(am)
}

pub fn matrix4_determinant(m: Matrix4x4) -> f32 {
    let am = to_ai_matrix4x4(m);
    unsafe { sys::aiMatrix4Determinant(&am) as f32 }
}

pub fn matrix4_is_identity(m: Matrix4x4) -> bool {
    let am = to_ai_matrix4x4(m);
    unsafe { sys::aiMatrix4IsIdentity(&am) != 0 }
}

pub fn matrix4_decompose_euler(m: Matrix4x4) -> (Vector3D, Vector3D, Vector3D) {
    let am = to_ai_matrix4x4(m);
    let mut s = sys::aiVector3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut r = sys::aiVector3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut p = sys::aiVector3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    unsafe { sys::aiMatrix4DecomposeIntoScalingEulerAnglesPosition(&am, &mut s, &mut r, &mut p) };
    (
        from_ai_vector3d(s),
        from_ai_vector3d(r),
        from_ai_vector3d(p),
    )
}

pub fn matrix4_decompose_axis_angle(m: Matrix4x4) -> (Vector3D, Vector3D, f32, Vector3D) {
    let am = to_ai_matrix4x4(m);
    let mut s = sys::aiVector3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut axis = sys::aiVector3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut angle: sys::ai_real = 0.0;
    let mut p = sys::aiVector3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    unsafe {
        sys::aiMatrix4DecomposeIntoScalingAxisAnglePosition(
            &am, &mut s, &mut axis, &mut angle, &mut p,
        )
    };
    (
        from_ai_vector3d(s),
        from_ai_vector3d(axis),
        angle as f32,
        from_ai_vector3d(p),
    )
}

pub fn matrix4_decompose_no_scaling(m: Matrix4x4) -> (Quaternion, Vector3D) {
    let am = to_ai_matrix4x4(m);
    let mut q = sys::aiQuaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let mut p = sys::aiVector3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    unsafe { sys::aiMatrix4DecomposeNoScaling(&am, &mut q, &mut p) };
    (from_ai_quaternion(q), from_ai_vector3d(p))
}

pub fn matrix4_from_euler(x: f32, y: f32, z: f32) -> Matrix4x4 {
    let mut out = sys::aiMatrix4x4::default();
    unsafe { sys::aiMatrix4FromEulerAngles(&mut out, x, y, z) };
    from_ai_matrix4x4(out)
}

pub fn matrix4_rotation_x(angle: f32) -> Matrix4x4 {
    let mut out = sys::aiMatrix4x4::default();
    unsafe { sys::aiMatrix4RotationX(&mut out, angle) };
    from_ai_matrix4x4(out)
}
pub fn matrix4_rotation_y(angle: f32) -> Matrix4x4 {
    let mut out = sys::aiMatrix4x4::default();
    unsafe { sys::aiMatrix4RotationY(&mut out, angle) };
    from_ai_matrix4x4(out)
}
pub fn matrix4_rotation_z(angle: f32) -> Matrix4x4 {
    let mut out = sys::aiMatrix4x4::default();
    unsafe { sys::aiMatrix4RotationZ(&mut out, angle) };
    from_ai_matrix4x4(out)
}

pub fn matrix4_from_rotation_axis(axis: Vector3D, angle: f32) -> Matrix4x4 {
    let mut out = sys::aiMatrix4x4::default();
    let ax = to_ai_vector3d(axis);
    unsafe { sys::aiMatrix4FromRotationAroundAxis(&mut out, &ax, angle) };
    from_ai_matrix4x4(out)
}

pub fn matrix4_translation(t: Vector3D) -> Matrix4x4 {
    let mut out = sys::aiMatrix4x4::default();
    let tt = to_ai_vector3d(t);
    unsafe { sys::aiMatrix4Translation(&mut out, &tt) };
    from_ai_matrix4x4(out)
}

pub fn matrix4_scaling(s: Vector3D) -> Matrix4x4 {
    let mut out = sys::aiMatrix4x4::default();
    let ss = to_ai_vector3d(s);
    unsafe { sys::aiMatrix4Scaling(&mut out, &ss) };
    from_ai_matrix4x4(out)
}

pub fn matrix4_from_to(from: Vector3D, to: Vector3D) -> Matrix4x4 {
    let mut out = sys::aiMatrix4x4::default();
    let f = to_ai_vector3d(from);
    let t = to_ai_vector3d(to);
    unsafe { sys::aiMatrix4FromTo(&mut out, &f, &t) };
    from_ai_matrix4x4(out)
}

// ===================== Quaternion extra =====================

pub fn quaternion_from_euler(x: f32, y: f32, z: f32) -> Quaternion {
    let mut q = sys::aiQuaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    unsafe { sys::aiQuaternionFromEulerAngles(&mut q, x, y, z) };
    from_ai_quaternion(q)
}

pub fn quaternion_from_axis_angle(axis: Vector3D, angle: f32) -> Quaternion {
    let mut q = sys::aiQuaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let ax = to_ai_vector3d(axis);
    unsafe { sys::aiQuaternionFromAxisAngle(&mut q, &ax, angle) };
    from_ai_quaternion(q)
}

pub fn quaternion_from_normalized_vector3(v: Vector3D) -> Quaternion {
    let mut q = sys::aiQuaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let vv = to_ai_vector3d(v);
    unsafe { sys::aiQuaternionFromNormalizedQuaternion(&mut q, &vv) };
    from_ai_quaternion(q)
}

pub fn quaternion_equal(a: Quaternion, b: Quaternion) -> bool {
    let aa = sys::aiQuaternion {
        w: a.w,
        x: a.x,
        y: a.y,
        z: a.z,
    };
    let bb = sys::aiQuaternion {
        w: b.w,
        x: b.x,
        y: b.y,
        z: b.z,
    };
    unsafe { sys::aiQuaternionAreEqual(&aa, &bb) != 0 }
}

pub fn quaternion_equal_epsilon(a: Quaternion, b: Quaternion, eps: f32) -> bool {
    let aa = sys::aiQuaternion {
        w: a.w,
        x: a.x,
        y: a.y,
        z: a.z,
    };
    let bb = sys::aiQuaternion {
        w: b.w,
        x: b.x,
        y: b.y,
        z: b.z,
    };
    unsafe { sys::aiQuaternionAreEqualEpsilon(&aa, &bb, eps) != 0 }
}

pub fn quaternion_conjugate(q: Quaternion) -> Quaternion {
    let mut qq = sys::aiQuaternion {
        w: q.w,
        x: q.x,
        y: q.y,
        z: q.z,
    };
    unsafe { sys::aiQuaternionConjugate(&mut qq) };
    from_ai_quaternion(qq)
}

pub fn quaternion_multiply(a: Quaternion, b: Quaternion) -> Quaternion {
    let mut dst = sys::aiQuaternion {
        w: a.w,
        x: a.x,
        y: a.y,
        z: a.z,
    };
    let qb = sys::aiQuaternion {
        w: b.w,
        x: b.x,
        y: b.y,
        z: b.z,
    };
    unsafe { sys::aiQuaternionMultiply(&mut dst, &qb) };
    from_ai_quaternion(dst)
}
