// Local APIC Driver — supports both xAPIC (MMIO) and x2APIC (MSR) modes.

use core::sync::atomic::{AtomicBool, Ordering};
#[path = "apic_support.rs"]
mod apic_support;
use apic_support::{
    broadcast_excluding_self_icr, calibrated_ticks_or_default, enable_spurious_vector, lapic_base,
    periodic_timer_vector, read_apic_off, write_apic_off, x2apic_broadcast_ipi_command,
    x2apic_ipi_command, MASKED_ONE_SHOT_TIMER, TIMER_DIVIDE_BY_16,
};

/// Local APIC ID Register
const LAPIC_ID_OFF: u32 = 0x020;
/// Spurious Interrupt Vector Register
const LAPIC_SVR_OFF: u32 = 0x0F0;
/// End of Interrupt Register
const LAPIC_EOI_OFF: u32 = 0x0B0;
/// Timer Divide Configuration Register
const LAPIC_TDCR_OFF: u32 = 0x3E0;
/// Timer Initial Count Register
const LAPIC_TICR_OFF: u32 = 0x380;
/// Timer Current Count Register
const LAPIC_TCCR_OFF: u32 = 0x390;
/// Timer LVT Register
const LAPIC_LVT_TIMER_OFF: u32 = 0x320;
/// Interrupt Command Register Low
const LAPIC_ICR_LOW_OFF: u32 = 0x300;
/// Interrupt Command Register High
const LAPIC_ICR_HIGH_OFF: u32 = 0x310;

/// Cached APIC timer ticks per 10ms interval
static mut CALIBRATED_TICKS: u32 = 0;

/// Initialize Local APIC for the current CPU.
/// This enables interrupts and sets up a periodic timer using PIT-calibrated frequency.
pub unsafe fn init_local_apic() {
    let base = lapic_base();

    // 1. Enable LAPIC via SVR (Bit 8) and set Spurious Vector to 0xFF
    let svr = enable_spurious_vector(unsafe { read_apic_off(base, LAPIC_SVR_OFF) });
    // Safety: `base` is the local APIC MMIO base for the current CPU.
    unsafe { write_apic_off(base, LAPIC_SVR_OFF, svr) };

    // 2. Calibrate timer using PIT Channel 2
    let ticks = unsafe { calibrate_apic_timer(base) };

    // 3. Set Timer Divide to 16
    // Safety: `base` is the local APIC MMIO base for the current CPU.
    unsafe { write_apic_off(base, LAPIC_TDCR_OFF, TIMER_DIVIDE_BY_16) };

    // 4. Set LVT Timer Vector to 32 (IRQ 0 equivalent) and Mode to Periodic (Bit 17)
    // Safety: `base` is the local APIC MMIO base for the current CPU.
    unsafe { write_apic_off(base, LAPIC_LVT_TIMER_OFF, periodic_timer_vector(32)) };

    // 5. Set Initial Count from calibration
    // Safety: `base` is the local APIC MMIO base for the current CPU.
    unsafe { write_apic_off(base, LAPIC_TICR_OFF, ticks) };
    // Safety: APIC calibration is serialized during CPU local APIC init.
    unsafe { CALIBRATED_TICKS = ticks };
}

