use crate::build_support::{
    config::BuildConfig,
    plan::{BuildMethod, BuildPlan, LinkKind},
    util,
};

use std::{fs, path::PathBuf};

pub fn probe(cfg: &BuildConfig, link_kind: LinkKind) -> BuildPlan {
    if cfg.is_windows() && cfg.is_msvc() {
        let mut vcpkg_cfg = vcpkg::Config::new();
        vcpkg_cfg.emit_includes(true);

        // vcpkg-rs requires explicit opt-in for dynamic linking.
        if matches!(link_kind, LinkKind::Dynamic) && std::env::var_os("VCPKGRS_DYNAMIC").is_none() {
            // `set_var` is `unsafe` because mutating the process environment is not thread-safe on
            // some platforms (it can race with `getenv`). Build scripts are single-threaded and run
            // before compilation, so this is an acceptable, bounded use.
            unsafe {
                std::env::set_var("VCPKGRS_DYNAMIC", "1");
            }
        }

        // Pick an explicit triplet when possible to keep behavior predictable.
        // Users can always override via VCPKGRS_TRIPLET / vcpkg env vars.
        let mut selected_triplet: Option<String> = None;
        if std::env::var("VCPKGRS_TRIPLET").is_err() {
            if cfg.use_static_crt() && matches!(link_kind, LinkKind::Dynamic) {
                util::warn(
                    "target uses crt-static but dynamic assimp linking was requested; vcpkg triplets do not cleanly support this combination (falling back to dynamic CRT triplet)",
                );
            }

            if let Some(triplet) =
                default_vcpkg_triplet(&cfg.target, link_kind, cfg.use_static_crt())
            {
                selected_triplet = Some(triplet.to_string());
                vcpkg_cfg.target_triplet(triplet);
            }
        } else if let Ok(t) = std::env::var("VCPKGRS_TRIPLET") {
            if !t.trim().is_empty() {
                selected_triplet = Some(t);
            }
        }

        ensure_vcpkg_layout(selected_triplet.as_deref());

        let lib = vcpkg_cfg
            .find_package("assimp")
            .unwrap_or_else(|e| {
                panic!(
                    "system linking (vcpkg) failed: {e}\n\
                     Hint: install assimp via vcpkg and set VCPKG_ROOT.\n\
                     If needed, set VCPKGRS_TRIPLET explicitly (e.g. `x64-windows`, `x64-windows-static-md`, `x64-windows-static`)."
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

fn ensure_vcpkg_layout(triplet: Option<&str>) {
    let current_root = std::env::var("VCPKG_ROOT").ok().map(PathBuf::from);
    let best_root = pick_vcpkg_root(triplet);

    let current_score = current_root
        .as_ref()
        .map(|p| score_vcpkg_root(p, triplet))
        .unwrap_or(0);
    let best_score = best_root
        .as_ref()
        .map(|p| score_vcpkg_root(p, triplet))
        .unwrap_or(0);

    if best_score > current_score {
        if let Some(root) = best_root {
            // `set_var` is `unsafe` because mutating the process environment is not thread-safe on
            // some platforms (it can race with `getenv`). Build scripts are single-threaded and run
            // before compilation, so this is an acceptable, bounded use.
            unsafe {
                std::env::set_var("VCPKG_ROOT", root);
            }
        }
    }

    // vcpkg-rs expects a vcpkg "root" with an `installed/vcpkg` metadata directory.
    // Some CI setups expose only `VCPKG_INSTALLATION_ROOT` and (depending on vcpkg version)
    // may not create the `installed/vcpkg/updates` directory by default, which makes vcpkg-rs fail.
    //
    // Create it opportunistically to keep system builds robust.
    let root = std::env::var("VCPKG_ROOT").ok();

    let Some(root) = root else {
        return;
    };

    let updates_dir: PathBuf = [root.as_str(), "installed", "vcpkg", "updates"]
        .iter()
        .collect();

    if updates_dir.exists() {
        return;
    }

    if let Err(e) = fs::create_dir_all(&updates_dir) {
        util::warn(format!(
            "failed to create vcpkg metadata directory {}: {}",
            updates_dir.display(),
            e
        ));
    }
}

fn pick_vcpkg_root(triplet: Option<&str>) -> Option<PathBuf> {
    let candidates = [
        std::env::var("VCPKG_ROOT").ok(),
        std::env::var("VCPKG_INSTALLATION_ROOT").ok(),
        std::env::var("VCPKG_INSTALLED_DIR").ok(),
    ];

    let mut best: Option<(u8, PathBuf)> = None;
    for c in candidates.into_iter().flatten() {
        let pb = PathBuf::from(c);
        for root in normalize_root_candidates(&pb) {
            let score = score_vcpkg_root(&root, triplet);
            if score == 0 {
                continue;
            }
            if best.as_ref().is_none_or(|(s, _)| score > *s) {
                best = Some((score, root));
            }
        }
    }

    best.map(|(_, p)| p)
}

fn normalize_root_candidates(root: &PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    out.push(root.clone());
    if root
        .file_name()
        .is_some_and(|n| n == std::ffi::OsStr::new("installed"))
    {
        if let Some(parent) = root.parent() {
            out.push(parent.to_path_buf());
        }
    }
    out
}

fn score_vcpkg_root(root: &PathBuf, triplet: Option<&str>) -> u8 {
    let installed_dir = root.join("installed");
    if !installed_dir.exists() {
        return 0;
    }
    if let Some(t) = triplet {
        if installed_dir.join(t).exists() {
            return 2;
        }
    }
    1
}

fn default_vcpkg_triplet(
    target: &str,
    link_kind: LinkKind,
    use_static_crt: bool,
) -> Option<&'static str> {
    let is_x64 = target.starts_with("x86_64-");
    let is_x86 = target.starts_with("i686-");
    let is_arm64 = target.starts_with("aarch64-");

    match link_kind {
        LinkKind::Dynamic => {
            // Prefer the canonical dynamic triplets when linking dynamically.
            // Note: vcpkg does not provide a standard "dynamic + static CRT" triplet; users must
            // override manually if they have a custom setup.
            if is_x64 {
                Some("x64-windows")
            } else if is_x86 {
                Some("x86-windows")
            } else if is_arm64 {
                Some("arm64-windows")
            } else {
                None
            }
        }
        LinkKind::Static => {
            // For static library linking, match the CRT when possible:
            // - `*-windows-static` for static CRT
            // - `*-windows-static-md` for dynamic CRT
            if use_static_crt {
                if is_x64 {
                    Some("x64-windows-static")
                } else if is_x86 {
                    Some("x86-windows-static")
                } else if is_arm64 {
                    Some("arm64-windows-static")
                } else {
                    None
                }
            } else if is_x64 {
                Some("x64-windows-static-md")
            } else if is_x86 {
                Some("x86-windows-static-md")
            } else if is_arm64 {
                Some("arm64-windows-static-md")
            } else {
                None
            }
        }
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
    #[cfg(windows)]
    {
        Vec::new()
    }

    #[cfg(not(windows))]
    {
        let mut roots = Vec::new();
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
        roots
    }
}
