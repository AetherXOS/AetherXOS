#[cfg(all(not(feature = "linux_compat"), feature = "posix_process"))]
use crate::kernel::syscalls::with_user_read_bytes;
#[cfg(not(feature = "linux_compat"))]
use crate::kernel::syscalls::{linux_errno, with_user_write_bytes};

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_getrlimit(resource: usize, rlim_ptr: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        match crate::modules::posix::process::getrlimit(resource as i32) {
            Ok((cur, max)) => with_user_write_bytes(rlim_ptr, 16, |dst| {
                dst[0..8].copy_from_slice(&cur.to_ne_bytes());
                dst[8..16].copy_from_slice(&max.to_ne_bytes());
                0
            })
            .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT)),
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = resource;
        let unlimited = u64::MAX;
        with_user_write_bytes(rlim_ptr, 16, |dst| {
            dst[0..8].copy_from_slice(&unlimited.to_ne_bytes());
            dst[8..16].copy_from_slice(&unlimited.to_ne_bytes());
            0
        })
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_setrlimit(resource: usize, rlim_ptr: usize) -> usize {
    #[cfg(feature = "posix_process")]
    {
        let (cur, max) = match with_user_read_bytes(rlim_ptr, 16, |src| {
            let cur = u64::from_ne_bytes([
                src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
            ]);
            let max = u64::from_ne_bytes([
                src[8], src[9], src[10], src[11], src[12], src[13], src[14], src[15],
            ]);
            (cur, max)
        }) {
            Ok(v) => v,
            Err(_) => return linux_errno(crate::modules::posix_consts::errno::EFAULT),
        };
        match crate::modules::posix::process::setrlimit(resource as i32, cur, max) {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_process"))]
    {
        let _ = (resource, rlim_ptr);
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_prlimit64(
    pid: usize,
    resource: usize,
    new_rlim_ptr: usize,
    old_rlim_ptr: usize,
) -> usize {
    let _ = pid;
    if old_rlim_ptr != 0 {
        let ret = sys_linux_getrlimit(resource, old_rlim_ptr);
        if ret != 0 {
            return ret;
        }
    }
    if new_rlim_ptr != 0 {
        return sys_linux_setrlimit(resource, new_rlim_ptr);
    }
    0
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod tests {
    use super::*;

    #[test_case]
    fn getrlimit_invalid_pointer_returns_efault() {
        assert_eq!(
            sys_linux_getrlimit(0, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn prlimit_invalid_old_pointer_returns_efault() {
        assert_eq!(
            sys_linux_prlimit64(0, 0, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }
}
