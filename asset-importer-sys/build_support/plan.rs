use std::path::PathBuf;

use crate::build_support::{config::BuildConfig, util};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    Static,
    Dynamic,
}

#[derive(Clone, Debug)]
pub struct BuildPlan {
    pub include_dirs: Vec<PathBuf>,
    pub link_kind: LinkKind,
    pub link_lib: Option<String>,
    pub link_search: Vec<PathBuf>,
    pub method: BuildMethod,
}

#[derive(Clone, Debug)]
pub enum BuildMethod {
    #[cfg(feature = "system")]
    System,
    #[cfg(feature = "prebuilt")]
    Prebuilt,
    Vendored,
}

impl BuildPlan {
    pub fn emit_link(&self, cfg: &BuildConfig) {
        for p in &self.link_search {
            println!("cargo:rustc-link-search=native={}", p.display());
        }
        if let Some(lib) = &self.link_lib {
            match self.link_kind {
                LinkKind::Static => println!("cargo:rustc-link-lib=static={}", lib),
                LinkKind::Dynamic => println!("cargo:rustc-link-lib={}", lib),
            }
        }

        // Expose include paths to downstream build scripts (DEP_ASSIMP_INCLUDE / DEP_ASSIMP_INCLUDE_PATHS).
        if let Some(first) = self.include_dirs.first() {
            println!("cargo:include={}", first.display());
        }
        if let Some(joined) = util::join_paths_for_env(&self.include_dirs) {
            println!("cargo:include_paths={}", joined);
        }

        // Verbose tracing for troubleshooting.
        if cfg.verbose {
            util::warn(format!(
                "Assimp plan: method={:?} link_kind={:?} lib={:?}",
                self.method, self.link_kind, self.link_lib
            ));
            for p in &self.include_dirs {
                util::warn(format!("Assimp include: {}", p.display()));
            }
            for p in &self.link_search {
                util::warn(format!("Assimp link search: {}", p.display()));
            }
        }
    }
}

pub fn resolve(cfg: &BuildConfig) -> BuildPlan {
    let link_kind = if cfg!(feature = "static-link") {
        LinkKind::Static
    } else {
        LinkKind::Dynamic
    };

    if cfg!(feature = "system") {
        #[cfg(not(feature = "system"))]
        {
            unreachable!("feature gate mismatch");
        }
        #[cfg(feature = "system")]
        {
            if matches!(link_kind, LinkKind::Static) {
                util::warn(
                    "feature `static-link` is ignored with `system` linking; using dynamic system lib",
                );
            }
            return crate::build_support::system::probe(cfg);
        }
    }

    // Environment override: force a local source build even when `prebuilt` is enabled.
    if cfg.force_build {
        return crate::build_support::vendored::build(cfg, link_kind);
    }

    // If build-from-source is explicitly requested, it always wins over prebuilt (even if prebuilt is enabled
    // through default features in the high-level crate).
    if cfg!(feature = "build-assimp") {
        return crate::build_support::vendored::build(cfg, link_kind);
    }

    if cfg!(feature = "prebuilt") {
        #[cfg(not(feature = "prebuilt"))]
        {
            unreachable!("feature gate mismatch");
        }
        #[cfg(feature = "prebuilt")]
        {
            return crate::build_support::prebuilt::prepare(cfg, link_kind);
        }
    }

    // Default for -sys: build from source (reliable, works offline).
    crate::build_support::vendored::build(cfg, link_kind)
}
