use crate::modules::posix::PosixErrno;
use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
#[path = "process/identity_env.rs"]
mod identity_env;
#[path = "process/exec_runtime.rs"]
mod exec_runtime;
#[path = "process/process_support.rs"]
mod process_support;
#[path = "process/wait_support.rs"]
mod wait_support;
#[path = "process/wait_api.rs"]
mod wait_api;
#[path = "process/runtime_control.rs"]
mod runtime_control;
#[path = "process/lifecycle_ops.rs"]
mod lifecycle_ops;
#[path = "process/process_groups.rs"]
mod process_groups;
pub use identity_env::{
    clearenv, current_umask, environ_snapshot, get_domainname, get_groups_len, get_groups_snapshot,
    get_hostname, get_personality, getdomainname, getegid, getenv, geteuid, getgid, getgroups,
    gethostname, getresgid, getresuid, getuid, initgroups, set_personality, setdomainname, setegid,
    setenv, seteuid, setgid, setgroups, sethostname, setresgid, setresuid, setuid, umask, unsetenv,
};
#[cfg(feature = "vfs")]
use process_support::basename_bytes;
use process_support::{
    alarm_ticks_from_seconds, clamp_nice, normalize_target_pid, remaining_alarm_seconds,
    validate_rlimit_pair, validate_rlimit_resource,
};
use wait_support::wait_code_from_status;
pub use wait_support::{
    encode_wait_exit_status, encode_wait_signal_status, wait_exit_code, wait_exited, wait_signaled,
    wait_term_signal,
};
const WAITPID_SPIN_BUDGET: usize = 4096;

lazy_static! {
    static ref EXIT_STATUS_TABLE: Mutex<BTreeMap<usize, i32>> = Mutex::new(BTreeMap::new());
    static ref NICE_VALUES: Mutex<BTreeMap<usize, i32>> = Mutex::new(BTreeMap::new());
    static ref EXEC_FS_ID: Mutex<Option<u32>> = Mutex::new(None);
    static ref RLIMIT_TABLE: Mutex<BTreeMap<i32, (u64, u64)>> = Mutex::new(BTreeMap::new());
    static ref PIDFD_TABLE: Mutex<BTreeMap<u32, usize>> = Mutex::new(BTreeMap::new());
    static ref PROCESS_PARENTS: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());
    static ref PROCESS_GROUPS: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());
    static ref PROCESS_SESSIONS: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());
    static ref ALARM_TABLE: Mutex<BTreeMap<usize, u64>> = Mutex::new(BTreeMap::new());
}
static NEXT_PIDFD: AtomicU32 = AtomicU32::new(64_000);
static SCHED_POLICY: AtomicU32 =
    AtomicU32::new(crate::modules::posix_consts::process::SCHED_OTHER as u32);
