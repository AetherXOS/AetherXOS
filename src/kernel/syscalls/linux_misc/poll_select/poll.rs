#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
use super::compat::{LinuxPollFdCompat, LinuxTimespecCompat};
use super::*;

#[cfg(all(
    feature = "linux_poll_timeout_ms",
    feature = "linux_poll_timeout_retries"
))]
compile_error!("linux_poll_timeout_ms and linux_poll_timeout_retries are mutually exclusive");

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn parse_sigmask(sigmask_ptr: usize, sigset_size: usize) -> Result<Option<u64>, usize> {
    if sigmask_ptr == 0 {
        return Ok(None);
    }

    if sigset_size != core::mem::size_of::<u64>() {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let mask = with_user_read_bytes(sigmask_ptr, core::mem::size_of::<u64>(), |src| {
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
fn timeout_ptr_to_retries(timeout_ptr: usize) -> Result<usize, usize> {
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

#[cfg(all(not(feature = "linux_compat"), feature = "linux_poll_timeout_ms"))]
fn poll_timeout_to_retries(timeout: usize) -> usize {
    if timeout == usize::MAX {
        return crate::config::KernelConfig::libnet_posix_blocking_recv_retries();
    }

    let timeout_ns = (timeout as u128).saturating_mul(1_000_000u128);
    if timeout_ns == 0 {
        return 0;
    }

    let tick_ns = core::cmp::max(crate::generated_consts::TIME_SLICE_NS as u128, 1u128);
    ((timeout_ns + tick_ns - 1) / tick_ns) as usize
}

#[cfg(all(not(feature = "linux_compat"), not(feature = "linux_poll_timeout_ms")))]
fn poll_timeout_to_retries(timeout: usize) -> usize {
    if timeout == usize::MAX {
        crate::config::KernelConfig::libnet_posix_blocking_recv_retries()
    } else {
        timeout
    }
}

#[cfg(not(feature = "linux_compat"))]
fn sys_linux_poll_with_retries(fds_ptr: usize, nfds: usize, retries: usize) -> usize {
    if nfds > linux_poll_fd_limit() {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if nfds == 0 {
        if retries != 0 {
            sys_yield();
        }
        return 0;
    }

    #[cfg(feature = "posix_net")]
    {
        let item_sz = core::mem::size_of::<LinuxPollFdCompat>();
        let total_sz = match item_sz.checked_mul(nfds) {
            Some(v) => v,
            None => return linux_errno(crate::modules::posix_consts::errno::EOVERFLOW),
        };

        let mut in_fds = alloc::vec::Vec::with_capacity(nfds);
        let rd = with_user_read_bytes(fds_ptr, total_sz, |src| {
            for i in 0..nfds {
                let base = i * item_sz;
                let fd =
                    i32::from_ne_bytes([src[base], src[base + 1], src[base + 2], src[base + 3]]);
                let events = i16::from_ne_bytes([src[base + 4], src[base + 5]]);
                let revents = i16::from_ne_bytes([src[base + 6], src[base + 7]]);
                in_fds.push(LinuxPollFdCompat {
                    fd,
                    events,
                    revents,
                });
            }
            0usize
        });
        if rd.is_err() {
            return linux_errno(crate::modules::posix_consts::errno::EFAULT);
        }

        let mut poll_fds = alloc::vec::Vec::with_capacity(nfds);
        for p in &in_fds {
            if p.fd < 0 {
                poll_fds.push(crate::modules::libnet::PosixPollFd::new(
                    0,
                    crate::modules::libnet::PosixPollEvents::empty(),
                ));
                continue;
            }
            poll_fds.push(crate::modules::libnet::PosixPollFd::new(
                p.fd as u32,
                crate::modules::libnet::PosixPollEvents::from_bits_truncate(p.events as u16),
            ));
        }

        let ready = match crate::modules::libnet::posix_poll_errno(&mut poll_fds, retries) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };

        let mut synthetic_ready = 0usize;
        for (i, p) in in_fds.iter().enumerate() {
            if p.fd < 0 {
                continue;
            }

            let synthetic_revents = crate::kernel::syscalls::linux_shim::net::userspace_display_poll_revents(
                p.fd as u32,
                p.events as u16,
            );
            if synthetic_revents == 0 {
                continue;
            }

            let was_empty = poll_fds[i].revents.is_empty();
            poll_fds[i].revents |= crate::modules::libnet::PosixPollEvents::from_bits_truncate(
                synthetic_revents,
            );
            if was_empty && !poll_fds[i].revents.is_empty() {
                synthetic_ready = synthetic_ready.saturating_add(1);
            }
        }

        let ready = ready.saturating_add(synthetic_ready);

        with_user_write_bytes(fds_ptr, total_sz, |dst| {
            for (i, p) in in_fds.iter().enumerate() {
                let base = i * item_sz;
                dst[base..base + 4].copy_from_slice(&p.fd.to_ne_bytes());
                dst[base + 4..base + 6].copy_from_slice(&p.events.to_ne_bytes());
                let revents_u16 = poll_fds[i].revents.bits();
                let revents_i16 = revents_u16 as i16;
                dst[base + 6..base + 8].copy_from_slice(&revents_i16.to_ne_bytes());
            }
            ready
        })
        .unwrap_or_else(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fds_ptr, nfds, retries);
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_poll(fds_ptr: usize, nfds: usize, timeout: usize) -> usize {
    let retries = poll_timeout_to_retries(timeout);
    sys_linux_poll_with_retries(fds_ptr, nfds, retries)
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn sys_linux_ppoll(
    fds_ptr: usize,
    nfds: usize,
    timeout_ptr: usize,
    _sigmask_ptr: usize,
    _sigset_size: usize,
) -> usize {
    #[cfg(feature = "posix_net")]
    {
        let mask = match parse_sigmask(_sigmask_ptr, _sigset_size) {
            Ok(v) => v,
            Err(err) => return err,
        };

        let retries = match timeout_ptr_to_retries(timeout_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        run_with_temporary_sigmask(mask, || sys_linux_poll_with_retries(fds_ptr, nfds, retries))
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (fds_ptr, nfds, timeout_ptr, _sigmask_ptr, _sigset_size);
        linux_errno(crate::modules::posix_consts::errno::ENOSYS)
    }
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_net"))]
mod tests {
    use super::*;

    #[test_case]
    fn ppoll_invalid_timeout_pointer_returns_efault() {
        assert_eq!(
            sys_linux_ppoll(0, 0, 1, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn ppoll_invalid_sigmask_pointer_returns_efault() {
        assert_eq!(
            sys_linux_ppoll(0, 0, 0, 1, core::mem::size_of::<u64>()),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn ppoll_rejects_invalid_sigset_size() {
        let mask = 0u64;
        assert_eq!(
            sys_linux_ppoll(0, 0, 0, (&mask as *const u64) as usize, 4),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn ppoll_rejects_negative_timeout_nsec() {
        let ts = LinuxTimespecCompat {
            tv_sec: 0,
            tv_nsec: -1,
        };
        assert_eq!(
            sys_linux_ppoll(0, 0, (&ts as *const LinuxTimespecCompat) as usize, 0, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn poll_rejects_nfds_over_limit() {
        assert_eq!(
            sys_linux_poll(0, linux_poll_fd_limit() + 1, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "linux_poll_timeout_ms"))]
mod timeout_mode_tests_ms {
    use super::*;

    #[test_case]
    fn poll_timeout_ms_zero_maps_to_zero_retries() {
        assert_eq!(poll_timeout_to_retries(0), 0);
    }

    #[test_case]
    fn poll_timeout_ms_max_maps_to_blocking_retries() {
        assert_eq!(
            poll_timeout_to_retries(usize::MAX),
            crate::config::KernelConfig::libnet_posix_blocking_recv_retries()
        );
    }

    #[test_case]
    fn poll_timeout_ms_uses_tick_based_round_up() {
        let timeout_ms = 7usize;
        let timeout_ns = (timeout_ms as u128).saturating_mul(1_000_000u128);
        let tick_ns = core::cmp::max(crate::generated_consts::TIME_SLICE_NS as u128, 1u128);
        let expected = ((timeout_ns + tick_ns - 1) / tick_ns) as usize;
        assert_eq!(poll_timeout_to_retries(timeout_ms), expected);
    }
}

#[cfg(all(
    test,
    not(feature = "linux_compat"),
    not(feature = "linux_poll_timeout_ms")
))]
mod timeout_mode_tests_retries {
    use super::*;

    #[test_case]
    fn poll_timeout_retries_mode_passthroughs_timeout() {
        assert_eq!(poll_timeout_to_retries(13), 13);
    }

    #[test_case]
    fn poll_timeout_retries_mode_max_maps_to_blocking_retries() {
        assert_eq!(
            poll_timeout_to_retries(usize::MAX),
            crate::config::KernelConfig::libnet_posix_blocking_recv_retries()
        );
    }
}
