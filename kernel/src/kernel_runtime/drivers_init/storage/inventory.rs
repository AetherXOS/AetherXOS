pub(super) fn log_storage_inventory(
    infos: &[aethercore::modules::drivers::StorageDriverInfo],
    telemetry_drivers: bool,
) {
    if !telemetry_drivers {
        return;
    }

    for info in infos {
        aethercore::klog_info!(
            "Storage device: kind={:?} base={:#x} irq={} block_size={}",
            info.kind,
            info.io_base,
            info.irq,
            info.block_size
        );
    }
}
