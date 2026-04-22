//! Strict Real-Time Pool Allocators
//!
//! When `rtos_strict` is enabled, this module monitors and enforces strict O(1) allocation bounds.
//! Unbounded dynamic heap allocations after initialization will trap/panic to enforce
//! DO-178C deterministic constraints.

#[cfg(feature = "rtos_strict")]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "rtos_strict")]
static PHASE_ZERO_COMPLETE: AtomicBool = AtomicBool::new(false);

/// Must be called after system initialization to freeze standard O(n) or non-deterministic allocations.
#[cfg(feature = "rtos_strict")]
pub fn enforce_fast_path_allocation_bounds() {
    PHASE_ZERO_COMPLETE.store(true, Ordering::SeqCst);
}

/// Fallback sanity check used in kernel runtime paths to trap unintended allocations.
#[cfg(feature = "rtos_strict")]
#[inline(always)]
pub fn check_rt_allocation_violation() {
    if PHASE_ZERO_COMPLETE.load(Ordering::Relaxed) {
        panic!("RTOS Strict Violation: Dynamic allocation attempted in fast path after Phase 0");
    }
}

#[cfg(not(feature = "rtos_strict"))]
#[inline(always)]
pub fn enforce_fast_path_allocation_bounds() {}

#[cfg(not(feature = "rtos_strict"))]
#[inline(always)]
pub fn check_rt_allocation_violation() {}
