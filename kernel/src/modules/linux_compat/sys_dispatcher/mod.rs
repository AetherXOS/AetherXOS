//! sys_dispatcher module.

use super::*;

mod advanced;
pub mod fs;
pub mod ipc;
pub mod net;
pub mod process;
pub mod registration;
pub mod sync;
pub mod table;
pub mod async_ring;

use crate::interfaces::dispatcher::Dispatcher;
use crate::interfaces::KernelResult;
use crate::kernel::syscalls::SyscallFrame;

/// Structure to hold and cast syscall arguments.
/// Improves readability and reduces boilerplate in dispatchers.
pub struct SyscallDispFrame {
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
}

impl SyscallDispFrame {
    pub fn new(
        arg1: usize,
        arg2: usize,
        arg3: usize,
        arg4: usize,
        arg5: usize,
        arg6: usize,
    ) -> Self {
        Self {
            a1: arg1,
            a2: arg2,
            a3: arg3,
            a4: arg4,
            a5: arg5,
            a6: arg6,
        }
    }

    /// Structured dispatch via the O(1) table.
    /// Returns the raw result as isize.
    pub fn dispatch_structured(nr: usize, frame: &mut SyscallFrame) -> isize {
        // Map the register-based frame into a SyscallDispFrame following the syscall ABI:
        // a1 = rdi, a2 = rsi, a3 = rdx, a4 = r10, a5 = r8, a6 = r9
        let mut f = SyscallDispFrame::new(
            frame.rdi as usize,
            frame.rsi as usize,
            frame.rdx as usize,
            frame.r10 as usize,
            frame.r8 as usize,
            frame.r9 as usize,
        );

        if let Some(res) = table::dispatch_table(nr, &mut f, frame) {
            res as isize
        } else {
            -38 // ENOSYS
        }
    }

    #[inline(always)]
    pub fn fd1(&self) -> Fd {
        fd!(self.a1)
    }
    #[inline(always)]
    pub fn fd2(&self) -> Fd {
        fd!(self.a2)
    }
    #[inline(always)]
    pub fn fd3(&self) -> Fd {
        fd!(self.a3)
    }
    #[inline(always)]
    pub fn fd4(&self) -> Fd {
        fd!(self.a4)
    }
    #[inline(always)]
    pub fn fd5(&self) -> Fd {
        fd!(self.a5)
    }

    #[inline(always)]
    pub fn u1<T>(&self) -> UserPtr<T> {
        uptr!(self.a1)
    }
    #[inline(always)]
    pub fn u2<T>(&self) -> UserPtr<T> {
        uptr!(self.a2)
    }
    #[inline(always)]
    pub fn u3<T>(&self) -> UserPtr<T> {
        uptr!(self.a3)
    }
    #[inline(always)]
    pub fn u4<T>(&self) -> UserPtr<T> {
        uptr!(self.a4)
    }
    #[inline(always)]
    pub fn u5<T>(&self) -> UserPtr<T> {
        uptr!(self.a5)
    }
    #[inline(always)]
    pub fn u6<T>(&self) -> UserPtr<T> {
        uptr!(self.a6)
    }
}

/// Main Linux-compatible syscall dispatcher.
/// Leverages specialized sub-dispatchers (FS, Net, Sync, Process).
pub fn sys_linux_compat(
    nr: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    frame: &mut SyscallFrame,
) -> Option<usize> {
    let mut f = SyscallDispFrame::new(a1, a2, a3, a4, a5, a6);

    // 1. FAST PATH: Table-based O(1) dispatch
    if let Some(res) = table::dispatch_table(nr, &mut f, frame) {
        return Some(res);
    }

    // 2. SLOW PATH: Legacy/Categorized dispatching (to be phased out)
    // Seccomp check
    if let Some(res) = crate::modules::linux_compat::seccomp::check_seccomp_policy(nr, &f) {
        return Some(res);
    }

    // Standard-based dispatching
    if let Some(res) = crate::modules::linux_compat::standards::unix::dispatch_unix(nr, &mut f) {
        return Some(res);
    }
    if let Some(res) = crate::modules::linux_compat::standards::posix::dispatch_posix(nr, &mut f) {
        return Some(res);
    }
    if let Some(res) =
        crate::modules::linux_compat::standards::linux::dispatch_linux(nr, &mut f, frame)
    {
        return Some(res);
    }

    // Legacy support (e.g. syscall multiplexers and old semantics)
    if crate::modules::linux_compat::config::LinuxCompatConfig::LEGACY_SUPPORT {
        if nr == LINUX_LEGACY_IPC_NR {
            return Some(
                crate::modules::linux_compat::standards::legacy::sys_linux_ipc(
                    f.a1,
                    f.a2,
                    f.a3,
                    f.a4,
                    f.u5(),
                    f.a6,
                ),
            );
        }
    }

    // Component-based dispatching for remainder (to be gradually phased out or re-categorized)
    if let Some(res) = fs::dispatch_fs(nr, &mut f) {
        return Some(res);
    }
    if let Some(res) = net::dispatch_net(nr, &mut f) {
        return Some(res);
    }
    if let Some(res) = sync::dispatch_sync(nr, &mut f) {
        return Some(res);
    }
    if let Some(res) = ipc::dispatch_ipc(nr, &mut f) {
        return Some(res);
    }
    if let Some(res) = process::dispatch_process(nr, &mut f, frame) {
        return Some(res);
    }
    if let Some(res) =
        advanced::dispatch_linux_advanced_syscall(frame, nr, f.a1, f.a2, f.a3, f.a4, f.a5, f.a6)
    {
        return Some(res);
    }

    None
}

/// Initializes any optional dispatcher indices.
pub fn init_dispatch_index() {
    crate::klog_info!("[SYSCALL] Initializing O(1) dispatch table...");
    registration::populate_table();
    crate::klog_info!("[SYSCALL] O(1) dispatch table ready.");
}
const LINUX_LEGACY_IPC_NR: usize = 117;


