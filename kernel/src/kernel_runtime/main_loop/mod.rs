pub mod state;
pub mod probe;
pub mod bootstrap;
pub mod support;

#[cfg(test)]
mod tests;

use core::sync::atomic::Ordering;
use state::*;
use probe::*;

pub(super) fn runtime_main_loop() -> ! {
    aethercore::klog_info!("[EARLY SERIAL] RUNTIME_MAIN_LOOP START");
    enter_runtime_main_loop_head();
    aethercore::klog_info!("[EARLY SERIAL] after enter_runtime_main_loop_head");

    loop {
        aethercore::klog_info!("[EARLY SERIAL] main loop iteration start");
        let iteration = MAIN_LOOP_ITERATIONS.fetch_add(1, Ordering::Relaxed);
        let decision = prepare_main_loop_cycle(iteration);
        aethercore::klog_info!("[EARLY SERIAL] ml boot call");
        service_bootstrap_iteration(decision);
        aethercore::klog_info!("[EARLY SERIAL] ml after bootstrap");

        #[cfg(all(feature = "drivers", feature = "networking"))]
        {
            support::service_network_runtime();
        }

        #[cfg(feature = "vfs")]
        {
            support::service_vfs_runtime();
        }

        #[cfg(all(feature = "vfs", feature = "linux_compat"))]
        {
            support::refresh_linux_compat_surface();
        }

        {
            if let Some(drift) = aethercore::kernel::policy::sample_policy_drift_if_due() {
                support::log_runtime_policy_drift(drift);
            }
        }

        #[cfg(feature = "libnet")]
        {
            let _ = aethercore::modules::libnet::run_service_fast_path_cycle_auto();
        }

        #[cfg(all(feature = "networking", not(feature = "libnet")))]
        {
            let _ = aethercore::modules::network::bridge::poll_smoltcp_runtime();
        }

        #[cfg(feature = "process_abstraction")]
        if should_skip_idle_halt_for_linked_probe() {
            core::hint::spin_loop();
            continue;
        }
        
        aethercore::kernel::idle_once();
    }
}

fn enter_runtime_main_loop_head() {
    aethercore::klog_info!("[EARLY SERIAL] main loop entered");
    aethercore::klog_info!("[MAIN LOOP] Entered kernel main loop");
    load_main_loop_boot_state();
}

fn load_main_loop_boot_state() {
    let boot_state = prepare_main_loop_boot_state();
    if boot_state.boot_info_present {
        #[cfg(feature = "process_abstraction")]
        LINKED_PROBE_ENABLED.store(boot_state.linked_probe_enabled, Ordering::Relaxed);
        
        if boot_state.linked_probe_enabled {
            aethercore::klog_info!("[LINKED PROBE] main loop armed for linked probe boot");
        }
    }
}

fn prepare_main_loop_boot_state() -> MainLoopBootState {
    if let Some(info) = super::boot_info::try_get() {
        MainLoopBootState {
            boot_info_present: true,
            #[cfg(feature = "process_abstraction")]
            linked_probe_enabled: info.kernel_cmdline_contains(b"AETHERCORE_RUN_LINKED_PROBE=1"),
        }
    } else {
        MainLoopBootState {
            boot_info_present: false,
            #[cfg(feature = "process_abstraction")]
            linked_probe_enabled: false,
        }
    }
}

fn prepare_main_loop_cycle(iteration: usize) -> MainLoopIterationDecision {
    if iteration == 0 {
        aethercore::klog_info!("[EARLY SERIAL] main loop first iteration entered");
    }
    prepare_main_loop_iteration()
}

fn prepare_main_loop_iteration() -> MainLoopIterationDecision {
    let state = load_main_loop_iteration_state();
    MainLoopIterationDecision {
        initrd_mount: if state.initrd_mounted { MainLoopOneShotAction::Skip } else { MainLoopOneShotAction::Attempt },
        linux_compat_init: if state.linux_compat_inited { MainLoopOneShotAction::Skip } else { MainLoopOneShotAction::Attempt },
        #[cfg(feature = "process_abstraction")]
        linked_probe: linked_probe_main_loop_action(state.linked_probe_enabled, state.linked_probe_verified),
    }
}

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

fn service_bootstrap_iteration(decision: MainLoopIterationDecision) {
    aethercore::klog_info!("[EARLY SERIAL] service_bootstrap_iteration entry");
    bootstrap::try_mount_initrd_once();
    aethercore::klog_info!("[EARLY SERIAL] service_bootstrap_iteration after initrd");
    bootstrap::try_init_linux_compat_once();
    aethercore::klog_info!("[EARLY SERIAL] service_bootstrap_iteration after linux_compat");

    #[cfg(feature = "process_abstraction")]
    {
        aethercore::klog_info!("[EARLY SERIAL] service_bootstrap_iteration probe service begin");
        service_linked_probe_for_iteration(LinkedProbeMainLoopState { action: decision.linked_probe });
        aethercore::klog_info!("[EARLY SERIAL] service_bootstrap_iteration probe service end");

        if LINKED_PROBE_SPAWNED.load(Ordering::Relaxed)
            && !LINKED_PROBE_VERIFIED.load(Ordering::Relaxed)
        {
            aethercore::klog_info!("[EARLY SERIAL] service_bootstrap_iteration forcing timer tick");
            crate::kernel_runtime::interrupts::timer_tick_handler(0);
            aethercore::klog_info!("[EARLY SERIAL] service_bootstrap_iteration after timer tick");
        }
    }
    aethercore::klog_info!("[EARLY SERIAL] service_bootstrap_iteration complete");
}

#[cfg(feature = "process_abstraction")]
fn should_skip_idle_halt_for_linked_probe() -> bool {
    LINKED_PROBE_ENABLED.load(Ordering::Relaxed) && !LINKED_PROBE_VERIFIED.load(Ordering::Relaxed)
}
