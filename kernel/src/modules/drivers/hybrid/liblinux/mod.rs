use super::LinuxIoRequestKind;

pub mod bridge;
pub mod conformance;
pub mod telemetry;
pub mod types;

pub use bridge::*;
pub use conformance::*;
pub use telemetry::*;
pub use types::*;

pub fn map_syscall_to_io_kind(syscall: LinuxSyscall) -> LinuxIoRequestKind {
    match syscall {
        LinuxSyscall::Read | LinuxSyscall::RecvMsg | LinuxSyscall::Poll | LinuxSyscall::EpollWait => {
            LinuxIoRequestKind::NetRx
        }
        LinuxSyscall::Write | LinuxSyscall::SendMsg | LinuxSyscall::Fsync => LinuxIoRequestKind::NetTx,
        LinuxSyscall::Ioctl => LinuxIoRequestKind::Control,
        LinuxSyscall::Mmap => LinuxIoRequestKind::BlockRead,
        LinuxSyscall::Munmap => LinuxIoRequestKind::BlockWrite,
        LinuxSyscall::OpenAt | LinuxSyscall::Socket => LinuxIoRequestKind::Control,
    }
}


