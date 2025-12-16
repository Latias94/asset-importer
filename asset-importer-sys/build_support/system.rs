use crate::build_support::{
    config::BuildConfig,
    plan::{BuildMethod, BuildPlan, LinkKind},
    util,
};

pub fn probe(cfg: &BuildConfig) -> BuildPlan {
    if cfg.is_windows() && cfg.is_msvc() {
        let lib = vcpkg::Config::new()
            .emit_includes(true)
            .find_package("assimp")
            .unwrap_or_else(|e| {
                panic!(
                    "system linking (vcpkg) failed: {e}\n\
                     Hint: install assimp via vcpkg and set VCPKG_ROOT."
                )
            });

        let include_dirs = lib.include_paths.iter().cloned().collect::<Vec<_>>();

        if include_dirs.is_empty() {
            util::warn("vcpkg returned no include paths for assimp; bindgen may fail");
        }

        return BuildPlan {
            include_dirs,
            link_kind: LinkKind::Dynamic,
            link_lib: None, // vcpkg emits all rustc link flags
            link_search: Vec::new(),
            method: BuildMethod::System,
        };
    }

    let lib = pkg_config::Config::new()
        .statik(false)
        .probe("assimp")
        .unwrap_or_else(|e| {
            panic!(
                "system linking (pkg-config) failed: {e}\n\
                 Hint: install assimp and ensure pkg-config can find assimp.pc."
            )
        });

    let include_dirs = lib.include_paths.iter().cloned().collect::<Vec<_>>();

    if include_dirs.is_empty() {
        util::warn("pkg-config returned no include paths for assimp; bindgen may fail");
    }

    BuildPlan {
        include_dirs,
        link_kind: LinkKind::Dynamic,
        link_lib: None, // pkg-config emits all rustc link flags
        link_search: Vec::new(),
        method: BuildMethod::System,
    }
}
