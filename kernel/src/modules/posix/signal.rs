use crate::modules::posix::PosixErrno;
#[cfg(all(feature = "posix_fs", feature = "vfs"))]
use crate::modules::vfs::File;
use alloc::collections::{BTreeMap, BTreeSet};
#[cfg(all(feature = "posix_fs", feature = "vfs"))]
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

pub type SigSet = u64;
pub type SignalHandler = fn(i32);

pub fn sig_ign(_signum: i32) {}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigmaskHow {
    Block = crate::modules::posix_consts::signal::SIG_BLOCK,
    Unblock = crate::modules::posix_consts::signal::SIG_UNBLOCK,
    SetMask = crate::modules::posix_consts::signal::SIG_SETMASK,
}

#[derive(Debug, Clone, Copy)]
pub struct SignalAction {
    pub handler: Option<SignalHandler>,
    pub restorer: u64,
    pub mask: SigSet,
    pub flags: u32,
}

const MAX_SIGNAL: i32 = 64;
const SIGNAL_WAIT_SPIN_BUDGET: usize = 4096;

static SIGNAL_DELIVERIES: AtomicU64 = AtomicU64::new(0);
static SIGNAL_PENDING_QUEUED: AtomicU64 = AtomicU64::new(0);
const POSIX_MINSIGSTKSZ: u64 = 2048;

lazy_static! {
    static ref SIGNAL_MASKS: Mutex<BTreeMap<usize, SigSet>> = Mutex::new(BTreeMap::new());
    pub(crate) static ref SIGNAL_PENDING: Mutex<BTreeMap<usize, BTreeSet<i32>>> =
        Mutex::new(BTreeMap::new());
    pub(crate) static ref SIGNAL_ACTIONS: Mutex<BTreeMap<(usize, i32), SignalAction>> =
        Mutex::new(BTreeMap::new());
}

#[inline(always)]
fn current_pid() -> usize {
    crate::modules::posix::process::getpid()
}

#[inline(always)]
pub fn current_pid_pub() -> usize {
    current_pid()
}

#[inline(always)]
fn sigbit(signum: i32) -> Option<SigSet> {
    if signum <= 0 || signum > MAX_SIGNAL {
        return None;
    }
    Some(1u64 << ((signum - 1) as u64))
}

#[inline(always)]
fn read_mask(pid: usize) -> SigSet {
    SIGNAL_MASKS.lock().get(&pid).copied().unwrap_or(0)
}

#[inline(always)]
fn write_mask(pid: usize, mask: SigSet) {
    SIGNAL_MASKS.lock().insert(pid, mask);
}

fn current_task_arc() -> Result<
    alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::task::KernelTask>>,
    PosixErrno,
> {
    let Some(cpu) = (unsafe { crate::kernel::cpu_local::CpuLocal::try_get() }) else {
        return Err(PosixErrno::Invalid);
    };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    crate::kernel::task::get_task(crate::interfaces::task::TaskId(current_tid))
        .ok_or(PosixErrno::Invalid)
}

pub fn sigemptyset() -> SigSet {
    0
}

pub fn sigfillset() -> SigSet {
    u64::MAX
}

pub fn sigaddset(set: &mut SigSet, signum: i32) -> Result<(), PosixErrno> {
    let bit = sigbit(signum).ok_or(PosixErrno::Invalid)?;
    *set |= bit;
    Ok(())
}

pub fn sigdelset(set: &mut SigSet, signum: i32) -> Result<(), PosixErrno> {
    let bit = sigbit(signum).ok_or(PosixErrno::Invalid)?;
    *set &= !bit;
    Ok(())
}

pub fn sigismember(set: SigSet, signum: i32) -> Result<bool, PosixErrno> {
    let bit = sigbit(signum).ok_or(PosixErrno::Invalid)?;
    Ok((set & bit) != 0)
}

#[inline(always)]
pub fn sigisemptyset(set: SigSet) -> bool {
    set == 0
}

pub fn sigprocmask(how: SigmaskHow, set: Option<SigSet>) -> Result<SigSet, PosixErrno> {
    let pid = current_pid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    let old = read_mask(pid);
    if let Some(newset) = set {
        let merged = match how {
            SigmaskHow::Block => old | newset,
            SigmaskHow::Unblock => old & !newset,
            SigmaskHow::SetMask => newset,
        };
        write_mask(pid, merged);
    }
    Ok(old)
}

#[inline(always)]
pub fn pthread_sigmask(how: SigmaskHow, set: Option<SigSet>) -> Result<SigSet, PosixErrno> {
    sigprocmask(how, set)
}

pub fn sigpending() -> SigSet {
    let pid = current_pid();
    if pid == 0 {
        return 0;
    }
    let pending = SIGNAL_PENDING.lock();
    let Some(set) = pending.get(&pid) else {
        return 0;
    };
    let mut out = 0u64;
    for signum in set {
        if let Some(bit) = sigbit(*signum) {
            out |= bit;
        }
    }
    out
}

pub fn sigaction(signum: i32, action: SignalAction) -> Result<Option<SignalAction>, PosixErrno> {
    let pid = current_pid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }
    let _ = sigbit(signum).ok_or(PosixErrno::Invalid)?;

    let mut actions = SIGNAL_ACTIONS.lock();
    let old = actions.insert((pid, signum), action);
    Ok(old)
}

pub fn signal(
    signum: i32,
    handler: Option<SignalHandler>,
) -> Result<Option<SignalAction>, PosixErrno> {
    sigaction(
        signum,
        SignalAction {
            handler,
            restorer: 0,
            mask: 0,
            flags: 0,
        },
    )
}

