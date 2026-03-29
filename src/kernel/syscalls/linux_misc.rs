#[cfg(not(feature = "linux_compat"))]
use super::{linux_errno, sys_yield, with_user_read_bytes, with_user_write_bytes};

mod poll_select;
mod proc_ctl;
mod runtime_info;

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxTimespecCompat {
    tv_sec: i64,
    tv_nsec: i64,
}

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxItimerspecCompat {
    it_interval: LinuxTimespecCompat,
    it_value: LinuxTimespecCompat,
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn read_user_c_string_compat(ptr: usize, max_len: usize) -> Result<alloc::string::String, usize> {
    if ptr == 0 || max_len == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }
    let mut out = alloc::vec::Vec::new();
    for i in 0..max_len {
        let Some(addr) = ptr.checked_add(i) else {
            return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
        };
        let b = with_user_read_bytes(addr, 1, |src| src[0])
            .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;
        if b == 0 {
            return alloc::string::String::from_utf8(out)
                .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EINVAL));
        }
        out.push(b);
    }
    Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn read_u64_from_user(ptr: usize) -> Result<u64, usize> {
    with_user_read_bytes(ptr, core::mem::size_of::<u64>(), |src| {
        let mut bytes = [0u8; core::mem::size_of::<u64>()];
        bytes.copy_from_slice(src);
        u64::from_ne_bytes(bytes)
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn write_zero_itimerspec(ptr: usize) -> usize {
    let zero = LinuxItimerspecCompat::default();
    with_user_write_bytes(ptr, core::mem::size_of::<LinuxItimerspecCompat>(), |dst| {
        let src = unsafe {
            core::slice::from_raw_parts(
                (&zero as *const LinuxItimerspecCompat).cast::<u8>(),
                core::mem::size_of::<LinuxItimerspecCompat>(),
            )
        };
        dst.copy_from_slice(src);
        0
    })
    .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_poll(fds_ptr: usize, nfds: usize, timeout: usize) -> usize {
    poll_select::sys_linux_poll(fds_ptr, nfds, timeout)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_ppoll(
    fds_ptr: usize,
    nfds: usize,
    timeout_ptr: usize,
    sigmask_ptr: usize,
    sigset_size: usize,
) -> usize {
    poll_select::sys_linux_ppoll(fds_ptr, nfds, timeout_ptr, sigmask_ptr, sigset_size)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_select(
    nfds: usize,
    readfds: usize,
    writefds: usize,
    exceptfds: usize,
    timeout: usize,
) -> usize {
    poll_select::sys_linux_select(nfds, readfds, writefds, exceptfds, timeout)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_pselect6(
    nfds: usize,
    readfds: usize,
    writefds: usize,
    exceptfds: usize,
    timeout_ptr: usize,
    sigmask_desc_ptr: usize,
) -> usize {
    poll_select::sys_linux_pselect6(
        nfds,
        readfds,
        writefds,
        exceptfds,
        timeout_ptr,
        sigmask_desc_ptr,
    )
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_gettimeofday(tv_ptr: usize) -> usize {
    runtime_info::sys_linux_gettimeofday(tv_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_time(tloc: usize) -> usize {
    runtime_info::sys_linux_time(tloc)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_getcpu(cpu_ptr: usize, node_ptr: usize) -> usize {
    runtime_info::sys_linux_getcpu(cpu_ptr, node_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_prctl(
    option: usize,
    arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
) -> usize {
    proc_ctl::sys_linux_prctl(option, arg2, _arg3, _arg4, _arg5)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_getparam(_pid: usize, param_ptr: usize) -> usize {
    proc_ctl::sys_linux_sched_getparam(_pid, param_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_getscheduler(_pid: usize) -> usize {
    proc_ctl::sys_linux_sched_getscheduler(_pid)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_setparam(pid: usize, param_ptr: usize) -> usize {
    proc_ctl::sys_linux_sched_setparam(pid, param_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_setscheduler(pid: usize, policy: usize, param_ptr: usize) -> usize {
    proc_ctl::sys_linux_sched_setscheduler(pid, policy, param_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_getaffinity(
    _pid: usize,
    cpusetsize: usize,
    mask_ptr: usize,
) -> usize {
    proc_ctl::sys_linux_sched_getaffinity(_pid, cpusetsize, mask_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_setaffinity(
    _pid: usize,
    _cpusetsize: usize,
    _mask_ptr: usize,
) -> usize {
    proc_ctl::sys_linux_sched_setaffinity(_pid, _cpusetsize, _mask_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sysinfo(info_ptr: usize) -> usize {
    runtime_info::sys_linux_sysinfo(info_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_getrandom(buf_ptr: usize, buflen: usize, _flags: usize) -> usize {
    runtime_info::sys_linux_getrandom(buf_ptr, buflen, _flags)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_eventfd(initval: usize, flags: usize) -> usize {
    #[cfg(feature = "posix_io")]
    {
        match crate::modules::posix::io::eventfd_create_errno(initval as u32, flags as i32) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_io"))]
    {
        let _ = (initval, flags);
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_eventfd2(initval: usize, flags: usize) -> usize {
    sys_linux_eventfd(initval, flags)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_timerfd_create(clockid: usize, flags: usize) -> usize {
    let allowed_flags = 0x1usize | 0x0008_0000usize | 0x0000_0800usize;
    if (flags & !allowed_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if clockid > crate::modules::posix_consts::time::CLOCK_MONOTONIC as usize {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::openat(fs_id, "/", "timerfd", true) {
            Ok(fd) => {
                if (flags & 0x0000_0800usize) != 0 {
                    let _ = crate::modules::posix::fs::fcntl_set_status_flags(
                        fd,
                        crate::modules::posix_consts::net::O_NONBLOCK,
                    );
                }
                fd as usize
            }
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_timerfd_settime(
    _fd: usize,
    flags: usize,
    new_value_ptr: usize,
    old_value_ptr: usize,
) -> usize {
    let _ = _fd;
    if (flags & !0x1usize) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if new_value_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }

    let parse_ok = with_user_read_bytes(
        new_value_ptr,
        core::mem::size_of::<LinuxItimerspecCompat>(),
        |_| 0usize,
    );
    if parse_ok.is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }

    if old_value_ptr != 0 {
        let rc = write_zero_itimerspec(old_value_ptr);
        if rc != 0 {
            return rc;
        }
    }
    0
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_timerfd_gettime(_fd: usize, curr_value_ptr: usize) -> usize {
    let _ = _fd;
    if curr_value_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    write_zero_itimerspec(curr_value_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_signalfd(fd: usize, mask_ptr: usize, sizemask: usize) -> usize {
    sys_linux_signalfd4(fd, mask_ptr, sizemask, 0)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_signalfd4(
    fd: usize,
    mask_ptr: usize,
    sizemask: usize,
    flags: usize,
) -> usize {
    if sizemask != core::mem::size_of::<u64>() {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    let mask = match read_u64_from_user(mask_ptr) {
        Ok(v) => v,
        Err(e) => return e,
    };

    #[cfg(feature = "posix_signal")]
    {
        let raw_fd = fd as i32;
        let result = if raw_fd >= 0 {
            crate::modules::posix::signal::signalfd_reconfigure_errno(raw_fd as u32, mask, flags as i32)
        } else {
            crate::modules::posix::signal::signalfd_create_errno(mask, flags as i32)
        };
        match result {
            Ok(out_fd) => out_fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_signal"))]
    {
        let _ = (fd, mask, flags);
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_inotify_init() -> usize {
    sys_linux_inotify_init1(0)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_inotify_init1(flags: usize) -> usize {
    let allowed_flags = 0x0000_0800usize | 0x0008_0000usize;
    if (flags & !allowed_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::inotify_init(flags as i32) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_inotify_add_watch(fd: usize, path_ptr: usize, mask: usize) -> usize {
    let path = match read_user_c_string_compat(path_ptr, crate::config::KernelConfig::syscall_max_path_len()) {
        Ok(v) => v,
        Err(e) => return e,
    };

    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::inotify_add_watch(fd as u32, &path, mask as u32) {
            Ok(wd) => wd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, path, mask);
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_inotify_rm_watch(fd: usize, wd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::inotify_rm_watch(fd as u32, wd as i32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (fd, wd);
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_memfd_create(name_ptr: usize, flags: usize) -> usize {
    use crate::kernel::syscalls::syscalls_consts::linux::memfd_flags::{
        MFD_ALLOW_SEALING, MFD_CLOEXEC, MFD_EXEC, MFD_HUGETLB, MFD_NOEXEC_SEAL,
    };

    let known_flags =
        MFD_CLOEXEC | MFD_ALLOW_SEALING | MFD_HUGETLB | MFD_NOEXEC_SEAL | MFD_EXEC | crate::kernel::syscalls::syscalls_consts::linux::MFD_HUGE_MASK;
    if (flags & !known_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & MFD_EXEC) != 0 && (flags & MFD_NOEXEC_SEAL) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let raw_name = if name_ptr == 0 {
        alloc::string::String::from("memfd")
    } else {
        match read_user_c_string_compat(name_ptr, 255) {
            Ok(v) if !v.is_empty() => v,
            Ok(_) => alloc::string::String::from("memfd"),
            Err(e) => return e,
        }
    };

    #[cfg(feature = "posix_fs")]
    {
        use core::sync::atomic::{AtomicU32, Ordering};

        static NEXT_MEMFD_ID: AtomicU32 = AtomicU32::new(1);

        let id = NEXT_MEMFD_ID.fetch_add(1, Ordering::Relaxed);
        let path = alloc::format!("/.memfd-{}-{}", id, raw_name.replace('/', "_"));
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::openat(fs_id, "/", &path, true) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = raw_name;
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_membarrier(cmd: usize, _flags: usize, _cpu_id: usize) -> usize {
    const MEMBARRIER_CMD_QUERY: usize = 0;
    const MEMBARRIER_CMD_GLOBAL: usize = 1 << 0;
    const MEMBARRIER_CMD_GLOBAL_EXPEDITED: usize = 1 << 1;
    const MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED: usize = 1 << 2;
    const MEMBARRIER_CMD_PRIVATE_EXPEDITED: usize = 1 << 3;
    const MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED: usize = 1 << 4;
    const MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE: usize = 1 << 5;

    let supported = MEMBARRIER_CMD_GLOBAL
        | MEMBARRIER_CMD_GLOBAL_EXPEDITED
        | MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED
        | MEMBARRIER_CMD_PRIVATE_EXPEDITED
        | MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED
        | MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE;

    if cmd == MEMBARRIER_CMD_QUERY {
        return supported;
    }
    if (cmd & supported) != 0 {
        return 0;
    }
    linux_errno(crate::modules::posix_consts::errno::EINVAL)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_rseq(
    rseq_ptr: usize,
    rseq_len: usize,
    flags: usize,
    _sig: usize,
) -> usize {
    // Minimal registration shim for runtimes that probe rseq during startup.
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if rseq_ptr == 0 || rseq_len < 32 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    // Validate user memory is at least readable/writable for the declared structure.
    if with_user_read_bytes(rseq_ptr, rseq_len, |_| 0usize).is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    if with_user_write_bytes(rseq_ptr, rseq_len, |_| 0usize).is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    0
}
