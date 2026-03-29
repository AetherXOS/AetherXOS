use crate::hal::common::virt::current_virtualization_power_tuning;
#[cfg(test)]
use crate::hal::common::virt::{virtualization_power_tuning, VirtualizationPowerTuning};
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

// ── MSR / hw-register constants ───────────────────────────────────────────────

/// Intel SpeedStep performance control MSR.
/// Bits 15:8 hold the target P-state ratio (ratio × 100 MHz = frequency).
const MSR_IA32_PERF_CTL: u32 = 0x199;

/// Ratio values written to IA32_PERF_CTL.  These are sane defaults for most
/// Intel mobile/desktop CPUs; a production driver would discover them via CPUID
/// or ACPI _PSS objects.
const PSTATE_RATIO_HIGH_PERF: u64 = 0x20; // ~3.2 GHz at base-100 MHz
const PSTATE_RATIO_BALANCED: u64 = 0x18; // ~2.4 GHz
const PSTATE_RATIO_POWER_SAVE: u64 = 0x08; // ~0.8 GHz (near p-floor)

const OVERRIDE_NONE: u8 = 0xFF;
const MAX_OVERRIDE_INDEX: u8 = 2;
const ACPI_PROFILE_DISABLED: u8 = 0;
const ACPI_PROFILE_ENABLED: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CState {
    C1 = 0,
    C2 = 1,
    C3 = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PState {
    HighPerf = 0,
    Balanced = 1,
    PowerSave = 2,
}

impl_enum_u8_default_conversions!(CState {
    C1,
    C2,
    C3,
}, default = C1);

impl_enum_u8_default_conversions!(PState {
    HighPerf,
    Balanced,
    PowerSave,
}, default = Balanced);

#[derive(Debug, Clone, Copy)]
pub struct PowerStats {
    pub idle_calls: u64,
    pub c1_entries: u64,
    pub c2_entries: u64,
    pub c3_entries: u64,
    pub pstate_switches: u64,
    pub current_pstate: PState,
    pub policy_override_active: bool,
    pub policy_override_set_calls: u64,
    pub policy_override_clear_calls: u64,
    pub cstate_override_active: bool,
    pub cstate_override_set_calls: u64,
    pub cstate_override_clear_calls: u64,
    pub acpi_profile_loaded: bool,
    pub acpi_fadt_revision: u8,
    pub policy_guard_hits: u64,
    pub runqueue_clamp_events: u64,
    pub failsafe_idle_fallbacks: u64,
    pub override_rejects_no_acpi: u64,
}

static IDLE_CALLS: AtomicU64 = AtomicU64::new(0);
static C1_ENTRIES: AtomicU64 = AtomicU64::new(0);
static C2_ENTRIES: AtomicU64 = AtomicU64::new(0);
static C3_ENTRIES: AtomicU64 = AtomicU64::new(0);
static PSTATE_SWITCHES: AtomicU64 = AtomicU64::new(0);
static CURRENT_PSTATE: AtomicU8 = AtomicU8::new(1);
static PSTATE_OVERRIDE: AtomicU8 = AtomicU8::new(OVERRIDE_NONE);
static PSTATE_OVERRIDE_SET_CALLS: AtomicU64 = AtomicU64::new(0);
static PSTATE_OVERRIDE_CLEAR_CALLS: AtomicU64 = AtomicU64::new(0);
static CSTATE_OVERRIDE: AtomicU8 = AtomicU8::new(OVERRIDE_NONE);
static CSTATE_OVERRIDE_SET_CALLS: AtomicU64 = AtomicU64::new(0);
static CSTATE_OVERRIDE_CLEAR_CALLS: AtomicU64 = AtomicU64::new(0);
static ACPI_PROFILE_LOADED: AtomicU8 = AtomicU8::new(0);
static ACPI_FADT_REVISION: AtomicU8 = AtomicU8::new(0);
static POLICY_GUARD_HITS: AtomicU64 = AtomicU64::new(0);
static RUNQUEUE_CLAMP_EVENTS: AtomicU64 = AtomicU64::new(0);
static FAILSAFE_IDLE_FALLBACKS: AtomicU64 = AtomicU64::new(0);
static OVERRIDE_REJECTS_NO_ACPI: AtomicU64 = AtomicU64::new(0);

fn choose_cstate(runqueue_len: usize) -> CState {
    let override_raw = CSTATE_OVERRIDE.load(Ordering::Relaxed);
    if override_raw <= MAX_OVERRIDE_INDEX {
        return CState::from_u8(override_raw);
    }

    let acpi_loaded = ACPI_PROFILE_LOADED.load(Ordering::Relaxed) == ACPI_PROFILE_ENABLED;
    if !acpi_loaded {
        FAILSAFE_IDLE_FALLBACKS.fetch_add(1, Ordering::Relaxed);
        return CState::C1;
    }

    let fadt_rev = ACPI_FADT_REVISION.load(Ordering::Relaxed);
    let tuning = current_virtualization_power_tuning();
    if tuning.prefer_shallow_idle {
        if runqueue_len == 0 {
            CState::C2
        } else {
            CState::C1
        }
    } else if runqueue_len == 0 {
        if fadt_rev >= 3 {
            CState::C3
        } else {
            CState::C2
        }
    } else if runqueue_len <= 1 {
        CState::C2
    } else {
        CState::C1
    }
}

fn choose_pstate(runqueue_len: usize) -> PState {
    let override_raw = PSTATE_OVERRIDE.load(Ordering::Relaxed);
    if override_raw <= MAX_OVERRIDE_INDEX {
        return PState::from_u8(override_raw);
    }

    if ACPI_PROFILE_LOADED.load(Ordering::Relaxed) == ACPI_PROFILE_DISABLED {
        FAILSAFE_IDLE_FALLBACKS.fetch_add(1, Ordering::Relaxed);
        return PState::HighPerf;
    }

    let tuning = current_virtualization_power_tuning();
    if tuning.prefer_active_pstate {
        if runqueue_len >= 1 {
            PState::HighPerf
        } else {
            PState::Balanced
        }
    } else if runqueue_len >= 4 {
        PState::HighPerf
    } else if runqueue_len >= 1 {
        PState::Balanced
    } else {
        PState::PowerSave
    }
}

fn runqueue_hint_clamped(runqueue_len: usize) -> usize {
    let saturation_limit = crate::config::KernelConfig::power_runqueue_saturation_limit();
    if runqueue_len > saturation_limit {
        RUNQUEUE_CLAMP_EVENTS.fetch_add(1, Ordering::Relaxed);
        saturation_limit
    } else {
        runqueue_len
    }
}

fn override_requires_acpi(pstate: PState) -> bool {
    matches!(pstate, PState::PowerSave)
}

fn cstate_override_requires_acpi(cstate: CState) -> bool {
    matches!(cstate, CState::C3)
}

pub fn set_pstate_override_guarded(pstate: PState) -> bool {
    let acpi_loaded = ACPI_PROFILE_LOADED.load(Ordering::Relaxed) == 1;
    if !acpi_loaded && override_requires_acpi(pstate) {
        OVERRIDE_REJECTS_NO_ACPI.fetch_add(1, Ordering::Relaxed);
        POLICY_GUARD_HITS.fetch_add(1, Ordering::Relaxed);
        return false;
    }
    PSTATE_OVERRIDE_SET_CALLS.fetch_add(1, Ordering::Relaxed);
    PSTATE_OVERRIDE.store(pstate.to_u8(), Ordering::Relaxed);
    true
}

pub fn set_pstate_override(pstate: PState) {
    let _ = set_pstate_override_guarded(pstate);
}

pub fn clear_pstate_override() {
    PSTATE_OVERRIDE_CLEAR_CALLS.fetch_add(1, Ordering::Relaxed);
    PSTATE_OVERRIDE.store(OVERRIDE_NONE, Ordering::Relaxed);
}

pub fn pstate_override() -> Option<PState> {
    let raw = PSTATE_OVERRIDE.load(Ordering::Relaxed);
    if raw <= MAX_OVERRIDE_INDEX {
        Some(PState::from_u8(raw))
    } else {
        None
    }
}

pub fn set_cstate_override_guarded(cstate: CState) -> bool {
    let acpi_loaded = ACPI_PROFILE_LOADED.load(Ordering::Relaxed) == ACPI_PROFILE_ENABLED;
    if !acpi_loaded && cstate_override_requires_acpi(cstate) {
        OVERRIDE_REJECTS_NO_ACPI.fetch_add(1, Ordering::Relaxed);
        POLICY_GUARD_HITS.fetch_add(1, Ordering::Relaxed);
        return false;
    }
    CSTATE_OVERRIDE_SET_CALLS.fetch_add(1, Ordering::Relaxed);
    CSTATE_OVERRIDE.store(cstate.to_u8(), Ordering::Relaxed);
    true
}

pub fn set_cstate_override(cstate: CState) {
    let _ = set_cstate_override_guarded(cstate);
}

pub fn clear_cstate_override() {
    CSTATE_OVERRIDE_CLEAR_CALLS.fetch_add(1, Ordering::Relaxed);
    CSTATE_OVERRIDE.store(OVERRIDE_NONE, Ordering::Relaxed);
}

pub fn cstate_override() -> Option<CState> {
    let raw = CSTATE_OVERRIDE.load(Ordering::Relaxed);
    if raw <= MAX_OVERRIDE_INDEX {
        Some(CState::from_u8(raw))
    } else {
        None
    }
}

pub fn init_from_acpi(has_fadt: bool, fadt_revision: u8) {
    if has_fadt {
        ACPI_PROFILE_LOADED.store(ACPI_PROFILE_ENABLED, Ordering::Relaxed);
        ACPI_FADT_REVISION.store(fadt_revision, Ordering::Relaxed);
    } else {
        ACPI_PROFILE_LOADED.store(ACPI_PROFILE_DISABLED, Ordering::Relaxed);
        ACPI_FADT_REVISION.store(0, Ordering::Relaxed);
    }
}

pub fn on_idle(runqueue_len: usize) -> CState {
    IDLE_CALLS.fetch_add(1, Ordering::Relaxed);
    let runqueue_len = runqueue_hint_clamped(runqueue_len);

    let cstate = choose_cstate(runqueue_len);
    match cstate {
        CState::C1 => {
            C1_ENTRIES.fetch_add(1, Ordering::Relaxed);
        }
        CState::C2 => {
            C2_ENTRIES.fetch_add(1, Ordering::Relaxed);
        }
        CState::C3 => {
            C3_ENTRIES.fetch_add(1, Ordering::Relaxed);
        }
    }

    let next_pstate = choose_pstate(runqueue_len);
    let next = next_pstate.to_u8();
    let prev = CURRENT_PSTATE.swap(next, Ordering::Relaxed);
    if prev != next {
        PSTATE_SWITCHES.fetch_add(1, Ordering::Relaxed);
        // Apply the new P-state to the hardware frequency control register.
        apply_pstate_msr(next_pstate);
    }

    cstate
}

// ── Hardware frequency scaling ────────────────────────────────────────────────

/// Write the chosen P-state into the CPU frequency control register.
///
/// x86_64  — writes Intel `IA32_PERF_CTL` (MSR 0x199) with the target ratio
///             encoded in bits 15:8.  The BIOS/firmware must have enabled EIST
///             (Enhanced Intel SpeedStep); if not, the write is silently ignored
///             by the hardware.
///
/// aarch64 — ARM frequency scaling is SoC-specific and typically handled via
///             ACPI CPPC objects or a device-tree cpufreq driver.  The stub here
///             records the intent and can be wired to a platform HAL extension.
fn apply_pstate_msr(pstate: PState) {
    #[cfg(target_arch = "x86_64")]
    {
        let ratio: u64 = match pstate {
            PState::HighPerf => PSTATE_RATIO_HIGH_PERF,
            PState::Balanced => PSTATE_RATIO_BALANCED,
            PState::PowerSave => PSTATE_RATIO_POWER_SAVE,
        };
        // IA32_PERF_CTL bits[15:8] = target ratio.
        let value: u64 = ratio << 8;
        // SAFETY: `wrmsr` is a privileged instruction that only affects the
        // current logical CPU's frequency setting register.  No memory is
        // written; the only side effect is the CPU's clock divider.
        unsafe {
            wrmsr(MSR_IA32_PERF_CTL, value);
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        // ARM CPUFREQ is platform-specific.  Record the desired level in a
        // per-CPU register if the HAL/BSP provides the hook; otherwise no-op.
        let _ = pstate; // intent recorded via CURRENT_PSTATE atomic above
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        let _ = pstate;
    }
}

/// Write a 64-bit value to an x86_64 MSR.
///
/// # Safety
/// Caller must ensure `msr` is a valid, writable MSR for the current CPU.
#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    // Safety: caller guarantees `msr` is a valid writable register on the current CPU.
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nostack, nomem),
        );
    }
}

