use crate::interfaces::security::{
    SecurityAction, SecurityContext, SecurityLevel, SecurityVerdict,
};
use crate::interfaces::task::TaskId;
use alloc::collections::BTreeMap;
use core::sync::atomic::Ordering;
use lazy_static::lazy_static;
use spin::Mutex;
use aethercore_common::{counter_inc, declare_counter_u64, telemetry};

// ─── Telemetry ──────────────────────────────────────────────────────
declare_counter_u64!(MAC_SET_LABEL_CALLS);
declare_counter_u64!(MAC_SET_CLEARANCE_CALLS);
declare_counter_u64!(MAC_CHECK_CALLS);
declare_counter_u64!(MAC_CHECK_HITS);
declare_counter_u64!(MAC_CHECK_DENIED);

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
    counter_inc!(MAC_SET_LABEL_CALLS);
    RESOURCE_LABELS
        .lock()
        .insert(resource, label.to_security_level());
}

/// Set the security label for a resource using SecurityLevel directly.
pub fn set_resource_security_level(resource: u64, level: SecurityLevel) {
    counter_inc!(MAC_SET_LABEL_CALLS);
    RESOURCE_LABELS.lock().insert(resource, level);
}

/// Set per-task clearance level.
pub fn set_task_clearance(task_id: TaskId, level: SecurityLevel) {
    counter_inc!(MAC_SET_CLEARANCE_CALLS);
    TASK_CLEARANCE.lock().insert(task_id, level);
}

/// Set the global fallback clearance (legacy API).
pub fn set_subject_clearance(clearance: MacLabel) {
    counter_inc!(MAC_SET_CLEARANCE_CALLS);
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
    counter_inc!(MAC_CHECK_CALLS);

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
        counter_inc!(MAC_CHECK_HITS);
    } else {
        counter_inc!(MAC_CHECK_DENIED);
    }
    ok
}

/// Full MAC access check using SecurityContext (preferred API).
pub fn check_access_full(
    ctx: &SecurityContext,
    resource: u64,
    _action: SecurityAction,
) -> SecurityVerdict {
    counter_inc!(MAC_CHECK_CALLS);

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
        counter_inc!(MAC_CHECK_HITS);
        if ctx.audit_enabled {
            SecurityVerdict::AuditAllow
        } else {
            SecurityVerdict::Allow
        }
    } else {
        counter_inc!(MAC_CHECK_DENIED);
        if ctx.audit_enabled {
            SecurityVerdict::AuditDeny
        } else {
            SecurityVerdict::Deny
        }
    }
}

pub fn stats() -> MacStats {
    MacStats {
        set_label_calls: telemetry::snapshot_u64(&MAC_SET_LABEL_CALLS),
        set_clearance_calls: telemetry::snapshot_u64(&MAC_SET_CLEARANCE_CALLS),
        check_calls: telemetry::snapshot_u64(&MAC_CHECK_CALLS),
        check_hits: telemetry::snapshot_u64(&MAC_CHECK_HITS),
        check_denied: telemetry::snapshot_u64(&MAC_CHECK_DENIED),
        resource_label_count: RESOURCE_LABELS.lock().len(),
        task_clearance_count: TASK_CLEARANCE.lock().len(),
    }
}

pub fn take_stats() -> MacStats {
    MacStats {
        set_label_calls: telemetry::take_u64(&MAC_SET_LABEL_CALLS),
        set_clearance_calls: telemetry::take_u64(&MAC_SET_CLEARANCE_CALLS),
        check_calls: telemetry::take_u64(&MAC_CHECK_CALLS),
        check_hits: telemetry::take_u64(&MAC_CHECK_HITS),
        check_denied: telemetry::take_u64(&MAC_CHECK_DENIED),
        resource_label_count: RESOURCE_LABELS.lock().len(),
        task_clearance_count: TASK_CLEARANCE.lock().len(),
    }
}
