//! Inspect mesh/morph animation channels and anim meshes
use asset_importer::{postprocess::PostProcessSteps, Animation, Importer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <model_file>", args[0]);
        std::process::exit(1);
    }
    let model = &args[1];

    let scene = Importer::new()
        .read_file(model)
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .import_file(model)?;

    println!("Meshes: {}", scene.num_meshes());
    for (i, mesh) in scene.meshes().enumerate() {
        println!("\nMesh {}: {} anim meshes", i, mesh.num_anim_meshes());
        for (j, am) in mesh.anim_meshes().enumerate() {
            println!(
                "  AnimMesh {}: name='{}' verts={} weight={:.3}",
                j,
                am.name(),
                am.num_vertices(),
                am.weight()
            );
        }
    }

    println!("\nAnimations: {}", scene.num_animations());
    for (ai, anim) in scene.animations().enumerate() {
        println!("\nAnimation {}:", ai);

        // Mesh animation channels (aiMeshAnim)
        println!("  Mesh channels: {}", anim.num_mesh_channels());
        for (ci, ch) in MeshChannels::new(&anim).enumerate() {
            println!(
                "    Channel {} name='{}' keys={}",
                ci,
                ch.name(),
                ch.num_keys()
            );
            for (ki, key) in ch.keys().iter().enumerate().take(4) {
                println!(
                    "      key[{}]: time={:.3} animMeshIndex={}",
                    ki, key.time, key.value
                );
            }
            if ch.num_keys() > 4 {
                println!("      ...");
            }
        }

        // Morph mesh animation channels (aiMeshMorphAnim)
        println!("  Morph channels: {}", anim.num_morph_mesh_channels());
        for (ci, ch) in MorphChannels::new(&anim).enumerate() {
            println!(
                "    MorphChannel {} name='{}' keys={}",
                ci,
                ch.name(),
                ch.num_keys()
            );
            if let Some(key0) = ch.key(0) {
                println!(
                    "      key[0]: time={:.3} values={} weights={}",
                    key0.time,
                    key0.values.len(),
                    key0.weights.len()
                );
                for k in 0..key0.values.len().min(4) {
                    println!(
                        "        idx={} weight={:.3}",
                        key0.values[k], key0.weights[k]
                    );
                }
                if key0.values.len() > 4 {
                    println!("        ...");
                }
            }
        }
    }

    Ok(())
}

// Small wrappers to make iteration in example ergonomic
struct MeshChannels<'a> {
    anim: &'a Animation,
    idx: usize,
}
impl<'a> MeshChannels<'a> {
    fn new(anim: &'a Animation) -> Self {
        Self { anim, idx: 0 }
    }
}
impl<'a> Iterator for MeshChannels<'a> {
    type Item = asset_importer::animation::MeshAnimation;
    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.anim.mesh_channel(self.idx)?;
        self.idx += 1;
        Some(ch)
    }
}

struct MorphChannels<'a> {
    anim: &'a Animation,
    idx: usize,
}
impl<'a> MorphChannels<'a> {
    fn new(anim: &'a Animation) -> Self {
        Self { anim, idx: 0 }
    }
}
impl<'a> Iterator for MorphChannels<'a> {
    type Item = asset_importer::animation::MorphMeshAnimation;
    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.anim.morph_mesh_channel(self.idx)?;
        self.idx += 1;
        Some(ch)
    }
}
