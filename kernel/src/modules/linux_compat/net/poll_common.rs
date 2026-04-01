use super::super::*;

pub(super) const MAX_POLL_FDS: usize = 1024;
pub(super) const MAX_SELECT_RETRIES: usize = 1_000_000;
pub(super) const MAX_EPOLL_EVENTS: usize = 4096;
pub(super) const EPOLL_CTL_DEL: i32 = crate::modules::posix_consts::net::EPOLL_CTL_DEL;
pub(super) const MICROS_PER_SECOND: u128 = 1_000_000;
pub(super) const NANOS_PER_SECOND: u128 = 1_000_000_000;
pub(super) const NANOS_PER_MICRO: u128 = 1_000;
pub(super) const NANOS_PER_MILLI: u128 = 1_000_000;
pub(super) const MIN_TICK_NS: u128 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct LinuxPselect6Sigmask {
    pub(super) ss_ptr: u64,
    pub(super) ss_len: usize,
}

#[inline]
pub(super) fn kernel_tick_ns() -> u128 {
    let slice_ns = crate::generated_consts::TIME_SLICE_NS as u128;
    if slice_ns == 0 {
        MIN_TICK_NS
    } else {
        slice_ns
    }
}

#[inline]
pub(super) fn retries_from_total_ns(total_ns: u128) -> usize {
    if total_ns == 0 {
        return 0;
    }
    let tick_ns = kernel_tick_ns();
    let ticks = ((total_ns + tick_ns - 1) / tick_ns).min(MAX_SELECT_RETRIES as u128);
    ticks as usize
}

#[inline]
pub(super) fn collect_fd_set(set: &LinuxFdSet, nfds: usize) -> alloc::vec::Vec<u32> {
    let mut out = alloc::vec::Vec::new();
    for fd in 0..nfds {
        let word = fd / 64;
        let bit = fd % 64;
        if (set.fds_bits[word] & (1u64 << bit)) != 0 {
            out.push(fd as u32);
        }
    }
    out
}

#[inline]
pub(super) fn build_fd_set(fds: &[u32], nfds: usize) -> LinuxFdSet {
    let mut out = LinuxFdSet {
        fds_bits: [0u64; LINUX_FD_SETSIZE / 64],
    };
    for &fd in fds {
        let idx = fd as usize;
        if idx >= nfds || idx >= LINUX_FD_SETSIZE {
            continue;
        }
        let word = idx / 64;
        let bit = idx % 64;
        out.fds_bits[word] |= 1u64 << bit;
    }
    out
}

#[inline]
pub(super) fn retries_from_timeout(timeout: UserPtr<LinuxTimeval>) -> Result<usize, usize> {
    if timeout.is_null() {
        let retries = crate::config::KernelConfig::libnet_posix_blocking_recv_retries();
        return Ok(core::cmp::min(retries, MAX_SELECT_RETRIES));
    }
    let tv = timeout.read()?;
    if tv.tv_sec < 0 || tv.tv_usec < 0 || tv.tv_usec >= MICROS_PER_SECOND as i64 {
        return Err(linux_inval());
    }

    let total_ns = (tv.tv_sec as u128)
        .saturating_mul(NANOS_PER_SECOND)
        .saturating_add((tv.tv_usec as u128).saturating_mul(NANOS_PER_MICRO));
    Ok(retries_from_total_ns(total_ns))
}

#[inline]
pub(super) fn retries_from_timespec(timeout: UserPtr<LinuxTimespec>) -> Result<usize, usize> {
    if timeout.is_null() {
        let retries = crate::config::KernelConfig::libnet_posix_blocking_recv_retries();
        return Ok(core::cmp::min(retries, MAX_SELECT_RETRIES));
    }
    let ts = timeout.read()?;
    if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= NANOS_PER_SECOND as i64 {
        return Err(linux_inval());
    }
    let total_ns = (ts.tv_sec as u128)
        .saturating_mul(NANOS_PER_SECOND)
        .saturating_add(ts.tv_nsec as u128);
    Ok(retries_from_total_ns(total_ns))
}

#[inline]
pub(super) fn parse_optional_sigmask(
    sigmask: UserPtr<u64>,
    sigsetsize: usize,
) -> Result<Option<u64>, usize> {
    if sigmask.is_null() {
        return Ok(None);
    }
    if sigsetsize != linux::SIGSET_SIZE {
        return Err(linux_inval());
    }
    sigmask.read().map(Some)
}

#[inline]
pub(super) fn parse_pselect6_sigmask(sigmask_arg: UserPtr<u8>) -> Result<Option<u64>, usize> {
    if sigmask_arg.is_null() {
        return Ok(None);
    }
    let sig = sigmask_arg.cast::<LinuxPselect6Sigmask>().read()?;
    if sig.ss_ptr == 0 {
        return Ok(None);
    }
    if sig.ss_len != linux::SIGSET_SIZE {
        return Err(linux_inval());
    }
    UserPtr::<u64>::new(sig.ss_ptr as usize).read().map(Some)
}

#[inline]
pub(super) fn run_with_temporary_sigmask<F>(temp_mask: Option<u64>, op: F) -> usize
where
    F: FnOnce() -> usize,
{
    #[cfg(feature = "posix_signal")]
    {
        use crate::modules::posix::signal::{self, SigmaskHow};

        if let Some(mask) = temp_mask {
            let old_mask = match signal::sigprocmask(SigmaskHow::SetMask, Some(mask)) {
                Ok(old) => old,
                Err(e) => return linux_errno(e.code()),
            };

            let result = op();

            if let Err(e) = signal::sigprocmask(SigmaskHow::SetMask, Some(old_mask)) {
                return linux_errno(e.code());
            }

            result
        } else {
            op()
        }
    }
    #[cfg(not(feature = "posix_signal"))]
    {
        let _ = temp_mask;
        op()
    }
}
