use anyhow::{Result, bail};
use std::collections::HashMap;
use std::fs;

use crate::utils::paths;

pub fn run_smoke() -> Result<()> {
    println!("[test::driver-smoke] Running native driver configuration smoke test");

    let root = paths::repo_root();
    let cargo_path = root.join("Cargo.toml");
    let generated_path = paths::kernel_src("generated_consts.rs");

    if !cargo_path.exists() || !generated_path.exists() {
        bail!("Missing Cargo.toml or kernel/src/generated_consts.rs at root");
    }

    let cargo_text = fs::read_to_string(&cargo_path)?;
    let mut cargo_metadata = HashMap::new();
    let mut in_drivers_section = false;
    for line in cargo_text.lines() {
        let line = line.trim();
        if line.starts_with("[package.metadata.aethercore.config.drivers]") {
            in_drivers_section = true;
            continue;
        } else if line.starts_with('[') {
            in_drivers_section = false;
        }
        
        if in_drivers_section && !line.is_empty() && !line.starts_with('#') {
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim().to_string();
                let val_str = v.trim().replace('_', "");
                if let Ok(val) = val_str.parse::<i64>() {
                    cargo_metadata.insert(key, val);
                }
            }
        }
    }

    let generated_text = fs::read_to_string(&generated_path)?;
    let const_re = regex::Regex::new(r"pub const ([A-Z0-9_]+): (?:u64|usize) = (\d+);")?;
    
    let mut generated_consts: HashMap<String, u64> = HashMap::new();
    for cap in const_re.captures_iter(&generated_text) {
        if let Ok(val) = cap[2].parse::<u64>() {
            generated_consts.insert(cap[1].to_string(), val);
        }
    }

    let mapping: &[(&str, &str)] = &[
        ("network_irq_service_budget", "DRIVER_NETWORK_IRQ_SERVICE_BUDGET"),
        ("network_loop_service_budget", "DRIVER_NETWORK_LOOP_SERVICE_BUDGET"),
        ("network_ring_limit", "DRIVER_NETWORK_RING_LIMIT"),
        ("network_quarantine_rebind_failures", "DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES"),
        ("network_quarantine_cooldown_samples", "DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES"),
        ("network_slo_max_drop_rate_per_mille", "DRIVER_NETWORK_SLO_MAX_DROP_RATE_PER_MILLE"),
        ("network_slo_max_tx_ring_utilization_percent", "DRIVER_NETWORK_SLO_MAX_TX_RING_UTILIZATION_PERCENT"),
        ("network_slo_max_rx_ring_utilization_percent", "DRIVER_NETWORK_SLO_MAX_RX_RING_UTILIZATION_PERCENT"),
        ("network_slo_max_io_errors", "DRIVER_NETWORK_SLO_MAX_IO_ERRORS"),
        ("ahci_io_timeout_spins", "DRIVER_AHCI_IO_TIMEOUT_SPINS"),
        ("nvme_disable_ready_timeout_spins", "DRIVER_NVME_DISABLE_READY_TIMEOUT_SPINS"),
        ("nvme_poll_timeout_spins", "DRIVER_NVME_POLL_TIMEOUT_SPINS"),
        ("nvme_io_timeout_spins", "DRIVER_NVME_IO_TIMEOUT_SPINS"),
        ("e1000_reset_timeout_spins", "DRIVER_E1000_RESET_TIMEOUT_SPINS"),
        ("e1000_buffer_size_bytes", "DRIVER_E1000_BUFFER_SIZE_BYTES"),
        ("e1000_rx_desc_count", "DRIVER_E1000_RX_DESC_COUNT"),
        ("e1000_tx_desc_count", "DRIVER_E1000_TX_DESC_COUNT"),
    ];

    let mut failures = Vec::new();

    for &(key, const_name) in mapping {
        let meta_val = cargo_metadata.get(key).copied();
        match meta_val {
            Some(m_val) => {
                let gen_val = generated_consts.get(const_name).copied();
                match gen_val {
                    Some(g_val) if g_val as i64 == m_val => { /* ok */ }
                    Some(g_val) => failures.push(format!("mismatch {}/{}: expected {}, got {}", key, const_name, m_val, g_val)),
                    None => failures.push(format!("missing generated const: {}", const_name)),
                }
            }
            None => failures.push(format!("missing metadata key: {}", key)),
        }
    }

    if failures.is_empty() {
        println!("[test::driver-smoke] PASS ({} constraints verified)", mapping.len());
        Ok(())
    } else {
        println!("[test::driver-smoke] FAIL");
        for f in &failures { println!("  - {}", f); }
        bail!("Driver configuration smoke gate failed.");
    }
}
