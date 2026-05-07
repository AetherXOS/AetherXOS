use crate::kernel::syscalls::linux_errno;
use super::state::*;

pub fn sys_linux_landlock_create_ruleset(
    attr_ptr: usize,
    size: usize,
    flags: usize,
) -> usize {
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if attr_ptr == 0 || size == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    let id = NEXT_LANDLOCK_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    LANDLOCK_RULESET_IDS.lock().insert(id);
    LANDLOCK_FD_BASE.saturating_add(id as usize)
}

pub fn sys_linux_landlock_add_rule(
    ruleset_fd: usize,
    rule_type: usize,
    rule_attr: usize,
    flags: usize,
) -> usize {
    let _ = (rule_type, rule_attr);
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if ruleset_fd < LANDLOCK_FD_BASE {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    let id = (ruleset_fd - LANDLOCK_FD_BASE) as u32;
    if !LANDLOCK_RULESET_IDS.lock().contains(&id) {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}

pub fn sys_linux_landlock_restrict_self(ruleset_fd: usize, flags: usize) -> usize {
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if ruleset_fd < LANDLOCK_FD_BASE {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    let id = (ruleset_fd - LANDLOCK_FD_BASE) as u32;
    if !LANDLOCK_RULESET_IDS.lock().contains(&id) {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}
