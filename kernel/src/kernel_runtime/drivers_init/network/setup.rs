use crate::kernel_runtime::networking::{
    E1000_IO_ERROR_STREAK, NETWORK_AUTO_POLICY_SWITCH_COOLDOWN, NETWORK_AUTO_POLICY_SWITCH_COUNT,
    NETWORK_DRIVER_QUARANTINE_E1000, NETWORK_DRIVER_QUARANTINE_EVENTS,
    NETWORK_DRIVER_QUARANTINE_VIRTIO, NETWORK_SLO_BREACH_STREAK, NETWORK_SLO_LAST_LOG_SAMPLE,
    NETWORK_SLO_REMEDIATION_ACTIONS, NETWORK_SLO_REMEDIATION_STAGE, VIRTIO_IO_ERROR_STREAK,
};

const LOW_LAT_IRQ_MAX: usize = 32;
const LOW_LAT_LOOP_MAX: usize = 96;
const LOW_LAT_RING_MAX: usize = 384;
const THROUGHPUT_IRQ_MIN: usize = 128;
const THROUGHPUT_LOOP_MIN: usize = 256;
const THROUGHPUT_RING_MIN: usize = 1024;

pub(super) fn configure_network_runtime_defaults() {
    let loop_budget = hypercore::config::KernelConfig::driver_network_loop_service_budget();
    let irq_budget = hypercore::config::KernelConfig::driver_network_irq_service_budget();
    let ring_limit = hypercore::config::KernelConfig::driver_network_ring_limit();

    hypercore::modules::drivers::configure_network_service_budgets(loop_budget, irq_budget);
    hypercore::modules::drivers::configure_network_ring_limit(ring_limit);
    let profile = classify_poll_profile(loop_budget, irq_budget, ring_limit);
    hypercore::modules::drivers::set_network_poll_profile(profile);

    hypercore::modules::drivers::network::set_driver_io_owned(false);
    detach_runtime_network_drivers();
    reset_network_runtime_counters();
}

fn classify_poll_profile(
    loop_budget: usize,
    irq_budget: usize,
    ring_limit: usize,
) -> hypercore::modules::drivers::NetworkPollProfile {
    if irq_budget <= LOW_LAT_IRQ_MAX
        && loop_budget <= LOW_LAT_LOOP_MAX
        && ring_limit <= LOW_LAT_RING_MAX
    {
        hypercore::modules::drivers::NetworkPollProfile::LowLatency
    } else if irq_budget >= THROUGHPUT_IRQ_MIN
        || loop_budget >= THROUGHPUT_LOOP_MIN
        || ring_limit >= THROUGHPUT_RING_MIN
    {
        hypercore::modules::drivers::NetworkPollProfile::Throughput
    } else {
        hypercore::modules::drivers::NetworkPollProfile::Balanced
    }
}

fn detach_runtime_network_drivers() {
    let _ = hypercore::modules::drivers::hotplug_detach_network_driver(
        hypercore::modules::drivers::ActiveNetworkDriver::VirtIo,
    );
    let _ = hypercore::modules::drivers::hotplug_detach_network_driver(
        hypercore::modules::drivers::ActiveNetworkDriver::E1000,
    );
}

fn reset_network_runtime_counters() {
    VIRTIO_IO_ERROR_STREAK.store(0, core::sync::atomic::Ordering::Relaxed);
    E1000_IO_ERROR_STREAK.store(0, core::sync::atomic::Ordering::Relaxed);
    NETWORK_SLO_BREACH_STREAK.store(0, core::sync::atomic::Ordering::Relaxed);
    NETWORK_AUTO_POLICY_SWITCH_COUNT.store(0, core::sync::atomic::Ordering::Relaxed);
    NETWORK_AUTO_POLICY_SWITCH_COOLDOWN.store(0, core::sync::atomic::Ordering::Relaxed);
    NETWORK_SLO_REMEDIATION_STAGE.store(0, core::sync::atomic::Ordering::Relaxed);
    NETWORK_SLO_REMEDIATION_ACTIONS.store(0, core::sync::atomic::Ordering::Relaxed);
    NETWORK_SLO_LAST_LOG_SAMPLE.store(0, core::sync::atomic::Ordering::Relaxed);
    NETWORK_DRIVER_QUARANTINE_VIRTIO.store(0, core::sync::atomic::Ordering::Relaxed);
    NETWORK_DRIVER_QUARANTINE_E1000.store(0, core::sync::atomic::Ordering::Relaxed);
    NETWORK_DRIVER_QUARANTINE_EVENTS.store(0, core::sync::atomic::Ordering::Relaxed);
}
