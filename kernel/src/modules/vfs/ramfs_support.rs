use crate::interfaces::TaskId;
use alloc::vec::Vec;

pub(super) const ROOT_TASK_ID: TaskId = TaskId(0);
pub(super) const ROOT_UID: u32 = 0;
pub(super) const ROOT_GID: u32 = 0;

#[derive(Clone, Copy)]
pub(super) struct RamMeta {
    pub(super) ino: u64,
    pub(super) mode: u16,
    pub(super) uid: u32,
    pub(super) gid: u32,
    pub(super) atime_sec: i64,
    pub(super) atime_nsec: i32,
    pub(super) mtime_sec: i64,
    pub(super) mtime_nsec: i32,
    pub(super) ctime_sec: i64,
    #[allow(dead_code)]
    pub(super) ctime_nsec: i32,
}

#[inline(always)]
pub(super) fn has_owner_access(tid: TaskId, owner_uid: u32) -> bool {
    tid == ROOT_TASK_ID || owner_uid == ROOT_UID || TaskId(owner_uid as usize) == tid
}

#[inline(always)]
pub(super) fn make_meta(ino: u64, mode: u16, owner: TaskId, now: i64) -> RamMeta {
    RamMeta {
        ino,
        mode,
        uid: owner.0 as u32,
        gid: ROOT_GID,
        atime_sec: now,
        atime_nsec: 0,
        mtime_sec: now,
        mtime_nsec: 0,
        ctime_sec: now,
        ctime_nsec: 0,
    }
}

pub(super) fn parent_dir(path: &[u8]) -> Option<Vec<u8>> {
    if path == b"/" {
        return None;
    }
    let slash = path.iter().rposition(|b| *b == b'/')?;
    if slash == 0 {
        Some(b"/".to_vec())
    } else {
        Some(path[..slash].to_vec())
    }
}

pub(super) fn is_child_of(path: &[u8], parent: &[u8]) -> bool {
    if parent == b"/" {
        return path.starts_with(b"/") && path.len() > 1;
    }
    path.len() > parent.len() && path.starts_with(parent) && path[parent.len()] == b'/'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn parent_dir_handles_root_and_nested_paths() {
        assert_eq!(parent_dir(b"/"), None);
        assert_eq!(parent_dir(b"/file"), Some(b"/".to_vec()));
        assert_eq!(parent_dir(b"/dir/file"), Some(b"/dir".to_vec()));
    }

    #[test_case]
    fn is_child_of_distinguishes_direct_prefixes() {
        assert!(is_child_of(b"/dir/file", b"/dir"));
        assert!(is_child_of(b"/anything", b"/"));
        assert!(!is_child_of(b"/dir", b"/dir"));
        assert!(!is_child_of(b"/directory/file", b"/dir"));
    }

    #[test_case]
    fn owner_access_allows_root_owner_and_matching_task() {
        assert!(has_owner_access(ROOT_TASK_ID, 77));
        assert!(has_owner_access(TaskId(5), ROOT_UID));
        assert!(has_owner_access(TaskId(9), 9));
        assert!(!has_owner_access(TaskId(9), 8));
    }
}
