use crate::interfaces::security::{ResourceKind, SecurityAction, SecurityContext, SecurityVerdict};
use crate::interfaces::task::TaskId;
use crate::interfaces::SecurityMonitor;
use alloc::collections::BTreeSet;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use aethercore_common::{counter_inc, declare_counter_u64, telemetry};

// ─── Telemetry ──────────────────────────────────────────────────────
declare_counter_u64!(SEL4_CHECK_CALLS);
declare_counter_u64!(SEL4_CHECK_HITS);
declare_counter_u64!(SEL4_CHECK_DENIED);
declare_counter_u64!(SEL4_POLICY_LOADS);
static SEL4_POLICIES_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy)]
pub struct SeL4Stats {
    pub check_calls: u64,
    pub check_hits: u64,
    pub check_denied: u64,
    pub policy_loads: u64,
    pub policies_active: bool,
    pub endpoint_count: usize,
}

/// seL4-style endpoint capability entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct EndpointCap {
    /// Task that holds this capability.
    holder: TaskId,
    /// Resource the capability grants access to.
    resource_id: u64,
    /// Allowed actions bitmask.
    permissions: u64,
}

/// SeL4-Style Security Monitor.
///
/// Strict deny-by-default system modeled after the seL4 microkernel:
/// - All access is denied unless explicitly granted via an endpoint capability.
/// - Capabilities are non-forgeable and managed by the kernel.
/// - Policy proofs can be loaded at boot to establish the initial access matrix.
pub struct SeL4Style {
    endpoints: Mutex<BTreeSet<EndpointCap>>,
}

impl SeL4Style {
    pub const fn new() -> Self {
        Self {
            endpoints: Mutex::new(BTreeSet::new()),
        }
    }

    /// Load a policy: grant an endpoint capability to a task.
    pub fn grant_endpoint(&self, holder: TaskId, resource_id: u64, permissions: u64) {
        counter_inc!(SEL4_POLICY_LOADS);
        SEL4_POLICIES_ACTIVE.store(true, Ordering::Relaxed);

        let cap = EndpointCap {
            holder,
            resource_id,
            permissions,
        };
        self.endpoints.lock().insert(cap);
    }

    /// Revoke an endpoint capability.
    pub fn revoke_endpoint(&self, holder: TaskId, resource_id: u64) -> bool {
        let mut endpoints = self.endpoints.lock();
        let before = endpoints.len();
        endpoints.retain(|cap| !(cap.holder == holder && cap.resource_id == resource_id));
        endpoints.len() < before
    }

    /// Revoke ALL capabilities for a task (e.g., on task termination).
    pub fn revoke_all_for_task(&self, holder: TaskId) {
        self.endpoints.lock().retain(|cap| cap.holder != holder);
    }

    /// Check if a specific endpoint capability exists.
    fn has_endpoint(&self, holder: TaskId, resource_id: u64, required_perm: u64) -> bool {
        let endpoints = self.endpoints.lock();
        endpoints.iter().any(|cap| {
            cap.holder == holder
                && cap.resource_id == resource_id
                && (cap.permissions & required_perm) == required_perm
        })
    }

    pub fn stats(&self) -> SeL4Stats {
        SeL4Stats {
            check_calls: telemetry::snapshot_u64(&SEL4_CHECK_CALLS),
            check_hits: telemetry::snapshot_u64(&SEL4_CHECK_HITS),
            check_denied: telemetry::snapshot_u64(&SEL4_CHECK_DENIED),
            policy_loads: telemetry::snapshot_u64(&SEL4_POLICY_LOADS),
            policies_active: SEL4_POLICIES_ACTIVE.load(Ordering::Relaxed),
            endpoint_count: self.endpoints.lock().len(),
        }
    }
}

impl SecurityMonitor for SeL4Style {
    fn check_access(&self, _resource_id: u64) -> bool {
        counter_inc!(SEL4_CHECK_CALLS);
        // Strict deny-by-default without context
        counter_inc!(SEL4_CHECK_DENIED);
        false
    }

