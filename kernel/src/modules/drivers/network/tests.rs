use super::*;

fn reset_network_test_state() {
    clear_active_driver();
    apply_poll_profile(NetworkPollProfile::Balanced);
    configure_ring_limit(NetworkDriverConfig::default().virtio_ring_limit);
    set_slo_thresholds(NetworkDriverSloThresholds {
        max_drop_rate_per_mille: DEFAULT_MAX_DROP_RATE_PER_MILLE,
        max_tx_ring_utilization_percent: DEFAULT_MAX_TX_RING_UTIL_PERCENT,
        max_rx_ring_utilization_percent: DEFAULT_MAX_RX_RING_UTIL_PERCENT,
        max_driver_io_errors: DEFAULT_MAX_DRIVER_IO_ERRORS,
    });
    VIRTIO_RX_RING.lock().clear();
    VIRTIO_TX_RING.lock().clear();
    E1000_RX_RING.lock().clear();
    E1000_TX_RING.lock().clear();
    TX_TO_NIC_FRAMES.store(0, Ordering::Relaxed);
    TX_TO_NIC_DROPS.store(0, Ordering::Relaxed);
    RX_TO_CORE_FRAMES.store(0, Ordering::Relaxed);
    RX_TO_CORE_DROPS.store(0, Ordering::Relaxed);
}

#[test_case]
fn active_driver_registration_switches_state() {
    reset_network_test_state();
    register_virtio();
    assert_eq!(active_driver(), ActiveNetworkDriver::VirtIo);

    register_e1000();
    assert_eq!(active_driver(), ActiveNetworkDriver::E1000);
}

#[test_case]
fn injected_rx_is_transferred_into_core_queue() {
    reset_network_test_state();
    register_virtio();
    let _ = inject_rx_frame(alloc::vec::Vec::from([0xde, 0xad, 0xbe, 0xef]));
    service_queues();

    let frame = crate::kernel::net_core::take_rx_frame();
    assert!(frame.is_some());
}

#[test_case]
fn poll_profile_application_updates_budgets() {
    reset_network_test_state();
    apply_poll_profile(NetworkPollProfile::Throughput);
    let cfg = get_config();
    assert!(cfg.irq_service_budget >= 128);
    assert!(cfg.loop_service_budget >= 256);
    assert!(cfg.virtio_ring_limit >= 1024);
}

#[test_case]
fn active_queue_clear_works() {
    reset_network_test_state();
    register_virtio();
    {
        let mut rx = VIRTIO_RX_RING.lock();
        rx.push_back(alloc::vec![1, 2, 3]);
        let mut tx = VIRTIO_TX_RING.lock();
        tx.push_back(alloc::vec![4, 5, 6]);
    }
    let summary = clear_active_driver_queues();
    assert!(summary.cleared_virtio_rx >= 1);
    assert!(summary.cleared_virtio_tx >= 1);
    assert_eq!(VIRTIO_RX_RING.lock().len(), 0);
    assert_eq!(VIRTIO_TX_RING.lock().len(), 0);
}

#[test_case]
fn rx_injection_respects_ring_limits_under_fault_load() {
    reset_network_test_state();
    register_e1000();
    configure_ring_limit(1);
    assert!(inject_rx_frame(alloc::vec![1, 2, 3]).is_ok());
    assert!(inject_rx_frame(alloc::vec![4, 5, 6]).is_err());
}

#[test_case]
fn slo_report_flags_breaches_when_faults_are_injected() {
    reset_network_test_state();
    register_virtio();
    configure_ring_limit(1);
    set_slo_thresholds(NetworkDriverSloThresholds {
        max_drop_rate_per_mille: 0,
        max_tx_ring_utilization_percent: 50,
        max_rx_ring_utilization_percent: 50,
        max_driver_io_errors: u64::MAX,
    });
    TX_TO_NIC_DROPS.store(8, Ordering::Relaxed);
    RX_TO_CORE_DROPS.store(2, Ordering::Relaxed);
    VIRTIO_TX_RING.lock().push_back(alloc::vec![0xaa]);
    VIRTIO_RX_RING.lock().push_back(alloc::vec![0xbb]);

    let report = slo_report();
    assert!(report.drop_rate_breach);
    assert!(report.tx_ring_breach);
    assert!(report.rx_ring_breach);
    assert!(report.breach_count >= 3);
}

#[test_case]
fn io_health_harness_escalates_from_rebind_to_failover() {
    let mut harness = NetworkIoHealthHarness::new(3, 2);

    assert_eq!(
        harness.observe_service_result(false),
        NetworkIoHealthAction::NoAction
    );
    assert_eq!(
        harness.observe_service_result(false),
        NetworkIoHealthAction::NoAction
    );
    assert_eq!(
        harness.observe_service_result(false),
        NetworkIoHealthAction::AttemptRebind
    );

    assert_eq!(
        harness.observe_rebind_result(false),
        NetworkIoHealthAction::NoAction
    );

    assert_eq!(
        harness.observe_service_result(false),
        NetworkIoHealthAction::NoAction
    );
    assert_eq!(
        harness.observe_service_result(false),
        NetworkIoHealthAction::NoAction
    );
    assert_eq!(
        harness.observe_service_result(false),
        NetworkIoHealthAction::AttemptRebind
    );

    assert_eq!(
        harness.observe_rebind_result(false),
        NetworkIoHealthAction::TriggerFailover
    );
}

#[test_case]
fn io_health_action_threshold_order_prefers_failover() {
    assert_eq!(
        evaluate_network_io_health_action(10, 2, 3, 2),
        NetworkIoHealthAction::TriggerFailover
    );
    assert_eq!(
        evaluate_network_io_health_action(3, 0, 3, 2),
        NetworkIoHealthAction::AttemptRebind
    );
}
