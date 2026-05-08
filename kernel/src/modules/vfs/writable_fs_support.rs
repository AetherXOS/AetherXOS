use alloc::string::String;
use aethercore_common::units::PAGE_SIZE_4K;

const PAGE_SIZE: usize = PAGE_SIZE_4K;
const S_IFREG: u16 = 0o100000;
const S_IFDIR: u16 = 0o040000;
const S_IFLNK: u16 = 0o120000;

/// Tracks overlay-specific metadata for a file/directory.
#[derive(Debug, Clone)]
pub(super) struct OverlayEntry {
    pub(super) ino: u64,
    pub(super) mode: u16,
    pub(super) uid: u32,
    pub(super) gid: u32,
    pub(super) size: u64,
    pub(super) atime: u64,
    pub(super) mtime: u64,
    pub(super) ctime: u64,
    pub(super) link_count: u32,
    pub(super) symlink_target: Option<String>,
    pub(super) whiteout: bool,
}

impl OverlayEntry {
    pub(super) fn new_file(ino: u64, mode: u16) -> Self {
        Self {
            ino,
            mode: S_IFREG | (mode & 0o7777),
            uid: 0,
            gid: 0,
            size: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            link_count: 1,
            symlink_target: None,
            whiteout: false,
        }
    }

    pub(super) fn new_dir(ino: u64, mode: u16) -> Self {
        Self {
            ino,
            mode: S_IFDIR | (mode & 0o7777),
            uid: 0,
            gid: 0,
            size: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            link_count: 2,
            symlink_target: None,
            whiteout: false,
        }
    }

    pub(super) fn new_symlink(ino: u64, target: String) -> Self {
        let size = target.len() as u64;
        Self {
            ino,
            mode: S_IFLNK | 0o777,
            uid: 0,
            gid: 0,
            size,
            atime: 0,
            mtime: 0,
            ctime: 0,
            link_count: 1,
            symlink_target: Some(target),
            whiteout: false,
        }
    }

    pub(super) fn is_dir(&self) -> bool {
        (self.mode & S_IFDIR) != 0
    }

    pub(super) fn is_symlink(&self) -> bool {
        (self.mode & S_IFLNK) == S_IFLNK
    }

    pub(super) fn to_stats(&self) -> crate::modules::vfs::types::FileStats {
        use crate::modules::vfs::types::VfsTimespec;
        crate::modules::vfs::types::FileStats {
            size: self.size,
            mode: self.mode as u32,
            uid: self.uid,
            gid: self.gid,
            nlink: self.link_count,
            atime: VfsTimespec { sec: self.atime, nsec: 0 },
            mtime: VfsTimespec { sec: self.mtime, nsec: 0 },
            ctime: VfsTimespec { sec: self.ctime, nsec: 0 },
            blksize: PAGE_SIZE as u32,
            blocks: (self.size + 511) / 512,
            ..crate::modules::vfs::types::FileStats::default()
        }
    }
}

// FNV-1a hash constants
pub const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
pub const FNV_PRIME: u64 = 0x100000001b3;

/// Simple FNV-1a hash for dentry names.
pub(super) fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = FNV_OFFSET_BASIS;
    for b in s.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn overlay_entry_constructors_set_expected_shape() {
        let file = OverlayEntry::new_file(1, 0o644);
        assert!(!file.is_dir());
        assert!(!file.is_symlink());
        assert_eq!(file.link_count, 1);

        let dir = OverlayEntry::new_dir(2, 0o755);
        assert!(dir.is_dir());
        assert_eq!(dir.link_count, 2);

        let symlink = OverlayEntry::new_symlink(3, String::from("/target"));
        assert!(symlink.is_symlink());
        assert_eq!(symlink.size, 7);
        assert_eq!(symlink.symlink_target.as_deref(), Some("/target"));
    }

    #[test_case]
    fn simple_hash_is_stable_and_sensitive_to_input() {
        assert_eq!(simple_hash("name"), simple_hash("name"));
        assert_ne!(simple_hash("name"), simple_hash("name2"));
        assert_ne!(simple_hash("alpha"), 0);
    }
}
