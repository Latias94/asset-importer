//! Version parsing shared by source builds, system discovery, and prebuilt packaging.

pub const BUNDLED_VERSION: &str = include_str!("../assimp-version.txt").trim_ascii();

#[cfg(test)]
#[allow(dead_code)]
#[path = "util.rs"]
mod build_util;

pub fn parse_header(contents: &str) -> Option<String> {
    fn number(contents: &str, name: &str) -> Option<u32> {
        contents.lines().find_map(|line| {
            let mut tokens = line.split_whitespace();
            if tokens.next()? != "#define" || tokens.next()? != name {
                return None;
            }
            tokens.next()?.parse().ok()
        })
    }

    let major =
        number(contents, "VER_MAJOR").or_else(|| number(contents, "ASSIMP_VERSION_MAJOR"))?;
    let minor =
        number(contents, "VER_MINOR").or_else(|| number(contents, "ASSIMP_VERSION_MINOR"))?;
    let patch =
        number(contents, "VER_PATCH").or_else(|| number(contents, "ASSIMP_VERSION_PATCH"))?;
    Some(format!("{major}.{minor}.{patch}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_profile_in_both_cargo_build_directory_layouts() {
        for out_dir in [
            "target/debug/build/asset-importer-sys-123/out",
            "target/debug/build/asset-importer-sys/123/out",
            "target/x86_64-pc-windows-msvc/debug/build/asset-importer-sys/123/out",
        ] {
            let expected = if out_dir.contains("x86_64-") {
                "target/x86_64-pc-windows-msvc/debug"
            } else {
                "target/debug"
            };
            assert_eq!(
                build_util::cargo_profile_dir(std::path::Path::new(out_dir)),
                std::path::Path::new(expected)
            );
        }
    }

    #[test]
    fn parses_revision_after_unrelated_defines() {
        let header = "#ifndef ASSIMP_REVISION_H_INC\n\
                      #define ASSIMP_REVISION_H_INC\n\
                      #define GitVersion 0xa79ff6bb\n\
                      #define VER_MAJOR 6\n\
                      #define VER_MINOR 0\n\
                      #define VER_PATCH 6\n";
        assert_eq!(parse_header(header).as_deref(), Some("6.0.6"));
        assert_eq!(BUNDLED_VERSION, "6.0.6");
    }

    #[test]
    fn parses_legacy_version_macros_with_comments() {
        let header = "#define ASSIMP_VERSION_MAJOR 6 // major\n\
                      #define ASSIMP_VERSION_MINOR 0\n\
                      #define ASSIMP_VERSION_PATCH 5\n";
        assert_eq!(parse_header(header).as_deref(), Some("6.0.5"));
    }

    #[test]
    fn rejects_missing_invalid_or_prefix_only_macros() {
        assert_eq!(parse_header("#define VER_MAJOR 6"), None);
        assert_eq!(parse_header("#define VER_MAJOR_SUFFIX 6"), None);
        assert_eq!(
            parse_header("#define VER_MAJOR six\n#define VER_MINOR 0\n#define VER_PATCH 6"),
            None
        );
    }
}
