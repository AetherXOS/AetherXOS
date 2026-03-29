
    #[cfg(feature = "network_transport")]
    use super::*;

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn posix_udp_roundtrip() {
        let rx = socket(AddressFamily::Inet, SocketType::Datagram).expect("rx socket");
        bind(rx, SocketAddrV4::localhost(41001)).expect("bind rx");

        let tx = socket(AddressFamily::Inet, SocketType::Datagram).expect("tx socket");
        bind(tx, SocketAddrV4::localhost(41000)).expect("bind tx");

        sendto(tx, SocketAddrV4::localhost(41001), b"posix-udp").expect("sendto");
        let packet = recvfrom(rx).expect("recvfrom");

        assert_eq!(packet.addr.port, 41000);
        assert_eq!(packet.payload, b"posix-udp");

        let _ = close(tx);
        let _ = close(rx);
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn posix_tcp_roundtrip() {
        let server = socket(AddressFamily::Inet, SocketType::Stream).expect("server socket");
        bind(server, SocketAddrV4::localhost(42001)).expect("bind server");
        listen(server, 8).expect("listen");

        let client = socket(AddressFamily::Inet, SocketType::Stream).expect("client socket");
        connect(client, SocketAddrV4::localhost(42001)).expect("connect");

        let accepted = accept(server).expect("accept");
        send(client, b"posix-tcp").expect("send");
        let recv_payload = recv(accepted).expect("recv");
        assert_eq!(recv_payload, b"posix-tcp");

        let _ = close(accepted);
        let _ = close(client);
        let _ = close(server);
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn posix_socket_options_and_peername() {
        let server = socket(AddressFamily::Inet, SocketType::Stream).expect("server socket");
        bind(server, SocketAddrV4::localhost(42011)).expect("bind server");
        listen(server, 8).expect("listen");

        let client = socket(AddressFamily::Inet, SocketType::Stream).expect("client socket");
        set_socket_option(client, SocketOption::NonBlocking, true).expect("set nonblocking");
        set_socket_option(client, SocketOption::ReuseAddr, true).expect("set reuseaddr");
        let options = socket_options(client).expect("socket options");
        assert!(options.nonblocking);
        assert!(options.reuse_addr);

        connect(client, SocketAddrV4::localhost(42011)).expect("connect");
        let accepted = accept(server).expect("accept");

        let client_peer = getpeername(client).expect("client peer");
        assert_eq!(client_peer.port, 42011);

        let accepted_peer = getpeername(accepted).expect("accepted peer");
        assert_ne!(accepted_peer.port, 0);

        let _ = close(accepted);
        let _ = close(client);
        let _ = close(server);
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn posix_shutdown_blocks_direction() {
        let server = socket(AddressFamily::Inet, SocketType::Stream).expect("server socket");
        bind(server, SocketAddrV4::localhost(42021)).expect("bind server");
        listen(server, 8).expect("listen");

        let client = socket(AddressFamily::Inet, SocketType::Stream).expect("client socket");
        connect(client, SocketAddrV4::localhost(42021)).expect("connect");
        let accepted = accept(server).expect("accept");

        shutdown(client, ShutdownHow::Write).expect("shutdown write");
        assert!(send(client, b"blocked").is_err());

        send(accepted, b"from-server").expect("send from server");
        let payload = recv(client).expect("recv still works");
        assert_eq!(payload, b"from-server");

        shutdown(client, ShutdownHow::Read).expect("shutdown read");
        assert!(recv(client).is_err());

        let _ = close(accepted);
        let _ = close(client);
        let _ = close(server);
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn posix_poll_reports_writable_then_readable() {
        let server = socket(AddressFamily::Inet, SocketType::Stream).expect("server socket");
        bind(server, SocketAddrV4::localhost(42031)).expect("bind server");
        listen(server, 8).expect("listen");

        let client = socket(AddressFamily::Inet, SocketType::Stream).expect("client socket");
        connect(client, SocketAddrV4::localhost(42031)).expect("connect");
        let accepted = accept(server).expect("accept");

        let mut writable_probe = [PosixPollFd::new(client, PosixPollEvents::OUT)];
        let ready = poll(&mut writable_probe, 0).expect("poll writable");
        assert_eq!(ready, 1);
        assert!(writable_probe[0].revents.contains(PosixPollEvents::OUT));

        send(accepted, b"poll-ready").expect("send");
        let mut readable_probe = [PosixPollFd::new(client, PosixPollEvents::IN)];
        let ready_read = poll(&mut readable_probe, 8).expect("poll readable");
        assert_eq!(ready_read, 1);
        assert!(readable_probe[0].revents.contains(PosixPollEvents::IN));

        let payload = recv(client).expect("recv");
        assert_eq!(payload, b"poll-ready");

        let _ = close(accepted);
        let _ = close(client);
        let _ = close(server);
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn posix_select_reports_datagram_readiness() {
        let rx = socket(AddressFamily::Inet, SocketType::Datagram).expect("rx socket");
        bind(rx, SocketAddrV4::localhost(42041)).expect("bind rx");

        let tx = socket(AddressFamily::Inet, SocketType::Datagram).expect("tx socket");
        bind(tx, SocketAddrV4::localhost(42040)).expect("bind tx");

        sendto(tx, SocketAddrV4::localhost(42041), b"select-ready").expect("sendto");
        let selected = select(&[rx], &[tx], &[], 8).expect("select");

        assert_eq!(selected.readable.as_slice(), &[rx]);
        assert_eq!(selected.writable.as_slice(), &[tx]);
        assert!(selected.exceptional.is_empty());

        let packet = recvfrom(rx).expect("recvfrom");
        assert_eq!(packet.payload, b"select-ready");

        let _ = close(tx);
        let _ = close(rx);
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn posix_fcntl_roundtrip_flags() {
        let fd = socket(AddressFamily::Inet, SocketType::Datagram).expect("socket");

        let initial = fcntl_getfl(fd).expect("fcntl get initial");
        assert!(!initial.contains(PosixFdFlags::NONBLOCK));

        let desired = PosixFdFlags::NONBLOCK;
        fcntl_setfl(fd, desired).expect("fcntl set");

        let after = fcntl(fd, FcntlCmd::GetFl).expect("fcntl get");
        assert!(after.contains(PosixFdFlags::NONBLOCK));
        assert_eq!(after.bits(), PosixFdFlags::NONBLOCK.bits());

        let _ = close(fd);
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn posix_errno_mapping_reports_ebadf() {
        let err = recv_errno(999_999).expect_err("expected ebadf");
        assert_eq!(err, PosixErrno::BadFileDescriptor);
        assert_eq!(err.code(), 9);
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn posix_numeric_constants_match_expected_values() {
        assert_eq!(
            AddressFamily::Inet.as_raw(),
            crate::modules::posix_consts::net::AF_INET
        );
        assert_eq!(
            SocketType::Stream.as_raw(),
            crate::modules::posix_consts::net::SOCK_STREAM
        );
        assert_eq!(
            SocketType::Datagram.as_raw(),
            crate::modules::posix_consts::net::SOCK_DGRAM
        );
        assert_eq!(
            ShutdownHow::Read.as_raw(),
            crate::modules::posix_consts::net::SHUT_RD
        );
        assert_eq!(
            ShutdownHow::Write.as_raw(),
            crate::modules::posix_consts::net::SHUT_WR
        );
        assert_eq!(
            ShutdownHow::Both.as_raw(),
            crate::modules::posix_consts::net::SHUT_RDWR
        );
        assert_eq!(
            PosixFdFlags::NONBLOCK.bits(),
            crate::modules::posix_consts::net::O_NONBLOCK
        );
        assert_eq!(
            PosixIoctlCmd::FionRead.as_raw(),
            crate::modules::posix_consts::net::FIONREAD
        );
        assert_eq!(
            PosixSockOpt::ReuseAddr.as_raw(),
            crate::modules::posix_consts::net::SO_REUSEADDR
        );
        assert_eq!(
            PosixSockOpt::SocketError.as_raw(),
            crate::modules::posix_consts::net::SO_ERROR
        );
        assert_eq!(
            PosixErrno::from_code(crate::modules::posix_consts::errno::EAGAIN),
            PosixErrno::Again
        );
        assert_eq!(
            PosixErrno::from_code(crate::modules::posix_consts::errno::EBADF),
            PosixErrno::BadFileDescriptor
        );
        assert_eq!(
            AddressFamily::from_raw(crate::modules::posix_consts::net::AF_INET),
            Some(AddressFamily::Inet)
        );
        assert_eq!(
            SocketType::from_raw(crate::modules::posix_consts::net::SOCK_STREAM),
            Some(SocketType::Stream)
        );
        assert_eq!(
            ShutdownHow::from_raw(crate::modules::posix_consts::net::SHUT_RDWR),
            Some(ShutdownHow::Both)
        );
        assert_eq!(
            PosixSockOpt::from_raw(crate::modules::posix_consts::net::SO_ERROR),
            Some(PosixSockOpt::SocketError)
        );
        assert_eq!(
            PosixIoctlCmd::from_raw(crate::modules::posix_consts::net::FIONREAD),
            Some(PosixIoctlCmd::FionRead)
        );
        assert_eq!(PosixIoctlCmd::from_raw(0), None);
        assert_eq!(
            PosixMsgFlags::PEEK.bits(),
            crate::modules::posix_consts::net::MSG_PEEK
        );
        assert_eq!(
            PosixMsgFlags::DONTWAIT.bits(),
            crate::modules::posix_consts::net::MSG_DONTWAIT
        );
        assert_eq!(
            PosixMsgFlags::WAITALL.bits(),
            crate::modules::posix_consts::net::MSG_WAITALL
        );
        assert_eq!(
            PosixMsgFlags::NOSIGNAL.bits(),
            crate::modules::posix_consts::net::MSG_NOSIGNAL
        );
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn posix_recv_peek_preserves_payload() {
        let server = socket(AddressFamily::Inet, SocketType::Stream).expect("server socket");
        bind(server, SocketAddrV4::localhost(42051)).expect("bind server");
        listen(server, 8).expect("listen");

        let client = socket(AddressFamily::Inet, SocketType::Stream).expect("client socket");
        connect(client, SocketAddrV4::localhost(42051)).expect("connect");
        let accepted = accept(server).expect("accept");

        send(accepted, b"peekable").expect("send");
        let peeked = recv_with_flags(client, PosixMsgFlags::PEEK).expect("peek recv");
        assert_eq!(peeked, b"peekable");

        let actual = recv(client).expect("actual recv");
        assert_eq!(actual, b"peekable");

        let _ = close(accepted);
        let _ = close(client);
        let _ = close(server);
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn posix_recv_dontwait_returns_would_block() {
        let fd = socket(AddressFamily::Inet, SocketType::Datagram).expect("socket");
        bind(fd, SocketAddrV4::localhost(42061)).expect("bind");

        let err =
            recv_with_flags_errno(fd, PosixMsgFlags::DONTWAIT).expect_err("expected would block");
        assert_eq!(err, PosixErrno::WouldBlock);

        let _ = close(fd);
    }
