use super::nvme::NvmeQueueProfile;
use crate::config::KernelConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverNetworkRuntimeConfig {
    pub irq_service_budget: usize,
    pub loop_service_budget: usize,
    pub ring_limit: usize,
    pub quarantine_rebind_failures: u64,
    pub quarantine_cooldown_samples: u64,
    pub slo_max_drop_rate_per_mille: u64,
    pub slo_max_tx_ring_utilization_percent: u64,
    pub slo_max_rx_ring_utilization_percent: u64,
    pub slo_max_io_errors: u64,
    pub low_latency_irq_budget_divisor: usize,
    pub low_latency_loop_budget_divisor: usize,
    pub low_latency_ring_limit_divisor: usize,
    pub throughput_irq_budget_multiplier: usize,
    pub throughput_loop_budget_multiplier: usize,
    pub throughput_ring_limit_multiplier: usize,
    pub e1000_buffer_size_bytes: usize,
    pub e1000_rx_desc_count: usize,
    pub e1000_tx_desc_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverStorageRuntimeConfig {
    pub nvme_queue_profile: NvmeQueueProfile,
    pub nvme_effective_io_queue_depth: usize,
    pub nvme_io_queue_depth_override: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverWaitRuntimeConfig {
    pub ahci_io_timeout_spins: usize,
    pub nvme_disable_ready_timeout_spins: usize,
    pub nvme_poll_timeout_spins: usize,
    pub nvme_io_timeout_spins: usize,
    pub e1000_reset_timeout_spins: usize,
}

pub fn driver_network_runtime_config() -> DriverNetworkRuntimeConfig {
    DriverNetworkRuntimeConfig {
        irq_service_budget: KernelConfig::driver_network_irq_service_budget(),
        loop_service_budget: KernelConfig::driver_network_loop_service_budget(),
        ring_limit: KernelConfig::driver_network_ring_limit(),
        quarantine_rebind_failures: KernelConfig::driver_network_quarantine_rebind_failures(),
        quarantine_cooldown_samples: KernelConfig::driver_network_quarantine_cooldown_samples(),
        slo_max_drop_rate_per_mille: KernelConfig::driver_network_slo_max_drop_rate_per_mille(),
        slo_max_tx_ring_utilization_percent:
            KernelConfig::driver_network_slo_max_tx_ring_utilization_percent(),
        slo_max_rx_ring_utilization_percent:
            KernelConfig::driver_network_slo_max_rx_ring_utilization_percent(),
        slo_max_io_errors: KernelConfig::driver_network_slo_max_io_errors(),
        low_latency_irq_budget_divisor: KernelConfig::driver_network_low_latency_irq_budget_divisor(
        ),
        low_latency_loop_budget_divisor:
            KernelConfig::driver_network_low_latency_loop_budget_divisor(),
        low_latency_ring_limit_divisor: KernelConfig::driver_network_low_latency_ring_limit_divisor(
        ),
        throughput_irq_budget_multiplier:
            KernelConfig::driver_network_throughput_irq_budget_multiplier(),
        throughput_loop_budget_multiplier:
            KernelConfig::driver_network_throughput_loop_budget_multiplier(),
        throughput_ring_limit_multiplier:
            KernelConfig::driver_network_throughput_ring_limit_multiplier(),
        e1000_buffer_size_bytes: KernelConfig::e1000_buffer_size_bytes(),
        e1000_rx_desc_count: KernelConfig::e1000_rx_desc_count(),
        e1000_tx_desc_count: KernelConfig::e1000_tx_desc_count(),
    }
}

pub fn set_driver_network_runtime_config(config: DriverNetworkRuntimeConfig) {
    KernelConfig::set_driver_network_irq_service_budget(Some(config.irq_service_budget));
    KernelConfig::set_driver_network_loop_service_budget(Some(config.loop_service_budget));
    KernelConfig::set_driver_network_ring_limit(Some(config.ring_limit));
    KernelConfig::set_driver_network_quarantine_rebind_failures(Some(
        config.quarantine_rebind_failures,
    ));
    KernelConfig::set_driver_network_quarantine_cooldown_samples(Some(
        config.quarantine_cooldown_samples,
    ));
    KernelConfig::set_driver_network_slo_max_drop_rate_per_mille(Some(
        config.slo_max_drop_rate_per_mille,
    ));
    KernelConfig::set_driver_network_slo_max_tx_ring_utilization_percent(Some(
        config.slo_max_tx_ring_utilization_percent,
    ));
    KernelConfig::set_driver_network_slo_max_rx_ring_utilization_percent(Some(
        config.slo_max_rx_ring_utilization_percent,
    ));
    KernelConfig::set_driver_network_slo_max_io_errors(Some(config.slo_max_io_errors));
    KernelConfig::set_driver_network_low_latency_irq_budget_divisor(Some(
        config.low_latency_irq_budget_divisor,
    ));
    KernelConfig::set_driver_network_low_latency_loop_budget_divisor(Some(
        config.low_latency_loop_budget_divisor,
    ));
    KernelConfig::set_driver_network_low_latency_ring_limit_divisor(Some(
        config.low_latency_ring_limit_divisor,
    ));
    KernelConfig::set_driver_network_throughput_irq_budget_multiplier(Some(
        config.throughput_irq_budget_multiplier,
    ));
    KernelConfig::set_driver_network_throughput_loop_budget_multiplier(Some(
        config.throughput_loop_budget_multiplier,
    ));
    KernelConfig::set_driver_network_throughput_ring_limit_multiplier(Some(
        config.throughput_ring_limit_multiplier,
    ));
    KernelConfig::set_e1000_buffer_size_bytes(Some(config.e1000_buffer_size_bytes));
    KernelConfig::set_e1000_rx_desc_count(Some(config.e1000_rx_desc_count));
    KernelConfig::set_e1000_tx_desc_count(Some(config.e1000_tx_desc_count));
    super::network::apply_runtime_config_from_kernel_config();
}

pub fn driver_storage_runtime_config() -> DriverStorageRuntimeConfig {
    DriverStorageRuntimeConfig {
        nvme_queue_profile: super::nvme::nvme_queue_profile(),
        nvme_effective_io_queue_depth: super::nvme::nvme_effective_io_queue_depth(),
        nvme_io_queue_depth_override: super::nvme::nvme_io_queue_depth_override(),
    }
}

pub fn set_driver_storage_runtime_config(config: DriverStorageRuntimeConfig) {
    super::nvme::set_nvme_queue_profile(config.nvme_queue_profile);
    super::nvme::set_nvme_io_queue_depth_override(config.nvme_io_queue_depth_override);
}

pub fn driver_wait_runtime_config() -> DriverWaitRuntimeConfig {
    DriverWaitRuntimeConfig {
        ahci_io_timeout_spins: KernelConfig::ahci_io_timeout_spins(),
        nvme_disable_ready_timeout_spins: KernelConfig::nvme_disable_ready_timeout_spins(),
        nvme_poll_timeout_spins: KernelConfig::nvme_poll_timeout_spins(),
        nvme_io_timeout_spins: KernelConfig::nvme_io_timeout_spins(),
        e1000_reset_timeout_spins: KernelConfig::e1000_reset_timeout_spins(),
    }
}

pub fn set_driver_wait_runtime_config(config: DriverWaitRuntimeConfig) {
    KernelConfig::set_ahci_io_timeout_spins(Some(config.ahci_io_timeout_spins));
    KernelConfig::set_nvme_disable_ready_timeout_spins(Some(
        config.nvme_disable_ready_timeout_spins,
    ));
    KernelConfig::set_nvme_poll_timeout_spins(Some(config.nvme_poll_timeout_spins));
    KernelConfig::set_nvme_io_timeout_spins(Some(config.nvme_io_timeout_spins));
    KernelConfig::set_e1000_reset_timeout_spins(Some(config.e1000_reset_timeout_spins));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn driver_network_runtime_config_roundtrip_and_clamp() {
        KernelConfig::reset_runtime_overrides();
        let original = driver_network_runtime_config();

        set_driver_network_runtime_config(DriverNetworkRuntimeConfig {
            irq_service_budget: usize::MAX,
            loop_service_budget: usize::MAX,
            ring_limit: usize::MAX,
            quarantine_rebind_failures: u64::MAX,
            quarantine_cooldown_samples: u64::MAX,
            slo_max_drop_rate_per_mille: u64::MAX,
            slo_max_tx_ring_utilization_percent: u64::MAX,
            slo_max_rx_ring_utilization_percent: u64::MAX,
            slo_max_io_errors: u64::MAX,
            low_latency_irq_budget_divisor: usize::MAX,
            low_latency_loop_budget_divisor: usize::MAX,
            low_latency_ring_limit_divisor: usize::MAX,
            throughput_irq_budget_multiplier: usize::MAX,
            throughput_loop_budget_multiplier: usize::MAX,
            throughput_ring_limit_multiplier: usize::MAX,
            e1000_buffer_size_bytes: usize::MAX,
            e1000_rx_desc_count: usize::MAX,
            e1000_tx_desc_count: usize::MAX,
        });
        let after = driver_network_runtime_config();
        assert_eq!(after.irq_service_budget, 65_536);
        assert_eq!(after.loop_service_budget, 65_536);
        assert_eq!(after.ring_limit, 65_536);
        assert_eq!(after.quarantine_rebind_failures, 1_000_000);
        assert_eq!(after.quarantine_cooldown_samples, 1_000_000);
        assert_eq!(after.slo_max_drop_rate_per_mille, 1000);
        assert_eq!(after.slo_max_tx_ring_utilization_percent, 100);
        assert_eq!(after.slo_max_rx_ring_utilization_percent, 100);
        assert_eq!(after.slo_max_io_errors, u64::MAX);
        assert_eq!(after.low_latency_irq_budget_divisor, 1024);
        assert_eq!(after.low_latency_loop_budget_divisor, 1024);
        assert_eq!(after.low_latency_ring_limit_divisor, 1024);
        assert_eq!(after.throughput_irq_budget_multiplier, 1024);
        assert_eq!(after.throughput_loop_budget_multiplier, 1024);
        assert_eq!(after.throughput_ring_limit_multiplier, 1024);
        assert_eq!(after.e1000_buffer_size_bytes, 16_384);
        assert_eq!(after.e1000_rx_desc_count, 4096);
        assert_eq!(after.e1000_tx_desc_count, 4096);
        let live = super::super::network::get_config();
        assert_eq!(live.irq_service_budget, after.irq_service_budget);
        assert_eq!(live.loop_service_budget, after.loop_service_budget);
        assert_eq!(live.virtio_ring_limit, after.ring_limit);
        assert_eq!(live.e1000_ring_limit, after.ring_limit);
        let live_slo = super::super::network::slo_thresholds();
        assert_eq!(live_slo.max_drop_rate_per_mille, 1000);
        assert_eq!(live_slo.max_tx_ring_utilization_percent, 100);
        assert_eq!(live_slo.max_rx_ring_utilization_percent, 100);
        assert_eq!(live_slo.max_driver_io_errors, u64::MAX);

        KernelConfig::reset_runtime_overrides();
        assert_eq!(driver_network_runtime_config(), original);
    }

    #[test_case]
    fn driver_storage_runtime_config_roundtrip() {
        let original = driver_storage_runtime_config();
        set_driver_storage_runtime_config(DriverStorageRuntimeConfig {
            nvme_queue_profile: NvmeQueueProfile::Throughput,
            nvme_effective_io_queue_depth: usize::MAX,
            nvme_io_queue_depth_override: Some(9999),
        });
        let after = driver_storage_runtime_config();
        assert_eq!(after.nvme_queue_profile, NvmeQueueProfile::Throughput);
        assert_eq!(after.nvme_effective_io_queue_depth, 1024);
        assert_eq!(after.nvme_io_queue_depth_override, Some(9999));
        set_driver_storage_runtime_config(original);
    }

    #[test_case]
    fn driver_wait_runtime_config_roundtrip_and_clamp() {
        KernelConfig::reset_runtime_overrides();
        let original = driver_wait_runtime_config();
        set_driver_wait_runtime_config(DriverWaitRuntimeConfig {
            ahci_io_timeout_spins: usize::MAX,
            nvme_disable_ready_timeout_spins: usize::MAX,
            nvme_poll_timeout_spins: usize::MAX,
            nvme_io_timeout_spins: usize::MAX,
            e1000_reset_timeout_spins: usize::MAX,
        });
        let after = driver_wait_runtime_config();
        assert_eq!(after.ahci_io_timeout_spins, 100_000_000);
        assert_eq!(after.nvme_disable_ready_timeout_spins, 100_000_000);
        assert_eq!(after.nvme_poll_timeout_spins, 100_000_000);
        assert_eq!(after.nvme_io_timeout_spins, 100_000_000);
        assert_eq!(after.e1000_reset_timeout_spins, 100_000_000);
        assert_eq!(
            super::super::ahci::wait_stats().io_timeout_spins,
            after.ahci_io_timeout_spins
        );
        assert_eq!(
            super::super::nvme::wait_stats().io_timeout_spins,
            after.nvme_io_timeout_spins
        );
        assert_eq!(
            super::super::e1000::wait_stats().reset_timeout_spins,
            after.e1000_reset_timeout_spins
        );

        KernelConfig::reset_runtime_overrides();
        assert_eq!(driver_wait_runtime_config(), original);
    }
}
