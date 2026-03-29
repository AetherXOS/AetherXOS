//! Driver config validation — IRQ budgets, timeouts, descriptor counts.

use crate::build_cfg::config_types::DriverConfig;

const MAX_SERVICE_BUDGET: usize = 65536;
const MAX_RING_LIMIT: usize = 65536;
const MAX_TIMEOUT_SPINS: usize = 100_000_000;
const MAX_E1000_BUFFER: usize = 16384;
const MAX_DESC_COUNT: usize = 4096;
const MIN_DESC_COUNT: usize = 8;

pub fn validate(c: &DriverConfig) -> Vec<String> {
    let mut e = Vec::new();

    if c.network_irq_service_budget == 0 || c.network_irq_service_budget > MAX_SERVICE_BUDGET {
        e.push(format!(
            "drivers.network_irq_service_budget {} out of range [1, {}]",
            c.network_irq_service_budget, MAX_SERVICE_BUDGET
        ));
    }
    if c.network_loop_service_budget == 0 || c.network_loop_service_budget > MAX_SERVICE_BUDGET {
        e.push(format!(
            "drivers.network_loop_service_budget {} out of range [1, {}]",
            c.network_loop_service_budget, MAX_SERVICE_BUDGET
        ));
    }
    if c.network_ring_limit == 0 || c.network_ring_limit > MAX_RING_LIMIT {
        e.push(format!(
            "drivers.network_ring_limit {} out of range [1, {}]",
            c.network_ring_limit, MAX_RING_LIMIT
        ));
    }
    if c.ahci_io_timeout_spins == 0 || c.ahci_io_timeout_spins > MAX_TIMEOUT_SPINS {
        e.push(format!(
            "drivers.ahci_io_timeout_spins {} out of range [1, {}]",
            c.ahci_io_timeout_spins, MAX_TIMEOUT_SPINS
        ));
    }
    if c.nvme_io_timeout_spins == 0 || c.nvme_io_timeout_spins > MAX_TIMEOUT_SPINS {
        e.push(format!(
            "drivers.nvme_io_timeout_spins {} out of range [1, {}]",
            c.nvme_io_timeout_spins, MAX_TIMEOUT_SPINS
        ));
    }
    if c.e1000_buffer_size_bytes < 256 || c.e1000_buffer_size_bytes > MAX_E1000_BUFFER {
        e.push(format!(
            "drivers.e1000_buffer_size_bytes {} out of range [256, {}]",
            c.e1000_buffer_size_bytes, MAX_E1000_BUFFER
        ));
    }
    if c.e1000_rx_desc_count < MIN_DESC_COUNT || c.e1000_rx_desc_count > MAX_DESC_COUNT {
        e.push(format!(
            "drivers.e1000_rx_desc_count {} out of range [{}, {}]",
            c.e1000_rx_desc_count, MIN_DESC_COUNT, MAX_DESC_COUNT
        ));
    }
    if c.e1000_tx_desc_count < MIN_DESC_COUNT || c.e1000_tx_desc_count > MAX_DESC_COUNT {
        e.push(format!(
            "drivers.e1000_tx_desc_count {} out of range [{}, {}]",
            c.e1000_tx_desc_count, MIN_DESC_COUNT, MAX_DESC_COUNT
        ));
    }
    // Divisors must not be zero (division by zero at runtime)
    if c.network_low_latency_irq_budget_divisor == 0 {
        e.push("drivers.network_low_latency_irq_budget_divisor must be > 0".to_string());
    }
    if c.network_low_latency_loop_budget_divisor == 0 {
        e.push("drivers.network_low_latency_loop_budget_divisor must be > 0".to_string());
    }
    if c.network_low_latency_ring_limit_divisor == 0 {
        e.push("drivers.network_low_latency_ring_limit_divisor must be > 0".to_string());
    }

    e
}
