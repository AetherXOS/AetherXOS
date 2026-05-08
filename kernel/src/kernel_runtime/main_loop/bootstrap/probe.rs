use core::sync::atomic::Ordering;

#[cfg(feature = "process_abstraction")]
include!(concat!(env!("OUT_DIR"), "/linked_probe_image.rs"));

#[cfg(feature = "process_abstraction")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkedProbeSpawnRequest {
    pub process_name: &'static [u8],
    pub image: &'static [u8],
    pub priority: u8,
    pub deadline: u64,
    pub burst_time: u64,
    pub kernel_stack_top: u64,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn linked_probe_spawn_request() -> LinkedProbeSpawnRequest {
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
pub fn try_spawn_linked_probe(request: LinkedProbeSpawnRequest) {
    let result = invoke_linked_probe_spawn(request);

    match result {
        Ok((pid, _tid)) => {
            aethercore::kernel::debug_trace::record_optional(
                "linked.probe",
                "spawn_returned",
                Some(pid as u64),
                false,
            );
            super::super::LINKED_PROBE_PID.store(pid, Ordering::Relaxed);
            super::super::LINKED_PROBE_SPAWNED.store(true, Ordering::Relaxed);
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
    aethercore::kernel::debug_trace::record_optional(
        "linked.probe",
        "linux_compat_ok",
        None,
        false,
    );
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
        None,
    )
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn enter_linked_probe_spawn_branch() {
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe linux compat gate returned\n");
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service spawn branch\n");
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe can spawn\n");
    aethercore::kernel::debug_trace::record_optional("linked.probe", "spawn_gate", None, false);
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn dispatch_linked_probe_spawn_transition(request: LinkedProbeSpawnRequest) {
    enter_linked_probe_spawn_branch();
    try_spawn_linked_probe(request);
}

#[cfg(feature = "process_abstraction")]

#[inline(always)]
pub fn linked_probe_service_active() -> bool {
    super::super::LINKED_PROBE_ENABLED.load(Ordering::Relaxed)
        && !super::super::LINKED_PROBE_VERIFIED.load(Ordering::Relaxed)
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn linked_probe_can_spawn(linux_compat_inited: bool, spawned: bool) -> bool {
    linux_compat_inited && !spawned
}

#[cfg(feature = "process_abstraction")]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkedProbeServiceAction {
    WaitForLinuxCompat,
    Spawn,
    ObserveExit,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn linked_probe_service_action(
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkedProbeServiceDecision {
    pub linux_compat_inited: bool,
    pub spawned: bool,
    pub action: LinkedProbeServiceAction,
}

#[cfg(feature = "process_abstraction")]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedLinkedProbeServiceDecision {
    pub decision: LinkedProbeServiceDecision,
    pub spawn_request: Option<LinkedProbeSpawnRequest>,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn linked_probe_service_decision(
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkedProbeRuntimeState {
    pub linux_compat_inited: bool,
    pub spawned: bool,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn load_linked_probe_runtime_state() -> LinkedProbeRuntimeState {
    LinkedProbeRuntimeState {
        linux_compat_inited: super::super::LINUX_COMPAT_INITED.load(Ordering::Relaxed),
        spawned: super::super::LINKED_PROBE_SPAWNED.load(Ordering::Relaxed),
    }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn prepare_linked_probe_service_decision() -> LinkedProbeServiceDecision {
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
pub fn prepare_linked_probe_service_transition() -> PreparedLinkedProbeServiceDecision {
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
pub fn enter_linked_probe_service() -> PreparedLinkedProbeServiceDecision {
    prepare_entered_linked_probe_service_transition()
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn prepare_linked_probe_service_entry() {
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service entered\n");
    aethercore::kernel::debug_trace::record_optional(
        "linked.probe",
        "service_entered",
        None,
        false,
    );
    aethercore::kernel::debug_trace::record_optional(
        "linked.probe",
        "cmdline_gate_ok",
        None,
        false,
    );
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn prepare_entered_linked_probe_service_transition() -> PreparedLinkedProbeServiceDecision {
    prepare_linked_probe_service_entry();
    prepare_linked_probe_service_transition()
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn dispatch_linked_probe_service_transition(
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
pub fn run_linked_probe_service_transition() -> bool {
    aethercore::kernel::debug_trace::record_optional(
        "linked.probe",
        "service_transition_run",
        None,
        false,
    );
    let transition = enter_linked_probe_service();
    dispatch_linked_probe_service_transition(transition)
}

#[cfg(feature = "process_abstraction")]

#[inline(always)]
pub fn enter_linked_probe_service_body() -> bool {
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
pub fn observe_linked_probe_exit() {
    let pid = super::super::LINKED_PROBE_PID.load(Ordering::Relaxed);
    if pid == 0 {
        return;
    }

    if let Some(process) =
        aethercore::kernel::launch::process_arc_by_id(aethercore::interfaces::task::ProcessId(pid))
    {
        let state_byte = process.lifecycle_state.load(core::sync::atomic::Ordering::Relaxed);
        let state = aethercore::kernel::process::ProcessLifecycleState::from_u8(state_byte);
        let status = process.exit_status.load(core::sync::atomic::Ordering::Relaxed);
        if state == Some(aethercore::kernel::process::ProcessLifecycleState::Exited) {
            aethercore::kernel::debug_trace::record_optional(
                "linked.probe",
                "exit_observed",
                Some(status as u64),
                false,
            );
            aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe exit observed\n");
            aethercore::klog_info!("[aether_init] linked probe exit status: {}", status);
            if status == 0 {
                super::super::LINKED_PROBE_VERIFIED.store(true, Ordering::Relaxed);
                aethercore::klog_info!("[aether_init] linked probe execution verified");
            }
        }
    }
}

#[cfg(feature = "process_abstraction")]

#[inline(always)]
pub fn run_entered_linked_probe_service() -> bool {
    if !enter_linked_probe_service_body() {
        return true;
    }
    aethercore::kernel::debug_trace::record_optional("linked.probe", "service_run", None, false);
    run_linked_probe_service_transition()
}

#[cfg(feature = "process_abstraction")]
pub fn service_linked_probe_once() {
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service fast path\n");

    if !super::super::LINKED_PROBE_ENABLED.load(Ordering::Relaxed)
        || super::super::LINKED_PROBE_VERIFIED.load(Ordering::Relaxed)
    {
        return;
    }

    if !super::super::LINUX_COMPAT_INITED.load(Ordering::Relaxed) {
        aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe waiting linux compat\n");
        return;
    }

    if !super::super::LINKED_PROBE_SPAWNED.load(Ordering::Relaxed) {
        aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe spawn fast path\n");
        try_spawn_linked_probe(linked_probe_spawn_request());
        return;
    }

    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe observe fast path\n");
    observe_linked_probe_exit();
}

