//! AArch64-specific config validation — PCI, SMP, IRQ, timer, GIC.

use crate::build_cfg::config_types::Aarch64Config;

pub fn validate(c: &Aarch64Config) -> Vec<String> {
    let mut e = Vec::new();

    if c.pci_max_device > 31 {
        e.push(format!(
            "aarch64.pci_max_device {} exceeds PCI limit 31",
            c.pci_max_device
        ));
    }
    if c.pci_max_function > 7 {
        e.push(format!(
            "aarch64.pci_max_function {} exceeds PCI limit 7",
            c.pci_max_function
        ));
    }
    if c.smp_boot_timeout_spins == 0 {
        e.push("aarch64.smp_boot_timeout_spins must be > 0".to_string());
    }
    if c.irq_storm_threshold == 0 {
        e.push("aarch64.irq_storm_threshold must be > 0".to_string());
    }
    if c.irq_storm_log_every == 0 {
        e.push("aarch64.irq_storm_log_every must be > 0".to_string());
    }
    if c.timer_rearm_min_ticks == 0 {
        e.push("aarch64.timer_rearm_min_ticks must be > 0".to_string());
    }
    if c.timer_rearm_min_ticks > c.timer_rearm_max_ticks {
        e.push(format!(
            "aarch64.timer_rearm_min_ticks ({}) > timer_rearm_max_ticks ({})",
            c.timer_rearm_min_ticks, c.timer_rearm_max_ticks
        ));
    }
    if c.irq_rate_track_limit == 0 {
        e.push("aarch64.irq_rate_track_limit must be > 0".to_string());
    }
    if c.irq_per_line_storm_threshold == 0 {
        e.push("aarch64.irq_per_line_storm_threshold must be > 0".to_string());
    }
    if c.irq_per_line_log_every == 0 {
        e.push("aarch64.irq_per_line_log_every must be > 0".to_string());
    }
    if c.tlb_shootdown_timeout_spins == 0 {
        e.push("aarch64.tlb_shootdown_timeout_spins must be > 0".to_string());
    }

    e
}
