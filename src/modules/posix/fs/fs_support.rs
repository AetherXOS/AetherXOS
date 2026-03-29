use crate::modules::posix::PosixErrno;
use alloc::string::String;

pub(super) fn map_fs_error(err: &'static str) -> PosixErrno {
    match err {
        "path empty"
        | "path too long"
        | "path contains invalid segment"
        | "path must be absolute" => PosixErrno::Invalid,
        "invalid path" => PosixErrno::Invalid,
        "invalid mount" | "mount unavailable" | "mount not found" => PosixErrno::BadFileDescriptor,
        "open failed" | "file not found" | "dir not found" | "parent not found" | "not found"
        | "device not found" => PosixErrno::NoEntry,
        "already exists" => PosixErrno::AlreadyExists,
        "dir not empty" => PosixErrno::Invalid,
        "permission denied" | "is a directory" => PosixErrno::PermissionDenied,
        "would block" | "already empty" => PosixErrno::Again,
        "timeout" => PosixErrno::TimedOut,
        "backend exists not supported"
        | "backend read not supported"
        | "backend read string not supported"
        | "backend write not supported"
        | "backend list dir not supported"
        | "backend metadata not supported" => PosixErrno::NotSupported,
        "not supported" => PosixErrno::NotSupported,
        _ => PosixErrno::Other,
    }
}

pub(super) fn normalize_path(path: &str) -> Result<String, PosixErrno> {
    if path.is_empty() || !path.starts_with('/') {
        return Err(PosixErrno::Invalid);
    }
    if path.len() > 1 && path.ends_with('/') {
        Ok(String::from(&path[..path.len() - 1]))
    } else {
        Ok(String::from(path))
    }
}

pub(super) fn apply_devfs_policy_mode(raw_mode: u16) -> u16 {
    match crate::config::KernelConfig::devfs_policy_profile() {
        crate::config::DevFsPolicyProfile::Strict => raw_mode.min(0o640),
        crate::config::DevFsPolicyProfile::Balanced => raw_mode,
        crate::config::DevFsPolicyProfile::Dev => raw_mode.max(0o666),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn normalize_path_accepts_absolute_and_trims_trailing_slash() {
        assert_eq!(normalize_path("/").unwrap(), "/");
        assert_eq!(normalize_path("/tmp").unwrap(), "/tmp");
        assert_eq!(normalize_path("/tmp/").unwrap(), "/tmp");
        assert_eq!(normalize_path("tmp"), Err(PosixErrno::Invalid));
        assert_eq!(normalize_path(""), Err(PosixErrno::Invalid));
    }

    #[test_case]
    fn map_fs_error_preserves_contract_categories() {
        assert_eq!(map_fs_error("path empty"), PosixErrno::Invalid);
        assert_eq!(
            map_fs_error("mount not found"),
            PosixErrno::BadFileDescriptor
        );
        assert_eq!(map_fs_error("file not found"), PosixErrno::NoEntry);
        assert_eq!(map_fs_error("already exists"), PosixErrno::AlreadyExists);
        assert_eq!(
            map_fs_error("permission denied"),
            PosixErrno::PermissionDenied
        );
        assert_eq!(
            map_fs_error("backend write not supported"),
            PosixErrno::NotSupported
        );
        assert_eq!(map_fs_error("mystery"), PosixErrno::Other);
    }

    #[test_case]
    fn devfs_policy_mode_stays_within_expected_bounds() {
        let clamped = apply_devfs_policy_mode(0o777);
        assert!((0o640..=0o777).contains(&clamped));
    }
}
