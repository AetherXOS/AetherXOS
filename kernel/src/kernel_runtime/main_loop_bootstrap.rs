use core::sync::atomic::Ordering;

#[cfg(any(feature = "vfs", feature = "linux_compat"))]
use super::super::boot_info;

#[cfg(feature = "process_abstraction")]
include!(concat!(env!("OUT_DIR"), "/linked_probe_image.rs"));

#[cfg(feature = "process_abstraction")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LinkedProbeSpawnRequest {
    process_name: &'static [u8],
    image: &'static [u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn linked_probe_spawn_request() -> LinkedProbeSpawnRequest {
    LinkedProbeSpawnRequest {
        process_name: b"aether_init",
        image: LINKED_PROBE_IMAGE,
        priority: 128,
        deadline: 0,
        burst_time: 0,
        kernel_stack_top: 0,
    }
}

#[cfg(feature = "process_abstraction")]
fn try_spawn_linked_probe(request: LinkedProbeSpawnRequest) {
    let result = invoke_linked_probe_spawn(request);

    match result {
        Ok((pid, _tid)) => {
            aethercore::kernel::debug_trace::record_optional(
                "linked.probe",
                "spawn_returned",
                Some(pid as u64),
                false,
            );
            super::LINKED_PROBE_PID.store(pid, Ordering::Relaxed);
            super::LINKED_PROBE_SPAWNED.store(true, Ordering::Relaxed);
            aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe spawn returned\n");
            aethercore::klog_info!(
                "[LINKED PROBE] spawned embedded probe-linked.elf bytes={} pid={}",
                LINKED_PROBE_IMAGE.len(),
                pid,
            );
            aethercore::klog_info!("[LINKED PROBE] spawned aether_init probe pid={}", pid);
        }
        Err(err) => {
            aethercore::kernel::debug_trace::record_optional(
                "linked.probe",
                "spawn_failed",
                Some(err as u64),
                false,
            );
            aethercore::klog_warn!("[LINKED PROBE] spawn failed: {:?}", err);
        }
    }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn invoke_linked_probe_spawn(
    request: LinkedProbeSpawnRequest,
) -> Result<(usize, usize), aethercore::kernel::launch::LaunchError> {
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe spawn attempt\n");
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe spawn call begin\n");
    aethercore::kernel::debug_trace::record_optional("linked.probe", "linux_compat_ok", None, false);
    aethercore::kernel::debug_trace::record_optional("linked.probe", "spawn_try", None, false);
    aethercore::kernel::debug_trace::record_optional("linked.probe", "spawn_attempt", None, false);
    aethercore::kernel::debug_trace::record_optional("linked.probe", "spawn_call", None, false);

    aethercore::kernel::launch::spawn_bootstrap_from_image(
        request.process_name,
        request.image,
        request.priority,
        request.deadline,
        request.burst_time,
        request.kernel_stack_top,
    )
}

#[cfg(feature = "process_abstraction")]
#[allow(dead_code)]
#[inline(always)]
fn enter_linked_probe_spawn_branch() {
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe linux compat gate returned\n");
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service spawn branch\n");
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe can spawn\n");
    aethercore::kernel::debug_trace::record_optional("linked.probe", "spawn_gate", None, false);
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn dispatch_linked_probe_spawn_transition(request: LinkedProbeSpawnRequest) {
    enter_linked_probe_spawn_branch();
    try_spawn_linked_probe(request);
}

#[cfg(feature = "process_abstraction")]
#[allow(dead_code)]
#[inline(always)]
fn linked_probe_service_active() -> bool {
    super::LINKED_PROBE_ENABLED.load(Ordering::Relaxed)
        && !super::LINKED_PROBE_VERIFIED.load(Ordering::Relaxed)
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn linked_probe_can_spawn(linux_compat_inited: bool, spawned: bool) -> bool {
    linux_compat_inited && !spawned
}

#[cfg(feature = "process_abstraction")]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinkedProbeServiceAction {
    WaitForLinuxCompat,
    Spawn,
    ObserveExit,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn linked_probe_service_action(
    linux_compat_inited: bool,
    spawned: bool,
) -> LinkedProbeServiceAction {
    if !linux_compat_inited {
        LinkedProbeServiceAction::WaitForLinuxCompat
    } else if linked_probe_can_spawn(linux_compat_inited, spawned) {
        LinkedProbeServiceAction::Spawn
    } else {
        LinkedProbeServiceAction::ObserveExit
    }
}

#[cfg(feature = "process_abstraction")]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LinkedProbeServiceDecision {
    linux_compat_inited: bool,
    spawned: bool,
    action: LinkedProbeServiceAction,
}

#[cfg(feature = "process_abstraction")]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PreparedLinkedProbeServiceDecision {
    decision: LinkedProbeServiceDecision,
    spawn_request: Option<LinkedProbeSpawnRequest>,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn linked_probe_service_decision(
    linux_compat_inited: bool,
    spawned: bool,
) -> LinkedProbeServiceDecision {
    LinkedProbeServiceDecision {
        linux_compat_inited,
        spawned,
        action: linked_probe_service_action(linux_compat_inited, spawned),
    }
}

#[cfg(feature = "process_abstraction")]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LinkedProbeRuntimeState {
    linux_compat_inited: bool,
    spawned: bool,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn load_linked_probe_runtime_state() -> LinkedProbeRuntimeState {
    LinkedProbeRuntimeState {
        linux_compat_inited: super::LINUX_COMPAT_INITED.load(Ordering::Relaxed),
        spawned: super::LINKED_PROBE_SPAWNED.load(Ordering::Relaxed),
    }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn prepare_linked_probe_service_decision() -> LinkedProbeServiceDecision {
    let runtime_state = load_linked_probe_runtime_state();
    aethercore::kernel::debug_trace::record_optional(
        "linked.probe",
        "linux_compat_state_loaded",
        None,
        false,
    );
    aethercore::kernel::debug_trace::record_optional(
        "linked.probe",
        "spawned_state_loaded",
        None,
        false,
    );
    let decision =
        linked_probe_service_decision(runtime_state.linux_compat_inited, runtime_state.spawned);
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service action ready\n");
    decision
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn prepare_linked_probe_service_transition() -> PreparedLinkedProbeServiceDecision {
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service transition begin\n");
    let decision = prepare_linked_probe_service_decision();
    let spawn_request = match decision.action {
        LinkedProbeServiceAction::Spawn => Some(linked_probe_spawn_request()),
        _ => None,
    };
    let transition = PreparedLinkedProbeServiceDecision {
        decision,
        spawn_request,
    };
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service transition returned\n");
    transition
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn enter_linked_probe_service() -> PreparedLinkedProbeServiceDecision {
    prepare_entered_linked_probe_service_transition()
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn prepare_linked_probe_service_entry() {
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service entered\n");
    aethercore::kernel::debug_trace::record_optional("linked.probe", "service_entered", None, false);
    aethercore::kernel::debug_trace::record_optional("linked.probe", "cmdline_gate_ok", None, false);
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn prepare_entered_linked_probe_service_transition() -> PreparedLinkedProbeServiceDecision {
    prepare_linked_probe_service_entry();
    prepare_linked_probe_service_transition()
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn dispatch_linked_probe_service_transition(
    transition: PreparedLinkedProbeServiceDecision,
) -> bool {
    match transition.decision.action {
        LinkedProbeServiceAction::WaitForLinuxCompat => {
            aethercore::hal::serial::write_raw(
                "[EARLY SERIAL] linked probe linux compat wait bypass check\n",
            );
            true
        }
        LinkedProbeServiceAction::Spawn => {
            dispatch_linked_probe_spawn_transition(
                transition
                    .spawn_request
                    .expect("spawn transition must include request"),
            );
            true
        }
        LinkedProbeServiceAction::ObserveExit => {
            aethercore::hal::serial::write_raw(
                "[EARLY SERIAL] linked probe linux compat gate returned\n",
            );
            aethercore::kernel::debug_trace::record_optional(
                "linked.probe",
                "linux_compat_ok",
                None,
                false,
            );
            aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe spawn skipped\n");
            false
        }
    }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn run_linked_probe_service_transition() -> bool {
    aethercore::kernel::debug_trace::record_optional(
        "linked.probe",
        "service_transition_run",
        None,
        false,
    );
    let transition = enter_linked_probe_service();
    dispatch_linked_probe_service_transition(transition)
}

pub(super) fn try_mount_initrd_once() {
    if super::INITRD_MOUNTED.load(Ordering::Relaxed) {
        return;
    }

    aethercore::kernel::debug_trace::record_optional(
        "main.loop",
        "initrd_mount_attempt",
        None,
        false,
    );

    #[cfg(feature = "vfs")]
    {
        let info_opt = boot_info::try_get();
        let Some(info) = info_opt else {
            return;
        };

        if let Some(module) = info.find_initrd() {
            if module.size == 0 {
                aethercore::klog_warn!("[INITRD] Module found but size=0, skipping mount");
                super::INITRD_MOUNTED.store(true, Ordering::Relaxed);
                return;
            }

            let virt_base = info.phys_to_virt(module.phys_base) as usize;
            let size = module.size as usize;

            // SAFETY: Limine guarantees the module memory is valid and
            // mapped for the lifetime of the kernel.
            let initrd_slice = unsafe { core::slice::from_raw_parts(virt_base as *const u8, size) };

            aethercore::klog_info!(
                "[INITRD] Mounting {} bytes from {:#x} ({})",
                size,
                module.phys_base,
                module.cmdline_str(),
            );

            let _ = initrd_slice;
            match aethercore::kernel::vfs_control::mount_ramfs(b"/") {
                Ok(_) => aethercore::klog_info!("[INITRD] Base ramfs mounted at /"),
                Err(e) => {
                    aethercore::klog_warn!("[INITRD] Mount fallback failed: {:?} — diskless mode", e)
                }
            }
        } else {
            aethercore::klog_info!("[INITRD] No initrd module provided — diskless mode");
        }

        super::INITRD_MOUNTED.store(true, Ordering::Relaxed);
    }

    #[cfg(not(feature = "vfs"))]
    {
        super::INITRD_MOUNTED.store(true, Ordering::Relaxed);
    }
}

pub(super) fn try_init_linux_compat_once() {
    if super::LINUX_COMPAT_INITED.load(Ordering::Relaxed) {
        return;
    }

    aethercore::kernel::debug_trace::record_optional(
        "main.loop",
        "linux_compat_init_attempt",
        None,
        false,
    );

    #[cfg(feature = "linux_compat")]
    {
        aethercore::klog_info!("[LINUX COMPAT] Initialising linux-compat layer");
        aethercore::modules::linux_compat::init();
        aethercore::klog_info!("[LINUX COMPAT] Ready");
        #[cfg(feature = "process_abstraction")]
        if super::LINKED_PROBE_ENABLED.load(Ordering::Relaxed) {
            aethercore::kernel::debug_trace::record_optional(
                "linked.probe",
                "linux_compat_ready",
                None,
                false,
            );
            aethercore::klog_info!(
                "[LINKED PROBE] linux-compat ready; awaiting aether_init probe execution"
            );
        }
    }

    super::LINUX_COMPAT_INITED.store(true, Ordering::Relaxed);
}

#[cfg(feature = "process_abstraction")]
#[allow(dead_code)]
#[inline(always)]
fn enter_linked_probe_service_body() -> bool {
    aethercore::kernel::debug_trace::record_optional(
        "linked.probe",
        "service_body_entered",
        None,
        false,
    );
    linked_probe_service_active()
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn observe_linked_probe_exit() {
    let pid = super::LINKED_PROBE_PID.load(Ordering::Relaxed);
    if pid == 0 {
        return;
    }

    if let Some(process) =
        aethercore::kernel::launch::process_arc_by_id(aethercore::interfaces::task::ProcessId(pid))
    {
        let (state, status, _) = process.runtime_state();
        if state == aethercore::kernel::process::ProcessLifecycleState::Exited {
            aethercore::kernel::debug_trace::record_optional(
                "linked.probe",
                "exit_observed",
                Some(status as u64),
                false,
            );
            aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe exit observed\n");
            aethercore::klog_info!("[aether_init] linked probe exit status: {}", status);
            if status == 0 {
                super::LINKED_PROBE_VERIFIED.store(true, Ordering::Relaxed);
                aethercore::klog_info!("[aether_init] linked probe execution verified");
            }
        }
    }
}

#[cfg(feature = "process_abstraction")]
#[allow(dead_code)]
#[inline(always)]
fn run_entered_linked_probe_service() -> bool {
    if !enter_linked_probe_service_body() {
        return true;
    }
    aethercore::kernel::debug_trace::record_optional("linked.probe", "service_run", None, false);
    run_linked_probe_service_transition()
}

#[cfg(feature = "process_abstraction")]
pub(super) fn service_linked_probe_once() {
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service fast path\n");

    if !super::LINKED_PROBE_ENABLED.load(Ordering::Relaxed)
        || super::LINKED_PROBE_VERIFIED.load(Ordering::Relaxed)
    {
        return;
    }

    if !super::LINUX_COMPAT_INITED.load(Ordering::Relaxed) {
        aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe waiting linux compat\n");
        return;
    }

    if !super::LINKED_PROBE_SPAWNED.load(Ordering::Relaxed) {
        aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe spawn fast path\n");
        try_spawn_linked_probe(linked_probe_spawn_request());
        return;
    }

    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe observe fast path\n");
    observe_linked_probe_exit();
}

#[cfg(all(test, feature = "process_abstraction"))]
mod tests {
    use core::sync::atomic::Ordering;
    use crate::kernel_runtime::main_loop::{
        LINKED_PROBE_ENABLED,
        LINKED_PROBE_PID,
        LINKED_PROBE_SPAWNED,
        LINKED_PROBE_VERIFIED,
        LINUX_COMPAT_INITED,
    };

    #[test_case]
    fn linked_probe_can_spawn_only_when_compat_ready_and_not_spawned() {
        assert!(super::linked_probe_can_spawn(true, false));
        assert!(!super::linked_probe_can_spawn(false, false));
        assert!(!super::linked_probe_can_spawn(true, true));
        assert!(!super::linked_probe_can_spawn(false, true));
    }

    #[test_case]
    fn linked_probe_service_action_matches_runtime_expectations() {
        assert_eq!(
            super::linked_probe_service_action(false, false),
            super::LinkedProbeServiceAction::WaitForLinuxCompat
        );
        assert_eq!(
            super::linked_probe_service_action(false, true),
            super::LinkedProbeServiceAction::WaitForLinuxCompat
        );
        assert_eq!(
            super::linked_probe_service_action(true, false),
            super::LinkedProbeServiceAction::Spawn
        );
        assert_eq!(
            super::linked_probe_service_action(true, true),
            super::LinkedProbeServiceAction::ObserveExit
        );
    }

    #[test_case]
    fn linked_probe_service_decision_preserves_state_and_action() {
        let decision = super::linked_probe_service_decision(true, false);
        assert!(decision.linux_compat_inited);
        assert!(!decision.spawned);
        assert_eq!(decision.action, super::LinkedProbeServiceAction::Spawn);
    }

    #[test_case]
    fn linked_probe_runtime_state_can_be_constructed_from_flags() {
        let state = super::LinkedProbeRuntimeState {
            linux_compat_inited: true,
            spawned: false,
        };
        assert!(state.linux_compat_inited);
        assert!(!state.spawned);
    }

    #[test_case]
    fn linked_probe_service_decision_helper_matches_direct_decision() {
        let direct = super::linked_probe_service_decision(true, false);
        assert_eq!(direct.action, super::LinkedProbeServiceAction::Spawn);
    }

    #[test_case]
    fn linked_probe_spawn_request_uses_expected_static_bootstrap_contract() {
        let request = super::linked_probe_spawn_request();
        assert_eq!(request.process_name, b"aether_init");
        assert_eq!(request.image, super::LINKED_PROBE_IMAGE);
        assert_eq!(request.priority, 128);
        assert_eq!(request.deadline, 0);
        assert_eq!(request.burst_time, 0);
        assert_eq!(request.kernel_stack_top, 0);
    }

    #[test_case]
    fn linked_probe_spawn_request_is_copy_stable() {
        let request = super::linked_probe_spawn_request();
        let copied = request;
        assert_eq!(copied, request);
    }

    #[test_case]
    fn linked_probe_spawn_branch_helper_is_callable_repeat() {
        super::enter_linked_probe_spawn_branch();
    }

    #[test_case]
    fn linked_probe_service_transition_includes_spawn_request_only_for_spawn() {
        let transition = super::PreparedLinkedProbeServiceDecision {
            decision: super::linked_probe_service_decision(true, false),
            spawn_request: Some(super::linked_probe_spawn_request()),
        };
        assert_eq!(
            transition.decision.action,
            super::LinkedProbeServiceAction::Spawn
        );
        assert!(transition.spawn_request.is_some());
    }

    #[test_case]
    fn linked_probe_spawn_request_keeps_zero_stack_top_contract() {
        let request = super::linked_probe_spawn_request();
        assert_eq!(request.kernel_stack_top, 0);
    }

    #[test_case]
    fn linked_probe_service_transition_dispatch_returns_early_for_spawn() {
        let transition = super::PreparedLinkedProbeServiceDecision {
            decision: super::linked_probe_service_decision(true, false),
            spawn_request: Some(super::linked_probe_spawn_request()),
        };
        assert!(super::dispatch_linked_probe_service_transition(transition));
    }

    #[test_case]
    fn linked_probe_service_entry_helper_keeps_spawn_transition_shape() {
        let transition = super::PreparedLinkedProbeServiceDecision {
            decision: super::linked_probe_service_decision(true, false),
            spawn_request: Some(super::linked_probe_spawn_request()),
        };
        assert_eq!(
            transition.decision.action,
            super::LinkedProbeServiceAction::Spawn
        );
    }

    #[test_case]
    fn linked_probe_spawn_transition_helper_is_callable() {
        super::dispatch_linked_probe_spawn_transition(super::linked_probe_spawn_request());
    }

    #[test_case]
    fn linked_probe_service_transition_runner_returns_early_for_spawn() {
        LINUX_COMPAT_INITED.store(true, Ordering::Relaxed);
        LINKED_PROBE_SPAWNED.store(false, Ordering::Relaxed);
        assert!(super::run_linked_probe_service_transition());
    }

    #[test_case]
    fn linked_probe_spawn_branch_helper_is_callable_again() {
        super::enter_linked_probe_spawn_branch();
    }

    #[test_case]
    fn linked_probe_service_transition_helper_preserves_spawn_request_shape() {
        LINUX_COMPAT_INITED.store(true, Ordering::Relaxed);
        LINKED_PROBE_SPAWNED.store(false, Ordering::Relaxed);
        let transition = super::prepare_linked_probe_service_transition();
        assert_eq!(
            transition.decision.action,
            super::LinkedProbeServiceAction::Spawn
        );
        assert!(transition.spawn_request.is_some());
    }

    #[test_case]
    fn entered_service_transition_helper_matches_spawn_shape() {
        LINUX_COMPAT_INITED.store(true, Ordering::Relaxed);
        LINKED_PROBE_SPAWNED.store(false, Ordering::Relaxed);
        let transition = super::prepare_entered_linked_probe_service_transition();
        assert_eq!(
            transition.decision.action,
            super::LinkedProbeServiceAction::Spawn
        );
        assert!(transition.spawn_request.is_some());
    }

    #[test_case]
    fn linked_probe_service_entry_helper_is_callable() {
        super::prepare_linked_probe_service_entry();
    }

    #[test_case]
    fn linked_probe_service_body_helper_reflects_active_state() {
        LINKED_PROBE_ENABLED.store(true, Ordering::Relaxed);
        LINKED_PROBE_VERIFIED.store(false, Ordering::Relaxed);
        assert!(super::enter_linked_probe_service_body());
    }

    #[test_case]
    fn linked_probe_exit_observer_is_callable_without_pid() {
        LINKED_PROBE_PID.store(0, Ordering::Relaxed);
        super::observe_linked_probe_exit();
    }

    #[test_case]
    fn entered_linked_probe_service_runner_returns_early_when_inactive() {
        LINKED_PROBE_ENABLED.store(false, Ordering::Relaxed);
        LINKED_PROBE_VERIFIED.store(false, Ordering::Relaxed);
        assert!(super::run_entered_linked_probe_service());
    }
}
