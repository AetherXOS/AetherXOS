use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static HW_DETECT_CALLS: AtomicU64 = AtomicU64::new(0);
static SMEP_ENABLED: AtomicBool = AtomicBool::new(false);
static SMAP_ENABLED: AtomicBool = AtomicBool::new(false);
static NX_ENABLED: AtomicBool = AtomicBool::new(false);
static UMIP_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareSecurityBackend {
    None,
    IntelSgx,
    ArmTrustZone,
}

/// CPU-level protection feature status.
#[derive(Debug, Clone, Copy)]
pub struct CpuProtectionStatus {
    /// SMEP: Supervisor Mode Execution Prevention — prevents Ring 0 from
    /// executing code in user-mapped pages.
    pub smep: bool,
    /// SMAP: Supervisor Mode Access Prevention — prevents Ring 0 from
    /// reading/writing user-mapped pages unless explicitly overridden.
    pub smap: bool,
    /// NX/XD: No-Execute / Execute-Disable bit support.
    pub nx: bool,
    /// UMIP: User-Mode Instruction Prevention — blocks SGDT/SIDT/SLDT/SMSW/STR
    /// from user mode.
    pub umip: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct HardwareSecurityStats {
    pub detect_calls: u64,
    pub backend: HardwareSecurityBackend,
    pub cpu_protection: CpuProtectionStatus,
}

pub fn detect_hardware_security() -> HardwareSecurityBackend {
    HW_DETECT_CALLS.fetch_add(1, Ordering::Relaxed);

    #[cfg(target_arch = "x86_64")]
    {
        let leaf = core::arch::x86_64::__cpuid_count(0x07, 0);
        if (leaf.ebx & (1 << 2)) != 0 {
            return HardwareSecurityBackend::IntelSgx;
        }
        return HardwareSecurityBackend::None;
    }

    #[cfg(target_arch = "aarch64")]
    {
        let id_pfr1: u64;
        unsafe {
            core::arch::asm!("mrs {}, id_aa64pfr0_el1", out(reg) id_pfr1, options(nomem, nostack));
        }
        // Check bits [15:12] for Security Extension (EL3) in ID_AA64PFR0_EL1
        if ((id_pfr1 >> 12) & 0xF) != 0 {
            return HardwareSecurityBackend::ArmTrustZone;
        }
        return HardwareSecurityBackend::None;
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        return HardwareSecurityBackend::None;
    }
}

pub fn hardware_security_stats() -> HardwareSecurityStats {
    HardwareSecurityStats {
        detect_calls: HW_DETECT_CALLS.load(Ordering::Relaxed),
        backend: detect_hardware_security(),
        cpu_protection: cpu_protection_status(),
    }
}

/// Returns current CPU protection feature status.
pub fn cpu_protection_status() -> CpuProtectionStatus {
    CpuProtectionStatus {
        smep: SMEP_ENABLED.load(Ordering::Relaxed),
        smap: SMAP_ENABLED.load(Ordering::Relaxed),
        nx: NX_ENABLED.load(Ordering::Relaxed),
        umip: UMIP_ENABLED.load(Ordering::Relaxed),
    }
}

// ─── SMEP / SMAP / NX / UMIP Detection & Enforcement ───────────────

/// Detect and enable all available CPU-level protection features.
/// Must be called once during early kernel boot (BSP).
pub fn enforce_cpu_protections() {
    #[cfg(target_arch = "x86_64")]
    {
        enforce_cpu_protections_x86_64();
    }
    #[cfg(target_arch = "aarch64")]
    {
        enforce_cpu_protections_aarch64();
    }
}

#[cfg(target_arch = "x86_64")]
fn enforce_cpu_protections_x86_64() {
    // CPUID leaf 7, sub-leaf 0: structured extended feature flags
    let leaf7 = core::arch::x86_64::__cpuid_count(0x07, 0);

    // SMEP — bit 7 of EBX
    let has_smep = (leaf7.ebx & (1 << 7)) != 0;
    // SMAP — bit 20 of EBX
    let has_smap = (leaf7.ebx & (1 << 20)) != 0;
    // UMIP — bit 2 of ECX
    let has_umip = (leaf7.ecx & (1 << 2)) != 0;

    // NX/XD: CPUID leaf 0x80000001, EDX bit 20
    let leaf_ext = core::arch::x86_64::__cpuid(0x80000001);
    let has_nx = (leaf_ext.edx & (1 << 20)) != 0;

    // Read CR4
    let mut cr4: u64;
    unsafe {
        core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack));
    }

    if has_smep {
        cr4 |= 1 << 20; // CR4.SMEP
        SMEP_ENABLED.store(true, Ordering::Relaxed);
    }
    if has_smap {
        cr4 |= 1 << 21; // CR4.SMAP
        SMAP_ENABLED.store(true, Ordering::Relaxed);
    }
    if has_umip {
        cr4 |= 1 << 11; // CR4.UMIP
        UMIP_ENABLED.store(true, Ordering::Relaxed);
    }

    // Write back CR4 with protection bits set
    unsafe {
        core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack));
    }

    // Enable NX via IA32_EFER MSR (bit 11)
    if has_nx {
        const IA32_EFER: u32 = 0xC000_0080;
        let efer_lo: u32;
        let efer_hi: u32;
        unsafe {
            core::arch::asm!(
                "rdmsr",
                in("ecx") IA32_EFER,
                out("eax") efer_lo,
                out("edx") efer_hi,
                options(nomem, nostack),
            );
        }
        let efer = ((efer_hi as u64) << 32) | (efer_lo as u64);
        let efer_new = efer | (1 << 11); // NXE bit
        let new_lo = efer_new as u32;
        let new_hi = (efer_new >> 32) as u32;
        unsafe {
            core::arch::asm!(
                "wrmsr",
                in("ecx") IA32_EFER,
                in("eax") new_lo,
                in("edx") new_hi,
                options(nomem, nostack),
            );
        }
        NX_ENABLED.store(true, Ordering::Relaxed);
    }
}

