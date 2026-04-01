use crate::interfaces::IpcChannel;
use crate::modules::ipc::futex::FutexWaitResult;
use crate::modules::posix::PosixErrno;

static GLOBAL_SIGNAL_CHANNEL: crate::modules::ipc::signal_only::SignalOnly =
    crate::modules::ipc::signal_only::SignalOnly::new();

pub fn futex_wait(key: u64, observed: u32, expected: u32) -> Result<(), PosixErrno> {
    match crate::modules::ipc::futex::global().wait(key, observed, expected) {
        FutexWaitResult::Enqueued => Ok(()),
        FutexWaitResult::ValueMismatch => Err(PosixErrno::Again),
    }
}

pub fn futex_wake(key: u64, max_wake: usize) -> Result<usize, PosixErrno> {
    if max_wake == 0 {
        return Err(PosixErrno::Invalid);
    }
    Ok(crate::modules::ipc::futex::global().wake(key, max_wake))
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
