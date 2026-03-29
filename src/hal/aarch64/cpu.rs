/// AArch64 CPU register access and utilities.
///
/// Provides:
///   - CPU-local ID discovery via TPIDR_EL1
///   - High-resolution timer via CNTVCT_EL0
///   - Full `CpuRegisters` trait implementation
///   - Cache maintenance operations (icache invalidate, dcache clean/invalidate)
///   - Barrier helpers (DSB, ISB, DMB)
///   - MPIDR-based topology detection (cluster, core, thread)
///   - Feature detection (FP, Advanced SIMD, SVE, CRC32, Crypto)
use crate::hal::common::cpu_features::{field_at_least_u64, field_present_u64};
use crate::interfaces::cpu::CpuRegisters;
use core::sync::atomic::{AtomicU64, Ordering};

const NANOS_PER_SEC: u128 = 1_000_000_000u128;
const CACHE_LINE_BYTES: usize = 64;
const MPIDR_AFF_MASK: u64 = 0xFF;
const MPIDR_MT_BIT: u64 = 1 << 24;
const ID_FIELD_MASK: u64 = 0xF;
const ID_FEATURE_ABSENT: u64 = 0xF;
static COUNTER_FREQ_HZ: AtomicU64 = AtomicU64::new(0);

// ── CPU-local ID ──────────────────────────────────────────────────────────────

/// Returns the logical CPU index stored in TPIDR_EL1 by the CpuLocal init path.
/// Returns 0 (BSP) if TPIDR_EL1 is not yet set.
#[inline(always)]
pub fn id() -> usize {
    let ptr: u64;
    unsafe {
        core::arch::asm!("mrs {}, tpidr_el1", out(reg) ptr, options(nomem, nostack));
    }
    if ptr == 0 {
        0
    } else {
        unsafe { *(ptr as *const usize) }
    }
}

/// Returns the raw TPIDR_EL1 value as a pointer-sized integer.
#[inline(always)]
pub unsafe fn get_per_cpu_ptr() -> *const () {
    let ptr: u64;
    core::arch::asm!("mrs {}, tpidr_el1", out(reg) ptr, options(nomem, nostack));
    ptr as *const ()
}

// ── High-resolution timer ─────────────────────────────────────────────────────

/// Read the AArch64 virtual counter (CNTVCT_EL0).
/// On most SoCs this runs at a fixed frequency readable via CNTFRQ_EL0.
#[inline(always)]
pub fn rdtsc() -> u64 {
    let val: u64;
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) val, options(nomem, nostack));
    }
    val
}

/// Read the virtual counter frequency in Hz (CNTFRQ_EL0).
#[inline(always)]
pub fn counter_frequency() -> u64 {
    let cached = COUNTER_FREQ_HZ.load(Ordering::Relaxed);
    if cached != 0 {
        return cached;
    }
    let freq: u64;
    unsafe {
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq, options(nomem, nostack));
    }
    let sanitized = freq.max(1);
    let _ = COUNTER_FREQ_HZ.compare_exchange(0, sanitized, Ordering::Relaxed, Ordering::Relaxed);
    COUNTER_FREQ_HZ.load(Ordering::Relaxed).max(1)
}

/// Force-refresh cached virtual counter frequency from CNTFRQ_EL0.
#[inline(always)]
pub fn refresh_counter_frequency() -> u64 {
    let freq: u64;
    unsafe {
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq, options(nomem, nostack));
    }
    let sanitized = freq.max(1);
    COUNTER_FREQ_HZ.store(sanitized, Ordering::Relaxed);
    sanitized
}

/// Convert a raw counter tick delta to nanoseconds.
pub fn ticks_to_ns(ticks: u64) -> u64 {
    let freq = counter_frequency().max(1);
    let ns = (ticks as u128).saturating_mul(NANOS_PER_SEC) / (freq as u128);
    ns.min(u64::MAX as u128) as u64
}

// ── MPIDR topology ────────────────────────────────────────────────────────────

