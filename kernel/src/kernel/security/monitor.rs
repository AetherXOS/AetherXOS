//! Capability-Based Security Monitor implementation for AetherXOS.

use crate::interfaces::security::{
    SecurityAction, SecurityContext, SecurityMonitor, SecurityVerdict, 
    ResourceKind, cap_flags
};

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
        _resource_kind: ResourceKind,
        action: SecurityAction,
    ) -> SecurityVerdict {
        // 1. Root (EUID 0) bypass for legacy POSIX parity
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
            SecurityVerdict::AuditDeny
        }
    }

    fn has_capability(&self, ctx: &SecurityContext, cap: u64) -> bool {
        ctx.has_capability(cap)
    }
}
