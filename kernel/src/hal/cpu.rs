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

#[inline(always)]
pub fn id_typed() -> crate::interfaces::task::CpuId {
    crate::interfaces::task::CpuId(id())
}

#[cfg(target_arch = "x86_64")]
pub type ArchCpuRegisters = crate::hal::x86_64::cpu::X86CpuRegisters;

#[cfg(target_arch = "aarch64")]
pub type ArchCpuRegisters = crate::hal::aarch64::cpu::AArch64CpuRegisters;
