#[path = "../build_support/library_filename.rs"]
mod library_filename;

use library_filename::{assimp_link_name, windows_zlib_link_name};

#[test]
fn windows_gnu_distinguishes_static_and_import_archives() {
    let target = "x86_64-pc-windows-gnu";
    for (file_name, static_name, dynamic_name) in [
        ("libassimp.a", Some("assimp"), None),
        ("libassimpd.a", Some("assimpd"), None),
        ("libassimp.dll.a", None, Some("assimp")),
        ("libassimpd.dll.a", None, Some("assimpd")),
        ("libassimp-6.dll.a", None, Some("assimp-6")),
        ("libassimp-6.dll", None, None),
        ("assimp-vc143-mt.lib", None, None),
    ] {
        assert_eq!(
            assimp_link_name(file_name, target, true),
            static_name,
            "{file_name}"
        );
        assert_eq!(
            assimp_link_name(file_name, target, false),
            dynamic_name,
            "{file_name}"
        );
    }
}

#[test]
fn windows_msvc_keeps_lib_names_for_both_link_kinds() {
    let target = "x86_64-pc-windows-msvc";
    for static_link in [true, false] {
        assert_eq!(
            assimp_link_name("assimp-vc143-mt.lib", target, static_link),
            Some("assimp-vc143-mt")
        );
        assert_eq!(assimp_link_name("libassimp.a", target, static_link), None);
        assert_eq!(
            assimp_link_name("libassimp.dll.a", target, static_link),
            None
        );
    }
}

#[test]
fn unix_link_names_preserve_debug_names_and_remove_shared_versions() {
    for (target, file_name, static_link, expected) in [
        (
            "x86_64-unknown-linux-gnu",
            "libassimp.a",
            true,
            Some("assimp"),
        ),
        (
            "x86_64-unknown-linux-gnu",
            "libassimp.so.6.0.6",
            false,
            Some("assimp"),
        ),
        (
            "x86_64-unknown-linux-gnu",
            "libassimpd.so",
            false,
            Some("assimpd"),
        ),
        (
            "aarch64-apple-darwin",
            "libassimp.6.0.6.dylib",
            false,
            Some("assimp"),
        ),
        (
            "aarch64-apple-darwin",
            "libassimpd.6.dylib",
            false,
            Some("assimpd"),
        ),
        ("x86_64-unknown-linux-gnu", "libassimp.so", true, None),
        ("x86_64-unknown-linux-gnu", "libassimp.a", false, None),
        ("x86_64-unknown-linux-gnu", "libassimp.dll.a", true, None),
        ("x86_64-unknown-linux-gnu", "libassimp.dll.a", false, None),
    ] {
        assert_eq!(
            assimp_link_name(file_name, target, static_link),
            expected,
            "{file_name}"
        );
    }
}

#[test]
fn bundled_windows_zlib_names_follow_the_target_abi() {
    let gnu = "x86_64-pc-windows-gnu";
    let msvc = "x86_64-pc-windows-msvc";
    for (file_name, gnu_name, msvc_name) in [
        ("libzlibstatic.a", Some("zlibstatic"), None),
        ("libzlibstaticd.a", Some("zlibstaticd"), None),
        ("libzlibstatic.dll.a", None, None),
        ("libz.a", None, None),
        ("zlibstatic.lib", None, Some("zlibstatic")),
        ("zlibstaticd.lib", None, Some("zlibstaticd")),
        ("zlib.lib", None, Some("zlib")),
    ] {
        assert_eq!(
            windows_zlib_link_name(file_name, gnu),
            gnu_name,
            "{file_name}"
        );
        assert_eq!(
            windows_zlib_link_name(file_name, msvc),
            msvc_name,
            "{file_name}"
        );
        assert_eq!(
            windows_zlib_link_name(file_name, "x86_64-unknown-linux-gnu"),
            None
        );
    }
}
