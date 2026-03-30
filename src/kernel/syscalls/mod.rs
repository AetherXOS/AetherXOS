#[cfg(target_arch = "x86_64")]
use crate::interfaces::cpu::CpuRegisters;
use crate::interfaces::Scheduler;
use core::sync::atomic::{AtomicUsize, Ordering};
// use x86_64::registers::control::Cr3;
// Removed x86 specific imports

mod control_plane;
mod core_runtime;
mod dispatch_helpers;
mod ipc_control;
mod linux_dispatch;
#[cfg(not(feature = "linux_compat"))]
mod linux_misc;
#[cfg(not(feature = "linux_compat"))]
mod linux_process;
#[cfg(not(feature = "linux_compat"))]
mod linux_shim;
mod stats_api;
pub mod syscalls_consts;
pub mod syscalls_user;
mod user_access;
mod vfs;

use self::control_plane::*;
pub(crate) use self::core_runtime::*;
pub(crate) use self::dispatch_helpers::*;
#[cfg(test)]
pub(crate) use self::dispatch_helpers::{
    encode_core_pressure_class, encode_scheduler_pressure_class, parse_process_priority,
    upcall_entry_pc_valid, write_core_pressure_snapshot_words, BinarySwitch, CStateOverrideMode,
    PowerOverrideMode,
};
#[cfg(test)]
pub(crate) use self::ipc_control::futex_key_from_ptr_or_hint;
pub(crate) use self::ipc_control::*;
#[cfg(all(test, not(feature = "linux_compat")))]
pub(crate) use self::linux_shim::{
    execve_stack_required_bytes, prepare_execve_user_stack, read_user_c_string_array,
};
pub use self::stats_api::{
    current_syscall_health, evaluate_syscall_health, recommended_syscall_health_action, stats,
    SyscallHealthAction, SyscallHealthReport, SyscallStats,
};
pub(crate) use self::user_access::*;
#[cfg(test)]
pub(crate) use self::user_access::{
    require_control_plane_access, user_access_range_check_with, user_access_range_valid_with,
};
pub use crate::kernel::syscalls::syscalls_consts::*;
#[cfg(test)]
pub(crate) use crate::kernel::syscalls::syscalls_user::{
    user_range_valid, user_word_aligned, UserAccessFault, UserAccessMode,
};
use alloc::collections::BTreeMap;
use lazy_static::lazy_static;

#[cfg(not(target_arch = "x86_64"))]
const LINUX_O_TRUNC: usize = 0o1000;
#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
const LINUX_O_APPEND: usize = 0o2000;
const SYSCALL_AFFINITY_MIGRATE_REQUIRED: usize = 1;
const FUTEX_WORD_BYTES: usize = core::mem::size_of::<u32>();
const FUTEX_WAIT_VALUE_MISMATCH: usize = 1;
const SYSCALL_ERR_INVALID_ARG: usize = !0;
const SYSCALL_ERR_USER_ACCESS_DENIED: usize = !1;
const SYSCALL_ERR_PERMISSION_DENIED: usize = !2;

lazy_static! {
    // tid -> (robust_list_head, robust_list_len)
    static ref ROBUST_LISTS: crate::kernel::sync::IrqSafeMutex<BTreeMap<usize, (usize, usize)>> =
        crate::kernel::sync::IrqSafeMutex::new(BTreeMap::new());
}

pub(crate) fn clear_robust_list_for_tid(tid: usize) {
    ROBUST_LISTS.lock().remove(&tid);
}

pub(crate) fn set_robust_list_for_tid(tid: usize, head: usize, len: usize) {
    ROBUST_LISTS.lock().insert(tid, (head, len));
}

