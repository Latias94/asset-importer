use crate::build_support::{
    config::BuildConfig,
    plan::{BuildMethod, BuildPlan, LinkKind},
    util,
};

pub fn probe(cfg: &BuildConfig, link_kind: LinkKind) -> BuildPlan {
    if cfg.is_windows() && cfg.is_msvc() {
        let mut vcpkg_cfg = vcpkg::Config::new();
        vcpkg_cfg.emit_includes(true);

        // Match Rust's CRT choice when possible.
        // Users can always override via VCPKGRS_TRIPLET / vcpkg env vars.
        if (cfg.use_static_crt() || matches!(link_kind, LinkKind::Static))
            && std::env::var("VCPKGRS_TRIPLET").is_err()
        {
            if let Some(triplet) = default_vcpkg_static_triplet(&cfg.target) {
                vcpkg_cfg.target_triplet(triplet);
            }
        }

        let lib = vcpkg_cfg
            .find_package("assimp")
            .unwrap_or_else(|e| {
                panic!(
                    "system linking (vcpkg) failed: {e}\n\
                     Hint: install assimp via vcpkg and set VCPKG_ROOT.\n\
                     If you're using `crt-static`, prefer a `*-windows-static` triplet (e.g. `x64-windows-static`)."
                )
            });

        let include_dirs = lib.include_paths.iter().cloned().collect::<Vec<_>>();

        if include_dirs.is_empty() {
            util::warn("vcpkg returned no include paths for assimp; bindgen may fail");
        } else {
            require_assimp_major_at_least(
                &include_dirs,
                6,
                "vcpkg",
                "Hint: install Assimp >= 6 via vcpkg (or use `--features build-assimp` / `--features prebuilt`).",
            );
        }

        return BuildPlan {
            include_dirs,
            link_kind,
            link_lib: None, // vcpkg emits all rustc link flags
            link_search: Vec::new(),
            method: BuildMethod::System,
        };
    }

    let lib = pkg_config::Config::new()
        .statik(matches!(link_kind, LinkKind::Static))
        .probe("assimp")
        .unwrap_or_else(|e| {
            panic!(
                "system linking (pkg-config) failed: {e}\n\
                 Hint: install assimp and ensure pkg-config can find assimp.pc."
            )
        });

    let required_major = 6;
    let major_from_pc = parse_major_from_version(&lib.version);

    if major_from_pc.is_some_and(|m| m < required_major) {
        panic!(
            "system assimp is too old (pkg-config reports version {}). This crate requires Assimp >= {}.\n\
             Hint: use `--features build-assimp` (vendored build), `--features prebuilt`, or install a newer Assimp and ensure pkg-config finds it.",
            lib.version, required_major
        );
    }

    let include_dirs = lib.include_paths.iter().cloned().collect::<Vec<_>>();

    if include_dirs.is_empty() {
        util::warn("pkg-config returned no include paths for assimp; bindgen may fail");

        // pkg-config may omit /usr/include-like paths. Try to double-check headers via common include roots.
        let fallback = common_include_roots();
        if !fallback.is_empty() {
            require_assimp_major_at_least(
                &fallback,
                required_major,
                "pkg-config (fallback include roots)",
                "Hint: install Assimp >= 6 (matching the headers) or use `--features build-assimp` / `--features prebuilt`.",
            );
        }
    } else {
        // pkg-config version strings can be missing or misleading on some distros;
        // double-check the headers we are about to run bindgen against.
        require_assimp_major_at_least(
            &include_dirs,
            required_major,
            "pkg-config",
            "Hint: install Assimp >= 6 (matching the headers) or use `--features build-assimp` / `--features prebuilt`.",
        );
    }

    BuildPlan {
        include_dirs,
        link_kind,
        link_lib: None, // pkg-config emits all rustc link flags
        link_search: Vec::new(),
        method: BuildMethod::System,
    }
}

fn default_vcpkg_static_triplet(target: &str) -> Option<&'static str> {
    // These are the canonical triplet names used by vcpkg for MSVC static CRT.
    // We only pick a default when targeting MSVC; MinGW uses a different toolchain.
    if target.starts_with("x86_64-") {
        Some("x64-windows-static")
    } else if target.starts_with("i686-") {
        Some("x86-windows-static")
    } else if target.starts_with("aarch64-") {
        Some("arm64-windows-static")
    } else {
        None
    }
}

fn parse_major_from_version(version: &str) -> Option<u32> {
    let first = version
        .split(|c: char| c == '.' || c == '-' || c == '+' || c == '~')
        .next()?
        .trim();
    first.parse::<u32>().ok()
}

fn require_assimp_major_at_least(
    include_dirs: &[std::path::PathBuf],
    required_major: u32,
    source: &str,
    hint: &str,
) {
    let Some(found_major) = read_assimp_major_from_headers(include_dirs) else {
        util::warn(format!(
            "could not determine Assimp version from headers discovered via {source}; skipping version gate"
        ));
        return;
    };

    if found_major < required_major {
        panic!(
            "system assimp headers are too old (detected major version {}). This crate requires Assimp >= {}.\n\
             {hint}",
            found_major, required_major
        );
    }
}

fn read_assimp_major_from_headers(include_dirs: &[std::path::PathBuf]) -> Option<u32> {
    let contents = include_dirs.iter().find_map(|dir| {
        let p = dir.join("assimp").join("revision.h");
        std::fs::read_to_string(&p).ok()
    })?;

    parse_define_u32(&contents, "VER_MAJOR")
        .or_else(|| parse_define_u32(&contents, "ASSIMP_VERSION_MAJOR"))
}

fn parse_define_u32(contents: &str, name: &str) -> Option<u32> {
    for line in contents.lines() {
        let line = line.trim();
        if !line.starts_with("#define") {
            continue;
        }
        let rest = line.strip_prefix("#define")?.trim_start();
        let rest = rest.strip_prefix(name)?.trim_start();
        let value = rest.split_whitespace().next()?;
        if let Ok(v) = value.parse::<u32>() {
            return Some(v);
        }
    }
    None
}

fn common_include_roots() -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();

    #[cfg(not(windows))]
    {
        for p in [
            "/usr/include",
            "/usr/local/include",
            "/opt/homebrew/include",
            "/opt/local/include",
        ] {
            let pb = std::path::PathBuf::from(p);
            if pb.exists() {
                roots.push(pb);
            }
        }
    }

    roots
}
