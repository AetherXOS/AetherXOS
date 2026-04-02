use super::quarantine::is_driver_quarantined;
use crate::kernel_runtime::network_policy_helpers::{
    preferred_policy_for_driver, select_network_failover_target,
};
use crate::kernel_runtime::networking::{
    E1000_IO_ERROR_STREAK, E1000_REBIND_FAILURE_STREAK, VIRTIO_IO_ERROR_STREAK,
    VIRTIO_REBIND_FAILURE_STREAK,
};

fn runtime_driver_available(driver: aethercore::modules::drivers::ActiveNetworkDriver) -> bool {
    if is_driver_quarantined(driver) {
        return false;
    }
    match driver {
        aethercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
            aethercore::modules::drivers::has_virtio_runtime_driver()
        }
        aethercore::modules::drivers::ActiveNetworkDriver::E1000 => {
            aethercore::modules::drivers::has_e1000_runtime_driver()
        }
        aethercore::modules::drivers::ActiveNetworkDriver::None => false,
    }
}

fn resolve_runtime_failover_target(
    current: aethercore::modules::drivers::ActiveNetworkDriver,
) -> aethercore::modules::drivers::ActiveNetworkDriver {
    select_network_failover_target(
        current,
        runtime_driver_available(aethercore::modules::drivers::ActiveNetworkDriver::VirtIo),
        runtime_driver_available(aethercore::modules::drivers::ActiveNetworkDriver::E1000),
    )
}

pub(super) fn try_network_failover_for_io_health(
    current: aethercore::modules::drivers::ActiveNetworkDriver,
    reason: &'static str,
) -> bool {
    let target = resolve_runtime_failover_target(current);
    if target == aethercore::modules::drivers::ActiveNetworkDriver::None {
        return false;
    }

    let switched = activate_runtime_network_driver(target, reason);
    if switched {
        aethercore::klog_warn!(
            "Network IO remediation: action=failover reason={} from={:?} to={:?}",
            reason,
            current,
            target
        );
    }
    switched
}

pub(super) fn activate_runtime_network_driver(
    target: aethercore::modules::drivers::ActiveNetworkDriver,
    reason: &'static str,
) -> bool {
    let current = aethercore::modules::drivers::active_network_driver();
    if target == current {
        return true;
    }
    if !runtime_driver_available(target) {
        return false;
    }

    aethercore::modules::drivers::network::set_driver_io_owned(false);
    let cleared = aethercore::modules::drivers::clear_network_driver_queues(current);
    match target {
        aethercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
            aethercore::modules::drivers::register_virtio_network_dataplane();
        }
        aethercore::modules::drivers::ActiveNetworkDriver::E1000 => {
            aethercore::modules::drivers::register_e1000_network_dataplane();
        }
        aethercore::modules::drivers::ActiveNetworkDriver::None => {
            aethercore::modules::drivers::clear_active_network_driver();
        }
    }
    aethercore::modules::drivers::set_network_driver_policy(preferred_policy_for_driver(target));
    aethercore::modules::drivers::note_policy_switch(target);
    aethercore::modules::drivers::network::set_driver_io_owned(
        target != aethercore::modules::drivers::ActiveNetworkDriver::None,
    );
    VIRTIO_IO_ERROR_STREAK.store(0, core::sync::atomic::Ordering::Relaxed);
    E1000_IO_ERROR_STREAK.store(0, core::sync::atomic::Ordering::Relaxed);
    VIRTIO_REBIND_FAILURE_STREAK.store(0, core::sync::atomic::Ordering::Relaxed);
    E1000_REBIND_FAILURE_STREAK.store(0, core::sync::atomic::Ordering::Relaxed);
    aethercore::klog_warn!(
        "Network driver activation: reason={} from={:?} to={:?} cleared(vrx={},vtx={},erx={},etx={})",
        reason,
        current,
        target,
        cleared.cleared_virtio_rx,
        cleared.cleared_virtio_tx,
        cleared.cleared_e1000_rx,
        cleared.cleared_e1000_tx
    );
    true
}
