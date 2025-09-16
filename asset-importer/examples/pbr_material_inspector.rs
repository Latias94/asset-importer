//! Inspect PBR material properties and textures
use asset_importer::{postprocess::PostProcessSteps, Importer, TextureType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <model_file>", args[0]);
        std::process::exit(1);
    }
    let model = &args[1];

    let scene = Importer::new()
        .read_file(model)
        .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::GEN_SMOOTH_NORMALS)
        .import_file(model)?;

    println!("Materials: {}", scene.num_materials());
    for (i, mat) in scene.materials().enumerate() {
        println!("\nMaterial {}: {}", i, mat.name());

        if let Some(c) = mat.base_color() {
            println!("  BaseColor: ({:.3},{:.3},{:.3},{:.3})", c.x, c.y, c.z, c.w);
        }
        if let Some(v) = mat.metallic_factor() {
            println!("  Metallic: {:.3}", v);
        }
        if let Some(v) = mat.roughness_factor() {
            println!("  Roughness: {:.3}", v);
        }
        if let Some(v) = mat.glossiness_factor() {
            println!("  Glossiness: {:.3}", v);
        }
        if let Some(v) = mat.specular_factor() {
            println!("  SpecularFactor: {:.3}", v);
        }
        if let Some(c) = mat.sheen_color_factor() {
            println!(
                "  SheenColor: ({:.3},{:.3},{:.3},{:.3})",
                c.x, c.y, c.z, c.w
            );
        }
        if let Some(v) = mat.sheen_roughness_factor() {
            println!("  SheenRoughness: {:.3}", v);
        }
        if let Some(v) = mat.clearcoat_factor() {
            println!("  Clearcoat: {:.3}", v);
        }
        if let Some(v) = mat.clearcoat_roughness_factor() {
            println!("  ClearcoatRoughness: {:.3}", v);
        }
        if let Some(v) = mat.transmission_factor() {
            println!("  Transmission: {:.3}", v);
        }
        if let Some(v) = mat.volume_thickness_factor() {
            println!("  VolumeThickness: {:.3}", v);
        }
        if let Some(v) = mat.volume_attenuation_distance() {
            println!("  VolumeAttenuationDistance: {:.3}", v);
        }
        if let Some(c) = mat.volume_attenuation_color() {
            println!(
                "  VolumeAttenuationColor: ({:.3},{:.3},{:.3})",
                c.x, c.y, c.z
            );
        }
        if let Some(v) = mat.emissive_intensity() {
            println!("  EmissiveIntensity: {:.3}", v);
        }
        if let Some(v) = mat.anisotropy_factor() {
            println!("  AnisotropyFactor: {:.3}", v);
        }
        if let Some(v) = mat.anisotropy_rotation() {
            println!("  AnisotropyRotation: {:.3}", v);
        }

        // Print common PBR textures if present
        let pbr_textures = [
            (TextureType::BaseColor, "BaseColorTex"),
            (TextureType::GltfMetallicRoughness, "MetalRoughTex"),
            (TextureType::Metalness, "MetalnessTex"),
            (TextureType::DiffuseRoughness, "RoughnessTex"),
            (TextureType::EmissionColor, "EmissiveTex"),
            (TextureType::Clearcoat, "ClearcoatTex"),
            (TextureType::Sheen, "SheenTex"),
            (TextureType::Transmission, "TransmissionTex"),
            (TextureType::Anisotropy, "AnisotropyTex"),
            (TextureType::Normals, "NormalTex"),
        ];
        for (ty, label) in pbr_textures.iter() {
            let n = mat.texture_count(*ty);
            if n == 0 {
                continue;
            }
            println!("  {} count: {}", label, n);
            for j in 0..n {
                if let Some(info) = mat.texture(*ty, j) {
                    print!("    {}: {}", j, info.path);
                    if !info.flags.is_empty() {
                        print!(" flags={:?}", info.flags);
                    }
                    println!();
                    if let Some(t) = info.uv_transform {
                        println!(
                            "      UVTransform: trans=({:.2},{:.2}) scale=({:.2},{:.2}) rot={:.2}",
                            t.translation.x, t.translation.y, t.scaling.x, t.scaling.y, t.rotation
                        );
                    }
                    if let Some(a) = info.axis {
                        println!("      Axis: ({:.2},{:.2},{:.2})", a.x, a.y, a.z);
                    }
                }
            }
        }
    }

    Ok(())
}
