use alloc::collections::BTreeSet;
use core::sync::atomic::{AtomicU32, Ordering};
use lazy_static::lazy_static;
use spin::Mutex as SpinMutex;

use crate::modules::posix::PosixErrno;

pub type PthreadT = usize;

const THREAD_JOIN_KEY_SALT: u64 = 0xA11C_EB55_0000_0000;

lazy_static! {
    static ref TERMINATED_THREADS: SpinMutex<BTreeSet<PthreadT>> = SpinMutex::new(BTreeSet::new());
    static ref DETACHED_THREADS: SpinMutex<BTreeSet<PthreadT>> = SpinMutex::new(BTreeSet::new());
}

#[inline(always)]
fn join_key(thread: PthreadT) -> u64 {
    THREAD_JOIN_KEY_SALT ^ (thread as u64)
}

#[inline(always)]
fn is_terminated(thread: PthreadT) -> bool {
    TERMINATED_THREADS.lock().contains(&thread)
}

fn mark_terminated(thread: PthreadT) {
    TERMINATED_THREADS.lock().insert(thread);
    let _ = crate::modules::posix::ipc::futex_wake(join_key(thread), usize::MAX);
    if DETACHED_THREADS.lock().contains(&thread) {
        clear_lifecycle(thread);
    }
}

fn clear_lifecycle(thread: PthreadT) {
    TERMINATED_THREADS.lock().remove(&thread);
    DETACHED_THREADS.lock().remove(&thread);
}

pub fn pthread_register(thread: PthreadT) -> Result<(), PosixErrno> {
    if thread == 0 {
        return Err(PosixErrno::Invalid);
    }
    clear_lifecycle(thread);
    Ok(())
}

#[inline(always)]
pub fn pthread_self() -> PthreadT {
    crate::modules::posix::process::gettid()
}

#[inline(always)]
pub fn pthread_equal(a: PthreadT, b: PthreadT) -> bool {
    a == b
}

#[inline(always)]
pub fn sched_yield() {
    crate::kernel::rt_preemption::request_forced_reschedule();
}

pub fn pthread_kill(thread: PthreadT, signal: i32) -> Result<(), PosixErrno> {
    #[cfg(feature = "process_abstraction")]
    {
        let Some(pid) =
            crate::kernel::launch::process_id_by_task(crate::interfaces::task::TaskId(thread))
        else {
            return Err(PosixErrno::NoEntry);
        };
        crate::modules::posix::process::kill(pid.0, signal)?;
        Ok(())
    }
    #[cfg(not(feature = "process_abstraction"))]
    {
        if thread == pthread_self() {
            crate::modules::posix::process::kill(crate::modules::posix::process::getpid(), signal)?;
            Ok(())
        } else {
            Err(PosixErrno::NoEntry)
        }
    }
}

#[inline(always)]
pub fn thread_exists(thread: PthreadT) -> bool {
    if thread == 0 || is_terminated(thread) {
        return false;
    }

    #[cfg(feature = "process_abstraction")]
    {
        crate::kernel::launch::process_id_by_task(crate::interfaces::task::TaskId(thread)).is_some()
    }
    #[cfg(not(feature = "process_abstraction"))]
    {
        thread == pthread_self() && thread != 0
    }
}

pub fn pthread_detach(thread: PthreadT) -> Result<(), PosixErrno> {
    if thread == 0 {
        return Err(PosixErrno::Invalid);
    }
    if !thread_exists(thread) && !is_terminated(thread) {
        Err(PosixErrno::NoEntry)
    } else {
        if is_terminated(thread) {
            clear_lifecycle(thread);
            return Ok(());
        }
        DETACHED_THREADS.lock().insert(thread);
        Ok(())
    }
}

pub fn pthread_join(thread: PthreadT, spin_budget: u64) -> Result<(), PosixErrno> {
    if spin_budget == 0 {
        return Err(PosixErrno::Invalid);
    }

    if thread == 0 || thread == pthread_self() {
        return Err(PosixErrno::Invalid);
    }

    if DETACHED_THREADS.lock().contains(&thread) {
        return Err(PosixErrno::Invalid);
    }

    for _ in 0..spin_budget {
        if is_terminated(thread) || !thread_exists(thread) {
            clear_lifecycle(thread);
            return Ok(());
        }
        crate::modules::posix::ipc::futex_wait(join_key(thread), 0, 0)?;
        sched_yield();
    }

    Err(PosixErrno::TimedOut)
}

pub fn pthread_exit() -> Result<(), PosixErrno> {
    let tid = pthread_self();
    if tid == 0 {
        return Err(PosixErrno::Invalid);
    }

    mark_terminated(tid);

    #[cfg(feature = "process_abstraction")]
    {
        if crate::kernel::launch::terminate_task(crate::interfaces::task::TaskId(tid)) {
            crate::kernel::rt_preemption::request_forced_reschedule();
            Ok(())
        } else {
            TERMINATED_THREADS.lock().remove(&tid);
            Err(PosixErrno::NoEntry)
        }
    }
    #[cfg(not(feature = "process_abstraction"))]
    {
        sched_yield();
        Ok(())
    }
}

#[cfg(feature = "process_abstraction")]
fn map_launch_error(err: crate::kernel::launch::LaunchError) -> PosixErrno {
    match err {
        crate::kernel::launch::LaunchError::LoaderFailed => PosixErrno::Invalid,
        crate::kernel::launch::LaunchError::SchedulerUnavailable => PosixErrno::Again,
        crate::kernel::launch::LaunchError::InvalidSpawnRequest => PosixErrno::Invalid,
    }
}

