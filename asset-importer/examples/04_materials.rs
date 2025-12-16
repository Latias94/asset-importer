//! Inspect materials: high-level fields and raw/typed properties

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

use asset_importer::{
    material::{PropertyTypeInfo, material_keys},
    postprocess::PostProcessSteps,
};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();
    let path = common::resolve_model_path(common::ModelSource::ArgOrExamplesDir, "pbr_sphere.gltf");
    let scene = common::import_scene(&path, PostProcessSteps::empty())?;

    println!("Materials: {}", scene.num_materials());
    for (i, mat) in scene.materials().enumerate() {
        println!("\n== Material[{}] ==", i);
        println!("name: {}", mat.name());
        if let Some(c) = mat.diffuse_color() {
            println!("diffuse: [{:.3}, {:.3}, {:.3}]", c.x, c.y, c.z);
        }
        if let Some(c) = mat.specular_color() {
            println!("specular: [{:.3}, {:.3}, {:.3}]", c.x, c.y, c.z);
        }
        if let Some(v) = mat.get_float_property(material_keys::OPACITY) {
            println!("opacity: {:.3}", v);
        }
        if let Some(v) = mat.blend_mode() {
            println!("blend_mode: {:?}", v);
        }

        // Texture summary
        for ty in [
            asset_importer::material::TextureType::Diffuse,
            asset_importer::material::TextureType::BaseColor,
            asset_importer::material::TextureType::Normals,
            asset_importer::material::TextureType::GltfMetallicRoughness,
        ] {
            let n = mat.texture_count(ty);
            if n > 0 {
                println!("textures.{:?} = {}", ty, n);
            }
        }

        // Raw/typed properties
        for p in mat.all_properties().into_iter().take(64) {
            let sem = p
                .semantic
                .map(|t| format!("{:?}", t))
                .unwrap_or_else(|| "-".into());
            print!(
                "  key='{}' sem={} idx={} len={} type={:?}",
                p.key, sem, p.index, p.data_length, p.type_info
            );
            match p.type_info {
                PropertyTypeInfo::String => {
                    if let Some(s) = mat.get_string_property_str(&p.key) {
                        println!(" value=\"{}\"", s);
                    } else {
                        println!();
                    }
                }
                PropertyTypeInfo::Integer => {
                    if let Some(v) = mat.get_property_i32_array_str(&p.key, p.semantic, p.index) {
                        println!(" ints={:?}", preview(&v[..]));
                    } else {
                        println!();
                    }
                }
                PropertyTypeInfo::Float => {
                    if let Some(v) = mat.get_property_f32_array_str(&p.key, p.semantic, p.index) {
                        println!(" floats={:?}", preview(&v[..]));
                    } else {
                        println!();
                    }
                }
                PropertyTypeInfo::Double => {
                    if let Some(v) = mat.get_property_f64_array_str(&p.key, p.semantic, p.index) {
                        println!(" doubles={:?}", preview(&v[..]));
                    } else {
                        println!();
                    }
                }
                PropertyTypeInfo::Buffer | PropertyTypeInfo::Unknown(_) => {
                    if let Some(raw) = mat.get_property_raw_str(&p.key, p.semantic, p.index) {
                        println!(" raw[{}]={:?}", raw.len(), preview(&raw));
                    } else {
                        println!();
                    }
                }
            }
        }
    }

    common::shutdown_logging();
    Ok(())
}

fn preview<T: std::fmt::Debug>(v: &[T]) -> String {
    const N: usize = 6;
    if v.len() <= N {
        format!("{:?}", v)
    } else {
        format!("{:?} .. ({} items)", &v[..N], v.len())
    }
}
