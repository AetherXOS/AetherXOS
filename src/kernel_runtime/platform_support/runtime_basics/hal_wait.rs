pub(crate) fn log_hal_wait_policy() {
    let waits = hypercore::hal::wait::snapshot();
    hypercore::klog_info!(
        "HAL wait policy: serial={}::{} max_spins={} fallback={:?} timeouts={} smp_boot={}::{} max_spins={} fallback={:?} timeouts={} tlb={}::{} max_spins={} fallback={:?} timeouts={} iommu={}::{} max_spins={} fallback={:?} timeouts={}",
        waits.serial_tx.component,
        waits.serial_tx.operation,
        waits.serial_tx.max_spins,
        waits.serial_tx.fallback,
        waits.serial_tx.timeout_events,
        waits.smp_boot.component,
        waits.smp_boot.operation,
        waits.smp_boot.max_spins,
        waits.smp_boot.fallback,
        waits.smp_boot.timeout_events,
        waits.tlb_shootdown.component,
        waits.tlb_shootdown.operation,
        waits.tlb_shootdown.max_spins,
        waits.tlb_shootdown.fallback,
        waits.tlb_shootdown.timeout_events,
        waits.iommu_mmio.component,
        waits.iommu_mmio.operation,
        waits.iommu_mmio.max_spins,
        waits.iommu_mmio.fallback,
        waits.iommu_mmio.timeout_events
    );
}