/// Calibrate APIC timer against PIT to determine ticks per ~10ms
unsafe fn calibrate_apic_timer(base: u64) -> u32 {
    use x86_64::instructions::port::Port;

    // PIT frequency is 1,193,182 Hz
    // We want to measure APIC ticks in ~10ms => PIT count for 10ms = 11932
    const PIT_10MS: u16 = 11932;

    let mut pit_cmd = Port::<u8>::new(0x43);
    let mut pit_ch2 = Port::<u8>::new(0x42);
    let mut pit_ctrl = Port::<u8>::new(0x61);

    // Set APIC timer divide to 16, one-shot mode with very high initial count
    // Safety: `base` is the local APIC MMIO base for the current CPU.
    unsafe { write_apic_off(base, LAPIC_TDCR_OFF, TIMER_DIVIDE_BY_16) };
    // Safety: `base` is the local APIC MMIO base for the current CPU.
    unsafe { write_apic_off(base, LAPIC_LVT_TIMER_OFF, MASKED_ONE_SHOT_TIMER) }; // Masked, one-shot

    // Setup PIT Channel 2 in mode 0 (terminal count)
    let gate = unsafe { pit_ctrl.read() };
    unsafe { pit_ctrl.write((gate & 0xFD) | 0x01) }; // Gate high, speaker off
    unsafe { pit_cmd.write(0xB0) }; // Ch2, lobyte/hibyte, mode 0
    unsafe { pit_ch2.write((PIT_10MS & 0xFF) as u8) };
    unsafe { pit_ch2.write((PIT_10MS >> 8) as u8) };

    // Reset PIT gate to start counting
    let gate = unsafe { pit_ctrl.read() };
    unsafe { pit_ctrl.write(gate & 0xFE) };
    unsafe { pit_ctrl.write(gate | 0x01) };

    // Start APIC timer with max count
    // Safety: `base` is the local APIC MMIO base for the current CPU.
    unsafe { write_apic_off(base, LAPIC_TICR_OFF, 0xFFFF_FFFF) };

    // Wait for PIT to finish (bit 5 of port 0x61 goes high when count reaches 0)
    let mut budget = 1_000_000u32;
    while (unsafe { pit_ctrl.read() } & 0x20) == 0 {
        budget = budget.saturating_sub(1);
        if budget == 0 {
            break; // Timeout fallback
        }
    }

    // Read APIC timer current count
    let after = unsafe { read_apic_off(base, LAPIC_TCCR_OFF) };
    let elapsed = 0xFFFF_FFFFu32.wrapping_sub(after);

    if elapsed == 0 || budget == 0 {
        // Calibration failed, use safe default (~10ms at ~100MHz bus)
        10_000_000
    } else {
        elapsed
    }
}

/// Send End of Interrupt
pub unsafe fn eoi() {
    // Safety: issuing EOI touches only the current CPU's local APIC MMIO window.
    unsafe { write_apic_off(lapic_base(), LAPIC_EOI_OFF, 0) };
}

/// Get current Local APIC ID
pub unsafe fn id() -> u32 {
    // Safety: reading the APIC ID register touches only the current CPU's local APIC MMIO window.
    (unsafe { read_apic_off(lapic_base(), LAPIC_ID_OFF) }) >> 24
}

/// Return calibrated ticks per timer period (for diagnostics)
pub fn calibrated_ticks() -> u32 {
    // Safety: reading the cached calibration value is atomic enough for diagnostics in this model.
    unsafe { CALIBRATED_TICKS }
}

/// Send Inter-Processor Interrupt (IPI) to a specific target CPU.
pub unsafe fn send_ipi(apic_id: u32, vector: u8) {
    let base = lapic_base();
    // Safety: writing ICR high programs the APIC destination for this IPI.
    unsafe { write_apic_off(base, LAPIC_ICR_HIGH_OFF, apic_id << 24) };
    // Vector | Fixed delivery mode
    // Safety: writing ICR low dispatches the IPI configured above.
    unsafe { write_apic_off(base, LAPIC_ICR_LOW_OFF, vector as u32) };
}

/// Send Inter-Processor Interrupt (IPI) to all other CPUs.
pub unsafe fn send_ipi_all_excluding_self(vector: u8) {
    let base = lapic_base();
    let icr = broadcast_excluding_self_icr(vector);
    // Safety: this writes the broadcast shorthand ICR on the current CPU's APIC.
    unsafe { write_apic_off(base, LAPIC_ICR_LOW_OFF, icr) };
}

// ────────────────────────────────────────────────────────────────
// X2APIC — MSR-based APIC access (faster, supports > 255 CPUs)
// ────────────────────────────────────────────────────────────────

/// Whether x2APIC mode is active.
static X2APIC_ENABLED: AtomicBool = AtomicBool::new(false);

/// x2APIC MSR base
const X2APIC_MSR_BASE: u32 = 0x800;

/// x2APIC MSR offsets (MSR = X2APIC_MSR_BASE + (MMIO_offset >> 4))
const X2APIC_ID: u32 = X2APIC_MSR_BASE + 0x02;
const X2APIC_EOI: u32 = X2APIC_MSR_BASE + 0x0B;
const X2APIC_SVR: u32 = X2APIC_MSR_BASE + 0x0F;
const X2APIC_ICR: u32 = X2APIC_MSR_BASE + 0x30;
const X2APIC_LVT_TIMER: u32 = X2APIC_MSR_BASE + 0x32;
const X2APIC_TICR: u32 = X2APIC_MSR_BASE + 0x38;
const _X2APIC_TCCR: u32 = X2APIC_MSR_BASE + 0x39;
const X2APIC_TDCR: u32 = X2APIC_MSR_BASE + 0x3E;

