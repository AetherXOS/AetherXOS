    #[cfg(feature = "posix_ipc")]
    use super::ipc;
    use super::PosixErrno;
    #[cfg(feature = "posix_process")]
    use super::process;
    #[cfg(feature = "posix_signal")]
    use super::signal;
    #[cfg(feature = "posix_thread")]
    use super::thread;
    #[cfg(feature = "posix_pipe")]
    use super::pipe;
    #[cfg(feature = "posix_io")]
    use super::io;
    #[cfg(feature = "posix_fs")]
    use super::fs;
    #[cfg(all(feature = "vfs", feature = "posix_mman"))]
    use super::mman;
    #[cfg(feature = "posix_fs")]
    use super::fs::SeekWhence;
    #[cfg(feature = "posix_time")]
    use super::time::{
        PosixClockId,
        PosixTimespec,
        clock_getres,
        clock_getres_raw,
        clock_gettime,
        clock_gettime64,
        clock_gettime_raw,
        clock_settime,
        clock_settime_raw,
        clock_nanosleep,
        clock_nanosleep_raw,
        gettimeofday,
        nanosleep,
        nanosleep_with_rem,
        settimeofday,
        sleep,
        time_now,
        timespec_get,
        timespec_getres,
        usleep,
    };

    #[test_case]
    fn errno_numeric_codes_are_stable() {
        assert_eq!(PosixErrno::Again.code(), crate::modules::posix_consts::errno::EAGAIN);
        assert_eq!(PosixErrno::BadFileDescriptor.code(), crate::modules::posix_consts::errno::EBADF);
        assert_eq!(PosixErrno::Invalid.code(), crate::modules::posix_consts::errno::EINVAL);
        assert_eq!(PosixErrno::NoSys.code(), crate::modules::posix_consts::errno::ENOSYS);
        assert_eq!(PosixErrno::from_code(crate::modules::posix_consts::errno::EEXIST), PosixErrno::AlreadyExists);
    }

    #[cfg(feature = "vfs")]
    #[test_case]
    fn seek_whence_numeric_values_are_posix_like() {
        assert_eq!(SeekWhence::Set.as_raw(), crate::modules::posix_consts::fs::SEEK_SET);
        assert_eq!(SeekWhence::Cur.as_raw(), crate::modules::posix_consts::fs::SEEK_CUR);
        assert_eq!(SeekWhence::End.as_raw(), crate::modules::posix_consts::fs::SEEK_END);
        assert_eq!(SeekWhence::from_raw(crate::modules::posix_consts::fs::SEEK_SET), Some(SeekWhence::Set));
        assert_eq!(SeekWhence::from_raw(crate::modules::posix_consts::fs::SEEK_CUR), Some(SeekWhence::Cur));
        assert_eq!(SeekWhence::from_raw(crate::modules::posix_consts::fs::SEEK_END), Some(SeekWhence::End));
        assert_eq!(SeekWhence::from_raw(77), None);
    }

    #[test_case]
    #[cfg(feature = "posix_time")]
    fn time_apis_are_consistent() {
        let mono = clock_gettime(PosixClockId::Monotonic);
        let real = clock_gettime(PosixClockId::Realtime);
        assert!(mono.sec >= 0);
        assert!(real.sec >= 0);

        assert_eq!(PosixClockId::Realtime.as_raw(), crate::modules::posix_consts::time::CLOCK_REALTIME);
        assert_eq!(PosixClockId::Monotonic.as_raw(), crate::modules::posix_consts::time::CLOCK_MONOTONIC);
        assert_eq!(
            PosixClockId::from_raw(crate::modules::posix_consts::time::CLOCK_REALTIME),
            Some(PosixClockId::Realtime)
        );
        assert_eq!(PosixClockId::from_raw(-123), None);
        assert_eq!(
            clock_gettime_raw(crate::modules::posix_consts::time::CLOCK_MONOTONIC)
                .expect("clock_gettime_raw")
                .sec
                >= 0,
            true
        );
        assert!(clock_gettime64(PosixClockId::Monotonic).sec >= 0);

        let res = clock_getres(PosixClockId::Monotonic);
        assert_eq!(res.sec, 0);
        assert!(res.nsec > 0);
        assert!(res.nsec <= 1_000_000_000);
        assert_eq!(
            clock_getres_raw(crate::modules::posix_consts::time::CLOCK_REALTIME)
                .expect("clock_getres_raw")
                .sec,
            0
        );
        assert_eq!(clock_getres_raw(99_999), Err(PosixErrno::Invalid));

        let now_rt = clock_gettime(PosixClockId::Realtime);
        clock_settime(PosixClockId::Realtime, now_rt).expect("clock_settime realtime");
        assert_eq!(clock_settime(PosixClockId::Monotonic, now_rt), Err(PosixErrno::Invalid));
        clock_settime_raw(crate::modules::posix_consts::time::CLOCK_REALTIME, now_rt)
            .expect("clock_settime_raw realtime");
        settimeofday(gettimeofday()).expect("settimeofday");

        let tv = gettimeofday();
        assert!(tv.sec >= 0);
        assert!(tv.usec >= 0);
        assert!(tv.usec < 1_000_000);
        assert!(
            timespec_get(crate::modules::posix_consts::time::TIME_UTC)
                .expect("timespec_get")
                .sec
                >= 0
        );
        assert_eq!(
            timespec_getres(crate::modules::posix_consts::time::TIME_UTC)
                .expect("timespec_getres")
                .sec,
            0
        );
        assert_eq!(timespec_get(0), Err(PosixErrno::Invalid));

        let t = time_now();
        assert!(t >= 0);

        nanosleep(PosixTimespec { sec: 0, nsec: 0 }).expect("nanosleep");
        assert_eq!(
            nanosleep_with_rem(PosixTimespec { sec: 0, nsec: 0 }).expect("nanosleep rem"),
            PosixTimespec { sec: 0, nsec: 0 }
        );
        clock_nanosleep(PosixClockId::Monotonic, 0, PosixTimespec { sec: 0, nsec: 0 })
            .expect("clock_nanosleep rel");
        clock_nanosleep_raw(
            crate::modules::posix_consts::time::CLOCK_MONOTONIC,
            crate::modules::posix_consts::time::TIMER_ABSTIME,
            mono,
        )
        .expect("clock_nanosleep abstime");
        assert_eq!(
            clock_nanosleep_raw(777, 0, PosixTimespec { sec: 0, nsec: 0 }),
            Err(PosixErrno::Invalid)
        );
        usleep(0).expect("usleep");
        sleep(0).expect("sleep");
    }

    #[test_case]
    #[cfg(feature = "posix_process")]
    fn process_numeric_constants_are_posix_like() {
        assert_eq!(crate::modules::posix_consts::process::SIGKILL, 9);
        assert_eq!(crate::modules::posix_consts::process::SIGTERM, 15);
        assert_eq!(crate::modules::posix_consts::process::WNOHANG, 1);
        assert_eq!(crate::modules::posix_consts::process::WUNTRACED, 2);
        assert_eq!(crate::modules::posix_consts::process::WSTOPPED, 2);
        assert_eq!(crate::modules::posix_consts::process::WEXITED, 4);
        assert_eq!(crate::modules::posix_consts::process::WCONTINUED, 8);
        assert_eq!(crate::modules::posix_consts::process::WNOWAIT, 0x0100_0000);
        assert_eq!(crate::modules::posix_consts::process::P_ALL, 0);
        assert_eq!(crate::modules::posix_consts::process::P_PID, 1);
        assert_eq!(crate::modules::posix_consts::process::P_PGID, 2);
        assert_eq!(crate::modules::posix_consts::process::SCHED_OTHER, 0);
        assert_eq!(crate::modules::posix_consts::process::SCHED_FIFO, 1);
        assert_eq!(crate::modules::posix_consts::process::SCHED_RR, 2);

        let tid = process::gettid();
        let pid = process::getpid();
        assert!(tid <= usize::MAX);
        assert!(pid <= usize::MAX);
        assert_eq!(process::getuid(), 0);
        assert_eq!(process::geteuid(), 0);
        assert_eq!(process::getgid(), 0);
        assert_eq!(process::getegid(), 0);
        let old_umask = process::umask(0o027);
        assert_eq!(old_umask, 0o022);
        assert_eq!(process::current_umask(), 0o027);
        let mut groups = [0u32; 4];
        process::setgroups(&[0, 10]).expect("setgroups");
        let gcount = process::getgroups(&mut groups).expect("getgroups");
        assert_eq!(gcount, 2);
        assert_eq!(groups[0], 0);
        assert_eq!(groups[1], 10);
        process::initgroups(7).expect("initgroups");
        let gcount2 = process::getgroups(&mut groups).expect("getgroups2");
        assert_eq!(gcount2, 1);
        assert_eq!(groups[0], 7);
        process::setenv("POSIX_MODE", "strict", true).expect("setenv");
        assert_eq!(process::getenv("POSIX_MODE").expect("getenv"), "strict");
        process::setenv("POSIX_MODE", "soft", false).expect("setenv no overwrite");
        assert_eq!(process::getenv("POSIX_MODE").expect("getenv no overwrite"), "strict");
        process::setenv("POSIX_MODE", "soft", true).expect("setenv overwrite");
        assert_eq!(process::getenv("POSIX_MODE").expect("getenv overwrite"), "soft");
        let env_snapshot = process::environ_snapshot();
        assert!(env_snapshot.iter().any(|(k, v)| k == "POSIX_MODE" && v == "soft"));
        process::unsetenv("POSIX_MODE").expect("unsetenv");
        assert_eq!(process::getenv("POSIX_MODE"), None);
        process::clearenv();
        assert!(process::environ_snapshot().is_empty());
        process::sethostname("hypercore-os").expect("sethostname");
        process::setdomainname("kernel.local").expect("setdomainname");
        let mut host = [0u8; 64];
        let mut domain = [0u8; 64];
        let hlen = process::gethostname(&mut host).expect("gethostname");
        let dlen = process::getdomainname(&mut domain).expect("getdomainname");
        assert_eq!(&host[..hlen], b"hypercore-os");
        assert_eq!(&domain[..dlen], b"kernel.local");
        assert_eq!(process::getresuid(), (0, 0, 0));
        assert_eq!(process::getresgid(), (0, 0, 0));
        assert_eq!(process::setuid(1), Err(PosixErrno::PermissionDenied));
        assert_eq!(process::seteuid(1), Err(PosixErrno::PermissionDenied));
        assert_eq!(process::setgid(1), Err(PosixErrno::PermissionDenied));
        assert_eq!(process::setegid(1), Err(PosixErrno::PermissionDenied));

        assert_eq!(process::getpgrp(), pid);
        assert_eq!(process::getpgid(0).expect("getpgid self"), pid);
        assert!(process::wait_exited(process::encode_wait_exit_status(7)));
        assert_eq!(process::wait_exit_code(process::encode_wait_exit_status(7)), 7);
        assert!(process::wait_signaled(process::encode_wait_signal_status(9)));
        assert_eq!(process::wait_term_signal(process::encode_wait_signal_status(9)), 9);

        #[cfg(not(feature = "process_abstraction"))]
        assert_eq!(process::fork(), Err(PosixErrno::NotSupported));
        #[cfg(feature = "process_abstraction")]
        {
            let r = process::fork();
            assert!(
                matches!(r, Ok(_)) || r == Err(PosixErrno::Invalid) || r == Err(PosixErrno::Again),
                "unexpected fork result: {:?}",
                r
            );
        }
        assert_eq!(
            process::fork_from_image(b"", b"", 10, 0, 0, 0),
            Err(PosixErrno::Invalid)
        );
        assert_eq!(
            process::execve("/bin/app", &[], &[]),
            Err({
                #[cfg(all(feature = "process_abstraction", feature = "vfs"))]
                {
                    PosixErrno::BadFileDescriptor
                }
                #[cfg(not(all(feature = "process_abstraction", feature = "vfs")))]
                {
                    PosixErrno::NotSupported
                }
            })
        );
        assert_eq!(process::posix_spawn_from_image(b"", b"", 10, 0, 0, 0), Err(PosixErrno::Invalid));
        assert_eq!(
            process::waitid(crate::modules::posix_consts::process::P_ALL, 0, crate::modules::posix_consts::process::WNOHANG)
                .expect("waitid nohang"),
            None
        );
        assert_eq!(
            process::waitid(
                crate::modules::posix_consts::process::P_ALL,
                0,
                crate::modules::posix_consts::process::WNOHANG
                    | crate::modules::posix_consts::process::WEXITED
                    | crate::modules::posix_consts::process::WNOWAIT,
            )
            .expect("waitid nohang extended"),
            None
        );
        let usage = process::getrusage(crate::modules::posix_consts::process::RUSAGE_SELF).expect("getrusage self");
        assert!(usage.ru_utime_ticks <= u64::MAX);
        let (_cpu, _node) = process::getcpu().expect("getcpu");
        if pid != 0 {
            let pidfd = process::pidfd_open(pid).expect("pidfd open");
            assert_eq!(process::pidfd_get_pid(pidfd).expect("pidfd get"), pid);
            process::pidfd_send_signal(pidfd, 0).expect("pidfd signal 0");
            process::pidfd_close(pidfd).expect("pidfd close");
            assert_eq!(process::parent_of(pid).expect("parent_of self"), process::getppid());
            assert_eq!(process::getpgid_of(pid).expect("getpgid_of self"), process::getpgid(0).expect("getpgid self"));
            assert_eq!(process::getsid(0).expect("getsid self"), pid);
            let setsid = process::setsid();
            assert!(
                setsid == Ok(pid) || setsid == Err(PosixErrno::PermissionDenied),
                "unexpected setsid result: {:?}",
                setsid
            );
            process::kill(pid, 0).expect("kill(pid,0)");
            process::setpgid(0, pid).expect("setpgid self");
            assert_eq!(process::getpriority(pid).expect("getpriority"), 0);
            process::setpriority(pid, 5).expect("setpriority");
            assert_eq!(process::getpriority(pid).expect("getpriority after"), 5);
            assert_eq!(process::nice(30).expect("nice"), 19);
            process::setrlimit(process::RLIMIT_NOFILE, 64, 128).expect("setrlimit nofile");
            assert_eq!(process::getrlimit(process::RLIMIT_NOFILE).expect("getrlimit nofile"), (64, 128));
            assert_eq!(
                process::prlimit(pid, process::RLIMIT_NOFILE, Some((80, 160))).expect("prlimit"),
                (64, 128)
            );
            assert_eq!(process::getrlimit(process::RLIMIT_NOFILE).expect("getrlimit nofile new"), (80, 160));
            process::sched_setscheduler(pid, crate::modules::posix_consts::process::SCHED_RR, 7)
                .expect("sched_setscheduler");
            assert_eq!(
                process::sched_getscheduler(pid).expect("sched_getscheduler"),
                crate::modules::posix_consts::process::SCHED_RR
            );
            assert_eq!(process::sched_getparam(pid).expect("sched_getparam"), 7);
            process::sched_setparam(pid, 3).expect("sched_setparam");
            assert_eq!(process::sched_getparam(pid).expect("sched_getparam after"), 3);
            assert_eq!(process::waitpid(pid, true).expect("waitpid nohang"), None);
            assert_eq!(
                process::waitpid_options(pid, crate::modules::posix_consts::process::WNOHANG)
                    .expect("waitpid options nohang"),
                None
            );
            assert_eq!(
                process::waitpid_options(
                    pid,
                    crate::modules::posix_consts::process::WNOHANG
                        | crate::modules::posix_consts::process::WUNTRACED
                        | crate::modules::posix_consts::process::WCONTINUED,
                )
                .expect("waitpid options extended"),
                None
            );
            assert_eq!(process::waitpid_status(pid, true).expect("waitpid status nohang"), None);
            assert_eq!(
                process::waitpid_status_options(pid, crate::modules::posix_consts::process::WNOHANG)
                    .expect("waitpid status options nohang"),
                None
            );
            assert_eq!(
                process::waitpid_status_options(
                    pid,
                    crate::modules::posix_consts::process::WNOHANG
                        | crate::modules::posix_consts::process::WUNTRACED,
                )
                .expect("waitpid status options extended"),
                None
            );
            assert_eq!(
                process::wait4(pid, crate::modules::posix_consts::process::WNOHANG)
                    .expect("wait4 nohang"),
                None
            );
            assert_eq!(process::wait(true).expect("wait nohang"), None);
            assert_eq!(process::wait_status(true).expect("wait status nohang"), None);
            assert_eq!(process::wait_any_status(true).expect("wait_any nohang"), None);
            assert_eq!(
                process::wait3(crate::modules::posix_consts::process::WNOHANG)
                    .expect("wait3 nohang"),
                None
            );
            assert_eq!(process::pending_exit_status_count(), 0);
            assert_eq!(process::get_cached_exit_status(pid), None);
        }
    }

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

    #[test_case]
    #[cfg(feature = "posix_pipe")]
    fn posix_pipe_roundtrip_and_nonblock_flow() {
        let (rfd, wfd) = pipe::pipe2(false).expect("pipe2");
        let wfd2 = pipe::dup(wfd).expect("dup writer");
        let wfd3 = pipe::dup2(wfd2, 60001).expect("dup2 writer");
        assert_eq!(wfd3, 60001);
        assert_eq!(pipe::pending_readable(rfd).expect("pending empty"), 0);
        assert_eq!(pipe::poll(rfd, crate::modules::posix_consts::net::POLLIN).expect("poll in empty"), 0);

        let wrote = pipe::write(wfd, b"pipe-data").expect("pipe write");
        assert_eq!(wrote, 9);
        assert_eq!(pipe::poll(rfd, crate::modules::posix_consts::net::POLLIN).expect("poll in ready"), crate::modules::posix_consts::net::POLLIN);
        assert_eq!(pipe::pending_readable(rfd).expect("pending non-empty"), 1);

        let mut out = [0u8; 16];
        let got = pipe::read(rfd, &mut out).expect("pipe read");
        assert_eq!(got, 9);
        assert_eq!(&out[..got], b"pipe-data");

        pipe::set_nonblock(rfd, true).expect("set nonblock");
        assert_eq!(pipe::read(rfd, &mut out), Err(super::PosixErrno::Again));

        pipe::close(wfd).expect("close writer");
        pipe::close(wfd2).expect("close dup writer");
        pipe::close(wfd3).expect("close dup2 writer");
        let eof = pipe::read(rfd, &mut out).expect("read eof");
        assert_eq!(eof, 0);
        pipe::close(rfd).expect("close reader");
    }

    #[test_case]
    #[cfg(all(feature = "posix_pipe", feature = "posix_fs"))]
    fn posix_pipe2_nonblock_is_visible_via_fcntl_flags() {
        let (rfd, wfd) = pipe::pipe2(true).expect("pipe2 nonblock");
        let expected = 0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32;
        assert_eq!(fs::fcntl_get_status_flags(rfd).expect("rfd flags"), expected);
        assert_eq!(fs::fcntl_get_status_flags(wfd).expect("wfd flags"), expected);

        let mut out = [0u8; 4];
        assert_eq!(pipe::read(rfd, &mut out), Err(super::PosixErrno::Again));

        pipe::close(wfd).expect("close writer");
        pipe::close(rfd).expect("close reader");
    }

    #[test_case]
    #[cfg(all(feature = "posix_io", feature = "posix_fs"))]
    fn posix_eventfd_nonblock_is_visible_via_fcntl_and_returns_again() {
        let fd = io::eventfd_create_errno(0, crate::modules::posix_consts::net::O_NONBLOCK)
            .expect("eventfd nonblock");
        let expected = 0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32;
        assert_eq!(fs::fcntl_get_status_flags(fd).expect("eventfd flags"), expected);

        let mut out = [0u8; 8];
        assert_eq!(fs::read(fd, &mut out), Err(super::PosixErrno::Again));
        fs::close(fd).expect("close eventfd");
    }

    #[test_case]
    #[cfg(all(feature = "posix_signal", feature = "posix_fs", feature = "vfs"))]
    fn posix_signalfd_nonblock_is_visible_via_fcntl_and_returns_again() {
        let fd = signal::signalfd_create_errno(0, crate::modules::posix_consts::net::O_NONBLOCK)
            .expect("signalfd nonblock");
        let expected = 0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32;
        assert_eq!(fs::fcntl_get_status_flags(fd).expect("signalfd flags"), expected);

        let mut out = [0u8; 128];
        assert_eq!(fs::read(fd, &mut out), Err(super::PosixErrno::Again));
        fs::close(fd).expect("close signalfd");
    }

    #[test_case]
    #[cfg(all(feature = "posix_io", feature = "posix_pipe", feature = "posix_time"))]
    fn posix_io_mixed_poll_and_select_work() {
        let (rfd, wfd) = pipe::pipe().expect("pipe");
        let wrote = pipe::write(wfd, b"x").expect("write one");
        assert_eq!(wrote, 1);

        let mut pfds = [io::PosixPollFd::new(rfd, crate::modules::posix_consts::net::POLLIN)];
        let ready = io::poll_mixed(&mut pfds, 0).expect("poll mixed");
        assert_eq!(ready, 1);
        assert_ne!(pfds[0].revents & crate::modules::posix_consts::net::POLLIN, 0);

        let mut pfds_ts = [io::PosixPollFd::new(rfd, crate::modules::posix_consts::net::POLLIN)];
        let ready_ts = io::poll_mixed_timespec(
            &mut pfds_ts,
            PosixTimespec { sec: 0, nsec: 1_000_000 },
        )
        .expect("poll mixed timespec");
        assert_eq!(ready_ts, 1);

        let sel = io::select_mixed(&[rfd], &[], &[], 0).expect("select mixed");
        assert_eq!(sel.readable.len(), 1);
        assert_eq!(sel.readable[0], rfd);

        let sel_ts = io::select_mixed_timespec(
            &[rfd],
            &[],
            &[],
            PosixTimespec { sec: 0, nsec: 1_000_000 },
        )
        .expect("select mixed timespec");
        assert_eq!(sel_ts.readable.len(), 1);
        assert_eq!(sel_ts.readable[0], rfd);

        let mut out = [0u8; 4];
        let _ = pipe::read(rfd, &mut out).expect("drain");
        pipe::close(wfd).expect("close wfd");
        pipe::close(rfd).expect("close rfd");
    }

    #[cfg(feature = "vfs")]
    #[test_case]
    fn posix_mman_file_mapping_management_works() {
        let fs_id = fs::mount_ramfs("/posix_mman").expect("mount");
        let fd = fs::open(fs_id, "/posix_mman/a.bin", true).expect("open create");
        fs::write(fd, b"abcdef").expect("write");
        fs::close(fd).expect("close");

        let map_id = mman::mmap_file(
            fs_id,
            "/posix_mman/a.bin",
            0,
            6,
            crate::modules::posix_consts::mman::PROT_READ | crate::modules::posix_consts::mman::PROT_WRITE,
            crate::modules::posix_consts::mman::MAP_SHARED,
        )
        .expect("mmap_file");

        assert_eq!(mman::get_flags(map_id).expect("flags"), crate::modules::posix_consts::mman::MAP_SHARED);
        assert!(mman::mincore(map_id).expect("mincore"));

        let mut buf = [0u8; 8];
        let rd = mman::mmap_read(map_id, &mut buf, 0).expect("mmap_read");
        assert_eq!(rd, 6);
        assert_eq!(&buf[..rd], b"abcdef");
        let wr = mman::mmap_write(map_id, b"XYZ", 0).expect("mmap_write");
        assert_eq!(wr, 3);

        mman::mprotect(map_id, crate::modules::posix_consts::mman::PROT_READ).expect("mprotect");
        assert_eq!(mman::get_prot(map_id).expect("prot"), crate::modules::posix_consts::mman::PROT_READ);
        assert!(mman::can_read(map_id).expect("can read"));
        assert!(!mman::can_write(map_id).expect("can write"));
        assert!(!mman::can_exec(map_id).expect("can exec"));
        assert_eq!(mman::mmap_write(map_id, b"Q", 0), Err(super::PosixErrno::PermissionDenied));

        mman::mlock(map_id).expect("mlock");
        assert!(mman::is_locked(map_id).expect("is_locked"));
        mman::munlock(map_id).expect("munlock");
        assert!(!mman::is_locked(map_id).expect("is_locked after"));

        mman::madvise(map_id, crate::modules::posix_consts::mman::MADV_SEQUENTIAL).expect("madvise");
        mman::msync_flags(map_id, crate::modules::posix_consts::mman::MS_SYNC).expect("msync flags");
        assert_eq!(mman::mapped_len(map_id).expect("mapped len"), 6);
        mman::mremap(map_id, 4).expect("mremap");
        assert_eq!(mman::mapped_len(map_id).expect("mapped len after"), 4);
        mman::msync_range(map_id, 0, 4).expect("msync_range");
        mman::msync(map_id).expect("msync");

        let anon_id = mman::mmap_anonymous(
            8,
            crate::modules::posix_consts::mman::PROT_READ | crate::modules::posix_consts::mman::PROT_WRITE,
            crate::modules::posix_consts::mman::MAP_PRIVATE,
        )
        .expect("mmap anonymous");
        assert_eq!(
            mman::get_flags(anon_id).expect("anon flags") & crate::modules::posix_consts::mman::MAP_ANONYMOUS,
            crate::modules::posix_consts::mman::MAP_ANONYMOUS
        );
        let wrote_anon = mman::mmap_write(anon_id, b"anon", 0).expect("anon write");
        assert_eq!(wrote_anon, 4);
        let mut anon_buf = [0u8; 8];
        let read_anon = mman::mmap_read(anon_id, &mut anon_buf, 0).expect("anon read");
        assert_eq!(read_anon, 8);
        assert_eq!(&anon_buf[..4], b"anon");

        mman::mlockall(
            crate::modules::posix_consts::mman::MCL_CURRENT
                | crate::modules::posix_consts::mman::MCL_FUTURE,
        )
        .expect("mlockall");
        assert_ne!(mman::mlockall_mode(), 0);
        assert!(mman::is_locked(anon_id).expect("anon locked"));
        mman::munlockall();
        assert_eq!(mman::mlockall_mode(), 0);
        assert!(!mman::is_locked(anon_id).expect("anon unlocked"));

        mman::munmap(anon_id).expect("munmap anon");
        mman::munmap(map_id).expect("munmap");

        fs::unlink(fs_id, "/posix_mman/a.bin").expect("unlink");
        fs::unmount(fs_id).expect("unmount");
    }

    #[test_case]
    fn posix_fs_append_flag_forces_writes_to_end() {
        let fs_id = fs::mount_ramfs("/posix_append").expect("mount");
        let fd = fs::open(fs_id, "/posix_append/log.txt", true).expect("open create");
        fs::write(fd, b"abc").expect("write initial");
        fs::lseek(fd, 0, fs::SeekWhence::Set).expect("rewind");
        fs::fcntl_set_status_flags(fd, crate::modules::posix_consts::fs::O_APPEND as u32)
            .expect("set append");
        fs::write(fd, b"Z").expect("append write");
        fs::lseek(fd, 0, fs::SeekWhence::Set).expect("rewind read");
        let mut out = [0u8; 8];
        let n = fs::read(fd, &mut out).expect("read back");
        assert_eq!(&out[..n], b"abcZ");
        fs::fcntl_set_status_flags(
            fd,
            (crate::modules::posix_consts::fs::O_APPEND as u32) | 0xFFFF_0000,
        )
        .expect("set masked flags");
        assert_eq!(
            fs::fcntl_get_status_flags(fd).expect("get masked flags"),
            crate::modules::posix_consts::fs::O_APPEND as u32
        );
        fs::close(fd).expect("close");
        fs::unlink(fs_id, "/posix_append/log.txt").expect("unlink");
        fs::unmount(fs_id).expect("unmount");
    }

    #[test_case]
    #[cfg(all(feature = "posix_fs", feature = "posix_pipe"))]
    fn posix_fcntl_nonblock_updates_pipe_runtime_behavior() {
        let (rfd, wfd) = pipe::pipe2(false).expect("pipe2");
        assert_eq!(fs::fcntl_get_status_flags(rfd).expect("initial flags"), 0x2);

        fs::fcntl_set_status_flags(
            rfd,
            0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32,
        )
        .expect("set nonblock");
        assert_eq!(
            fs::fcntl_get_status_flags(rfd).expect("flags after nonblock"),
            0x2 | crate::modules::posix_consts::net::O_NONBLOCK as u32
        );

        let mut out = [0u8; 4];
        assert_eq!(pipe::read(rfd, &mut out), Err(super::PosixErrno::Again));

        fs::fcntl_set_status_flags(rfd, 0x2).expect("clear nonblock");
        assert_eq!(fs::fcntl_get_status_flags(rfd).expect("flags after clear"), 0x2);

        pipe::close(wfd).expect("close writer");
        let eof = pipe::read(rfd, &mut out).expect("read eof after clear");
        assert_eq!(eof, 0);
        pipe::close(rfd).expect("close reader");
    }

    #[test_case]
    #[cfg(all(feature = "posix_thread", feature = "posix_ipc"))]
    fn thread_library_mutex_and_condvar_work() {
        let me = thread::pthread_self();
        assert!(thread::pthread_equal(me, me));
        thread::sched_yield();

        let mutex = thread::PthreadMutex::new(0xAA11);
        let cond = thread::PthreadCondvar::new(0xAA22);

        assert!(mutex.try_lock().expect("try_lock") );
        assert!(!mutex.try_lock().expect("try_lock contended"));
        mutex.unlock().expect("unlock");

        mutex.lock().expect("lock");
        cond.wait(&mutex).expect("cond wait");
        assert!(ipc::futex_pending_waiters(cond.key()) >= 1);
        let woke = cond.signal().expect("cond signal");
        assert!(woke >= 1);
        mutex.unlock().expect("unlock after cond wait");
    }

    #[test_case]
    #[cfg(feature = "posix_thread")]
    fn thread_library_semaphore_and_rwlock_work() {
        let sem = thread::PosixSemaphore::new(1, 0xBB11);
        assert!(sem.try_wait().expect("sem try_wait first"));
        assert!(!sem.try_wait().expect("sem try_wait empty"));
        sem.post().expect("sem post");
        sem.wait().expect("sem wait");

        let rw = thread::PthreadRwLock::new(0xBB22);
        rw.rdlock().expect("rw rdlock");
        rw.unlock().expect("rw runlock");
        rw.wrlock().expect("rw wrlock");
        rw.unlock().expect("rw wunlock");
    }

    #[test_case]
    #[cfg(feature = "posix_thread")]
    fn thread_lifecycle_helpers_behave_consistently() {
        let me = thread::pthread_self();
        if me != 0 {
            assert!(thread::thread_exists(me));
            assert_eq!(thread::pthread_join(me, 2), Err(PosixErrno::Invalid));
            thread::pthread_detach(me).expect("detach self");
            assert_eq!(thread::pthread_join(me, 2), Err(PosixErrno::Invalid));
        }

        let synthetic = me.saturating_add(10_000);
        thread::pthread_register(synthetic).expect("register synthetic");
        assert!(thread::thread_exists(synthetic));
        thread::pthread_detach(synthetic).expect("detach synthetic");

        assert_eq!(thread::pthread_join(usize::MAX, 0), Err(PosixErrno::Invalid));
    }

    #[test_case]
    #[cfg(feature = "posix_thread")]
    fn thread_create_from_image_validates_inputs() {
        assert_eq!(thread::pthread_register(0), Err(PosixErrno::Invalid));
        let result = thread::pthread_create_from_image(b"", b"", 10, 0, 0, 0);
        #[cfg(feature = "process_abstraction")]
        assert_eq!(result, Err(PosixErrno::Invalid));
        #[cfg(not(feature = "process_abstraction"))]
        assert_eq!(result, Err(PosixErrno::NotSupported));
    }
}

