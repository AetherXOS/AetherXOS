#[cfg(not(feature = "linux_compat"))]
use super::{linux_errno, sys_yield, with_user_read_bytes, with_user_write_bytes};
#[cfg(not(feature = "linux_compat"))]
use alloc::collections::{BTreeMap, BTreeSet};
#[cfg(not(feature = "linux_compat"))]
use lazy_static::lazy_static;

mod poll_select;
mod proc_ctl;
mod runtime_info;
#[cfg(all(test, not(feature = "linux_compat")))]
mod runtime_stress_tests;

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxTimespecCompat {
    tv_sec: i64,
    tv_nsec: i64,
}

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxItimerspecCompat {
    it_interval: LinuxTimespecCompat,
    it_value: LinuxTimespecCompat,
}

#[cfg(not(feature = "linux_compat"))]
#[derive(Clone, Copy, Default)]
struct TimerfdRuntimeState {
    spec: LinuxItimerspecCompat,
    armed_at_ns: u128,
}

#[cfg(not(feature = "linux_compat"))]
#[derive(Clone)]
struct FanotifyMarkState {
    mask: usize,
    dirfd: isize,
    path: alloc::string::String,
}

#[cfg(not(feature = "linux_compat"))]
#[derive(Clone)]
struct InotifyWatchState {
    wd: i32,
    path: alloc::string::String,
    mask: u32,
}

#[cfg(not(feature = "linux_compat"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxFutexWaitVCompat {
    val: u64,
    uaddr: u64,
    flags: u32,
    __reserved: u32,
}

#[cfg(not(feature = "linux_compat"))]
lazy_static! {
    static ref TIMERFD_STATE_BY_FD: crate::kernel::sync::IrqSafeMutex<BTreeMap<u32, TimerfdRuntimeState>> =
        crate::kernel::sync::IrqSafeMutex::new(BTreeMap::new());
    static ref IO_URING_IDS: crate::kernel::sync::IrqSafeMutex<BTreeSet<u32>> =
        crate::kernel::sync::IrqSafeMutex::new(BTreeSet::new());
    static ref LANDLOCK_RULESET_IDS: crate::kernel::sync::IrqSafeMutex<BTreeSet<u32>> =
        crate::kernel::sync::IrqSafeMutex::new(BTreeSet::new());
    static ref BPF_MAP_IDS: crate::kernel::sync::IrqSafeMutex<BTreeSet<u32>> =
        crate::kernel::sync::IrqSafeMutex::new(BTreeSet::new());
    static ref FANOTIFY_MARKS_BY_FD: crate::kernel::sync::IrqSafeMutex<BTreeMap<u32, alloc::vec::Vec<FanotifyMarkState>>> =
        crate::kernel::sync::IrqSafeMutex::new(BTreeMap::new());
    static ref EVENTFD_STATE_BY_FD: crate::kernel::sync::IrqSafeMutex<BTreeMap<u32, u64>> =
        crate::kernel::sync::IrqSafeMutex::new(BTreeMap::new());
    static ref MEMFD_NAME_BY_FD: crate::kernel::sync::IrqSafeMutex<BTreeMap<u32, alloc::string::String>> =
        crate::kernel::sync::IrqSafeMutex::new(BTreeMap::new());
    static ref INOTIFY_WATCHES_BY_FD: crate::kernel::sync::IrqSafeMutex<BTreeMap<u32, alloc::vec::Vec<InotifyWatchState>>> =
        crate::kernel::sync::IrqSafeMutex::new(BTreeMap::new());
    static ref SIGNALFD_MASK_BY_FD: crate::kernel::sync::IrqSafeMutex<BTreeMap<u32, u64>> =
        crate::kernel::sync::IrqSafeMutex::new(BTreeMap::new());
}

#[cfg(not(feature = "linux_compat"))]
static NEXT_IO_URING_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
#[cfg(not(feature = "linux_compat"))]
static NEXT_LANDLOCK_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
#[cfg(not(feature = "linux_compat"))]
static NEXT_BPF_MAP_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
#[cfg(not(feature = "linux_compat"))]
static NEXT_FANOTIFY_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
#[cfg(not(feature = "linux_compat"))]
static NEXT_EVENTFD_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
#[cfg(not(feature = "linux_compat"))]
static NEXT_TIMERFD_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
#[cfg(not(feature = "linux_compat"))]
static NEXT_MEMFD_SYNTH_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
#[cfg(not(feature = "linux_compat"))]
static NEXT_INOTIFY_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
#[cfg(not(feature = "linux_compat"))]
static NEXT_INOTIFY_WD: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(1);
#[cfg(not(feature = "linux_compat"))]
static NEXT_SIGNALFD_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);

