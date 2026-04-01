use alloc::string::String;

pub(super) fn validate_diskfs_path(path: &str) -> Result<String, &'static str> {
    if let Some(normalized_bytes) = crate::modules::vfs::path::normalize_path(path) {
        if let Ok(s) = String::from_utf8(normalized_bytes) {
            return Ok(s);
        }
    }
    Err("invalid path")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn validate_diskfs_path_normalizes_and_rejects_invalid_input() {
        assert_eq!(
            validate_diskfs_path("/a//b/./c"),
            Ok(String::from("/a/b/c"))
        );
        assert_eq!(validate_diskfs_path("relative/path"), Err("invalid path"));
        assert_eq!(validate_diskfs_path(""), Err("invalid path"));
    }
}
