//! Memory safety enhancements
//! 
//! This module provides memory safety with:
//! - Bounds checking enforcement
//! - Use-after-free detection
//! - Double-free detection
//! - Memory leak detection
//! - Safe abstractions for unsafe operations

use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};

const MAX_TRACKED_ALLOCATIONS: usize = 65536;

// Telemetry
static MEMSAFE_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static MEMSAFE_DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static MEMSAFE_VIOLATIONS: AtomicU64 = AtomicU64::new(0);
static MEMSAFE_LEAKS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct MemorySafetyStats {
    pub allocations: u64,
    pub deallocations: u64,
    pub violations: u64,
    pub leaks: u64,
}

pub fn memory_safety_stats() -> MemorySafetyStats {
    MemorySafetyStats {
        allocations: MEMSAFE_ALLOCATIONS.load(Ordering::Relaxed),
        deallocations: MEMSAFE_DEALLOCATIONS.load(Ordering::Relaxed),
        violations: MEMSAFE_VIOLATIONS.load(Ordering::Relaxed),
        leaks: MEMSAFE_LEAKS.load(Ordering::Relaxed),
    }
}

/// Tracked allocation for safety monitoring
#[repr(C)]
pub struct TrackedAllocation {
    ptr: AtomicPtr<u8>,
    size: AtomicU64,
    freed: AtomicBool,
    allocation_id: AtomicU64,
}

impl TrackedAllocation {
    const fn new(ptr: *mut u8, size: u64, allocation_id: u64) -> Self {
        Self {
            ptr: AtomicPtr::new(ptr),
            size: AtomicU64::new(size),
            freed: AtomicBool::new(false),
            allocation_id: AtomicU64::new(allocation_id),
        }
    }

    #[inline(always)]
    fn mark_freed(&self) {
        self.freed.store(true, Ordering::Release);
    }

    #[inline(always)]
    fn is_freed(&self) -> bool {
        self.freed.load(Ordering::Acquire)
    }
}

/// Memory safety monitor
pub struct MemorySafetyMonitor {
    allocations: [AtomicPtr<TrackedAllocation>; MAX_TRACKED_ALLOCATIONS],
    allocation_counter: AtomicU64,
    monitoring_enabled: AtomicBool,
}

impl MemorySafetyMonitor {
    pub const fn new() -> Self {
        const NULL_PTR: AtomicPtr<TrackedAllocation> = AtomicPtr::new(core::ptr::null_mut());
        Self {
            allocations: [NULL_PTR; MAX_TRACKED_ALLOCATIONS],
            allocation_counter: AtomicU64::new(0),
            monitoring_enabled: AtomicBool::new(true),
        }
    }

    #[inline(always)]
    pub fn enable(&self) {
        self.monitoring_enabled.store(true, Ordering::Release);
    }

    #[inline(always)]
    pub fn disable(&self) {
        self.monitoring_enabled.store(false, Ordering::Release);
    }

    /// Track an allocation
    pub fn track_allocation(&self, ptr: *mut u8, size: u64) -> Result<u64, &'static str> {
        if !self.monitoring_enabled.load(Ordering::Acquire) {
            return Ok(0);
        }

        MEMSAFE_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        
        let allocation_id = self.allocation_counter.fetch_add(1, Ordering::Relaxed);
        let tracked = unsafe {
            alloc::alloc::alloc(
                core::alloc::Layout::new::<TrackedAllocation>()
            ) as *mut TrackedAllocation
        };
        
        if tracked.is_null() {
            return Err("allocation failed");
        }

        unsafe {
            tracked.write(TrackedAllocation::new(ptr, size, allocation_id));
        }

        let idx = (allocation_id as usize) % MAX_TRACKED_ALLOCATIONS;
        self.allocations[idx].store(tracked, Ordering::Release);
        
        Ok(allocation_id)
    }

    /// Track a deallocation
    pub fn track_deallocation(&self, ptr: *mut u8) -> Result<(), &'static str> {
        if !self.monitoring_enabled.load(Ordering::Acquire) {
            return Ok(());
        }

        MEMSAFE_DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        
        for tracked_ptr in &self.allocations {
            let tracked = tracked_ptr.load(Ordering::Acquire);
            if !tracked.is_null() {
                unsafe {
                    let tracked_ref = &*tracked;
                    if tracked_ref.ptr.load(Ordering::Acquire) == ptr {
                        if tracked_ref.is_freed() {
                            MEMSAFE_VIOLATIONS.fetch_add(1, Ordering::Relaxed);
                            return Err("double-free detected");
                        }
                        tracked_ref.mark_freed();
                        return Ok(());
                    }
                }
            }
        }
        
        Err("allocation not found")
    }

    /// Check for use-after-free
    #[inline(always)]
    pub fn check_use_after_free(&self, ptr: *mut u8) -> bool {
        for tracked_ptr in &self.allocations {
            let tracked = tracked_ptr.load(Ordering::Acquire);
            if !tracked.is_null() {
                unsafe {
                    let tracked_ref = &*tracked;
                    if tracked_ref.ptr.load(Ordering::Acquire) == ptr && tracked_ref.is_freed() {
                        MEMSAFE_VIOLATIONS.fetch_add(1, Ordering::Relaxed);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check for memory leaks
    pub fn check_leaks(&self) -> u64 {
        let mut leaks = 0;
        
        for tracked_ptr in &self.allocations {
            let tracked = tracked_ptr.load(Ordering::Acquire);
            if !tracked.is_null() {
                unsafe {
                    let tracked_ref = &*tracked;
                    if !tracked_ref.is_freed() {
                        leaks += 1;
                    }
                }
            }
        }
        
        MEMSAFE_LEAKS.store(leaks, Ordering::Release);
        leaks
    }

    /// Safe bounds-checked access
    #[inline(always)]
    pub fn safe_read(&self, ptr: *const u8, offset: usize, size: usize) -> Result<(), &'static str> {
        if self.check_use_after_free(ptr as *mut u8) {
            return Err("use-after-free");
        }

        for tracked_ptr in &self.allocations {
            let tracked = tracked_ptr.load(Ordering::Acquire);
            if !tracked.is_null() {
                unsafe {
                    let tracked_ref = &*tracked;
                    if tracked_ref.ptr.load(Ordering::Acquire) == ptr as *mut u8 {
                        let alloc_size = tracked_ref.size.load(Ordering::Acquire) as usize;
                        if offset + size > alloc_size {
                            MEMSAFE_VIOLATIONS.fetch_add(1, Ordering::Relaxed);
                            return Err("out of bounds");
                        }
                        return Ok(());
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_tracked_allocation() {
        let allocation = TrackedAllocation::new(core::ptr::null_mut(), 4096, 1);
        assert!(!allocation.is_freed());
        
        allocation.mark_freed();
        assert!(allocation.is_freed());
    }

    #[test_case]
    fn test_memory_safety_stats() {
        let _stats = memory_safety_stats();
    }
}
