//! Skinning weights: build per-vertex influence stats from bones (zero-copy).
//!
//! This example prints:
//! - per-mesh bone count
//! - per-vertex influence count distribution
//! - vertices whose weight sum deviates from 1.0 (common data issue)
//!
//! Usage:
//!   cargo run -p asset-importer --example 16_skinning_weights --no-default-features --features build-assimp -- <rigged_model>

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

use asset_importer::postprocess::PostProcessSteps;

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();

    let path = common::resolve_model_path(common::ModelSource::ArgOnly, "unused");
    let scene = common::import_scene(&path, PostProcessSteps::empty())?;

    println!("Loaded: {}", path.display());
    println!("Meshes: {}", scene.num_meshes());

    let mut any_bones = false;
    for (mesh_index, mesh) in scene.meshes().enumerate() {
        if !mesh.has_bones() {
            continue;
        }
        any_bones = true;

        let vertex_count = mesh.num_vertices();
        let bone_count = mesh.num_bones();

        let mut influences: Vec<u16> = vec![0; vertex_count];
        let mut sums: Vec<f32> = vec![0.0; vertex_count];

        for bone in mesh.bones() {
            for w in bone.weights_raw() {
                let vid = w.mVertexId as usize;
                if vid >= vertex_count {
                    continue;
                }
                influences[vid] = influences[vid].saturating_add(1);
                sums[vid] += w.mWeight;
            }
        }

        let mut max_influences = 0u16;
        let mut histogram: [usize; 9] = [0; 9]; // 0..=7 and 8+
        for &n in &influences {
            max_influences = max_influences.max(n);
            let bin = if n as usize >= 8 { 8 } else { n as usize };
            histogram[bin] += 1;
        }

        let eps = 1e-3f32;
        let mut bad: Vec<(usize, u16, f32)> = Vec::new();
        for (i, (&n, &sum)) in influences.iter().zip(sums.iter()).enumerate() {
            if n == 0 {
                continue;
            }
            if (sum - 1.0).abs() > eps {
                bad.push((i, n, sum));
            }
        }
        bad.sort_by(|a, b| (b.2 - 1.0).abs().partial_cmp(&(a.2 - 1.0).abs()).unwrap());

        println!(
            "Mesh #{mesh_index} name={} vertices={} bones={}",
            mesh.name_str(),
            vertex_count,
            bone_count
        );
        println!("  max influences per vertex: {}", max_influences);
        println!(
            "  influence histogram: 0={} 1={} 2={} 3={} 4={} 5={} 6={} 7={} 8+={}",
            histogram[0],
            histogram[1],
            histogram[2],
            histogram[3],
            histogram[4],
            histogram[5],
            histogram[6],
            histogram[7],
            histogram[8],
        );

        if bad.is_empty() {
            println!("  weight sums: OK (within eps={})", eps);
        } else {
            println!(
                "  weight sums: {} vertex(es) deviate from 1.0 (showing up to 10)",
                bad.len()
            );
            for (i, n, sum) in bad.iter().take(10) {
                println!("    vtx={} influences={} sum={:.6}", i, n, sum);
            }
        }
    }

    if !any_bones {
        println!("No rigged meshes with bones found in this model.");
    }

    common::shutdown_logging();
    Ok(())
}