#[cfg(target_arch = "aarch64")]
fn enforce_cpu_protections_aarch64() {
    // AArch64: WXN (Write-implies-XN) and PAN (Privileged Access Never)
    // are controlled via SCTLR_EL1.

    // Read SCTLR_EL1
    let mut sctlr: u64;
    unsafe {
        core::arch::asm!("mrs {}, sctlr_el1", out(reg) sctlr, options(nomem, nostack));
    }

    // WXN — bit 19: any writable page is automatically non-executable (analogue of NX)
    sctlr |= 1 << 19;
    NX_ENABLED.store(true, Ordering::Relaxed);

    // Check PAN support (ID_AA64MMFR1_EL1 bits [23:20])
    let mmfr1: u64;
    unsafe {
        core::arch::asm!("mrs {}, id_aa64mmfr1_el1", out(reg) mmfr1, options(nomem, nostack));
    }
    let pan_support = (mmfr1 >> 20) & 0xF;
    if pan_support >= 1 {
        // Enable PAN — PSTATE.PAN = 1 (analogue of SMAP)
        unsafe {
            core::arch::asm!("msr S3_0_C4_C1_1, {0}", in(reg) 1u64, options(nomem, nostack));
        }
        SMAP_ENABLED.store(true, Ordering::Relaxed);
    }

    // Write updated SCTLR_EL1
    unsafe {
        core::arch::asm!("msr sctlr_el1, {}", in(reg) sctlr, options(nomem, nostack));
        core::arch::asm!("isb", options(nomem, nostack));
    }

    // SMEP equivalent: EL1 cannot execute EL0 pages via PXN (Privileged Execute Never)
    // This is enforced at page-table level in AArch64 — set SMEP flag to indicate awareness
    SMEP_ENABLED.store(true, Ordering::Relaxed);
}

/// Temporarily disable SMAP for a controlled kernel access to user memory.
/// Returns a guard that re-enables SMAP on drop.
pub fn smap_disable_guard() -> SmapGuard {
    #[cfg(target_arch = "x86_64")]
    {
        if SMAP_ENABLED.load(Ordering::Relaxed) {
            unsafe {
                core::arch::asm!("stac", options(nomem, nostack));
            }
            return SmapGuard { was_active: true };
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        if SMAP_ENABLED.load(Ordering::Relaxed) {
            unsafe {
                core::arch::asm!("msr S3_0_C4_C1_1, {0}", in(reg) 0u64, options(nomem, nostack));
            }
            return SmapGuard { was_active: true };
        }
    }
    SmapGuard { was_active: false }
}

/// RAII guard that re-enables SMAP/PAN on drop.
pub struct SmapGuard {
    was_active: bool,
}

impl Drop for SmapGuard {
    fn drop(&mut self) {
        if self.was_active {
            #[cfg(target_arch = "x86_64")]
            unsafe {
                core::arch::asm!("clac", options(nomem, nostack));
            }
            #[cfg(target_arch = "aarch64")]
            unsafe {
                core::arch::asm!("msr S3_0_C4_C1_1, {0}", in(reg) 1u64, options(nomem, nostack));
            }
        }
    }
}

// ─── Hardware Enclave & Trusted Execution Calls ─────────────────────

#[cfg(target_arch = "x86_64")]
#[inline(always)]
/// Intel SGX Enclave Call Wrapper (Ring 0 Setup Instructions)
pub unsafe fn encls_leaf(leaf: u32, rbx: u64, rcx: u64, rdx: u64) -> (u64, u64) {
    let out_bx: u64;
    let out_cx: u64;
    // Safety: caller guarantees the current CPU/ring supports the requested ENCLS leaf.
    unsafe {
        core::arch::asm!(
            "xchg rbx, {temp_bx}",
            "encls",
            "xchg rbx, {temp_bx}",
            temp_bx = inout(reg) rbx => out_bx,
            in("eax") leaf,
            inout("rcx") rcx => out_cx,
            inout("rdx") rdx => _,
            options(nomem, nostack)
        );
    }
    (out_bx, out_cx)
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
/// Arm TrustZone Secure Monitor Call (SMC) Wrapper
pub unsafe fn smc_call(x0: u64, x1: u64, x2: u64, x3: u64) -> (u64, u64, u64, u64) {
    let out0: u64;
    let out1: u64;
    let out2: u64;
    let out3: u64;
    unsafe {
        core::arch::asm!(
            "smc #0",
            inout("x0") x0 => out0,
            inout("x1") x1 => out1,
            inout("x2") x2 => out2,
            inout("x3") x3 => out3,
            options(nomem, nostack)
        );
    }
    (out0, out1, out2, out3)
}
