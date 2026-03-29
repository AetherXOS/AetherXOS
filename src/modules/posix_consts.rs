pub mod errno {
    pub const EPERM: i32 = 1;
    pub const ESRCH: i32 = 3;
    pub const ENOMEM: i32 = 12;
    pub const EIO: i32 = 5;
    pub const EBADF: i32 = 9;
    pub const EAGAIN: i32 = 11;
    pub const EACCES: i32 = 13;
    pub const EBUSY: i32 = 16;
    pub const EEXIST: i32 = 17;
    pub const EINVAL: i32 = 22;
    pub const EROFS: i32 = 30;
    pub const ENOSYS: i32 = 38;
    pub const EOPNOTSUPP: i32 = 95;
    pub const EADDRINUSE: i32 = 98;
    pub const ENOTCONN: i32 = 107;
    pub const ETIMEDOUT: i32 = 110;
    pub const EAFNOSUPPORT: i32 = 97;
    pub const EPROTONOSUPPORT: i32 = 93;
    pub const EINPROGRESS: i32 = 115;
    pub const EISCONN: i32 = 106;
    pub const ENOENT: i32 = 2;
    pub const EFAULT: i32 = 14;
    pub const ENOTDIR: i32 = 20;
    pub const ERANGE: i32 = 34;
    pub const EPIPE: i32 = 32;
    pub const EMLINK: i32 = 31;
    pub const ENAMETOOLONG: i32 = 36;
    // -- extended errno set --
    pub const EMFILE: i32 = 24;
    pub const ENFILE: i32 = 23;
    pub const E2BIG: i32 = 7;
    pub const ECHILD: i32 = 10;
    pub const EINTR: i32 = 4;
    pub const ENXIO: i32 = 6;
    pub const ENOSPC: i32 = 28;
    pub const EISDIR: i32 = 21;
    pub const EXDEV: i32 = 18;
    pub const ENOTTY: i32 = 25;
    pub const EDEADLK: i32 = 35;
    pub const ENOTEMPTY: i32 = 39;
    pub const ELOOP: i32 = 40;
    pub const EOVERFLOW: i32 = 75;
    pub const EILSEQ: i32 = 84;
    pub const EDOM: i32 = 33;
    pub const ENOTSUP: i32 = EOPNOTSUPP;
    pub const EWOULDBLOCK: i32 = EAGAIN;
    pub const ESOCKTNOSUPPORT: i32 = 94;
    pub const EPFNOSUPPORT: i32 = 96;
    pub const ESHUTDOWN: i32 = 108;
    pub const ETOOMANYREFS: i32 = 109;
    pub const ECONNRESET: i32 = 104;
    pub const ECONNREFUSED: i32 = 111;
    pub const EHOSTUNREACH: i32 = 113;
    pub const EALREADY: i32 = 114;
    pub const ENOMSG: i32 = 122;
}

pub mod net {
    pub const AF_UNSPEC: i32 = 0;
    pub const AF_UNIX: i32 = 1;
    pub const AF_LOCAL: i32 = AF_UNIX;
    pub const AF_INET: i32 = 2;
    pub const UNIX_PATH_MAX: usize = 108;

    pub const SOCK_STREAM: i32 = 1;
    pub const SOCK_DGRAM: i32 = 2;
    pub const SOCK_TYPE_MASK: i32 = 0x0000_000f;
    pub const SOCK_CLOEXEC: i32 = 0x0008_0000;
    pub const SOCK_NONBLOCK: i32 = O_NONBLOCK as i32;

    pub const SHUT_RD: i32 = 0;
    pub const SHUT_WR: i32 = 1;
    pub const SHUT_RDWR: i32 = 2;

    pub const O_NONBLOCK: u32 = 0x0000_0800;

    pub const MSG_PEEK: u32 = 0x0000_0002;
    pub const MSG_DONTWAIT: u32 = 0x0000_0040;
    pub const MSG_WAITALL: u32 = 0x0000_0100;
    pub const MSG_TRUNC: u32 = 0x0000_0020;
    pub const MSG_NOSIGNAL: u32 = 0x0000_4000;
    pub const MSG_CMSG_CLOEXEC: u32 = 0x4000_0000;

