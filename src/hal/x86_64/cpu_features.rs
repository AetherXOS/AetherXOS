use raw_cpuid::CpuId;
use core::sync::atomic::{AtomicU64, Ordering};

/// Global bitmask of detected CPU features.
/// Bits are defined locally or using standard feature flags.
static FEATURE_FLAGS: AtomicU64 = AtomicU64::new(0);

pub const FEATURE_FPU:   u64 = 1 << 0;
pub const FEATURE_SSE:   u64 = 1 << 1;
pub const FEATURE_AVX:   u64 = 1 << 2;
pub const FEATURE_APIC:  u64 = 1 << 3;
pub const FEATURE_FSGS:  u64 = 1 << 4; // FSGSBASE
pub const FEATURE_NX:    u64 = 1 << 5; // No-Execute
pub const FEATURE_PCID:  u64 = 1 << 6; // Process Context IDs
pub const FEATURE_UMIP:  u64 = 1 << 7; // User-Mode Instruction Prevention

pub fn init() {
    let cpuid = CpuId::new();
    let mut flags = 0u64;

    if let Some(f) = cpuid.get_feature_info() {
        if f.has_fpu()  { flags |= FEATURE_FPU; }
        if f.has_sse()  { flags |= FEATURE_SSE; }
        if f.has_apic() { flags |= FEATURE_APIC; }
    }

    if let Some(f) = cpuid.get_extended_feature_info() {
        if f.has_fsgsbase() { flags |= FEATURE_FSGS; }
        if f.has_avx()      { flags |= FEATURE_AVX; }
        if f.has_umip()     { flags |= FEATURE_UMIP; }
    }

    if let Some(f) = cpuid.get_extended_processor_and_feature_identifiers() {
        if f.has_execute_disable() { flags |= FEATURE_NX; }
    }

    FEATURE_FLAGS.store(flags, Ordering::SeqCst);
    
    crate::klog_info!("x86_64 CPU features detected: 0x{:X}", flags);
}

pub fn has_feature(feature_bit: u64) -> bool {
    (FEATURE_FLAGS.load(Ordering::Acquire) & feature_bit) != 0
}
