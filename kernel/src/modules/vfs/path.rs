use alloc::string::String;
use alloc::vec::Vec;

pub fn valid_path(path: &str) -> bool {
    let max_path_len = crate::config::KernelConfig::diskfs_max_path_len();
    if path.is_empty() || !path.starts_with('/') {
        return false;
    }
    if path.len() > max_path_len {
        return false;
    }
    if path.contains('\0') {
        return false;
    }
    true
}

pub fn normalize_path(path: &str) -> Option<Vec<u8>> {
    normalize_str(path).map(|s| s.into_bytes())
}

/// Like `normalize_path` but returns a `String`.
pub fn normalize_str(path: &str) -> Option<String> {
    if !valid_path(path) {
        #[cfg(feature = "vfs_telemetry")]
        crate::modules::vfs::telemetry::note_invalid_path();
        return None;
    }

    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        } else if segment == ".." {
            parts.pop();
        } else {
            parts.push(segment);
        }
    }

    let mut normalized = String::from("/");
    normalized.push_str(&parts.join("/"));

    let max_path_len = crate::config::KernelConfig::diskfs_max_path_len();
    if normalized.len() > max_path_len {
        #[cfg(feature = "vfs_telemetry")]
        crate::modules::vfs::telemetry::note_invalid_path();
        return None;
    }

    Some(normalized)
}

/// Split a normalized path into its components (without the leading '/').
/// e.g. `/foo/bar/baz` → `["foo", "bar", "baz"]`.
pub fn path_components(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.into())
        .collect()
}
