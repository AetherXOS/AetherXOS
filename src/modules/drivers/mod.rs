pub mod ahci;
pub mod block;
pub mod catalog;
pub mod config;
pub mod e1000;
pub mod families;
pub mod framebuffer;
pub mod lifecycle;
pub mod network;
pub mod network_io_health;
pub mod nvme;
pub mod policy;
pub mod probe;
pub mod ps2_keyboard;
pub mod registry;
pub mod storage;
pub mod virtio_block;
pub mod virtio_net;
pub mod wait;

// Re-exports
pub use block::{BlockDevice, BlockDeviceInfo, BlockDriverKind};
pub use catalog::{
    network_probe_plan, probe_first_network_driver, probe_first_network_driver_default_policy,
    probe_first_network_driver_plan_order, probe_network_driver_with_policy,
    probe_policy_fallback_kind, probe_policy_primary_kind, DriverBus, NetworkProbeStep,
    ProbeDependency, ProbedNetworkDriver,
};
pub use config::{
    driver_network_runtime_config, driver_storage_runtime_config, driver_wait_runtime_config,
    set_driver_network_runtime_config, set_driver_storage_runtime_config,
    set_driver_wait_runtime_config, DriverNetworkRuntimeConfig, DriverStorageRuntimeConfig,
    DriverWaitRuntimeConfig,
};
pub use e1000::{
    dataplane_stats as e1000_dataplane_stats, reset_dataplane_stats as reset_e1000_dataplane_stats,
    E1000DataplaneStats, E1000,
};
pub use framebuffer::{
    clear as framebuffer_clear, console_reset as framebuffer_console_reset,
    console_set_colors as framebuffer_set_colors, console_write as framebuffer_console_write,
    draw_char, fill_rect, info as framebuffer_info, init as framebuffer_init,
    is_initialized as framebuffer_is_initialized, put_pixel, scroll_up as framebuffer_scroll_up,
    stats as framebuffer_stats, Color, FramebufferInfo, FramebufferStats, PixelFormat,
};
pub use lifecycle::{
    DriverClass, DriverErrorKind, DriverHealth, DriverIoGate, DriverLifecycle,
    DriverRecoveryPolicy, DriverState, DriverStateMachine, DriverStatus, LifecycleAdapter,
    PciProbeDriver,
};
pub use network::{
    active_driver as active_network_driver, apply_poll_profile as apply_network_poll_profile,
    clear_active_driver as clear_active_network_driver,
    clear_active_driver_queues as clear_active_network_driver_queues,
    clear_driver_queues as clear_network_driver_queues,
    configure_ring_limit as configure_network_ring_limit,
    configure_service_budgets as configure_network_service_budgets,
    evaluate_network_io_health_action, has_active_driver as has_active_network_driver,
    inject_rx_frame as inject_network_rx_frame, poll_profile as network_poll_profile,
    register_e1000 as register_e1000_network_dataplane,
    register_virtio as register_virtio_network_dataplane, service_irq as service_network_irq,
    service_queues as service_network_queues, set_poll_profile as set_network_poll_profile,
    set_slo_thresholds as set_network_slo_thresholds, slo_report as network_slo_report,
    slo_thresholds as network_slo_thresholds, stats as network_dataplane_stats,
    ActiveNetworkDriver, NetworkDataplaneStats, NetworkDriverSloReport, NetworkDriverSloThresholds,
    NetworkIoHealthAction, NetworkIoHealthHarness, NetworkPollProfile, NetworkQueueResetSummary,
};
pub use nvme::{
    nvme_effective_io_queue_depth, nvme_io_queue_depth_override, nvme_queue_profile,
    set_nvme_io_queue_depth_override, set_nvme_queue_profile, NvmeQueueProfile,
};
pub use policy::{
    network_driver_policy, network_driver_policy_snapshot, network_remediation_profile,
    remediation_tuning_for_profile, set_network_driver_policy, set_network_remediation_profile,
    NetworkDriverPolicy, NetworkDriverPolicySnapshot, NetworkRemediationProfile,
    NetworkRemediationTuning,
};
pub use probe::{
    device_matches_any_pci_id, device_matches_pci_class, pci_bar0_io_base, pci_bar0_mmio_base,
    pci_class, pci_id, probe_first_pci_by_class, probe_first_pci_by_ids, PciClassCode, PciId,
};
pub use ps2_keyboard::{
    handle_irq as ps2_keyboard_irq, has_events as ps2_keyboard_has_events,
    init as ps2_keyboard_init, keycode_to_ascii, read_event as ps2_keyboard_read,
    stats as ps2_keyboard_stats, KeyEvent, KeyState, Keycode, LedState, ModifierState,
    Ps2KeyboardStats,
};
pub use registry::{
    clear_network_runtime_registry, has_e1000_runtime_driver, has_virtio_runtime_driver,
    hotplug_attach_network_driver, hotplug_detach_network_driver, latest_runtime_registry_event,
    note_policy_switch, note_quarantine, note_rebind_result, register_network_runtime_driver,
    runtime_registry_events, runtime_registry_snapshot, unregister_network_runtime_driver,
    with_e1000_runtime_driver_mut, with_virtio_runtime_driver_mut, DriverRuntimeEvent,
    DriverRuntimeEventKind, DriverRuntimeRegistrySnapshot, RUNTIME_REGISTRY_EVENT_CAPACITY,
};
pub use storage::{
    ProbedStorageDriver, StorageDependency, StorageLifecycleSummary, StorageManager,
    StorageProbeReport, StorageProbeStep,
};
pub use virtio_net::VirtIoNet;
pub use wait::{
    snapshot as wait_policy_snapshot, DriverWaitDescriptor, DriverWaitFallbackKind,
    DriverWaitPolicySnapshot,
};
