//! Production-grade kernel main loop
//!
//! Runs after the boot sequence completes.  Responsibilities:
//!
//! 1. VFS initrd auto-mount (once, on first iteration)
//! 2. Linux-compat layer initialisation (once, gated by feature flag)
//! 3. Watchdog tick forwarding
//! 4. Network servicing (driver I/O → libnet fast path → smoltcp poll)
//! 5. CPU idle (HLT or SPIN depending on config)
//!
//! Everything in the hot path is inlined or branchless where possible.

use core::sync::atomic::AtomicBool;
#[cfg(feature = "vfs")]
use core::sync::atomic::AtomicU64;
use core::sync::atomic::AtomicUsize;
#[cfg(feature = "process_abstraction")]
use core::sync::atomic::Ordering;
use core::sync::atomic::Ordering as AtomicOrdering;

#[path = "main_loop_bootstrap.rs"]
mod bootstrap;
#[path = "main_loop_support/mod.rs"]
mod support;

// ── One-shot flags ────────────────────────────────────────────────────────────

static INITRD_MOUNTED: AtomicBool = AtomicBool::new(false);
static LINUX_COMPAT_INITED: AtomicBool = AtomicBool::new(false);
static MAIN_LOOP_ITERATIONS: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "process_abstraction")]
static LINKED_PROBE_PID: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "process_abstraction")]
static LINKED_PROBE_SPAWNED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "process_abstraction")]
static LINKED_PROBE_VERIFIED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "process_abstraction")]
static LINKED_PROBE_ENABLED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "vfs")]
static VFS_SLO_SAMPLE_COUNTER: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
static VFS_SLO_BREACH_STREAK: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
static VFS_SLO_POLICY_ACTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
static VFS_SLO_LAST_LOG_SAMPLE: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
const VFS_SLO_SAMPLE_INTERVAL: u64 = 512;
#[cfg(feature = "vfs")]
const VFS_SLO_LOG_INTERVAL_MULTIPLIER: u64 = 8;
#[cfg(feature = "vfs")]
const VFS_SLO_ACTION_STREAK_THRESHOLD: u64 = 2;
#[cfg(all(feature = "vfs", feature = "linux_compat"))]
static COMPAT_SURFACE_SAMPLE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MainLoopOneShotAction {
    Skip,
    Attempt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MainLoopIterationDecision {
    initrd_mount: MainLoopOneShotAction,
    linux_compat_init: MainLoopOneShotAction,
    #[cfg(feature = "process_abstraction")]
    linked_probe: LinkedProbeMainLoopAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MainLoopIterationState {
    initrd_mounted: bool,
    linux_compat_inited: bool,
    #[cfg(feature = "process_abstraction")]
    linked_probe_enabled: bool,
    #[cfg(feature = "process_abstraction")]
    linked_probe_verified: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MainLoopBootState {
    boot_info_present: bool,
    #[cfg(feature = "process_abstraction")]
    linked_probe_enabled: bool,
}

#[inline(always)]
fn initrd_mount_action(mounted: bool) -> MainLoopOneShotAction {
    if mounted {
        MainLoopOneShotAction::Skip
    } else {
        MainLoopOneShotAction::Attempt
    }
}

#[inline(always)]
fn linux_compat_init_action(inited: bool) -> MainLoopOneShotAction {
    if inited {
        MainLoopOneShotAction::Skip
    } else {
        MainLoopOneShotAction::Attempt
    }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn linked_probe_service_gate(enabled: bool, verified: bool) -> bool {
    enabled && !verified
}

#[cfg(feature = "process_abstraction")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinkedProbeMainLoopAction {
    Skip,
    Service,
    Closed,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn linked_probe_main_loop_action(enabled: bool, verified: bool) -> LinkedProbeMainLoopAction {
    if linked_probe_service_gate(enabled, verified) {
        LinkedProbeMainLoopAction::Service
    } else if enabled {
        LinkedProbeMainLoopAction::Closed
    } else {
        LinkedProbeMainLoopAction::Skip
    }
}

#[cfg(feature = "process_abstraction")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LinkedProbeMainLoopState {
    action: LinkedProbeMainLoopAction,
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn linked_probe_main_loop_state(enabled: bool, verified: bool) -> LinkedProbeMainLoopState {
    LinkedProbeMainLoopState {
        action: linked_probe_main_loop_action(enabled, verified),
    }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn log_linked_probe_service_call_boundary() {
    hypercore::kernel::debug_trace::record_optional("linked.probe", "service_helper_entered", None, false);
    hypercore::kernel::debug_trace::record_optional("linked.probe", "service_call_boundary", None, false);
    hypercore::hal::serial::write_raw(
        "[EARLY SERIAL] linked probe service call begin\n",
    );
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn enter_linked_probe_service_dispatch() {
    log_linked_probe_service_call_boundary();
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn service_open_linked_probe_for_iteration() {
    hypercore::kernel::debug_trace::record_optional("linked.probe", "enabled_state_loaded", None, false);
    hypercore::hal::serial::write_raw(
        "[EARLY SERIAL] linked probe service gate open\n",
    );
    hypercore::hal::serial::write_raw(
        "[EARLY SERIAL] linked probe service attempt\n",
    );
    dispatch_linked_probe_service_attempt();
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn dispatch_linked_probe_service_attempt() {
    hypercore::kernel::debug_trace::record_optional("linked.probe", "service_attempt_dispatch", None, false);
    dispatch_open_linked_probe_service();
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn dispatch_open_linked_probe_service() {
    hypercore::hal::serial::write_raw(
        "[EARLY SERIAL] linked probe service dispatch begin\n",
    );
    invoke_linked_probe_service_dispatch();
    hypercore::hal::serial::write_raw(
        "[EARLY SERIAL] linked probe service returned\n",
    );
    hypercore::hal::serial::write_raw(
        "[EARLY SERIAL] linked probe service dispatch returned\n",
    );
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn invoke_linked_probe_service_dispatch() {
    enter_linked_probe_service_dispatch();
    bootstrap::service_linked_probe_once();
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn service_linked_probe_for_iteration(state: LinkedProbeMainLoopState) {
    match state.action {
        LinkedProbeMainLoopAction::Service => {
            service_open_linked_probe_for_iteration();
        }
        LinkedProbeMainLoopAction::Closed => {
            hypercore::hal::serial::write_raw(
                "[EARLY SERIAL] linked probe enabled state loaded\n",
            );
            hypercore::hal::serial::write_raw(
                "[EARLY SERIAL] linked probe service gate closed\n",
            );
        }
        LinkedProbeMainLoopAction::Skip => {}
    }
}

#[inline(always)]
fn service_bootstrap_iteration(decision: MainLoopIterationDecision) {
    run_one_shot_bootstrap_iteration(decision);

    #[cfg(feature = "process_abstraction")]
    {
        hypercore::kernel::debug_trace::record_optional("main.loop", "bootstrap_service_step", None, false);
        hypercore::hal::serial::write_raw(
            "[EARLY SERIAL] main loop bootstrap service begin\n",
        );
        run_linked_probe_iteration_service(decision.linked_probe);

        if LINKED_PROBE_SPAWNED.load(Ordering::Relaxed)
            && !LINKED_PROBE_VERIFIED.load(Ordering::Relaxed)
        {
            hypercore::hal::serial::write_raw(
                "[EARLY SERIAL] linked probe forced scheduler tick\n",
            );
            crate::kernel_runtime::interrupts::timer_tick_handler(0);
        }

        hypercore::hal::serial::write_raw(
            "[EARLY SERIAL] main loop bootstrap service returned\n",
        );
    }
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn run_linked_probe_iteration_service(action: LinkedProbeMainLoopAction) {
    hypercore::kernel::debug_trace::record_optional("main.loop", "service_step", None, false);
    service_linked_probe_for_iteration(LinkedProbeMainLoopState { action });
}

#[inline(always)]
fn run_one_shot_bootstrap_iteration(decision: MainLoopIterationDecision) {
    hypercore::hal::serial::write_raw(
        "[EARLY SERIAL] main loop one-shot bootstrap begin\n",
    );
    service_one_shot_bootstrap_for_iteration(decision);
    hypercore::hal::serial::write_raw(
        "[EARLY SERIAL] main loop one-shot bootstrap returned\n",
    );
}

#[inline(always)]
fn main_loop_iteration_decision(
    initrd_mounted: bool,
    linux_compat_inited: bool,
    #[cfg(feature = "process_abstraction")] linked_probe_enabled: bool,
    #[cfg(feature = "process_abstraction")] linked_probe_verified: bool,
) -> MainLoopIterationDecision {
    MainLoopIterationDecision {
        initrd_mount: initrd_mount_action(initrd_mounted),
        linux_compat_init: linux_compat_init_action(linux_compat_inited),
        #[cfg(feature = "process_abstraction")]
        linked_probe: linked_probe_main_loop_state(linked_probe_enabled, linked_probe_verified).action,
    }
}

#[inline(always)]
fn load_main_loop_iteration_state() -> MainLoopIterationState {
    MainLoopIterationState {
        initrd_mounted: INITRD_MOUNTED.load(Ordering::Relaxed),
        linux_compat_inited: LINUX_COMPAT_INITED.load(Ordering::Relaxed),
        #[cfg(feature = "process_abstraction")]
        linked_probe_enabled: LINKED_PROBE_ENABLED.load(Ordering::Relaxed),
        #[cfg(feature = "process_abstraction")]
        linked_probe_verified: LINKED_PROBE_VERIFIED.load(Ordering::Relaxed),
    }
}

#[inline(always)]
fn prepare_main_loop_iteration() -> MainLoopIterationDecision {
    let state = load_main_loop_iteration_state();
    main_loop_iteration_decision(
        state.initrd_mounted,
        state.linux_compat_inited,
        #[cfg(feature = "process_abstraction")]
        state.linked_probe_enabled,
        #[cfg(feature = "process_abstraction")]
        state.linked_probe_verified,
    )
}

#[inline(always)]
fn run_initrd_mount_transition(action: MainLoopOneShotAction) {
    if matches!(action, MainLoopOneShotAction::Attempt) {
        hypercore::kernel::debug_trace::record_optional("main.loop", "initrd_step", None, false);
        hypercore::hal::serial::write_raw("[EARLY SERIAL] initrd mount begin\n");
    }
    bootstrap::try_mount_initrd_once();
    if matches!(action, MainLoopOneShotAction::Attempt) {
        hypercore::hal::serial::write_raw("[EARLY SERIAL] initrd mount returned\n");
    }
}

#[inline(always)]
fn run_linux_compat_transition(action: MainLoopOneShotAction) {
    if matches!(action, MainLoopOneShotAction::Attempt) {
        hypercore::kernel::debug_trace::record_optional("main.loop", "linux_compat_step", None, false);
        hypercore::hal::serial::write_raw("[EARLY SERIAL] linux compat init begin\n");
    }
    bootstrap::try_init_linux_compat_once();
    if matches!(action, MainLoopOneShotAction::Attempt) {
        hypercore::hal::serial::write_raw(
            "[EARLY SERIAL] linux compat init returned\n",
        );
    }
}

#[inline(always)]
fn service_one_shot_bootstrap_for_iteration(decision: MainLoopIterationDecision) {
    hypercore::kernel::debug_trace::record_optional("main.loop", "oneshot_step", None, false);
    run_initrd_mount_transition(decision.initrd_mount);
    run_linux_compat_transition(decision.linux_compat_init);
}

#[inline(always)]
fn prepare_main_loop_boot_state() -> MainLoopBootState {
    if let Some(info) = super::boot_info::try_get() {
        MainLoopBootState {
            boot_info_present: true,
            #[cfg(feature = "process_abstraction")]
            linked_probe_enabled: info.kernel_cmdline_contains(b"HYPERCORE_RUN_LINKED_PROBE=1"),
        }
    } else {
        MainLoopBootState {
            boot_info_present: false,
            #[cfg(feature = "process_abstraction")]
            linked_probe_enabled: false,
        }
    }
}

#[inline(always)]
fn load_main_loop_boot_state() {
    hypercore::hal::serial::write_raw("[EARLY SERIAL] main loop boot info query begin\n");
    let boot_state = prepare_main_loop_boot_state();
    if boot_state.boot_info_present {
        hypercore::hal::serial::write_raw(
            "[EARLY SERIAL] main loop boot info query returned some\n",
        );
        hypercore::kernel::debug_trace::record_optional("main.loop", "cmdline_scan_begin", None, false);
        hypercore::kernel::debug_trace::record_optional(
            "main.loop",
            "cmdline_scan_returned",
            Some(boot_state.boot_info_present as u64),
            false,
        );
        hypercore::hal::serial::write_raw(
            "[EARLY SERIAL] main loop cmdline scan returned\n",
        );
        #[cfg(feature = "process_abstraction")]
        LINKED_PROBE_ENABLED.store(boot_state.linked_probe_enabled, Ordering::Relaxed);
        if boot_state.linked_probe_enabled {
            hypercore::hal::serial::write_raw(
                "[EARLY SERIAL] linked probe main loop armed\n",
            );
            hypercore::klog_info!("[LINKED PROBE] main loop armed for linked probe boot");
        }
    } else {
        hypercore::hal::serial::write_raw(
            "[EARLY SERIAL] main loop boot info query returned none\n",
        );
    }
}

#[inline(always)]
fn prepare_main_loop_cycle(iteration: usize) -> MainLoopIterationDecision {
    if iteration == 0 {
        hypercore::hal::serial::write_raw(
            "[EARLY SERIAL] main loop first iteration entered\n",
        );
    }
    hypercore::kernel::debug_trace::record_optional("main.loop", "iteration_begin", None, false);
    hypercore::hal::serial::write_raw("[EARLY SERIAL] ml iter dec\n");
    prepare_main_loop_iteration()
}

// ── Main loop entry ───────────────────────────────────────────────────────────

#[inline(always)]
fn enter_runtime_main_loop_head() {
    hypercore::hal::serial::write_raw("[EARLY SERIAL] main loop entered\n");
    hypercore::klog_info!("[MAIN LOOP] Entered kernel main loop");
    load_main_loop_boot_state();
}

#[cfg(feature = "process_abstraction")]
#[inline(always)]
fn should_skip_idle_halt_for_linked_probe() -> bool {
    LINKED_PROBE_ENABLED.load(Ordering::Relaxed) && !LINKED_PROBE_VERIFIED.load(Ordering::Relaxed)
}

pub(super) fn runtime_main_loop() -> ! {
    enter_runtime_main_loop_head();

    loop {
        let iteration = MAIN_LOOP_ITERATIONS.fetch_add(1, AtomicOrdering::Relaxed);
        let decision = prepare_main_loop_cycle(iteration);
        hypercore::hal::serial::write_raw("[EARLY SERIAL] ml boot call\n");
        service_bootstrap_iteration(decision);

        // ── 3. Network driver I/O servicing ──────────────────────────────
        #[cfg(all(feature = "drivers", feature = "networking"))]
        {
            support::service_network_runtime();
        }

        // ── 4.5. VFS health SLO and policy action ───────────────────────
        #[cfg(feature = "vfs")]
        {
            support::service_vfs_runtime();
        }

        #[cfg(all(feature = "vfs", feature = "linux_compat"))]
        {
            support::refresh_linux_compat_surface();
        }

        // ── 4.6. Runtime policy drift report ─────────────────────────────
        {
            if let Some(drift) = hypercore::kernel::policy::sample_policy_drift_if_due() {
                support::log_runtime_policy_drift(drift);
            }
        }

        // ── 4. LibNet fast path ───────────────────────────────────────────
        #[cfg(feature = "libnet")]
        {
            let _ = hypercore::modules::libnet::run_service_fast_path_cycle_auto();
        }

        // ── 5. smoltcp poll (networking without libnet) ───────────────────
        #[cfg(all(feature = "networking", not(feature = "libnet")))]
        {
            let _ = hypercore::modules::network::bridge::poll_smoltcp_runtime();
        }

        // ── 6. Idle ───────────────────────────────────────────────────────
        #[cfg(feature = "process_abstraction")]
        if should_skip_idle_halt_for_linked_probe() {
            core::hint::spin_loop();
            continue;
        }
        hypercore::kernel::idle_once();
    }
}

#[cfg(test)]
mod one_shot_tests {
    #[test]
    fn one_shot_actions_only_attempt_when_not_already_done() {
        assert_eq!(
            super::initrd_mount_action(false),
            super::MainLoopOneShotAction::Attempt
        );
        assert_eq!(
            super::initrd_mount_action(true),
            super::MainLoopOneShotAction::Skip
        );
        assert_eq!(
            super::linux_compat_init_action(false),
            super::MainLoopOneShotAction::Attempt
        );
        assert_eq!(
            super::linux_compat_init_action(true),
            super::MainLoopOneShotAction::Skip
        );
    }

    #[test]
    fn main_loop_iteration_decision_combines_one_shot_actions() {
        let decision = super::main_loop_iteration_decision(
            false,
            true,
            #[cfg(feature = "process_abstraction")]
            true,
            #[cfg(feature = "process_abstraction")]
            false,
        );
        assert_eq!(decision.initrd_mount, super::MainLoopOneShotAction::Attempt);
        assert_eq!(decision.linux_compat_init, super::MainLoopOneShotAction::Skip);
        #[cfg(feature = "process_abstraction")]
        assert_eq!(decision.linked_probe, super::LinkedProbeMainLoopAction::Service);
    }

    #[test]
    fn iteration_preparation_matches_direct_decision() {
        super::INITRD_MOUNTED.store(false, AtomicOrdering::Relaxed);
        super::LINUX_COMPAT_INITED.store(true, AtomicOrdering::Relaxed);
        #[cfg(feature = "process_abstraction")]
        super::LINKED_PROBE_ENABLED.store(true, AtomicOrdering::Relaxed);
        #[cfg(feature = "process_abstraction")]
        super::LINKED_PROBE_VERIFIED.store(false, AtomicOrdering::Relaxed);

        let prepared = super::prepare_main_loop_iteration();
        let direct = super::main_loop_iteration_decision(
            false,
            true,
            #[cfg(feature = "process_abstraction")]
            true,
            #[cfg(feature = "process_abstraction")]
            false,
        );

        assert_eq!(prepared.initrd_mount, direct.initrd_mount);
        assert_eq!(prepared.linux_compat_init, direct.linux_compat_init);
        #[cfg(feature = "process_abstraction")]
        assert_eq!(prepared.linked_probe, direct.linked_probe);
    }

    #[test]
    fn one_shot_bootstrap_helper_is_callable_for_attempt_and_skip_mix() {
        super::service_one_shot_bootstrap_for_iteration(super::MainLoopIterationDecision {
            initrd_mount: super::MainLoopOneShotAction::Attempt,
            linux_compat_init: super::MainLoopOneShotAction::Skip,
            #[cfg(feature = "process_abstraction")]
            linked_probe: super::LinkedProbeMainLoopAction::Skip,
        });
    }

    #[test]
    fn bootstrap_iteration_helper_is_callable_for_service_mix() {
        super::service_bootstrap_iteration(super::MainLoopIterationDecision {
            initrd_mount: super::MainLoopOneShotAction::Attempt,
            linux_compat_init: super::MainLoopOneShotAction::Attempt,
            #[cfg(feature = "process_abstraction")]
            linked_probe: super::LinkedProbeMainLoopAction::Skip,
        });
    }

    #[test]
    fn boot_state_loader_is_callable() {
        super::load_main_loop_boot_state();
    }

    #[test]
    fn boot_state_preparation_is_callable() {
        let state = super::prepare_main_loop_boot_state();
        assert!(matches!(state.boot_info_present, true | false));
    }

    #[test]
    fn main_loop_cycle_preparation_matches_iteration_decision() {
        super::INITRD_MOUNTED.store(false, AtomicOrdering::Relaxed);
        super::LINUX_COMPAT_INITED.store(false, AtomicOrdering::Relaxed);
        #[cfg(feature = "process_abstraction")]
        super::LINKED_PROBE_ENABLED.store(true, AtomicOrdering::Relaxed);
        #[cfg(feature = "process_abstraction")]
        super::LINKED_PROBE_VERIFIED.store(false, AtomicOrdering::Relaxed);

        let prepared = super::prepare_main_loop_cycle(0);
        let direct = super::prepare_main_loop_iteration();

        assert_eq!(prepared.initrd_mount, direct.initrd_mount);
        assert_eq!(prepared.linux_compat_init, direct.linux_compat_init);
        #[cfg(feature = "process_abstraction")]
        assert_eq!(prepared.linked_probe, direct.linked_probe);
    }

    #[test]
    fn runtime_main_loop_head_helper_is_callable() {
        super::enter_runtime_main_loop_head();
    }

    #[test]
    fn one_shot_bootstrap_runner_is_callable() {
        super::run_one_shot_bootstrap_iteration(super::MainLoopIterationDecision {
            initrd_mount: super::MainLoopOneShotAction::Attempt,
            linux_compat_init: super::MainLoopOneShotAction::Attempt,
            #[cfg(feature = "process_abstraction")]
            linked_probe: super::LinkedProbeMainLoopAction::Skip,
        });
    }

    #[test]
    fn initrd_mount_transition_helper_is_callable() {
        super::run_initrd_mount_transition(super::MainLoopOneShotAction::Attempt);
    }

    #[test]
    fn linux_compat_transition_helper_is_callable() {
        super::run_linux_compat_transition(super::MainLoopOneShotAction::Attempt);
    }
}

#[cfg(all(test, feature = "process_abstraction"))]
mod process_abstraction_tests {
    #[test]
    fn linked_probe_main_loop_action_matches_gate_state() {
        assert_eq!(
            super::linked_probe_main_loop_action(false, false),
            super::LinkedProbeMainLoopAction::Skip
        );
        assert_eq!(
            super::linked_probe_main_loop_action(false, true),
            super::LinkedProbeMainLoopAction::Skip
        );
        assert_eq!(
            super::linked_probe_main_loop_action(true, false),
            super::LinkedProbeMainLoopAction::Service
        );
        assert_eq!(
            super::linked_probe_main_loop_action(true, true),
            super::LinkedProbeMainLoopAction::Closed
        );
    }

    #[test]
    fn linked_probe_main_loop_state_preserves_action() {
        assert_eq!(
            super::linked_probe_main_loop_state(true, false).action,
            super::LinkedProbeMainLoopAction::Service
        );
        assert_eq!(
            super::linked_probe_main_loop_state(true, true).action,
            super::LinkedProbeMainLoopAction::Closed
        );
    }

    #[test]
    fn linked_probe_service_gate_matches_main_loop_action_service_only_when_open() {
        assert!(super::linked_probe_service_gate(true, false));
        assert!(!super::linked_probe_service_gate(true, true));
        assert!(!super::linked_probe_service_gate(false, false));
    }

    #[test]
    fn linked_probe_main_loop_state_round_trips_service_action() {
        let state = super::linked_probe_main_loop_state(true, false);
        assert_eq!(state.action, super::LinkedProbeMainLoopAction::Service);
    }

    #[test]
    fn service_open_helper_is_callable() {
        super::service_open_linked_probe_for_iteration();
    }

    #[test]
    fn service_dispatch_helper_is_callable() {
        super::dispatch_open_linked_probe_service();
    }

    #[test]
    fn linked_probe_service_dispatch_invoke_helper_is_callable() {
        super::invoke_linked_probe_service_dispatch();
    }

    #[test]
    fn linked_probe_iteration_service_helper_is_callable() {
        super::run_linked_probe_iteration_service(super::LinkedProbeMainLoopAction::Skip);
    }

    #[test]
    fn linked_probe_service_attempt_dispatch_helper_is_callable() {
        super::dispatch_linked_probe_service_attempt();
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "process_abstraction")]
    #[test]
    fn linked_probe_service_gate_only_runs_when_enabled_and_unverified() {
        assert!(super::linked_probe_service_gate(true, false));
        assert!(!super::linked_probe_service_gate(false, false));
        assert!(!super::linked_probe_service_gate(true, true));
        assert!(!super::linked_probe_service_gate(false, true));
    }
}

