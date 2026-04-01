use super::*;

pub(super) static ACTIVE_PRESET_RAW: AtomicU64 =
    AtomicU64::new(preset_to_raw(CoreRuntimePolicyPreset::Server));
pub(super) static PRESET_SET_CALLS: AtomicU64 = AtomicU64::new(0);
pub(super) static PRESET_APPLY_CALLS: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
pub(super) const fn preset_to_raw(preset: CoreRuntimePolicyPreset) -> u64 {
    match preset {
        CoreRuntimePolicyPreset::Interactive => 0,
        CoreRuntimePolicyPreset::Server => 1,
        CoreRuntimePolicyPreset::Realtime => 2,
    }
}

#[inline(always)]
pub(super) const fn raw_to_preset(raw: u64) -> CoreRuntimePolicyPreset {
    match raw {
        0 => CoreRuntimePolicyPreset::Interactive,
        2 => CoreRuntimePolicyPreset::Realtime,
        _ => CoreRuntimePolicyPreset::Server,
    }
}

pub fn set_runtime_policy_preset(preset: CoreRuntimePolicyPreset) {
    PRESET_SET_CALLS.fetch_add(1, Ordering::Relaxed);
    ACTIVE_PRESET_RAW.store(preset_to_raw(preset), Ordering::Relaxed);
}

pub fn runtime_policy_preset() -> CoreRuntimePolicyPreset {
    raw_to_preset(ACTIVE_PRESET_RAW.load(Ordering::Relaxed))
}
