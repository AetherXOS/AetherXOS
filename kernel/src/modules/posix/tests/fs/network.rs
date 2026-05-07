use super::*;
use crate::modules::posix::net;

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_socketpair_sendmsg_recvmsg_flow() {
    let (fd_a, fd_b) = net::socketpair().expect("socketpair");
    let sent = net::sendmsg(fd_a, &[b"hello"]).expect("sendmsg");
    assert_eq!(sent, 5);

    let mut p1 = [0u8; 2];
    let mut p2 = [0u8; 4];
    {
        let mut iov = [&mut p1[..], &mut p2[..]];
        let recv = net::recvmsg(fd_b, &mut iov).expect("recvmsg");
        assert_eq!(recv, 5);
    }
    assert_eq!(&p1, b"he");
    assert_eq!(&p2[..3], b"llo");

    net::close(fd_a).expect("close socketpair a");
    net::close(fd_b).expect("close socketpair b");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_socket_raw_nonblock_flag_applies() {
    let fd = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_INET,
        crate::modules::posix_consts::net::SOCK_DGRAM | crate::modules::posix_consts::net::SOCK_NONBLOCK,
        0,
    )
    .expect("socket raw nonblock");

    let got_flags = crate::modules::libnet::posix_fcntl_getfl_errno(fd).expect("fcntl getfl");
    assert!(got_flags.contains(crate::modules::libnet::PosixFdFlags::NONBLOCK));

    net::close(fd).expect("close socket raw");

    let udp_fd = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_INET,
        crate::modules::posix_consts::net::SOCK_DGRAM,
        crate::modules::posix_consts::net::IPPROTO_UDP,
    )
    .expect("socket raw udp protocol");
    net::close(udp_fd).expect("close socket udp");

    let tcp_fd = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_INET,
        crate::modules::posix_consts::net::SOCK_STREAM,
        crate::modules::posix_consts::net::IPPROTO_TCP,
    )
    .expect("socket raw tcp protocol");
    net::close(tcp_fd).expect("close socket tcp");

    assert_eq!(
        net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_INET,
            crate::modules::posix_consts::net::SOCK_STREAM,
            crate::modules::posix_consts::net::IPPROTO_UDP,
        ),
        Err(PosixErrno::Invalid)
    );
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_recvmsg_waitall_fills_iov() {
    let (fd_a, fd_b) = net::socketpair().expect("socketpair");
    let sent = net::sendmsg(fd_a, &[b"abc", b"def"]).expect("sendmsg");
    assert_eq!(sent, 6);

    let mut p1 = [0u8; 2];
    let mut p2 = [0u8; 4];
    {
        let mut iov = [&mut p1[..], &mut p2[..]];
        let recv = net::recvmsg_flags(
            fd_b,
            &mut iov,
            crate::modules::libnet::PosixMsgFlags::WAITALL,
        )
        .expect("recvmsg waitall");
        assert_eq!(recv, 6);
    }
    assert_eq!(&p1, b"ab");
    assert_eq!(&p2, b"cdef");

    net::close(fd_a).expect("close socketpair a");
    net::close(fd_b).expect("close socketpair b");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_unix_socketpair_raw_nonblock() {
    let (fd_a, fd_b) = net::socketpair_raw_errno(
        crate::modules::posix_consts::net::AF_UNIX,
        crate::modules::posix_consts::net::SOCK_STREAM | crate::modules::posix_consts::net::SOCK_NONBLOCK,
        0,
    )
    .expect("socketpair raw unix nonblock");

    let flags_a = crate::modules::libnet::posix_fcntl_getfl_errno(fd_a).expect("fcntl a");
    let flags_b = crate::modules::libnet::posix_fcntl_getfl_errno(fd_b).expect("fcntl b");
    assert!(flags_a.contains(crate::modules::libnet::PosixFdFlags::NONBLOCK));
    assert!(flags_b.contains(crate::modules::libnet::PosixFdFlags::NONBLOCK));

    net::close(fd_a).expect("close a");
    net::close(fd_b).expect("close b");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_sendmmsg_recvmmsg_flow() {
    let (fd_a, fd_b) = net::socketpair().expect("socketpair");

    let msgs = [b"one".as_slice(), b"two".as_slice(), b"three".as_slice()];
    let sent_msgs = net::sendmmsg(fd_a, &msgs, crate::modules::libnet::PosixMsgFlags::empty())
        .expect("sendmmsg");
    assert_eq!(sent_msgs, 3);

    let mut b1 = [0u8; 8];
    let mut b2 = [0u8; 8];
    let mut b3 = [0u8; 8];
    let mut out = [&mut b1[..], &mut b2[..], &mut b3[..]];
    let recv_msgs = net::recvmmsg(fd_b, &mut out, crate::modules::libnet::PosixMsgFlags::empty())
        .expect("recvmmsg");
    assert_eq!(recv_msgs, 3);
    assert_eq!(&b1[..3], b"one");
    assert_eq!(&b2[..3], b"two");
    assert_eq!(&b3[..5], b"three");

    net::close(fd_a).expect("close socketpair a");
    net::close(fd_b).expect("close socketpair b");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_unix_path_bind_connect_accept_flow() {
    let listener = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_UNIX,
        crate::modules::posix_consts::net::SOCK_STREAM,
        0,
    )
    .expect("listener socket");
    let client = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_UNIX,
        crate::modules::posix_consts::net::SOCK_STREAM,
        0,
    )
    .expect("client socket");

    net::unix_bind_path(listener, b"/tmp/hyper-unix.sock").expect("unix bind path");
    net::unix_listen(listener, 8).expect("unix listen");
    net::unix_connect_path(client, b"/tmp/hyper-unix.sock").expect("unix connect path");

    let accepted = net::unix_accept(listener).expect("unix accept");
    let sent = net::sendmsg(client, &[b"unix-flow"]).expect("sendmsg");
    assert_eq!(sent, 9);

    let mut buf = [0u8; 16];
    let mut iov = [&mut buf[..]];
    let got = net::recvmsg(accepted, &mut iov).expect("recvmsg");
    assert_eq!(got, 9);
    assert_eq!(&buf[..9], b"unix-flow");

    net::close(accepted).expect("close accepted");
    net::close(client).expect("close client");
    net::close(listener).expect("close listener");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_async_nonblocking_helpers_work() {
    let (fd_a, fd_b) = net::socketpair_raw_errno(
        crate::modules::posix_consts::net::AF_UNIX,
        crate::modules::posix_consts::net::SOCK_STREAM | crate::modules::posix_consts::net::SOCK_NONBLOCK,
        0,
    )
    .expect("socketpair raw nonblock");

    let mut empty = [0u8; 8];
    let mut empty_iov = [&mut empty[..]];
    let initial = net::recvmsg_async(fd_b, &mut empty_iov);
    assert_eq!(initial, Err(PosixErrno::Again));

    let sent = net::sendmsg_async(fd_a, &[b"async"]).expect("sendmsg async");
    assert_eq!(sent, 5);

    let mut out = [0u8; 8];
    let mut out_iov = [&mut out[..]];
    let got = net::recvmsg_async(fd_b, &mut out_iov).expect("recvmsg async");
    assert_eq!(got, 5);
    assert_eq!(&out[..5], b"async");

    net::close(fd_a).expect("close a");
    net::close(fd_b).expect("close b");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_unix_abstract_namespace_flow() {
    let listener = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_UNIX,
        crate::modules::posix_consts::net::SOCK_STREAM,
        0,
    )
    .expect("listener socket");
    let client = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_UNIX,
        crate::modules::posix_consts::net::SOCK_STREAM,
        0,
    )
    .expect("client socket");

    net::unix_bind_addr(listener, b"\0hyper-abstract").expect("bind abstract");
    net::unix_listen(listener, 4).expect("listen abstract");
    net::unix_connect_addr(client, b"\0hyper-abstract").expect("connect abstract");
    let accepted = net::unix_accept(listener).expect("accept abstract");

    net::sendmsg(client, &[b"abst"]).expect("send");
    let mut out = [0u8; 8];
    let mut iov = [&mut out[..]];
    let n = net::recvmsg(accepted, &mut iov).expect("recv");
    assert_eq!(n, 4);
    assert_eq!(&out[..4], b"abst");

    net::close(accepted).expect("close accepted");
    net::close(client).expect("close client");
    net::close(listener).expect("close listener");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_epoll_reports_ready_fds() {
    let (fd_a, fd_b) = net::socketpair().expect("socketpair");
    let epfd = net::epoll_create1(0).expect("epoll create");
    net::epoll_ctl(
        epfd,
        crate::modules::posix_consts::net::EPOLL_CTL_ADD,
        fd_b,
        crate::modules::posix_consts::net::EPOLLIN,
    )
    .expect("epoll ctl add");

    net::sendmsg(fd_a, &[b"evt"]).expect("sendmsg");
    let ready = net::epoll_wait(epfd, 8, 8).expect("epoll wait");
    assert!(!ready.is_empty());
    assert!(ready.iter().any(|ev| ev.fd == fd_b && (ev.events & crate::modules::posix_consts::net::EPOLLIN) != 0));

    net::epoll_close(epfd).expect("epoll close");
    net::close(fd_a).expect("close a");
    net::close(fd_b).expect("close b");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_await_helpers_work() {
    let (fd_a, fd_b) = net::socketpair().expect("socketpair");
    net::await_writable(fd_a, 2).expect("await writable");
    net::sendmsg(fd_a, &[b"ready"]).expect("send");
    net::await_readable(fd_b, 8).expect("await readable");

    let mut out = [0u8; 8];
    let mut iov = [&mut out[..]];
    let n = net::recvmsg(fd_b, &mut iov).expect("recv");
    assert_eq!(n, 5);
    assert_eq!(&out[..5], b"ready");

    net::close(fd_a).expect("close a");
    net::close(fd_b).expect("close b");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_typed_socket_api_works() {
    use crate::modules::posix_consts::net_typed as typed_net;

    let fd = net::socket_typed_errno(
        typed_net::AddressFamily::INET,
        typed_net::SocketType::DGRAM.with_flag(typed_net::SocketType::NONBLOCK),
        typed_net::Protocol::DEFAULT,
    )
    .expect("typed socket");

    let flags = crate::modules::libnet::posix_fcntl_getfl_errno(fd).expect("fcntl getfl");
    assert!(flags.contains(crate::modules::libnet::PosixFdFlags::NONBLOCK));

    net::close(fd).expect("close");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_sockaddr_un_helpers_work() {
    let listener = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_UNIX,
        crate::modules::posix_consts::net::SOCK_STREAM,
        0,
    )
    .expect("listener socket");
    let client = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_UNIX,
        crate::modules::posix_consts::net::SOCK_STREAM,
        0,
    )
    .expect("client socket");

    let addr = net::SockAddrUn::from_path(b"/tmp/hyper-sockaddr-un.sock").expect("sockaddr path");
    assert!(!addr.is_abstract());
    net::unix_bind_sockaddr(listener, &addr).expect("bind sockaddr");
    net::unix_listen(listener, 8).expect("listen");
    net::unix_connect_sockaddr(client, &addr).expect("connect sockaddr");

    let accepted = net::unix_accept(listener).expect("accept");
    net::sendmsg(client, &[b"su"]).expect("sendmsg");
    let mut out = [0u8; 4];
    let mut iov = [&mut out[..]];
    let n = net::recvmsg(accepted, &mut iov).expect("recvmsg");
    assert_eq!(n, 2);
    assert_eq!(&out[..2], b"su");

    net::close(accepted).expect("close accepted");
    net::close(client).expect("close client");
    net::close(listener).expect("close listener");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_epoll_edge_triggered_behaves_like_et() {
    let (fd_a, fd_b) = net::socketpair().expect("socketpair");
    let epfd = net::epoll_create1(0).expect("epoll create");
    net::epoll_ctl(
        epfd,
        crate::modules::posix_consts::net::EPOLL_CTL_ADD,
        fd_b,
        crate::modules::posix_consts::net::EPOLLIN | crate::modules::posix_consts::net::EPOLLET,
    )
    .expect("epoll ctl add");

    net::sendmsg(fd_a, &[b"edge"]).expect("send");
    let ready1 = net::epoll_wait(epfd, 8, 8).expect("epoll wait 1");
    assert!(!ready1.is_empty());

    let ready2 = net::epoll_wait(epfd, 8, 2).expect("epoll wait 2");
    assert!(ready2.is_empty());

    let mut out = [0u8; 8];
    let mut iov = [&mut out[..]];
    let n = net::recvmsg(fd_b, &mut iov).expect("recv");
    assert_eq!(n, 4);

    net::sendmsg(fd_a, &[b"edge2"]).expect("send2");
    let ready3 = net::epoll_wait(epfd, 8, 8).expect("epoll wait 3");
    assert!(!ready3.is_empty());

    net::epoll_close(epfd).expect("epoll close");
    net::close(fd_a).expect("close a");
    net::close(fd_b).expect("close b");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_sockaddr_un_raw_roundtrip() {
    let addr = net::SockAddrUn::from_path(b"\0hyper-raw").expect("from path");
    let raw = addr.encode_raw();
    let decoded = net::SockAddrUn::decode_raw(&raw).expect("decode raw");
    assert!(decoded.is_abstract());
    assert_eq!(decoded.as_path_bytes(), b"\0hyper-raw");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_epoll_pwait_works() {
    let (fd_a, fd_b) = net::socketpair().expect("socketpair");
    let epfd = net::epoll_create1(0).expect("epoll create");
    net::epoll_ctl(
        epfd,
        crate::modules::posix_consts::net::EPOLL_CTL_ADD,
        fd_b,
        net::EPOLLIN,
    )
    .expect("epoll ctl add");

    net::sendmsg(fd_a, &[b"pw"]).expect("send");
    let ready = net::epoll_pwait(epfd, 4, 8, None).expect("epoll pwait");
    assert!(!ready.is_empty());

    net::epoll_close(epfd).expect("epoll close");
    net::close(fd_a).expect("close a");
    net::close(fd_b).expect("close b");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_epoll_pwait2_works() {
    let (fd_a, fd_b) = net::socketpair().expect("socketpair");
    let epfd = net::epoll_create1(0).expect("epoll create");
    net::epoll_ctl(
        epfd,
        crate::modules::posix_consts::net::EPOLL_CTL_ADD,
        fd_b,
        net::EPOLLIN,
    )
    .expect("epoll ctl add");

    net::sendmsg(fd_a, &[b"pw2"]).expect("send");
    let ready = net::epoll_pwait2(
        epfd,
        4,
        Some(net::EpollTimeout { sec: 0, nsec: 1_000_000 }),
        None,
    )
    .expect("epoll pwait2");
    assert!(!ready.is_empty());

    net::epoll_close(epfd).expect("epoll close");
    net::close(fd_a).expect("close a");
    net::close(fd_b).expect("close b");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_unix_sendto_recvfrom_addr_flow() {
    let rx = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_UNIX,
        crate::modules::posix_consts::net::SOCK_DGRAM,
        0,
    )
    .expect("rx socket");
    let tx = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_UNIX,
        crate::modules::posix_consts::net::SOCK_DGRAM,
        0,
    )
    .expect("tx socket");

    let rx_addr = net::SockAddrUn::from_path(b"/tmp/hyper-unix-rx.sock").expect("rx addr");
    let tx_addr = net::SockAddrUn::from_path(b"/tmp/hyper-unix-tx.sock").expect("tx addr");
    net::unix_bind_sockaddr(rx, &rx_addr).expect("bind rx");
    net::unix_bind_sockaddr(tx, &tx_addr).expect("bind tx");

    let sent = net::unix_sendto_sockaddr(tx, &rx_addr, b"unix-dgram").expect("unix sendto");
    assert_eq!(sent, 10);

    let (from, payload) = net::unix_recvfrom_addr(rx, crate::modules::libnet::PosixMsgFlags::empty())
        .expect("unix recvfrom addr");
    assert_eq!(from.as_path_bytes(), tx_addr.as_path_bytes());
    assert_eq!(payload, b"unix-dgram");

    net::close(tx).expect("close tx");
    net::close(rx).expect("close rx");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_epoll_wait_timeout_ms_works() {
    let (fd_a, fd_b) = net::socketpair().expect("socketpair");
    let epfd = net::epoll_create1(0).expect("epoll create");
    net::epoll_ctl(
        epfd,
        crate::modules::posix_consts::net::EPOLL_CTL_ADD,
        fd_b,
        net::EPOLLIN,
    )
    .expect("epoll ctl add");

    net::sendmsg(fd_a, &[b"ms"]).expect("send");
    let ready = net::epoll_wait_timeout_ms(epfd, 4, 10).expect("epoll wait timeout ms");
    assert!(!ready.is_empty());

    net::epoll_close(epfd).expect("epoll close");
    net::close(fd_a).expect("close a");
    net::close(fd_b).expect("close b");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_accept4_raw_nonblock_works() {
    let server = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_INET,
        crate::modules::posix_consts::net::SOCK_STREAM,
        0,
    )
    .expect("server socket");
    crate::modules::libnet::posix_bind_errno(server, crate::modules::libnet::PosixSocketAddrV4::localhost(42151))
        .expect("bind server");
    crate::modules::libnet::posix_listen_errno(server, 8).expect("listen");

    let client = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_INET,
        crate::modules::posix_consts::net::SOCK_STREAM,
        0,
    )
    .expect("client socket");
    crate::modules::libnet::posix_connect_errno(client, crate::modules::libnet::PosixSocketAddrV4::localhost(42151))
        .expect("connect");

    let accepted = net::accept4_raw_errno(
        server,
        crate::modules::posix_consts::net::SOCK_NONBLOCK,
    )
    .expect("accept4 raw");
    let flags = crate::modules::libnet::posix_fcntl_getfl_errno(accepted).expect("fcntl getfl");
    assert!(flags.contains(crate::modules::libnet::PosixFdFlags::NONBLOCK));

    net::close(accepted).expect("close accepted");
    net::close(client).expect("close client");
    net::close(server).expect("close server");
}

#[cfg(feature = "networking")]
#[test_case]
fn posix_network_sockopt_raw_roundtrip_works() {
    let fd = net::socket_raw_errno(
        crate::modules::posix_consts::net::AF_INET,
        crate::modules::posix_consts::net::SOCK_DGRAM,
        0,
    )
    .expect("socket");

    net::setsockopt_raw(
        fd,
        crate::modules::posix_consts::net::SOL_SOCKET,
        crate::modules::posix_consts::net::SO_REUSEADDR,
        1,
    )
    .expect("setsockopt raw reuseaddr");
    let reuse = net::getsockopt_raw(
        fd,
        crate::modules::posix_consts::net::SOL_SOCKET,
        crate::modules::posix_consts::net::SO_REUSEADDR,
    )
    .expect("getsockopt raw reuseaddr");
    assert_eq!(reuse, 1);

    super::net::setsockopt_raw(
        fd,
        crate::modules::posix_consts::net::SOL_SOCKET,
        crate::modules::posix_consts::net::SO_RCVTIMEO,
        3,
    )
    .expect("setsockopt raw rcvtimeo");
    let recv_retry = net::getsockopt_raw(
        fd,
        crate::modules::posix_consts::net::SOL_SOCKET,
        crate::modules::posix_consts::net::SO_RCVTIMEO,
    )
    .expect("getsockopt raw rcvtimeo");
    assert_eq!(recv_retry, 3);

    net::close(fd).expect("close");
}
