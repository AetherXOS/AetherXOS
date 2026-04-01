#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
use super::*;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
use crate::kernel::syscalls::linux_shim::util::{read_user_pod, write_user_pod};

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub(super) struct LinuxPollFdCompat {
    pub(super) fd: i32,
    pub(super) events: i16,
    pub(super) revents: i16,
}

#[cfg(not(feature = "linux_compat"))]
pub(super) const LINUX_FD_SETSIZE_COMPAT: usize = 1024;

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[inline]
pub(super) fn clamp_fdset_nfds(nfds: usize) -> usize {
    core::cmp::min(nfds, LINUX_FD_SETSIZE_COMPAT)
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[inline]
pub(super) fn linux_unblockable_signal_mask() -> u64 {
    let sigkill_bit =
        1u64 << ((crate::modules::posix_consts::signal::SIGKILL as u64).saturating_sub(1));
    let sigstop_bit =
        1u64 << ((crate::modules::posix_consts::signal::SIGSTOP as u64).saturating_sub(1));
    sigkill_bit | sigstop_bit
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[inline]
pub(super) fn sanitize_wait_sigmask(mask: u64) -> u64 {
    mask & !linux_unblockable_signal_mask()
}

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(super) struct LinuxFdSetCompat {
    pub(super) fds_bits: [u64; LINUX_FD_SETSIZE_COMPAT / 64],
}

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub(super) struct LinuxTimevalCompat {
    pub(super) tv_sec: i64,
    pub(super) tv_usec: i64,
}

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub(super) struct LinuxTimespecCompat {
    pub(super) tv_sec: i64,
    pub(super) tv_nsec: i64,
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn linux_poll_fd_limit() -> usize {
    const MIN_CAP: usize = 64;
    const MAX_CAP: usize = 8192;
    crate::config::KernelConfig::network_loopback_queue_limit().clamp(MIN_CAP, MAX_CAP)
}

#[cfg(all(
    not(feature = "linux_compat"),
    any(feature = "posix_net", feature = "linux_poll_timeout_ms")
))]
#[inline]
pub(super) fn timeout_ns_to_retries(total_ns: u128) -> usize {
    if total_ns == 0 {
        return 0;
    }
    let tick_ns = core::cmp::max(crate::generated_consts::TIME_SLICE_NS as u128, 1u128);
    ((total_ns + tick_ns - 1) / tick_ns) as usize
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
pub(super) fn collect_fd_set_compat(set: &LinuxFdSetCompat, nfds: usize) -> alloc::vec::Vec<u32> {
    let capped_nfds = clamp_fdset_nfds(nfds);
    let mut out = alloc::vec::Vec::new();
    for fd in 0..capped_nfds {
        let word = fd / 64;
        let bit = fd % 64;
        if (set.fds_bits[word] & (1u64 << bit)) != 0 {
            out.push(fd as u32);
        }
    }
    out
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
pub(super) fn build_fd_set_compat(fds: &[u32], nfds: usize) -> LinuxFdSetCompat {
    let capped_nfds = clamp_fdset_nfds(nfds);
    let mut out = LinuxFdSetCompat {
        fds_bits: [0u64; LINUX_FD_SETSIZE_COMPAT / 64],
    };
    for &fd in fds {
        let idx = fd as usize;
        if idx >= capped_nfds {
            continue;
        }
        let word = idx / 64;
        let bit = idx % 64;
        out.fds_bits[word] |= 1u64 << bit;
    }
    out
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
pub(super) fn read_fd_set_compat(ptr: usize, nfds: usize) -> Result<alloc::vec::Vec<u32>, usize> {
    let set = read_user_pod::<LinuxFdSetCompat>(ptr)?;

    Ok(collect_fd_set_compat(&set, nfds))
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
pub(super) fn write_fd_set_compat(ptr: usize, fds: &[u32], nfds: usize) -> Result<(), usize> {
    let out = build_fd_set_compat(fds, nfds);
    write_user_pod(ptr, &out)?;
    Ok(())
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
impl Default for LinuxFdSetCompat {
    fn default() -> Self {
        Self {
            fds_bits: [0u64; LINUX_FD_SETSIZE_COMPAT / 64],
        }
    }
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_net"))]
mod tests {
    use super::*;

    #[test_case]
    fn collect_fd_set_caps_nfds_to_fdset_size() {
        let set = LinuxFdSetCompat {
            fds_bits: [u64::MAX; LINUX_FD_SETSIZE_COMPAT / 64],
        };

        let collected = collect_fd_set_compat(&set, LINUX_FD_SETSIZE_COMPAT + 256);

        assert_eq!(collected.len(), LINUX_FD_SETSIZE_COMPAT);
        assert_eq!(collected.first().copied(), Some(0));
        assert_eq!(
            collected.last().copied(),
            Some((LINUX_FD_SETSIZE_COMPAT - 1) as u32)
        );
    }

    #[test_case]
    fn build_fd_set_ignores_entries_beyond_capped_nfds() {
        let out = build_fd_set_compat(
            &[0, 63, 64, (LINUX_FD_SETSIZE_COMPAT - 1) as u32],
            64,
        );

        assert_eq!(out.fds_bits[0], (1u64 << 0) | (1u64 << 63));
        assert_eq!(out.fds_bits[1], 0);
    }

    #[test_case]
    fn sanitize_wait_sigmask_clears_unblockable_bits() {
        let kill_bit =
            1u64 << ((crate::modules::posix_consts::signal::SIGKILL as u64).saturating_sub(1));
        let stop_bit =
            1u64 << ((crate::modules::posix_consts::signal::SIGSTOP as u64).saturating_sub(1));
        let keep_bit = 1u64 << 2;

        let sanitized = sanitize_wait_sigmask(kill_bit | stop_bit | keep_bit);
        assert_eq!(sanitized & kill_bit, 0);
        assert_eq!(sanitized & stop_bit, 0);
        assert_ne!(sanitized & keep_bit, 0);
    }
}
