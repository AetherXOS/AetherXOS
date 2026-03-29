use super::config::LinuxCompatConfig;
use crate::kernel::syscalls::syscalls_consts::{
    USER_SPACE_BOTTOM_INCLUSIVE, USER_SPACE_TOP_EXCLUSIVE,
};
use crate::kernel::syscalls::{user_readable_range_valid, with_user_read_bytes};
use alloc::format;

#[path = "helpers/io.rs"]
mod io;

pub use io::{read_user_c_string, read_user_iovec, read_user_sockaddr, read_user_string_vec};
#[cfg(feature = "posix_net")]
pub use io::{read_sockaddr_in, write_sockaddr_in};
#[cfg(feature = "posix_fs")]
pub use io::resolve_linux_at;

#[cfg(any(feature = "posix_fs", feature = "posix_net"))]
use crate::kernel::syscalls::with_user_write_bytes;

use super::types::LinuxIoVec;
#[cfg(feature = "posix_process")]
use super::types::LinuxRusage;
#[cfg(feature = "posix_net")]
use super::types::LinuxSockAddrIn;
#[cfg(feature = "posix_fs")]
use super::types::LinuxStat;
#[cfg(any(feature = "posix_process", feature = "posix_net"))]
use super::types::LinuxTimeval;

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const NANOS_PER_MICROSECOND: u64 = 1_000;

#[cfg(feature = "posix_fs")]
pub use super::wrappers::*;

#[cfg(feature = "posix_process")]
#[inline(always)]
fn ns_to_linux_timeval(total_ns: u64) -> LinuxTimeval {
    LinuxTimeval {
        tv_sec: (total_ns / NANOS_PER_SECOND) as i64,
        tv_usec: ((total_ns % NANOS_PER_SECOND) / NANOS_PER_MICROSECOND) as i64,
    }
}

#[macro_export]
macro_rules! uptr {
    ($val:expr) => {
        $crate::modules::linux_compat::wrappers::UserPtr::new($val)
    };
}
#[macro_export]
macro_rules! fd {
    ($val:expr) => {
        $crate::modules::linux_compat::wrappers::Fd::from($val)
    };
}

#[inline(always)]
pub fn arg5_to_zero(value: usize) -> usize {
    value
}

#[cfg(feature = "posix_fs")]
pub fn fill_linux_stat(pstat: crate::modules::posix::fs::PosixStat) -> LinuxStat {
    let mut mode = pstat.mode as u32;
    if pstat.is_dir {
        mode |= crate::kernel::syscalls::syscalls_consts::linux::S_IFDIR;
    } else if pstat.is_symlink {
        mode |= crate::kernel::syscalls::syscalls_consts::linux::S_IFLNK;
    } else {
        mode |= crate::kernel::syscalls::syscalls_consts::linux::S_IFREG;
    }

    LinuxStat {
        st_dev: 1,
        st_ino: pstat.ino,
        st_nlink: 1,
        st_mode: mode,
        st_uid: pstat.uid,
        st_gid: pstat.gid,
        __pad0: 0,
        st_rdev: 0,
        st_size: pstat.size as i64,
        st_blksize: crate::kernel::syscalls::syscalls_consts::linux::STAT_BLKSIZE,
        st_blocks: (pstat.size as i64
            + (crate::kernel::syscalls::syscalls_consts::linux::STAT_BLOCK_SIZE - 1))
            / crate::kernel::syscalls::syscalls_consts::linux::STAT_BLOCK_SIZE,
        st_atime: pstat.atime,
        st_atime_nsec: 0,
        st_mtime: pstat.mtime,
        st_mtime_nsec: 0,
        st_ctime: pstat.ctime,
        st_ctime_nsec: 0,
        __unused: [0; 3],
    }
}

