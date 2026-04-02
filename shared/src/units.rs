//! Shared unit helpers and canonical constants.

/// Canonical page size (4 KiB).
pub const PAGE_SIZE_4K: usize = 4096;

/// Converts KiB to bytes.
#[must_use]
pub const fn kib(v: usize) -> usize {
    v * 1024
}

/// Converts MiB to bytes.
#[must_use]
pub const fn mib(v: usize) -> usize {
    kib(v) * 1024
}

/// Converts GiB to bytes.
#[must_use]
pub const fn gib(v: usize) -> usize {
    mib(v) * 1024
}

/// Converts milliseconds to nanoseconds.
#[must_use]
pub const fn ms_to_ns(v: u64) -> u64 {
    v * 1_000_000
}

/// Converts seconds to milliseconds.
#[must_use]
pub const fn sec_to_ms(v: u64) -> u64 {
    v * 1_000
}
