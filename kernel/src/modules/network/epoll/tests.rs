use super::*;

#[test_case]
fn epoll_instance_clamps_fd_capacity_to_runtime_limit() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_network_epoll_max_fds_per_instance(Some(2));

    let mut epoll = EpollInstance::new(8);
    assert!(epoll
        .ctl(
            EpollOp::Add,
            3,
            Some(EpollEvent {
                events: EpollEvents::EPOLLIN,
                data: 11,
            }),
        )
        .is_ok());
    assert!(epoll
        .ctl(
            EpollOp::Add,
            4,
            Some(EpollEvent {
                events: EpollEvents::EPOLLIN,
                data: 22,
            }),
        )
        .is_ok());
    assert_eq!(
        epoll.ctl(
            EpollOp::Add,
            5,
            Some(EpollEvent {
                events: EpollEvents::EPOLLIN,
                data: 33,
            }),
        ),
        Err(EpollError::TooManyFds)
    );

    crate::config::KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn epoll_wait_clamps_events_to_runtime_limit() {
    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_network_epoll_max_events(Some(1));

    let mut epoll = EpollInstance::new(8);
    for fd in 10..13 {
        assert!(epoll
            .ctl(
                EpollOp::Add,
                fd,
                Some(EpollEvent {
                    events: EpollEvents::EPOLLIN,
                    data: fd as u64,
                }),
            )
            .is_ok());
        epoll.notify_ready(fd, EpollEvents::EPOLLIN);
    }

    let ready = epoll.wait(8);
    assert_eq!(ready.len(), 1);

    crate::config::KernelConfig::reset_runtime_overrides();
}
