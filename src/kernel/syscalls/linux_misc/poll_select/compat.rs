#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
use super::*;

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(super) struct LinuxPollFdCompat {
    pub(super) fd: i32,
    pub(super) events: i16,
    pub(super) revents: i16,
}

#[cfg(not(feature = "linux_compat"))]
pub(super) const LINUX_FD_SETSIZE_COMPAT: usize = 1024;

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(super) struct LinuxFdSetCompat {
    pub(super) fds_bits: [u64; LINUX_FD_SETSIZE_COMPAT / 64],
}

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(super) struct LinuxTimevalCompat {
    pub(super) tv_sec: i64,
    pub(super) tv_usec: i64,
}

#[repr(C)]
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[derive(Clone, Copy)]
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

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
pub(super) fn collect_fd_set_compat(set: &LinuxFdSetCompat, nfds: usize) -> alloc::vec::Vec<u32> {
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

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
pub(super) fn build_fd_set_compat(fds: &[u32], nfds: usize) -> LinuxFdSetCompat {
    let mut out = LinuxFdSetCompat {
        fds_bits: [0u64; LINUX_FD_SETSIZE_COMPAT / 64],
    };
    for &fd in fds {
        let idx = fd as usize;
        if idx >= nfds || idx >= LINUX_FD_SETSIZE_COMPAT {
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
    let set = with_user_read_bytes(ptr, core::mem::size_of::<LinuxFdSetCompat>(), |src| {
        let mut out = LinuxFdSetCompat {
            fds_bits: [0u64; LINUX_FD_SETSIZE_COMPAT / 64],
        };
        for i in 0..(LINUX_FD_SETSIZE_COMPAT / 64) {
            let base = i * 8;
            out.fds_bits[i] = u64::from_ne_bytes([
                src[base],
                src[base + 1],
                src[base + 2],
                src[base + 3],
                src[base + 4],
                src[base + 5],
                src[base + 6],
                src[base + 7],
            ]);
        }
        out
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;

    Ok(collect_fd_set_compat(&set, nfds))
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
pub(super) fn write_fd_set_compat(ptr: usize, fds: &[u32], nfds: usize) -> Result<(), usize> {
    let out = build_fd_set_compat(fds, nfds);
    with_user_write_bytes(ptr, core::mem::size_of::<LinuxFdSetCompat>(), |dst| {
        for i in 0..(LINUX_FD_SETSIZE_COMPAT / 64) {
            let base = i * 8;
            dst[base..base + 8].copy_from_slice(&out.fds_bits[i].to_ne_bytes());
        }
        0usize
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;
    Ok(())
}
