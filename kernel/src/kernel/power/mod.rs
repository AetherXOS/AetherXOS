use crate::hal::common::virt::current_virtualization_power_tuning;
#[cfg(test)]
use crate::hal::common::virt::{virtualization_power_tuning, VirtualizationPowerTuning};
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

// Ratios moved to HAL

const OVERRIDE_NONE: u8 = 0xFF;
const MAX_OVERRIDE_INDEX: u8 = 2;
const ACPI_PROFILE_DISABLED: u8 = 0;
const ACPI_PROFILE_ENABLED: u8 = 1;

define_enum!(pub enum CState : u8 {
    C1 = 0,
    C2 = 1,
    C3 = 2,
});

define_enum!(pub enum PState : u8 {
    HighPerf = 0,
    Balanced = 1,
    PowerSave = 2,
});

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
        return CState::from_raw(override_raw).unwrap_or(CState::C1);
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
        return PState::from_raw(override_raw).unwrap_or(PState::Balanced);
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
    PSTATE_OVERRIDE.store(pstate.to_raw(), Ordering::Relaxed);
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
        PState::from_raw(raw)
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
    CSTATE_OVERRIDE.store(cstate.to_raw(), Ordering::Relaxed);
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
        CState::from_raw(raw)
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
    let next = next_pstate.to_raw();
    let prev = CURRENT_PSTATE.swap(next, Ordering::Relaxed);
    if prev != next {
        PSTATE_SWITCHES.fetch_add(1, Ordering::Relaxed);
        
        use crate::interfaces::{HardwareAbstraction, PerformanceProfile};
        let profile = match next_pstate {
            PState::HighPerf => PerformanceProfile::HighPerformance,
            PState::Balanced => PerformanceProfile::Balanced,
            PState::PowerSave => PerformanceProfile::PowerSaving,
        };
        crate::hal::HAL::set_performance_profile(profile);
    }

    cstate
}

// Hardware frequency scaling delegated to HAL

pub fn stats() -> PowerStats {
    PowerStats {
        idle_calls: IDLE_CALLS.load(Ordering::Relaxed),
        c1_entries: C1_ENTRIES.load(Ordering::Relaxed),
        c2_entries: C2_ENTRIES.load(Ordering::Relaxed),
        c3_entries: C3_ENTRIES.load(Ordering::Relaxed),
        pstate_switches: PSTATE_SWITCHES.load(Ordering::Relaxed),
        current_pstate: PState::from_raw(CURRENT_PSTATE.load(Ordering::Relaxed)).unwrap_or(PState::Balanced),
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
