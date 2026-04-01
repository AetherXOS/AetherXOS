use super::*;

mod advanced;
pub mod fs;
pub mod ipc;
pub mod net;
pub mod process;
pub mod sync;

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

    linux_trace!(
        "[SYSCALL] nr={}, a1={:#x}, a2={:#x}, a3={:#x}, a4={:#x}, a5={:#x}, a6={:#x}\n",
        nr,
        f.a1,
        f.a2,
        f.a3,
        f.a4,
        f.a5,
        f.a6
    );

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
/// Current dispatcher uses direct match routing, so this is a no-op.
pub fn init_dispatch_index() {}
const LINUX_LEGACY_IPC_NR: usize = 117;
