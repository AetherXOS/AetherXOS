#[inline(always)]
pub(super) fn apply_balanced_nvme_queue_profile() {
    #[cfg(feature = "drivers")]
    {
        crate::modules::drivers::set_nvme_queue_profile(
            crate::modules::drivers::NvmeQueueProfile::Balanced,
        );
    }
}

#[inline(always)]
pub(super) fn apply_throughput_nvme_queue_profile() {
    #[cfg(feature = "drivers")]
    {
        crate::modules::drivers::set_nvme_queue_profile(
            crate::modules::drivers::NvmeQueueProfile::Throughput,
        );
    }
}

#[inline(always)]
pub(super) fn apply_low_latency_nvme_queue_profile() {
    #[cfg(feature = "drivers")]
    {
        crate::modules::drivers::set_nvme_queue_profile(
            crate::modules::drivers::NvmeQueueProfile::LowLatency,
        );
    }
}
