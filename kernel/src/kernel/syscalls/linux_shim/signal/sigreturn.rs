use super::support::*;

#[cfg(all(not(feature = "linux_compat"), target_arch = "x86_64"))]
pub fn sys_linux_rt_sigreturn_shim(
    frame: &mut crate::kernel::syscalls::SyscallFrame,
) -> usize {
    let uctx = match read_ucontext(frame.rsi as usize) {
        Ok(v) => v,
        Err(_) => match read_ucontext((frame.rsi as usize).saturating_add(8)) {
            Ok(v) => v,
            Err(err) => return err,
        },
    };

    let m = &uctx.mcontext;
    frame.r15 = m.r15;
    frame.r14 = m.r14;
    frame.r13 = m.r13;
    frame.r12 = m.r12;
    frame.rbp = m.rbp;
    frame.rbx = m.rbx;
    frame.rflags = m.eflags;
    frame.rax = m.rax;
    frame.rdx = m.rdx;
    frame.rsi = m.rsi;
    frame.rdi = m.rdi;
    frame.rip = m.rip;

    #[cfg(feature = "posix_signal")]
    {
        let _ = crate::modules::posix::signal::sigprocmask(
            crate::modules::posix::signal::SigmaskHow::SetMask,
            Some(uctx.sigmask),
        );
    }

    frame.rax as usize
}

#[cfg(all(not(feature = "linux_compat"), not(target_arch = "x86_64")))]
pub fn sys_linux_rt_sigreturn_shim(
    _frame: &mut crate::kernel::syscalls::SyscallFrame,
) -> usize {
    linux_errno(crate::modules::posix_consts::errno::EINVAL)
}
