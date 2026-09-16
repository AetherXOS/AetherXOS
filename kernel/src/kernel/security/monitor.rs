//! # Safety
//!
//! All `unsafe` blocks in this module are justified by the calling
//! functions which validate addresses, alignment, and invariants beforehand.
//!
//! Capability-Based Security Monitor implementation for AetherXOS.

use crate::interfaces::security::{
    SecurityAction, SecurityContext, SecurityMonitor, SecurityVerdict, 
    ResourceKind, cap_flags
};
use crate::kernel::cpu_local::CpuLocal;

fn security_audit_context() -> (usize, usize) {
    let Some(cpu) = (unsafe { CpuLocal::try_get() }) else {
        return (0, 0);
    };

    let task_id = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);
    let process_id = crate::kernel::launch::process_id_by_task(crate::interfaces::task::TaskId(task_id))
        .map(|pid| pid.0)
        .unwrap_or(0);

    (task_id, process_id)
}

/// The primary security monitor for AetherXOS, enforcing capability-based access control.
pub struct CapabilitySecurityMonitor;

impl SecurityMonitor for CapabilitySecurityMonitor {
    fn policy_name(&self) -> &'static str {
        "Capability-Based Access Control (CBAC)"
    }

    fn check_access(&self, _resource_id: u64) -> bool {
        // Legacy fast-path: assume allowed if no specific block
        true
    }

    fn check_access_full(
        &self,
        ctx: &SecurityContext,
        _resource_id: u64,
        resource_kind: ResourceKind,
        action: SecurityAction,
    ) -> SecurityVerdict {
        // 1. Prevent capability aliasing for indices >= 64
        if let SecurityAction::Capability(c) = action {
            if c >= 64 {
                let (task_id, process_id) = security_audit_context();
                crate::klog_warn!(
                    "security audit deny: policy={} tid={} pid={} action={:?} resource_kind={:?} (invalid capability index >= 64)",
                    self.policy_name(),
                    task_id,
                    process_id,
                    action,
                    resource_kind
                );
                return SecurityVerdict::AuditDeny;
            }
        }

        // 2. Root (EUID 0) bypass for legacy POSIX parity
        if ctx.is_root() {
            return SecurityVerdict::Allow;
        }

        // 2. Check specific capabilities based on the requested action
        let required_cap = match action {
            SecurityAction::Chown => cap_flags::CAP_CHOWN,
            SecurityAction::SetUid => cap_flags::CAP_SETUID,
            SecurityAction::SetGid => cap_flags::CAP_SETGID,
            SecurityAction::NetBind => cap_flags::CAP_NET_BIND,
            SecurityAction::Admin => cap_flags::CAP_SYS_ADMIN,
            SecurityAction::Reboot => cap_flags::CAP_SYS_BOOT,
            SecurityAction::SetTime => cap_flags::CAP_SYS_TIME,
            SecurityAction::ModuleLoad => cap_flags::CAP_SYS_MODULE,
            SecurityAction::RawIo => cap_flags::CAP_SYS_RAWIO,
            SecurityAction::Mount => cap_flags::CAP_MOUNT,
            SecurityAction::Unmount => cap_flags::CAP_UNMOUNT,
            
            // Capability-specific check
            SecurityAction::Capability(c) => 1u64 << (c % 64),
            
            // Default: Allow basic operations for now (Read/Write/Exec)
            // unless we transition to full mandatory access control (MAC).
            _ => 0,
        };

        if required_cap == 0 || ctx.has_capability(required_cap) {
            SecurityVerdict::Allow
        } else {
            let (task_id, process_id) = security_audit_context();
            crate::klog_warn!(
                "security audit deny: policy={} tid={} pid={} action={:?} resource_kind={:?} required_cap={:#x}",
                self.policy_name(),
                task_id,
                process_id,
                action,
                resource_kind,
                required_cap,
            );
            SecurityVerdict::AuditDeny
        }
    }

    fn has_capability(&self, ctx: &SecurityContext, cap: u64) -> bool {
        ctx.has_capability(cap)
    }
}

