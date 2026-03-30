use super::*;

#[test_case]
fn epoll_create_rejects_zero_size() {
    assert_eq!(
        sys_linux_epoll_create(0),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn epoll_validate_maxevents_rejects_out_of_policy_limit() {
    crate::config::KernelConfig::set_network_epoll_max_events(Some(1));
    assert_eq!(
        sys_linux_epoll_pwait(1, 0, 2, 0, 0, 0),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
    crate::config::KernelConfig::set_network_epoll_max_events(None);
}

#[test_case]
fn invalid_user_pointers_return_efault_for_helpers() {
    let bad_ptr = 0x1usize;
    assert_eq!(
        parse_sigmask(bad_ptr, core::mem::size_of::<u64>()),
        Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
    );
    assert_eq!(
        timeout_ptr_to_retries(bad_ptr),
        Err(linux_errno(crate::modules::posix_consts::errno::EFAULT))
    );
}

#[test_case]
fn epoll_create1_rejects_unknown_flags() {
    assert_eq!(
        sys_linux_epoll_create1(0x4000),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn epoll_ctl_add_mod_del_lifecycle_succeeds() {
    let epfd = sys_linux_epoll_create1(0);
    assert!(epfd > 0);

    let event = LinuxEpollEventCompat {
        events: crate::modules::posix_consts::net::EPOLLIN,
        data: 0,
    };

    assert_eq!(
        sys_linux_epoll_ctl(
            epfd,
            crate::modules::posix_consts::net::EPOLL_CTL_ADD as usize,
            7,
            (&event as *const LinuxEpollEventCompat) as usize,
        ),
        0
    );
    assert_eq!(
        sys_linux_epoll_ctl(
            epfd,
            crate::modules::posix_consts::net::EPOLL_CTL_MOD as usize,
            7,
            (&event as *const LinuxEpollEventCompat) as usize,
        ),
        0
    );
    assert_eq!(
        sys_linux_epoll_ctl(
            epfd,
            crate::modules::posix_consts::net::EPOLL_CTL_DEL as usize,
            7,
            0,
        ),
        0
    );
}

#[test_case]
fn epoll_pwait_rejects_invalid_sigset_size() {
    let epfd = sys_linux_epoll_create1(0);
    let mask = 0u64;
    assert_eq!(
        sys_linux_epoll_pwait(epfd, 0, 1, 0, (&mask as *const u64) as usize, 4),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn epoll_pwait_sigmask_sanitizes_unblockable_signals() {
    let kill_bit =
        1u64 << ((crate::modules::posix_consts::signal::SIGKILL as u64).saturating_sub(1));
    let stop_bit =
        1u64 << ((crate::modules::posix_consts::signal::SIGSTOP as u64).saturating_sub(1));
    let keep_bit = 1u64 << 5;
    let mask = kill_bit | stop_bit | keep_bit;

    assert_eq!(
        parse_sigmask((&mask as *const u64) as usize, core::mem::size_of::<u64>()),
        Ok(Some(keep_bit))
    );
}

#[test_case]
fn epoll_pwait2_rejects_negative_timeout_nsec() {
    let epfd = sys_linux_epoll_create1(0);
    let ts = LinuxTimespecCompat {
        tv_sec: 0,
        tv_nsec: -1,
    };
    assert_eq!(
        sys_linux_epoll_pwait2(
            epfd,
            0,
            1,
            (&ts as *const LinuxTimespecCompat) as usize,
            0,
            0,
        ),
        linux_errno(crate::modules::posix_consts::errno::EINVAL)
    );
}

#[test_case]
fn epoll_pwait_empty_registry_returns_zero() {
    let epfd = sys_linux_epoll_create1(0);
    let mut out = [0u8; core::mem::size_of::<LinuxEpollEventCompat>()];
    assert_eq!(
        sys_linux_epoll_pwait(epfd, out.as_mut_ptr() as usize, 1, 0, 0, 0),
        0
    );
}

#[test_case]
fn timeout_ns_to_retries_rounds_up_by_tick() {
    let timeout_ns = 7_500_000u128;
    let tick_ns = core::cmp::max(crate::generated_consts::TIME_SLICE_NS as u128, 1u128);
    let expected = ((timeout_ns + tick_ns - 1) / tick_ns) as usize;
    assert_eq!(timeout_ns_to_retries(timeout_ns), expected);
}

#[test_case]
fn timeout_arg_max_maps_to_blocking_retries() {
    assert_eq!(
        timeout_arg_to_retries(usize::MAX),
        crate::config::KernelConfig::libnet_posix_blocking_recv_retries()
    );
}
