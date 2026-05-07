use super::*;
use crate::modules::posix::ipc;

#[test_case]
#[cfg(feature = "posix_ipc")]
fn ipc_futex_and_signal_flow_works() {
    let key = 0xBEEF_u64;
    assert_eq!(ipc::futex_wait(key, 7, 8), Err(PosixErrno::Again));
    ipc::futex_wait(key, 7, 7).expect("futex wait");
    assert!(ipc::futex_pending_waiters(key) >= 1);
    let woke = ipc::futex_wake(key, 1).expect("futex wake");
    assert_eq!(woke, 1);

    let evt = ipc::futex_receive_wake_event().expect("wake event");
    assert_eq!(evt.0, key);
    assert_eq!(evt.1, 1);

    assert!(!ipc::signal_try_wait());
    ipc::signal_notify();
    assert!(ipc::signal_try_wait());
    assert!(!ipc::signal_try_wait());
}
