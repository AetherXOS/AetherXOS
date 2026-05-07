use super::*;
use crate::modules::posix::{signal, process};

#[test_case]
#[cfg(all(feature = "posix_signal", feature = "posix_process", feature = "posix_time"))]
fn posix_signal_mask_action_and_pause_flow() {
    assert!(signal::sigisemptyset(signal::sigemptyset()));
    let mut set = signal::sigemptyset();
    signal::sigaddset(&mut set, crate::modules::posix_consts::signal::SIGUSR1).expect("sigaddset");
    assert!(signal::sigismember(set, crate::modules::posix_consts::signal::SIGUSR1).expect("sigismember"));

    let old = signal::sigprocmask(signal::SigmaskHow::Block, Some(set)).expect("sigprocmask block");
    assert_eq!(old, 0);
    let old2 = signal::pthread_sigmask(signal::SigmaskHow::Unblock, Some(set)).expect("pthread_sigmask");
    assert_eq!(old2, set);
    signal::sigprocmask(signal::SigmaskHow::Block, Some(set)).expect("reblock");

    signal::kill(process::getpid(), 0).expect("kill self probe");

    signal::raise(crate::modules::posix_consts::signal::SIGUSR1).expect("raise queued");
    signal::kill(process::getpid(), crate::modules::posix_consts::signal::SIGUSR1)
        .expect("kill self queued");
    let pending = signal::sigpending();
    assert_ne!(pending, 0);

    let resumed = signal::sigsuspend(0).expect("sigsuspend delivery");
    assert_eq!(resumed, crate::modules::posix_consts::signal::SIGUSR1);

    signal::sigqueue(process::getpid(), crate::modules::posix_consts::signal::SIGUSR1).expect("sigqueue suspend timeout");
    let resumed_opt = signal::sigsuspend_timeout(
        0,
        PosixTimespec { sec: 0, nsec: 1_000_000 },
    )
    .expect("sigsuspend_timeout");
    assert_eq!(resumed_opt, Some(crate::modules::posix_consts::signal::SIGUSR1));

    signal::sigqueue(process::getpid(), crate::modules::posix_consts::signal::SIGUSR2).expect("sigqueue");
    let mut waitset = signal::sigemptyset();
    signal::sigaddset(&mut waitset, crate::modules::posix_consts::signal::SIGUSR2).expect("waitset add");
    let waited_blocking = signal::sigwait(waitset).expect("sigwait");
    assert_eq!(waited_blocking, crate::modules::posix_consts::signal::SIGUSR2);

    signal::sigqueue(process::getpid(), crate::modules::posix_consts::signal::SIGUSR2).expect("sigqueue second");
    let waited = signal::sigtimedwait(waitset, 8).expect("sigtimedwait");
    assert_eq!(waited, Some(crate::modules::posix_consts::signal::SIGUSR2));
    signal::killpg(process::getpgrp(), 0).expect("killpg probe");
    signal::killpg(process::getpgrp(), crate::modules::posix_consts::signal::SIGUSR2)
        .expect("killpg queued");
    let waited_pg = signal::sigtimedwait(waitset, 8).expect("sigtimedwait killpg");
    assert_eq!(waited_pg, Some(crate::modules::posix_consts::signal::SIGUSR2));
    signal::tkill(process::gettid(), crate::modules::posix_consts::signal::SIGUSR2).expect("tkill queued");
    let waited_tkill = signal::sigtimedwait(waitset, 8).expect("sigtimedwait tkill");
    assert_eq!(waited_tkill, Some(crate::modules::posix_consts::signal::SIGUSR2));
    signal::tgkill(process::getpid(), process::gettid(), crate::modules::posix_consts::signal::SIGUSR2)
        .expect("tgkill queued");
    let waited_tgkill = signal::sigtimedwait(waitset, 8).expect("sigtimedwait tgkill");
    assert_eq!(waited_tgkill, Some(crate::modules::posix_consts::signal::SIGUSR2));
    signal::sigqueue(process::getpid(), crate::modules::posix_consts::signal::SIGUSR2).expect("sigqueue third");
    let ts_wait = signal::sigtimedwait_ts(
        waitset,
        PosixTimespec { sec: 0, nsec: 1_000_000 },
    )
    .expect("sigtimedwait ts");
    assert_eq!(ts_wait, Some(crate::modules::posix_consts::signal::SIGUSR2));
    assert_eq!(signal::sigtimedwait(waitset, 0).expect("sigtimedwait immediate"), None);

    signal::signal_action(
        crate::modules::posix_consts::signal::SIGUSR1,
        None,
        0,
        crate::modules::posix_consts::signal::SA_RESETHAND,
    )
    .expect("sigaction reset");
    signal::raise(crate::modules::posix_consts::signal::SIGUSR1).expect("raise reset");
    let old = signal::signal_action(
        crate::modules::posix_consts::signal::SIGUSR1,
        None,
        0,
        0,
    )
    .expect("sigaction readback");
    assert!(old.is_none());

    let _ = signal::sigprocmask(signal::SigmaskHow::SetMask, Some(0));
    assert_eq!(signal::pause(), Err(super::PosixErrno::TimedOut));

    let old_stack = signal::sigaltstack(None).expect("sigaltstack read");
    assert_eq!(old_stack, None);
    assert_eq!(
        signal::sigaltstack(Some(crate::interfaces::task::SignalStack {
            ss_sp: 0x1000,
            ss_flags: 0,
            ss_size: 1024,
        })),
        Err(super::PosixErrno::Invalid)
    );
    let installed = crate::interfaces::task::SignalStack {
        ss_sp: 0x2000,
        ss_flags: 0,
        ss_size: 8192,
    };
    let prev_stack = signal::sigaltstack(Some(installed)).expect("sigaltstack install");
    assert_eq!(prev_stack, None);
    assert_eq!(signal::sigaltstack(None).expect("sigaltstack reread"), Some(installed));
    let disabled_prev = signal::sigaltstack(Some(crate::interfaces::task::SignalStack {
        ss_sp: 0,
        ss_flags: crate::modules::posix_consts::signal::SS_DISABLE,
        ss_size: 0,
    }))
    .expect("sigaltstack disable");
    assert_eq!(disabled_prev, Some(installed));
    assert_eq!(signal::sigaltstack(None).expect("sigaltstack disabled read"), None);
}
