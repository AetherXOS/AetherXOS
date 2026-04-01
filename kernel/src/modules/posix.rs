macro_rules! define_errno {
    ($($variant:ident = $const:path,)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum PosixErrno {
            $($variant,)*
            Other,
        }
        impl PosixErrno {
            pub const fn code(self) -> i32 {
                match self {
                    $(Self::$variant => $const,)*
                    Self::Other => crate::modules::posix_consts::errno::EIO,
                }
            }
            pub const fn from_code(code: i32) -> Self {
                match code {
                    $($const => Self::$variant,)*
                    _ => Self::Other,
                }
            }
        }
    }
}

define_errno! {
    Again = crate::modules::posix_consts::errno::EAGAIN,
    BadFileDescriptor = crate::modules::posix_consts::errno::EBADF,
    Invalid = crate::modules::posix_consts::errno::EINVAL,
    NotConnected = crate::modules::posix_consts::errno::ENOTCONN,
    AddrInUse = crate::modules::posix_consts::errno::EADDRINUSE,
    TimedOut = crate::modules::posix_consts::errno::ETIMEDOUT,
    NotSupported = crate::modules::posix_consts::errno::EOPNOTSUPP,
    PermissionDenied = crate::modules::posix_consts::errno::EACCES,
    NoEntry = crate::modules::posix_consts::errno::ENOENT,
    AlreadyExists = crate::modules::posix_consts::errno::EEXIST,
    NoSys = crate::modules::posix_consts::errno::ENOSYS,
    TooManyFiles = crate::modules::posix_consts::errno::EMFILE,
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
