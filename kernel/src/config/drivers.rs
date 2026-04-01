use core::sync::atomic::Ordering;

use super::{
    apply_profile_override, DriverNetworkRuntimeProfile, KernelConfig,
    DEFAULT_DRIVER_NETWORK_IRQ_SERVICE_BUDGET, DEFAULT_DRIVER_NETWORK_LOOP_SERVICE_BUDGET,
    DEFAULT_DRIVER_NETWORK_LOW_LATENCY_IRQ_BUDGET_DIVISOR,
    DEFAULT_DRIVER_NETWORK_LOW_LATENCY_LOOP_BUDGET_DIVISOR,
    DEFAULT_DRIVER_NETWORK_LOW_LATENCY_RING_LIMIT_DIVISOR,
    DEFAULT_DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES,
    DEFAULT_DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES, DEFAULT_DRIVER_NETWORK_RING_LIMIT,
    DEFAULT_DRIVER_NETWORK_SLO_MAX_DROP_RATE_PER_MILLE, DEFAULT_DRIVER_NETWORK_SLO_MAX_IO_ERRORS,
    DEFAULT_DRIVER_NETWORK_SLO_MAX_RX_RING_UTILIZATION_PERCENT,
    DEFAULT_DRIVER_NETWORK_SLO_MAX_TX_RING_UTILIZATION_PERCENT,
    DEFAULT_DRIVER_NETWORK_THROUGHPUT_IRQ_BUDGET_MULTIPLIER,
    DEFAULT_DRIVER_NETWORK_THROUGHPUT_LOOP_BUDGET_MULTIPLIER,
    DEFAULT_DRIVER_NETWORK_THROUGHPUT_RING_LIMIT_MULTIPLIER,
    DRIVER_NETWORK_IRQ_SERVICE_BUDGET_OVERRIDE, DRIVER_NETWORK_LOOP_SERVICE_BUDGET_OVERRIDE,
    DRIVER_NETWORK_LOW_LATENCY_IRQ_BUDGET_DIVISOR_OVERRIDE,
    DRIVER_NETWORK_LOW_LATENCY_LOOP_BUDGET_DIVISOR_OVERRIDE,
    DRIVER_NETWORK_LOW_LATENCY_RING_LIMIT_DIVISOR_OVERRIDE,
    DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES_OVERRIDE,
    DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES_OVERRIDE, DRIVER_NETWORK_RING_LIMIT_OVERRIDE,
    DRIVER_NETWORK_SLO_MAX_DROP_RATE_PER_MILLE_OVERRIDE, DRIVER_NETWORK_SLO_MAX_IO_ERRORS_OVERRIDE,
    DRIVER_NETWORK_SLO_MAX_RX_RING_UTILIZATION_PERCENT_OVERRIDE,
    DRIVER_NETWORK_SLO_MAX_TX_RING_UTILIZATION_PERCENT_OVERRIDE,
    DRIVER_NETWORK_THROUGHPUT_IRQ_BUDGET_MULTIPLIER_OVERRIDE,
    DRIVER_NETWORK_THROUGHPUT_LOOP_BUDGET_MULTIPLIER_OVERRIDE,
    DRIVER_NETWORK_THROUGHPUT_RING_LIMIT_MULTIPLIER_OVERRIDE,
    MAX_DRIVER_NETWORK_PROFILE_TUNING_FACTOR, MAX_DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES,
    MAX_DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES, MAX_DRIVER_NETWORK_RING_LIMIT,
    MAX_DRIVER_NETWORK_SERVICE_BUDGET,
};

