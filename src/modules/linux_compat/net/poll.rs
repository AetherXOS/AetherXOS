use super::super::*;

// Constants and helpers from poll_common
const MAX_POLL_FDS: usize = 1024;
const MAX_SELECT_RETRIES: usize = 1_000_000;
const MAX_EPOLL_EVENTS: usize = 4096;
const EPOLL_CTL_DEL: i32 = crate::modules::posix_consts::net::EPOLL_CTL_DEL;
const MICROS_PER_SECOND: u128 = 1_000_000;
const NANOS_PER_SECOND: u128 = 1_000_000_000;
const NANOS_PER_MICRO: u128 = 1_000;
const NANOS_PER_MILLI: u128 = 1_000_000;
const MIN_TICK_NS: u128 = 1;
const LINUX_FD_SETSIZE: usize = 1024;

mod helpers;
use helpers::{
    build_fd_set, collect_fd_set, retries_from_timespec, retries_from_timeout,
    retries_from_total_ns, run_with_temporary_sigmask, run_with_temporary_sigmask_result,
};

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxPselect6Sigmask {
    ss_ptr: u64,
    ss_len: usize,
}

#[inline]
fn parse_pselect6_sigmask(sigmask_arg: UserPtr<u8>) -> Result<Option<u64>, usize> {
    helpers::parse_pselect6_sigmask(sigmask_arg)
}

#[inline]
fn parse_optional_sigmask(sigmask: UserPtr<u64>, sigsetsize: usize) -> Result<Option<u64>, usize> {
    helpers::parse_optional_sigmask(sigmask, sigsetsize)
}

