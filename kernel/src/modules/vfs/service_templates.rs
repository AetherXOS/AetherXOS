#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageServicePreset {
    ThroughputHeavy,
    Balanced,
    LowLatency,
}

pub fn recommended_storage_preset() -> StorageServicePreset {
    preset_from_pressure_snapshot(crate::kernel::pressure::snapshot())
}

pub fn recommended_io_policy() -> crate::modules::vfs::IoPolicy {
    io_policy_for_preset(recommended_storage_preset())
}

#[cfg(feature = "vfs_disk_fs")]
pub fn apply_recommended_io_policy(fs: &mut crate::modules::vfs::disk_fs::DiskFsLibrary) {
    fs.set_io_policy(recommended_io_policy());
}

fn preset_from_pressure_snapshot(
    pressure: crate::kernel::pressure::CorePressureSnapshot,
) -> StorageServicePreset {
    if pressure.scheduler_class == crate::kernel::pressure::SchedulerPressureClass::Critical {
        if pressure.rt_starvation_alert {
            return StorageServicePreset::LowLatency;
        }
        return StorageServicePreset::ThroughputHeavy;
    }

    match pressure.class {
        crate::kernel::pressure::CorePressureClass::Critical => {
            StorageServicePreset::ThroughputHeavy
        }
        crate::kernel::pressure::CorePressureClass::High => {
            if pressure.rt_starvation_alert {
                StorageServicePreset::LowLatency
            } else {
                StorageServicePreset::ThroughputHeavy
            }
        }
        crate::kernel::pressure::CorePressureClass::Elevated => StorageServicePreset::Balanced,
        crate::kernel::pressure::CorePressureClass::Nominal => {
            if pressure.runqueue_total == 0 && pressure.net_saturation_percent < 20 {
                StorageServicePreset::Balanced
            } else {
                StorageServicePreset::LowLatency
            }
        }
    }
}

pub fn io_policy_for_preset(preset: StorageServicePreset) -> crate::modules::vfs::IoPolicy {
    match preset {
        StorageServicePreset::ThroughputHeavy => crate::modules::vfs::IoPolicy::Unbuffered,
        StorageServicePreset::Balanced => crate::modules::vfs::IoPolicy::Buffered,
        StorageServicePreset::LowLatency => crate::modules::vfs::IoPolicy::Unbuffered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::pressure::{
        CorePressureClass, CorePressureSnapshot, SchedulerPressureClass,
    };

    #[test_case]
    fn pressure_to_storage_preset_prefers_throughput_on_critical() {
        let pressure = CorePressureSnapshot {
            schema_version: crate::kernel::pressure::CORE_PRESSURE_SCHEMA_VERSION,
            online_cpus: 4,
            runqueue_total: 28,
            runqueue_max: 16,
            runqueue_avg_milli: 7000,
            rt_starvation_alert: false,
            rt_forced_reschedules: 9,
            watchdog_stall_detections: 0,
            net_queue_limit: 1024,
            net_rx_depth: 900,
            net_tx_depth: 700,
            net_saturation_percent: 87,
            lb_imbalance_p50: 4,
            lb_imbalance_p90: 8,
            lb_imbalance_p99: 16,
            lb_prefer_local_forced_moves: 0,
            class: CorePressureClass::Critical,
            scheduler_class: SchedulerPressureClass::Critical,
        };

        assert_eq!(
            preset_from_pressure_snapshot(pressure),
            StorageServicePreset::ThroughputHeavy
        );
    }

    #[test_case]
    fn pressure_to_storage_preset_prefers_latency_under_rt_alert() {
        let pressure = CorePressureSnapshot {
            schema_version: crate::kernel::pressure::CORE_PRESSURE_SCHEMA_VERSION,
            online_cpus: 2,
            runqueue_total: 4,
            runqueue_max: 3,
            runqueue_avg_milli: 2000,
            rt_starvation_alert: true,
            rt_forced_reschedules: 1,
            watchdog_stall_detections: 0,
            net_queue_limit: 1024,
            net_rx_depth: 200,
            net_tx_depth: 100,
            net_saturation_percent: 20,
            lb_imbalance_p50: 2,
            lb_imbalance_p90: 4,
            lb_imbalance_p99: 6,
            lb_prefer_local_forced_moves: 0,
            class: CorePressureClass::High,
            scheduler_class: SchedulerPressureClass::Critical,
        };

        assert_eq!(
            preset_from_pressure_snapshot(pressure),
            StorageServicePreset::LowLatency
        );
    }

    #[test_case]
    fn storage_preset_maps_to_expected_io_policy() {
        assert_eq!(
            io_policy_for_preset(StorageServicePreset::Balanced),
            crate::modules::vfs::IoPolicy::Buffered
        );
        assert_eq!(
            io_policy_for_preset(StorageServicePreset::LowLatency),
            crate::modules::vfs::IoPolicy::Unbuffered
        );
    }
}
