use std::{env, path::PathBuf};

#[derive(Clone, Debug)]
pub struct BuildConfig {
    pub manifest_dir: PathBuf,
    pub out_dir: PathBuf,
    #[cfg_attr(not(feature = "prebuilt"), allow(dead_code))]
    pub target: String,
    pub target_os: String,
    pub target_env: String,
    pub profile: String,
    pub target_features: String,
    pub docs_rs: bool,
    pub verbose: bool,
    /// Force building Assimp from source even when `prebuilt` is enabled.
    pub force_build: bool,
    #[cfg_attr(not(feature = "prebuilt"), allow(dead_code))]
    pub offline: bool,
}

impl BuildConfig {
    pub fn new() -> Self {
        let target_features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
        let offline = env::var("ASSET_IMPORTER_OFFLINE").is_ok()
            || env::var("CARGO_NET_OFFLINE").is_ok_and(|v| v == "true");
        let force_build = matches!(env::var("ASSET_IMPORTER_FORCE_BUILD"), Ok(v) if !v.is_empty());

        Self {
            manifest_dir: PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()),
            out_dir: PathBuf::from(env::var("OUT_DIR").unwrap()),
            target: env::var("TARGET").unwrap_or_default(),
            target_os: env::var("CARGO_CFG_TARGET_OS").unwrap_or_default(),
            target_env: env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default(),
            profile: env::var("PROFILE").unwrap_or_else(|_| "release".to_string()),
            target_features,
            docs_rs: env::var("DOCS_RS").is_ok(),
            verbose: env::var("ASSET_IMPORTER_VERBOSE").is_ok(),
            force_build,
            offline,
        }
    }

    pub fn is_windows(&self) -> bool {
        self.target_os == "windows"
    }

    pub fn is_macos(&self) -> bool {
        self.target_os == "macos"
    }

    pub fn is_msvc(&self) -> bool {
        self.target_env == "msvc"
    }

    pub fn is_debug(&self) -> bool {
        self.profile == "debug"
    }

    pub fn use_static_crt(&self) -> bool {
        self.is_windows()
            && self.is_msvc()
            && self
                .target_features
                .split(',')
                .any(|f| f.trim() == "crt-static")
    }

    pub fn cmake_profile(&self) -> &str {
        // On MSVC, avoid Debug CRT and iterator-debug by using RelWithDebInfo when Cargo is in debug.
        if self.is_msvc() && self.is_debug() {
            "RelWithDebInfo"
        } else if self.is_debug() {
            "Debug"
        } else {
            "Release"
        }
    }

    pub fn macos_deployment_target(&self) -> String {
        env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "10.12".to_string())
    }

    pub fn assimp_source_dir(&self) -> PathBuf {
        env::var("ASSIMP_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| self.manifest_dir.join("assimp"))
    }

    pub fn emit_rerun_triggers(&self) {
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed=wrapper.h");
        println!("cargo:rerun-if-changed=wrapper.cpp");

        // When building from vendored sources, ensure changes to the Assimp checkout retrigger builds.
        // (Cargo does not automatically watch git submodules.)
        let assimp_dir = self.assimp_source_dir();
        for p in [
            assimp_dir.join("CMakeLists.txt"),
            assimp_dir.join("include").join("assimp").join("version.h"),
            assimp_dir.join("include").join("assimp").join("anim.h"),
        ] {
            if p.exists() {
                println!("cargo:rerun-if-changed={}", p.display());
            }
        }

        // Build method inputs
        println!("cargo:rerun-if-env-changed=ASSIMP_DIR");
        println!("cargo:rerun-if-env-changed=ASSET_IMPORTER_PACKAGE_DIR");
        println!("cargo:rerun-if-env-changed=ASSET_IMPORTER_CACHE_DIR");
        println!("cargo:rerun-if-env-changed=ASSET_IMPORTER_OFFLINE");
        println!("cargo:rerun-if-env-changed=ASSET_IMPORTER_FORCE_BUILD");
        println!("cargo:rerun-if-env-changed=ASSET_IMPORTER_VERBOSE");
        println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");
        println!("cargo:rerun-if-env-changed=CARGO_NET_OFFLINE");

        // System discovery knobs (pkg-config/vcpkg)
        println!("cargo:rerun-if-env-changed=PKG_CONFIG");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_LIBDIR");
        println!("cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR");
        println!("cargo:rerun-if-env-changed=VCPKG_ROOT");
        println!("cargo:rerun-if-env-changed=VCPKG_INSTALLATION_ROOT");
        println!("cargo:rerun-if-env-changed=VCPKG_INSTALLED_DIR");
        println!("cargo:rerun-if-env-changed=VCPKGRS_TRIPLET");
        println!("cargo:rerun-if-env-changed=VCPKGRS_DYNAMIC");

        // Toolchain knobs
        println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");
    }
}