    fn check_access_full(
        &self,
        ctx: &SecurityContext,
        resource_id: u64,
        _resource_kind: ResourceKind,
        action: SecurityAction,
    ) -> SecurityVerdict {
        counter_inc!(SEL4_CHECK_CALLS);

        // Kernel context always allowed (kernel is trusted)
        if ctx.security_level == crate::interfaces::security::SecurityLevel::KernelOnly {
            counter_inc!(SEL4_CHECK_HITS);
            return SecurityVerdict::Allow;
        }

        // Map action to permission bit
        let required_perm = match action {
            SecurityAction::Read => 0x01,
            SecurityAction::Write => 0x02,
            SecurityAction::Execute => 0x04,
            SecurityAction::Create => 0x08,
            SecurityAction::Delete => 0x10,
            SecurityAction::Admin => 0x20,
            SecurityAction::Mount | SecurityAction::Unmount => 0x40,
            SecurityAction::Signal => 0x80,
            SecurityAction::IpcSend | SecurityAction::IpcRecv => 0x100,
            SecurityAction::NetBind | SecurityAction::NetConnect => 0x200,
            _ => 0x20, // Admin for anything else
        };

        if self.has_endpoint(ctx.task_id, resource_id, required_perm) {
            counter_inc!(SEL4_CHECK_HITS);
            return if ctx.audit_enabled {
                SecurityVerdict::AuditAllow
            } else {
                SecurityVerdict::Allow
            };
        }

        // Strict deny
        counter_inc!(SEL4_CHECK_DENIED);
        if ctx.audit_enabled {
            SecurityVerdict::AuditDeny
        } else {
            SecurityVerdict::Deny
        }
    }

    fn grant(
        &self,
        ctx: &SecurityContext,
        resource_id: u64,
        _resource_kind: ResourceKind,
        action: SecurityAction,
    ) -> bool {
        // Only kernel can grant in seL4 model
        if ctx.security_level != crate::interfaces::security::SecurityLevel::KernelOnly {
            return false;
        }
        let perm = match action {
            SecurityAction::Read => 0x01,
            SecurityAction::Write => 0x02,
            SecurityAction::Execute => 0x04,
            SecurityAction::Admin => 0x20,
            _ => 0x20,
        };
        self.grant_endpoint(ctx.task_id, resource_id, perm);
        true
    }

    fn revoke(
        &self,
        ctx: &SecurityContext,
        resource_id: u64,
        _resource_kind: ResourceKind,
        _action: SecurityAction,
    ) -> bool {
        if ctx.security_level != crate::interfaces::security::SecurityLevel::KernelOnly {
            return false;
        }
        self.revoke_endpoint(ctx.task_id, resource_id)
    }

    fn policy_name(&self) -> &'static str {
        "SeL4Style"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::security::SecurityContext;

    #[test_case]
    fn sel4_default_deny() {
        let sel4 = SeL4Style::new();
        assert!(!sel4.check_access(42));
    }

    #[test_case]
    fn sel4_granted_endpoint_allows() {
        let sel4 = SeL4Style::new();
        let tid = TaskId(5);
        sel4.grant_endpoint(tid, 100, 0x01); // Read permission

        let ctx = SecurityContext::user(tid, crate::interfaces::task::ProcessId(1), 1000, 1000);
        let verdict = sel4.check_access_full(&ctx, 100, ResourceKind::File, SecurityAction::Read);
        assert!(verdict.is_allowed());

        // Write should still be denied
        let verdict = sel4.check_access_full(&ctx, 100, ResourceKind::File, SecurityAction::Write);
        assert!(!verdict.is_allowed());
    }

    #[test_case]
    fn sel4_kernel_always_allowed() {
        let sel4 = SeL4Style::new();
        let ctx = SecurityContext::kernel();
        let verdict =
            sel4.check_access_full(&ctx, 999, ResourceKind::Process, SecurityAction::Admin);
        assert!(verdict.is_allowed());
    }

    #[test_case]
    fn sel4_revoke_endpoint() {
        let sel4 = SeL4Style::new();
        let tid = TaskId(10);
        sel4.grant_endpoint(tid, 200, 0xFF);
        assert!(sel4.revoke_endpoint(tid, 200));

        let ctx = SecurityContext::user(tid, crate::interfaces::task::ProcessId(2), 1000, 1000);
        let verdict = sel4.check_access_full(&ctx, 200, ResourceKind::File, SecurityAction::Read);
        assert!(!verdict.is_allowed());
    }
}
