//! Hardware Abstraction Layer

pub mod common;
pub mod cpu;
pub mod serial;

#[derive(Debug, Default)]
#[repr(C)]
pub struct CpuContext {
    // Platform-defined context
    // This is simplified for demonstration
    pub sp: usize,
    pub ip: usize,
}

/// Trait for performing low-level context switches.
pub trait ContextSwitch {
    /// Save the current CPU state (registers, flags) into this context.
    unsafe extern "C" fn save(&mut self);

    /// Restore the CPU state from this context.
    /// This function generally does not return.
    unsafe extern "C" fn restore(&self) -> !;

    /// Switch from the current context to the specific next context.
    /// This is the most common operation.
    unsafe extern "C" fn switch_to(&mut self, next: &Self);
}

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

pub mod iommu;
pub mod wait;
pub use crate::kernel::syscalls::syscalls_consts;

// Re-export specific architectures based on build target
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;
