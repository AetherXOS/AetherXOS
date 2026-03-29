use super::*;
use crate::kernel::syscalls::linux_nr;

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
struct LinuxEpollEventCompat {
    events: u32,
    data: u64,
}

fn linux_shim_call(
    syscall_id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> usize {
    let mut frame = crate::kernel::syscalls::SyscallFrame::default();
    super::super::linux_shim::sys_linux_shim(
        syscall_id,
        arg1,
        arg2,
        arg3,
        arg4,
        arg5,
        0,
        &mut frame as *mut _,
    )
    .unwrap_or_else(|| linux_errno(crate::modules::posix_consts::errno::ENOSYS))
}

fn is_linux_error(ret: usize) -> bool {
    ret >= linux_errno(1)
}

#[test_case]
fn p2_python_async_runtime_timerfd_poll_signalfd_stress() {
    for _ in 0..32usize {
        let tfd = sys_linux_timerfd_create(
            crate::modules::posix_consts::time::CLOCK_MONOTONIC as usize,
            0,
        );
        if is_linux_error(tfd) {
            return;
        }

        let spec = LinuxItimerspecCompat {
            it_interval: LinuxTimespecCompat {
                tv_sec: 0,
                tv_nsec: 1_000_000,
            },
            it_value: LinuxTimespecCompat {
                tv_sec: 0,
                tv_nsec: 1_000_000,
            },
        };
        assert_eq!(
            sys_linux_timerfd_settime(tfd, 0, (&spec as *const LinuxItimerspecCompat) as usize, 0),
            0
        );

        let mut curr = LinuxItimerspecCompat::default();
        assert_eq!(
            sys_linux_timerfd_gettime(tfd, (&mut curr as *mut LinuxItimerspecCompat) as usize),
            0
        );

        let _ = sys_linux_poll(0, 0, 0);
        let _ = sys_linux_select(0, 0, 0, 0, 0);

        let mask = 0u64;
        let sfd = sys_linux_signalfd4(
            usize::MAX,
            (&mask as *const u64) as usize,
            core::mem::size_of::<u64>(),
            0,
        );
        if !is_linux_error(sfd) {
            let _ = linux_shim_call(
                linux_nr::CLOSE,
                sfd,
                0,
                0,
                0,
                0,
            );
        }

        let _ = linux_shim_call(
            linux_nr::CLOSE,
            tfd,
            0,
            0,
            0,
            0,
        );
    }
}

#[test_case]
fn p2_flutter_gui_event_loop_like_inotify_eventfd_timerfd_soak() {
    for _ in 0..24usize {
        let ifd = sys_linux_inotify_init1(0);
        if is_linux_error(ifd) {
            return;
        }

        let efd = sys_linux_eventfd2(0, 0);
        if is_linux_error(efd) {
            let _ = linux_shim_call(
                linux_nr::CLOSE,
                ifd,
                0,
                0,
                0,
                0,
            );
            return;
        }

        let tfd = sys_linux_timerfd_create(
            crate::modules::posix_consts::time::CLOCK_MONOTONIC as usize,
            0,
        );
        if is_linux_error(tfd) {
            let _ = linux_shim_call(
                linux_nr::CLOSE,
                ifd,
                0,
                0,
                0,
                0,
            );
            let _ = linux_shim_call(
                linux_nr::CLOSE,
                efd,
                0,
                0,
                0,
                0,
            );
            return;
        }

        let _ = sys_linux_poll(0, 0, 0);

        let _ = linux_shim_call(
            linux_nr::CLOSE,
            ifd,
            0,
            0,
            0,
            0,
        );
        let _ = linux_shim_call(
            linux_nr::CLOSE,
            efd,
            0,
            0,
            0,
            0,
        );
        let _ = linux_shim_call(
            linux_nr::CLOSE,
            tfd,
            0,
            0,
            0,
            0,
        );
    }
}

#[test_case]
fn p2_mixed_poll_select_epoll_timerfd_churn() {
    #[cfg(feature = "posix_net")]
    {
        let epfd = linux_shim_call(
            crate::kernel::syscalls::syscalls_consts::linux_numbers::EPOLL_CREATE1,
            0,
            0,
            0,
            0,
            0,
        );
        if is_linux_error(epfd) {
            return;
        }

        for _ in 0..16usize {
            let tfd = linux_shim_call(
                crate::kernel::syscalls::syscalls_consts::linux_numbers::TIMERFD_CREATE,
                crate::modules::posix_consts::time::CLOCK_MONOTONIC as usize,
                0,
                0,
                0,
                0,
            );
            if is_linux_error(tfd) {
                break;
            }

            let spec = LinuxItimerspecCompat {
                it_interval: LinuxTimespecCompat {
                    tv_sec: 0,
                    tv_nsec: 1_000_000,
                },
                it_value: LinuxTimespecCompat {
                    tv_sec: 0,
                    tv_nsec: 1_000_000,
                },
            };
            assert_eq!(
                linux_shim_call(
                    crate::kernel::syscalls::syscalls_consts::linux_numbers::TIMERFD_SETTIME,
                    tfd,
                    0,
                    (&spec as *const LinuxItimerspecCompat) as usize,
                    0,
                    0,
                ),
                0
            );

            let ev = LinuxEpollEventCompat {
                events: crate::modules::posix_consts::net::EPOLLIN,
                data: tfd as u64,
            };
            assert_eq!(
                linux_shim_call(
                    crate::kernel::syscalls::syscalls_consts::linux_numbers::EPOLL_CTL,
                    epfd,
                    crate::modules::posix_consts::net::EPOLL_CTL_ADD as usize,
                    tfd,
                    (&ev as *const LinuxEpollEventCompat) as usize,
                    0,
                ),
                0
            );

            let _ = sys_linux_poll(0, 0, 0);
            let _ = sys_linux_select(0, 0, 0, 0, 0);

            let mut out = [LinuxEpollEventCompat::default(); 4];
            let ready = linux_shim_call(
                crate::kernel::syscalls::syscalls_consts::linux_numbers::EPOLL_WAIT,
                epfd,
                out.as_mut_ptr() as usize,
                out.len(),
                1,
                0,
            );
            assert!(!is_linux_error(ready));

            let _ = linux_shim_call(
                crate::kernel::syscalls::syscalls_consts::linux_numbers::EPOLL_CTL,
                epfd,
                crate::modules::posix_consts::net::EPOLL_CTL_DEL as usize,
                tfd,
                0,
                0,
            );
            let _ = linux_shim_call(
                crate::kernel::syscalls::syscalls_consts::linux_numbers::CLOSE,
                tfd,
                0,
                0,
                0,
                0,
            );
        }

        let _ = linux_shim_call(
            crate::kernel::syscalls::syscalls_consts::linux_numbers::CLOSE,
            epfd,
            0,
            0,
            0,
            0,
        );
    }
}
