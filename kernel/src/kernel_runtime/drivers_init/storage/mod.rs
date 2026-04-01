mod inventory;
mod logging;
mod plan;

pub(super) fn init_storage_drivers(
    devices: &[hypercore::hal::pci::PciDevice],
    telemetry_drivers: bool,
) {
    if telemetry_drivers {
        plan::log_storage_probe_plan();
    }

    hypercore::modules::drivers::StorageManager::init_global(devices);
    let global_lock = hypercore::modules::drivers::StorageManager::global();
    let mut storage = global_lock.lock();
    let Some(storage) = storage.as_mut() else {
        hypercore::klog_warn!("StorageManager global state was not initialized");
        return;
    };

    let infos = storage.infos_vec();
    let lifecycle = storage.lifecycle_summary();
    let probe_report = storage.probe_report();
    let block_stats = hypercore::modules::drivers::block::stats();

    if telemetry_drivers {
        logging::log_storage_probe_report(&probe_report);
        logging::log_storage_driver_stats(infos.len(), &block_stats);
        logging::log_storage_lifecycle(&lifecycle);
        logging::log_driver_wait_policy();
    }

    inventory::log_storage_inventory(&infos, telemetry_drivers);

    let _ = storage.first_by_kind(hypercore::modules::drivers::BlockDriverKind::Nvme);
}
