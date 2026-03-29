#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixErrno {
    Again,
    BadFileDescriptor,
    Invalid,
    NotConnected,
    AddrInUse,
    TimedOut,
    NotSupported,
    PermissionDenied,
    NoEntry,
    AlreadyExists,
    NoSys,
    TooManyFiles,
    Other,
}

impl PosixErrno {
    pub const fn code(self) -> i32 {
        match self {
            Self::Again => crate::modules::posix_consts::errno::EAGAIN,
            Self::BadFileDescriptor => crate::modules::posix_consts::errno::EBADF,
            Self::Invalid => crate::modules::posix_consts::errno::EINVAL,
            Self::NotConnected => crate::modules::posix_consts::errno::ENOTCONN,
            Self::AddrInUse => crate::modules::posix_consts::errno::EADDRINUSE,
            Self::TimedOut => crate::modules::posix_consts::errno::ETIMEDOUT,
            Self::NotSupported => crate::modules::posix_consts::errno::EOPNOTSUPP,
            Self::PermissionDenied => crate::modules::posix_consts::errno::EACCES,
            Self::NoEntry => crate::modules::posix_consts::errno::ENOENT,
            Self::AlreadyExists => crate::modules::posix_consts::errno::EEXIST,
            Self::NoSys => crate::modules::posix_consts::errno::ENOSYS,
            Self::TooManyFiles => crate::modules::posix_consts::errno::EMFILE,
            Self::Other => crate::modules::posix_consts::errno::EIO,
        }
    }

    pub const fn from_code(code: i32) -> Self {
        match code {
            crate::modules::posix_consts::errno::EAGAIN => Self::Again,
            crate::modules::posix_consts::errno::EBADF => Self::BadFileDescriptor,
            crate::modules::posix_consts::errno::EINVAL => Self::Invalid,
            crate::modules::posix_consts::errno::ENOTCONN => Self::NotConnected,
            crate::modules::posix_consts::errno::EADDRINUSE => Self::AddrInUse,
            crate::modules::posix_consts::errno::ETIMEDOUT => Self::TimedOut,
            crate::modules::posix_consts::errno::EOPNOTSUPP => Self::NotSupported,
            crate::modules::posix_consts::errno::EACCES => Self::PermissionDenied,
            crate::modules::posix_consts::errno::ENOENT => Self::NoEntry,
            crate::modules::posix_consts::errno::EEXIST => Self::AlreadyExists,
            crate::modules::posix_consts::errno::ENOSYS => Self::NoSys,
            _ => Self::Other,
        }
    }
}

#[cfg(feature = "posix_time")]
#[path = "posix/time.rs"]
pub mod time;

#[cfg(feature = "posix_process")]
#[path = "posix/process.rs"]
pub mod process;

#[cfg(feature = "posix_ipc")]
#[path = "posix/ipc.rs"]
pub mod ipc;

#[cfg(all(feature = "posix_ipc", feature = "posix_thread"))]
#[path = "posix/semaphore.rs"]
pub mod semaphore;

#[cfg(feature = "posix_thread")]
#[path = "posix/thread.rs"]
pub mod thread;

#[cfg(feature = "posix_signal")]
#[path = "posix/signal.rs"]
pub mod signal;

#[cfg(feature = "posix_pipe")]
#[path = "posix/pipe.rs"]
pub mod pipe;

#[cfg(feature = "posix_io")]
#[path = "posix/io.rs"]
pub mod io;

#[cfg(feature = "posix_ipc")]
#[path = "posix/mq.rs"]
pub mod mq;

#[cfg(feature = "posix_fs")]
#[path = "posix/fs.rs"]
pub mod fs;

#[cfg(all(feature = "vfs", feature = "posix_mman"))]
#[path = "posix/mman.rs"]
pub mod mman;

#[cfg(feature = "posix_net")]
#[path = "posix/net.rs"]
pub mod net;

#[cfg(test)]
#[path = "posix/tests_smoke.rs"]
mod tests_smoke;

#[cfg(all(test, feature = "posix_deep_tests"))]
#[path = "posix/tests_deep/mod.rs"]
mod tests_deep;
