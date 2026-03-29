pub(super) const PERIODIC_TIMER_BIT: u32 = 0x20_000;
pub(super) const APIC_ENABLE_BIT: u32 = 0x100;
pub(super) const SPURIOUS_VECTOR: u32 = 0xFF;
pub(super) const TIMER_DIVIDE_BY_16: u32 = 0x03;
pub(super) const MASKED_ONE_SHOT_TIMER: u32 = 0x10_000;
pub(super) const BROADCAST_EXCLUDING_SELF: u32 = 0x3 << 18;
pub(super) const X2APIC_FALLBACK_TICKS: u32 = 10_000_000;

#[inline(always)]
pub(super) fn lapic_base() -> u64 {
    const LAPIC_BASE: u64 = 0xFEE0_0000;
    let hhdm = crate::hal::common::boot::hhdm_offset().unwrap_or(0);
    LAPIC_BASE + hhdm
}

#[inline(always)]
pub(super) unsafe fn read_apic_off(base: u64, offset: u32) -> u32 {
    // SAFETY: Caller guarantees the address points into the local APIC MMIO window.
    unsafe { crate::hal::common::mmio::read_virt_u32(base + offset as u64) }
}

#[inline(always)]
pub(super) unsafe fn write_apic_off(base: u64, offset: u32, val: u32) {
    // SAFETY: Caller guarantees the address points into the local APIC MMIO window.
    unsafe { crate::hal::common::mmio::write_virt_u32(base + offset as u64, val) };
}

#[inline(always)]
pub(super) fn enable_spurious_vector(current: u32) -> u32 {
    (current | APIC_ENABLE_BIT) | SPURIOUS_VECTOR
}

#[inline(always)]
pub(super) fn periodic_timer_vector(vector: u8) -> u32 {
    vector as u32 | PERIODIC_TIMER_BIT
}

#[inline(always)]
pub(super) fn broadcast_excluding_self_icr(vector: u8) -> u32 {
    BROADCAST_EXCLUDING_SELF | vector as u32
}

#[inline(always)]
pub(super) fn x2apic_ipi_command(apic_id: u32, vector: u8) -> u64 {
    ((apic_id as u64) << 32) | (vector as u64)
}

#[inline(always)]
pub(super) fn x2apic_broadcast_ipi_command(vector: u8) -> u64 {
    (BROADCAST_EXCLUDING_SELF as u64) | (vector as u64)
}

#[inline(always)]
pub(super) fn calibrated_ticks_or_default(ticks: u32) -> u32 {
    if ticks > 0 {
        ticks
    } else {
        X2APIC_FALLBACK_TICKS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn spurious_vector_always_enables_apic_and_vector() {
        assert_eq!(enable_spurious_vector(0), 0x1FF);
        assert_eq!(enable_spurious_vector(0x200), 0x3FF);
    }

    #[test_case]
    fn ipi_builders_encode_destination_and_broadcast() {
        assert_eq!(x2apic_ipi_command(0x22, 0x40), 0x0000_0022_0000_0040);
        assert_eq!(broadcast_excluding_self_icr(0x40), (0x3 << 18) | 0x40);
        assert_eq!(
            x2apic_broadcast_ipi_command(0x40),
            ((0x3 << 18) | 0x40) as u64
        );
    }

    #[test_case]
    fn calibrated_ticks_falls_back_when_zero() {
        assert_eq!(calibrated_ticks_or_default(1234), 1234);
        assert_eq!(calibrated_ticks_or_default(0), X2APIC_FALLBACK_TICKS);
    }
}
