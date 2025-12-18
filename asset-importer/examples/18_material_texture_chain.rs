//! Material texture chain: material slot -> path -> (optional) embedded texture -> bytes/texels.
//!
//! This example demonstrates:
//! - `Material::texture_refs(TextureType)` (no allocation for the path)
//! - detecting embedded textures by `"*0"`, `"*1"`, ...
//! - `Scene::embedded_texture_by_name` + `Texture::data_ref()` for zero-copy access
//!
//! Usage:
//!   cargo run -p asset-importer --example 18_material_texture_chain --no-default-features --features build-assimp -- <model>

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

use asset_importer::{
    material::TextureType, postprocess::PostProcessSteps, texture::TextureDataRef,
};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();

    let path = common::resolve_model_path(
        common::ModelSource::ArgOrExamplesDir,
        "cube_with_materials.obj",
    );
    let scene = common::import_scene(&path, PostProcessSteps::empty())?;

    println!("Loaded: {}", path.display());
    println!(
        "Materials: {}  Textures(embedded): {}",
        scene.num_materials(),
        scene.num_textures()
    );

    let interesting = [
        TextureType::BaseColor,
        TextureType::Diffuse,
        TextureType::Normals,
        TextureType::Metalness,
        TextureType::DiffuseRoughness,
        TextureType::AmbientOcclusion,
        TextureType::EmissionColor,
    ];

    for (mi, mat) in scene.materials().enumerate() {
        let mut any = false;
        for ty in interesting {
            let count = mat.texture_count(ty);
            if count == 0 {
                continue;
            }
            any = true;

            println!("Material #{mi} type={:?} count={}", ty, count);
            for (ti, info) in mat.texture_refs(ty).enumerate() {
                let path_str = info.path_str();
                let path_str = path_str.as_ref();
                println!(
                    "  [{ti}] path={} uv={} blend={:.3}",
                    path_str, info.uv_index, info.blend_factor
                );

                if let Some(embedded_name) = path_str.strip_prefix('*') {
                    let embedded = format!("*{}", embedded_name);
                    match scene.embedded_texture_by_name(&embedded)? {
                        None => println!("       embedded: not found"),
                        Some(tex) => {
                            let hint = tex.format_hint_str();
                            match tex.data_ref()? {
                                TextureDataRef::Compressed(bytes) => {
                                    println!(
                                        "       embedded: compressed bytes={} hint={}",
                                        bytes.len(),
                                        hint
                                    );
                                }
                                TextureDataRef::Texels(texels) => {
                                    println!(
                                        "       embedded: texels={} hint={}",
                                        texels.len(),
                                        hint
                                    );
                                }
                            }
                        }
                    }
                } else {
                    println!("       external: {}", path_str);
                }
            }
        }

        if any {
            println!("---");
        }
    }

    common::shutdown_logging();
    Ok(())
}
