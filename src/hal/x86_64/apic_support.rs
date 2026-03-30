use crate::kernel::bit_utils::apic as bits;
use crate::hal::common::mmio::MmioBlock;

pub(super) const PERIODIC_TIMER_BIT: u32 = 1 << 17;
pub(super) const APIC_ENABLE_BIT: u32 = 1 << 8;
pub(super) const SPURIOUS_VECTOR: u32 = 0xFF;
pub(super) const TIMER_DIVIDE_BY_16: u32 = 0x03;
pub(super) const MASKED_ONE_SHOT_TIMER: u32 = 1 << 16;
pub(super) const X2APIC_FALLBACK_TICKS: u32 = 10_000_000;

#[inline(always)]
pub(super) fn lapic_base() -> u64 {
    let hhdm = crate::hal::common::boot::hhdm_offset().unwrap_or(0);
    bits::LAPIC_DEFAULT_BASE + hhdm
}

#[inline(always)]
pub(super) fn get_lapic_block() -> MmioBlock {
    MmioBlock::new(lapic_base() as usize)
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
    let mut icr = vector as u32;
    icr = bits::ICR_DEST_SHORTHAND.write(icr, bits::ICR_DEST_ALL_EXCLUDING_SELF);
    icr
}

#[inline(always)]
pub(super) fn x2apic_ipi_command(apic_id: u32, vector: u8) -> u64 {
    ((apic_id as u64) << 32) | (vector as u64)
}

#[inline(always)]
pub(super) fn x2apic_broadcast_ipi_command(vector: u8) -> u64 {
    let mut icr = vector as u32;
    icr = bits::ICR_DEST_SHORTHAND.write(icr, bits::ICR_DEST_ALL_EXCLUDING_SELF);
    icr as u64
}

#[inline(always)]
pub(super) fn calibrated_ticks_or_default(ticks: u32) -> u32 {
    if ticks > 0 { ticks } else { X2APIC_FALLBACK_TICKS }
}
