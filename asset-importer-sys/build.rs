mod build_support;

use build_support::{bindings, bridge, config::BuildConfig, plan, system_deps};

fn main() {
    let cfg = BuildConfig::new();
    cfg.emit_rerun_triggers();

    println!("cargo:rustc-env=ASSET_IMPORTER_BUILD_TARGET={}", cfg.target);
    println!(
        "cargo:rustc-env=ASSET_IMPORTER_BUILD_CRT={}",
        if cfg.is_msvc() {
            if cfg.use_static_crt() { "mt" } else { "md" }
        } else {
            ""
        }
    );

    if cfg.docs_rs {
        bindings::run_docsrs(&cfg);
        return;
    }

    let plan = plan::resolve(&cfg);

    // Ensure the Rust binary links against the correct C++ runtime / platform deps.
    system_deps::emit(&cfg);

    // Link Assimp (system / prebuilt / built-from-source).
    plan.emit_link(&cfg);

    // Use shipped bindings, or generate bindings from the selected headers when requested.
    bindings::run(&cfg, &plan);

    // Build our small C++ bridge (progress handler + IOSystem wrappers).
    bridge::build(&cfg, &plan);
}