    pub const EPOLLIN: u32 = POLLIN as u32;
    pub const EPOLLOUT: u32 = POLLOUT as u32;
    pub const EPOLLERR: u32 = POLLERR as u32;
    pub const EPOLLHUP: u32 = POLLHUP as u32;
    pub const EPOLLET: u32 = 1u32 << 31;
    pub const EPOLL_CLOEXEC: i32 = SOCK_CLOEXEC;
    pub const EPOLL_CTL_ADD: i32 = 1;
    pub const EPOLL_CTL_DEL: i32 = 2;
    pub const EPOLL_CTL_MOD: i32 = 3;

    pub const POLLIN: u16 = 0x0001;
    pub const POLLOUT: u16 = 0x0004;
    pub const POLLERR: u16 = 0x0008;
    pub const POLLHUP: u16 = 0x0010;
    pub const POLLNVAL: u16 = 0x0020;

    pub const FIONREAD: u64 = 0x541B;

    pub const SOL_SOCKET: i32 = 1;
    pub const IPPROTO_IP: i32 = 0;
    pub const IPPROTO_TCP: i32 = 6;
    pub const IPPROTO_UDP: i32 = 17;

    pub const SO_TYPE: i32 = 3;
    pub const SO_REUSEADDR: i32 = 2;
    pub const SO_ERROR: i32 = 4;
    pub const SO_DOMAIN: i32 = 39;
    pub const SO_RCVTIMEO: i32 = 20;
    pub const SO_SNDTIMEO: i32 = 21;

    pub const SO_HYPER_NONBLOCK: i32 = 0x1001;
    pub const SO_HYPER_RCVTIMEO_RETRIES: i32 = 0x1002;
    pub const SO_HYPER_SNDTIMEO_RETRIES: i32 = 0x1003;
}

pub mod net_typed {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AddressFamily(pub i32);

    impl AddressFamily {
        pub const UNSPEC: Self = Self(crate::modules::posix_consts::net::AF_UNSPEC);
        pub const UNIX: Self = Self(crate::modules::posix_consts::net::AF_UNIX);
        pub const LOCAL: Self = Self(crate::modules::posix_consts::net::AF_LOCAL);
        pub const INET: Self = Self(crate::modules::posix_consts::net::AF_INET);

        pub const fn as_raw(self) -> i32 {
            self.0
        }
    }

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SocketType(pub i32);

    impl SocketType {
        pub const STREAM: Self = Self(crate::modules::posix_consts::net::SOCK_STREAM);
        pub const DGRAM: Self = Self(crate::modules::posix_consts::net::SOCK_DGRAM);
        pub const NONBLOCK: Self = Self(crate::modules::posix_consts::net::SOCK_NONBLOCK);
        pub const CLOEXEC: Self = Self(crate::modules::posix_consts::net::SOCK_CLOEXEC);

        pub const fn as_raw(self) -> i32 {
            self.0
        }

        pub const fn with_flag(self, flag: Self) -> Self {
            Self(self.0 | flag.0)
        }
    }

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Protocol(pub i32);

    impl Protocol {
        pub const DEFAULT: Self = Self(0);
        pub const IP: Self = Self(crate::modules::posix_consts::net::IPPROTO_IP);
        pub const TCP: Self = Self(crate::modules::posix_consts::net::IPPROTO_TCP);
        pub const UDP: Self = Self(crate::modules::posix_consts::net::IPPROTO_UDP);

        pub const fn as_raw(self) -> i32 {
            self.0
        }
    }
}

pub mod fs {
    pub const SEEK_SET: i32 = 0;
    pub const SEEK_CUR: i32 = 1;
    pub const SEEK_END: i32 = 2;
    pub const O_RDONLY: i32 = 0;
    pub const O_WRONLY: i32 = 1;
    pub const O_RDWR: i32 = 2;
    pub const O_CREAT: i32 = 0o100;
    pub const O_EXCL: i32 = 0o200;
    pub const O_APPEND: i32 = 0o2000;
    pub const O_TRUNC: i32 = 0o1000;
    pub const FALLOC_FL_KEEP_SIZE: u32 = 0x01;
    pub const FALLOC_FL_PUNCH_HOLE: u32 = 0x02;
}

