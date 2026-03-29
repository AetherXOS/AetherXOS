    use super::fs;
    use super::time::PosixTimespec;

    #[test_case]
    fn posix_fs_basic_file_flow() {
        let fs_id = fs::mount_ramfs("/posix").expect("mount");

        let fd = fs::open(fs_id, "/posix/demo.txt", true).expect("open create");
        let wrote = fs::write(fd, b"hello-posix").expect("write");
        assert_eq!(wrote, b"hello-posix".len());

        let _ = fs::lseek(fd, 0, fs::SeekWhence::Set).expect("seek start");
        let mut buf = [0u8; 16];
        let read = fs::read(fd, &mut buf).expect("read");
        assert_eq!(&buf[..read], b"hello-posix");

        let md = fs::stat(fs_id, "/posix/demo.txt").expect("stat");
        assert_eq!(md.size, b"hello-posix".len() as u64);

        fs::close(fd).expect("close");
        fs::unlink(fs_id, "/posix/demo.txt").expect("unlink");
        fs::unmount(fs_id).expect("unmount");
    }

    #[test_case]
    fn posix_fs_directory_ops_work() {
        let fs_id = fs::mount_ramfs("/posix_dir_ops").expect("mount");
        fs::mkdir(fs_id, "/dir", 0o755).expect("mkdir");

        let fd = fs::open(fs_id, "/dir/a", true).expect("create");
        fs::write(fd, b"x").expect("write");
        fs::close(fd).expect("close");

        fs::rename(fs_id, "/dir/a", "/dir/b").expect("rename");
        assert!(fs::access(fs_id, "/dir/b").expect("access"));
        fs::unlink(fs_id, "/dir/b").expect("unlink");
        fs::rmdir(fs_id, "/dir").expect("rmdir");
        fs::unmount(fs_id).expect("unmount");
    }

    #[test_case]
    fn posix_fs_extended_apis_work() {
        let fs_id = fs::mount_ramfs("/posix_ext").expect("mount");

        let fd = fs::creat(fs_id, "/ext_file.txt", 0o644).expect("creat");
        fs::write(fd, b"abcdef").expect("write");

        fs::ftruncate(fd, 3).expect("ftruncate");
        let _ = fs::lseek(fd, 0, fs::SeekWhence::Set).expect("seek");
        let mut buf = [0u8; 8];
        let read = fs::read(fd, &mut buf).expect("read");
        assert_eq!(&buf[..read], b"abc");

        let md = fs::stat(fs_id, "/ext_file.txt").expect("stat");
        assert_eq!(md.mode, 0o644);
        assert_eq!(md.uid, 0);
        assert_eq!(md.gid, 0);

        fs::chmod(fs_id, "/ext_file.txt", 0o600).expect("chmod");
        fs::chown(fs_id, "/ext_file.txt", 1000, 1000).expect("chown");
        let md_after = fs::stat(fs_id, "/ext_file.txt").expect("stat after chmod/chown");
        assert_eq!(md_after.mode, 0o600);
        assert_eq!(md_after.uid, 1000);
        assert_eq!(md_after.gid, 1000);
        fs::link(fs_id, "/ext_file.txt", "/hard.txt").expect("link");
        let hard_target = fs::stat(fs_id, "/hard.txt").expect("stat hard");
        assert_eq!(hard_target.size, md_after.size);
        fs::symlink(fs_id, "/ext_file.txt", "/sym.txt").expect("symlink");
        let target = fs::readlink(fs_id, "/sym.txt").expect("readlink");
        assert_eq!(target, "/ext_file.txt");
        fs::utimensat(fs_id, "/ext_file.txt").expect("utimensat");

        let map_id = fs::mmap(fs_id, "/ext_file.txt", 0, 3, true).expect("mmap");
        let mut mapped = [0u8; 3];
        let map_read = fs::mmap_read(map_id, &mut mapped, 0).expect("mmap_read");
        assert_eq!(map_read, 3);
        assert_eq!(&mapped, b"abc");

        let map_write = fs::mmap_write(map_id, b"XYZ", 0).expect("mmap_write");
        assert_eq!(map_write, 3);
        fs::fdatasync(fd).expect("fdatasync");
        fs::msync(map_id).expect("msync");
        fs::munmap(map_id).expect("munmap");

        let _ = fs::lseek(fd, 0, fs::SeekWhence::Set).expect("seek after msync");
        let mut verify = [0u8; 4];
        let verify_read = fs::read(fd, &mut verify).expect("read verify");
        assert_eq!(verify_read, 3);
        assert_eq!(&verify[..3], b"XYZ");

        fs::close(fd).expect("close");
        fs::unlink(fs_id, "/hard.txt").expect("unlink hard");
        fs::unlink(fs_id, "/sym.txt").expect("unlink symlink");
        fs::unlink(fs_id, "/ext_file.txt").expect("unlink file");
        fs::unmount(fs_id).expect("unmount");
    }

    #[test_case]
    fn posix_fs_bulk_new_apis_work() {
        let fs_id = fs::mount_ramfs("/posix_bulk").expect("mount");
        let fd = fs::open(fs_id, "/bulk_a.txt", true).expect("open create");
        let _ = fs::writev(fd, &[b"ab", b"cd"]).expect("writev");

        let mut pbuf = [0u8; 2];
        let pread_n = fs::pread(fd, &mut pbuf, 1).expect("pread");
        assert_eq!(pread_n, 2);
        assert_eq!(&pbuf, b"bc");

        let pwrite_n = fs::pwrite(fd, b"ZZ", 2).expect("pwrite");
        assert_eq!(pwrite_n, 2);

        let pwritev_n = fs::pwritev(fd, &[b"12", b"34"], 0).expect("pwritev");
        assert_eq!(pwritev_n, 4);

        let mut pv1 = [0u8; 2];
        let mut pv2 = [0u8; 2];
        let mut piov = [&mut pv1[..], &mut pv2[..]];
        let preadv_n = fs::preadv(fd, &mut piov, 0).expect("preadv");
        assert_eq!(preadv_n, 4);
        assert_eq!(&pv1, b"12");
        assert_eq!(&pv2, b"34");

        let _ = fs::lseek(fd, 0, fs::SeekWhence::Set).expect("seek start");
        let mut r1 = [0u8; 2];
        let mut r2 = [0u8; 3];
        let mut iov = [&mut r1[..], &mut r2[..]];
        let rv = fs::readv(fd, &mut iov).expect("readv");
        assert_eq!(rv, 4);
        assert_eq!(&r1, b"ab");
        assert_eq!(&r2[..2], b"ZZ");

        let st = fs::fstat(fd).expect("fstat");
        assert_eq!(st.size, 4);
        fs::fdatasync(fd).expect("fdatasync");

        let fd2 = fs::dup(fd).expect("dup");
        let fd3 = fs::dup2(fd, 60000).expect("dup2");
        assert_eq!(fd3, 60000);

        let lst = fs::lstat(fs_id, "/bulk_a.txt").expect("lstat");
        assert_eq!(lst.mode, 0o644);
        assert_eq!(lst.uid, 0);
        assert_eq!(lst.gid, 0);

        fs::fchmod(fd2, 0o640).expect("fchmod");
        fs::fchown(fd2, 2000, 3000).expect("fchown");
        let lst_after = fs::lstat(fs_id, "/bulk_a.txt").expect("lstat after fchmod/fchown");
        assert_eq!(lst_after.mode, 0o640);
        assert_eq!(lst_after.uid, 2000);
        assert_eq!(lst_after.gid, 3000);
        fs::chdir(fs_id, "/").expect("chdir");
        assert_eq!(fs::getcwd(fs_id).expect("getcwd"), "/");
        assert_eq!(fs::umask(0o027), 0o022);

        let copied = fs::copy_file_range(fs_id, "/bulk_a.txt", "/bulk_b.txt").expect("copy_file_range");
        assert_eq!(copied, 4);
        let rp = fs::realpath(fs_id, "/bulk_b.txt").expect("realpath");
        assert_eq!(rp, "/bulk_b.txt");
        fs::posix_fallocate(fd2, 16).expect("posix_fallocate");
        fs::fallocate(fd2, 0, 20, 4).expect("fallocate with offset");
        let grown = fs::fstat(fd2).expect("fstat grown");
        assert!(grown.size >= 24);
        fs::fallocate(
            fd2,
            crate::modules::posix_consts::fs::FALLOC_FL_KEEP_SIZE,
            30,
            8,
        )
        .expect("fallocate keep size");
        let same = fs::fstat(fd2).expect("fstat keep size");
        assert_eq!(same.size, grown.size);
        fs::fallocate(
            fd2,
            crate::modules::posix_consts::fs::FALLOC_FL_KEEP_SIZE
                | crate::modules::posix_consts::fs::FALLOC_FL_PUNCH_HOLE,
            0,
            2,
        )
        .expect("fallocate punch hole");
        fs::lseek(fd2, 0, fs::SeekWhence::Set).expect("lseek start");
        let mut punched = [0xFFu8; 2];
        let punched_n = fs::read(fd2, &mut punched).expect("read punched");
        assert_eq!(punched_n, 2);
        assert_eq!(punched, [0u8; 2]);

        let entries = fs::scandir(fs_id, "/").expect("scandir");
        assert!(!entries.is_empty());

        fs::renameat(fs_id, "/", "bulk_b.txt", "/", "bulk_c.txt").expect("renameat");
        fs::unlinkat(fs_id, "/", "bulk_c.txt").expect("unlinkat");

        fs::close(fd3).expect("close fd3");
        fs::close(fd2).expect("close fd2");
        fs::close(fd).expect("close fd");
        fs::unlink(fs_id, "/bulk_a.txt").expect("unlink a");
        fs::unmount(fs_id).expect("unmount");
    }

    #[test_case]
    fn posix_fs_at_and_time_apis_work() {
        let fs_id = fs::mount_ramfs("/posix_at").expect("mount");
        let fd = fs::openat(fs_id, "/", "at_f.txt", true).expect("openat create");
        fs::write(fd, b"hello").expect("write");

        assert!(fs::faccessat(fs_id, "/", "at_f.txt").expect("faccessat"));
        let st = fs::fstatat(fs_id, "/", "at_f.txt", true).expect("fstatat");
        assert_eq!(st.size, 5);

        fs::symlinkat(fs_id, "/at_f.txt", "/", "ln.txt").expect("symlinkat");
        let link_target = fs::readlinkat(fs_id, "/", "ln.txt").expect("readlinkat");
        assert_eq!(link_target, "/at_f.txt");
        fs::linkat(fs_id, "/", "at_f.txt", "/", "hard.txt").expect("linkat");

        let atime = PosixTimespec { sec: 123, nsec: 0 };
        let mtime = PosixTimespec { sec: 456, nsec: 0 };
        fs::utimes(fs_id, "/at_f.txt", atime, mtime).expect("utimes");
        fs::futimes(fd, atime, mtime).expect("futimes");
        fs::futimens(fd, atime, mtime).expect("futimens");
        fs::mkdirat(fs_id, "/", "sub", 0o755).expect("mkdirat");
        fs::rmdir(fs_id, "/sub").expect("rmdir sub");

        fs::close(fd).expect("close");
        fs::unlink(fs_id, "/hard.txt").expect("unlink hard");
        fs::unlink(fs_id, "/ln.txt").expect("unlink ln");
        fs::unlink(fs_id, "/at_f.txt").expect("unlink f");
        fs::unmount(fs_id).expect("unmount");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_socketpair_sendmsg_recvmsg_flow() {
        let (fd_a, fd_b) = super::net::socketpair().expect("socketpair");
        let sent = super::net::sendmsg(fd_a, &[b"hello"]).expect("sendmsg");
        assert_eq!(sent, 5);

        let mut p1 = [0u8; 2];
        let mut p2 = [0u8; 4];
        {
            let mut iov = [&mut p1[..], &mut p2[..]];
            let recv = super::net::recvmsg(fd_b, &mut iov).expect("recvmsg");
            assert_eq!(recv, 5);
        }
        assert_eq!(&p1, b"he");
        assert_eq!(&p2[..3], b"llo");

        super::net::close(fd_a).expect("close socketpair a");
        super::net::close(fd_b).expect("close socketpair b");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_socket_raw_nonblock_flag_applies() {
        let fd = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_INET,
            crate::modules::posix_consts::net::SOCK_DGRAM | crate::modules::posix_consts::net::SOCK_NONBLOCK,
            0,
        )
        .expect("socket raw nonblock");

        let got_flags = crate::modules::libnet::posix_fcntl_getfl_errno(fd).expect("fcntl getfl");
        assert!(got_flags.contains(crate::modules::libnet::PosixFdFlags::NONBLOCK));

        super::net::close(fd).expect("close socket raw");

        let udp_fd = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_INET,
            crate::modules::posix_consts::net::SOCK_DGRAM,
            crate::modules::posix_consts::net::IPPROTO_UDP,
        )
        .expect("socket raw udp protocol");
        super::net::close(udp_fd).expect("close socket udp");

        let tcp_fd = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_INET,
            crate::modules::posix_consts::net::SOCK_STREAM,
            crate::modules::posix_consts::net::IPPROTO_TCP,
        )
        .expect("socket raw tcp protocol");
        super::net::close(tcp_fd).expect("close socket tcp");

        assert_eq!(
            super::net::socket_raw_errno(
                crate::modules::posix_consts::net::AF_INET,
                crate::modules::posix_consts::net::SOCK_STREAM,
                crate::modules::posix_consts::net::IPPROTO_UDP,
            ),
            Err(super::PosixErrno::Invalid)
        );
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_recvmsg_waitall_fills_iov() {
        let (fd_a, fd_b) = super::net::socketpair().expect("socketpair");
        let sent = super::net::sendmsg(fd_a, &[b"abc", b"def"]).expect("sendmsg");
        assert_eq!(sent, 6);

        let mut p1 = [0u8; 2];
        let mut p2 = [0u8; 4];
        {
            let mut iov = [&mut p1[..], &mut p2[..]];
            let recv = super::net::recvmsg_flags(
                fd_b,
                &mut iov,
                crate::modules::libnet::PosixMsgFlags::WAITALL,
            )
            .expect("recvmsg waitall");
            assert_eq!(recv, 6);
        }
        assert_eq!(&p1, b"ab");
        assert_eq!(&p2, b"cdef");

        super::net::close(fd_a).expect("close socketpair a");
        super::net::close(fd_b).expect("close socketpair b");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_unix_socketpair_raw_nonblock() {
        let (fd_a, fd_b) = super::net::socketpair_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM | crate::modules::posix_consts::net::SOCK_NONBLOCK,
            0,
        )
        .expect("socketpair raw unix nonblock");

        let flags_a = crate::modules::libnet::posix_fcntl_getfl_errno(fd_a).expect("fcntl a");
        let flags_b = crate::modules::libnet::posix_fcntl_getfl_errno(fd_b).expect("fcntl b");
        assert!(flags_a.contains(crate::modules::libnet::PosixFdFlags::NONBLOCK));
        assert!(flags_b.contains(crate::modules::libnet::PosixFdFlags::NONBLOCK));

        super::net::close(fd_a).expect("close a");
        super::net::close(fd_b).expect("close b");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_sendmmsg_recvmmsg_flow() {
        let (fd_a, fd_b) = super::net::socketpair().expect("socketpair");

        let msgs = [b"one".as_slice(), b"two".as_slice(), b"three".as_slice()];
        let sent_msgs = super::net::sendmmsg(fd_a, &msgs, crate::modules::libnet::PosixMsgFlags::empty())
            .expect("sendmmsg");
        assert_eq!(sent_msgs, 3);

        let mut b1 = [0u8; 8];
        let mut b2 = [0u8; 8];
        let mut b3 = [0u8; 8];
        let mut out = [&mut b1[..], &mut b2[..], &mut b3[..]];
        let recv_msgs = super::net::recvmmsg(fd_b, &mut out, crate::modules::libnet::PosixMsgFlags::empty())
            .expect("recvmmsg");
        assert_eq!(recv_msgs, 3);
        assert_eq!(&b1[..3], b"one");
        assert_eq!(&b2[..3], b"two");
        assert_eq!(&b3[..5], b"three");

        super::net::close(fd_a).expect("close socketpair a");
        super::net::close(fd_b).expect("close socketpair b");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_unix_path_bind_connect_accept_flow() {
        let listener = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("listener socket");
        let client = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("client socket");

        super::net::unix_bind_path(listener, b"/tmp/hyper-unix.sock").expect("unix bind path");
        super::net::unix_listen(listener, 8).expect("unix listen");
        super::net::unix_connect_path(client, b"/tmp/hyper-unix.sock").expect("unix connect path");

        let accepted = super::net::unix_accept(listener).expect("unix accept");
        let sent = super::net::sendmsg(client, &[b"unix-flow"]).expect("sendmsg");
        assert_eq!(sent, 9);

        let mut buf = [0u8; 16];
        let mut iov = [&mut buf[..]];
        let got = super::net::recvmsg(accepted, &mut iov).expect("recvmsg");
        assert_eq!(got, 9);
        assert_eq!(&buf[..9], b"unix-flow");

        super::net::close(accepted).expect("close accepted");
        super::net::close(client).expect("close client");
        super::net::close(listener).expect("close listener");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_async_nonblocking_helpers_work() {
        let (fd_a, fd_b) = super::net::socketpair_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM | crate::modules::posix_consts::net::SOCK_NONBLOCK,
            0,
        )
        .expect("socketpair raw nonblock");

        let mut empty = [0u8; 8];
        let mut empty_iov = [&mut empty[..]];
        let initial = super::net::recvmsg_async(fd_b, &mut empty_iov);
        assert_eq!(initial, Err(super::PosixErrno::Again));

        let sent = super::net::sendmsg_async(fd_a, &[b"async"]).expect("sendmsg async");
        assert_eq!(sent, 5);

        let mut out = [0u8; 8];
        let mut out_iov = [&mut out[..]];
        let got = super::net::recvmsg_async(fd_b, &mut out_iov).expect("recvmsg async");
        assert_eq!(got, 5);
        assert_eq!(&out[..5], b"async");

        super::net::close(fd_a).expect("close a");
        super::net::close(fd_b).expect("close b");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_unix_abstract_namespace_flow() {
        let listener = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("listener socket");
        let client = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("client socket");

        super::net::unix_bind_addr(listener, b"\0hyper-abstract").expect("bind abstract");
        super::net::unix_listen(listener, 4).expect("listen abstract");
        super::net::unix_connect_addr(client, b"\0hyper-abstract").expect("connect abstract");
        let accepted = super::net::unix_accept(listener).expect("accept abstract");

        super::net::sendmsg(client, &[b"abst"]).expect("send");
        let mut out = [0u8; 8];
        let mut iov = [&mut out[..]];
        let n = super::net::recvmsg(accepted, &mut iov).expect("recv");
        assert_eq!(n, 4);
        assert_eq!(&out[..4], b"abst");

        super::net::close(accepted).expect("close accepted");
        super::net::close(client).expect("close client");
        super::net::close(listener).expect("close listener");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_epoll_reports_ready_fds() {
        let (fd_a, fd_b) = super::net::socketpair().expect("socketpair");
        let epfd = super::net::epoll_create1(0).expect("epoll create");
        super::net::epoll_ctl(
            epfd,
            crate::modules::posix_consts::net::EPOLL_CTL_ADD,
            fd_b,
            crate::modules::posix_consts::net::EPOLLIN,
        )
        .expect("epoll ctl add");

        super::net::sendmsg(fd_a, &[b"evt"]).expect("sendmsg");
        let ready = super::net::epoll_wait(epfd, 8, 8).expect("epoll wait");
        assert!(!ready.is_empty());
        assert!(ready.iter().any(|ev| ev.fd == fd_b && (ev.events & crate::modules::posix_consts::net::EPOLLIN) != 0));

        super::net::epoll_close(epfd).expect("epoll close");
        super::net::close(fd_a).expect("close a");
        super::net::close(fd_b).expect("close b");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_await_helpers_work() {
        let (fd_a, fd_b) = super::net::socketpair().expect("socketpair");
        super::net::await_writable(fd_a, 2).expect("await writable");
        super::net::sendmsg(fd_a, &[b"ready"]).expect("send");
        super::net::await_readable(fd_b, 8).expect("await readable");

        let mut out = [0u8; 8];
        let mut iov = [&mut out[..]];
        let n = super::net::recvmsg(fd_b, &mut iov).expect("recv");
        assert_eq!(n, 5);
        assert_eq!(&out[..5], b"ready");

        super::net::close(fd_a).expect("close a");
        super::net::close(fd_b).expect("close b");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_typed_socket_api_works() {
        use crate::modules::posix_consts::net_typed as typed_net;

        let fd = super::net::socket_typed_errno(
            typed_net::AddressFamily::INET,
            typed_net::SocketType::DGRAM.with_flag(typed_net::SocketType::NONBLOCK),
            typed_net::Protocol::DEFAULT,
        )
        .expect("typed socket");

        let flags = crate::modules::libnet::posix_fcntl_getfl_errno(fd).expect("fcntl getfl");
        assert!(flags.contains(crate::modules::libnet::PosixFdFlags::NONBLOCK));

        super::net::close(fd).expect("close");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_sockaddr_un_helpers_work() {
        let listener = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("listener socket");
        let client = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("client socket");

        let addr = super::net::SockAddrUn::from_path(b"/tmp/hyper-sockaddr-un.sock").expect("sockaddr path");
        assert!(!addr.is_abstract());
        super::net::unix_bind_sockaddr(listener, &addr).expect("bind sockaddr");
        super::net::unix_listen(listener, 8).expect("listen");
        super::net::unix_connect_sockaddr(client, &addr).expect("connect sockaddr");

        let accepted = super::net::unix_accept(listener).expect("accept");
        super::net::sendmsg(client, &[b"su"]).expect("sendmsg");
        let mut out = [0u8; 4];
        let mut iov = [&mut out[..]];
        let n = super::net::recvmsg(accepted, &mut iov).expect("recvmsg");
        assert_eq!(n, 2);
        assert_eq!(&out[..2], b"su");

        super::net::close(accepted).expect("close accepted");
        super::net::close(client).expect("close client");
        super::net::close(listener).expect("close listener");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_epoll_edge_triggered_behaves_like_et() {
        let (fd_a, fd_b) = super::net::socketpair().expect("socketpair");
        let epfd = super::net::epoll_create1(0).expect("epoll create");
        super::net::epoll_ctl(
            epfd,
            crate::modules::posix_consts::net::EPOLL_CTL_ADD,
            fd_b,
            crate::modules::posix_consts::net::EPOLLIN | crate::modules::posix_consts::net::EPOLLET,
        )
        .expect("epoll ctl add");

        super::net::sendmsg(fd_a, &[b"edge"]).expect("send");
        let ready1 = super::net::epoll_wait(epfd, 8, 8).expect("epoll wait 1");
        assert!(!ready1.is_empty());

        let ready2 = super::net::epoll_wait(epfd, 8, 2).expect("epoll wait 2");
        assert!(ready2.is_empty());

        let mut out = [0u8; 8];
        let mut iov = [&mut out[..]];
        let n = super::net::recvmsg(fd_b, &mut iov).expect("recv");
        assert_eq!(n, 4);

        super::net::sendmsg(fd_a, &[b"edge2"]).expect("send2");
        let ready3 = super::net::epoll_wait(epfd, 8, 8).expect("epoll wait 3");
        assert!(!ready3.is_empty());

        super::net::epoll_close(epfd).expect("epoll close");
        super::net::close(fd_a).expect("close a");
        super::net::close(fd_b).expect("close b");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_sockaddr_un_raw_roundtrip() {
        let addr = super::net::SockAddrUn::from_path(b"\0hyper-raw").expect("from path");
        let raw = addr.encode_raw();
        let decoded = super::net::SockAddrUn::decode_raw(&raw).expect("decode raw");
        assert!(decoded.is_abstract());
        assert_eq!(decoded.as_path_bytes(), b"\0hyper-raw");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_epoll_pwait_works() {
        let (fd_a, fd_b) = super::net::socketpair().expect("socketpair");
        let epfd = super::net::epoll_create1(0).expect("epoll create");
        super::net::epoll_ctl(
            epfd,
            crate::modules::posix_consts::net::EPOLL_CTL_ADD,
            fd_b,
            crate::modules::posix_consts::net::EPOLLIN,
        )
        .expect("epoll ctl add");

        super::net::sendmsg(fd_a, &[b"pw"]).expect("send");
        let ready = super::net::epoll_pwait(epfd, 4, 8, None).expect("epoll pwait");
        assert!(!ready.is_empty());

        super::net::epoll_close(epfd).expect("epoll close");
        super::net::close(fd_a).expect("close a");
        super::net::close(fd_b).expect("close b");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_epoll_pwait2_works() {
        let (fd_a, fd_b) = super::net::socketpair().expect("socketpair");
        let epfd = super::net::epoll_create1(0).expect("epoll create");
        super::net::epoll_ctl(
            epfd,
            crate::modules::posix_consts::net::EPOLL_CTL_ADD,
            fd_b,
            crate::modules::posix_consts::net::EPOLLIN,
        )
        .expect("epoll ctl add");

        super::net::sendmsg(fd_a, &[b"pw2"]).expect("send");
        let ready = super::net::epoll_pwait2(
            epfd,
            4,
            Some(super::net::EpollTimeout { sec: 0, nsec: 1_000_000 }),
            None,
        )
        .expect("epoll pwait2");
        assert!(!ready.is_empty());

        super::net::epoll_close(epfd).expect("epoll close");
        super::net::close(fd_a).expect("close a");
        super::net::close(fd_b).expect("close b");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_unix_sendto_recvfrom_addr_flow() {
        let rx = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_DGRAM,
            0,
        )
        .expect("rx socket");
        let tx = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_UNIX,
            crate::modules::posix_consts::net::SOCK_DGRAM,
            0,
        )
        .expect("tx socket");

        let rx_addr = super::net::SockAddrUn::from_path(b"/tmp/hyper-unix-rx.sock").expect("rx addr");
        let tx_addr = super::net::SockAddrUn::from_path(b"/tmp/hyper-unix-tx.sock").expect("tx addr");
        super::net::unix_bind_sockaddr(rx, &rx_addr).expect("bind rx");
        super::net::unix_bind_sockaddr(tx, &tx_addr).expect("bind tx");

        let sent = super::net::unix_sendto_sockaddr(tx, &rx_addr, b"unix-dgram").expect("unix sendto");
        assert_eq!(sent, 10);

        let (from, payload) = super::net::unix_recvfrom_addr(rx, crate::modules::libnet::PosixMsgFlags::empty())
            .expect("unix recvfrom addr");
        assert_eq!(from.as_path_bytes(), tx_addr.as_path_bytes());
        assert_eq!(payload, b"unix-dgram");

        super::net::close(tx).expect("close tx");
        super::net::close(rx).expect("close rx");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_epoll_wait_timeout_ms_works() {
        let (fd_a, fd_b) = super::net::socketpair().expect("socketpair");
        let epfd = super::net::epoll_create1(0).expect("epoll create");
        super::net::epoll_ctl(
            epfd,
            crate::modules::posix_consts::net::EPOLL_CTL_ADD,
            fd_b,
            crate::modules::posix_consts::net::EPOLLIN,
        )
        .expect("epoll ctl add");

        super::net::sendmsg(fd_a, &[b"ms"]).expect("send");
        let ready = super::net::epoll_wait_timeout_ms(epfd, 4, 10).expect("epoll wait timeout ms");
        assert!(!ready.is_empty());

        super::net::epoll_close(epfd).expect("epoll close");
        super::net::close(fd_a).expect("close a");
        super::net::close(fd_b).expect("close b");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_accept4_raw_nonblock_works() {
        let server = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_INET,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("server socket");
        crate::modules::libnet::posix_bind_errno(server, crate::modules::libnet::PosixSocketAddrV4::localhost(42151))
            .expect("bind server");
        crate::modules::libnet::posix_listen_errno(server, 8).expect("listen");

        let client = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_INET,
            crate::modules::posix_consts::net::SOCK_STREAM,
            0,
        )
        .expect("client socket");
        crate::modules::libnet::posix_connect_errno(client, crate::modules::libnet::PosixSocketAddrV4::localhost(42151))
            .expect("connect");

        let accepted = super::net::accept4_raw_errno(
            server,
            crate::modules::posix_consts::net::SOCK_NONBLOCK,
        )
        .expect("accept4 raw");
        let flags = crate::modules::libnet::posix_fcntl_getfl_errno(accepted).expect("fcntl getfl");
        assert!(flags.contains(crate::modules::libnet::PosixFdFlags::NONBLOCK));

        super::net::close(accepted).expect("close accepted");
        super::net::close(client).expect("close client");
        super::net::close(server).expect("close server");
    }

    #[cfg(feature = "networking")]
    #[test_case]
    fn posix_network_sockopt_raw_roundtrip_works() {
        let fd = super::net::socket_raw_errno(
            crate::modules::posix_consts::net::AF_INET,
            crate::modules::posix_consts::net::SOCK_DGRAM,
            0,
        )
        .expect("socket");

        super::net::setsockopt_raw(
            fd,
            crate::modules::posix_consts::net::SOL_SOCKET,
            crate::modules::posix_consts::net::SO_REUSEADDR,
            1,
        )
        .expect("setsockopt raw reuseaddr");
        let reuse = super::net::getsockopt_raw(
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
        let recv_retry = super::net::getsockopt_raw(
            fd,
            crate::modules::posix_consts::net::SOL_SOCKET,
            crate::modules::posix_consts::net::SO_RCVTIMEO,
        )
        .expect("getsockopt raw rcvtimeo");
        assert_eq!(recv_retry, 3);

        super::net::close(fd).expect("close");
    }

    #[test_case]
    fn posix_dup2_same_fd_still_validates_oldfd() {
        assert_eq!(fs::dup2(424242, 424242), Err(PosixErrno::BadFileDescriptor));
    }
}