#[cfg(feature = "posix_process")]
pub fn fill_linux_rusage(pru: crate::modules::posix::process::PosixRusage) -> LinuxRusage {
    let ns_per_tick = crate::config::KernelConfig::time_slice();
    let utime_ns = pru.ru_utime_ticks * ns_per_tick;
    let stime_ns = pru.ru_stime_ticks * ns_per_tick;

    LinuxRusage {
        ru_utime: ns_to_linux_timeval(utime_ns),
        ru_stime: ns_to_linux_timeval(stime_ns),
        ru_maxrss: pru.ru_maxrss as i64,
        ru_ixrss: 0,
        ru_idrss: 0,
        ru_isrss: 0,
        ru_minflt: pru.ru_minflt as i64,
        ru_majflt: pru.ru_majflt as i64,
        ru_nswap: pru.ru_nswap as i64,
        ru_inblock: 0,
        ru_oublock: 0,
        ru_msgsnd: 0,
        ru_msgrcv: 0,
        ru_nsignals: 0,
        ru_nvcsw: 0,
        ru_nivcsw: 0,
    }
}

#[inline(always)]
pub fn linux_errno(errno: i32) -> usize {
    (-(errno as isize)) as usize
}

#[inline(always)]
pub fn linux_nosys() -> usize {
    linux_errno(crate::modules::posix_consts::errno::ENOSYS)
}

#[inline(always)]
pub fn linux_inval() -> usize {
    linux_errno(crate::modules::posix_consts::errno::EINVAL)
}

#[inline(always)]
pub fn linux_fault() -> usize {
    linux_errno(crate::modules::posix_consts::errno::EFAULT)
}

#[inline(always)]
pub fn linux_esrch() -> usize {
    linux_errno(crate::modules::posix_consts::errno::ESRCH)
}

#[inline(always)]
pub fn linux_eacces() -> usize {
    linux_errno(crate::modules::posix_consts::errno::EACCES)
}

#[inline(always)]
pub fn linux_enomem() -> usize {
    linux_errno(crate::modules::posix_consts::errno::ENOMEM)
}

#[inline(always)]
pub fn linux_eperm() -> usize {
    linux_errno(crate::modules::posix_consts::errno::EPERM)
}

#[cfg(feature = "posix_fs")]
#[macro_export]
macro_rules! resolve_at {
    ($dirfd:expr, $path_ptr:expr) => {
        match $crate::modules::linux_compat::helpers::resolve_linux_at($dirfd, $path_ptr) {
            Ok(v) => v,
            Err(e) => return e,
        }
    };
}

#[macro_export]
macro_rules! require_posix_fs {
    (($($arg:ident),*) => $body:expr) => {
        {
            #[cfg(feature = "posix_fs")]
            return { $body };
            #[cfg(not(feature = "posix_fs"))]
            return {
                $(let _ = $arg;)*
                $crate::modules::linux_compat::helpers::linux_nosys()
            };
        }
    };
}

/// Centralized Linux syscall tracing.
#[macro_export]
macro_rules! linux_trace {
    ($($arg:tt)*) => {
        if $crate::modules::linux_compat::config::LinuxCompatConfig::VERBOSE_LOGS {
            $crate::klog_info!($($arg)*);
        }
    };
}

/// Unified macro for path-based syscalls.
/// Handles:
/// 1. DirFD resolution (relative to chroot).
/// 2. User space string reading.
/// 3. Read-only filesystem checks for write operations.
#[macro_export]
macro_rules! syscall_path_at {
    ($dirfd:expr, $pathname:expr, $op:ident, $fs_id:ident, $dir:ident, $path:ident, $abs:ident) => {
        let ($fs_id, $dir, $path) = resolve_at!($dirfd, $pathname);
        let $abs = match $crate::modules::posix::fs::resolve_at_path($fs_id, &$dir, &$path) {
            Ok(p) => p,
            Err(e) => return $crate::modules::linux_compat::helpers::linux_errno(e.code()),
        };
        if (stringify!($op) == "write" || stringify!($op) == "create")
            && $crate::modules::linux_compat::fs::mount::linux_path_is_readonly(&$abs)
        {
            return $crate::modules::linux_compat::error::err::rofs();
        }
    };
}

