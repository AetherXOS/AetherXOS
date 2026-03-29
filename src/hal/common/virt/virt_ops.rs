use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestLaunchHook {
    pub action: &'static str,
    pub allowed: bool,
    pub requires_control_plane: bool,
    pub requires_memory_isolation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestRuntimeHook {
    pub action: &'static str,
    pub allowed: bool,
    pub resumable: bool,
    pub snapshot_capable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestExitHook {
    pub action: &'static str,
    pub allowed: bool,
    pub requires_trap_handling: bool,
    pub requires_time_virtualization: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestOperationHooks {
    pub launch: GuestLaunchHook,
    pub runtime: GuestRuntimeHook,
    pub exit: GuestExitHook,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestOperationPlan {
    pub next_action: &'static str,
    pub launch_allowed: bool,
    pub runtime_allowed: bool,
    pub exit_allowed: bool,
    pub resume_allowed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestResumeIntent {
    pub action: &'static str,
    pub allowed: bool,
    pub requires_snapshot_capable_state: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestEntryIntent {
    pub action: &'static str,
    pub allowed: bool,
    pub requires_memory_isolation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestTrapIntent {
    pub action: &'static str,
    pub allowed: bool,
    pub requires_time_virtualization: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestOperationDecision {
    pub next_action: &'static str,
    pub launch_action: &'static str,
    pub runtime_action: &'static str,
    pub exit_action: &'static str,
    pub resume_allowed: bool,
    pub entry_allowed: bool,
    pub trap_allowed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestRuntimeOperation {
    pub phase: &'static str,
    pub action: &'static str,
    pub allowed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestOperationBundle {
    pub hooks: GuestOperationHooks,
    pub plan: GuestOperationPlan,
    pub decision: GuestOperationDecision,
    pub resume_intent: GuestResumeIntent,
    pub entry_intent: GuestEntryIntent,
    pub trap_intent: GuestTrapIntent,
    pub entry_operation: GuestRuntimeOperation,
    pub resume_operation: GuestRuntimeOperation,
    pub trap_operation: GuestRuntimeOperation,
}

#[inline(always)]
pub fn guest_launch_hook(profile: GuestOperationProfile) -> GuestLaunchHook {
    GuestLaunchHook {
        action: profile.launch_stage,
        allowed: profile.guest_entry_ready
            && crate::config::KernelConfig::virtualization_entry_enabled(),
        requires_control_plane: !profile.control_ready,
        requires_memory_isolation: !profile.memory_isolation_ready,
    }
}

#[inline(always)]
pub fn guest_next_runtime_action(hooks: GuestOperationHooks) -> &'static str {
    if hooks.runtime.allowed && hooks.runtime.resumable {
        hooks.runtime.action
    } else if hooks.launch.allowed {
        hooks.launch.action
    } else {
        hooks.exit.action
    }
}

#[inline(always)]
pub fn guest_runtime_hook(profile: GuestOperationProfile) -> GuestRuntimeHook {
    GuestRuntimeHook {
        action: profile.runtime_stage,
        allowed: profile.control_ready
            && profile.trap_ready
            && crate::config::KernelConfig::virtualization_resume_enabled(),
        resumable: profile.resume_ready
            && crate::config::KernelConfig::virtualization_resume_enabled(),
        snapshot_capable: profile.snapshot_ready
            && crate::config::KernelConfig::virtualization_snapshot_enabled(),
    }
}

#[inline(always)]
pub fn guest_exit_hook(profile: GuestOperationProfile) -> GuestExitHook {
    GuestExitHook {
        action: profile.exit_stage,
        allowed: profile.trap_ready
            && crate::config::KernelConfig::virtualization_trap_dispatch_enabled(),
        requires_trap_handling: !profile.trap_ready,
        requires_time_virtualization:
            crate::config::KernelConfig::virtualization_trap_tracing_enabled()
                && guest_exit_requires_time_virtualization(profile.exit_stage),
    }
}

#[inline(always)]
pub fn guest_operation_hooks(profile: GuestOperationProfile) -> GuestOperationHooks {
    GuestOperationHooks {
        launch: guest_launch_hook(profile),
        runtime: guest_runtime_hook(profile),
        exit: guest_exit_hook(profile),
    }
}

#[inline(always)]
pub fn guest_operation_plan(hooks: GuestOperationHooks) -> GuestOperationPlan {
    let next_action = guest_preferred_next_action(
        hooks.launch.allowed,
        hooks.launch.action,
        hooks.runtime.allowed,
        hooks.runtime.action,
        hooks.exit.action,
    );
    GuestOperationPlan {
        next_action,
        launch_allowed: hooks.launch.allowed,
        runtime_allowed: hooks.runtime.allowed,
        exit_allowed: hooks.exit.allowed,
        resume_allowed: hooks.runtime.resumable,
    }
}

#[inline(always)]
pub fn guest_resume_intent(hooks: GuestOperationHooks) -> GuestResumeIntent {
    GuestResumeIntent {
        action: hooks.runtime.action,
        allowed: hooks.runtime.allowed && hooks.runtime.resumable,
        requires_snapshot_capable_state: !hooks.runtime.snapshot_capable,
    }
}

#[inline(always)]
pub fn guest_entry_intent(hooks: GuestOperationHooks) -> GuestEntryIntent {
    GuestEntryIntent {
        action: hooks.launch.action,
        allowed: hooks.launch.allowed,
        requires_memory_isolation: hooks.launch.requires_memory_isolation,
    }
}

#[inline(always)]
pub fn guest_trap_intent(hooks: GuestOperationHooks) -> GuestTrapIntent {
    GuestTrapIntent {
        action: hooks.exit.action,
        allowed: hooks.exit.allowed,
        requires_time_virtualization: hooks.exit.requires_time_virtualization,
    }
}

#[inline(always)]
pub fn guest_operation_decision(hooks: GuestOperationHooks) -> GuestOperationDecision {
    let plan = guest_operation_plan(hooks);
    let resume = guest_resume_intent(hooks);
    let entry = guest_entry_intent(hooks);
    let trap = guest_trap_intent(hooks);
    GuestOperationDecision {
        next_action: plan.next_action,
        launch_action: hooks.launch.action,
        runtime_action: hooks.runtime.action,
        exit_action: hooks.exit.action,
        resume_allowed: resume.allowed,
        entry_allowed: entry.allowed,
        trap_allowed: trap.allowed,
    }
}

#[inline(always)]
pub fn guest_operation_bundle(profile: GuestOperationProfile) -> GuestOperationBundle {
    let hooks = guest_operation_hooks(profile);
    let plan = guest_operation_plan(hooks);
    let resume_intent = guest_resume_intent(hooks);
    let entry_intent = guest_entry_intent(hooks);
    let trap_intent = guest_trap_intent(hooks);
    GuestOperationBundle {
        hooks,
        plan,
        decision: guest_operation_decision(hooks),
        resume_intent,
        entry_intent,
        trap_intent,
        entry_operation: guest_entry_operation(entry_intent),
        resume_operation: guest_resume_operation(resume_intent),
        trap_operation: guest_trap_operation(trap_intent),
    }
}

#[inline(always)]
pub fn guest_entry_operation(intent: GuestEntryIntent) -> GuestRuntimeOperation {
    GuestRuntimeOperation {
        phase: "entry",
        action: intent.action,
        allowed: intent.allowed,
    }
}

#[inline(always)]
pub fn guest_resume_operation(intent: GuestResumeIntent) -> GuestRuntimeOperation {
    GuestRuntimeOperation {
        phase: "resume",
        action: intent.action,
        allowed: intent.allowed,
    }
}

#[inline(always)]
pub fn guest_trap_operation(intent: GuestTrapIntent) -> GuestRuntimeOperation {
    GuestRuntimeOperation {
        phase: "trap",
        action: intent.action,
        allowed: intent.allowed,
    }
}