pub fn pthread_create_from_image(
    process_name: &[u8],
    image: &[u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<PthreadT, PosixErrno> {
    #[cfg(feature = "process_abstraction")]
    {
        let (_pid, tid) = crate::kernel::launch::spawn_bootstrap_from_image(
            process_name,
            image,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
        )
        .map_err(map_launch_error)?;
        clear_lifecycle(tid);
        Ok(tid)
    }
    #[cfg(not(feature = "process_abstraction"))]
    {
        let _ = (
            process_name,
            image,
            priority,
            deadline,
            burst_time,
            kernel_stack_top,
        );
        Err(PosixErrno::Again)
    }
}

pub struct PthreadMutex {
    state: AtomicU32,
    key: u64,
}

impl PthreadMutex {
    pub const fn new(key: u64) -> Self {
        Self {
            state: AtomicU32::new(0),
            key,
        }
    }

    #[inline(always)]
    pub fn key(&self) -> u64 {
        self.key
    }

    pub fn lock(&self) -> Result<(), PosixErrno> {
        loop {
            if self
                .state
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Ok(());
            }

            crate::modules::posix::ipc::futex_wait(self.key, 1, 1)?;
            sched_yield();
        }
    }

    pub fn try_lock(&self) -> Result<bool, PosixErrno> {
        match self
            .state
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => Ok(true),
            Err(1) => Ok(false),
            Err(_) => Err(PosixErrno::Other),
        }
    }

    pub fn unlock(&self) -> Result<(), PosixErrno> {
        if self.state.swap(0, Ordering::Release) == 0 {
            return Err(PosixErrno::Invalid);
        }
        let _ = crate::modules::posix::ipc::futex_wake(self.key, 1)?;
        Ok(())
    }
}

pub struct PthreadCondvar {
    key: u64,
}

pub struct PosixSemaphore {
    value: AtomicU32,
    key: u64,
}

impl PosixSemaphore {
    pub const fn new(initial: u32, key: u64) -> Self {
        Self {
            value: AtomicU32::new(initial),
            key,
        }
    }

    pub fn post(&self) -> Result<(), PosixErrno> {
        self.value.fetch_add(1, Ordering::Release);
        let _ = crate::modules::posix::ipc::futex_wake(self.key, 1)?;
        Ok(())
    }

    pub fn wait(&self) -> Result<(), PosixErrno> {
        loop {
            let observed = self.value.load(Ordering::Acquire);
            if observed > 0 {
                if self
                    .value
                    .compare_exchange(observed, observed - 1, Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
                {
                    return Ok(());
                }
                continue;
            }

            crate::modules::posix::ipc::futex_wait(self.key, 0, 0)?;
            sched_yield();
        }
    }

    pub fn try_wait(&self) -> Result<bool, PosixErrno> {
        let observed = self.value.load(Ordering::Acquire);
        if observed == 0 {
            return Ok(false);
        }

        match self.value.compare_exchange(
            observed,
            observed - 1,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

pub struct PthreadRwLock {
    readers: AtomicU32,
    writer: AtomicU32,
    key: u64,
}

impl PthreadRwLock {
    pub const fn new(key: u64) -> Self {
        Self {
            readers: AtomicU32::new(0),
            writer: AtomicU32::new(0),
            key,
        }
    }

    pub fn rdlock(&self) -> Result<(), PosixErrno> {
        loop {
            while self.writer.load(Ordering::Acquire) != 0 {
                crate::modules::posix::ipc::futex_wait(self.key, 1, 1)?;
                sched_yield();
            }

            self.readers.fetch_add(1, Ordering::AcqRel);
            if self.writer.load(Ordering::Acquire) == 0 {
                return Ok(());
            }

            self.readers.fetch_sub(1, Ordering::AcqRel);
        }
    }

    pub fn wrlock(&self) -> Result<(), PosixErrno> {
        loop {
            if self
                .writer
                .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                while self.readers.load(Ordering::Acquire) != 0 {
                    let readers = self.readers.load(Ordering::Acquire);
                    if readers != 0 {
                        crate::modules::posix::ipc::futex_wait(self.key, readers, readers)?;
                        sched_yield();
                    }
                }
                return Ok(());
            }

            crate::modules::posix::ipc::futex_wait(self.key, 1, 1)?;
            sched_yield();
        }
    }

    pub fn unlock(&self) -> Result<(), PosixErrno> {
        if self.writer.load(Ordering::Acquire) != 0 {
            self.writer.store(0, Ordering::Release);
            let _ = crate::modules::posix::ipc::futex_wake(self.key, usize::MAX)?;
            return Ok(());
        }

        let prev = self.readers.fetch_sub(1, Ordering::AcqRel);
        if prev == 0 {
            self.readers.store(0, Ordering::Release);
            return Err(PosixErrno::Invalid);
        }

        if prev == 1 {
            let _ = crate::modules::posix::ipc::futex_wake(self.key, usize::MAX)?;
        }
        Ok(())
    }
}

impl PthreadCondvar {
    pub const fn new(key: u64) -> Self {
        Self { key }
    }

    #[inline(always)]
    pub fn key(&self) -> u64 {
        self.key
    }

    pub fn wait(&self, mutex: &PthreadMutex) -> Result<(), PosixErrno> {
        mutex.unlock()?;
        crate::modules::posix::ipc::futex_wait(self.key, 0, 0)?;
        mutex.lock()?;
        Ok(())
    }

    pub fn signal(&self) -> Result<usize, PosixErrno> {
        crate::modules::posix::ipc::futex_wake(self.key, 1)
    }

    pub fn broadcast(&self) -> Result<usize, PosixErrno> {
        crate::modules::posix::ipc::futex_wake(self.key, usize::MAX)
    }
}
