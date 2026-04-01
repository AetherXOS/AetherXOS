use super::*;
use crate::modules::linux_compat::poll::helpers::kernel_tick_ns;

#[test_case]
fn retries_from_total_ns_rounds_up() {
    let tick = kernel_tick_ns();
    assert_eq!(retries_from_total_ns(0), 0);
    assert_eq!(retries_from_total_ns(1), 1);
    assert_eq!(retries_from_total_ns(tick), 1);
    assert_eq!(retries_from_total_ns(tick + 1), 2);
}

#[test_case]
fn retries_from_total_ns_caps_without_overflow() {
    assert_eq!(retries_from_total_ns(u128::MAX), MAX_SELECT_RETRIES);
}

#[test_case]
fn parse_optional_sigmask_validates_size() {
    assert_eq!(parse_optional_sigmask(UserPtr::new(0), 0), Ok(None));
    assert_eq!(
        parse_optional_sigmask(UserPtr::new(0x1000), linux::SIGSET_SIZE + 1),
        Err(linux_inval())
    );

    let mut mask: u64 = 0xA5A5_5A5A_F0F0_0F0F;
    let ptr = UserPtr::<u64>::new((&mut mask as *mut u64) as usize);
    assert_eq!(
        parse_optional_sigmask(ptr, linux::SIGSET_SIZE),
        Ok(Some(mask))
    );
}

#[test_case]
fn parse_pselect6_sigmask_null_and_len_validation() {
    assert_eq!(parse_pselect6_sigmask(UserPtr::new(0)), Ok(None));

    let mut sigset_mask: u64 = 0;
    let sigset_ptr = (&mut sigset_mask as *mut u64) as usize;
    let mut arg = LinuxPselect6Sigmask {
        ss_ptr: sigset_ptr as u64,
        ss_len: linux::SIGSET_SIZE + 1,
    };
    let arg_ptr =
        UserPtr::<LinuxPselect6Sigmask>::new((&mut arg as *mut LinuxPselect6Sigmask) as usize);

    assert_eq!(parse_pselect6_sigmask(arg_ptr.cast()), Err(linux_inval()));
}

#[test_case]
fn retries_from_timespec_and_timeval_validate_ranges() {
    let bad_ts = LinuxTimespec {
        tv_sec: 1,
        tv_nsec: NANOS_PER_SECOND as i64,
    };
    let bad_ts_ptr = UserPtr::new((&bad_ts as *const LinuxTimespec) as usize);
    assert_eq!(retries_from_timespec(bad_ts_ptr), Err(linux_inval()));

    let bad_tv = LinuxTimeval {
        tv_sec: 1,
        tv_usec: MICROS_PER_SECOND as i64,
    };
    let bad_tv_ptr = UserPtr::new((&bad_tv as *const LinuxTimeval) as usize);
    assert_eq!(retries_from_timeout(bad_tv_ptr), Err(linux_inval()));
}

#[test_case]
fn syscall_negative_paths_poll_select_epoll() {
    assert_eq!(sys_linux_epoll_create(0), linux_inval());

    let epfd = Fd(3);
    let events_ptr = UserPtr::<LinuxEpollEvent>::new(0);

    #[cfg(feature = "posix_net")]
    {
        assert_eq!(
            sys_linux_epoll_pwait(epfd, events_ptr, 0, 0, UserPtr::new(0), 0),
            linux_inval()
        );
        assert_eq!(
            sys_linux_epoll_pwait(
                epfd,
                events_ptr,
                1,
                0,
                UserPtr::new(0x1000),
                linux::SIGSET_SIZE + 1,
            ),
            linux_inval()
        );

        assert_eq!(
            sys_linux_ppoll(
                UserPtr::new(0),
                MAX_POLL_FDS + 1,
                UserPtr::new(0),
                UserPtr::new(0),
                0,
            ),
            linux_inval()
        );

        assert_eq!(
            sys_linux_pselect6(
                LINUX_FD_SETSIZE + 1,
                UserPtr::new(0),
                UserPtr::new(0),
                UserPtr::new(0),
                UserPtr::new(0),
                UserPtr::new(0),
            ),
            linux_inval()
        );
    }

    #[cfg(not(feature = "posix_net"))]
    {
        assert_eq!(
            sys_linux_epoll_pwait(epfd, events_ptr, 1, 0, UserPtr::new(0), 0),
            linux_nosys()
        );
        assert_eq!(
            sys_linux_ppoll(
                UserPtr::new(0),
                MAX_POLL_FDS + 1,
                UserPtr::new(0),
                UserPtr::new(0),
                0,
            ),
            linux_nosys()
        );
        assert_eq!(
            sys_linux_pselect6(
                LINUX_FD_SETSIZE + 1,
                UserPtr::new(0),
                UserPtr::new(0),
                UserPtr::new(0),
                UserPtr::new(0),
                UserPtr::new(0),
            ),
            linux_nosys()
        );
    }
}

#[test_case]
fn epoll_create1_sets_linux_cloexec_flag() {
    let fd = sys_linux_epoll_create1(crate::modules::posix_consts::net::EPOLL_CLOEXEC as usize)
        as u32;
    assert_eq!(
        crate::modules::linux_compat::fs::io::linux_fd_get_descriptor_flags(fd)
            & crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC,
        crate::modules::linux_compat::fs::io::LINUX_FD_CLOEXEC
    );
}