static PROCESS_EVENT_EPOCH: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PosixWaitIdInfo {
    pub pid: usize,
    pub status: i32,
    pub code: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PosixRusage {
    pub ru_utime_ticks: u64,
    pub ru_stime_ticks: u64,
    pub ru_maxrss: u64,
    pub ru_minflt: u64,
    pub ru_majflt: u64,
    pub ru_nswap: u64,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixSignal {
    Term = crate::modules::posix_consts::process::SIGTERM,
    Kill = crate::modules::posix_consts::process::SIGKILL,
}

#[inline(always)]
fn record_exit_status(pid: usize, status: i32) {
    if pid != 0 {
        EXIT_STATUS_TABLE.lock().insert(pid, status);
        PROCESS_EVENT_EPOCH.fetch_add(1, Ordering::Relaxed);
    }
}

#[inline(always)]
fn take_exit_status(pid: usize) -> Option<i32> {
    EXIT_STATUS_TABLE.lock().remove(&pid)
}

#[inline(always)]
fn peek_exit_status(pid: usize) -> Option<i32> {
    EXIT_STATUS_TABLE.lock().get(&pid).copied()
}

#[inline(always)]
fn current_process_event_epoch() -> u64 {
    PROCESS_EVENT_EPOCH.load(Ordering::Relaxed)
}

#[inline(always)]
fn note_process_event() {
    PROCESS_EVENT_EPOCH.fetch_add(1, Ordering::Relaxed);
}

fn wait_for_process_event(snapshot: u64) -> bool {
    for _ in 0..WAITPID_SPIN_BUDGET {
        let now = current_process_event_epoch();
        if now != snapshot {
            return true;
        }
        crate::kernel::rt_preemption::request_forced_reschedule();
    }
    false
}

#[inline(always)]
fn ensure_process_metadata(pid: usize) {
    if pid == 0 {
        return;
    }
    PROCESS_PARENTS.lock().entry(pid).or_insert(0);
    PROCESS_GROUPS.lock().entry(pid).or_insert(pid);
    PROCESS_SESSIONS.lock().entry(pid).or_insert(pid);
}

#[inline(always)]
fn register_spawned_process(parent_pid: usize, child_pid: usize) {
    if child_pid == 0 {
        return;
    }
    let parent_group = PROCESS_GROUPS
        .lock()
        .get(&parent_pid)
        .copied()
        .unwrap_or(parent_pid);
    let parent_session = PROCESS_SESSIONS
        .lock()
        .get(&parent_pid)
        .copied()
        .unwrap_or(parent_pid);

    PROCESS_PARENTS.lock().insert(child_pid, parent_pid);
    PROCESS_GROUPS.lock().insert(child_pid, parent_group);
    PROCESS_SESSIONS.lock().insert(child_pid, parent_session);
    note_process_event();
}

#[inline(always)]
fn clear_process_metadata(pid: usize) {
    PROCESS_PARENTS.lock().remove(&pid);
    PROCESS_GROUPS.lock().remove(&pid);
    PROCESS_SESSIONS.lock().remove(&pid);
    note_process_event();
}

impl PosixSignal {
    pub fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            crate::modules::posix_consts::process::SIGTERM => Some(Self::Term),
            crate::modules::posix_consts::process::SIGKILL => Some(Self::Kill),
            _ => None,
        }
    }
}

