//! Inspect animations: channels, interpolation, behaviours, and key previews

#[path = "common/mod.rs"]
mod common;

use std::error::Error;

use asset_importer::postprocess::PostProcessSteps;

fn main() -> Result<(), Box<dyn Error>> {
    common::init_logging_from_env();
    let path = common::resolve_model_path(common::ModelSource::ArgOrExamplesDir, "morph_mesh.glb");
    let scene = common::import_scene(&path, PostProcessSteps::empty())?;

    println!("Animations: {}", scene.num_animations());
    for (ai, anim) in scene.animations().enumerate() {
        println!("\n== Animation[{}] ==", ai);
        println!(
            "name='{}' duration_ticks={:.3} ticks_per_second={:.3}",
            anim.name(),
            anim.duration(),
            anim.ticks_per_second()
        );

        println!("Node channels: {}", anim.num_channels());
        for (ci, ch) in anim.channels().enumerate().take(8) {
            println!(
                "  Channel[{}]: node='{}' pos_keys={} rot_keys={} scale_keys={} pre={:?} post={:?}",
                ci,
                ch.node_name(),
                ch.num_position_keys(),
                ch.num_rotation_keys(),
                ch.num_scaling_keys(),
                ch.pre_state(),
                ch.post_state()
            );
            let pk = ch.position_keys();
            if let Some(k) = pk.first() {
                println!(
                    "    pos[0]: t={:.3} v=({:.3},{:.3},{:.3}) {:?}",
                    k.time, k.value.x, k.value.y, k.value.z, k.interpolation
                );
            }
            let rk = ch.rotation_keys();
            if let Some(k) = rk.first() {
                println!(
                    "    rot[0]: t={:.3} q=({:.3},{:.3},{:.3},{:.3}) {:?}",
                    k.time, k.value.x, k.value.y, k.value.z, k.value.w, k.interpolation
                );
            }
            let sk = ch.scaling_keys();
            if let Some(k) = sk.first() {
                println!(
                    "    scale[0]: t={:.3} v=({:.3},{:.3},{:.3}) {:?}",
                    k.time, k.value.x, k.value.y, k.value.z, k.interpolation
                );
            }
        }

        println!("Morph mesh channels: {}", anim.num_morph_mesh_channels());
        for (ci, ch) in anim.morph_mesh_channels().enumerate().take(8) {
            println!(
                "  Morph[{}]: name='{}' keys={}",
                ci,
                ch.name(),
                ch.num_keys()
            );
            if let Some(k0) = ch.key(0) {
                let values_len = k0.values().len();
                let weights_len = k0.weights().len();
                println!(
                    "    key0: time={:.3} values={} weights={}",
                    k0.time(),
                    values_len,
                    weights_len
                );
            }
        }
    }

    common::shutdown_logging();
    Ok(())
}
