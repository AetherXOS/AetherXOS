use super::*;

#[test_case]
fn driver_network_runtime_profile_roundtrip_and_reset() {
    KernelConfig::reset_runtime_overrides();

    let profile = super::DriverNetworkRuntimeProfile {
        irq_service_budget: 31,
        loop_service_budget: 41,
        ring_limit: 1024,
        quarantine_rebind_failures: 123,
        quarantine_cooldown_samples: 456,
        slo_max_drop_rate_per_mille: 66,
        slo_max_tx_ring_utilization_percent: 81,
        slo_max_rx_ring_utilization_percent: 79,
        slo_max_io_errors: 7,
        low_latency_irq_budget_divisor: 3,
        low_latency_loop_budget_divisor: 4,
        low_latency_ring_limit_divisor: 5,
        throughput_irq_budget_multiplier: 6,
        throughput_loop_budget_multiplier: 7,
        throughput_ring_limit_multiplier: 8,
    };
    KernelConfig::set_driver_network_runtime_profile(Some(profile));

    let got = KernelConfig::driver_network_runtime_profile();
    assert_eq!(got, profile);

    KernelConfig::set_driver_network_runtime_profile(None);
    let reset = KernelConfig::driver_network_runtime_profile();
    assert_eq!(
        reset.irq_service_budget,
        crate::generated_consts::DRIVER_NETWORK_IRQ_SERVICE_BUDGET
    );
    assert_eq!(
        reset.loop_service_budget,
        crate::generated_consts::DRIVER_NETWORK_LOOP_SERVICE_BUDGET
    );
    assert_eq!(
        reset.ring_limit,
        crate::generated_consts::DRIVER_NETWORK_RING_LIMIT
    );
    assert_eq!(
        reset.quarantine_rebind_failures,
        crate::generated_consts::DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES
    );
    assert_eq!(
        reset.quarantine_cooldown_samples,
        crate::generated_consts::DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES
    );
    assert_eq!(
        reset.slo_max_drop_rate_per_mille,
        crate::generated_consts::DRIVER_NETWORK_SLO_MAX_DROP_RATE_PER_MILLE
    );
    assert_eq!(
        reset.slo_max_tx_ring_utilization_percent,
        crate::generated_consts::DRIVER_NETWORK_SLO_MAX_TX_RING_UTILIZATION_PERCENT
    );
    assert_eq!(
        reset.slo_max_rx_ring_utilization_percent,
        crate::generated_consts::DRIVER_NETWORK_SLO_MAX_RX_RING_UTILIZATION_PERCENT
    );
    assert_eq!(
        reset.slo_max_io_errors,
        crate::generated_consts::DRIVER_NETWORK_SLO_MAX_IO_ERRORS
    );
    assert_eq!(
        reset.low_latency_irq_budget_divisor,
        crate::generated_consts::DRIVER_NETWORK_LOW_LATENCY_IRQ_BUDGET_DIVISOR
    );
    assert_eq!(
        reset.low_latency_loop_budget_divisor,
        crate::generated_consts::DRIVER_NETWORK_LOW_LATENCY_LOOP_BUDGET_DIVISOR
    );
    assert_eq!(
        reset.low_latency_ring_limit_divisor,
        crate::generated_consts::DRIVER_NETWORK_LOW_LATENCY_RING_LIMIT_DIVISOR
    );
    assert_eq!(
        reset.throughput_irq_budget_multiplier,
        crate::generated_consts::DRIVER_NETWORK_THROUGHPUT_IRQ_BUDGET_MULTIPLIER
    );
    assert_eq!(
        reset.throughput_loop_budget_multiplier,
        crate::generated_consts::DRIVER_NETWORK_THROUGHPUT_LOOP_BUDGET_MULTIPLIER
    );
    assert_eq!(
        reset.throughput_ring_limit_multiplier,
        crate::generated_consts::DRIVER_NETWORK_THROUGHPUT_RING_LIMIT_MULTIPLIER
    );
}
