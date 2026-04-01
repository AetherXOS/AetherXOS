use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestExecutionStep {
    pub phase: &'static str,
    pub action: &'static str,
    pub allowed: bool,
    pub blocked_by: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestRuntimeExecution {
    pub next_action: &'static str,
    pub entry: GuestExecutionStep,
    pub resume: GuestExecutionStep,
    pub trap: GuestExecutionStep,
}

#[inline(always)]
pub fn execute_guest_entry(bundle: GuestOperationBundle) -> GuestExecutionStep {
    let policy = crate::config::KernelConfig::virtualization_policy_profile();
    GuestExecutionStep {
        phase: bundle.entry_operation.phase,
        action: bundle.entry_operation.action,
        allowed: bundle.entry_operation.allowed,
        blocked_by: if bundle.entry_operation.allowed {
            None
        } else if !policy.cargo.entry {
            Some(BLOCKED_BY_ENTRY_COMPILETIME)
        } else if !policy.runtime.entry {
            Some(BLOCKED_BY_ENTRY_POLICY)
        } else if bundle.entry_intent.requires_memory_isolation {
            Some("memory-isolation")
        } else if !bundle.hooks.launch.allowed {
            Some("control-plane")
        } else {
            Some("launch-state")
        },
    }
}

#[inline(always)]
pub fn execute_guest_resume(bundle: GuestOperationBundle) -> GuestExecutionStep {
    let policy = crate::config::KernelConfig::virtualization_policy_profile();
    GuestExecutionStep {
        phase: bundle.resume_operation.phase,
        action: bundle.resume_operation.action,
        allowed: bundle.resume_operation.allowed,
        blocked_by: if bundle.resume_operation.allowed {
            None
        } else if !policy.cargo.resume {
            Some(BLOCKED_BY_RESUME_COMPILETIME)
        } else if !policy.runtime.resume {
            Some(BLOCKED_BY_RESUME_POLICY)
        } else if !policy.cargo.snapshot && bundle.resume_intent.requires_snapshot_capable_state {
            Some(BLOCKED_BY_SNAPSHOT_COMPILETIME)
        } else if !policy.runtime.snapshot && bundle.resume_intent.requires_snapshot_capable_state {
            Some(BLOCKED_BY_SNAPSHOT_POLICY)
        } else if bundle.resume_intent.requires_snapshot_capable_state {
            Some("snapshot-state")
        } else {
            Some("runtime-state")
        },
    }
}

#[inline(always)]
pub fn execute_guest_trap(bundle: GuestOperationBundle) -> GuestExecutionStep {
    let policy = crate::config::KernelConfig::virtualization_policy_profile();
    GuestExecutionStep {
        phase: bundle.trap_operation.phase,
        action: bundle.trap_operation.action,
        allowed: bundle.trap_operation.allowed,
        blocked_by: if bundle.trap_operation.allowed {
            None
        } else if !policy.cargo.trap_dispatch {
            Some(BLOCKED_BY_TRAP_DISPATCH_COMPILETIME)
        } else if !policy.runtime.trap_dispatch {
            Some(BLOCKED_BY_TRAP_DISPATCH_POLICY)
        } else if !policy.cargo.trap_tracing
            && guest_exit_requires_time_virtualization(bundle.trap_intent.action)
        {
            Some(BLOCKED_BY_TRAP_TRACING_COMPILETIME)
        } else if !policy.runtime.trap_tracing
            && guest_exit_requires_time_virtualization(bundle.trap_intent.action)
        {
            Some(BLOCKED_BY_TRAP_TRACING_POLICY)
        } else if bundle.trap_intent.requires_time_virtualization {
            Some("time-virtualization")
        } else {
            Some("trap-state")
        },
    }
}

#[inline(always)]
pub fn guest_runtime_execution(bundle: GuestOperationBundle) -> GuestRuntimeExecution {
    GuestRuntimeExecution {
        next_action: bundle.decision.next_action,
        entry: execute_guest_entry(bundle),
        resume: execute_guest_resume(bundle),
        trap: execute_guest_trap(bundle),
    }
}