pub(crate) fn robust_list_for_tid(tid: usize) -> Option<(usize, usize)> {
    ROBUST_LISTS.lock().get(&tid).copied()
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(crate) fn linux_seccomp_mode_for_tid(tid: usize) -> u8 {
    linux_misc::linux_prctl_seccomp_mode_for_tid(tid)
}

#[cfg(not(feature = "linux_compat"))]
#[allow(dead_code)]
pub(crate) fn linux_no_new_privs_for_tid(tid: usize) -> bool {
    linux_misc::linux_prctl_no_new_privs_for_tid(tid)
}

#[cfg(all(test, not(feature = "linux_compat")))]
#[allow(dead_code)]
pub(crate) fn linux_set_prctl_state_for_tid_for_test(
    tid: usize,
    seccomp_mode: u8,
    no_new_privs: bool,
) {
    linux_misc::linux_set_prctl_state_for_tid_for_test(tid, seccomp_mode, no_new_privs)
}

#[cfg(feature = "linux_compat")]
pub(crate) fn linux_seccomp_mode_for_tid(_tid: usize) -> u8 {
    0
}

#[cfg(feature = "linux_compat")]
pub(crate) fn linux_no_new_privs_for_tid(_tid: usize) -> bool {
    false
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SyscallReturn {
    ret: usize,
    new_rip: usize,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct SyscallFrame {
    pub rax: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rip: u64,    // Pushed rcx
    pub rflags: u64, // Pushed r11
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
}

static EXECVE_NEW_ENTRY: AtomicUsize = AtomicUsize::new(0);

/// Helper to handle syscalls in Rust
#[no_mangle]
extern "C" fn rust_syscall_handler(
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
    syscall_id: usize,
    user_rip: usize,
    user_rflags: usize,
    frame_ptr: *mut SyscallFrame,
) -> SyscallReturn {
    use core::sync::atomic::Ordering;

    let args = [arg1, arg2, arg3, arg4, arg5, arg6];
    SYSCALL_TOTAL.fetch_add(1, Ordering::Relaxed);

    if crate::config::KernelConfig::is_syscall_tracing_enabled() {
        crate::klog_trace!("SYSCALL START: id={} args={:x?} rip={:#x}", syscall_id, args, user_rip);
    }

    let normal_ret = match syscall_id {
        nr::YIELD => sys_yield(),
        nr::EXIT => sys_exit(arg1),
        nr::PRINT => sys_print(arg1, arg2),
        nr::SET_TLS => sys_set_tls(arg1),
        nr::GET_TLS => sys_get_tls(),
        nr::SET_AFFINITY => sys_set_affinity(arg1),
        nr::GET_AFFINITY => sys_get_affinity(),
        nr::GET_ABI_INFO => sys_get_abi_info(arg1, arg2),
        nr::GET_LAUNCH_STATS => sys_get_launch_stats(arg1, arg2),
        nr::GET_PROCESS_COUNT => sys_get_process_count(),
        nr::LIST_PROCESS_IDS => sys_list_process_ids(arg1, arg2),
        nr::SPAWN_PROCESS => sys_spawn_process(arg1, arg2, arg3, arg4, arg5, arg6),
        nr::GET_PROCESS_IMAGE_STATE => sys_get_process_image_state(arg1, arg2, arg3),
        nr::GET_PROCESS_MAPPING_STATE => sys_get_process_mapping_state(arg1, arg2, arg3),
        nr::VFS_MOUNT_RAMFS => crate::kernel::syscalls::vfs::sys_vfs_mount_ramfs(arg1, arg2),
        nr::VFS_LIST_MOUNTS => crate::kernel::syscalls::vfs::sys_vfs_list_mounts(arg1, arg2),
        nr::GET_POWER_STATS => sys_get_power_stats(arg1, arg2),
        nr::SET_POWER_OVERRIDE => sys_set_power_override(arg1),
        nr::CLEAR_POWER_OVERRIDE => sys_clear_power_override(),
        nr::GET_NETWORK_STATS => sys_get_network_stats(arg1, arg2),
        nr::SET_NETWORK_POLLING => sys_set_network_polling(arg1),
        nr::TERMINATE_PROCESS => sys_terminate_process(arg1),
        nr::GET_PROCESS_LAUNCH_CONTEXT => sys_get_process_launch_context(arg1, arg2, arg3),
        nr::VFS_GET_MOUNT_PATH => {
            crate::kernel::syscalls::vfs::sys_vfs_get_mount_path(arg1, arg2, arg3)
        }
        nr::VFS_UNMOUNT => crate::kernel::syscalls::vfs::sys_vfs_unmount(arg1),
        nr::VFS_GET_STATS => crate::kernel::syscalls::vfs::sys_vfs_get_stats(arg1, arg2),
        nr::NETWORK_RESET_STATS => sys_network_reset_stats(),
        nr::NETWORK_FORCE_POLL => sys_network_force_poll(),
        nr::SET_CSTATE_OVERRIDE => sys_set_cstate_override(arg1),
        nr::CLEAR_CSTATE_OVERRIDE => sys_clear_cstate_override(),
        nr::CLAIM_NEXT_LAUNCH_CONTEXT => sys_claim_next_launch_context(arg1, arg2),
        nr::ACK_LAUNCH_CONTEXT => sys_ack_launch_context(arg1, arg2),
        nr::GET_LAUNCH_CONTEXT_STAGE => sys_get_launch_context_stage(arg1),
        nr::TERMINATE_TASK => sys_terminate_task(arg1),
        nr::GET_PROCESS_ID_BY_TASK => sys_get_process_id_by_task(arg1),
        nr::VFS_UNMOUNT_PATH => crate::kernel::syscalls::vfs::sys_vfs_unmount_path(arg1, arg2),
        nr::NETWORK_REINITIALIZE => sys_network_reinitialize(),
        nr::CONSUME_READY_LAUNCH_CONTEXT => sys_consume_ready_launch_context(arg1, arg2),
        nr::EXECUTE_READY_LAUNCH_CONTEXT => sys_execute_ready_launch_context(),
        nr::FUTEX_WAIT => sys_futex_wait(arg1, arg2, arg3),
        nr::FUTEX_WAKE => sys_futex_wake(arg1, arg2, arg3),
        nr::UPCALL_REGISTER => sys_upcall_register(arg1, arg2, arg3, arg4),
        nr::UPCALL_UNREGISTER => sys_upcall_unregister(arg1),
        nr::UPCALL_QUERY => sys_upcall_query(arg1, arg2, arg3),
        nr::UPCALL_CONSUME => sys_upcall_consume(arg1, arg2),
        nr::UPCALL_INJECT_VIRQ => sys_upcall_inject_virtual_irq(arg1, arg2, arg3),
        nr::SET_NETWORK_BACKPRESSURE_POLICY => {
            sys_set_network_backpressure_policy(arg1, arg2, arg3)
        }
        nr::SET_NETWORK_ALERT_THRESHOLDS => sys_set_network_alert_thresholds(arg1, arg2, arg3),
        nr::GET_NETWORK_ALERT_REPORT => sys_get_network_alert_report(arg1, arg2),
        nr::RESOLVE_PLT => sys_resolve_plt(arg1, arg2),
        nr::VFS_OPEN => {
            SYSCALL_VFS_OPEN_CALLS.fetch_add(1, Ordering::Relaxed);
            crate::kernel::syscalls::vfs::sys_vfs_open(arg1, arg2, arg3)
        }
        nr::VFS_READ => {
            SYSCALL_VFS_READ_CALLS.fetch_add(1, Ordering::Relaxed);
            crate::kernel::syscalls::vfs::sys_vfs_read(arg1, arg2, arg3)
        }
        nr::VFS_WRITE => {
            SYSCALL_VFS_WRITE_CALLS.fetch_add(1, Ordering::Relaxed);
            crate::kernel::syscalls::vfs::sys_vfs_write(arg1, arg2, arg3)
        }
        nr::VFS_CLOSE => {
            SYSCALL_VFS_CLOSE_CALLS.fetch_add(1, Ordering::Relaxed);
            crate::kernel::syscalls::vfs::sys_vfs_close(arg1)
        }
        nr::GET_CRASH_REPORT => sys_get_crash_report(arg1, arg2),
        nr::LIST_CRASH_EVENTS => sys_list_crash_events(arg1, arg2),
        nr::GET_CORE_PRESSURE_SNAPSHOT => sys_get_core_pressure_snapshot(arg1, arg2),
        nr::GET_LOTTERY_REPLAY_LATEST => sys_get_lottery_replay_latest(arg1, arg2),
        nr::SET_POLICY_DRIFT_CONTROL => sys_set_policy_drift_control(arg1, arg2),
        nr::GET_POLICY_DRIFT_CONTROL => sys_get_policy_drift_control(arg1, arg2),
        nr::GET_POLICY_DRIFT_REASON_TEXT => {
            SYSCALL_POLICY_DRIFT_REASON_TEXT_CALLS.fetch_add(1, Ordering::Relaxed);
            sys_get_policy_drift_reason_text(arg1, arg2, arg3)
        }
        _ => {
            // Check for Linux ABI compatibility
            if let Some(ret) = linux_dispatch::dispatch_linux_syscall(
                syscall_id,
                arg1, arg2, arg3, arg4, arg5, arg6,
                user_rip, user_rflags, frame_ptr,
            ) {
                ret
            } else {
                SYSCALL_UNKNOWN.fetch_add(1, Ordering::Relaxed);
                crate::klog_warn!("Unknown syscall: {} from rip {:#x}", syscall_id, user_rip);
                !0
            }
        }
    };

    if crate::config::KernelConfig::is_syscall_tracing_enabled() {
        crate::klog_trace!("SYSCALL END: id={} result={:#x}", syscall_id, normal_ret);
    }

    #[cfg(all(feature = "process_abstraction", feature = "posix_mman"))]
    if let Some(process) = crate::kernel::launch::current_process_arc() {
        let _ = process.refresh_linux_runtime_vvar();
    }

    // Professional Signal Delivery: Check signals before returning to user-space
    crate::kernel::signal::check_signals(unsafe { &mut *frame_ptr });

    let new_rip = EXECVE_NEW_ENTRY.swap(0, Ordering::Relaxed);
    SyscallReturn {
        ret: normal_ret,
        new_rip,
    }
}

#[inline(always)]
#[allow(dead_code)]
pub(super) fn linux_errno(errno: i32) -> usize {
    (-(errno as isize)) as usize
}

#[cfg(test)]
mod tests;