/// `epoll_create1(2)` — Create an epoll file descriptor.
pub fn sys_linux_epoll_create1(flags: usize) -> usize {
    crate::require_posix_net!((flags) => {
        match crate::modules::posix::net::epoll_create1(flags as i32) {
            Ok(fd) => {
                if (flags & crate::modules::posix_consts::net::EPOLL_CLOEXEC as usize) != 0 {
                    crate::modules::linux_compat::fs::io::linux_fd_set_descriptor_flags(
                        fd,
                        crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
                    );
                } else {
                    crate::modules::linux_compat::fs::io::linux_fd_clear_descriptor_flags(fd);
                }
                fd as usize
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `epoll_create(2)` — Legacy version.
pub fn sys_linux_epoll_create(size: usize) -> usize {
    if size == 0 {
        return linux_inval();
    }
    sys_linux_epoll_create1(0)
}

/// `epoll_ctl(2)` — Control interface for an epoll file descriptor.
pub fn sys_linux_epoll_ctl(
    epfd: Fd,
    op: usize,
    fd: Fd,
    event_ptr: UserPtr<LinuxEpollEvent>,
) -> usize {
    crate::require_posix_net!((epfd, op, fd, event_ptr) => {
        let op_i32 = op as i32;
        let events = if op_i32 != EPOLL_CTL_DEL {
            match event_ptr.read() { Ok(ev) => ev.events, Err(e) => return e }
        } else {
            0
        };

        match crate::modules::posix::net::epoll_ctl(epfd.as_u32(), op_i32, fd.as_u32(), events) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `epoll_wait(2)` — Wait for an I/O event on an epoll file descriptor.
pub fn sys_linux_epoll_wait(
    epfd: Fd,
    events_ptr: UserPtr<LinuxEpollEvent>,
    maxevents: usize,
    timeout: i32,
) -> usize {
    sys_linux_epoll_pwait(epfd, events_ptr, maxevents, timeout, UserPtr::new(0), 0)
}

/// `epoll_pwait(2)` — Wait for events with signal mask.
pub fn sys_linux_epoll_pwait(
    epfd: Fd,
    events_ptr: UserPtr<LinuxEpollEvent>,
    maxevents: usize,
    timeout: i32,
    sigmask: UserPtr<u64>,
    sigsetsize: usize,
) -> usize {
    crate::require_posix_net!((epfd, events_ptr, maxevents, timeout, sigmask, sigsetsize) => {
        if maxevents == 0 || maxevents > MAX_EPOLL_EVENTS { return linux_inval(); }

        let temp_mask = match parse_optional_sigmask(sigmask, sigsetsize) {
            Ok(mask) => mask,
            Err(e) => return e,
        };

        let retries = if timeout < 0 {
            crate::config::KernelConfig::libnet_posix_blocking_recv_retries()
        } else {
            let timeout_ms = timeout as u128;
            let total_ns = timeout_ms.saturating_mul(NANOS_PER_MILLI);
            retries_from_total_ns(total_ns)
        };

        run_with_temporary_sigmask(temp_mask, || {
            match crate::modules::posix::net::epoll_pwait(epfd.as_u32(), maxevents, retries, temp_mask) {
                Ok(events) => {
                    for (i, ev) in events.iter().enumerate() {
                        if let Err(e) = events_ptr.add(i).write(&LinuxEpollEvent { events: ev.events, data: ev.fd as u64 }) { return e; }
                    }
                    events.len()
                }
                Err(e) => linux_errno(e.code()),
            }
        })
    })
}

/// `poll(2)` — Wait for some event on a file descriptor.
pub fn sys_linux_poll(fds_ptr: UserPtr<LinuxPollFd>, nfds: usize, timeout: i32) -> usize {
    crate::require_posix_net!((fds_ptr, nfds, timeout) => {
        if nfds > MAX_POLL_FDS { return linux_errno(crate::modules::posix_consts::errno::EINVAL); }

        let mut poll_fds = alloc::vec::Vec::with_capacity(nfds);
        for i in 0..nfds {
            let ufd = match fds_ptr.add(i).read() { Ok(v) => v, Err(e) => return e };
            poll_fds.push(crate::modules::libnet::PosixPollFd {
                fd: ufd.fd as u32,
                events: crate::modules::libnet::PosixPollEvents::from_bits_truncate(ufd.events as u16),
                revents: crate::modules::libnet::PosixPollEvents::empty(),
            });
        }

        let retries = if timeout < 0 {
            crate::config::KernelConfig::libnet_posix_blocking_recv_retries()
        } else {
            let timeout_ms = timeout as u128;
            let total_ns = timeout_ms.saturating_mul(NANOS_PER_MILLI);
            retries_from_total_ns(total_ns)
        };

        match crate::modules::libnet::posix_poll_errno(&mut poll_fds, retries) {
            Ok(count) => {
                for i in 0..nfds {
                    let kfd = poll_fds[i];
                    let ufd = LinuxPollFd {
                        fd: kfd.fd as i32,
                        events: kfd.events.bits() as i16,
                        revents: kfd.revents.bits() as i16,
                    };
                    if let Err(e) = fds_ptr.add(i).write(&ufd) { return e; }
                }
                count
            }
            Err(e) => linux_errno(e.code()),
        }
    })
}

/// `ppoll(2)` — Wait for some event with signal mask and high-res timeout.
pub fn sys_linux_ppoll(
    fds_ptr: UserPtr<LinuxPollFd>,
    nfds: usize,
    timeout_ptr: UserPtr<LinuxTimespec>,
    sigmask: UserPtr<u64>,
    sigsetsize: usize,
) -> usize {
    crate::require_posix_net!((fds_ptr, nfds, timeout_ptr, sigmask, sigsetsize) => {
        if nfds > MAX_POLL_FDS { return linux_errno(crate::modules::posix_consts::errno::EINVAL); }

        // Convert user fds to kernel pollfds
        let mut poll_fds = alloc::vec::Vec::with_capacity(nfds);
        for i in 0..nfds {
            let ufd = match fds_ptr.add(i).read() { Ok(v) => v, Err(e) => return e };
            poll_fds.push(crate::modules::libnet::PosixPollFd {
                fd: ufd.fd as u32,
                events: crate::modules::libnet::PosixPollEvents::from_bits_truncate(ufd.events as u16),
                revents: crate::modules::libnet::PosixPollEvents::empty(),
            });
        }

        let retries = match retries_from_timespec(timeout_ptr) {
            Ok(v) => v,
            Err(e) => return e,
        };

        let temp_mask = match parse_optional_sigmask(sigmask, sigsetsize) {
            Ok(mask) => mask,
            Err(e) => return e,
        };

        run_with_temporary_sigmask(temp_mask, || {
            match crate::modules::libnet::posix_poll_errno(&mut poll_fds, retries) {
                Ok(count) => {
                    // Copy revents back
                    for i in 0..nfds {
                        let kfd = poll_fds[i];
                        let ufd = LinuxPollFd {
                            fd: kfd.fd as i32,
                            events: kfd.events.bits() as i16,
                            revents: kfd.revents.bits() as i16,
                        };
                        if let Err(e) = fds_ptr.add(i).write(&ufd) {
                            return e;
                        }
                    }
                    count
                }
                Err(e) => linux_errno(e.code()),
            }
        })
    })
}

/// `select(2)` — Synchronous I/O multiplexing.
pub fn sys_linux_select(
    nfds: usize,
    readfds: UserPtr<LinuxFdSet>,
    writefds: UserPtr<LinuxFdSet>,
    exceptfds: UserPtr<LinuxFdSet>,
    timeout: UserPtr<LinuxTimeval>,
) -> usize {
    crate::require_posix_net!((nfds, readfds, writefds, exceptfds, timeout) => {
        if nfds > LINUX_FD_SETSIZE { return linux_inval(); }

        let retries = match retries_from_timeout(timeout) {
            Ok(v) => v,
            Err(e) => return e,
        };

        let read_in = if readfds.is_null() {
            alloc::vec::Vec::new()
        } else {
            match readfds.read() {
                Ok(set) => collect_fd_set(&set, nfds),
                Err(e) => return e,
            }
        };
        let write_in = if writefds.is_null() {
            alloc::vec::Vec::new()
        } else {
            match writefds.read() {
                Ok(set) => collect_fd_set(&set, nfds),
                Err(e) => return e,
            }
        };
        let except_in = if exceptfds.is_null() {
            alloc::vec::Vec::new()
        } else {
            match exceptfds.read() {
                Ok(set) => collect_fd_set(&set, nfds),
                Err(e) => return e,
            }
        };

        let result = match crate::modules::libnet::posix_select_errno(
            &read_in,
            &write_in,
            &except_in,
            retries,
        ) {
            Ok(r) => r,
            Err(e) => return linux_errno(e.code()),
        };

        if !readfds.is_null() {
            let out = build_fd_set(&result.readable, nfds);
            if let Err(e) = readfds.write(&out) {
                return e;
            }
        }
        if !writefds.is_null() {
            let out = build_fd_set(&result.writable, nfds);
            if let Err(e) = writefds.write(&out) {
                return e;
            }
        }
        if !exceptfds.is_null() {
            let out = build_fd_set(&result.exceptional, nfds);
            if let Err(e) = exceptfds.write(&out) {
                return e;
            }
        }

        result.readable.len() + result.writable.len() + result.exceptional.len()
    })
}

/// `epoll_pwait2(2)` — Wait for events using `timespec` timeout.
pub fn sys_linux_epoll_pwait2(
    epfd: Fd,
    events_ptr: UserPtr<LinuxEpollEvent>,
    maxevents: usize,
    timeout_ptr: UserPtr<LinuxTimespec>,
    sigmask: UserPtr<u64>,
    sigsetsize: usize,
) -> usize {
    let timeout_ms = if timeout_ptr.is_null() {
        -1
    } else {
        let ts = match timeout_ptr.read() {
            Ok(v) => v,
            Err(e) => return e,
        };
        if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
            return linux_inval();
        }
        let ms_u128 = (ts.tv_sec as u128)
            .saturating_mul(1000)
            .saturating_add((ts.tv_nsec as u128) / 1_000_000);
        core::cmp::min(ms_u128, i32::MAX as u128) as i32
    };
    sys_linux_epoll_pwait(epfd, events_ptr, maxevents, timeout_ms, sigmask, sigsetsize)
}

/// `pselect6(2)` — select with timespec timeout and optional temporary sigmask.
pub fn sys_linux_pselect6(
    nfds: usize,
    readfds: UserPtr<LinuxFdSet>,
    writefds: UserPtr<LinuxFdSet>,
    exceptfds: UserPtr<LinuxFdSet>,
    timeout: UserPtr<LinuxTimespec>,
    sigmask_arg: UserPtr<u8>,
) -> usize {
    crate::require_posix_net!((nfds, readfds, writefds, exceptfds, timeout, sigmask_arg) => {
        if nfds > LINUX_FD_SETSIZE {
            return linux_inval();
        }

        let temp_mask = match parse_pselect6_sigmask(sigmask_arg) {
            Ok(mask) => mask,
            Err(e) => return e,
        };

        let retries = match retries_from_timespec(timeout) {
            Ok(v) => v,
            Err(e) => return e,
        };

        let read_in = if readfds.is_null() {
            alloc::vec::Vec::new()
        } else {
            match readfds.read() {
                Ok(set) => collect_fd_set(&set, nfds),
                Err(e) => return e,
            }
        };
        let write_in = if writefds.is_null() {
            alloc::vec::Vec::new()
        } else {
            match writefds.read() {
                Ok(set) => collect_fd_set(&set, nfds),
                Err(e) => return e,
            }
        };
        let except_in = if exceptfds.is_null() {
            alloc::vec::Vec::new()
        } else {
            match exceptfds.read() {
                Ok(set) => collect_fd_set(&set, nfds),
                Err(e) => return e,
            }
        };

        let result_sets = match run_with_temporary_sigmask_result(temp_mask, || {
            crate::modules::libnet::posix_select_errno(&read_in, &write_in, &except_in, retries)
                .map_err(|e| linux_errno(e.code()))
        }) {
            Ok(r) => r,
            Err(e) => return e,
        };

        if !readfds.is_null() {
            let out = build_fd_set(&result_sets.readable, nfds);
            if let Err(e) = readfds.write(&out) {
                return e;
            }
        }
        if !writefds.is_null() {
            let out = build_fd_set(&result_sets.writable, nfds);
            if let Err(e) = writefds.write(&out) {
                return e;
            }
        }
        if !exceptfds.is_null() {
            let out = build_fd_set(&result_sets.exceptional, nfds);
            if let Err(e) = exceptfds.write(&out) {
                return e;
            }
        }

        result_sets.readable.len() + result_sets.writable.len() + result_sets.exceptional.len()
    })
}

#[cfg(test)]
#[path = "poll/tests.rs"]
mod tests;
