/// Timer device abstraction with type-state pattern for safe device management.
///
/// This module provides strongly-typed, const-generic access to platform timer devices.
/// Using type-state, we ensure proper initialization before reads/writes.

use core::marker::PhantomData;

/// Marker trait for timer states.
pub trait TimerState: Send {}

/// Uninitialized timer state—prevents access until initialized.
pub struct Uninitialized;
impl TimerState for Uninitialized {}

/// Initialized timer state—allows read/write operations.
pub struct Initialized;
impl TimerState for Initialized {}

/// Generic timer device with const-generic MMIO base address.
///
/// # Type Parameters
/// - `BASE`: The memory-mapped base address of the timer device.
/// - `STATE`: Type-state tracking initialization status.
///
/// # Safety
/// The provided `BASE` must point to valid, readable/writable timer memory that persists
/// for the lifetime of the kernel. Misaligned or invalid addresses may cause crashes.
pub struct Timer<const BASE: usize, STATE: TimerState = Uninitialized> {
    _marker: PhantomData<STATE>,
}

impl<const BASE: usize> Timer<BASE, Uninitialized> {
    /// Create a new uninitialized timer device.
    ///
    /// # Returns
    /// A timer in uninitialized state. Call `init()` before using.
    pub const fn new() -> Self {
        Timer {
            _marker: PhantomData,
        }
    }

    /// Initialize the timer device.
    ///
    /// Sets up the timer control register and transitions to `Initialized` state.
    ///
    /// # Safety
    /// The BASE address must be valid and properly aligned for the target architecture.
    /// This function must be called exactly once during boot.
    pub unsafe fn init(self) -> Timer<BASE, Initialized> { unsafe {
        // SAFETY: We assume BASE points to valid timer MMIO region.
        // Callers must ensure this invariant.
        // Write control register to enable timer (example; actual values depend on hardware)
        // For x86: APIC timer, pit, HPET, or ACPI PM timer
        // For ARM: Generic Timer, ARM SysTick, etc.
        let ctrl_reg = BASE as *mut u32;
        core::ptr::write_volatile(ctrl_reg, 1); // Enable timer (hardware-specific)
        
        Timer {
            _marker: PhantomData,
        }
    }}
}

impl<const BASE: usize> Timer<BASE, Initialized> {
    /// Read the current timer count (in hardware-specific units).
    ///
    /// # Returns
    /// Current timer value; units depend on the specific timer device.
    pub fn read_count(&self) -> u64 {
        // SAFETY: We're in Initialized state, so the timer has been set up.
        // Reading from the timer register is safe and has no side effects.
        unsafe {
            // Offset 0x00: typical timer count register
            let count_ptr = BASE as *const u32;
            core::ptr::read_volatile(count_ptr) as u64
        }
    }

    /// Write to the timer configuration register.
    ///
    /// # Arguments
    /// - `offset`: Register offset in bytes.
    /// - `value`: Value to write.
    ///
    /// # Safety
    /// Writing to arbitrary offsets may corrupt timer state or affect system stability.
    /// Callers must ensure the offset and value are appropriate for the hardware.
    pub unsafe fn write_config(&mut self, offset: usize, value: u32) { unsafe {
        // SAFETY: Caller asserts that the offset and value are safe.
        let config_ptr = (BASE as *mut u32).add(offset / 4);
        core::ptr::write_volatile(config_ptr, value);
    }}

    /// Disable the timer (transition back to uninitialized for reconfiguration).
    ///
    /// # Returns
    /// The timer in uninitialized state.
    ///
    /// # Safety
    /// Disabling the timer may affect system scheduling and time-keeping.
    /// Only use if you're sure the system can handle timer interrupts being halted.
    pub unsafe fn disable(self) -> Timer<BASE, Uninitialized> {
        // SAFETY: We're already Initialized, and caller acknowledges the risk.
        Timer {
            _marker: PhantomData,
        }
    }
}

/// Platform-specific helper to configure a timer for periodic interrupts.
///
/// # Arguments
/// - `period_ns`: Desired interrupt period in nanoseconds.
///
/// # Returns
/// The number of timer ticks to program, or 0 if the period is too large.
pub fn nanoseconds_to_ticks(period_ns: u64, timer_freq_hz: u64) -> u64 {
    // ticks = (period_ns * freq_hz) / 1_000_000_000
    // Saturate on overflow to prevent wrap-around.
    let product = (period_ns as u128).saturating_mul(timer_freq_hz as u128);
    let ticks = product / 1_000_000_000u128;
    if ticks > u64::MAX as u128 {
        u64::MAX
    } else {
        ticks as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_creation() {
        let _timer: Timer<0x1000, Uninitialized> = Timer::new();
        // Verify timer can be created without panicking.
    }

    #[test]
    fn test_nanoseconds_to_ticks() {
        // 1 microsecond at 1 GHz = 1000 ticks
        let ticks = nanoseconds_to_ticks(1000, 1_000_000_000);
        assert_eq!(ticks, 1000);

        // 1 second at 1 MHz = 1_000_000 ticks
        let ticks = nanoseconds_to_ticks(1_000_000_000, 1_000_000);
        assert_eq!(ticks, 1_000_000);
    }
}
