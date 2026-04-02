use crate::interfaces::security::{
    cap_flags, ResourceKind, SecurityAction, SecurityContext, SecurityVerdict,
};
use crate::interfaces::SecurityMonitor;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use spin::Mutex;
use aethercore_common::{counter_inc, declare_counter_u64, telemetry};

use crate::interfaces::task::TaskId;

// ─── Telemetry Counters ─────────────────────────────────────────────
declare_counter_u64!(ACL_GRANT_CALLS);
declare_counter_u64!(ACL_REVOKE_CALLS);
declare_counter_u64!(ACL_CHECK_CALLS);
declare_counter_u64!(ACL_CHECK_HITS);
declare_counter_u64!(ACL_DENY_CALLS);
declare_counter_u64!(ACL_FULL_CHECK_CALLS);

#[derive(Debug, Clone, Copy)]
pub struct AclStats {
    pub grant_calls: u64,
    pub revoke_calls: u64,
    pub check_calls: u64,
    pub check_hits: u64,
    pub deny_calls: u64,
    pub full_check_calls: u64,
}

pub fn stats() -> AclStats {
    AclStats {
        grant_calls: telemetry::snapshot_u64(&ACL_GRANT_CALLS),
        revoke_calls: telemetry::snapshot_u64(&ACL_REVOKE_CALLS),
        check_calls: telemetry::snapshot_u64(&ACL_CHECK_CALLS),
        check_hits: telemetry::snapshot_u64(&ACL_CHECK_HITS),
        deny_calls: telemetry::snapshot_u64(&ACL_DENY_CALLS),
        full_check_calls: telemetry::snapshot_u64(&ACL_FULL_CHECK_CALLS),
    }
}

pub fn take_stats() -> AclStats {
    AclStats {
        grant_calls: telemetry::take_u64(&ACL_GRANT_CALLS),
        revoke_calls: telemetry::take_u64(&ACL_REVOKE_CALLS),
        check_calls: telemetry::take_u64(&ACL_CHECK_CALLS),
        check_hits: telemetry::take_u64(&ACL_CHECK_HITS),
        deny_calls: telemetry::take_u64(&ACL_DENY_CALLS),
        full_check_calls: telemetry::take_u64(&ACL_FULL_CHECK_CALLS),
    }
}

// ─── ACL Entry ──────────────────────────────────────────────────────

/// Permission bits per ACL entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AclPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub admin: bool,
}

impl AclPermissions {
    pub const fn all() -> Self {
        Self {
            read: true,
            write: true,
            execute: true,
            admin: true,
        }
    }
    pub const fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            execute: false,
            admin: false,
        }
    }
    pub const fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            execute: false,
            admin: false,
        }
    }
    pub const fn none() -> Self {
        Self {
            read: false,
            write: false,
            execute: false,
            admin: false,
        }
    }

    fn permits(&self, action: SecurityAction) -> bool {
        match action {
            SecurityAction::Read => self.read,
            SecurityAction::Write => self.write,
            SecurityAction::Execute => self.execute,
            SecurityAction::Admin
            | SecurityAction::Chown
            | SecurityAction::Chmod
            | SecurityAction::SetUid
            | SecurityAction::SetGid
            | SecurityAction::ModuleLoad
            | SecurityAction::Reboot => self.admin,
            SecurityAction::Create
            | SecurityAction::Delete
            | SecurityAction::Mount
            | SecurityAction::Unmount => self.write && self.admin,
            SecurityAction::Signal
            | SecurityAction::IpcSend
            | SecurityAction::IpcRecv
            | SecurityAction::NetBind
            | SecurityAction::NetConnect
            | SecurityAction::PtraceAttach
            | SecurityAction::ModuleUnload
            | SecurityAction::SetTime
            | SecurityAction::RawIo
            | SecurityAction::Capability(_) => self.admin,
        }
    }
}

/// Key for the ACL table: (resource_id, task_id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AclKey {
    resource: u64,
    task: TaskId,
}

/// Access Control List (ACL) Monitor — Production Grade.
///
/// Maps (resource_id, task_id) -> AclPermissions.
/// Supports per-task, per-resource, per-action access checks with audit verdicts.
pub struct AccessControlList {
    entries: Mutex<BTreeMap<AclKey, AclPermissions>>,
    /// Fallback: list of task IDs with blanket access per resource (legacy compat).
    legacy: Mutex<BTreeMap<u64, Vec<TaskId>>>,
}

impl AccessControlList {
    pub const fn new() -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
            legacy: Mutex::new(BTreeMap::new()),
        }
    }

    /// Grant granular permission.
    pub fn grant_permission(&self, resource_id: u64, task_id: TaskId, perms: AclPermissions) {
        counter_inc!(ACL_GRANT_CALLS);
        let key = AclKey {
            resource: resource_id,
            task: task_id,
        };
        self.entries.lock().insert(key, perms);
    }

    /// Revoke granular permission.
    pub fn revoke_permission(&self, resource_id: u64, task_id: TaskId) -> bool {
        counter_inc!(ACL_REVOKE_CALLS);
        let key = AclKey {
            resource: resource_id,
            task: task_id,
        };
        self.entries.lock().remove(&key).is_some()
    }

    /// Legacy grant (blanket access).
    pub fn grant_access(&self, resource_id: u64, task_id: TaskId) {
        counter_inc!(ACL_GRANT_CALLS);
        let mut legacy = self.legacy.lock();
        let allowed = legacy.entry(resource_id).or_insert(Vec::new());
        if !allowed.contains(&task_id) {
            allowed.push(task_id);
        }
    }

    /// Legacy revoke.
    pub fn revoke_access(&self, resource_id: u64, task_id: TaskId) -> bool {
        counter_inc!(ACL_REVOKE_CALLS);
        let mut legacy = self.legacy.lock();
        let Some(allowed) = legacy.get_mut(&resource_id) else {
            return false;
        };
        if let Some(pos) = allowed.iter().position(|id| *id == task_id) {
            allowed.remove(pos);
            if allowed.is_empty() {
                legacy.remove(&resource_id);
            }
            true
        } else {
            false
        }
    }
}

