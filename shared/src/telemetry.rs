//! Shared telemetry helpers and naming conventions.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Standard telemetry counter suffixes.
pub mod suffix {
    pub const CALLS: &str = "calls";
    pub const HITS: &str = "hits";
    pub const DENIED: &str = "denied";
    pub const FAIL: &str = "fail";
    pub const SUCCESS: &str = "success";
}

/// Builds a canonical telemetry key from a prefix and suffix.
#[must_use]
pub fn key(prefix: &str, suffix: &str) -> alloc::string::String {
    let mut out = alloc::string::String::with_capacity(prefix.len() + suffix.len() + 1);
    out.push_str(prefix);
    out.push('_');
    out.push_str(suffix);
    out
}

/// Snapshot an AtomicU64 counter with relaxed ordering.
#[inline(always)]
#[must_use]
pub fn snapshot_u64(counter: &AtomicU64) -> u64 {
    counter.load(Ordering::Relaxed)
}

/// Snapshot an AtomicUsize counter with relaxed ordering.
#[inline(always)]
#[must_use]
pub fn snapshot_usize(counter: &AtomicUsize) -> usize {
    counter.load(Ordering::Relaxed)
}

/// Atomically take-and-reset an AtomicU64 counter.
#[inline(always)]
#[must_use]
pub fn take_u64(counter: &AtomicU64) -> u64 {
    counter.swap(0, Ordering::AcqRel)
}

/// Atomically take-and-reset an AtomicUsize counter.
#[inline(always)]
#[must_use]
pub fn take_usize(counter: &AtomicUsize) -> usize {
    counter.swap(0, Ordering::AcqRel)
}
