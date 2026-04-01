#[cfg(not(feature = "posix_process"))]
use super::super::task_time::sys_linux_getpid;
use super::*;
#[cfg(not(feature = "linux_compat"))]
use alloc::collections::BTreeMap;
#[cfg(not(feature = "linux_compat"))]
use crate::kernel::syscalls::linux_shim::util::write_user_pod;
#[cfg(not(feature = "linux_compat"))]
use spin::Mutex;

#[cfg(not(feature = "linux_compat"))]
static LINUX_FD_FLAGS: Mutex<BTreeMap<u32, usize>> = Mutex::new(BTreeMap::new());
#[cfg(not(feature = "linux_compat"))]
static LINUX_PIDFD_MAP: Mutex<BTreeMap<u32, LinuxPidFdEntry>> = Mutex::new(BTreeMap::new());

#[cfg(not(feature = "linux_compat"))]
#[derive(Clone, Copy)]
struct LinuxPidFdEntry {
    target_pid: usize,
    owner_tid: usize,
}

#[cfg(not(feature = "linux_compat"))]
const LINUX_FD_CLOEXEC: usize = 0x1;

#[cfg(not(feature = "linux_compat"))]
fn linux_fd_get_descriptor_flags(fd: u32) -> usize {
    LINUX_FD_FLAGS.lock().get(&fd).copied().unwrap_or(0)
}

