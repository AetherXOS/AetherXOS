use hypercore::config::KernelConfig;
use serial_test::serial;

use crate::common::ctx;

#[test]
#[serial]
fn runtime_limits_snapshot_remains_non_zero_and_consistent() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();
    let limits = KernelConfig::runtime_limits();

    assert!(limits.watchdog_hard_stall_ns > 0);
    assert!(limits.module_loader_max_load_segments > 0);
    assert!(limits.module_loader_max_total_image_bytes > 0);
    assert!(limits.launch_max_process_name_len > 0);
    assert!(limits.launch_max_boot_image_bytes > 0);
    assert!(limits.launch_handoff_stage_timeout_epochs > 0);
    assert!(limits.vfs_max_mounts > 0);
    assert!(limits.vfs_max_mount_path >= 2);
    assert!(limits.irqsafe_mutex_deadlock_spin_limit > 0);
    assert!(KernelConfig::time_slice() > 0);
    assert!(KernelConfig::stack_size() > 0);
}

#[test]
#[serial]
fn runtime_override_reset_restores_default_bounds() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();

    let default_history_len = KernelConfig::telemetry_history_len();
    let default_mount_path = KernelConfig::vfs_max_mount_path();

    KernelConfig::set_telemetry_history_len(Some(usize::MAX));
    KernelConfig::set_vfs_max_mount_path(Some(1));

    assert_eq!(KernelConfig::telemetry_history_len(), 1_000_000);
    assert_eq!(KernelConfig::vfs_max_mount_path(), 2);

    KernelConfig::reset_runtime_overrides();

    assert_eq!(KernelConfig::telemetry_history_len(), default_history_len);
    assert_eq!(KernelConfig::vfs_max_mount_path(), default_mount_path);
}
