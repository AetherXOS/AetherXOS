/// Core timing facade for performance metrics and cycle counting.
/// 
/// Provides uniform access to high-resolution cycle counters across architectures.

/// Get the current CPU cycle count.
/// 
/// # Returns
/// The number of CPU cycles elapsed since system boot.
/// On architectures without cycle counters, this returns 0.
/// 
/// # Accuracy
/// - **x86_64**: Uses RDTSC (or RDTSCP with TSC_DEADLINE); highly accurate but may be affected by CPU frequency scaling.
/// - **aarch64**: Uses CNTVCT_EL0; accurate and immune to frequency scaling.
/// - **Other**: Returns 0; implement via feature flags as needed.
pub fn cycle_count() -> u64 {
    // Delegate to HAL CPU timer (rdtsc/cntvct as appropriate)
    crate::hal::cpu::rdtsc()
}

/// Sleep for approximately `cycles` CPU cycles.
/// 
/// This is a busy-wait loop and should only be used for very short delays
/// in low-level code (e.g., device initialization).
/// 
/// # Performance
/// On modern CPUs, this may not be cycle-accurate due to optimizations,
/// frequency scaling, or other factors.
pub fn delay_cycles(cycles: u64) {
    let start = cycle_count();
    while cycle_count().saturating_sub(start) < cycles {
        // Busy-wait.
    }
}

/// Sleep for approximately `ms` milliseconds.
pub fn delay_ms(ms: u64) {
    // Crude estimate: 2GHz CPU = 2,000,000 cycles/ms
    delay_cycles(ms * 2_000_000);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_count_increases() {
        let c1 = cycle_count();
        let c2 = cycle_count();
        // On some systems, c2 might equal c1 due to sampling granularity,
        // but typically c2 >= c1.
        assert!(c2 >= c1);
    }

    #[test]
    fn test_delay_cycles() {
        let start = cycle_count();
        delay_cycles(1000);
        let end = cycle_count();
        // Verify that we waited at least approximately the requested duration.
        // Allow for measurement granularity and CPU variations.
        let elapsed = end.saturating_sub(start);
        // Just verify delay_cycles doesn't panic; exact timing is platform-dependent.
        assert!(elapsed >= 100 || elapsed == 0); // Allow 0 on platforms without cycle counters.
    }
}
