//! Generic CPU Abstractions
//!
//! Delegates to architecture-specific implementations.

#[cfg(target_arch = "x86_64")]
pub use crate::hal::x86_64::cpu::*;

#[cfg(target_arch = "aarch64")]
pub use crate::hal::aarch64::cpu::*;

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub fn id() -> usize {
    0
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub unsafe fn get_per_cpu_ptr() -> *const () {
    core::ptr::null()
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub fn rdtsc() -> u64 {
    0
}

/// Check if AES-NI is available on this platform.
pub fn has_aes_ni() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        crate::hal::x86_64::cpu::has_aes_ni()
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Check if SHA-NI is available on this platform.
pub fn has_sha_ni() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        crate::hal::x86_64::cpu::has_sha_ni()
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

#[inline(always)]
pub fn id_typed() -> crate::interfaces::task::CpuId {
    crate::interfaces::task::CpuId(id())
}

#[cfg(target_arch = "x86_64")]
pub type ArchCpuRegisters = crate::hal::x86_64::cpu::X86CpuRegisters;

#[cfg(target_arch = "aarch64")]
pub type ArchCpuRegisters = crate::hal::aarch64::cpu::AArch64CpuRegisters;
