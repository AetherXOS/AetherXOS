// Local APIC Driver — supports both xAPIC (MMIO) and x2APIC (MSR) modes.

use crate::kernel::bit_utils::apic as bits;
use crate::hal::x86_64::{pic, cpu};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use crate::hal::common::mmio::MmioBlock;

#[path = "apic_support.rs"]
mod apic_support;
use apic_support::{
    broadcast_excluding_self_icr, calibrated_ticks_or_default, enable_spurious_vector,
    get_lapic_block, periodic_timer_vector, x2apic_broadcast_ipi_command,
    x2apic_ipi_command, MASKED_ONE_SHOT_TIMER, TIMER_DIVIDE_BY_16,
};

/// LAPIC Configuration
const LAPIC_PERIODIC_TIMER_VECTOR: u8 = 32;
const APIC_CALIBRATION_FALLBACK_TICKS: u32 = 10_000_000;
const APIC_CALIBRATION_SPIN_BUDGET: u32 = 1_000_000;
const APIC_TIMER_MAX_INITIAL_COUNT: u32 = 0xFFFF_FFFF;

// PIT Ports (Magic values moved to private constants here for now, better in bit_utils later)
const PIT_COMMAND_PORT: u16 = 0x43;
const PIT_CHANNEL2_PORT: u16 = 0x42;
const PIT_CONTROL_PORT: u16 = 0x61;
const PIT_OUT_STATUS_BIT: u8 = 0x20;

/// Cached APIC timer ticks per 10ms interval
static CALIBRATED_TICKS: AtomicU32 = AtomicU32::new(0);

pub unsafe fn init_local_apic() {
    // SAFETY: MMIO LAPIC accesses require privileged execution and a valid mapped LAPIC base.
    unsafe {
        let block = get_lapic_block();

        // 1. Enable LAPIC via SVR
        let svr = block.reg::<u32>(bits::LAPIC_SVR).read();
        block
            .reg::<u32>(bits::LAPIC_SVR)
            .write(enable_spurious_vector(svr));

        // 2. Calibrate
        let ticks = calibrate_apic_timer(&block);

        // 3. Set Timer Divide
        block.reg::<u32>(bits::LAPIC_TDCR).write(TIMER_DIVIDE_BY_16);

        // 4. Set Vector and Mode
        block
            .reg::<u32>(bits::LAPIC_LVT_TIMER)
            .write(periodic_timer_vector(LAPIC_PERIODIC_TIMER_VECTOR));

        // 5. Set Initial Count
        block.reg::<u32>(bits::LAPIC_TICR).write(ticks);
        CALIBRATED_TICKS.store(ticks, Ordering::Relaxed);
    }
}

pub fn init() {
    unsafe {
        pic::Pic::disable();
        init_local_apic();
    }
}

unsafe fn calibrate_apic_timer(block: &MmioBlock) -> u32 {
    use x86_64::instructions::port::Port;
    const PIT_10MS: u16 = 11932;

    // SAFETY: PIT I/O ports and LAPIC MMIO registers are valid in early kernel init.
    unsafe {
        let mut pit_cmd = Port::<u8>::new(PIT_COMMAND_PORT);
        let mut pit_ch2 = Port::<u8>::new(PIT_CHANNEL2_PORT);
        let mut pit_ctrl = Port::<u8>::new(PIT_CONTROL_PORT);

        block.reg::<u32>(bits::LAPIC_TDCR).write(TIMER_DIVIDE_BY_16);
        block.reg::<u32>(bits::LAPIC_LVT_TIMER).write(MASKED_ONE_SHOT_TIMER);

        let gate = pit_ctrl.read();
        pit_ctrl.write((gate & 0xFD) | 0x01);
        pit_cmd.write(0xB0);
        pit_ch2.write((PIT_10MS & 0xFF) as u8);
        pit_ch2.write((PIT_10MS >> 8) as u8);

        let gate = pit_ctrl.read();
        pit_ctrl.write(gate & !0x01);
        pit_ctrl.write(gate | 0x01);

        block
            .reg::<u32>(bits::LAPIC_TICR)
            .write(APIC_TIMER_MAX_INITIAL_COUNT);

        let mut budget = APIC_CALIBRATION_SPIN_BUDGET;
        while (pit_ctrl.read() & PIT_OUT_STATUS_BIT) == 0 {
            budget = budget.saturating_sub(1);
            if budget == 0 {
                break;
            }
        }

        let after = block.reg::<u32>(bits::LAPIC_TCCR).read();
        let elapsed = APIC_TIMER_MAX_INITIAL_COUNT.wrapping_sub(after);

        if elapsed == 0 || budget == 0 {
            APIC_CALIBRATION_FALLBACK_TICKS
        } else {
            elapsed
        }
    }
}

