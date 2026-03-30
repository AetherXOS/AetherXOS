use super::super::*;
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
use crate::kernel::syscalls::linux_shim::util::{
    define_user_pod_codec, read_user_pod, write_user_pod,
};
#[cfg(all(
    not(feature = "linux_compat"),
    feature = "posix_net",
    feature = "linux_userspace_graphics"
))]
use alloc::collections::{BTreeMap, BTreeSet};
#[cfg(all(
    not(feature = "linux_compat"),
    feature = "posix_net",
    feature = "linux_userspace_graphics"
))]
use lazy_static::lazy_static;
#[cfg(all(
    not(feature = "linux_compat"),
    feature = "posix_net",
    feature = "linux_userspace_graphics"
))]
use spin::Mutex;

#[repr(C, packed)]
#[cfg(not(feature = "linux_compat"))]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
struct LinuxEpollEventCompat {
    events: u32,
    data: u64,
}

#[repr(C)]
#[cfg(not(feature = "linux_compat"))]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
struct LinuxTimespecCompat {
    tv_sec: i64,
    tv_nsec: i64,
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
define_user_pod_codec!(read_linux_epoll_event_pod, _write_linux_epoll_event_pod, LinuxEpollEventCompat);
#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
define_user_pod_codec!(read_linux_timespec_pod, _write_linux_timespec_pod, LinuxTimespecCompat);

#[cfg(all(
    not(feature = "linux_compat"),
    feature = "posix_net",
    feature = "linux_userspace_graphics"
))]
lazy_static! {
    static ref USERSPACE_DISPLAY_EPOLL_REGISTRY: Mutex<BTreeMap<u32, BTreeMap<u32, u32>>> =
        Mutex::new(BTreeMap::new());
}

#[cfg(all(
    not(feature = "linux_compat"),
    feature = "posix_net",
    feature = "linux_userspace_graphics"
))]
fn record_userspace_display_epoll_interest(epfd: u32, op_i32: i32, fd: u32, events: u32) {
    let ctl_add = crate::modules::posix_consts::net::EPOLL_CTL_ADD;
    let ctl_mod = crate::modules::posix_consts::net::EPOLL_CTL_MOD;
    let ctl_del = crate::modules::posix_consts::net::EPOLL_CTL_DEL;

    let mut guard = USERSPACE_DISPLAY_EPOLL_REGISTRY.lock();
    let entry = guard.entry(epfd).or_default();

    if op_i32 == ctl_del {
        entry.remove(&fd);
        return;
    }

    if op_i32 == ctl_add || op_i32 == ctl_mod {
        if crate::kernel::syscalls::linux_shim::net::userspace_display_fd_is_bound(fd) {
            entry.insert(fd, events);
        } else {
            entry.remove(&fd);
        }
    }
}

