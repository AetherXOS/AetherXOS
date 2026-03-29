use super::*;
use core::sync::atomic::Ordering;

pub fn sys_linux_userfaultfd(flags: usize) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    let id = NEXT_USERFAULTFD_ID.fetch_add(1, Ordering::Relaxed);
    LINUX_USERFAULTFD_IDS.lock().insert(id);
    id as usize
}

pub fn sys_linux_membarrier(cmd: usize, flags: usize, _cpu_id: usize) -> usize {
    const MEMBARRIER_CMD_QUERY: usize = 0;
    if flags != 0 {
        return linux_inval();
    }
    if cmd == MEMBARRIER_CMD_QUERY {
        // We report private expedited support as baseline (bit 3).
        return 1 << MEMBARRIER_PRIVATE_EXPEDITED_BIT;
    }
    0
}

pub fn sys_linux_pidfd_send_signal(
    pidfd: Fd,
    sig: usize,
    _info: UserPtr<u8>,
    flags: usize,
) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    crate::require_posix_process!((pidfd, sig, flags) => {
        match crate::modules::posix::process::pidfd_send_signal(pidfd.as_u32(), sig as i32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_io_uring_setup(entries: usize, _params: UserPtr<u8>) -> usize {
    if entries == 0 {
        return linux_inval();
    }
    let id = NEXT_IO_URING_FD.fetch_add(1, Ordering::Relaxed);
    LINUX_IO_URING_IDS.lock().insert(id);
    id as usize
}

pub fn sys_linux_io_uring_enter(
    fd: Fd,
    _to_submit: usize,
    _min_complete: usize,
    flags: usize,
    _sig: UserPtr<u8>,
) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    if !LINUX_IO_URING_IDS.lock().contains(&fd.as_u32()) {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}

pub fn sys_linux_io_uring_register(
    fd: Fd,
    _opcode: usize,
    _arg: UserPtr<u8>,
    _nr_args: usize,
) -> usize {
    if !LINUX_IO_URING_IDS.lock().contains(&fd.as_u32()) {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}

pub fn sys_linux_pidfd_open(pid: usize, flags: usize) -> usize {
    let allowed_flags = linux::PIDFD_NONBLOCK;
    if (flags & !allowed_flags) != 0 {
        return linux_inval();
    }
    crate::require_posix_process!((pid, flags) => {
        match crate::modules::posix::process::pidfd_open(pid) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_pidfd_getfd(pidfd: Fd, targetfd: usize, flags: usize) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    crate::require_posix_process!((pidfd, targetfd, flags) => {
        let owner = match crate::modules::posix::process::pidfd_get_pid(pidfd.as_u32()) {
            Ok(p) => p,
            Err(e) => return linux_errno(e.code()),
        };
        let current = crate::modules::posix::process::getpid();
        if owner != current {
            return linux_eperm();
        }
        #[cfg(feature = "posix_fs")]
        {
            match crate::modules::posix::fs::dup(targetfd as u32) {
                Ok(fd) => fd as usize,
                Err(e) => linux_errno(e.code()),
            }
        }
        #[cfg(not(feature = "posix_fs"))]
        {
            linux_errno(crate::modules::posix_consts::errno::EBADF)
        }
    })
}

pub fn sys_linux_process_madvise(
    pidfd: Fd,
    _iovec: UserPtr<LinuxIoVec>,
    _vlen: usize,
    _advice: usize,
    flags: usize,
) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    crate::require_posix_process!((pidfd, flags) => {
        match crate::modules::posix::process::pidfd_get_pid(pidfd.as_u32()) {
            Ok(_) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_quotactl_fd(fd: Fd, cmd: usize, id: usize, addr: UserPtr<u8>) -> usize {
    if let Err(e) = require_control_plane_access(crate::modules::security::RESOURCE_VFS_STATS) {
        return e;
    }
    let _ = (cmd, id);
    crate::require_posix_fs!((fd, cmd, id, addr) => {
        if crate::modules::posix::fs::fd_fs_context(fd.as_u32()).is_err() {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        }
        if !addr.is_null() {
            let zero = [0u8; 8];
            let rc = crate::kernel::syscalls::with_user_write_bytes(addr.addr, zero.len(), |dst| {
                dst.copy_from_slice(&zero);
                0
            });
            if let Err(e) = rc {
                return e;
            }
        }
        0
    })
}

pub fn sys_linux_landlock_create_ruleset(attr: UserPtr<u8>, size: usize, flags: usize) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    if size > 0 && attr.is_null() {
        return linux_fault();
    }
    let id = NEXT_LANDLOCK_RULESET_ID.fetch_add(1, Ordering::Relaxed);
    LINUX_LANDLOCK_RULESETS.lock().insert(id);
    id as usize
}

pub fn sys_linux_landlock_add_rule(
    ruleset_fd: Fd,
    _rule_type: usize,
    _rule_attr: UserPtr<u8>,
    flags: usize,
) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    if !LINUX_LANDLOCK_RULESETS
        .lock()
        .contains(&(ruleset_fd.as_u32()))
    {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}

pub fn sys_linux_landlock_restrict_self(ruleset_fd: Fd, flags: usize) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    if !LINUX_LANDLOCK_RULESETS
        .lock()
        .contains(&(ruleset_fd.as_u32()))
    {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}

pub fn sys_linux_memfd_secret(flags: usize) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    crate::require_posix_fs!((flags) => {
        let id = NEXT_MEMFD_ID.fetch_add(1, Ordering::Relaxed);
        let path = alloc::format!("/.memfd-secret-{}", id);
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::openat(fs_id, "/", &path, true) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_process_mrelease(pidfd: Fd, flags: usize) -> usize {
    if flags != 0 {
        return linux_inval();
    }
    crate::require_posix_process!((pidfd, flags) => {
        match crate::modules::posix::process::pidfd_get_pid(pidfd.as_u32()) {
            Ok(_) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

pub fn sys_linux_cachestat(
    _fd: Fd,
    _cstat_range: UserPtr<u8>,
    cstat: UserPtr<u8>,
    flags: usize,
) -> usize {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct LinuxCacheStatOut {
        nr_cache: u64,
        nr_dirty: u64,
        nr_writeback: u64,
        nr_evicted: u64,
        nr_recently_evicted: u64,
    }

    if flags != 0 {
        return linux_inval();
    }
    if cstat.is_null() {
        return linux_fault();
    }
    let out = LinuxCacheStatOut {
        nr_cache: 0,
        nr_dirty: 0,
        nr_writeback: 0,
        nr_evicted: 0,
        nr_recently_evicted: 0,
    };
    match cstat.cast::<LinuxCacheStatOut>().write(&out) {
        Ok(()) => 0,
        Err(e) => e,
    }
}
