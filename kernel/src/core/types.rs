//! Core zero-sized types and strongly typed identifiers.

/// Marker for a capability-bearing object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapabilityToken;

/// Type-level wrapper for a fixed MMIO base address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MmioBase<const BASE_ADDR: usize>;

impl<const BASE_ADDR: usize> MmioBase<BASE_ADDR> {
    /// Returns the encoded physical base address.
    pub const fn addr() -> usize {
        BASE_ADDR
    }
}