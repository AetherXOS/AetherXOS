use super::*;

pub(super) fn evaluate_drift(
    preset: CoreRuntimePolicyPreset,
    pressure: crate::kernel::pressure::CorePressureSnapshot,
    network_breaches: u8,
    vfs_breaches: u8,
    driver_wait_delta: u64,
) -> (bool, DriftReasonCode) {
    let latency_bias = current_virtualization_runtime_governor().latency_bias;
    let thresholds = drift_threshold_profile(preset, latency_bias);
    match preset {
        CoreRuntimePolicyPreset::Interactive => {
            if pressure_exceeds_threshold(pressure.class, thresholds.pressure_class_threshold) {
                return (true, DriftReasonCode::PressureHigh);
            }
            if network_breaches > thresholds.network_breach_limit {
                return (true, DriftReasonCode::NetworkSlo);
            }
            if vfs_breaches > thresholds.vfs_breach_limit {
                return (true, DriftReasonCode::VfsSlo);
            }
            if driver_wait_delta > thresholds.driver_wait_limit {
                return (true, DriftReasonCode::DriverWaitTimeout);
            }
            (false, DriftReasonCode::None)
        }
        CoreRuntimePolicyPreset::Server => {
            if pressure.rt_starvation_alert {
                return (true, DriftReasonCode::RtStarvation);
            }
            if pressure_exceeds_threshold(pressure.class, thresholds.pressure_class_threshold) {
                return (true, DriftReasonCode::PressureHigh);
            }
            if driver_wait_delta > thresholds.driver_wait_limit {
                return (true, DriftReasonCode::DriverWaitTimeout);
            }
            (false, DriftReasonCode::None)
        }
        CoreRuntimePolicyPreset::Realtime => {
            if pressure.rt_starvation_alert {
                return (true, DriftReasonCode::RtStarvation);
            }
            if network_breaches > thresholds.network_breach_limit {
                return (true, DriftReasonCode::NetworkSlo);
            }
            if vfs_breaches > thresholds.vfs_breach_limit {
                return (true, DriftReasonCode::VfsSlo);
            }
            if driver_wait_delta > thresholds.driver_wait_limit {
                return (true, DriftReasonCode::DriverWaitTimeout);
            }
            (false, DriftReasonCode::None)
        }
    }
}
