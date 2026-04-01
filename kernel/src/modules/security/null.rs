use crate::interfaces::security::{ResourceKind, SecurityAction, SecurityContext, SecurityVerdict};
use crate::interfaces::SecurityMonitor;

/// The Null Monitor.
/// Returns Allow for everything.
/// With #[inline(always)], the compiler will optimize this method call
/// into a literal `true`, and ideally remove the branch instruction entirely.

pub struct NullMonitor;

impl SecurityMonitor for NullMonitor {
    #[inline(always)]
    fn check_access(&self, _resource_id: u64) -> bool {
        true
    }

    #[inline(always)]
    fn check_access_full(
        &self,
        _ctx: &SecurityContext,
        _resource_id: u64,
        _resource_kind: ResourceKind,
        _action: SecurityAction,
    ) -> SecurityVerdict {
        SecurityVerdict::Allow
    }

    fn policy_name(&self) -> &'static str {
        "NullMonitor"
    }
}

impl NullMonitor {
    pub const fn new() -> Self {
        Self
    }
}