/// IA32_APIC_BASE MSR
const IA32_APIC_BASE_MSR: u32 = 0x1B;
/// x2APIC enable bit in IA32_APIC_BASE
const X2APIC_ENABLE_BIT: u64 = 1 << 10;
/// APIC global enable bit
const APIC_GLOBAL_ENABLE: u64 = 1 << 11;

/// Detect if the CPU supports x2APIC (CPUID leaf 1, ECX bit 21).
pub fn supports_x2apic() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        let cpuid = core::arch::x86_64::__cpuid(1);
        (cpuid.ecx & (1 << 21)) != 0
    }
    #[cfg(not(target_arch = "x86_64"))]
    false
}

/// Check whether x2APIC mode is currently active.
pub fn is_x2apic() -> bool {
    X2APIC_ENABLED.load(Ordering::Relaxed)
}

/// Enable x2APIC mode if supported. Must be called before init_local_apic on APs.
///
/// # Safety
/// Must be called on each CPU core. Transition is one-way (xAPIC → x2APIC).
#[cfg(target_arch = "x86_64")]
pub unsafe fn enable_x2apic() {
    if !supports_x2apic() {
        return;
    }
    let base = unsafe { rdmsr(IA32_APIC_BASE_MSR) };
    // Set both global enable and x2APIC enable
    // Safety: caller ensures the current CPU is transitioning its local APIC mode.
    unsafe {
        wrmsr(
            IA32_APIC_BASE_MSR,
            base | APIC_GLOBAL_ENABLE | X2APIC_ENABLE_BIT,
        )
    };
    X2APIC_ENABLED.store(true, Ordering::Release);
}

/// Initialize Local APIC in x2APIC mode.
///
/// # Safety
/// Requires enable_x2apic() to have been called first.
#[cfg(target_arch = "x86_64")]
pub unsafe fn init_x2apic() {
    // Enable LAPIC via SVR
    let svr = enable_spurious_vector(unsafe { rdmsr(X2APIC_SVR as u32) as u32 });
    unsafe { wrmsr(X2APIC_SVR as u32, svr as u64) };

    // Timer setup: divide by 16, periodic, vector 32
    unsafe { wrmsr(X2APIC_TDCR as u32, TIMER_DIVIDE_BY_16 as u64) };
    unsafe { wrmsr(X2APIC_LVT_TIMER as u32, periodic_timer_vector(32) as u64) };

    // Use same calibrated ticks from xAPIC calibration
    let ticks = calibrated_ticks_or_default(unsafe { CALIBRATED_TICKS });
    unsafe { wrmsr(X2APIC_TICR as u32, ticks as u64) };
}

/// Send EOI in x2APIC mode.
#[cfg(target_arch = "x86_64")]
pub unsafe fn x2apic_eoi() {
    unsafe { wrmsr(X2APIC_EOI as u32, 0) };
}

/// Get APIC ID in x2APIC mode (full 32-bit, no shift).
#[cfg(target_arch = "x86_64")]
pub unsafe fn x2apic_id() -> u32 {
    unsafe { rdmsr(X2APIC_ID as u32) as u32 }
}

/// Send IPI in x2APIC mode.
/// In x2APIC, ICR is a single 64-bit MSR: destination in bits [63:32], command in [31:0].
#[cfg(target_arch = "x86_64")]
pub unsafe fn x2apic_send_ipi(apic_id: u32, vector: u8) {
    let icr = x2apic_ipi_command(apic_id, vector);
    unsafe { wrmsr(X2APIC_ICR as u32, icr) };
}

/// Send IPI to all excluding self in x2APIC mode.
#[cfg(target_arch = "x86_64")]
pub unsafe fn x2apic_send_ipi_all_excluding_self(vector: u8) {
    let icr = x2apic_broadcast_ipi_command(vector);
    unsafe { wrmsr(X2APIC_ICR as u32, icr) };
}

// MSR read/write wrappers
#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn rdmsr(msr: u32) -> u64 {
    let (low, high): (u32, u32);
    // Safety: caller guarantees the requested MSR is valid in the current CPU mode.
    unsafe { core::arch::asm!("rdmsr", out("eax") low, out("edx") high, in("ecx") msr) };
    ((high as u64) << 32) | (low as u64)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    // Safety: caller guarantees the requested MSR is valid in the current CPU mode.
    unsafe { core::arch::asm!("wrmsr", in("eax") low, in("edx") high, in("ecx") msr) };
}
