use crate::kernel_runtime::networking::{
    NETWORK_DRIVER_QUARANTINE_COOLDOWN_SAMPLES, NETWORK_DRIVER_QUARANTINE_E1000,
    NETWORK_DRIVER_QUARANTINE_EVENTS, NETWORK_DRIVER_QUARANTINE_VIRTIO,
};

#[inline(always)]
fn quarantine_counter(
    driver: aethercore::modules::drivers::ActiveNetworkDriver,
) -> Option<&'static core::sync::atomic::AtomicU64> {
    match driver {
        aethercore::modules::drivers::ActiveNetworkDriver::VirtIo => {
            Some(&NETWORK_DRIVER_QUARANTINE_VIRTIO)
        }
        aethercore::modules::drivers::ActiveNetworkDriver::E1000 => {
            Some(&NETWORK_DRIVER_QUARANTINE_E1000)
        }
        aethercore::modules::drivers::ActiveNetworkDriver::None => None,
    }
}

pub(super) fn is_driver_quarantined(
    driver: aethercore::modules::drivers::ActiveNetworkDriver,
) -> bool {
    let Some(counter) = quarantine_counter(driver) else {
        return false;
    };
    let remaining = counter.load(core::sync::atomic::Ordering::Relaxed);
    if remaining == 0 {
        return false;
    }
    counter.store(
        remaining.saturating_sub(1),
        core::sync::atomic::Ordering::Relaxed,
    );
    true
}

pub(super) fn quarantine_driver(
    driver: aethercore::modules::drivers::ActiveNetworkDriver,
    reason: &'static str,
    failures: u64,
) {
    let Some(counter) = quarantine_counter(driver) else {
        return;
    };
    let cooldown = NETWORK_DRIVER_QUARANTINE_COOLDOWN_SAMPLES.max(1);
    counter.store(cooldown, core::sync::atomic::Ordering::Relaxed);
    NETWORK_DRIVER_QUARANTINE_EVENTS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    aethercore::modules::drivers::note_quarantine(driver);
    let driver_raw = match driver {
        aethercore::modules::drivers::ActiveNetworkDriver::VirtIo => 1u64,
        aethercore::modules::drivers::ActiveNetworkDriver::E1000 => 2u64,
        aethercore::modules::drivers::ActiveNetworkDriver::None => 0u64,
    };
    aethercore::kernel::crash_log::record(
        aethercore::kernel::crash_log::EVENT_DRIVER_QUARANTINE,
        0,
        driver_raw,
        failures,
    );
    aethercore::klog_warn!(
        "Network driver quarantined: driver={:?} reason={} failures={} cooldown_samples={}",
        driver,
        reason,
        failures,
        cooldown
    );
}
