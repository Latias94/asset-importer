//! Embedded textures: iterate compressed/uncompressed textures and dump compressed ones to disk.
//!
//! Usage:
//!   cargo run -p asset-importer --example 15_embedded_textures --no-default-features --features build-assimp -- <model>

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

use asset_importer::{postprocess::PostProcessSteps, texture::TextureDataRef};

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();

    let path = common::resolve_model_path(
        common::ModelSource::ArgOrExamplesDir,
        "cube_with_materials.obj",
    );
    let scene = common::import_scene(&path, PostProcessSteps::empty())?;

    println!("Loaded: {}", path.display());
    println!("Embedded textures: {}", scene.num_textures());

    if !scene.has_textures() {
        println!("No embedded textures found.");
        common::shutdown_logging();
        return Ok(());
    }

    let out_dir = std::env::temp_dir().join("asset-importer-textures");
    std::fs::create_dir_all(&out_dir)?;
    println!("Output dir: {}", out_dir.display());

    for (i, tex) in scene.textures().enumerate() {
        let name = tex
            .filename_str()
            .map(|s| s.into_owned())
            .unwrap_or_else(|| format!("texture_{i}"));
        let hint = tex.format_hint_str();
        let (w, h) = tex.dimensions();

        println!(
            "Texture #{i}: name={} hint={} dims=({}, {}) compressed={}",
            name,
            hint,
            w,
            h,
            tex.is_compressed()
        );

        match tex.data_ref()? {
            TextureDataRef::Compressed(bytes) => {
                let ext = if hint.is_empty() {
                    "bin"
                } else {
                    hint.as_ref()
                };
                let dst = out_dir.join(sanitize_filename(&format!("{i}_{name}.{ext}")));
                std::fs::write(&dst, bytes)?;
                println!("  wrote {} bytes -> {}", bytes.len(), dst.display());
            }
            TextureDataRef::Texels(texels) => {
                // For uncompressed textures Assimp provides ARGB8888 texels.
                // Encoding to PNG/JPEG is intentionally not included in this crate.
                println!("  uncompressed texels: {} (ARGB8888)", texels.len());
                if let Some(t0) = texels.first() {
                    println!(
                        "  first texel (b,g,r,a)=({},{},{},{})",
                        t0.b, t0.g, t0.r, t0.a
                    );
                }
            }
        }
    }

    common::shutdown_logging();
    Ok(())
}

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}
