#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
use super::compat::{LinuxPollFdCompat, LinuxTimespecCompat};
use super::*;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
use crate::kernel::syscalls::linux_shim::util::{read_user_pod, write_user_pod};

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

    let mask = read_user_pod::<u64>(sigmask_ptr)?;

    Ok(Some(super::compat::sanitize_wait_sigmask(mask)))
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

    let ts = read_user_pod::<LinuxTimespecCompat>(timeout_ptr)?;

    if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let total_ns = (ts.tv_sec as u128)
        .saturating_mul(1_000_000_000u128)
        .saturating_add(ts.tv_nsec as u128);
    Ok(super::compat::timeout_ns_to_retries(total_ns))
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn poll_entry_ptr(fds_ptr: usize, index: usize, item_sz: usize) -> Result<usize, usize> {
    fds_ptr
        .checked_add(index.saturating_mul(item_sz))
        .ok_or_else(|| linux_errno(crate::modules::posix_consts::errno::EOVERFLOW))
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn read_poll_fds_from_user(fds_ptr: usize, nfds: usize) -> Result<alloc::vec::Vec<LinuxPollFdCompat>, usize> {
    let item_sz = core::mem::size_of::<LinuxPollFdCompat>();
    let mut in_fds = alloc::vec::Vec::with_capacity(nfds);
    for i in 0..nfds {
        let entry_ptr = poll_entry_ptr(fds_ptr, i, item_sz)?;
        in_fds.push(read_user_pod::<LinuxPollFdCompat>(entry_ptr)?);
    }
    Ok(in_fds)
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn build_poll_request(in_fds: &[LinuxPollFdCompat]) -> alloc::vec::Vec<crate::modules::libnet::PosixPollFd> {
    in_fds
        .iter()
        .map(|p| {
            if p.fd < 0 {
                return crate::modules::libnet::PosixPollFd::new(
                    0,
                    crate::modules::libnet::PosixPollEvents::empty(),
                );
            }

            crate::modules::libnet::PosixPollFd::new(
                p.fd as u32,
                crate::modules::libnet::PosixPollEvents::from_bits_truncate(p.events as u16),
            )
        })
        .collect()
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn apply_synthetic_revents(
    in_fds: &[LinuxPollFdCompat],
    poll_fds: &mut [crate::modules::libnet::PosixPollFd],
) -> usize {
    let mut synthetic_ready = 0usize;
    for (i, p) in in_fds.iter().enumerate() {
        if p.fd < 0 {
            continue;
        }

        let synthetic_revents = crate::kernel::syscalls::linux_shim::net::userspace_display_poll_revents(
            p.fd as u32,
            p.events as u16,
        ) | super::super::timerfd_poll_revents(p.fd as u32, p.events as u16);
        if synthetic_revents == 0 {
            continue;
        }

        let was_empty = poll_fds[i].revents.is_empty();
        poll_fds[i].revents |=
            crate::modules::libnet::PosixPollEvents::from_bits_truncate(synthetic_revents);
        if was_empty && !poll_fds[i].revents.is_empty() {
            synthetic_ready = synthetic_ready.saturating_add(1);
        }
    }
    synthetic_ready
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn write_poll_fds_to_user(
    fds_ptr: usize,
    in_fds: &[LinuxPollFdCompat],
    poll_fds: &[crate::modules::libnet::PosixPollFd],
) -> Result<(), usize> {
    let item_sz = core::mem::size_of::<LinuxPollFdCompat>();
    for (i, p) in in_fds.iter().enumerate() {
        let entry_ptr = poll_entry_ptr(fds_ptr, i, item_sz)?;
        let out = LinuxPollFdCompat {
            fd: p.fd,
            events: p.events,
            revents: poll_fds[i].revents.bits() as i16,
        };
        write_user_pod(entry_ptr, &out)
            .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;
    }
    Ok(())
}

#[cfg(all(not(feature = "linux_compat"), feature = "linux_poll_timeout_ms"))]
fn poll_timeout_to_retries(timeout: usize) -> usize {
    if timeout == usize::MAX {
        return crate::config::KernelConfig::libnet_posix_blocking_recv_retries();
    }

    let timeout_ns = (timeout as u128).saturating_mul(1_000_000u128);
    super::compat::timeout_ns_to_retries(timeout_ns)
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
        let in_fds = match read_poll_fds_from_user(fds_ptr, nfds) {
            Ok(v) => v,
            Err(err) => return err,
        };
        let mut poll_fds = build_poll_request(&in_fds);

        let ready = match crate::modules::libnet::posix_poll_errno(&mut poll_fds, retries) {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };

        let synthetic_ready = apply_synthetic_revents(&in_fds, &mut poll_fds);

        let ready = ready.saturating_add(synthetic_ready);

        if let Err(err) = write_poll_fds_to_user(fds_ptr, &in_fds, &poll_fds) {
            return err;
        }

        ready
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
    fn ppoll_sigmask_sanitizes_unblockable_signals() {
        let kill_bit =
            1u64 << ((crate::modules::posix_consts::signal::SIGKILL as u64).saturating_sub(1));
        let stop_bit =
            1u64 << ((crate::modules::posix_consts::signal::SIGSTOP as u64).saturating_sub(1));
        let keep_bit = 1u64 << 3;
        let mask = kill_bit | stop_bit | keep_bit;

        assert_eq!(
            parse_sigmask((&mask as *const u64) as usize, core::mem::size_of::<u64>()),
            Ok(Some(keep_bit))
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
        let expected = super::compat::timeout_ns_to_retries(timeout_ns);
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
