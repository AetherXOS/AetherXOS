//! Unified Kernel Error Handling System
//! Provides a standard way to represent and propagate failures within the kernel.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    // Basic Errors
    InvalidInput,
    NotFound,
    AlreadyExists,
    PermissionDenied,
    NotSupported,
    NoMemory,
    Busy,
    Timeout,
    Disconnected,

    // File System Errors
    IoError,
    NoSpace,
    IsDirectory,
    NotDirectory,
    BadDescriptor,
    ReadOnlyFileSystem,

    // Process & Task Errors
    NoSuchProcess,
    NoSuchThread,
    ProcessTerminated,
    SignalConflict,

    // Resource Errors
    ResourceExhausted,
    LimitExceeded,

    // Hardware/Platform Errors
    HardwareFailure,
    InterruptError,
    DmaError,

    // Internal
    InternalError,
    Invalid,
    InvalidTask,
    Again,
    Interrupted,
    Overflow,
}

impl KernelError {
    /// Map internal kernel error to a POSIX/Linux errno code.
    pub fn to_posix_errno(&self) -> i32 {
        use crate::modules::posix_consts::errno as e;
        match self {
            Self::InvalidInput => e::EINVAL,
            Self::NotFound => e::ENOENT,
            Self::AlreadyExists => e::EEXIST,
            Self::PermissionDenied => e::EPERM,
            Self::NotSupported => e::ENOSYS,
            Self::NoMemory => e::ENOMEM,
            Self::Busy => e::EBUSY,
            Self::Timeout => e::ETIMEDOUT,
            Self::Disconnected => e::ENOTCONN,
            Self::IoError => e::EIO,
            Self::NoSpace => e::ENOSPC,
            Self::IsDirectory => e::EISDIR,
            Self::NotDirectory => e::ENOTDIR,
            Self::BadDescriptor => e::EBADF,
            Self::ReadOnlyFileSystem => e::EROFS,
            Self::NoSuchProcess => e::ESRCH,
            Self::NoSuchThread => e::ESRCH,
            Self::ProcessTerminated => e::ECHILD,
            Self::ResourceExhausted => e::ENOMEM,
            Self::LimitExceeded => e::EMFILE,
            Self::Invalid => e::EINVAL,
            Self::InvalidTask => e::ESRCH,
            Self::Again => e::EAGAIN,
            _ => e::EFAULT,
        }
    }

    pub fn code(&self) -> i32 {
        self.to_posix_errno()
    }
}

impl From<&'static str> for KernelError {
    fn from(_: &'static str) -> Self {
        Self::InternalError
    }
}

/// Generic Result type for kernel-internal operations.
pub type KernelResult<T = ()> = core::result::Result<T, KernelError>;
