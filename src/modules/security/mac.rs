use crate::interfaces::security::{
    SecurityAction, SecurityContext, SecurityLevel, SecurityVerdict,
};
use crate::interfaces::task::TaskId;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

// ─── Telemetry ──────────────────────────────────────────────────────
static MAC_SET_LABEL_CALLS: AtomicU64 = AtomicU64::new(0);
static MAC_SET_CLEARANCE_CALLS: AtomicU64 = AtomicU64::new(0);
static MAC_CHECK_CALLS: AtomicU64 = AtomicU64::new(0);
static MAC_CHECK_HITS: AtomicU64 = AtomicU64::new(0);
static MAC_CHECK_DENIED: AtomicU64 = AtomicU64::new(0);

/// Legacy label enum (kept for backward compat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MacLabel {
    Confidential,
    Secret,
    TopSecret,
}

impl MacLabel {
    pub fn to_security_level(self) -> SecurityLevel {
        match self {
            MacLabel::Confidential => SecurityLevel::Confidential,
            MacLabel::Secret => SecurityLevel::Secret,
            MacLabel::TopSecret => SecurityLevel::TopSecret,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MacStats {
    pub set_label_calls: u64,
    pub set_clearance_calls: u64,
    pub check_calls: u64,
    pub check_hits: u64,
    pub check_denied: u64,
    pub resource_label_count: usize,
    pub task_clearance_count: usize,
}

// ─── Per-Resource Labels ────────────────────────────────────────────
lazy_static! {
    static ref RESOURCE_LABELS: Mutex<BTreeMap<u64, SecurityLevel>> = Mutex::new(BTreeMap::new());
    /// Per-task clearance level (TaskId -> SecurityLevel).
    static ref TASK_CLEARANCE: Mutex<BTreeMap<TaskId, SecurityLevel>> = Mutex::new(BTreeMap::new());
    /// Global fallback clearance (used when no per-task clearance is set).
    static ref GLOBAL_CLEARANCE: Mutex<SecurityLevel> = Mutex::new(SecurityLevel::TopSecret);
}

// ─── Public API ─────────────────────────────────────────────────────

/// Set the security label for a resource.
pub fn set_resource_label(resource: u64, label: MacLabel) {
    MAC_SET_LABEL_CALLS.fetch_add(1, Ordering::Relaxed);
    RESOURCE_LABELS
        .lock()
        .insert(resource, label.to_security_level());
}

/// Set the security label for a resource using SecurityLevel directly.
pub fn set_resource_security_level(resource: u64, level: SecurityLevel) {
    MAC_SET_LABEL_CALLS.fetch_add(1, Ordering::Relaxed);
    RESOURCE_LABELS.lock().insert(resource, level);
}

/// Set per-task clearance level.
pub fn set_task_clearance(task_id: TaskId, level: SecurityLevel) {
    MAC_SET_CLEARANCE_CALLS.fetch_add(1, Ordering::Relaxed);
    TASK_CLEARANCE.lock().insert(task_id, level);
}

/// Set the global fallback clearance (legacy API).
pub fn set_subject_clearance(clearance: MacLabel) {
    MAC_SET_CLEARANCE_CALLS.fetch_add(1, Ordering::Relaxed);
    *GLOBAL_CLEARANCE.lock() = clearance.to_security_level();
}

/// Get the effective clearance for a task.
pub fn effective_clearance(task_id: TaskId) -> SecurityLevel {
    let task_map = TASK_CLEARANCE.lock();
    if let Some(level) = task_map.get(&task_id) {
        return *level;
    }
    *GLOBAL_CLEARANCE.lock()
}

/// Check MAC access: does the current task/context have clearance for this resource?
pub fn check_access(resource: u64) -> bool {
    MAC_CHECK_CALLS.fetch_add(1, Ordering::Relaxed);

    let required = RESOURCE_LABELS
        .lock()
        .get(&resource)
        .copied()
        .unwrap_or(SecurityLevel::Unclassified);

    // Try to get the current task's clearance
    let clearance = unsafe {
        crate::kernel::cpu_local::CpuLocal::try_get()
            .map(|cpu| {
                let tid = TaskId(cpu.current_task.load(Ordering::Relaxed));
                effective_clearance(tid)
            })
            .unwrap_or(*GLOBAL_CLEARANCE.lock())
    };

    let ok = clearance >= required;
    if ok {
        MAC_CHECK_HITS.fetch_add(1, Ordering::Relaxed);
    } else {
        MAC_CHECK_DENIED.fetch_add(1, Ordering::Relaxed);
    }
    ok
}

/// Full MAC access check using SecurityContext (preferred API).
pub fn check_access_full(
    ctx: &SecurityContext,
    resource: u64,
    _action: SecurityAction,
) -> SecurityVerdict {
    MAC_CHECK_CALLS.fetch_add(1, Ordering::Relaxed);

    let required = RESOURCE_LABELS
        .lock()
        .get(&resource)
        .copied()
        .unwrap_or(SecurityLevel::Unclassified);

    // Use the security level from the context, or fall back to per-task map
    let clearance = if ctx.security_level != SecurityLevel::Unclassified {
        ctx.security_level
    } else {
        effective_clearance(ctx.task_id)
    };

    // Bell-LaPadula Mandatory Access Control model:
    // - Simple Security Property (no read up): clearance >= resource_level
    // - *-Property (no write down): clearance <= resource_level for writes
    let allowed = match _action {
        // Read: clearance must dominate resource level (no read up)
        SecurityAction::Read => clearance >= required,
        // Write: strict *-property — subject cannot write to lower classification
        // This prevents information leakage from higher to lower levels
        SecurityAction::Write => clearance <= required,
        // Execute: clearance must meet or exceed resource level
        _ => clearance >= required,
    };

    if allowed {
        MAC_CHECK_HITS.fetch_add(1, Ordering::Relaxed);
        if ctx.audit_enabled {
            SecurityVerdict::AuditAllow
        } else {
            SecurityVerdict::Allow
        }
    } else {
        MAC_CHECK_DENIED.fetch_add(1, Ordering::Relaxed);
        if ctx.audit_enabled {
            SecurityVerdict::AuditDeny
        } else {
            SecurityVerdict::Deny
        }
    }
}

pub fn stats() -> MacStats {
    MacStats {
        set_label_calls: MAC_SET_LABEL_CALLS.load(Ordering::Relaxed),
        set_clearance_calls: MAC_SET_CLEARANCE_CALLS.load(Ordering::Relaxed),
        check_calls: MAC_CHECK_CALLS.load(Ordering::Relaxed),
        check_hits: MAC_CHECK_HITS.load(Ordering::Relaxed),
        check_denied: MAC_CHECK_DENIED.load(Ordering::Relaxed),
        resource_label_count: RESOURCE_LABELS.lock().len(),
        task_clearance_count: TASK_CLEARANCE.lock().len(),
    }
}
