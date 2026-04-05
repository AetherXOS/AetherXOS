#[cfg(target_arch = "x86_64")]
pub use crate::hal::x86_64::serial::SERIAL1;

#[cfg(target_arch = "aarch64")]
pub use crate::hal::aarch64::serial::SERIAL1;

#[cfg(target_arch = "x86_64")]
pub type SerialRuntimeStats = crate::hal::x86_64::serial::SerialRuntimeStats;

#[cfg(target_arch = "aarch64")]
pub type SerialRuntimeStats = crate::hal::aarch64::serial::SerialRuntimeStats;

#[inline(always)]
pub fn stats() -> SerialRuntimeStats {
    #[cfg(target_arch = "x86_64")]
    {
        return crate::hal::x86_64::serial::stats();
    }
    #[cfg(target_arch = "aarch64")]
    {
        return crate::hal::aarch64::serial::stats();
    }
}

#[inline(always)]
pub const fn tx_timeout_spins() -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        return crate::hal::x86_64::serial::tx_timeout_spins();
    }
    #[cfg(target_arch = "aarch64")]
    {
        return crate::hal::aarch64::serial::tx_timeout_spins();
    }
}

#[inline(always)]
pub fn write_raw(s: &str) {
    if !crate::config::KernelConfig::should_emit_early_serial_line(s) {
        return;
    }

    #[cfg(target_arch = "x86_64")]
    {
        crate::hal::x86_64::serial::write_raw(s);
    }
    #[cfg(target_arch = "aarch64")]
    {
        crate::hal::aarch64::serial::write_raw(s);
    }
}

#[inline(always)]
pub fn write_line(s: &str) {
    write_raw(s);
    write_raw("\n");
}

#[inline(always)]
pub fn write_hex(label: &str, value: u64) {
    if !crate::config::KernelConfig::serial_early_debug_enabled() {
        return;
    }

    #[cfg(target_arch = "x86_64")]
    {
        crate::hal::x86_64::serial::write_hex(label, value);
    }
    #[cfg(target_arch = "aarch64")]
    {
        crate::hal::aarch64::serial::write_hex(label, value);
    }
}

#[inline(always)]
pub fn write_trace(scope: &str, stage: &str) {
    if !crate::config::KernelConfig::serial_early_debug_enabled() {
        return;
    }

    #[cfg(target_arch = "x86_64")]
    {
        crate::hal::x86_64::serial::write_trace(scope, stage);
    }
    #[cfg(target_arch = "aarch64")]
    {
        crate::hal::aarch64::serial::write_trace(scope, stage);
    }
}

#[inline(always)]
pub fn write_trace_line(scope: &str, stage: &str) {
    write_trace(scope, stage);
    write_raw("\n");
}

#[inline(always)]
pub fn write_trace_hex(scope: &str, key: &str, value: u64) {
    if !crate::config::KernelConfig::serial_early_debug_enabled() {
        return;
    }

    #[cfg(target_arch = "x86_64")]
    {
        crate::hal::x86_64::serial::write_trace_hex(scope, key, value);
    }
    #[cfg(target_arch = "aarch64")]
    {
        crate::hal::aarch64::serial::write_trace_hex(scope, key, value);
    }
}

#[inline(always)]
pub fn write_dump_bytes(label: &str, bytes: &[u8]) {
    if !crate::config::KernelConfig::serial_early_debug_enabled() {
        return;
    }

    #[cfg(target_arch = "x86_64")]
    {
        crate::hal::x86_64::serial::write_dump_bytes(label, bytes);
    }
    #[cfg(target_arch = "aarch64")]
    {
        crate::hal::aarch64::serial::write_dump_bytes(label, bytes);
    }
}
