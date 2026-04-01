use super::super::*;
use alloc::collections::BTreeMap;
use spin::Mutex;
use crate::modules::linux_compat::helpers::*;
use crate::kernel::syscalls::current_process_id;

static SECCOMP_MODE_BY_PID: Mutex<BTreeMap<u32, u8>> = Mutex::new(BTreeMap::new());

const PTRACE_TRACEME: usize = 0;
const PTRACE_ATTACH: usize = 16;
const PTRACE_DETACH: usize = 17;
const PTRACE_CONT: usize = 7;
const PTRACE_SYSCALL: usize = 24;
const PTRACE_INTERRUPT: usize = 0x4207;
const PTRACE_LISTEN: usize = 0x4208;

const SECCOMP_SET_MODE_STRICT: usize = 0;
const SECCOMP_SET_MODE_FILTER: usize = 1;
const SECCOMP_GET_ACTION_AVAIL: usize = 2;
const SECCOMP_GET_NOTIF_SIZES: usize = 3;

const SECCOMP_FILTER_FLAG_TSYNC: usize = 1 << 0;
const SECCOMP_FILTER_FLAG_LOG: usize = 1 << 1;
const SECCOMP_FILTER_FLAG_SPEC_ALLOW: usize = 1 << 2;
const SECCOMP_FILTER_FLAG_NEW_LISTENER: usize = 1 << 3;
const SECCOMP_FILTER_FLAG_TSYNC_ESRCH: usize = 1 << 4;
const SECCOMP_FILTER_ALLOWED_FLAGS: usize = SECCOMP_FILTER_FLAG_TSYNC
    | SECCOMP_FILTER_FLAG_LOG
    | SECCOMP_FILTER_FLAG_SPEC_ALLOW
    | SECCOMP_FILTER_FLAG_NEW_LISTENER
    | SECCOMP_FILTER_FLAG_TSYNC_ESRCH;

const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
const SECCOMP_RET_KILL_THREAD: u32 = 0x0000_0000;
const SECCOMP_RET_TRAP: u32 = 0x0003_0000;
const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;
const SECCOMP_RET_TRACE: u32 = 0x7ff0_0000;
const SECCOMP_RET_LOG: u32 = 0x7ffc_0000;
const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;

const SECCOMP_MODE_STRICT_VALUE: u8 = 1;
const SECCOMP_MODE_FILTER_VALUE: u8 = 2;

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxSeccompNotifSizes {
    seccomp_notif: u16,
    seccomp_notif_resp: u16,
    seccomp_data: u16,
}

#[inline(always)]
fn ptrace_feature_enabled() -> bool {
    cfg!(feature = "linux_compat_trace_seccomp")
        && crate::modules::linux_compat::config::ptrace_compat_enabled()
}

#[inline(always)]
fn seccomp_feature_enabled() -> bool {
    cfg!(feature = "linux_compat_trace_seccomp")
        && crate::modules::linux_compat::config::seccomp_compat_enabled()
}

pub fn sys_linux_ptrace(request: usize, pid: usize, _addr: usize, _data: usize) -> usize {
    if let Err(e) = super::require_control_plane_access(crate::modules::security::RESOURCE_PROCESS_PTRACE) {
        return e;
    }

    if !ptrace_feature_enabled() {
        return linux_inval();
    }

    match request {
        PTRACE_TRACEME => 0,
        PTRACE_ATTACH | PTRACE_DETACH | PTRACE_CONT | PTRACE_SYSCALL | PTRACE_INTERRUPT
        | PTRACE_LISTEN => {
            if pid == 0 {
                linux_errno(crate::modules::posix_consts::errno::ESRCH)
            } else {
                0
            }
        }
        _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
    }
}

pub fn sys_linux_seccomp(operation: usize, flags: usize, args: UserPtr<u8>) -> usize {
    if let Err(e) = super::require_control_plane_access(crate::modules::security::RESOURCE_SECURITY_POLICY) {
        return e;
    }

    if !seccomp_feature_enabled() {
        return linux_inval();
    }

    match operation {
        SECCOMP_SET_MODE_STRICT => {
            if flags != 0 {
                return linux_errno(crate::modules::posix_consts::errno::EINVAL);
            }
            if let Some(pid) = current_process_id() {
                SECCOMP_MODE_BY_PID
                    .lock()
                    .insert(pid as u32, SECCOMP_MODE_STRICT_VALUE);
            }
            0
        }
        SECCOMP_SET_MODE_FILTER => {
            if args.is_null() || (flags & !SECCOMP_FILTER_ALLOWED_FLAGS) != 0 {
                return linux_errno(crate::modules::posix_consts::errno::EINVAL);
            }
            if let Some(pid) = current_process_id() {
                SECCOMP_MODE_BY_PID
                    .lock()
                    .insert(pid as u32, SECCOMP_MODE_FILTER_VALUE);
            }
            0
        }
        SECCOMP_GET_ACTION_AVAIL => {
            if args.is_null() {
                return linux_fault();
            }
            let action = match args.cast::<u32>().read() {
                Ok(v) => v,
                Err(e) => return e,
            };
            match action {
                SECCOMP_RET_KILL_PROCESS
                | SECCOMP_RET_KILL_THREAD
                | SECCOMP_RET_TRAP
                | SECCOMP_RET_ERRNO
                | SECCOMP_RET_TRACE
                | SECCOMP_RET_LOG
                | SECCOMP_RET_ALLOW => 0,
                _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
            }
        }
        SECCOMP_GET_NOTIF_SIZES => {
            if args.is_null() {
                return linux_fault();
            }
            let sizes = LinuxSeccompNotifSizes {
                seccomp_notif: 80,
                seccomp_notif_resp: 24,
                seccomp_data: 64,
            };
            match args.cast::<LinuxSeccompNotifSizes>().write(&sizes) {
                Ok(()) => 0,
                Err(e) => e,
            }
        }
        _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
    }
}