#[cfg(not(feature = "linux_compat"))]
fn linux_fd_set_descriptor_flags(fd: u32, flags: usize) {
    let masked = flags & LINUX_FD_CLOEXEC;
    let mut table = LINUX_FD_FLAGS.lock();
    if masked == 0 {
        table.remove(&fd);
    } else {
        table.insert(fd, masked);
    }
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
fn linux_fd_clear_descriptor_flags(fd: u32) {
    LINUX_FD_FLAGS.lock().remove(&fd);
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(crate) fn clear_linux_fd_flags(fd: u32) {
    linux_fd_clear_descriptor_flags(fd);
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
fn linux_pidfd_clear(fd: u32) {
    LINUX_PIDFD_MAP.lock().remove(&fd);
}

#[cfg(not(feature = "linux_compat"))]
fn linux_current_tid() -> usize {
    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    cpu.current_task.load(core::sync::atomic::Ordering::Relaxed)
}

#[cfg(not(feature = "linux_compat"))]
fn linux_task_exists(pid: usize) -> bool {
    crate::kernel::task::get_task(crate::interfaces::task::TaskId(pid)).is_some()
}

#[cfg(not(feature = "linux_compat"))]
fn linux_pidfd_entry_for_caller(pidfd: usize) -> Result<LinuxPidFdEntry, usize> {
    let entry = LINUX_PIDFD_MAP
        .lock()
        .get(&(pidfd as u32))
        .copied()
        .ok_or_else(|| linux_errno(crate::modules::posix_consts::errno::EBADF))?;

    if linux_current_tid() != entry.owner_tid {
        return Err(linux_errno(crate::modules::posix_consts::errno::EPERM));
    }
    if !linux_task_exists(entry.target_pid) {
        return Err(linux_errno(crate::modules::posix_consts::errno::ESRCH));
    }

    Ok(entry)
}

#[cfg(not(feature = "linux_compat"))]
fn linux_pidfd_getfd_access_allowed(caller_tid: usize, target_pid: usize) -> bool {
    caller_tid == target_pid || caller_tid == 1
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(crate) fn clear_linux_pidfd_entry(fd: u32) {
    linux_pidfd_clear(fd);
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_pipe(pipefd_ptr: usize, flags: usize) -> usize {
    #[cfg(feature = "posix_pipe")]
    {
        let nonblock = (flags & 0x800) != 0;
        match crate::modules::posix::pipe::pipe2(nonblock) {
            Ok((rfd, wfd)) => {
                let rfd_u32 = rfd as u32;
                let wfd_u32 = wfd as u32;
                if write_user_pod(pipefd_ptr, &rfd_u32).is_err()
                    || write_user_pod(pipefd_ptr + core::mem::size_of::<u32>(), &wfd_u32).is_err()
                {
                    return linux_errno(crate::modules::posix_consts::errno::EFAULT);
                }
                0
            }
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_pipe"))]
    {
        let _ = (pipefd_ptr, flags);
        linux_errno(crate::modules::posix_consts::errno::EMFILE)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_dup(oldfd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::dup(oldfd as u32) {
            Ok(fd) => return fd as usize,
            Err(crate::modules::posix::PosixErrno::BadFileDescriptor) => {}
            Err(err) => return linux_errno(err.code()),
        }
    }
    #[cfg(feature = "posix_net")]
    {
        match crate::modules::libnet::posix_dup_errno(oldfd as u32) {
            Ok(fd) => return fd as usize,
            Err(crate::modules::libnet::PosixErrno::BadFileDescriptor) => {}
            Err(err) => return linux_errno(err.code()),
        }
    }
    #[cfg(all(not(feature = "posix_fs"), not(feature = "posix_net")))]
    {
        let _ = oldfd;
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }

    #[cfg(any(feature = "posix_fs", feature = "posix_net"))]
    {
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_dup2(oldfd: usize, newfd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::dup2(oldfd as u32, newfd as u32) {
            Ok(fd) => return fd as usize,
            Err(crate::modules::posix::PosixErrno::BadFileDescriptor) => {}
            Err(err) => return linux_errno(err.code()),
        }
    }
    #[cfg(feature = "posix_net")]
    {
        match crate::modules::libnet::posix_dup2_errno(oldfd as u32, newfd as u32) {
            Ok(fd) => return fd as usize,
            Err(crate::modules::libnet::PosixErrno::BadFileDescriptor) => {}
            Err(err) => return linux_errno(err.code()),
        }
    }
    #[cfg(all(not(feature = "posix_fs"), not(feature = "posix_net")))]
    {
        let _ = (oldfd, newfd);
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }

    #[cfg(any(feature = "posix_fs", feature = "posix_net"))]
    {
        linux_errno(crate::modules::posix_consts::errno::EBADF)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_dup3(oldfd: usize, newfd: usize, flags: usize) -> usize {
    const O_CLOEXEC: usize = 0x80000;

    if oldfd == newfd {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & !O_CLOEXEC) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let duped = sys_linux_dup2(oldfd, newfd);
    if duped == linux_errno(crate::modules::posix_consts::errno::EBADF)
        || duped == linux_errno(crate::modules::posix_consts::errno::EINVAL)
    {
        return duped;
    }

    if (flags & O_CLOEXEC) != 0 {
        linux_fd_set_descriptor_flags(newfd as u32, LINUX_FD_CLOEXEC);
    } else {
        linux_fd_clear_descriptor_flags(newfd as u32);
    }
    duped
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_fcntl(fd: usize, cmd: usize, arg: usize) -> usize {
    const F_DUPFD: usize = 0;
    const F_GETFD: usize = 1;
    const F_SETFD: usize = 2;
    const F_GETFL: usize = 3;
    const F_SETFL: usize = 4;
    const F_GETLK: usize = 5;
    const F_SETLK: usize = 6;
    const F_SETLKW: usize = 7;
    const F_SETOWN: usize = 8;
    const F_GETOWN: usize = 9;
    const F_OFD_GETLK: usize = 36;
    const F_OFD_SETLK: usize = 37;
    const F_OFD_SETLKW: usize = 38;
    const F_DUPFD_CLOEXEC: usize = 1030;
    const F_SETPIPE_SZ: usize = 1031;
    const F_GETPIPE_SZ: usize = 1032;
    const F_UNLCK: i16 = 2;
    const PIPE_BUF_SIZE: usize = 65536;

    match cmd {
        F_DUPFD => {
            #[cfg(feature = "posix_fs")]
            {
                match crate::modules::posix::fs::dup_at_least(fd as u32, arg as u32) {
                    Ok(newfd) => {
                        linux_fd_clear_descriptor_flags(newfd);
                        newfd as usize
                    }
                    Err(err) => linux_errno(err.code()),
                }
            }
            #[cfg(not(feature = "posix_fs"))]
            {
                let _ = arg;
                sys_linux_dup(fd)
            }
        }
        F_DUPFD_CLOEXEC => {
            #[cfg(feature = "posix_fs")]
            {
                match crate::modules::posix::fs::dup_at_least(fd as u32, arg as u32) {
                    Ok(newfd) => {
                        linux_fd_set_descriptor_flags(newfd, LINUX_FD_CLOEXEC);
                        newfd as usize
                    }
                    Err(err) => linux_errno(err.code()),
                }
            }
            #[cfg(not(feature = "posix_fs"))]
            {
                let _ = arg;
                sys_linux_dup(fd)
            }
        }
        F_GETFD => linux_fd_get_descriptor_flags(fd as u32) & LINUX_FD_CLOEXEC,
        F_SETFD => {
            linux_fd_set_descriptor_flags(fd as u32, arg);
            0
        }
        F_GETFL => {
            #[cfg(feature = "posix_fs")]
            {
                match crate::modules::posix::fs::fcntl_get_status_flags(fd as u32) {
                    Ok(flags) => return flags as usize,
                    Err(crate::modules::posix::PosixErrno::BadFileDescriptor) => {}
                    Err(err) => return linux_errno(err.code()),
                }
            }
            #[cfg(feature = "posix_net")]
            {
                match crate::modules::libnet::posix_fcntl_getfl_errno(fd as u32) {
                    Ok(flags) => return flags.bits() as usize,
                    Err(crate::modules::libnet::PosixErrno::BadFileDescriptor) => {}
                    Err(err) => return linux_errno(err.code()),
                }
            }
            #[cfg(all(not(feature = "posix_fs"), not(feature = "posix_net")))]
            {
                0o02
            }
            #[cfg(any(feature = "posix_fs", feature = "posix_net"))]
            {
                linux_errno(crate::modules::posix_consts::errno::EBADF)
            }
        }
        F_SETFL => {
            #[cfg(feature = "posix_fs")]
            {
                match crate::modules::posix::fs::fcntl_set_status_flags(fd as u32, arg as u32) {
                    Ok(()) => return 0,
                    Err(crate::modules::posix::PosixErrno::BadFileDescriptor) => {}
                    Err(err) => return linux_errno(err.code()),
                }
            }
            #[cfg(feature = "posix_net")]
            {
                let flags = crate::modules::libnet::PosixFdFlags::from_bits_truncate(arg as u32);
                match crate::modules::libnet::posix_fcntl_setfl_errno(fd as u32, flags) {
                    Ok(()) => return 0,
                    Err(crate::modules::libnet::PosixErrno::BadFileDescriptor) => {}
                    Err(err) => return linux_errno(err.code()),
                }
            }
            #[cfg(all(not(feature = "posix_fs"), not(feature = "posix_net")))]
            {
                let _ = arg;
                0
            }
            #[cfg(any(feature = "posix_fs", feature = "posix_net"))]
            {
                linux_errno(crate::modules::posix_consts::errno::EBADF)
            }
        }
        F_GETLK | F_OFD_GETLK => {
            if arg == 0 {
                return linux_errno(crate::modules::posix_consts::errno::EFAULT);
            }
            write_user_pod(arg, &F_UNLCK)
                .map(|_| 0usize)
                .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
        }
        F_SETLK | F_SETLKW | F_OFD_SETLK | F_OFD_SETLKW => 0,
        F_GETOWN => {
            #[cfg(feature = "posix_process")]
            {
                crate::modules::posix::process::getpid() as usize
            }
            #[cfg(not(feature = "posix_process"))]
            {
                sys_linux_getpid()
            }
        }
        F_SETOWN => {
            let _ = arg;
            0
        }
        F_GETPIPE_SZ => PIPE_BUF_SIZE,
        F_SETPIPE_SZ => arg
            .max(4096)
            .min(linux_shim_pipe_set_max_size())
            .next_power_of_two(),
        _ => {
            let _ = (fd, arg);
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        }
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_close_range(first: usize, last: usize, flags: usize) -> usize {
    const CLOSE_RANGE_UNSHARE: usize = 1 << 1;
    const CLOSE_RANGE_CLOEXEC: usize = 1 << 2;

    if first > last {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & !(CLOSE_RANGE_UNSHARE | CLOSE_RANGE_CLOEXEC)) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let cap_last = core::cmp::min(last, first.saturating_add(8192));
    for fd in first..=cap_last {
        if (flags & CLOSE_RANGE_CLOEXEC) != 0 {
            linux_fd_set_descriptor_flags(fd as u32, LINUX_FD_CLOEXEC);
        } else {
            let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd);
        }
    }
    0
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_pidfd_open(pid: usize, flags: usize) -> usize {
    // Basic shim: validates arguments and returns an fd-backed token when possible.
    if pid == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & !crate::kernel::syscalls::syscalls_consts::linux::PIDFD_NONBLOCK) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if !linux_task_exists(pid) {
        return linux_errno(crate::modules::posix_consts::errno::ESRCH);
    }

    let caller = linux_current_tid();
    if caller != pid && caller != 1 {
        return linux_errno(crate::modules::posix_consts::errno::EPERM);
    }

    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        let path = alloc::format!("/.pidfd-{}", pid);
        match crate::modules::posix::fs::openat(fs_id, "/", &path, true) {
            Ok(fd) => {
                LINUX_PIDFD_MAP.lock().insert(
                    fd,
                    LinuxPidFdEntry {
                        target_pid: pid,
                        owner_tid: caller,
                    },
                );
                fd as usize
            }
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let _ = (pid, flags);
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_pidfd_send_signal(
    pidfd: usize,
    sig: usize,
    _info_ptr: usize,
    flags: usize,
) -> usize {
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if sig > 64 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let entry = match linux_pidfd_entry_for_caller(pidfd) {
        Ok(v) => v,
        Err(err) => return err,
    };

    if sig == 0 {
        return 0;
    }
    crate::kernel::syscalls::linux_shim::task_time::sys_linux_kill(entry.target_pid, sig)
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_pidfd_getfd(
    pidfd: usize,
    targetfd: usize,
    flags: usize,
) -> usize {
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    let entry = match linux_pidfd_entry_for_caller(pidfd) {
        Ok(v) => v,
        Err(err) => return err,
    };
    let caller_tid = linux_current_tid();
    if !linux_pidfd_getfd_access_allowed(caller_tid, entry.target_pid) {
        return linux_errno(crate::modules::posix_consts::errno::EPERM);
    }

    // Best-effort shim behavior: duplicate into caller namespace after access checks.
    sys_linux_dup(targetfd)
}

#[cfg(not(feature = "linux_compat"))]
fn linux_shim_pipe_set_max_size() -> usize {
    const MIN_CAP: usize = 64 * 1024;
    const MAX_CAP: usize = 16 * 1024 * 1024;

    crate::config::KernelConfig::launch_max_boot_image_bytes().clamp(MIN_CAP, MAX_CAP)
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_net"))]
mod tests {
    use super::*;

    #[test_case]
    fn socket_fd_close_falls_back_to_network_layer() {
        let (fd_a, fd_b) = crate::modules::posix::net::socketpair_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("socketpair");

        assert_eq!(
            crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_a as usize),
            0
        );
        assert_eq!(
            crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_b as usize),
            0
        );
    }

    #[test_case]
    fn socket_fd_dup_and_dup3_preserve_linux_descriptor_flags() {
        let (fd_a, fd_b) = crate::modules::posix::net::socketpair_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("socketpair");

        let duped = sys_linux_dup(fd_a as usize);
        assert!(duped > 2);

        let target_fd = duped + 17;
        assert_eq!(sys_linux_dup3(fd_a as usize, target_fd, 0x80000), target_fd);
        assert_eq!(
            linux_fd_get_descriptor_flags(target_fd as u32) & LINUX_FD_CLOEXEC,
            LINUX_FD_CLOEXEC
        );

        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(duped);
        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(target_fd);
        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_a as usize);
        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_b as usize);
    }

    #[test_case]
    fn socket_fd_fcntl_status_flags_use_network_backend() {
        let (fd_a, fd_b) = crate::modules::posix::net::socketpair_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("socketpair");

        let getfl = sys_linux_fcntl(fd_a as usize, 3, 0);
        assert_ne!(
            getfl,
            linux_errno(crate::modules::posix_consts::errno::EBADF)
        );

        assert_eq!(sys_linux_fcntl(fd_a as usize, 4, 0), 0);

        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_a as usize);
        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(fd_b as usize);
    }
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_fs"))]
mod pidfd_tests {
    use super::*;

    #[test_case]
    fn pidfd_open_rejects_zero_pid() {
        assert_eq!(
            sys_linux_pidfd_open(0, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn pidfd_send_signal_rejects_unknown_pidfd() {
        assert_eq!(
            sys_linux_pidfd_send_signal(424242, 0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EBADF)
        );
    }

    #[test_case]
    fn pidfd_open_rejects_missing_task() {
        assert_eq!(
            sys_linux_pidfd_open(9_999_999, 0),
            linux_errno(crate::modules::posix_consts::errno::ESRCH)
        );
    }

    #[test_case]
    fn pidfd_getfd_rejects_nonzero_flags() {
        let pidfd = sys_linux_pidfd_open(1, 0);
        if pidfd >= linux_errno(crate::modules::posix_consts::errno::MAX_ERRNO) {
            return;
        }
        let rc = sys_linux_pidfd_getfd(pidfd, 0, 1);
        assert_eq!(rc, linux_errno(crate::modules::posix_consts::errno::EINVAL));
        let _ = crate::kernel::syscalls::linux_shim::fs::sys_linux_close(pidfd);
    }

    #[test_case]
    fn pidfd_getfd_access_matrix_allows_self_and_supervisor() {
        assert!(linux_pidfd_getfd_access_allowed(77, 77));
        assert!(linux_pidfd_getfd_access_allowed(1, 77));
        assert!(!linux_pidfd_getfd_access_allowed(33, 77));
    }
}