#[cfg(all(
    not(feature = "linux_compat"),
    feature = "posix_net",
    feature = "linux_userspace_graphics"
))]
fn synthetic_userspace_display_epoll_rows(
    epfd: u32,
    maxevents: usize,
    existing_rows: &[(u32, u64)],
) -> alloc::vec::Vec<(u32, u64)> {
    if maxevents == 0 {
        return alloc::vec::Vec::new();
    }

    let mut seen = BTreeSet::new();
    for (_, fd) in existing_rows {
        seen.insert(*fd as u32);
    }

    let Some(registry) = USERSPACE_DISPLAY_EPOLL_REGISTRY.lock().get(&epfd).cloned() else {
        return alloc::vec::Vec::new();
    };

    let mut rows = alloc::vec::Vec::new();
    for (fd, requested) in registry.iter() {
        if seen.contains(fd) {
            continue;
        }

        let revents =
            crate::kernel::syscalls::linux_shim::net::userspace_display_epoll_revents(*fd, *requested);
        if revents == 0 {
            continue;
        }

        rows.push((revents, *fd as u64));
        if rows.len() >= maxevents {
            break;
        }
    }

    rows
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[inline]
fn timeout_ns_to_retries(total_ns: u128) -> usize {
    if total_ns == 0 {
        return 0;
    }

    let tick_ns = core::cmp::max(crate::generated_consts::TIME_SLICE_NS as u128, 1u128);
    ((total_ns + tick_ns - 1) / tick_ns) as usize
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[inline]
fn timeout_arg_to_retries(timeout: usize) -> usize {
    if timeout == usize::MAX {
        crate::config::KernelConfig::libnet_posix_blocking_recv_retries()
    } else {
        timeout
    }
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[inline]
fn validate_maxevents(maxevents: usize) -> Result<usize, usize> {
    if maxevents == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let limit = crate::config::KernelConfig::network_epoll_max_events();
    if maxevents > limit {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    Ok(maxevents)
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn timeout_ptr_to_retries(timeout_ptr: usize) -> Result<usize, usize> {
    if timeout_ptr == 0 {
        return Ok(crate::config::KernelConfig::libnet_posix_blocking_recv_retries());
    }

    let ts = read_linux_timespec_pod(timeout_ptr)?;

    if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let total_ns = (ts.tv_sec as u128)
        .saturating_mul(1_000_000_000u128)
        .saturating_add(ts.tv_nsec as u128);
    Ok(timeout_ns_to_retries(total_ns))
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn parse_sigmask(sigmask_ptr: usize, sigset_size: usize) -> Result<Option<u64>, usize> {
    if sigmask_ptr == 0 {
        return Ok(None);
    }

    if sigset_size != core::mem::size_of::<u64>() {
        return Err(linux_errno(crate::modules::posix_consts::errno::EINVAL));
    }

    let mask = read_user_pod::<u64>(sigmask_ptr)?;

    Ok(Some(sanitize_wait_sigmask(mask)))
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
#[inline]
fn sanitize_wait_sigmask(mask: u64) -> u64 {
    let sigkill_bit =
        1u64 << ((crate::modules::posix_consts::signal::SIGKILL as u64).saturating_sub(1));
    let sigstop_bit =
        1u64 << ((crate::modules::posix_consts::signal::SIGSTOP as u64).saturating_sub(1));
    mask & !(sigkill_bit | sigstop_bit)
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn collect_epoll_rows(
    epfd: u32,
    maxevents: usize,
    events: &[crate::modules::posix::net::EpollEvent],
) -> alloc::vec::Vec<(u32, u64)> {
    let mut rows = alloc::vec::Vec::with_capacity(events.len());
    for ev in events.iter() {
        rows.push((ev.events, ev.fd as u64));
    }

    #[cfg(feature = "linux_userspace_graphics")]
    {
        let remaining = maxevents.saturating_sub(rows.len());
        if remaining > 0 {
            rows.extend(synthetic_userspace_display_epoll_rows(
                epfd,
                remaining,
                &rows,
            ));
        }
    }

    rows
}

#[cfg(all(not(feature = "linux_compat"), feature = "posix_net"))]
fn write_epoll_rows_to_user(events_ptr: usize, rows: &[(u32, u64)]) -> usize {
    let item_sz = core::mem::size_of::<LinuxEpollEventCompat>();
    let total_sz = match item_sz.checked_mul(rows.len()) {
        Some(v) => v,
        None => return linux_errno(crate::modules::posix_consts::errno::EOVERFLOW),
    };

    if total_sz == 0 {
        return 0;
    }

    for (i, (events, fd)) in rows.iter().enumerate() {
        let dst_ptr = match events_ptr.checked_add(i.saturating_mul(item_sz)) {
            Some(v) => v,
            None => return linux_errno(crate::modules::posix_consts::errno::EOVERFLOW),
        };
        let entry = LinuxEpollEventCompat {
            events: *events,
            data: *fd,
        };
        if write_user_pod(dst_ptr, &entry).is_err() {
            return linux_errno(crate::modules::posix_consts::errno::EFAULT);
        }
    }

    rows.len()
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_epoll_create(size: usize) -> usize {
    if size == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    sys_linux_epoll_create1(0)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_epoll_create1(flags: usize) -> usize {
    #[cfg(feature = "posix_net")]
    {
        match crate::modules::posix::net::epoll_create1(flags as i32) {
            Ok(fd) => fd as usize,
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = flags;
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_epoll_ctl(epfd: usize, op: usize, fd: usize, event_ptr: usize) -> usize {
    #[cfg(feature = "posix_net")]
    {
        let op_i32 = op as i32;
        let events = if op_i32 == crate::modules::posix_consts::net::EPOLL_CTL_DEL {
            0u32
        } else {
            if event_ptr == 0 {
                return linux_errno(crate::modules::posix_consts::errno::EFAULT);
            }
            match read_linux_epoll_event_pod(event_ptr) {
                Ok(ev) => ev.events,
                Err(_) => return linux_errno(crate::modules::posix_consts::errno::EFAULT),
            }
        };

        let result = match crate::modules::posix::net::epoll_ctl(epfd as u32, op_i32, fd as u32, events)
        {
            Ok(()) => 0,
            Err(err) => linux_errno(err.code()),
        };

        #[cfg(feature = "linux_userspace_graphics")]
        if result == 0 {
            record_userspace_display_epoll_interest(epfd as u32, op_i32, fd as u32, events);
        }

        result
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (epfd, op, fd, event_ptr);
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_epoll_pwait(
    epfd: usize,
    events_ptr: usize,
    maxevents: usize,
    timeout: usize,
    _sigmask_ptr: usize,
    _sigset_size: usize,
) -> usize {
    #[cfg(feature = "posix_net")]
    {
        let maxevents = match validate_maxevents(maxevents) {
            Ok(v) => v,
            Err(err) => return err,
        };

        let sigmask = match parse_sigmask(_sigmask_ptr, _sigset_size) {
            Ok(v) => v,
            Err(err) => return err,
        };

        let retries = timeout_arg_to_retries(timeout);

        let events =
            match crate::modules::posix::net::epoll_pwait(epfd as u32, maxevents, retries, sigmask)
            {
                Ok(v) => v,
                Err(err) => return linux_errno(err.code()),
            };

        let rows = collect_epoll_rows(epfd as u32, maxevents, &events);
        write_epoll_rows_to_user(events_ptr, &rows)
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (
            epfd,
            events_ptr,
            maxevents,
            timeout,
            _sigmask_ptr,
            _sigset_size,
        );
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_epoll_pwait2(
    epfd: usize,
    events_ptr: usize,
    maxevents: usize,
    timeout_ptr: usize,
    _sigmask_ptr: usize,
    _sigset_size: usize,
) -> usize {
    #[cfg(feature = "posix_net")]
    {
        let maxevents = match validate_maxevents(maxevents) {
            Ok(v) => v,
            Err(err) => return err,
        };

        let sigmask = match parse_sigmask(_sigmask_ptr, _sigset_size) {
            Ok(v) => v,
            Err(err) => return err,
        };

        let retries = match timeout_ptr_to_retries(timeout_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };

        let events =
            match crate::modules::posix::net::epoll_pwait(epfd as u32, maxevents, retries, sigmask)
            {
                Ok(v) => v,
                Err(err) => return linux_errno(err.code()),
            };

        let rows = collect_epoll_rows(epfd as u32, maxevents, &events);
        write_epoll_rows_to_user(events_ptr, &rows)
    }
    #[cfg(not(feature = "posix_net"))]
    {
        let _ = (
            epfd,
            events_ptr,
            maxevents,
            timeout_ptr,
            _sigmask_ptr,
            _sigset_size,
        );
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    }
}

#[cfg(all(test, not(feature = "linux_compat"), feature = "posix_net"))]
#[path = "epoll/tests.rs"]
mod tests;
