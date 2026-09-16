//! Unified error hierarchy for the AetherXOS kernel.
//!
//! Provides a structured error system replacing ad-hoc `&'static str`
//! and duplicated error enums across the codebase.

use core::fmt;

// ── Top-level kernel error ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// Invalid argument or parameter.
    InvalidArgument(&'static str),
    /// Resource not found.
    NotFound(&'static str),
    /// Permission denied.
    PermissionDenied(&'static str),
    /// Resource already exists.
    AlreadyExists(&'static str),
    /// Operation not supported.
    Unsupported(&'static str),
    /// Resource exhausted (OOM, etc.).
    ResourceExhausted(&'static str),
    /// I/O error.
    Io(&'static str),
    /// Internal invariant violated.
    Internal(&'static str),
    /// Timed out.
    Timeout(&'static str),
    /// Try again / transient failure.
    WouldBlock,
    /// Wrapped POSIX errno.
    Posix(i32),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(msg) => write!(f, "Invalid argument: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {msg}"),
            Self::AlreadyExists(msg) => write!(f, "Already exists: {msg}"),
            Self::Unsupported(msg) => write!(f, "Unsupported: {msg}"),
            Self::ResourceExhausted(msg) => write!(f, "Resource exhausted: {msg}"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::Internal(msg) => write!(f, "Internal error: {msg}"),
            Self::Timeout(msg) => write!(f, "Timeout: {msg}"),
            Self::WouldBlock => write!(f, "Operation would block"),
            Self::Posix(errno) => write!(f, "POSIX error {errno}"),
        }
    }
}

impl From<&'static str> for KernelError {
    fn from(msg: &'static str) -> Self {
        Self::Internal(msg)
    }
}

/// Convenience alias.
pub type KernelResult<T> = Result<T, KernelError>;

// ── Domain-specific error types (convertible to KernelError) ─────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyscallError {
    BadAddress,
    BadFileDescriptor,
    Interrupted,
    InvalidArgument(&'static str),
    NoMemory,
    NotFound,
    PermissionDenied,
    WouldBlock,
    Unknown(u32),
}

impl From<SyscallError> for KernelError {
    fn from(e: SyscallError) -> Self {
        match e {
            SyscallError::InvalidArgument(m) => Self::InvalidArgument(m),
            SyscallError::NotFound => Self::NotFound("syscall target"),
            SyscallError::PermissionDenied => Self::PermissionDenied("syscall"),
            SyscallError::NoMemory => Self::ResourceExhausted("syscall"),
            SyscallError::WouldBlock => Self::WouldBlock,
            SyscallError::BadAddress => Self::InvalidArgument("bad address"),
            SyscallError::BadFileDescriptor => Self::NotFound("fd"),
            SyscallError::Interrupted => Self::Internal("interrupted"),
            SyscallError::Unknown(n) => Self::Internal("unknown syscall error"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VfsError {
    NotFound,
    AlreadyExists,
    NotADirectory,
    IsADirectory,
    ReadOnlyFilesystem,
    Io(&'static str),
    NoSpace,
    InvalidPath(&'static str),
}

impl From<VfsError> for KernelError {
    fn from(e: VfsError) -> Self {
        match e {
            VfsError::NotFound => Self::NotFound("vfs entry"),
            VfsError::AlreadyExists => Self::AlreadyExists("vfs entry"),
            VfsError::NotADirectory => Self::InvalidArgument("not a directory"),
            VfsError::IsADirectory => Self::InvalidArgument("is a directory"),
            VfsError::ReadOnlyFilesystem => Self::PermissionDenied("read-only fs"),
            VfsError::Io(msg) => Self::Io(msg),
            VfsError::NoSpace => Self::ResourceExhausted("vfs space"),
            VfsError::InvalidPath(msg) => Self::InvalidArgument(msg),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverError {
    NotFound,
    InitFailed(&'static str),
    Io(&'static str),
    Timeout(&'static str),
    Unsupported(&'static str),
}

impl From<DriverError> for KernelError {
    fn from(e: DriverError) -> Self {
        match e {
            DriverError::NotFound => Self::NotFound("driver"),
            DriverError::InitFailed(m) => Self::Internal(m),
            DriverError::Io(m) => Self::Io(m),
            DriverError::Timeout(m) => Self::Timeout(m),
            DriverError::Unsupported(m) => Self::Unsupported(m),
        }
    }
}