pub fn stats() -> PowerStats {
    PowerStats {
        idle_calls: IDLE_CALLS.load(Ordering::Relaxed),
        c1_entries: C1_ENTRIES.load(Ordering::Relaxed),
        c2_entries: C2_ENTRIES.load(Ordering::Relaxed),
        c3_entries: C3_ENTRIES.load(Ordering::Relaxed),
        pstate_switches: PSTATE_SWITCHES.load(Ordering::Relaxed),
        current_pstate: PState::from_u8(CURRENT_PSTATE.load(Ordering::Relaxed)),
        policy_override_active: pstate_override().is_some(),
        policy_override_set_calls: PSTATE_OVERRIDE_SET_CALLS.load(Ordering::Relaxed),
        policy_override_clear_calls: PSTATE_OVERRIDE_CLEAR_CALLS.load(Ordering::Relaxed),
        cstate_override_active: cstate_override().is_some(),
        cstate_override_set_calls: CSTATE_OVERRIDE_SET_CALLS.load(Ordering::Relaxed),
        cstate_override_clear_calls: CSTATE_OVERRIDE_CLEAR_CALLS.load(Ordering::Relaxed),
        acpi_profile_loaded: ACPI_PROFILE_LOADED.load(Ordering::Relaxed) == ACPI_PROFILE_ENABLED,
        acpi_fadt_revision: ACPI_FADT_REVISION.load(Ordering::Relaxed),
        policy_guard_hits: POLICY_GUARD_HITS.load(Ordering::Relaxed),
        runqueue_clamp_events: RUNQUEUE_CLAMP_EVENTS.load(Ordering::Relaxed),
        failsafe_idle_fallbacks: FAILSAFE_IDLE_FALLBACKS.load(Ordering::Relaxed),
        override_rejects_no_acpi: OVERRIDE_REJECTS_NO_ACPI.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests;
