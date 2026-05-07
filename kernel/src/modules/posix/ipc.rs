use crate::interfaces::IpcChannel;
#[cfg(feature = "ipc_futex")]
use crate::modules::ipc::futex::FutexWaitResult;
use crate::modules::posix::PosixErrno;

static GLOBAL_SIGNAL_CHANNEL: crate::modules::ipc::signal_only::SignalOnly =
    crate::modules::ipc::signal_only::SignalOnly::new();

#[cfg(feature = "ipc_futex")]
pub fn futex_wait(key: u64, observed: u32, expected: u32) -> Result<(), PosixErrno> {
    match crate::modules::ipc::futex::global().wait(key, observed, expected) {
        FutexWaitResult::Enqueued => Ok(()),
        FutexWaitResult::ValueMismatch => Err(PosixErrno::Again),
    }
}
#[cfg(not(feature = "ipc_futex"))]
pub fn futex_wait(_key: u64, _observed: u32, _expected: u32) -> Result<(), PosixErrno> {
    Err(PosixErrno::NoSys)
}

#[cfg(feature = "ipc_futex")]
pub fn futex_wake(key: u64, max_wake: usize) -> Result<usize, PosixErrno> {
    if max_wake == 0 {
        return Err(PosixErrno::Invalid);
    }
    Ok(crate::modules::ipc::futex::global().wake(key, max_wake))
}
#[cfg(not(feature = "ipc_futex"))]
pub fn futex_wake(_key: u64, _max_wake: usize) -> Result<usize, PosixErrno> {
    Err(PosixErrno::NoSys)
}

#[inline(always)]
pub fn futex_pending_waiters(key: u64) -> usize {
    let _ = key;
    0
}

pub fn futex_receive_wake_event() -> Option<(u64, usize)> {
    None
}

pub fn signal_notify() {
    GLOBAL_SIGNAL_CHANNEL.send(&[]);
}

pub fn signal_try_wait() -> bool {
    let mut scratch = [0u8; 1];
    GLOBAL_SIGNAL_CHANNEL.receive(&mut scratch).is_some()
}
