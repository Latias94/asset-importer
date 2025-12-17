//! glam integration demo (requires `--features glam`).

use std::error::Error;

#[cfg(not(feature = "glam"))]
fn main() -> Result<(), Box<dyn Error>> {
    eprintln!("This example requires the 'glam' feature.");
    eprintln!(
        "Run with: cargo run -p asset-importer --example 11_glam_integration --features glam"
    );
    std::process::exit(1);
}

#[cfg(feature = "glam")]
fn main() -> Result<(), Box<dyn Error>> {
    use asset_importer::{Matrix4x4, Quaternion, Vector2D, Vector3D, Vector4D};

    let v2 = Vector2D::new(1.0, 2.0);
    let gv2: glam::Vec2 = v2.into();
    let back_v2: Vector2D = gv2.into();
    assert_eq!(v2, back_v2);

    let v3 = Vector3D::new(1.0, 2.0, 3.0);
    let gv3: glam::Vec3 = v3.into();
    let back_v3: Vector3D = gv3.into();
    assert_eq!(v3, back_v3);

    let v4 = Vector4D::new(1.0, 2.0, 3.0, 4.0);
    let gv4: glam::Vec4 = v4.into();
    let back_v4: Vector4D = gv4.into();
    assert_eq!(v4, back_v4);

    let q = Quaternion::from_xyzw(0.0, 0.38268343, 0.0, 0.9238795);
    let gq: glam::Quat = q.into();
    let back_q: Quaternion = gq.into();
    // Quaternion equivalence can have sign ambiguity; compare absolute dot.
    let dot = q.dot(back_q).abs();
    assert!(dot > 1.0 - f32::EPSILON);

    let m = Matrix4x4::IDENTITY;
    let gm: glam::Mat4 = m.into();
    let back_m: Matrix4x4 = gm.into();
    assert_eq!(m, back_m);

    println!("glam conversions OK.");
    Ok(())
}
