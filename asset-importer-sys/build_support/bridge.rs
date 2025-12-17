use crate::build_support::{config::BuildConfig, plan::BuildPlan};

pub fn build(cfg: &BuildConfig, plan: &BuildPlan) {
    let mut build = cc::Build::new();
    build.cpp(true);
    build.std("c++17");
    build.file(cfg.manifest_dir.join("wrapper.cpp"));

    for dir in &plan.include_dirs {
        build.include(dir);
    }

    configure_cpp_flags(&mut build, cfg);
    build.compile("assimp_rust_bridge");
}

fn configure_cpp_flags(build: &mut cc::Build, cfg: &BuildConfig) {
    if cfg.is_windows() && cfg.is_msvc() {
        build.flag("/EHsc");

        // Match Rust's CRT family, but never use the MSVC debug CRT (Rust doesn't link it).
        let use_static = cfg.use_static_crt();
        build.static_crt(use_static);
        if use_static {
            build.flag("/MT");
        } else {
            build.flag("/MD");
        }

        // Keep debug info without toggling CRT flavor.
        if cfg.is_debug() {
            build.debug(true);
            build.opt_level(0);
        } else {
            build.debug(false);
            build.opt_level(2);
        }

        build.flag("/D_ITERATOR_DEBUG_LEVEL=0");
        return;
    }

    if cfg.is_macos() {
        build.flag(format!(
            "-mmacosx-version-min={}",
            cfg.macos_deployment_target()
        ));
    }
}
