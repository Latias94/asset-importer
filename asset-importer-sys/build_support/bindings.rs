use std::path::PathBuf;

use crate::build_support::{config::BuildConfig, plan::BuildPlan, util};

pub fn run_docsrs(cfg: &BuildConfig) {
    util::warn("DOCS_RS detected: skipping native build and linking; generating bindings only");
    println!("cargo:rustc-cfg=docsrs");

    // Prefer pregenerated bindings when present.
    if use_pregenerated_bindings(cfg) {
        return;
    }

    // Best-effort: generate from vendored headers if available.
    let assimp_include = cfg.assimp_source_dir().join("include");
    if assimp_include.join("assimp").join("scene.h").exists() {
        let plan = BuildPlan {
            include_dirs: vec![assimp_include],
            link_kind: crate::build_support::plan::LinkKind::Dynamic,
            link_lib: None,
            link_search: Vec::new(),
            method: crate::build_support::plan::BuildMethod::Vendored,
        };
        run(cfg, &plan);
        return;
    }

    panic!(
        "DOCS_RS build: Assimp headers not found and no pregenerated bindings present.\n\
         Either vendor assimp headers in the published crate or add src/bindings_pregenerated.rs."
    );
}

pub fn run(cfg: &BuildConfig, plan: &BuildPlan) {
    let wrapper_h = cfg.manifest_dir.join("wrapper.h");

    let include_dirs = ensure_config_h(cfg, &plan.include_dirs);

    let mut builder = bindgen::Builder::default()
        .header(wrapper_h.to_string_lossy())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    for dir in &include_dirs {
        builder = builder.clang_arg(format!("-I{}", dir.display()));
    }

    builder = builder
        .allowlist_function("ai.*")
        .allowlist_type("ai.*")
        .allowlist_var("ai.*")
        .allowlist_var("AI_.*")
        .derive_default(true)
        .derive_debug(true)
        .derive_copy(true)
        .derive_eq(true)
        .derive_partialeq(true)
        .derive_hash(true)
        .prepend_enum_name(false)
        .layout_tests(false)
        // Keep enum shapes compatible with the safe layer (it expects Rust enums with variants).
        .rustified_enum("aiReturn")
        .rustified_enum("aiOrigin")
        .rustified_enum("aiTextureType")
        .rustified_enum("aiTextureOp")
        .rustified_enum("aiTextureMapping")
        .rustified_enum("aiTextureMapMode")
        .rustified_enum("aiShadingMode")
        .rustified_enum("aiBlendMode")
        .rustified_enum("aiPostProcessSteps")
        .rustified_enum("aiComponent")
        .rustified_enum("aiPrimitiveType")
        .rustified_enum("aiDefaultLogStream")
        .rustified_enum("aiImporterFlags")
        .rustified_enum("aiMetadataType")
        .rustified_enum("aiLightSourceType")
        .rustified_enum("aiAnimBehaviour")
        .rustified_enum("aiAnimInterpolation")
        .rustified_enum("aiMorphingMethod")
        .rustified_enum("aiPropertyTypeInfo")
        .rustified_enum("aiTextureFlags")
        .rustified_enum("aiRustPropertyKind");

    let bindings = builder
        .generate()
        .expect("Unable to generate Assimp bindings");
    let out_file = cfg.out_dir.join("bindings.rs");
    bindings
        .write_to_file(&out_file)
        .expect("Couldn't write bindings.rs");
}

fn ensure_config_h(cfg: &BuildConfig, include_dirs: &[PathBuf]) -> Vec<PathBuf> {
    // Assimp expects <assimp/config.h> to exist. In a pure source checkout, only config.h.in is present.
    // For bindgen we can generate a minimal config.h into OUT_DIR and add it as the highest-priority include dir.
    let has_config_h = include_dirs
        .iter()
        .any(|d| d.join("assimp").join("config.h").exists());
    if has_config_h {
        return include_dirs.to_vec();
    }

    let has_config_h_in = include_dirs
        .iter()
        .any(|d| d.join("assimp").join("config.h.in").exists());
    if !has_config_h_in {
        return include_dirs.to_vec();
    }

    let out_include_root = cfg.out_dir.join("include");
    let out_assimp_dir = out_include_root.join("assimp");
    let out_config_h = out_assimp_dir.join("config.h");
    let _ = std::fs::create_dir_all(&out_assimp_dir);

    if !out_config_h.exists() {
        let minimal = r#"#ifndef AI_CONFIG_H_INC
#define AI_CONFIG_H_INC

// Minimal config.h for bindgen/header parsing.
// If you need exact build-time configuration, build Assimp from source and use the generated headers.

#ifndef AI_CONFIG_CHECK_IDENTITY_MATRIX_EPSILON_DEFAULT
#define AI_CONFIG_CHECK_IDENTITY_MATRIX_EPSILON_DEFAULT 10e-3f
#endif

#endif // AI_CONFIG_H_INC
"#;
        let _ = std::fs::write(&out_config_h, minimal);
    }

    let mut out = Vec::with_capacity(include_dirs.len() + 1);
    out.push(out_include_root);
    out.extend_from_slice(include_dirs);
    out
}

fn use_pregenerated_bindings(cfg: &BuildConfig) -> bool {
    let pregenerated = cfg
        .manifest_dir
        .join("src")
        .join("bindings_pregenerated.rs");
    if !pregenerated.exists() {
        return false;
    }
    let out_file = cfg.out_dir.join("bindings.rs");
    let content = std::fs::read_to_string(&pregenerated).unwrap_or_else(|e| {
        panic!(
            "Failed to read pregenerated bindings {}: {}",
            pregenerated.display(),
            e
        )
    });
    std::fs::write(&out_file, content)
        .unwrap_or_else(|e| panic!("Failed to write bindings.rs to OUT_DIR: {}", e));
    true
}
