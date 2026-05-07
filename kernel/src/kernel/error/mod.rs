use crate::modules::posix::PosixErrno;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AetherError {
    Posix(PosixErrno),
    Internal(&'static str),
    OutOfMemory,
    NotSupported,
    HardwareError,
}

pub type AetherResult<T> = Result<T, AetherError>;

impl From<PosixErrno> for AetherError {
    fn from(e: PosixErrno) -> Self {
        Self::Posix(e)
    }
}

impl From<&'static str> for AetherError {
    fn from(e: &'static str) -> Self {
        Self::Internal(e)
    }
}