pub fn signal_action(
    signum: i32,
    handler: Option<SignalHandler>,
    mask: SigSet,
    flags: u32,
    restorer: u64,
) -> Result<Option<SignalAction>, PosixErrno> {
    sigaction(
        signum,
        SignalAction {
            handler,
            restorer,
            mask,
            flags,
        },
    )
}

fn deliver_to_pid(pid: usize, signum: i32) -> Result<(), PosixErrno> {
    let bit = sigbit(signum).ok_or(PosixErrno::Invalid)?;
    let mask = read_mask(pid);

    if (mask & bit) != 0 {
        SIGNAL_PENDING
            .lock()
            .entry(pid)
            .or_insert_with(BTreeSet::new)
            .insert(signum);
        SIGNAL_PENDING_QUEUED.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }

    SIGNAL_DELIVERIES.fetch_add(1, Ordering::Relaxed);
    if let Some(action) = SIGNAL_ACTIONS.lock().get(&(pid, signum)).copied() {
        let old_mask = read_mask(pid);
        let mut handler_mask = old_mask | action.mask;
        if (action.flags & crate::modules::posix_consts::signal::SA_NODEFER) == 0 {
            handler_mask |= bit;
        }
        write_mask(pid, handler_mask);

        if let Some(handler) = action.handler {
            handler(signum);
            if (action.flags & crate::modules::posix_consts::signal::SA_RESETHAND) != 0 {
                SIGNAL_ACTIONS.lock().remove(&(pid, signum));
            }
            let _restart = (action.flags & crate::modules::posix_consts::signal::SA_RESTART) != 0;
            write_mask(pid, old_mask);
            return Ok(());
        }

        write_mask(pid, old_mask);
    }

    match signum {
        crate::modules::posix_consts::process::SIGTERM
        | crate::modules::posix_consts::process::SIGKILL => {
            crate::modules::posix::process::kill(pid, signum)
        }
        _ => Ok(()),
    }
}

pub fn raise(signum: i32) -> Result<(), PosixErrno> {
    let pid = current_pid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }
    deliver_to_pid(pid, signum)
}

pub fn kill(pid: usize, signum: i32) -> Result<(), PosixErrno> {
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    if signum == 0 {
        return crate::modules::posix::process::kill(pid, 0);
    }

    let _ = sigbit(signum).ok_or(PosixErrno::Invalid)?;

    if pid == current_pid() {
        deliver_to_pid(pid, signum)
    } else {
        match signum {
            crate::modules::posix_consts::process::SIGTERM
            | crate::modules::posix_consts::process::SIGKILL => {
                crate::modules::posix::process::kill(pid, signum)
            }
            _ => {
                if crate::modules::posix::process::kill(pid, 0).is_err() {
                    return Err(PosixErrno::NoEntry);
                }
                SIGNAL_PENDING
                    .lock()
                    .entry(pid)
                    .or_insert_with(BTreeSet::new)
                    .insert(signum);
                SIGNAL_PENDING_QUEUED.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        }
    }
}

pub fn tkill(tid: usize, signum: i32) -> Result<(), PosixErrno> {
    if tid == 0 {
        return Err(PosixErrno::Invalid);
    }

    #[cfg(feature = "process_abstraction")]
    {
        let Some(pid) =
            crate::kernel::launch::process_id_by_task(crate::interfaces::task::TaskId(tid))
                .map(|pid| pid.0)
        else {
            return Err(PosixErrno::NoEntry);
        };
        return kill(pid, signum);
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        if tid != crate::modules::posix::process::gettid() {
            return Err(PosixErrno::NoEntry);
        }
        kill(crate::modules::posix::process::getpid(), signum)
    }
}

pub fn tgkill(tgid: usize, tid: usize, signum: i32) -> Result<(), PosixErrno> {
    if tgid == 0 || tid == 0 {
        return Err(PosixErrno::Invalid);
    }

    #[cfg(feature = "process_abstraction")]
    {
        let Some(pid) =
            crate::kernel::launch::process_id_by_task(crate::interfaces::task::TaskId(tid))
                .map(|pid| pid.0)
        else {
            return Err(PosixErrno::NoEntry);
        };
        if pid != tgid {
            return Err(PosixErrno::NoEntry);
        }
        return kill(pid, signum);
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        if tid != crate::modules::posix::process::gettid() {
            return Err(PosixErrno::NoEntry);
        }
        if tgid != crate::modules::posix::process::getpid() {
            return Err(PosixErrno::NoEntry);
        }
        kill(tgid, signum)
    }
}

pub fn killpg(pgid: usize, signum: i32) -> Result<(), PosixErrno> {
    if signum != 0 {
        let _ = sigbit(signum).ok_or(PosixErrno::Invalid)?;
    }

    let group = if pgid == 0 {
        crate::modules::posix::process::getpgrp()
    } else {
        pgid
    };

    if group == 0 {
        return Err(PosixErrno::Invalid);
    }

    let mut pids = [0usize; 64];
    let written = crate::modules::posix::process::process_ids_snapshot(&mut pids);
    if written == 0 {
        return Err(PosixErrno::NoEntry);
    }

    let mut delivered = false;
    for pid in pids.iter().copied().take(written) {
        if pid == 0 {
            continue;
        }
        if crate::modules::posix::process::getpgid_of(pid).ok() != Some(group) {
            continue;
        }
        if kill(pid, signum).is_ok() {
            delivered = true;
        }
    }

    if delivered {
        Ok(())
    } else {
        Err(PosixErrno::NoEntry)
    }
}

#[path = "signal/wait.rs"]
mod wait;

pub use wait::*;
