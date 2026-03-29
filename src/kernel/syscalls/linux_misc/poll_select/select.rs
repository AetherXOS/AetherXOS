#[cfg(not(feature = "linux_compat"))]
use super::compat::LINUX_FD_SETSIZE_COMPAT;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
use super::compat::{
    read_fd_set_compat, write_fd_set_compat, LinuxTimespecCompat, LinuxTimevalCompat,
};
use super::*;

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxPselect6SigmaskCompat {
    ss_ptr: usize,
    ss_len: usize,
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn timeout_to_retries(timeout: usize) -> Result<usize, usize> {
    if timeout == 0 {
        return Ok(crate::config::KernelConfig::libnet_posix_blocking_recv_retries());
    }

    let tv = with_user_read_bytes(timeout, core::mem::size_of::<LinuxTimevalCompat>(), |src| {
        LinuxTimevalCompat {
            tv_sec: i64::from_ne_bytes([
                src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
            ]),
            tv_usec: i64::from_ne_bytes([
                src[8], src[9], src[10], src[11], src[12], src[13], src[14], src[15],
            ]),
        }
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;

    if tv.tv_sec < 0 || tv.tv_usec < 0 || tv.tv_usec >= 1_000_000 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let total_ns = (tv.tv_sec as u128)
        .saturating_mul(1_000_000_000u128)
        .saturating_add((tv.tv_usec as u128).saturating_mul(1_000u128));
    if total_ns == 0 {
        return Ok(0);
    }

    let tick_ns = core::cmp::max(crate::generated_consts::TIME_SLICE_NS as u128, 1u128);
    Ok(((total_ns + tick_ns - 1) / tick_ns) as usize)
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn timespec_timeout_ptr_to_retries(timeout_ptr: usize) -> Result<usize, usize> {
    if timeout_ptr == 0 {
        return Ok(crate::config::KernelConfig::libnet_posix_blocking_recv_retries());
    }

    let ts = with_user_read_bytes(
        timeout_ptr,
        core::mem::size_of::<LinuxTimespecCompat>(),
        |src| LinuxTimespecCompat {
            tv_sec: i64::from_ne_bytes([
                src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
            ]),
            tv_nsec: i64::from_ne_bytes([
                src[8], src[9], src[10], src[11], src[12], src[13], src[14], src[15],
            ]),
        },
    )
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;

    if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let total_ns = (ts.tv_sec as u128)
        .saturating_mul(1_000_000_000u128)
        .saturating_add(ts.tv_nsec as u128);
    if total_ns == 0 {
        return Ok(0);
    }

    let tick_ns = core::cmp::max(crate::generated_consts::TIME_SLICE_NS as u128, 1u128);
    Ok(((total_ns + tick_ns - 1) / tick_ns) as usize)
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn parse_pselect6_sigmask(sigmask_desc_ptr: usize) -> Result<Option<u64>, usize> {
    if sigmask_desc_ptr == 0 {
        return Ok(None);
    }

    let desc = with_user_read_bytes(
        sigmask_desc_ptr,
        core::mem::size_of::<LinuxPselect6SigmaskCompat>(),
        |src| LinuxPselect6SigmaskCompat {
            ss_ptr: usize::from_ne_bytes([
                src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
            ]),
            ss_len: usize::from_ne_bytes([
                src[8], src[9], src[10], src[11], src[12], src[13], src[14], src[15],
            ]),
        },
    )
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;

    if desc.ss_ptr == 0 {
        return Ok(None);
    }

    if desc.ss_len != core::mem::size_of::<u64>() {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let mask = with_user_read_bytes(desc.ss_ptr, core::mem::size_of::<u64>(), |src| {
        u64::from_ne_bytes([
            src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
        ])
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;

    Ok(Some(mask))
}

#[cfg(all(
    not(feature = "linux_compat"),
    feature = "posix_net",
    feature = "posix_signal"
))]
fn run_with_temporary_sigmask(mask: Option<u64>, op: impl FnOnce() -> usize) -> usize {
    let Some(new_mask) = mask else {
        return op();
    };

    let old_mask = match crate::modules::posix::signal::sigprocmask(
        crate::modules::posix::signal::SigmaskHow::SetMask,
        Some(new_mask),
    ) {
        Ok(v) => v,
        Err(err) => return linux_errno(err.code()),
    };

    let ret = op();
    let _ = crate::modules::posix::signal::sigprocmask(
        crate::modules::posix::signal::SigmaskHow::SetMask,
        Some(old_mask),
    );
    ret
}

#[cfg(all(
    not(feature = "linux_compat"),
    feature = "posix_net",
    not(feature = "posix_signal")
))]
fn run_with_temporary_sigmask(_mask: Option<u64>, op: impl FnOnce() -> usize) -> usize {
    op()
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn sys_linux_select_with_retries(
    nfds: usize,
    readfds: usize,
    writefds: usize,
    exceptfds: usize,
    retries: usize,
) -> usize {
    let read_in = if readfds == 0 {
        alloc::vec::Vec::new()
    } else {
        match read_fd_set_compat(readfds, nfds) {
            Ok(v) => v,
            Err(err) => return err,
        }
    };

    let write_in = if writefds == 0 {
        alloc::vec::Vec::new()
    } else {
        match read_fd_set_compat(writefds, nfds) {
            Ok(v) => v,
            Err(err) => return err,
        }
    };

    let except_in = if exceptfds == 0 {
        alloc::vec::Vec::new()
    } else {
        match read_fd_set_compat(exceptfds, nfds) {
            Ok(v) => v,
            Err(err) => return err,
        }
    };

    let result = match crate::modules::libnet::posix_select_errno(
        &read_in, &write_in, &except_in, retries,
    ) {
        Ok(v) => v,
        Err(err) => return linux_errno(err.code()),
    };

    if readfds != 0 && write_fd_set_compat(readfds, &result.readable, nfds).is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    if writefds != 0 && write_fd_set_compat(writefds, &result.writable, nfds).is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    if exceptfds != 0 && write_fd_set_compat(exceptfds, &result.exceptional, nfds).is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }

    result.readable.len() + result.writable.len() + result.exceptional.len()
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_select(
    nfds: usize,
    readfds: usize,
    writefds: usize,
    exceptfds: usize,
    timeout: usize,
) -> usize {
    if nfds > LINUX_FD_SETSIZE_COMPAT {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_net")]
    {
        let retries = match timeout_to_retries(timeout) {
            Ok(v) => v,
            Err(err) => return err,
        };
        return sys_linux_select_with_retries(nfds, readfds, writefds, exceptfds, retries);
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (nfds, readfds, writefds, exceptfds, timeout);
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_pselect6(
    nfds: usize,
    readfds: usize,
    writefds: usize,
    exceptfds: usize,
    timeout_ptr: usize,
    sigmask_desc_ptr: usize,
) -> usize {
    if nfds > LINUX_FD_SETSIZE_COMPAT {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_net")]
    {
        let mask = match parse_pselect6_sigmask(sigmask_desc_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };

        let retries = match timespec_timeout_ptr_to_retries(timeout_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };

        run_with_temporary_sigmask(mask, || {
            sys_linux_select_with_retries(nfds, readfds, writefds, exceptfds, retries)
        })
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (
            nfds,
            readfds,
            writefds,
            exceptfds,
            timeout_ptr,
            sigmask_desc_ptr,
        );
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_net"))]
mod tests {
    use super::*;

    #[test_case]
    fn select_invalid_timeout_pointer_returns_efault() {
        assert_eq!(
            sys_linux_select(0, 0, 0, 0, 1),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn pselect6_invalid_sigmask_descriptor_pointer_returns_efault() {
        assert_eq!(
            sys_linux_pselect6(0, 0, 0, 0, 0, 1),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn select_rejects_invalid_timeval_usec() {
        let tv = LinuxTimevalCompat {
            tv_sec: 0,
            tv_usec: 1_000_000,
        };
        assert_eq!(
            sys_linux_select(0, 0, 0, 0, (&tv as *const LinuxTimevalCompat) as usize),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn pselect6_rejects_invalid_timespec_nsec() {
        let ts = LinuxTimespecCompat {
            tv_sec: 0,
            tv_nsec: -1,
        };
        assert_eq!(
            sys_linux_pselect6(0, 0, 0, 0, (&ts as *const LinuxTimespecCompat) as usize, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn pselect6_rejects_invalid_sigmask_length() {
        let mask = 0u64;
        let desc = LinuxPselect6SigmaskCompat {
            ss_ptr: (&mask as *const u64) as usize,
            ss_len: 4,
        };
        assert_eq!(
            sys_linux_pselect6(
                0,
                0,
                0,
                0,
                0,
                (&desc as *const LinuxPselect6SigmaskCompat) as usize,
            ),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }
}
