use approx::assert_abs_diff_eq;
use asset_importer::{
    aabb::AABB,
    math,
    types::{Color3D, Matrix4x4, Quaternion, Vector3D, Vector4D},
    utils::{color, transform},
};

fn assert_vector_close(actual: Vector3D, expected: Vector3D) {
    assert_abs_diff_eq!(actual.x, expected.x, epsilon = 1e-5);
    assert_abs_diff_eq!(actual.y, expected.y, epsilon = 1e-5);
    assert_abs_diff_eq!(actual.z, expected.z, epsilon = 1e-5);
}

fn assert_matrix_close(actual: Matrix4x4, expected: Matrix4x4) {
    for (actual_column, expected_column) in actual
        .to_cols_array_2d()
        .into_iter()
        .zip(expected.to_cols_array_2d())
    {
        for (actual, expected) in actual_column.into_iter().zip(expected_column) {
            assert_abs_diff_eq!(actual, expected, epsilon = 1e-5);
        }
    }
}

#[test]
fn zero_euler_angles_produce_identity() {
    assert_eq!(math::matrix4_from_euler(0.0, 0.0, 0.0), Matrix4x4::IDENTITY);
}

#[test]
fn euler_rotation_is_invertible_and_preserves_composed_translation() {
    let rotation = math::matrix4_from_euler(0.37, -0.61, 1.23);
    assert_abs_diff_eq!(math::matrix4_determinant(rotation), 1.0, epsilon = 1e-5);
    assert_eq!(rotation.w_axis, Vector4D::new(0.0, 0.0, 0.0, 1.0));
    assert_eq!(
        [rotation.x_axis.w, rotation.y_axis.w, rotation.z_axis.w],
        [0.0, 0.0, 0.0]
    );
    assert_matrix_close(
        math::multiply_matrix4(rotation, math::matrix4_inverse(rotation)),
        Matrix4x4::IDENTITY,
    );

    let translation = Vector3D::new(4.0, -3.0, 2.0);
    let translation_matrix = Matrix4x4::from_scale_rotation_translation(
        Vector3D::splat(1.0),
        Quaternion::IDENTITY,
        translation,
    );
    let composed = math::multiply_matrix4(translation_matrix, rotation);
    assert_eq!(composed.w_axis, translation.extend(1.0));
}

#[test]
fn signed_scales_survive_direct_and_utility_decomposition() {
    for (rotation, translation) in [
        (Quaternion::IDENTITY, Vector3D::ZERO),
        (
            Quaternion::from_xyzw(0.2, -0.3, 0.1, 0.9).normalize(),
            Vector3D::new(7.0, -11.0, 5.0),
        ),
    ] {
        for scale in [
            Vector3D::new(-1.0, 1.0, 1.0),
            Vector3D::new(2.0, 3.0, 4.0),
            Vector3D::new(-2.0, 3.0, 4.0),
            Vector3D::new(2.0, -3.0, 4.0),
            Vector3D::new(2.0, 3.0, -4.0),
            Vector3D::new(-2.0, -3.0, 4.0),
            Vector3D::new(-2.0, 3.0, -4.0),
            Vector3D::new(2.0, -3.0, -4.0),
            Vector3D::new(-2.0, -3.0, -4.0),
        ] {
            let original = Matrix4x4::from_scale_rotation_translation(scale, rotation, translation);
            let (scale, rotation, position) = original.to_scale_rotation_translation();
            assert_eq!(position, translation);
            assert_abs_diff_eq!(rotation.dot(rotation), 1.0, epsilon = 1e-5);
            assert_matrix_close(
                Matrix4x4::from_scale_rotation_translation(scale, rotation, position),
                original,
            );

            let (position, rotation, scale) = transform::decompose_matrix(original);
            assert_eq!(position, translation);
            assert_abs_diff_eq!(rotation.dot(rotation), 1.0, epsilon = 1e-5);
            assert_matrix_close(
                transform::compose_matrix(position, rotation, scale),
                original,
            );
        }
    }
}

#[test]
fn unbounded_boxes_remain_conservative_under_identity_and_translation() {
    for matrix in [
        Matrix4x4::IDENTITY,
        Matrix4x4::from_scale_rotation_translation(
            Vector3D::splat(1.0),
            Quaternion::IDENTITY,
            Vector3D::new(7.0, -11.0, 5.0),
        ),
    ] {
        assert_eq!(AABB::empty().transformed(&matrix), AABB::empty());
        for bounds in [
            AABB::infinite(),
            AABB::new(
                Vector3D::new(f32::NEG_INFINITY, -2.0, -3.0),
                Vector3D::new(1.0, 2.0, 3.0),
            ),
            AABB::new(
                Vector3D::new(-1.0, -2.0, -3.0),
                Vector3D::new(1.0, 2.0, f32::INFINITY),
            ),
        ] {
            let transformed = bounds.transformed(&matrix);
            assert_eq!(transformed, AABB::infinite());
            assert!(transformed.contains_point(Vector3D::ZERO));
            assert!(transformed.contains_point(Vector3D::new(-1e6, 2e6, -3e6)));
        }
    }
}

