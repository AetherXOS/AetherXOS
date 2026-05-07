use super::*;
use crate::modules::posix::process;

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
    process::sethostname("aethercore-os").expect("sethostname");
    process::setdomainname("kernel.local").expect("setdomainname");
    let mut host = [0u8; 64];
    let mut domain = [0u8; 64];
    let hlen = process::gethostname(&mut host).expect("gethostname");
    let dlen = process::getdomainname(&mut domain).expect("getdomainname");
    assert_eq!(&host[..hlen], b"aethercore-os");
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
