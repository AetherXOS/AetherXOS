use super::state::LAST_DRIVER_WAIT_TIMEOUT_TOTAL;
use super::*;

#[inline(always)]
pub(super) fn network_slo_breach_count() -> u8 {
    #[cfg(all(feature = "drivers", feature = "networking"))]
    {
        return crate::modules::drivers::network_slo_report().breach_count;
    }
    #[allow(unreachable_code)]
    0
}

#[inline(always)]
pub(super) fn vfs_slo_breach_count() -> u8 {
    #[cfg(feature = "vfs")]
    {
        return crate::modules::vfs::evaluate_mount_health_slo().breach_count;
    }
    #[allow(unreachable_code)]
    0
}

#[inline(always)]
fn driver_wait_timeout_total() -> u64 {
    #[cfg(feature = "drivers")]
    {
        let waits = crate::modules::drivers::wait_policy_snapshot();
        return waits
            .nvme_disable_ready
            .timeout_events
            .saturating_add(waits.nvme_controller_ready.timeout_events)
            .saturating_add(waits.nvme_admin.timeout_events)
            .saturating_add(waits.nvme_io.timeout_events)
            .saturating_add(waits.ahci_read.timeout_events)
            .saturating_add(waits.ahci_write.timeout_events)
            .saturating_add(waits.e1000_reset.timeout_events);
    }
    #[allow(unreachable_code)]
    0
}

#[inline(always)]
pub(super) fn driver_wait_timeout_delta() -> u64 {
    let total = driver_wait_timeout_total();
    let prev = LAST_DRIVER_WAIT_TIMEOUT_TOTAL.swap(total, Ordering::Relaxed);
    total.saturating_sub(prev)
}