#[test]
fn affine_box_transform_matches_independent_bounds() {
    let bounds = AABB::new(
        Vector3D::new(-1.0, -2.0, -3.0),
        Vector3D::new(4.0, 5.0, 6.0),
    );
    let matrix = Matrix4x4::from_cols(
        Vector4D::new(0.0, -2.0, 0.0, 0.0),
        Vector4D::new(-3.0, 0.0, 0.0, 0.0),
        Vector4D::new(0.0, 0.0, 0.5, 0.0),
        Vector4D::new(7.0, -11.0, 5.0, 1.0),
    );
    assert_eq!(
        bounds.transformed(&matrix),
        AABB::new(
            Vector3D::new(-8.0, -19.0, 3.5),
            Vector3D::new(13.0, -9.0, 8.0),
        )
    );
}

#[test]
fn projected_boxes_crossing_or_touching_w_zero_are_unbounded() {
    let projection = Matrix4x4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 1.0, 10.0);
    assert_eq!(AABB::empty().transformed(&projection), AABB::empty());
    for (min_z, max_z) in [(-1.0, 1.0), (-1.0, 0.0), (0.0, 1.0), (0.0, 0.0)] {
        let bounds = AABB::new(
            Vector3D::new(-1.0, -1.0, min_z),
            Vector3D::new(1.0, 1.0, max_z),
        );
        assert_eq!(bounds.transformed(&projection), AABB::infinite());
    }

    let bounds = AABB::new(Vector3D::splat(-1.0), Vector3D::splat(1.0));
    let point = Vector3D::new(1.0, 0.0, -0.01);
    assert!(bounds.contains_point(point));
    assert!(
        bounds
            .transformed(&projection)
            .contains_point(projection.transform_point3(point))
    );
}

#[test]
fn constant_sign_projective_bounds_contain_interior_points() {
    let projection = Matrix4x4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 1.0, 10.0);
    for (min_z, max_z) in [(-4.0, -2.0), (2.0, 4.0)] {
        let bounds = AABB::new(
            Vector3D::new(-1.0, -1.0, min_z),
            Vector3D::new(1.0, 1.0, max_z),
        );
        let projected = bounds.transformed(&projection);
        for endpoint in [projected.min, projected.max] {
            assert!(endpoint.x.is_finite() && endpoint.y.is_finite() && endpoint.z.is_finite());
        }
        for x in [-1.0, 0.0, 1.0] {
            for y in [-1.0, 0.0, 1.0] {
                for z in [min_z, (min_z + max_z) * 0.5, max_z] {
                    let point = projection.transform_point3(Vector3D::new(x, y, z));
                    assert!(projected.contains_point(point));
                }
            }
        }
    }
}

#[test]
fn vector3_comparison_uses_xyz_lexicographic_order() {
    for (a, b) in [
        (
            Vector3D::new(0.0, 100.0, 100.0),
            Vector3D::new(1.0, 0.0, 0.0),
        ),
        (Vector3D::new(1.0, 0.0, 100.0), Vector3D::new(1.0, 1.0, 0.0)),
        (Vector3D::new(1.0, 2.0, 3.0), Vector3D::new(1.0, 2.0, 4.0)),
    ] {
        assert!(math::vector3_less_than(a, b));
        assert!(!math::vector3_less_than(b, a));
        assert!(!math::vector3_less_than(a, a));
    }
}

#[test]
fn rgb_hsv_round_trips_cover_all_six_hue_sectors() {
    for (rgb, expected_hue) in [
        (Color3D::new(1.0, 0.0, 0.0), 0.0),
        (Color3D::new(1.0, 0.5, 0.0), 30.0),
        (Color3D::new(1.0, 1.0, 0.0), 60.0),
        (Color3D::new(0.5, 1.0, 0.0), 90.0),
        (Color3D::new(0.0, 1.0, 0.0), 120.0),
        (Color3D::new(0.0, 1.0, 0.5), 150.0),
        (Color3D::new(0.0, 1.0, 1.0), 180.0),
        (Color3D::new(0.0, 0.5, 1.0), 210.0),
        (Color3D::new(0.0, 0.0, 1.0), 240.0),
        (Color3D::new(0.5, 0.0, 1.0), 270.0),
        (Color3D::new(1.0, 0.0, 1.0), 300.0),
        (Color3D::new(1.0, 0.0, 0.5), 330.0),
        (Color3D::new(0.75, 0.25, 0.5), 330.0),
        (Color3D::ZERO, 0.0),
        (Color3D::splat(0.25), 0.0),
        (Color3D::splat(1.0), 0.0),
    ] {
        let hsv = color::rgb_to_hsv(rgb);
        assert!((0.0..360.0).contains(&hsv.x));
        assert_abs_diff_eq!(hsv.x, expected_hue, epsilon = 1e-4);
        assert_vector_close(color::hsv_to_rgb(hsv), rgb);
    }
}

#[test]
fn hsv_hue_wraps_across_positive_and_negative_turns() {
    for (hue, expected) in [
        (0.0, Color3D::new(0.8, 0.4, 0.4)),
        (60.0, Color3D::new(0.8, 0.8, 0.4)),
        (180.0, Color3D::new(0.4, 0.8, 0.8)),
        (300.0, Color3D::new(0.8, 0.4, 0.8)),
    ] {
        for turns in [-2.0, -1.0, 0.0, 1.0, 2.0] {
            let hsv = Vector3D::new(hue + turns * 360.0, 0.5, 0.8);
            assert_vector_close(color::hsv_to_rgb(hsv), expected);
        }
    }
}