impl KernelConfig {
    pub fn driver_network_irq_service_budget() -> usize {
        let override_value = DRIVER_NETWORK_IRQ_SERVICE_BUDGET_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_IRQ_SERVICE_BUDGET
        } else {
            override_value.clamp(1, MAX_DRIVER_NETWORK_SERVICE_BUDGET)
        }
    }

    pub fn set_driver_network_irq_service_budget(value: Option<usize>) {
        DRIVER_NETWORK_IRQ_SERVICE_BUDGET_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn driver_network_loop_service_budget() -> usize {
        let override_value = DRIVER_NETWORK_LOOP_SERVICE_BUDGET_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_LOOP_SERVICE_BUDGET
        } else {
            override_value.clamp(1, MAX_DRIVER_NETWORK_SERVICE_BUDGET)
        }
    }

    pub fn set_driver_network_loop_service_budget(value: Option<usize>) {
        DRIVER_NETWORK_LOOP_SERVICE_BUDGET_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn driver_network_ring_limit() -> usize {
        let override_value = DRIVER_NETWORK_RING_LIMIT_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_RING_LIMIT
        } else {
            override_value.clamp(1, MAX_DRIVER_NETWORK_RING_LIMIT)
        }
    }

    pub fn set_driver_network_ring_limit(value: Option<usize>) {
        DRIVER_NETWORK_RING_LIMIT_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn driver_network_quarantine_rebind_failures() -> u64 {
        let override_value =
            DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES
        } else {
            override_value.clamp(1, MAX_DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES)
        }
    }

    pub fn driver_network_quarantine_cooldown_samples() -> u64 {
        let override_value =
            DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES
        } else {
            override_value.clamp(1, MAX_DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES)
        }
    }

    pub fn set_driver_network_quarantine_rebind_failures(value: Option<u64>) {
        DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_driver_network_quarantine_cooldown_samples(value: Option<u64>) {
        DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn driver_network_slo_max_drop_rate_per_mille() -> u64 {
        let override_value =
            DRIVER_NETWORK_SLO_MAX_DROP_RATE_PER_MILLE_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_SLO_MAX_DROP_RATE_PER_MILLE
        } else {
            override_value.min(1000)
        }
    }

    pub fn driver_network_slo_max_tx_ring_utilization_percent() -> u64 {
        let override_value =
            DRIVER_NETWORK_SLO_MAX_TX_RING_UTILIZATION_PERCENT_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_SLO_MAX_TX_RING_UTILIZATION_PERCENT
        } else {
            override_value.min(100)
        }
    }

    pub fn driver_network_slo_max_rx_ring_utilization_percent() -> u64 {
        let override_value =
            DRIVER_NETWORK_SLO_MAX_RX_RING_UTILIZATION_PERCENT_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_SLO_MAX_RX_RING_UTILIZATION_PERCENT
        } else {
            override_value.min(100)
        }
    }

    pub fn driver_network_slo_max_io_errors() -> u64 {
        let override_value = DRIVER_NETWORK_SLO_MAX_IO_ERRORS_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_SLO_MAX_IO_ERRORS
        } else {
            override_value
        }
    }

    pub fn set_driver_network_slo_max_drop_rate_per_mille(value: Option<u64>) {
        DRIVER_NETWORK_SLO_MAX_DROP_RATE_PER_MILLE_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_driver_network_slo_max_tx_ring_utilization_percent(value: Option<u64>) {
        DRIVER_NETWORK_SLO_MAX_TX_RING_UTILIZATION_PERCENT_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_driver_network_slo_max_rx_ring_utilization_percent(value: Option<u64>) {
        DRIVER_NETWORK_SLO_MAX_RX_RING_UTILIZATION_PERCENT_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_driver_network_slo_max_io_errors(value: Option<u64>) {
        DRIVER_NETWORK_SLO_MAX_IO_ERRORS_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn driver_network_low_latency_irq_budget_divisor() -> usize {
        let override_value =
            DRIVER_NETWORK_LOW_LATENCY_IRQ_BUDGET_DIVISOR_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_LOW_LATENCY_IRQ_BUDGET_DIVISOR
        } else {
            override_value.clamp(1, MAX_DRIVER_NETWORK_PROFILE_TUNING_FACTOR)
        }
    }

    pub fn driver_network_low_latency_loop_budget_divisor() -> usize {
        let override_value =
            DRIVER_NETWORK_LOW_LATENCY_LOOP_BUDGET_DIVISOR_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_LOW_LATENCY_LOOP_BUDGET_DIVISOR
        } else {
            override_value.clamp(1, MAX_DRIVER_NETWORK_PROFILE_TUNING_FACTOR)
        }
    }

    pub fn driver_network_low_latency_ring_limit_divisor() -> usize {
        let override_value =
            DRIVER_NETWORK_LOW_LATENCY_RING_LIMIT_DIVISOR_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_LOW_LATENCY_RING_LIMIT_DIVISOR
        } else {
            override_value.clamp(1, MAX_DRIVER_NETWORK_PROFILE_TUNING_FACTOR)
        }
    }

    pub fn driver_network_throughput_irq_budget_multiplier() -> usize {
        let override_value =
            DRIVER_NETWORK_THROUGHPUT_IRQ_BUDGET_MULTIPLIER_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_THROUGHPUT_IRQ_BUDGET_MULTIPLIER
        } else {
            override_value.clamp(1, MAX_DRIVER_NETWORK_PROFILE_TUNING_FACTOR)
        }
    }

    pub fn driver_network_throughput_loop_budget_multiplier() -> usize {
        let override_value =
            DRIVER_NETWORK_THROUGHPUT_LOOP_BUDGET_MULTIPLIER_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_THROUGHPUT_LOOP_BUDGET_MULTIPLIER
        } else {
            override_value.clamp(1, MAX_DRIVER_NETWORK_PROFILE_TUNING_FACTOR)
        }
    }

    pub fn driver_network_throughput_ring_limit_multiplier() -> usize {
        let override_value =
            DRIVER_NETWORK_THROUGHPUT_RING_LIMIT_MULTIPLIER_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DRIVER_NETWORK_THROUGHPUT_RING_LIMIT_MULTIPLIER
        } else {
            override_value.clamp(1, MAX_DRIVER_NETWORK_PROFILE_TUNING_FACTOR)
        }
    }

    pub fn set_driver_network_low_latency_irq_budget_divisor(value: Option<usize>) {
        DRIVER_NETWORK_LOW_LATENCY_IRQ_BUDGET_DIVISOR_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_driver_network_low_latency_loop_budget_divisor(value: Option<usize>) {
        DRIVER_NETWORK_LOW_LATENCY_LOOP_BUDGET_DIVISOR_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_driver_network_low_latency_ring_limit_divisor(value: Option<usize>) {
        DRIVER_NETWORK_LOW_LATENCY_RING_LIMIT_DIVISOR_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_driver_network_throughput_irq_budget_multiplier(value: Option<usize>) {
        DRIVER_NETWORK_THROUGHPUT_IRQ_BUDGET_MULTIPLIER_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_driver_network_throughput_loop_budget_multiplier(value: Option<usize>) {
        DRIVER_NETWORK_THROUGHPUT_LOOP_BUDGET_MULTIPLIER_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn set_driver_network_throughput_ring_limit_multiplier(value: Option<usize>) {
        DRIVER_NETWORK_THROUGHPUT_RING_LIMIT_MULTIPLIER_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn driver_network_runtime_profile() -> DriverNetworkRuntimeProfile {
        DriverNetworkRuntimeProfile {
            irq_service_budget: Self::driver_network_irq_service_budget(),
            loop_service_budget: Self::driver_network_loop_service_budget(),
            ring_limit: Self::driver_network_ring_limit(),
            quarantine_rebind_failures: Self::driver_network_quarantine_rebind_failures(),
            quarantine_cooldown_samples: Self::driver_network_quarantine_cooldown_samples(),
            slo_max_drop_rate_per_mille: Self::driver_network_slo_max_drop_rate_per_mille(),
            slo_max_tx_ring_utilization_percent:
                Self::driver_network_slo_max_tx_ring_utilization_percent(),
            slo_max_rx_ring_utilization_percent:
                Self::driver_network_slo_max_rx_ring_utilization_percent(),
            slo_max_io_errors: Self::driver_network_slo_max_io_errors(),
            low_latency_irq_budget_divisor: Self::driver_network_low_latency_irq_budget_divisor(),
            low_latency_loop_budget_divisor: Self::driver_network_low_latency_loop_budget_divisor(),
            low_latency_ring_limit_divisor: Self::driver_network_low_latency_ring_limit_divisor(),
            throughput_irq_budget_multiplier: Self::driver_network_throughput_irq_budget_multiplier(
            ),
            throughput_loop_budget_multiplier:
                Self::driver_network_throughput_loop_budget_multiplier(),
            throughput_ring_limit_multiplier: Self::driver_network_throughput_ring_limit_multiplier(
            ),
        }
    }

    pub fn driver_network_cargo_profile() -> DriverNetworkRuntimeProfile {
        DriverNetworkRuntimeProfile {
            irq_service_budget: DEFAULT_DRIVER_NETWORK_IRQ_SERVICE_BUDGET,
            loop_service_budget: DEFAULT_DRIVER_NETWORK_LOOP_SERVICE_BUDGET,
            ring_limit: DEFAULT_DRIVER_NETWORK_RING_LIMIT,
            quarantine_rebind_failures: DEFAULT_DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES,
            quarantine_cooldown_samples: DEFAULT_DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES,
            slo_max_drop_rate_per_mille: DEFAULT_DRIVER_NETWORK_SLO_MAX_DROP_RATE_PER_MILLE,
            slo_max_tx_ring_utilization_percent:
                DEFAULT_DRIVER_NETWORK_SLO_MAX_TX_RING_UTILIZATION_PERCENT,
            slo_max_rx_ring_utilization_percent:
                DEFAULT_DRIVER_NETWORK_SLO_MAX_RX_RING_UTILIZATION_PERCENT,
            slo_max_io_errors: DEFAULT_DRIVER_NETWORK_SLO_MAX_IO_ERRORS,
            low_latency_irq_budget_divisor: DEFAULT_DRIVER_NETWORK_LOW_LATENCY_IRQ_BUDGET_DIVISOR,
            low_latency_loop_budget_divisor: DEFAULT_DRIVER_NETWORK_LOW_LATENCY_LOOP_BUDGET_DIVISOR,
            low_latency_ring_limit_divisor: DEFAULT_DRIVER_NETWORK_LOW_LATENCY_RING_LIMIT_DIVISOR,
            throughput_irq_budget_multiplier:
                DEFAULT_DRIVER_NETWORK_THROUGHPUT_IRQ_BUDGET_MULTIPLIER,
            throughput_loop_budget_multiplier:
                DEFAULT_DRIVER_NETWORK_THROUGHPUT_LOOP_BUDGET_MULTIPLIER,
            throughput_ring_limit_multiplier:
                DEFAULT_DRIVER_NETWORK_THROUGHPUT_RING_LIMIT_MULTIPLIER,
        }
    }

    pub fn set_driver_network_runtime_profile(value: Option<DriverNetworkRuntimeProfile>) {
        apply_profile_override(
            value,
            |profile| {
                Self::set_driver_network_irq_service_budget(Some(profile.irq_service_budget));
                Self::set_driver_network_loop_service_budget(Some(profile.loop_service_budget));
                Self::set_driver_network_ring_limit(Some(profile.ring_limit));
                Self::set_driver_network_quarantine_rebind_failures(Some(
                    profile.quarantine_rebind_failures,
                ));
                Self::set_driver_network_quarantine_cooldown_samples(Some(
                    profile.quarantine_cooldown_samples,
                ));
                Self::set_driver_network_slo_max_drop_rate_per_mille(Some(
                    profile.slo_max_drop_rate_per_mille,
                ));
                Self::set_driver_network_slo_max_tx_ring_utilization_percent(Some(
                    profile.slo_max_tx_ring_utilization_percent,
                ));
                Self::set_driver_network_slo_max_rx_ring_utilization_percent(Some(
                    profile.slo_max_rx_ring_utilization_percent,
                ));
                Self::set_driver_network_slo_max_io_errors(Some(profile.slo_max_io_errors));
                Self::set_driver_network_low_latency_irq_budget_divisor(Some(
                    profile.low_latency_irq_budget_divisor,
                ));
                Self::set_driver_network_low_latency_loop_budget_divisor(Some(
                    profile.low_latency_loop_budget_divisor,
                ));
                Self::set_driver_network_low_latency_ring_limit_divisor(Some(
                    profile.low_latency_ring_limit_divisor,
                ));
                Self::set_driver_network_throughput_irq_budget_multiplier(Some(
                    profile.throughput_irq_budget_multiplier,
                ));
                Self::set_driver_network_throughput_loop_budget_multiplier(Some(
                    profile.throughput_loop_budget_multiplier,
                ));
                Self::set_driver_network_throughput_ring_limit_multiplier(Some(
                    profile.throughput_ring_limit_multiplier,
                ));
            },
            || {
                Self::set_driver_network_irq_service_budget(None);
                Self::set_driver_network_loop_service_budget(None);
                Self::set_driver_network_ring_limit(None);
                Self::set_driver_network_quarantine_rebind_failures(None);
                Self::set_driver_network_quarantine_cooldown_samples(None);
                Self::set_driver_network_slo_max_drop_rate_per_mille(None);
                Self::set_driver_network_slo_max_tx_ring_utilization_percent(None);
                Self::set_driver_network_slo_max_rx_ring_utilization_percent(None);
                Self::set_driver_network_slo_max_io_errors(None);
                Self::set_driver_network_low_latency_irq_budget_divisor(None);
                Self::set_driver_network_low_latency_loop_budget_divisor(None);
                Self::set_driver_network_low_latency_ring_limit_divisor(None);
                Self::set_driver_network_throughput_irq_budget_multiplier(None);
                Self::set_driver_network_throughput_loop_budget_multiplier(None);
                Self::set_driver_network_throughput_ring_limit_multiplier(None);
            },
        );
    }
}
