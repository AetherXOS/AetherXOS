/// AArch64 Generic Timer (ARM Architecture Reference Manual, D13).
///
/// Uses the EL1 virtual timer (cntv_*) which is the preferred timer for
/// OS kernels running at EL1.  The physical counter (cntpct_el0) is used
/// for raw wall-clock reads.
use crate::interfaces::Timer;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::generated_consts::{AARCH64_TIMER_REARM_MAX_TICKS, AARCH64_TIMER_REARM_MIN_TICKS};
use crate::hal::common::timer::{clamp_ticks, ns_to_ticks, ticks_to_ns};

use crate::kernel::bit_utils::BitField32;

// ── Named constants for CNTV_CTL_EL0 bits ────────────────────────────────────
/// CNTV_CTL_EL0 bit definitions
pub const CNTV_CTL_ENABLE:  u32 = 1 << 0;
pub const CNTV_CTL_IMASK:   u32 = 1 << 1;
pub const CNTV_CTL_ISTATUS: u32 = 1 << 2;

// ── Calibration state ─────────────────────────────────────────────────────────

/// Clock frequency in Hz read from CNTFRQ_EL0 during `init()`.
static TIMER_FREQ_HZ: AtomicU64 = AtomicU64::new(0);

/// Absolute virtual counter value recorded at `init()` time.
static BOOT_COUNTER: AtomicU64 = AtomicU64::new(0);
static LAST_PROGRAMMED_TICKS: AtomicU64 = AtomicU64::new(0);
static REARM_CLAMP_MIN_HITS: AtomicU64 = AtomicU64::new(0);
static REARM_CLAMP_MAX_HITS: AtomicU64 = AtomicU64::new(0);

// ── Low-level register helpers ────────────────────────────────────────────────

#[inline(always)]
fn read_cntfrq() -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) v);
    }
    v
}

#[inline(always)]
fn read_cntvct() -> u64 {
    let v: u64;
    // ISB before reading to ensure ordering vs preceding memory ops.
    unsafe {
        core::arch::asm!(
            "isb",
            "mrs {}, cntvct_el0",
            out(reg) v,
        );
    }
    v
}

#[inline(always)]
fn write_cntv_tval(ticks: u64) {
    unsafe {
        core::arch::asm!("msr cntv_tval_el0, {}", in(reg) ticks);
    }
}

#[inline(always)]
fn write_cntv_ctl(val: u32) {
    unsafe {
        let v = val as u64;
        core::arch::asm!("msr cntv_ctl_el0, {}", in(reg) v);
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub struct GenericTimer;

#[derive(Debug, Clone, Copy)]
pub struct GenericTimerStats {
    pub frequency_hz: u64,
    pub last_programmed_ticks: u64,
    pub clamp_min_hits: u64,
    pub clamp_max_hits: u64,
}

impl GenericTimer {
    /// Initialise the timer: read the clock frequency and record the boot
    /// counter.  Must be called once by the BSP before `enable()`.
    pub fn init() {
        let freq = read_cntfrq();
        TIMER_FREQ_HZ.store(freq, Ordering::Relaxed);
        BOOT_COUNTER.store(read_cntvct(), Ordering::Relaxed);
        crate::klog_info!(
            "AArch64 GenericTimer: freq={} Hz, boot_cnt={}",
            freq,
            BOOT_COUNTER.load(Ordering::Relaxed)
        );
    }

    /// Enable the virtual timer and arm it to fire after `period_ns` nanoseconds.
    pub fn enable_with_period_ns(period_ns: u64) {
        let freq = TIMER_FREQ_HZ.load(Ordering::Relaxed);
        let ticks = ns_to_ticks(period_ns, freq, 1_000);
        Self::set_timer(ticks);
    }

    /// Enable with a tick count directly (useful for SMP APs after BSP init).
    pub fn enable() {
        // Default: 10 ms period.
        Self::enable_with_period_ns(10_000_000);
    }

    /// Rearm the timer for the next interrupt.  Call this at the end of the
    /// timer interrupt handler.
    pub fn rearm(period_ns: u64) {
        Self::enable_with_period_ns(period_ns);
    }

    /// Program the timer using raw ticks with config-driven clamps.
    pub fn set_timer(ticks: u64) {
        let (programmed, clamped_min, clamped_max) = clamp_ticks(
            ticks,
            AARCH64_TIMER_REARM_MIN_TICKS,
            AARCH64_TIMER_REARM_MAX_TICKS,
        );
        if clamped_min {
            REARM_CLAMP_MIN_HITS.fetch_add(1, Ordering::Relaxed);
        } else if clamped_max {
            REARM_CLAMP_MAX_HITS.fetch_add(1, Ordering::Relaxed);
        }

        LAST_PROGRAMMED_TICKS.store(programmed, Ordering::Relaxed);
        write_cntv_tval(programmed);
        
        let mut ctl = BitField32::new(0);
        ctl.set_bit(0, true); // ENABLE
        write_cntv_ctl(ctl.bits());
    }

    /// Disable the virtual timer.
    pub fn disable() {
        write_cntv_ctl(0);
    }

    /// Mask the timer interrupt without stopping the counter.
    pub fn mask() {
        let mut ctl = BitField32::new(0);
        ctl.set_bit(0, true); // ENABLE
        ctl.set_bit(1, true); // IMASK
        write_cntv_ctl(ctl.bits());
    }

    /// Read the clock frequency in Hz.
    #[inline(always)]
    pub fn frequency() -> u64 {
        TIMER_FREQ_HZ.load(Ordering::Relaxed)
    }

    /// Read the raw virtual counter.
    #[inline(always)]
    pub fn counter() -> u64 {
        read_cntvct()
    }

    /// Return nanoseconds elapsed since `init()` was called.
    pub fn uptime_ns() -> u64 {
        let freq = TIMER_FREQ_HZ.load(Ordering::Relaxed);
        let elapsed_ticks = read_cntvct().wrapping_sub(BOOT_COUNTER.load(Ordering::Relaxed));
        ticks_to_ns(elapsed_ticks, freq)
    }

    /// Set a one-shot alarm `delta_ns` from now.
    pub fn set_oneshot_ns(delta_ns: u64) {
        let freq = TIMER_FREQ_HZ.load(Ordering::Relaxed);
        let ticks = ns_to_ticks(delta_ns, freq, 1_000);
        Self::set_timer(ticks);
    }

    #[inline(always)]
    pub fn last_programmed_ticks() -> u64 {
        LAST_PROGRAMMED_TICKS.load(Ordering::Relaxed)
    }

    pub fn stats() -> GenericTimerStats {
        GenericTimerStats {
            frequency_hz: TIMER_FREQ_HZ.load(Ordering::Relaxed),
            last_programmed_ticks: LAST_PROGRAMMED_TICKS.load(Ordering::Relaxed),
            clamp_min_hits: REARM_CLAMP_MIN_HITS.load(Ordering::Relaxed),
            clamp_max_hits: REARM_CLAMP_MAX_HITS.load(Ordering::Relaxed),
        }
    }
}

impl Timer for GenericTimer {
    fn init(&mut self) {
        Self::init();
    }

    fn set_oneshot_ns(&mut self, ns: u64) {
        Self::set_oneshot_ns(ns);
    }

    fn uptime_ns(&self) -> u64 {
        Self::uptime_ns()
    }

    fn frequency_hz(&self) -> u64 {
        Self::frequency()
    }
}
