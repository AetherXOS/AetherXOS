//! Audit Tracing for Sensitive Syscalls
//!
//! Provides capability/audit trace IDs for sensitive operations like
//! mounts, namespace transitions, and credential changes.

use core::sync::atomic::{AtomicU64, Ordering};

static NEXT_AUDIT_ID: AtomicU64 = AtomicU64::new(1);

/// A unique ID representing a sensitive kernel operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditTraceId(pub u64);

impl AuditTraceId {
    /// Generate a new unique audit trace ID
    pub fn new() -> Self {
        AuditTraceId(NEXT_AUDIT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Log a sensitive operation with its audit trace ID
pub fn audit_log(id: AuditTraceId, subsystem: &str, action: &str, details: &str) {
    crate::klog_info!(
        "[AUDIT:{}] SUBSYS={} ACTION={} DETAILS={}",
        id.0,
        subsystem,
        action,
        details
    );
}
