use crate::build_support::config::BuildConfig;

pub fn emit(cfg: &BuildConfig) {
    if cfg.is_windows() {
        if cfg.is_msvc() {
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=shell32");
            println!("cargo:rustc-link-lib=ole32");
            println!("cargo:rustc-link-lib=oleaut32");
            println!("cargo:rustc-link-lib=uuid");
            println!("cargo:rustc-link-lib=advapi32");
        } else {
            // MinGW
            println!("cargo:rustc-link-lib=stdc++");
        }
        return;
    }

    if cfg.is_macos() {
        println!("cargo:rustc-link-lib=c++");
        // Keep parity with existing behavior (some toolchains use Foundation APIs for file/locale).
        println!("cargo:rustc-link-lib=framework=Foundation");
        return;
    }

    // Linux/BSD
    println!("cargo:rustc-link-lib=stdc++");
}