pub mod process {
    pub const SIGKILL: i32 = 9;
    pub const SIGTERM: i32 = 15;
    pub const WNOHANG: i32 = 1;
    pub const WUNTRACED: i32 = 2;
    pub const WSTOPPED: i32 = WUNTRACED;
    pub const WEXITED: i32 = 4;
    pub const WCONTINUED: i32 = 8;
    pub const WNOWAIT: i32 = 0x0100_0000;

    pub const RLIMIT_CPU: i32 = 0;
    pub const RLIMIT_FSIZE: i32 = 1;
    pub const RLIMIT_DATA: i32 = 2;
    pub const RLIMIT_STACK: i32 = 3;
    pub const RLIMIT_CORE: i32 = 4;
    pub const RLIMIT_RSS: i32 = 5;
    pub const RLIMIT_NPROC: i32 = 6;
    pub const RLIMIT_NOFILE: i32 = 7;

    pub const P_ALL: i32 = 0;
    pub const P_PID: i32 = 1;
    pub const P_PGID: i32 = 2;

    pub const CLD_EXITED: i32 = 1;
    pub const CLD_KILLED: i32 = 2;
    pub const CLD_DUMPED: i32 = 3;
    pub const CLD_TRAPPED: i32 = 4;
    pub const CLD_STOPPED: i32 = 5;
    pub const CLD_CONTINUED: i32 = 6;

    pub const RUSAGE_SELF: i32 = 0;
    pub const RUSAGE_CHILDREN: i32 = -1;

    pub const SCHED_OTHER: i32 = 0;
    pub const SCHED_FIFO: i32 = 1;
    pub const SCHED_RR: i32 = 2;

    pub const PRIO_PROCESS: i32 = 0;
}

pub mod ipc {
    pub const FUTEX_WAIT: i32 = 0;
    pub const FUTEX_WAKE: i32 = 1;
}

pub mod thread {
    pub const PTHREAD_CREATE_JOINABLE: i32 = 0;
    pub const PTHREAD_CREATE_DETACHED: i32 = 1;
}

pub mod time {
    pub const TIME_UTC: i32 = 1;

    pub const CLOCK_REALTIME: i32 = 0;
    pub const CLOCK_MONOTONIC: i32 = 1;
    pub const CLOCK_REALTIME_COARSE: i32 = 5;
    pub const CLOCK_MONOTONIC_COARSE: i32 = 6;

    pub const TIMER_ABSTIME: i32 = 1;
}

pub mod signal {
    pub const SIGINT: i32 = 2;
    pub const SIGCHLD: i32 = 17;
    pub const SIGUSR1: i32 = 10;
    pub const SIGUSR2: i32 = 12;

    pub const SIG_BLOCK: i32 = 0;
    pub const SIG_UNBLOCK: i32 = 1;
    pub const SIG_SETMASK: i32 = 2;

    pub const SA_RESTART: u32 = 0x1000_0000;
    pub const SA_RESETHAND: u32 = 0x8000_0000;
    pub const SA_NODEFER: u32 = 0x4000_0000;
    pub const SS_DISABLE: i32 = 2;
}

pub mod mman {
    pub const PROT_NONE: u32 = 0;
    pub const PROT_READ: u32 = 1 << 0;
    pub const PROT_WRITE: u32 = 1 << 1;
    pub const PROT_EXEC: u32 = 1 << 2;

    pub const MAP_SHARED: u32 = 1 << 0;
    pub const MAP_PRIVATE: u32 = 1 << 1;
    pub const MAP_ANONYMOUS: u32 = 1 << 5;

    pub const MS_ASYNC: u32 = 1 << 0;
    pub const MS_INVALIDATE: u32 = 1 << 1;
    pub const MS_SYNC: u32 = 1 << 2;

    pub const MCL_CURRENT: u32 = 1 << 0;
    pub const MCL_FUTURE: u32 = 1 << 1;

    pub const MADV_NORMAL: i32 = 0;
    pub const MADV_RANDOM: i32 = 1;
    pub const MADV_SEQUENTIAL: i32 = 2;
    pub const MADV_WILLNEED: i32 = 3;
    pub const MADV_DONTNEED: i32 = 4;
}
