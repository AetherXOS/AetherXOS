use super::super::*;
use super::SyscallDispFrame;
use crate::hal::syscalls_consts::linux_nr;

pub fn dispatch_net(nr: usize, f: &mut SyscallDispFrame) -> Option<usize> {
    match nr {
        linux_nr::ACCEPT => Some(sys_linux_accept(f.fd1(), f.u2(), f.u3())),
        linux_nr::ACCEPT4 => Some(sys_linux_accept4(f.fd1(), f.u2(), f.u3(), f.a4)),
        linux_nr::BIND => Some(sys_linux_bind(f.fd1(), f.u2(), f.a3)),
        linux_nr::CONNECT => Some(sys_linux_connect(f.fd1(), f.u2(), f.a3)),
        linux_nr::GETPEERNAME => Some(sys_linux_getpeername(f.fd1(), f.u2(), f.u3())),
        linux_nr::GETSOCKNAME => Some(sys_linux_getsockname(f.fd1(), f.u2(), f.u3())),
        linux_nr::GETSOCKOPT => Some(sys_linux_getsockopt(f.fd1(), f.a2, f.a3, f.u4(), f.u5())),
        linux_nr::LISTEN => Some(sys_linux_listen(f.fd1(), f.a2)),
        linux_nr::RECVMMSG => Some(sys_linux_recvmmsg(
            f.fd1(),
            f.u2::<u8>().cast(),
            f.a3,
            f.a4,
            f.u5::<u8>().cast(),
        )),
        linux_nr::RECVFROM => Some(sys_linux_recvfrom(
            f.fd1(),
            f.u2(),
            f.a3,
            f.a4,
            f.u5(),
            f.u6(),
        )),
        linux_nr::RECVMSG => Some(sys_linux_recvmsg(f.fd1(), f.u2::<u8>().cast(), f.a3)),
        linux_nr::SENDMMSG => Some(sys_linux_sendmmsg(f.fd1(), f.u2::<u8>().cast(), f.a3, f.a4)),
        linux_nr::SENDMSG => Some(sys_linux_sendmsg(f.fd1(), f.u2::<u8>().cast(), f.a3)),
        linux_nr::SENDTO => Some(sys_linux_sendto(f.fd1(), f.u2(), f.a3, f.a4, f.u5(), f.a6)),
        linux_nr::SETSOCKOPT => Some(sys_linux_setsockopt(f.fd1(), f.a2, f.a3, f.u4(), f.a5)),
        linux_nr::SHUTDOWN => Some(sys_linux_shutdown(f.fd1(), f.a2)),
        linux_nr::SOCKET => Some(sys_linux_socket(f.a1, f.a2, f.a3)),
        linux_nr::SOCKETPAIR => Some(sys_linux_socketpair(f.a1, f.a2, f.a3, f.u4::<u8>().cast())),

        // ── I/O Multiplexing ────────────────────────────────────────────────
        linux_nr::EPOLL_CREATE => Some(sys_linux_epoll_create(f.a1)),
        linux_nr::EPOLL_PWAIT => Some(sys_linux_epoll_pwait(
            f.fd1(),
            f.u2(),
            f.a3,
            f.a4 as i32,
            f.u5::<u8>().cast(),
            f.a6,
        )),
        linux_nr::EPOLL_PWAIT2 => Some(sys_linux_epoll_pwait2(
            f.fd1(),
            f.u2(),
            f.a3,
            f.u4::<u8>().cast(),
            f.u5::<u8>().cast(),
            f.a6,
        )),
        linux_nr::PPOLL => Some(sys_linux_ppoll(
            f.u1(),
            f.a2,
            f.u3::<u8>().cast(),
            f.u4::<u8>().cast(),
            f.a5,
        )),
        _ => None,
    }
}