/// Decoded MPIDR_EL1 topology fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpidrTopology {
    /// Affinity level 0 — logical core within cluster.
    pub aff0: u8,
    /// Affinity level 1 — cluster.
    pub aff1: u8,
    /// Affinity level 2 — socket / die.
    pub aff2: u8,
    /// Affinity level 3 — system / package (ARMv8.3+).
    pub aff3: u8,
    /// Multi-threading indicator (MT bit, MPIDR[24]).
    pub multi_thread: bool,
}

/// Read and decode MPIDR_EL1.
pub fn mpidr_topology() -> MpidrTopology {
    let mpidr: u64;
    unsafe {
        core::arch::asm!("mrs {}, mpidr_el1", out(reg) mpidr, options(nomem, nostack));
    }
    MpidrTopology {
        aff0: (mpidr & MPIDR_AFF_MASK) as u8,
        aff1: ((mpidr >> 8) & MPIDR_AFF_MASK) as u8,
        aff2: ((mpidr >> 16) & MPIDR_AFF_MASK) as u8,
        aff3: ((mpidr >> 32) & MPIDR_AFF_MASK) as u8,
        multi_thread: (mpidr & MPIDR_MT_BIT) != 0,
    }
}

/// Return the raw 64-bit MPIDR value (useful for PSCI calls).
#[inline(always)]
pub fn raw_mpidr() -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!("mrs {}, mpidr_el1", out(reg) v, options(nomem, nostack));
    }
    v
}

// ── Memory barriers ───────────────────────────────────────────────────────────

/// Data Synchronization Barrier — Inner Shareable.
#[inline(always)]
pub fn dsb_ish() {
    unsafe {
        core::arch::asm!("dsb ish", options(nomem, nostack));
    }
}

/// Data Synchronization Barrier — System.
#[inline(always)]
pub fn dsb_sy() {
    unsafe {
        core::arch::asm!("dsb sy", options(nostack));
    }
}

/// Data Memory Barrier — Inner Shareable.
#[inline(always)]
pub fn dmb_ish() {
    unsafe {
        core::arch::asm!("dmb ish", options(nomem, nostack));
    }
}

/// Instruction Synchronization Barrier.
#[inline(always)]
pub fn isb() {
    unsafe {
        core::arch::asm!("isb", options(nomem, nostack));
    }
}

// ── Cache maintenance ─────────────────────────────────────────────────────────

/// Invalidate the ICache from PoU (Point of Unification) for the entire IS.
pub fn icache_invalidate_all_inner_shareable() {
    unsafe {
        core::arch::asm!("ic ialluis", "dsb ish", "isb", options(nostack));
    }
}

/// Clean and invalidate a single D-cache line by virtual address to PoC.
#[inline(always)]
pub fn dcache_clean_invalidate_line(addr: usize) {
    unsafe {
        core::arch::asm!(
            "dc civac, {}",
            in(reg) addr,
            options(nostack)
        );
    }
}

/// Clean a range of virtual addresses to the Point of Coherency.
/// `start` must be cache-line aligned (64 bytes typical).
pub fn dcache_clean_range(start: usize, len: usize) {
    if len == 0 {
        return;
    }
    let end = start.saturating_add(len);
    let mut addr = start & !(CACHE_LINE_BYTES - 1);
    while addr < end {
        unsafe {
            core::arch::asm!("dc cvac, {}", in(reg) addr, options(nostack));
        }
        match addr.checked_add(CACHE_LINE_BYTES) {
            Some(next) => addr = next,
            None => break,
        }
    }
    dsb_sy();
}

/// Invalidate (but do not clean) a range of virtual D-cache lines.
pub fn dcache_invalidate_range(start: usize, len: usize) {
    if len == 0 {
        return;
    }
    let end = start.saturating_add(len);
    let mut addr = start & !(CACHE_LINE_BYTES - 1);
    while addr < end {
        unsafe {
            core::arch::asm!("dc ivac, {}", in(reg) addr, options(nostack));
        }
        match addr.checked_add(CACHE_LINE_BYTES) {
            Some(next) => addr = next,
            None => break,
        }
    }
    dsb_sy();
}

// ── CPU feature detection ─────────────────────────────────────────────────────

