use std::path::Path;

/// Convert a Windows path to MSYS2-compatible format if needed.
pub fn maybe_msys_path(path: &Path, _xorriso_bin: &str) -> String {
    let raw = path.to_string_lossy().to_string();
    maybe_msys_path_for_platform(&raw, cfg!(windows))
}

fn maybe_msys_path_for_platform(raw: &str, is_windows: bool) -> String {
    let is_drive_path = raw.len() >= 2 && raw.as_bytes()[1] == b':';
    if is_windows && is_drive_path {
        let drive = raw.as_bytes()[0].to_ascii_lowercase() as char;
        let path_part = raw[2..].replace('\\', "/");
        format!("/{}{}", drive, path_part)
    } else {
        raw.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::maybe_msys_path_for_platform;

    #[test]
    fn maybe_msys_path_converts_drive_paths_on_windows_branch() {
        let converted = maybe_msys_path_for_platform(r"C:\work\artifacts\boot.iso", true);
        assert_eq!(converted, "/c/work/artifacts/boot.iso");
    }

    #[test]
    fn maybe_msys_path_keeps_drive_paths_on_non_windows_branch() {
        let raw = r"C:\work\artifacts\boot.iso";
        let converted = maybe_msys_path_for_platform(raw, false);
        assert_eq!(converted, raw);
    }
}
