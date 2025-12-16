mod build_support;

use build_support::{bindings, bridge, config::BuildConfig, plan, system_deps};

fn main() {
    let cfg = BuildConfig::new();
    cfg.emit_rerun_triggers();

    if cfg.docs_rs {
        bindings::run_docsrs(&cfg);
        return;
    }

    let plan = plan::resolve(&cfg);

    // Ensure the Rust binary links against the correct C++ runtime / platform deps.
    system_deps::emit(&cfg);

    // Link Assimp (system / prebuilt / built-from-source).
    plan.emit_link(&cfg);

    // Generate Rust bindings (always from headers that match the chosen link strategy).
    bindings::run(&cfg, &plan);

    // Build our small C++ bridge (progress handler + IOSystem wrappers).
    bridge::build(&cfg, &plan);
}
