use crate::interfaces::TaskId;
use alloc::vec::Vec;

pub(super) const ROOT_TASK_ID: TaskId = TaskId(0);

pub(super) fn normalize_mount_path(path: &[u8], max_mount_path: usize) -> Option<Vec<u8>> {
    if path.is_empty() || path.len() > max_mount_path {
        return None;
    }
    if path[0] != b'/' || path.contains(&0) {
        return None;
    }

    let mut out = Vec::with_capacity(max_mount_path);
    out.push(b'/');
    let mut index = 1usize;
    let mut wrote_segment = false;

    while index <= path.len() {
        let mut end = index;
        while end < path.len() && path[end] != b'/' {
            end += 1;
        }

        let segment = &path[index..end];
        if !segment.is_empty() {
            if segment == b"." {
            } else if segment == b".." {
                return None;
            } else {
                if wrote_segment {
                    if out.len() >= max_mount_path {
                        return None;
                    }
                    out.push(b'/');
                }

                if out.len().saturating_add(segment.len()) > max_mount_path {
                    return None;
                }

                out.extend_from_slice(segment);
                wrote_segment = true;
            }
        }

        index = end.saturating_add(1);
    }

    if !wrote_segment {
        out.truncate(1);
    }

    Some(out)
}

#[inline(always)]
pub(super) fn current_task_id() -> TaskId {
    unsafe {
        crate::kernel::cpu_local::CpuLocal::try_get()
            .map(|cpu| TaskId(cpu.current_task.load(core::sync::atomic::Ordering::Relaxed)))
            .unwrap_or(ROOT_TASK_ID)
    }
}

#[inline(always)]
pub(super) fn can_access_mount(owner: TaskId, tid: TaskId) -> bool {
    tid == ROOT_TASK_ID || owner == tid
}

#[inline(always)]
pub(super) fn valid_initrd_path(path: &str) -> bool {
    path.starts_with('/') && !path.contains("..") && !path.contains('\0')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn normalize_mount_path_rejects_bad_segments_and_trims_noise() {
        assert_eq!(normalize_mount_path(b"/", 16).unwrap(), b"/".to_vec());
        assert_eq!(
            normalize_mount_path(b"/var//log/", 16).unwrap(),
            b"/var/log".to_vec()
        );
        assert!(normalize_mount_path(b"relative", 16).is_none());
        assert!(normalize_mount_path(b"/a/../b", 16).is_none());
        assert!(normalize_mount_path(b"/bad\0path", 16).is_none());
    }

    #[test_case]
    fn mount_access_and_initrd_path_helpers_match_policy() {
        assert!(can_access_mount(ROOT_TASK_ID, TaskId(7)));
        assert!(can_access_mount(TaskId(9), TaskId(9)));
        assert!(!can_access_mount(TaskId(9), TaskId(8)));

        assert!(valid_initrd_path("/init"));
        assert!(!valid_initrd_path("init"));
        assert!(!valid_initrd_path("/../init"));
        assert!(!valid_initrd_path("/bad\0name"));
    }
}