pub unsafe fn eoi() {
    // SAFETY: EOI write is valid after LAPIC mapping and init.
    unsafe { get_lapic_block().reg::<u32>(bits::LAPIC_EOI).write(0) };
}

pub unsafe fn id() -> u32 {
    // SAFETY: LAPIC ID register is readable after LAPIC mapping and init.
    unsafe { get_lapic_block().reg::<u32>(bits::LAPIC_ID).read() >> 24 }
}

pub unsafe fn send_ipi(apic_id: u32, vector: u8) {
    // SAFETY: ICR access is valid for LAPIC-enabled CPUs.
    unsafe {
        let block = get_lapic_block();
        block.reg::<u32>(bits::LAPIC_ICR_HIGH).write(apic_id << 24);
        block.reg::<u32>(bits::LAPIC_ICR_LOW).write(vector as u32);
    }
}

pub unsafe fn send_ipi_all_excluding_self(vector: u8) {
    // SAFETY: ICR write is valid for LAPIC-enabled CPUs.
    unsafe {
        let block = get_lapic_block();
        block
            .reg::<u32>(bits::LAPIC_ICR_LOW)
            .write(broadcast_excluding_self_icr(vector));
    }
}

// ── X2APIC Mode ───────────────────────────────────────────────────────────────

static X2APIC_ENABLED: AtomicBool = AtomicBool::new(false);

use bits::X2APIC_MSR_BASE;
const X2APIC_SVR: u32 = X2APIC_MSR_BASE + 0x0F;
const X2APIC_TICR: u32 = X2APIC_MSR_BASE + 0x38;
const X2APIC_TDCR: u32 = X2APIC_MSR_BASE + 0x3E;
const X2APIC_LVT_TIMER: u32 = X2APIC_MSR_BASE + 0x32;
const X2APIC_ICR: u32 = X2APIC_MSR_BASE + 0x30;
const X2APIC_EOI: u32 = X2APIC_MSR_BASE + 0x0B;
const X2APIC_ID: u32 = X2APIC_MSR_BASE + 0x02;

pub fn supports_x2apic() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        (core::arch::x86_64::__cpuid(1).ecx & (1 << 21)) != 0
    }
    #[cfg(not(target_arch = "x86_64"))]
    false
}

pub fn is_x2apic() -> bool { X2APIC_ENABLED.load(Ordering::Relaxed) }

pub unsafe fn enable_x2apic() {
    if !supports_x2apic() {
        return;
    }
    // SAFETY: x2APIC enable uses architectural MSRs on x86_64 with CPUID support bit set.
    unsafe {
        let base = cpu::read_msr(0x1B);
        cpu::write_msr(0x1B, base | 0x400 | 0x800); // x2APIC + Global Enable
    }
    X2APIC_ENABLED.store(true, Ordering::Release);
}

pub unsafe fn init_x2apic() {
    // SAFETY: x2APIC MSRs are valid after x2APIC enable.
    unsafe {
        let svr = enable_spurious_vector(cpu::read_msr(X2APIC_SVR) as u32);
        cpu::write_msr(X2APIC_SVR, svr as u64);
        cpu::write_msr(X2APIC_TDCR, TIMER_DIVIDE_BY_16 as u64);
        cpu::write_msr(
            X2APIC_LVT_TIMER,
            periodic_timer_vector(LAPIC_PERIODIC_TIMER_VECTOR) as u64,
        );
        let ticks = calibrated_ticks_or_default(CALIBRATED_TICKS.load(Ordering::Relaxed));
        cpu::write_msr(X2APIC_TICR, ticks as u64);
    }
}

pub unsafe fn x2apic_eoi() {
    // SAFETY: EOI MSR write is valid while x2APIC is enabled.
    unsafe { cpu::write_msr(X2APIC_EOI, 0) };
}

pub unsafe fn x2apic_id() -> u32 {
    // SAFETY: ID MSR read is valid while x2APIC is enabled.
    unsafe { cpu::read_msr(X2APIC_ID) as u32 }
}

pub unsafe fn x2apic_send_ipi(apic_id: u32, vector: u8) {
    // SAFETY: ICR MSR write is valid while x2APIC is enabled.
    unsafe { cpu::write_msr(X2APIC_ICR, x2apic_ipi_command(apic_id, vector)) };
}

pub unsafe fn x2apic_send_ipi_all_excluding_self(vector: u8) {
    // SAFETY: ICR MSR write is valid while x2APIC is enabled.
    unsafe { cpu::write_msr(X2APIC_ICR, x2apic_broadcast_ipi_command(vector)) };
}