#[inline(always)]
pub fn gettid() -> usize {
    unsafe {
        crate::kernel::cpu_local::CpuLocal::try_get()
            .map(|cpu| cpu.current_task.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn pid_for_tid(task_id: usize) -> Option<usize> {
    crate::kernel::launch::process_id_by_task(crate::interfaces::task::TaskId(task_id))
        .map(|pid| pid.0)
}

#[cfg(not(feature = "process_abstraction"))]
#[inline(always)]
fn pid_for_tid(task_id: usize) -> Option<usize> {
    let current = gettid();
    if current != 0 && task_id == current {
        Some(current)
    } else {
        None
    }
}

#[inline(always)]
pub fn getpid() -> usize {
    let tid = gettid();
    if tid == 0 {
        return 0;
    }
    let pid = pid_for_tid(tid).unwrap_or(0);
    ensure_process_metadata(pid);
    pid
}

#[inline(always)]
pub fn getppid() -> usize {
    process_groups::getppid()
}

pub fn getpgid(pid: usize) -> Result<usize, PosixErrno> {
    process_groups::getpgid(pid)
}

#[inline(always)]
pub fn getpgrp() -> usize {
    process_groups::getpgrp()
}

fn process_exists(pid: usize) -> bool {
    if pid == 0 {
        return false;
    }

    #[cfg(feature = "process_abstraction")]
    {
        let mut ids = [crate::interfaces::task::ProcessId(0); 64];
        let written = crate::kernel::launch::process_ids_snapshot(&mut ids);
        ids[..written].iter().any(|p| p.0 == pid)
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        pid == getpid()
    }
}

pub fn getsid(pid: usize) -> Result<usize, PosixErrno> {
    process_groups::getsid(pid)
}

pub fn setsid() -> Result<usize, PosixErrno> {
    process_groups::setsid()
}

pub fn setpgid(pid: usize, pgid: usize) -> Result<(), PosixErrno> {
    process_groups::setpgid(pid, pgid)
}

#[cfg(feature = "process_abstraction")]
pub fn process_count() -> usize {
    process_groups::process_count()
}

#[cfg(not(feature = "process_abstraction"))]
pub fn process_count() -> usize {
    process_groups::process_count()
}

#[cfg(feature = "process_abstraction")]
pub fn process_ids_snapshot(out: &mut [usize]) -> usize {
    process_groups::process_ids_snapshot(out)
}

#[cfg(not(feature = "process_abstraction"))]
pub fn process_ids_snapshot(out: &mut [usize]) -> usize {
    process_groups::process_ids_snapshot(out)
}

pub fn kill(pid: usize, signal: i32) -> Result<(), PosixErrno> {
    lifecycle_ops::kill(pid, signal)
}

pub fn exit_with_status(code: u8) -> Result<(), PosixErrno> {
    lifecycle_ops::exit_with_status(code)
}

#[inline(always)]
pub fn _exit(code: u8) -> Result<(), PosixErrno> {
    lifecycle_ops::_exit(code)
}

pub fn fork() -> Result<usize, PosixErrno> {
    lifecycle_ops::fork()
}

pub fn fork_from_image(
    process_name: &[u8],
    image: &[u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<usize, PosixErrno> {
    lifecycle_ops::fork_from_image(process_name, image, priority, deadline, burst_time, kernel_stack_top)
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
pub fn set_exec_fs(fs_id: u32) {
    exec_runtime::set_exec_fs(fs_id);
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
#[allow(dead_code)]
fn resolve_interp_path(image: &[u8]) -> Result<Option<String>, PosixErrno> {
    exec_runtime::resolve_interp_path(image)
}

pub fn execve(path: &str, _argv: &[&str], _envp: &[&str]) -> Result<(), PosixErrno> {
    exec_runtime::execve(path, _argv, _envp)
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
pub fn execveat(fs_id: u32, path: &str, _argv: &[&str], _envp: &[&str]) -> Result<(), PosixErrno> {
    exec_runtime::execveat(fs_id, path, _argv, _envp)
}

#[cfg(all(feature = "vfs", feature = "posix_fs"))]
pub fn posix_spawn_from_path(
    fs_id: u32,
    path: &str,
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<usize, PosixErrno> {
    exec_runtime::posix_spawn_from_path(fs_id, path, priority, deadline, burst_time, kernel_stack_top)
}

pub fn posix_spawn_from_image(
    process_name: &[u8],
    image: &[u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<usize, PosixErrno> {
    exec_runtime::posix_spawn_from_image(process_name, image, priority, deadline, burst_time, kernel_stack_top)
}

pub fn getpriority(pid: usize) -> Result<i32, PosixErrno> {
    lifecycle_ops::getpriority(pid)
}

pub fn setpriority(pid: usize, prio: i32) -> Result<(), PosixErrno> {
    lifecycle_ops::setpriority(pid, prio)
}

pub fn nice(increment: i32) -> Result<i32, PosixErrno> {
    lifecycle_ops::nice(increment)
}

pub fn raise(signal: i32) -> Result<(), PosixErrno> {
    lifecycle_ops::raise(signal)
}

pub fn killpg(pgid: usize, signal: i32) -> Result<(), PosixErrno> {
    lifecycle_ops::killpg(pgid, signal)
}

pub fn waitpid(pid: usize, nohang: bool) -> Result<Option<usize>, PosixErrno> {
    wait_api::waitpid(pid, nohang)
}

pub fn waitpid_options(pid: usize, options: i32) -> Result<Option<usize>, PosixErrno> {
    wait_api::waitpid_options(pid, options)
}

pub fn waitpid_status(pid: usize, nohang: bool) -> Result<Option<i32>, PosixErrno> {
    wait_api::waitpid_status(pid, nohang)
}

pub fn waitpid_status_options(pid: usize, options: i32) -> Result<Option<i32>, PosixErrno> {
    wait_api::waitpid_status_options(pid, options)
}

pub fn wait(nohang: bool) -> Result<Option<usize>, PosixErrno> {
    wait_api::wait(nohang)
}

pub fn wait_status(nohang: bool) -> Result<Option<(usize, i32)>, PosixErrno> {
    wait_api::wait_status(nohang)
}

pub fn wait_any_status(nohang: bool) -> Result<Option<(usize, i32)>, PosixErrno> {
    wait_api::wait_any_status(nohang)
}

pub fn waitid(
    id_type: i32,
    id: usize,
    options: i32,
) -> Result<Option<PosixWaitIdInfo>, PosixErrno> {
    wait_api::waitid(id_type, id, options)
}

pub fn wait4(pid: usize, options: i32) -> Result<Option<(usize, i32, PosixRusage)>, PosixErrno> {
    wait_api::wait4(pid, options)
}

pub fn wait3(options: i32) -> Result<Option<(usize, i32, PosixRusage)>, PosixErrno> {
    wait_api::wait3(options)
}

#[inline(always)]
pub fn pending_exit_status_count() -> usize {
    wait_api::pending_exit_status_count()
}

#[inline(always)]
pub fn get_cached_exit_status(pid: usize) -> Option<i32> {
    wait_api::get_cached_exit_status(pid)
}

pub const RLIMIT_NOFILE: i32 = process_support::RLIMIT_NOFILE;
pub const RLIMIT_NPROC: i32 = process_support::RLIMIT_NPROC;
pub const RLIMIT_STACK: i32 = process_support::RLIMIT_STACK;
pub const PRIO_PROCESS: i32 = 0;

pub fn getrlimit(resource: i32) -> Result<(u64, u64), PosixErrno> {
    runtime_control::getrlimit(resource)
}

pub fn setrlimit(resource: i32, soft: u64, hard: u64) -> Result<(), PosixErrno> {
    runtime_control::setrlimit(resource, soft, hard)
}

pub fn prlimit(
    pid: usize,
    resource: i32,
    new: Option<(u64, u64)>,
) -> Result<(u64, u64), PosixErrno> {
    runtime_control::prlimit(pid, resource, new)
}

pub fn sched_getscheduler(pid: usize) -> Result<i32, PosixErrno> {
    runtime_control::sched_getscheduler(pid)
}

pub fn sched_setscheduler(pid: usize, policy: i32, priority: i32) -> Result<(), PosixErrno> {
    runtime_control::sched_setscheduler(pid, policy, priority)
}

pub fn sched_getparam(pid: usize) -> Result<i32, PosixErrno> {
    runtime_control::sched_getparam(pid)
}

pub fn sched_setparam(pid: usize, priority: i32) -> Result<(), PosixErrno> {
    runtime_control::sched_setparam(pid, priority)
}

pub fn getcpu() -> Result<(u32, u32), PosixErrno> {
    runtime_control::getcpu()
}

pub fn getrusage(who: i32) -> Result<PosixRusage, PosixErrno> {
    runtime_control::getrusage(who)
}

pub fn getpgid_of(pid: usize) -> Result<usize, PosixErrno> {
    runtime_control::getpgid_of(pid)
}

pub fn parent_of(pid: usize) -> Result<usize, PosixErrno> {
    runtime_control::parent_of(pid)
}

pub fn pidfd_open(pid: usize) -> Result<u32, PosixErrno> {
    runtime_control::pidfd_open(pid)
}

pub fn pidfd_get_pid(pidfd: u32) -> Result<usize, PosixErrno> {
    runtime_control::pidfd_get_pid(pidfd)
}

pub fn pidfd_send_signal(pidfd: u32, signal: i32) -> Result<(), PosixErrno> {
    runtime_control::pidfd_send_signal(pidfd, signal)
}

pub fn pidfd_close(pidfd: u32) -> Result<(), PosixErrno> {
    runtime_control::pidfd_close(pidfd)
}

pub fn alarm(seconds: usize) -> usize {
    runtime_control::alarm(seconds)
}

pub fn get_process_name(pid: usize) -> Result<alloc::string::String, PosixErrno> {
    runtime_control::get_process_name(pid)
}

pub fn set_process_name(pid: usize, name: &str) -> Result<(), PosixErrno> {
    runtime_control::set_process_name(pid, name)
}
#[cfg(test)]
#[path = "process/tests.rs"]
mod tests;
