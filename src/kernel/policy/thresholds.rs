use super::*;

#[inline(always)]
pub(crate) fn governor_adjusted_drift_limit(base: u8, latency_bias: &'static str) -> u8 {
    match latency_bias {
        GOVERNOR_BIAS_AGGRESSIVE => base.saturating_sub(1),
        GOVERNOR_BIAS_RELAXED => base.saturating_add(1),
        _ => base,
    }
}

#[inline(always)]
pub(crate) fn governor_adjusted_driver_wait_limit(base: u64, latency_bias: &'static str) -> u64 {
    match latency_bias {
        GOVERNOR_BIAS_AGGRESSIVE => base.saturating_sub(1),
        GOVERNOR_BIAS_RELAXED => base.saturating_add(1),
        _ => base,
    }
}

#[inline(always)]
pub(crate) fn governor_adjusted_pressure_threshold(
    latency_bias: &'static str,
) -> crate::kernel::pressure::CorePressureClass {
    match latency_bias {
        GOVERNOR_BIAS_AGGRESSIVE => crate::kernel::pressure::CorePressureClass::Elevated,
        _ => crate::kernel::pressure::CorePressureClass::High,
    }
}

#[inline(always)]
pub(crate) fn pressure_exceeds_threshold(
    pressure_class: crate::kernel::pressure::CorePressureClass,
    threshold: crate::kernel::pressure::CorePressureClass,
) -> bool {
    pressure_class_rank(pressure_class) >= pressure_class_rank(threshold)
}

#[inline(always)]
fn pressure_class_rank(class: crate::kernel::pressure::CorePressureClass) -> u8 {
    match class {
        crate::kernel::pressure::CorePressureClass::Nominal => 0,
        crate::kernel::pressure::CorePressureClass::Elevated => 1,
        crate::kernel::pressure::CorePressureClass::High => 2,
        crate::kernel::pressure::CorePressureClass::Critical => 3,
    }
}

#[inline(always)]
pub(crate) fn drift_threshold_profile(
    preset: CoreRuntimePolicyPreset,
    latency_bias: &'static str,
) -> DriftThresholdProfile {
    let mut profile = match preset {
        CoreRuntimePolicyPreset::Interactive => DriftThresholdProfile {
            pressure_class_threshold: crate::kernel::pressure::CorePressureClass::High,
            network_breach_limit: 2,
            vfs_breach_limit: 2,
            driver_wait_limit: 1,
        },
        CoreRuntimePolicyPreset::Server => DriftThresholdProfile {
            pressure_class_threshold: crate::kernel::pressure::CorePressureClass::Critical,
            network_breach_limit: 3,
            vfs_breach_limit: 3,
            driver_wait_limit: 2,
        },
        CoreRuntimePolicyPreset::Realtime => DriftThresholdProfile {
            pressure_class_threshold: crate::kernel::pressure::CorePressureClass::High,
            network_breach_limit: 1,
            vfs_breach_limit: 1,
            driver_wait_limit: 1,
        },
    };
    profile.network_breach_limit =
        governor_adjusted_drift_limit(profile.network_breach_limit, latency_bias);
    profile.vfs_breach_limit =
        governor_adjusted_drift_limit(profile.vfs_breach_limit, latency_bias);
    profile.driver_wait_limit =
        governor_adjusted_driver_wait_limit(profile.driver_wait_limit, latency_bias);
    profile.pressure_class_threshold = governor_adjusted_pressure_threshold(latency_bias);
    profile
}