/// ARM CPU feature flags detected from ID_AA64ISAR0_EL1 / ID_AA64PFR0_EL1.
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuFeatures {
    pub fp: bool,          // Floating-point (mandatory ARMv8.0)
    pub asimd: bool,       // Advanced SIMD / Neon
    pub sve: bool,         // Scalable Vector Extension
    pub crc32: bool,       // CRC32 instructions
    pub crypto_aes: bool,  // AES crypto extension
    pub crypto_sha1: bool, // SHA-1
    pub crypto_sha2: bool, // SHA-256/512
    pub lse: bool,         // Large System Extensions (atomic ops)
    pub rdm: bool,         // Rounding Double Multiply
    pub dotprod: bool,     // Dot Product
    pub rndr: bool,        // Random number generation (RNDR/RNDRRS)
}

/// Read CPU feature registers and return decoded flags.
pub fn detect_features() -> CpuFeatures {
    let isar0: u64;
    let pfr0: u64;
    let isar1: u64;

    unsafe {
        core::arch::asm!("mrs {}, id_aa64isar0_el1", out(reg) isar0, options(nomem, nostack));
        core::arch::asm!("mrs {}, id_aa64pfr0_el1",  out(reg) pfr0,  options(nomem, nostack));
        core::arch::asm!("mrs {}, id_aa64isar1_el1", out(reg) isar1, options(nomem, nostack));
    }

    CpuFeatures {
        fp: field_present_u64(pfr0, 16, ID_FIELD_MASK, ID_FEATURE_ABSENT), // FP: 0=supported, 0xF=not present
        asimd: field_present_u64(pfr0, 20, ID_FIELD_MASK, ID_FEATURE_ABSENT), // AdvSIMD
        sve: field_at_least_u64(pfr0, 32, ID_FIELD_MASK, 1),
        crc32: field_at_least_u64(isar0, 16, ID_FIELD_MASK, 1),
        crypto_aes: field_at_least_u64(isar0, 4, ID_FIELD_MASK, 1),
        crypto_sha1: field_at_least_u64(isar0, 8, ID_FIELD_MASK, 1),
        crypto_sha2: field_at_least_u64(isar0, 12, ID_FIELD_MASK, 1),
        lse: field_at_least_u64(isar0, 20, ID_FIELD_MASK, 2),
        rdm: field_at_least_u64(isar0, 28, ID_FIELD_MASK, 1),
        dotprod: field_at_least_u64(isar1, 44, ID_FIELD_MASK, 1),
        rndr: field_at_least_u64(isar1, 60, ID_FIELD_MASK, 1),
    }
}

// ── CpuRegisters trait ────────────────────────────────────────────────────────

pub struct AArch64CpuRegisters;

impl CpuRegisters for AArch64CpuRegisters {
    fn read_page_fault_addr() -> u64 {
        let far: u64;
        unsafe {
            core::arch::asm!("mrs {}, far_el1", out(reg) far, options(nomem, nostack));
        }
        far
    }

    fn read_page_table_root() -> u64 {
        let ttbr0: u64;
        unsafe {
            core::arch::asm!("mrs {}, ttbr0_el1", out(reg) ttbr0, options(nomem, nostack));
        }
        ttbr0
    }

    fn write_page_table_root(addr: u64) {
        unsafe {
            core::arch::asm!("msr ttbr0_el1, {}", in(reg) addr, options(nomem, nostack));
            core::arch::asm!("isb", options(nomem, nostack));
        }
    }

    fn read_tls_base() -> u64 {
        let v: u64;
        unsafe {
            core::arch::asm!("mrs {}, tpidr_el0", out(reg) v, options(nomem, nostack));
        }
        v
    }

    fn write_tls_base(addr: u64) {
        unsafe {
            core::arch::asm!("msr tpidr_el0, {}", in(reg) addr, options(nomem, nostack));
        }
    }

    fn read_per_cpu_base() -> u64 {
        let v: u64;
        unsafe {
            core::arch::asm!("mrs {}, tpidr_el1", out(reg) v, options(nomem, nostack));
        }
        v
    }

    fn write_per_cpu_base(addr: u64) {
        unsafe {
            core::arch::asm!("msr tpidr_el1, {}", in(reg) addr, options(nomem, nostack));
        }
    }
}
