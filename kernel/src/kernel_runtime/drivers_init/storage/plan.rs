pub(super) fn log_storage_probe_plan() {
    let plan = aethercore::modules::drivers::StorageManager::probe_plan();
    for step in plan {
        aethercore::klog_info!(
            "Storage probe step: order={} name={} kind={:?} dep={:?}",
            step.order,
            step.name,
            step.kind,
            step.dependency
        );
    }
}
