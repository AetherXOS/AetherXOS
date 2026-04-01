//! x86_64 CPU register access, utilities, and feature detection.

use crate::hal::common::cpu_features::{field_at_least_u64, has_bit_u32};
use crate::interfaces::cpu::CpuRegisters;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__cpuid, __cpuid_count, __rdtscp, _rdtsc};
use x86_64::registers::control::{Cr2, Cr3};
use x86_64::registers::model_specific::{FsBase, GsBase};
use x86_64::structures::paging::PhysFrame;
use x86_64::PhysAddr;
use x86_64::VirtAddr;

// ── CPU Identification ────────────────────────────────────────────────────────

/// Returns the logical CPU index stored in the `CpuLocal` structure.
/// The BSP sets this up during `early_init`, and APs set it during `ap_entry`.
/// The value is stored at offset 0 of the GS segment.
#[inline(always)]
pub fn id() -> usize {
    let cpu_id: usize;
    unsafe {
        core::arch::asm!("mov {}, gs:[0]", out(reg) cpu_id, options(readonly, nostack));
    }
    cpu_id
}

/// Get the raw GS base pointer.
#[inline(always)]
pub unsafe fn get_per_cpu_ptr() -> *const () {
    GsBase::read().as_u64() as *const ()
}

// ── Diagnostics and Features ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
pub struct CpuFeatures {
    pub sse: bool,
    pub sse2: bool,
    pub sse3: bool,
    pub sse4_1: bool,
    pub sse4_2: bool,
    pub avx: bool,
    pub avx2: bool,
    pub pdpe1gb: bool, // 1GB huge pages
    pub rdtscp: bool,
    pub vmx: bool,
    pub svm: bool,
    pub x2apic: bool,
    pub invariant_tsc: bool,
}

/// Detect CPU features using the `cpuid` instruction.
pub fn detect_features() -> CpuFeatures {
    let mut f = CpuFeatures::default();
    let max_leaf = __cpuid(0).eax;
    if max_leaf >= 1 {
        let res = __cpuid(1);
        f.sse = has_bit_u32(res.edx, 25);
        f.sse2 = has_bit_u32(res.edx, 26);
        f.sse3 = has_bit_u32(res.ecx, 0);
        f.vmx = has_bit_u32(res.ecx, 5);
        f.sse4_1 = has_bit_u32(res.ecx, 19);
        f.sse4_2 = has_bit_u32(res.ecx, 20);
        f.x2apic = has_bit_u32(res.ecx, 21);
        f.avx = has_bit_u32(res.ecx, 28);
    }

    if max_leaf >= 7 {
        let res7 = __cpuid_count(7, 0);
        f.avx2 = has_bit_u32(res7.ebx, 5);
    }

    let max_ext_leaf = __cpuid(0x8000_0000).eax;
    if max_ext_leaf >= 0x8000_0001 {
        let res_ext = __cpuid(0x8000_0001);
        f.svm = has_bit_u32(res_ext.ecx, 2);
        f.pdpe1gb = has_bit_u32(res_ext.edx, 26);
        f.rdtscp = has_bit_u32(res_ext.edx, 27);
    }

    if max_ext_leaf >= 0x8000_0007 {
        let res_inv = __cpuid(0x8000_0007);
        f.invariant_tsc = field_at_least_u64(res_inv.edx as u64, 8, 0x1, 1);
    }
    f
}

// ── Timers and Power ──────────────────────────────────────────────────────────

/// Read the Time Stamp Counter (RDTSC).
#[inline(always)]
pub fn rdtsc() -> u64 {
    unsafe { _rdtsc() }
}

/// Read the Time Stamp Counter and Processor ID (RDTSCP).
/// Returns `(tsc_value, core_id)`.
#[inline(always)]
pub fn rdtscp() -> (u64, u32) {
    let mut aux = 0;
    let tsc = unsafe { __rdtscp(&mut aux) };
    (tsc, aux)
}

/// Best-effort TSC frequency discovery in Hz.
/// Uses CPUID leaf 0x15 when available, then falls back to base MHz from 0x16.
pub fn tsc_frequency_hz() -> Option<u64> {
    let max_leaf = __cpuid(0).eax;
    if max_leaf >= 0x15 {
        let leaf15 = __cpuid(0x15);
        let denom = leaf15.eax as u64;
        let numer = leaf15.ebx as u64;
        let crystal_hz = leaf15.ecx as u64;
        if denom != 0 && numer != 0 && crystal_hz != 0 {
            return crystal_hz.checked_mul(numer)?.checked_div(denom);
        }
    }
    if max_leaf >= 0x16 {
        let leaf16 = __cpuid(0x16);
        let base_mhz = leaf16.eax as u64;
        if base_mhz != 0 {
            return base_mhz.checked_mul(1_000_000);
        }
    }
    None
}

/// Pause the CPU pipeline (useful in spinlocks).
#[inline(always)]
pub fn pause() {
    unsafe {
        core::arch::asm!("pause", options(nomem, nostack));
    }
}

/// Invalidate a specific page from the TLB.
// ── MSR Access ────────────────────────────────────────────────────────────────

/// Read a Model Specific Register (MSR).

pub unsafe fn read_msr(msr: u32) -> u64 {
    let (high, low): (u32, u32);

    // Safety: caller guarantees the requested MSR is valid in the current execution mode.
    unsafe {
        core::arch::asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high, options(nomem, nostack));
    }

    ((high as u64) << 32) | (low as u64)
}

/// Write to a Model Specific Register (MSR).

pub unsafe fn write_msr(msr: u32, value: u64) {
    let low = value as u32;

    let high = (value >> 32) as u32;

    // Safety: caller guarantees the requested MSR is valid in the current execution mode.
    unsafe {
        core::arch::asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nomem, nostack));
    }
}

#[inline(always)]

pub fn invlpg(addr: u64) {
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) addr, options(nostack));
    }
}

// ── CpuRegisters Impl ─────────────────────────────────────────────────────────

pub struct X86CpuRegisters;

impl CpuRegisters for X86CpuRegisters {
    fn read_page_fault_addr() -> u64 {
        Cr2::read().as_u64()
    }

    fn read_page_table_root() -> u64 {
        let (frame, _) = Cr3::read();
        frame.start_address().as_u64()
    }

    fn write_page_table_root(addr: u64) {
        let frame = PhysFrame::containing_address(PhysAddr::new(addr));
        let (_, flags) = Cr3::read();
        unsafe {
            Cr3::write(frame, flags);
        }
    }

    fn read_tls_base() -> u64 {
        FsBase::read().as_u64()
    }

    fn write_tls_base(addr: u64) {
        FsBase::write(VirtAddr::new(addr));
    }

    fn read_per_cpu_base() -> u64 {
        GsBase::read().as_u64()
    }

    fn write_per_cpu_base(addr: u64) {
        GsBase::write(VirtAddr::new(addr));
    }
}