#[cfg(not(feature = "linux_compat"))]
const IO_URING_FD_BASE: usize = 700_000;
#[cfg(not(feature = "linux_compat"))]
const LANDLOCK_FD_BASE: usize = 710_000;
#[cfg(not(feature = "linux_compat"))]
const BPF_FD_BASE: usize = 720_000;
#[cfg(not(feature = "linux_compat"))]
const FANOTIFY_FD_BASE: usize = 730_000;
#[cfg(not(feature = "linux_compat"))]
const EVENTFD_FD_BASE: usize = 740_000;
#[cfg(not(feature = "linux_compat"))]
const TIMERFD_FD_BASE: usize = 750_000;
#[cfg(not(feature = "linux_compat"))]
const MEMFD_FD_BASE: usize = 760_000;
#[cfg(not(feature = "linux_compat"))]
const INOTIFY_FD_BASE: usize = 770_000;
#[cfg(not(feature = "linux_compat"))]
const SIGNALFD_FD_BASE: usize = 780_000;

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn validate_timespec_compat(ts: LinuxTimespecCompat) -> bool {
    ts.tv_sec >= 0 && ts.tv_nsec >= 0 && ts.tv_nsec < 1_000_000_000
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn timespec_to_ns(ts: LinuxTimespecCompat) -> Option<u128> {
    if !validate_timespec_compat(ts) {
        return None;
    }
    Some((ts.tv_sec as u128).saturating_mul(1_000_000_000u128).saturating_add(ts.tv_nsec as u128))
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn ns_to_timespec(ns: u128) -> LinuxTimespecCompat {
    LinuxTimespecCompat {
        tv_sec: (ns / 1_000_000_000u128) as i64,
        tv_nsec: (ns % 1_000_000_000u128) as i64,
    }
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn monotonic_now_ns() -> u128 {
    #[cfg(feature = "posix_time")]
    {
        if let Ok(ts) = crate::modules::posix::time::clock_gettime_raw(
            crate::modules::posix_consts::time::CLOCK_MONOTONIC,
        ) {
            return (ts.sec as u128)
                .saturating_mul(1_000_000_000u128)
                .saturating_add(ts.nsec as u128);
        }
    }
    let tick_ns = core::cmp::max(crate::generated_consts::TIME_SLICE_NS as u128, 1u128);
    (crate::hal::cpu::rdtsc() as u128).saturating_mul(tick_ns)
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
#[inline]
fn timerfd_is_expired(state: &TimerfdRuntimeState, now_ns: u128) -> bool {
    let Some(initial_ns) = timespec_to_ns(state.spec.it_value) else {
        return false;
    };
    if initial_ns == 0 {
        return false;
    }
    now_ns.saturating_sub(state.armed_at_ns) >= initial_ns
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn timerfd_current_spec(state: &TimerfdRuntimeState, now_ns: u128) -> LinuxItimerspecCompat {
    let mut out = state.spec;
    let Some(initial_ns) = timespec_to_ns(state.spec.it_value) else {
        out.it_value = LinuxTimespecCompat::default();
        return out;
    };
    if initial_ns == 0 {
        out.it_value = LinuxTimespecCompat::default();
        return out;
    }

    let elapsed_ns = now_ns.saturating_sub(state.armed_at_ns);
    let interval_ns = timespec_to_ns(state.spec.it_interval).unwrap_or(0);

    let remaining_ns = if elapsed_ns < initial_ns {
        initial_ns.saturating_sub(elapsed_ns)
    } else if interval_ns == 0 {
        0
    } else {
        let passed_after_first = elapsed_ns.saturating_sub(initial_ns);
        let rem = passed_after_first % interval_ns;
        if rem == 0 { interval_ns } else { interval_ns.saturating_sub(rem) }
    };

    out.it_value = ns_to_timespec(remaining_ns);
    out
}

#[cfg(not(feature = "linux_compat"))]
pub(crate) fn timerfd_poll_revents(fd: u32, requested_events: u16) -> u16 {
    let wants_read = (requested_events & crate::modules::posix_consts::net::POLLIN) != 0;
    if !wants_read {
        return 0;
    }
    let now_ns = monotonic_now_ns();
    let guard = TIMERFD_STATE_BY_FD.lock();
    let Some(state) = guard.get(&fd) else {
        return 0;
    };
    if timerfd_is_expired(state, now_ns) {
        crate::modules::posix_consts::net::POLLIN
    } else {
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn read_itimerspec_from_user(ptr: usize) -> Result<LinuxItimerspecCompat, usize> {
    read_user_struct::<LinuxItimerspecCompat>(ptr)
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn write_itimerspec_to_user(ptr: usize, spec: LinuxItimerspecCompat) -> usize {
    write_user_struct::<LinuxItimerspecCompat>(ptr, &spec)
        .map(|_| 0usize)
        .unwrap_or_else(|err| err)
}

#[cfg(not(feature = "linux_compat"))]
fn read_user_struct<T: Copy + Default>(ptr: usize) -> Result<T, usize> {
    let size = core::mem::size_of::<T>();
    with_user_read_bytes(ptr, size, |src| {
        let mut out = T::default();
        let dst = unsafe { core::slice::from_raw_parts_mut((&mut out as *mut T).cast::<u8>(), size) };
        dst.copy_from_slice(src);
        out
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
fn write_user_struct<T: Copy>(ptr: usize, value: &T) -> Result<(), usize> {
    let size = core::mem::size_of::<T>();
    with_user_write_bytes(ptr, size, |dst| {
        let src = unsafe { core::slice::from_raw_parts((value as *const T).cast::<u8>(), size) };
        dst.copy_from_slice(src);
        0usize
    })
    .map(|_| ())
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn read_user_c_string_compat(ptr: usize, max_len: usize) -> Result<alloc::string::String, usize> {
    if ptr == 0 || max_len == 0 {
        return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
    }
    let mut out = alloc::vec::Vec::new();
    for i in 0..max_len {
        let Some(addr) = ptr.checked_add(i) else {
            return Err(linux_errno(crate::modules::posix_consts::errno::EFAULT));
        };
        let b = with_user_read_bytes(addr, 1, |src| src[0])
            .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))?;
        if b == 0 {
            return alloc::string::String::from_utf8(out)
                .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EINVAL));
        }
        out.push(b);
    }
    Err(linux_errno(crate::modules::posix_consts::errno::EINVAL))
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
fn read_u64_from_user(ptr: usize) -> Result<u64, usize> {
    with_user_read_bytes(ptr, core::mem::size_of::<u64>(), |src| {
        let mut bytes = [0u8; core::mem::size_of::<u64>()];
        bytes.copy_from_slice(src);
        u64::from_ne_bytes(bytes)
    })
    .map_err(|_| linux_errno(crate::modules::posix_consts::errno::EFAULT))
}

#[cfg(not(feature = "linux_compat"))]
#[inline]
pub(super) fn sys_linux_poll(fds_ptr: usize, nfds: usize, timeout: usize) -> usize {
    poll_select::sys_linux_poll(fds_ptr, nfds, timeout)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_ppoll(
    fds_ptr: usize,
    nfds: usize,
    timeout_ptr: usize,
    sigmask_ptr: usize,
    sigset_size: usize,
) -> usize {
    poll_select::sys_linux_ppoll(fds_ptr, nfds, timeout_ptr, sigmask_ptr, sigset_size)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_select(
    nfds: usize,
    readfds: usize,
    writefds: usize,
    exceptfds: usize,
    timeout: usize,
) -> usize {
    poll_select::sys_linux_select(nfds, readfds, writefds, exceptfds, timeout)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_pselect6(
    nfds: usize,
    readfds: usize,
    writefds: usize,
    exceptfds: usize,
    timeout_ptr: usize,
    sigmask_desc_ptr: usize,
) -> usize {
    poll_select::sys_linux_pselect6(
        nfds,
        readfds,
        writefds,
        exceptfds,
        timeout_ptr,
        sigmask_desc_ptr,
    )
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_gettimeofday(tv_ptr: usize) -> usize {
    runtime_info::sys_linux_gettimeofday(tv_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_time(tloc: usize) -> usize {
    runtime_info::sys_linux_time(tloc)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_getcpu(cpu_ptr: usize, node_ptr: usize) -> usize {
    runtime_info::sys_linux_getcpu(cpu_ptr, node_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_prctl(
    option: usize,
    arg2: usize,
    _arg3: usize,
    _arg4: usize,
    _arg5: usize,
) -> usize {
    proc_ctl::sys_linux_prctl(option, arg2, _arg3, _arg4, _arg5)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_getparam(_pid: usize, param_ptr: usize) -> usize {
    proc_ctl::sys_linux_sched_getparam(_pid, param_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_getscheduler(_pid: usize) -> usize {
    proc_ctl::sys_linux_sched_getscheduler(_pid)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_setparam(pid: usize, param_ptr: usize) -> usize {
    proc_ctl::sys_linux_sched_setparam(pid, param_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_setscheduler(pid: usize, policy: usize, param_ptr: usize) -> usize {
    proc_ctl::sys_linux_sched_setscheduler(pid, policy, param_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_getaffinity(
    _pid: usize,
    cpusetsize: usize,
    mask_ptr: usize,
) -> usize {
    proc_ctl::sys_linux_sched_getaffinity(_pid, cpusetsize, mask_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sched_setaffinity(
    _pid: usize,
    _cpusetsize: usize,
    _mask_ptr: usize,
) -> usize {
    proc_ctl::sys_linux_sched_setaffinity(_pid, _cpusetsize, _mask_ptr)
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(crate) fn linux_prctl_seccomp_mode_for_tid(tid: usize) -> u8 {
    proc_ctl::seccomp_mode_for_tid(tid)
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(crate) fn linux_prctl_no_new_privs_for_tid(tid: usize) -> bool {
    proc_ctl::no_new_privs_for_tid(tid)
}

#[cfg(all(test, not(feature = "linux_compat")))]
#[allow(dead_code)]
pub(crate) fn linux_set_prctl_state_for_tid_for_test(
    tid: usize,
    seccomp_mode: u8,
    no_new_privs: bool,
) {
    proc_ctl::set_prctl_state_for_tid_for_test(tid, seccomp_mode, no_new_privs)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_sysinfo(info_ptr: usize) -> usize {
    runtime_info::sys_linux_sysinfo(info_ptr)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_getrandom(buf_ptr: usize, buflen: usize, _flags: usize) -> usize {
    runtime_info::sys_linux_getrandom(buf_ptr, buflen, _flags)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_eventfd(initval: usize, flags: usize) -> usize {
    #[cfg(feature = "posix_io")]
    {
        match crate::modules::posix::io::eventfd_create_errno(initval as u32, flags as i32) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_io"))]
    {
        let id = NEXT_EVENTFD_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let fd = (EVENTFD_FD_BASE as u32).saturating_add(id);
        EVENTFD_STATE_BY_FD.lock().insert(fd, initval as u64);
        let _ = flags;
        fd as usize
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_eventfd2(initval: usize, flags: usize) -> usize {
    sys_linux_eventfd(initval, flags)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_timerfd_create(clockid: usize, flags: usize) -> usize {
    let allowed_flags = 0x1usize | 0x0008_0000usize | 0x0000_0800usize;
    if (flags & !allowed_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if clockid > crate::modules::posix_consts::time::CLOCK_MONOTONIC as usize {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::openat(fs_id, "/", "timerfd", true) {
            Ok(fd) => {
                if (flags & 0x0000_0800usize) != 0 {
                    let _ = crate::modules::posix::fs::fcntl_set_status_flags(
                        fd,
                        crate::modules::posix_consts::net::O_NONBLOCK,
                    );
                }
                TIMERFD_STATE_BY_FD
                    .lock()
                    .insert(
                        fd,
                        TimerfdRuntimeState {
                            spec: LinuxItimerspecCompat::default(),
                            armed_at_ns: monotonic_now_ns(),
                        },
                    );
                fd as usize
            }
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let id = NEXT_TIMERFD_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let fd = (TIMERFD_FD_BASE as u32).saturating_add(id);
        TIMERFD_STATE_BY_FD
            .lock()
            .insert(
                fd,
                TimerfdRuntimeState {
                    spec: LinuxItimerspecCompat::default(),
                    armed_at_ns: monotonic_now_ns(),
                },
            );
        fd as usize
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_timerfd_settime(
    fd: usize,
    flags: usize,
    new_value_ptr: usize,
    old_value_ptr: usize,
) -> usize {
    if (flags & !0x1usize) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if new_value_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }

    let new_spec = match read_itimerspec_from_user(new_value_ptr) {
        Ok(v) => v,
        Err(err) => return err,
    };
    if !validate_timespec_compat(new_spec.it_interval) || !validate_timespec_compat(new_spec.it_value)
    {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let mut state = TIMERFD_STATE_BY_FD.lock();
    let Some(slot) = state.get_mut(&(fd as u32)) else {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    };

    if old_value_ptr != 0 {
        let rc = write_itimerspec_to_user(old_value_ptr, timerfd_current_spec(slot, monotonic_now_ns()));
        if rc != 0 {
            return rc;
        }
    }

    slot.spec = new_spec;
    slot.armed_at_ns = monotonic_now_ns();
    0
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_timerfd_gettime(fd: usize, curr_value_ptr: usize) -> usize {
    if curr_value_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    let state = TIMERFD_STATE_BY_FD.lock();
    let Some(spec) = state.get(&(fd as u32)).copied() else {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    };
    write_itimerspec_to_user(curr_value_ptr, timerfd_current_spec(&spec, monotonic_now_ns()))
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_bpf(cmd: usize, attr_ptr: usize, size: usize) -> usize {
    const BPF_CMD_MAP_CREATE: usize = 0;
    const BPF_CMD_MAP_LOOKUP_ELEM: usize = 1;
    const BPF_CMD_MAP_UPDATE_ELEM: usize = 2;
    const BPF_CMD_MAP_DELETE_ELEM: usize = 3;
    const BPF_CMD_MAP_GET_NEXT_KEY: usize = 4;
    if attr_ptr == 0 || size == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    if matches!(
        cmd,
        BPF_CMD_MAP_LOOKUP_ELEM
            | BPF_CMD_MAP_UPDATE_ELEM
            | BPF_CMD_MAP_DELETE_ELEM
            | BPF_CMD_MAP_GET_NEXT_KEY
    ) {
        return 0;
    }
    if cmd != BPF_CMD_MAP_CREATE {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    let id = NEXT_BPF_MAP_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    BPF_MAP_IDS.lock().insert(id);
    BPF_FD_BASE.saturating_add(id as usize)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_io_uring_setup(entries: usize, params_ptr: usize) -> usize {
    if entries == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if params_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    let id = NEXT_IO_URING_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    IO_URING_IDS.lock().insert(id);
    IO_URING_FD_BASE.saturating_add(id as usize)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_io_uring_enter(
    fd: usize,
    to_submit: usize,
    min_complete: usize,
    flags: usize,
    sig_ptr: usize,
    sigsz: usize,
) -> usize {
    let _ = (to_submit, min_complete, flags, sig_ptr, sigsz);
    if fd < IO_URING_FD_BASE {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    let id = (fd - IO_URING_FD_BASE) as u32;
    if !IO_URING_IDS.lock().contains(&id) {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_io_uring_register(
    fd: usize,
    opcode: usize,
    arg_ptr: usize,
    nr_args: usize,
) -> usize {
    let _ = (opcode, arg_ptr, nr_args);
    if fd < IO_URING_FD_BASE {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    let id = (fd - IO_URING_FD_BASE) as u32;
    if !IO_URING_IDS.lock().contains(&id) {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_landlock_create_ruleset(
    attr_ptr: usize,
    size: usize,
    flags: usize,
) -> usize {
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if attr_ptr == 0 || size == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    let id = NEXT_LANDLOCK_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    LANDLOCK_RULESET_IDS.lock().insert(id);
    LANDLOCK_FD_BASE.saturating_add(id as usize)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_landlock_add_rule(
    ruleset_fd: usize,
    rule_type: usize,
    rule_attr: usize,
    flags: usize,
) -> usize {
    let _ = (rule_type, rule_attr);
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if ruleset_fd < LANDLOCK_FD_BASE {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    let id = (ruleset_fd - LANDLOCK_FD_BASE) as u32;
    if !LANDLOCK_RULESET_IDS.lock().contains(&id) {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_landlock_restrict_self(ruleset_fd: usize, flags: usize) -> usize {
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if ruleset_fd < LANDLOCK_FD_BASE {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    let id = (ruleset_fd - LANDLOCK_FD_BASE) as u32;
    if !LANDLOCK_RULESET_IDS.lock().contains(&id) {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    }
    0
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fanotify_init(flags: usize, event_f_flags: usize) -> usize {
    const FAN_CLASS_NOTIF: usize = 0x0000;
    const FAN_CLASS_CONTENT: usize = 0x0004;
    const FAN_CLASS_PRE_CONTENT: usize = 0x0008;
    const FAN_CLOEXEC: usize = 0x0000_0001;
    const FAN_NONBLOCK: usize = 0x0000_0002;
    const FAN_UNLIMITED_QUEUE: usize = 0x0000_0010;
    const FAN_UNLIMITED_MARKS: usize = 0x0000_0020;
    const FAN_REPORT_FID: usize = 0x0000_0200;

    let class_bits = flags & (FAN_CLASS_CONTENT | FAN_CLASS_PRE_CONTENT);
    if class_bits == (FAN_CLASS_CONTENT | FAN_CLASS_PRE_CONTENT) {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let allowed_init_flags = FAN_CLASS_NOTIF
        | FAN_CLASS_CONTENT
        | FAN_CLASS_PRE_CONTENT
        | FAN_CLOEXEC
        | FAN_NONBLOCK
        | FAN_UNLIMITED_QUEUE
        | FAN_UNLIMITED_MARKS
        | FAN_REPORT_FID;
    if (flags & !allowed_init_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let allowed_event_f_flags =
        crate::modules::posix_consts::fs::O_RDONLY as usize
            | crate::kernel::syscalls::syscalls_consts::linux::open_flags::O_CLOEXEC
            | crate::modules::posix_consts::net::O_NONBLOCK as usize;
    if (event_f_flags & !allowed_event_f_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_fs")]
    {
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(err) => return linux_errno(err.code()),
        };
        let id = NEXT_FANOTIFY_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let path = alloc::format!("/.fanotify-{}", id);
        match crate::modules::posix::fs::openat(fs_id, "/", &path, true) {
            Ok(fd) => {
                if (flags & FAN_CLOEXEC) != 0 {
                    let _ = crate::modules::posix::fs::fcntl_set_descriptor_flags(
                        fd,
                        crate::modules::posix_consts::net::FD_CLOEXEC,
                    );
                }
                if (flags & FAN_NONBLOCK) != 0 {
                    let _ = crate::modules::posix::fs::fcntl_set_status_flags(
                        fd,
                        crate::modules::posix_consts::net::O_NONBLOCK,
                    );
                }
                FANOTIFY_MARKS_BY_FD.lock().insert(fd, alloc::vec::Vec::new());
                fd as usize
            }
            Err(err) => linux_errno(err.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let id = NEXT_FANOTIFY_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let _ = (flags, event_f_flags);
        FANOTIFY_MARKS_BY_FD
            .lock()
            .entry((FANOTIFY_FD_BASE as u32).saturating_add(id))
            .or_default();
        FANOTIFY_FD_BASE.saturating_add(id as usize)
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_fanotify_mark(
    fanotify_fd: usize,
    flags: usize,
    mask: usize,
    dirfd: isize,
    path_ptr: usize,
) -> usize {
    const FAN_MARK_ADD: usize = 0x0000_0001;
    const FAN_MARK_REMOVE: usize = 0x0000_0002;
    const FAN_MARK_FLUSH: usize = 0x0000_0080;

    let op_count = usize::from((flags & FAN_MARK_ADD) != 0)
        + usize::from((flags & FAN_MARK_REMOVE) != 0)
        + usize::from((flags & FAN_MARK_FLUSH) != 0);
    if op_count != 1 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let fd = fanotify_fd as u32;
    let mut marks = FANOTIFY_MARKS_BY_FD.lock();
    let Some(fd_marks) = marks.get_mut(&fd) else {
        return linux_errno(crate::modules::posix_consts::errno::EBADF);
    };

    if (flags & FAN_MARK_FLUSH) != 0 {
        fd_marks.clear();
        return 0;
    }

    if path_ptr == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    if mask == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let path = match read_user_c_string_compat(path_ptr, crate::config::KernelConfig::syscall_max_path_len()) {
        Ok(v) => v,
        Err(e) => return e,
    };

    if (flags & FAN_MARK_ADD) != 0 {
        fd_marks.push(FanotifyMarkState {
            mask,
            dirfd,
            path,
        });
        return 0;
    }

    if let Some(idx) = fd_marks
        .iter()
        .position(|entry| entry.path == path && entry.dirfd == dirfd && entry.mask == mask)
    {
        fd_marks.swap_remove(idx);
        return 0;
    }

    linux_errno(crate::modules::posix_consts::errno::ENOENT)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_futex_waitv(
    waiters_ptr: usize,
    nr_futexes: usize,
    flags: usize,
    timeout_ptr: usize,
) -> usize {
    if flags != 0 || waiters_ptr == 0 || nr_futexes == 0 || nr_futexes > crate::generated_consts::LINUX_FUTEX_WAITV_MAX {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let item_sz = core::mem::size_of::<LinuxFutexWaitVCompat>();
    for i in 0..nr_futexes {
        let ptr = match waiters_ptr.checked_add(i.saturating_mul(item_sz)) {
            Some(v) => v,
            None => return linux_errno(crate::modules::posix_consts::errno::EFAULT),
        };
        let waiter = match read_user_struct::<LinuxFutexWaitVCompat>(ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        if waiter.__reserved != 0 {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
    }

    if timeout_ptr != 0 {
        let ts = match read_user_struct::<LinuxTimespecCompat>(timeout_ptr) {
            Ok(v) => v,
            Err(err) => return err,
        };
        if !validate_timespec_compat(ts) {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        }
        return linux_errno(crate::modules::posix_consts::errno::ETIMEDOUT);
    }

    linux_errno(crate::modules::posix_consts::errno::EAGAIN)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_signalfd(fd: usize, mask_ptr: usize, sizemask: usize) -> usize {
    sys_linux_signalfd4(fd, mask_ptr, sizemask, 0)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_signalfd4(
    fd: usize,
    mask_ptr: usize,
    sizemask: usize,
    flags: usize,
) -> usize {
    if sizemask != core::mem::size_of::<u64>() {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    let mask = match read_u64_from_user(mask_ptr) {
        Ok(v) => v,
        Err(e) => return e,
    };

    #[cfg(feature = "posix_signal")]
    {
        let raw_fd = fd as i32;
        let result = if raw_fd >= 0 {
            crate::modules::posix::signal::signalfd_reconfigure_errno(raw_fd as u32, mask, flags as i32)
        } else {
            crate::modules::posix::signal::signalfd_create_errno(mask, flags as i32)
        };
        match result {
            Ok(out_fd) => out_fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_signal"))]
    {
        let raw_fd = fd as i32;
        if raw_fd >= 0 {
            let mut table = SIGNALFD_MASK_BY_FD.lock();
            let Some(slot) = table.get_mut(&(raw_fd as u32)) else {
                return linux_errno(crate::modules::posix_consts::errno::EBADF);
            };
            *slot = mask;
            return raw_fd as usize;
        }

        let id = NEXT_SIGNALFD_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let out_fd = (SIGNALFD_FD_BASE as u32).saturating_add(id);
        SIGNALFD_MASK_BY_FD.lock().insert(out_fd, mask);
        let _ = flags;
        out_fd as usize
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_inotify_init() -> usize {
    sys_linux_inotify_init1(0)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_inotify_init1(flags: usize) -> usize {
    let allowed_flags = 0x0000_0800usize | 0x0008_0000usize;
    if (flags & !allowed_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::inotify_init(flags as i32) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let id = NEXT_INOTIFY_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let fd = (INOTIFY_FD_BASE as u32).saturating_add(id);
        INOTIFY_WATCHES_BY_FD.lock().insert(fd, alloc::vec::Vec::new());
        let _ = flags;
        fd as usize
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_inotify_add_watch(fd: usize, path_ptr: usize, mask: usize) -> usize {
    let path = match read_user_c_string_compat(path_ptr, crate::config::KernelConfig::syscall_max_path_len()) {
        Ok(v) => v,
        Err(e) => return e,
    };

    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::inotify_add_watch(fd as u32, &path, mask as u32) {
            Ok(wd) => wd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let fd = fd as u32;
        let mut watches = INOTIFY_WATCHES_BY_FD.lock();
        let Some(list) = watches.get_mut(&fd) else {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        };

        let wd = NEXT_INOTIFY_WD.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        list.push(InotifyWatchState {
            wd,
            path,
            mask: mask as u32,
        });
        wd as usize
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_inotify_rm_watch(fd: usize, wd: usize) -> usize {
    #[cfg(feature = "posix_fs")]
    {
        match crate::modules::posix::fs::inotify_rm_watch(fd as u32, wd as i32) {
            Ok(()) => 0,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let fd = fd as u32;
        let mut watches = INOTIFY_WATCHES_BY_FD.lock();
        let Some(list) = watches.get_mut(&fd) else {
            return linux_errno(crate::modules::posix_consts::errno::EBADF);
        };

        let target = wd as i32;
        let Some(index) = list.iter().position(|entry| entry.wd == target) else {
            return linux_errno(crate::modules::posix_consts::errno::EINVAL);
        };
        let removed = list.swap_remove(index);
        let _ = (removed.path, removed.mask);
        0
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_memfd_create(name_ptr: usize, flags: usize) -> usize {
    use crate::kernel::syscalls::syscalls_consts::linux::memfd_flags::{
        MFD_ALLOW_SEALING, MFD_CLOEXEC, MFD_EXEC, MFD_HUGETLB, MFD_NOEXEC_SEAL,
    };

    let known_flags =
        MFD_CLOEXEC | MFD_ALLOW_SEALING | MFD_HUGETLB | MFD_NOEXEC_SEAL | MFD_EXEC | crate::kernel::syscalls::syscalls_consts::linux::MFD_HUGE_MASK;
    if (flags & !known_flags) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if (flags & MFD_EXEC) != 0 && (flags & MFD_NOEXEC_SEAL) != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let raw_name = if name_ptr == 0 {
        alloc::string::String::from("memfd")
    } else {
        match read_user_c_string_compat(name_ptr, 255) {
            Ok(v) if !v.is_empty() => v,
            Ok(_) => alloc::string::String::from("memfd"),
            Err(e) => return e,
        }
    };

    #[cfg(feature = "posix_fs")]
    {
        use core::sync::atomic::{AtomicU32, Ordering};

        static NEXT_MEMFD_ID: AtomicU32 = AtomicU32::new(1);

        let id = NEXT_MEMFD_ID.fetch_add(1, Ordering::Relaxed);
        let path = alloc::format!("/.memfd-{}-{}", id, raw_name.replace('/', "_"));
        let fs_id = match crate::modules::posix::fs::default_fs_id() {
            Ok(v) => v,
            Err(e) => return linux_errno(e.code()),
        };
        match crate::modules::posix::fs::openat(fs_id, "/", &path, true) {
            Ok(fd) => fd as usize,
            Err(e) => linux_errno(e.code()),
        }
    }
    #[cfg(not(feature = "posix_fs"))]
    {
        let id = NEXT_MEMFD_SYNTH_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let fd = (MEMFD_FD_BASE as u32).saturating_add(id);
        MEMFD_NAME_BY_FD.lock().insert(fd, raw_name);
        fd as usize
    }
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_membarrier(cmd: usize, _flags: usize, _cpu_id: usize) -> usize {
    const MEMBARRIER_CMD_QUERY: usize = 0;
    const MEMBARRIER_CMD_GLOBAL: usize = 1 << 0;
    const MEMBARRIER_CMD_GLOBAL_EXPEDITED: usize = 1 << 1;
    const MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED: usize = 1 << 2;
    const MEMBARRIER_CMD_PRIVATE_EXPEDITED: usize = 1 << 3;
    const MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED: usize = 1 << 4;
    const MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE: usize = 1 << 5;

    let supported = MEMBARRIER_CMD_GLOBAL
        | MEMBARRIER_CMD_GLOBAL_EXPEDITED
        | MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED
        | MEMBARRIER_CMD_PRIVATE_EXPEDITED
        | MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED
        | MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE;

    if cmd == MEMBARRIER_CMD_QUERY {
        return supported;
    }
    if (cmd & supported) != 0 {
        return 0;
    }
    linux_errno(crate::modules::posix_consts::errno::EINVAL)
}

#[cfg(not(feature = "linux_compat"))]
pub(super) fn sys_linux_rseq(
    rseq_ptr: usize,
    rseq_len: usize,
    flags: usize,
    _sig: usize,
) -> usize {
    // Many modern runtimes probe rseq during startup; accepting valid registration avoids
    // brittle early-process failures while still rejecting malformed descriptors.
    if flags != 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }
    if rseq_ptr == 0 || rseq_len < 32 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    // Enforce user-space memory reachability up-front so later rseq reads/writes have
    // deterministic EFAULT behavior instead of deferred faults.
    if with_user_read_bytes(rseq_ptr, rseq_len, |_| 0usize).is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    if with_user_write_bytes(rseq_ptr, rseq_len, |_| 0usize).is_err() {
        return linux_errno(crate::modules::posix_consts::errno::EFAULT);
    }
    0
}

#[cfg(all(test, not(feature = "linux_compat")))]
mod modern_syscall_policy_tests {
    use super::*;

    #[test_case]
    fn io_uring_setup_rejects_zero_entries() {
        let params = [0u8; 16];
        assert_eq!(
            sys_linux_io_uring_setup(0, params.as_ptr() as usize),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn bpf_map_create_rejects_null_attr() {
        assert_eq!(
            sys_linux_bpf(0, 0, 16),
            linux_errno(crate::modules::posix_consts::errno::EFAULT)
        );
    }

    #[test_case]
    fn landlock_ruleset_rejects_nonzero_flags() {
        let attr = [0u8; 16];
        assert_eq!(
            sys_linux_landlock_create_ruleset(attr.as_ptr() as usize, attr.len(), 1),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }

    #[test_case]
    fn fanotify_init_allocates_descriptor() {
        let fd = sys_linux_fanotify_init(0, 0);
        assert!(fd > 0, "fanotify_init should return a valid descriptor token");
    }

    #[test_case]
    fn fanotify_mark_rejects_invalid_fd() {
        let path = b"/tmp\0";
        assert_eq!(
            sys_linux_fanotify_mark(0xFFFF, 0x1, 1, -1, path.as_ptr() as usize),
            linux_errno(crate::modules::posix_consts::errno::EBADF)
        );
    }

    #[test_case]
    fn futex_waitv_rejects_invalid_flags() {
        let waiter = LinuxFutexWaitVCompat::default();
        assert_eq!(
            sys_linux_futex_waitv((&waiter as *const LinuxFutexWaitVCompat) as usize, 1, 1, 0),
            linux_errno(crate::modules::posix_consts::errno::EINVAL)
        );
    }
}

