use super::dispatch_helpers::current_process_id;
use super::syscalls_consts::syscall_name;

#[inline(always)]
pub(crate) fn trace_syscall_start(syscall_id: usize, args: [usize; 6], user_rip: usize) {
    if crate::config::KernelConfig::is_syscall_tracing_enabled() {
        crate::klog_trace!(
            "SYSCALL START: id={} name={} args={:x?} rip={:#x}",
            syscall_id,
            syscall_name(syscall_id),
            args,
            user_rip
        );
    }
}

#[inline(always)]
pub(crate) fn trace_syscall_end(syscall_id: usize, result: usize) {
    if crate::config::KernelConfig::is_syscall_tracing_enabled() {
        crate::klog_trace!(
            "SYSCALL END: id={} name={} result={:#x}",
            syscall_id,
            syscall_name(syscall_id),
            result
        );
    }
}

#[inline(always)]
pub(crate) fn log_unknown_syscall(
    syscall_id: usize,
    args: [usize; 6],
    user_rip: usize,
    user_rflags: usize,
) {
    if crate::config::KernelConfig::is_advanced_debug_enabled() {
        let current_pid = current_process_id().unwrap_or(0);
        crate::klog_warn!(
            "Unknown syscall: id={} name={} pid={} rip={:#x} rflags={:#x} args={:x?}",
            syscall_id,
            syscall_name(syscall_id),
            current_pid,
            user_rip,
            user_rflags,
            args
        );
    } else {
        crate::klog_warn!("Unknown syscall: {} from rip {:#x}", syscall_id, user_rip);
    }
}

#[inline(always)]
pub(crate) fn log_unknown_linux_syscall(
    syscall_id: usize,
    user_rip: usize,
    user_rflags: usize,
    args: [usize; 6],
) {
    if crate::config::KernelConfig::is_advanced_debug_enabled() {
        let current_pid = current_process_id().unwrap_or(0);
        crate::klog_warn!(
            "Unknown linux syscall: nr={} pid={} rip={:#x} rflags={:#x} args={:x?}",
            syscall_id,
            current_pid,
            user_rip,
            user_rflags,
            args
        );
    } else {
        crate::klog_warn!("Unknown linux syscall: {} from rip {:#x}", syscall_id, user_rip);
    }
}