/// Helper to write structs to user space with standard error handling.
#[macro_export]
macro_rules! write_user_struct {
    ($ptr:expr, $val:expr) => {
        match $ptr.write(&$val) {
            Ok(_) => 0usize,
            Err(e) => return e,
        }
    };
}

#[macro_export]
macro_rules! require_posix_net {
    (($($arg:ident),*) => $body:expr) => {
        {
            #[cfg(feature = "posix_net")]
            return { $body };
            #[cfg(not(feature = "posix_net"))]
            return {
                $(let _ = $arg;)*
                $crate::modules::linux_compat::helpers::linux_nosys()
            };
        }
    };
}

#[macro_export]
macro_rules! require_posix_process {
    (($($arg:ident),*) => $body:expr) => {
        {
            #[cfg(feature = "posix_process")]
            return { $body };
            #[cfg(not(feature = "posix_process"))]
            return {
                $(let _ = $arg;)*
                crate::modules::linux_compat::helpers::linux_nosys()
            };
        }
    };
}

#[macro_export]
macro_rules! require_posix_time {
    (($($arg:ident),*) => $body:expr) => {
        {
            #[cfg(feature = "posix_time")]
            return { $body };
            #[cfg(not(feature = "posix_time"))]
            return {
                $(let _ = $arg;)*
                crate::modules::linux_compat::helpers::linux_nosys()
            };
        }
    };
}

#[macro_export]
macro_rules! require_posix_ipc {
    (($($arg:ident),*) => $body:expr) => {
        {
            #[cfg(feature = "posix_ipc")]
            return { $body };
            #[cfg(not(feature = "posix_ipc"))]
            return {
                $(let _ = $arg;)*
                crate::modules::linux_compat::helpers::linux_nosys()
            };
        }
    };
}

#[macro_export]
macro_rules! require_posix_thread {
    (($($arg:ident),*) => $body:expr) => {
        {
            #[cfg(feature = "posix_thread")]
            return { $body };
            #[cfg(not(feature = "posix_thread"))]
            return {
                $(let _ = $arg;)*
                crate::modules::linux_compat::helpers::linux_nosys()
            };
        }
    };
}

#[macro_export]
macro_rules! require_posix_signal {
    (($($arg:ident),*) => $body:expr) => {
        {
            #[cfg(feature = "posix_signal")]
            return { $body };
            #[cfg(not(feature = "posix_signal"))]
            return {
                $(let _ = $arg;)*
                crate::modules::linux_compat::helpers::linux_nosys()
            };
        }
    };
}

#[macro_export]
macro_rules! require_posix_pipe {
    (($($arg:ident),*) => $body:expr) => {
        {
            #[cfg(feature = "posix_pipe")]
            return { $body };
            #[cfg(not(feature = "posix_pipe"))]
            return {
                $(let _ = $arg;)*
                crate::modules::linux_compat::helpers::linux_nosys()
            };
        }
    };
}

#[macro_export]
macro_rules! require_posix_io {
    (($($arg:ident),*) => $body:expr) => {
        {
            #[cfg(feature = "posix_io")]
            return { $body };
            #[cfg(not(feature = "posix_io"))]
            return {
                $(let _ = $arg;)*
                crate::modules::linux_compat::helpers::linux_nosys()
            };
        }
    };
}

#[macro_export]
macro_rules! require_posix_mman {
    (($($arg:ident),*) => $body:expr) => {
        {
            #[cfg(feature = "posix_mman")]
            return { $body };
            #[cfg(not(feature = "posix_mman"))]
            return {
                $(let _ = $arg;)*
                crate::modules::linux_compat::helpers::linux_nosys()
            };
        }
    };
}
