use super::super::*;
use crate::hal::syscalls_consts::linux_nr;
use crate::modules::linux_compat::sys_dispatcher::SyscallDispFrame;

/// Dispatcher for fundamental UNIX syscalls (V7/BSD style).
pub fn dispatch_unix(nr: usize, f: &mut SyscallDispFrame) -> Option<usize> {
    match nr {
        linux_nr::READ => Some(sys_linux_read(f.fd1(), f.u2(), f.a3)),
        linux_nr::WRITE => Some(sys_linux_write(f.fd1(), f.u2(), f.a3)),
        linux_nr::OPEN => Some(sys_linux_open(f.u1(), f.a2, f.a3)),
        linux_nr::CLOSE => Some(sys_linux_close(f.fd1())),
        linux_nr::LSEEK => Some(sys_linux_lseek(f.fd1(), f.a2 as i64, f.a3)),
        linux_nr::DUP => Some(sys_linux_dup(f.fd1())),
        linux_nr::DUP2 => Some(sys_linux_dup2(f.fd1(), f.fd2())),
        linux_nr::FORK => Some(sys_linux_fork()),
        linux_nr::VFORK => Some(sys_linux_fork()),
        linux_nr::EXIT => Some(crate::kernel::syscalls::sys_exit(f.a1)),
        linux_nr::CHMOD => Some(sys_linux_chmod(f.u1(), f.a2)),
        linux_nr::CHOWN => Some(sys_linux_chown(f.u1(), f.a2, f.a3)),
        linux_nr::LINK => Some(sys_linux_link(f.u1(), f.u2())),
        linux_nr::UNLINK => Some(sys_linux_unlink(f.u1())),
        _ => None,
    }
}
