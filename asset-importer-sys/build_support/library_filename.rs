pub fn assimp_link_name<'a>(
    file_name: &'a str,
    target: &str,
    static_link: bool,
) -> Option<&'a str> {
    let lower = file_name.to_ascii_lowercase();
    let windows = target.contains("-windows-");
    if windows && target.ends_with("-msvc") {
        return (lower.starts_with("assimp") && lower.ends_with(".lib"))
            .then(|| &file_name[..file_name.len() - 4]);
    }

    if !lower.starts_with("libassimp") {
        return None;
    }

    let suffix = if static_link {
        if lower.ends_with(".dll.a") {
            return None;
        }
        ".a"
    } else if windows {
        ".dll.a"
    } else if lower.ends_with(".dylib") {
        ".dylib"
    } else {
        let end = lower.find(".so")?;
        let version = &lower[end + 3..];
        if !version.is_empty() && !version.starts_with('.') {
            return None;
        }
        return Some(&file_name[3..end]);
    };

    if !lower.ends_with(suffix) {
        return None;
    }
    let stem = &file_name[3..file_name.len() - suffix.len()];
    Some(stem.split('.').next().unwrap_or(stem))
}

pub fn windows_zlib_link_name<'a>(file_name: &'a str, target: &str) -> Option<&'a str> {
    if !target.contains("-windows-") {
        return None;
    }

    let lower = file_name.to_ascii_lowercase();
    if target.ends_with("-msvc") {
        if lower.ends_with(".lib") && (lower.starts_with("zlibstatic") || lower == "zlib.lib") {
            return Some(&file_name[..file_name.len() - 4]);
        }
    } else if lower.starts_with("libzlibstatic")
        && lower.ends_with(".a")
        && !lower.ends_with(".dll.a")
    {
        return Some(&file_name[3..file_name.len() - 2]);
    }
    None
}
