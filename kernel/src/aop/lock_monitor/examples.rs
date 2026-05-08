use super::*;
use alloc::format;

/// Example of a function that holds a lock.
#[lock_monitor(threshold = 500)]
pub fn protected_resource_access() {
    // Simulate some work while holding a virtual lock
    crate::core::time::delay_cycles(200);
}

/// Example of potential high contention path.
#[lock_monitor(threshold = 100)]
pub fn frequent_short_lock() {
    crate::core::time::delay_cycles(50);
}