impl SecurityMonitor for AccessControlList {
    fn check_access(&self, resource_id: u64) -> bool {
        counter_inc!(ACL_CHECK_CALLS);

        let current_tid = unsafe {
            crate::kernel::cpu_local::CpuLocal::try_get()
                .map(|cpu| TaskId(cpu.current_task.load(Ordering::Relaxed)))
        };

        let Some(tid) = current_tid else {
            return true; // Early boot — no task running
        };

        // Check granular entries first
        let key = AclKey {
            resource: resource_id,
            task: tid,
        };
        if let Some(perms) = self.entries.lock().get(&key) {
            if perms.read || perms.write || perms.execute || perms.admin {
                counter_inc!(ACL_CHECK_HITS);
                return true;
            }
        }

        // Fall back to legacy list
        if let Some(allowed) = self.legacy.lock().get(&resource_id) {
            let ok = allowed.contains(&tid) || allowed.contains(&TaskId(0));
            if ok {
                counter_inc!(ACL_CHECK_HITS);
            }
            return ok;
        }
        counter_inc!(ACL_DENY_CALLS);
        false
    }

    fn check_access_full(
        &self,
        ctx: &SecurityContext,
        resource_id: u64,
        _resource_kind: ResourceKind,
        action: SecurityAction,
    ) -> SecurityVerdict {
        counter_inc!(ACL_FULL_CHECK_CALLS);

        // Root bypass
        if ctx.is_root() || ctx.privileged {
            counter_inc!(ACL_CHECK_HITS);
            return if ctx.audit_enabled {
                SecurityVerdict::AuditAllow
            } else {
                SecurityVerdict::Allow
            };
        }

        // DAC override capability
        if ctx.has_capability(cap_flags::CAP_DAC_OVERRIDE) {
            counter_inc!(ACL_CHECK_HITS);
            return SecurityVerdict::AuditAllow;
        }

        let key = AclKey {
            resource: resource_id,
            task: ctx.task_id,
        };
        if let Some(perms) = self.entries.lock().get(&key) {
            if perms.permits(action) {
                counter_inc!(ACL_CHECK_HITS);
                return SecurityVerdict::Allow;
            }
        }

        // Legacy fallback
        if let Some(allowed) = self.legacy.lock().get(&resource_id) {
            if allowed.contains(&ctx.task_id) || allowed.contains(&TaskId(0)) {
                ACL_CHECK_HITS.fetch_add(1, Ordering::Relaxed);
                return SecurityVerdict::Allow;
            }
        }

        counter_inc!(ACL_DENY_CALLS);
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
        // Only admin or root can grant
        if !ctx.is_root() && !ctx.has_capability(cap_flags::CAP_SYS_ADMIN) {
            return false;
        }
        let perms = match action {
            SecurityAction::Read => AclPermissions::read_only(),
            SecurityAction::Write => AclPermissions::read_write(),
            SecurityAction::Admin => AclPermissions::all(),
            _ => AclPermissions {
                read: false,
                write: false,
                execute: false,
                admin: false,
            },
        };
        self.grant_permission(resource_id, ctx.task_id, perms);
        true
    }

    fn revoke(
        &self,
        ctx: &SecurityContext,
        resource_id: u64,
        _resource_kind: ResourceKind,
        _action: SecurityAction,
    ) -> bool {
        if !ctx.is_root() && !ctx.has_capability(cap_flags::CAP_SYS_ADMIN) {
            return false;
        }
        self.revoke_permission(resource_id, ctx.task_id)
    }

    fn policy_name(&self) -> &'static str {
        "AccessControlList"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn acl_grant_and_check_supervisor_access() {
        let acl = AccessControlList::new();
        acl.grant_access(10, TaskId(0));
        assert!(acl.check_access(10));
    }

    #[test_case]
    fn acl_revoke_removes_access() {
        let acl = AccessControlList::new();
        let tid = TaskId(0);
        acl.grant_access(11, tid);
        assert!(acl.revoke_access(11, tid));
        assert!(!acl.check_access(11));
    }

    #[test_case]
    fn acl_deduplicates_grants() {
        let acl = AccessControlList::new();
        let tid = TaskId(0);
        acl.grant_access(12, tid);
        acl.grant_access(12, tid);
        assert!(acl.revoke_access(12, tid));
        assert!(!acl.revoke_access(12, tid));
    }

    #[test_case]
    fn acl_granular_permission_check() {
        let acl = AccessControlList::new();
        let perms = AclPermissions::read_only();
        acl.grant_permission(100, TaskId(5), perms);

        let ctx =
            SecurityContext::user(TaskId(5), crate::interfaces::task::ProcessId(1), 1000, 1000);
        let verdict = acl.check_access_full(&ctx, 100, ResourceKind::File, SecurityAction::Read);
        assert!(verdict.is_allowed());

        let verdict = acl.check_access_full(&ctx, 100, ResourceKind::File, SecurityAction::Write);
        assert!(!verdict.is_allowed());
    }

    #[test_case]
    fn acl_root_bypass() {
        let acl = AccessControlList::new();
        let ctx = SecurityContext::kernel();
        let verdict = acl.check_access_full(&ctx, 999, ResourceKind::File, SecurityAction::Admin);
        assert!(verdict.is_allowed());
    }
}
