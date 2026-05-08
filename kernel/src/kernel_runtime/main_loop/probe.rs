use crate::core::log;

#[cfg(feature = "process_abstraction")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkedProbeMainLoopAction {
    Skip,
    Service,
    Closed,
}

#[cfg(feature = "process_abstraction")]
pub struct LinkedProbeMainLoopState {
    pub action: LinkedProbeMainLoopAction,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn linked_probe_service_gate(enabled: bool, verified: bool) -> bool {
    enabled && !verified
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn linked_probe_main_loop_action(enabled: bool, verified: bool) -> LinkedProbeMainLoopAction {
    if linked_probe_service_gate(enabled, verified) {
        LinkedProbeMainLoopAction::Service
    } else if enabled {
        LinkedProbeMainLoopAction::Closed
    } else {
        LinkedProbeMainLoopAction::Skip
    }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
pub fn linked_probe_main_loop_state(enabled: bool, verified: bool) -> LinkedProbeMainLoopState {
    LinkedProbeMainLoopState {
        action: linked_probe_main_loop_action(enabled, verified),
    }
}

#[cfg(feature = "process_abstraction")]
pub fn service_linked_probe_for_iteration(state: LinkedProbeMainLoopState) {
    match state.action {
        LinkedProbeMainLoopAction::Service => {
            service_open_linked_probe_for_iteration();
        }
        LinkedProbeMainLoopAction::Closed => {
            log::trace("linked probe enabled state loaded");
            log::trace("linked probe service gate closed");
        }
        LinkedProbeMainLoopAction::Skip => {}
    }
}

#[cfg(feature = "process_abstraction")]
fn service_open_linked_probe_for_iteration() {
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service gate open\n");
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service attempt\n");
    dispatch_linked_probe_service_attempt();
}

#[cfg(feature = "process_abstraction")]
fn dispatch_linked_probe_service_attempt() {
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service dispatch begin\n");
    crate::kernel_runtime::main_loop::bootstrap::service_linked_probe_once();
    aethercore::hal::serial::write_raw("[EARLY SERIAL] linked probe service dispatch returned\n");
}
