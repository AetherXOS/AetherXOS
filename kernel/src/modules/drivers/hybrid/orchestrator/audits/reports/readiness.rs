use crate::modules::drivers::hybrid::orchestrator::audits::models::{DriverKitHealthSnapshot, HybridReadinessReport, HybridReleaseGateMatrix};
use crate::modules::drivers::hybrid::orchestrator::telemetry::LibLinuxTelemetryStore;
use crate::modules::drivers::hybrid::sidecar::telemetry::SideCarTelemetryStore;

pub fn readiness_report(_d: Option<DriverKitHealthSnapshot>) -> HybridReadinessReport { unimplemented!() }
pub fn readiness_report_with_telemetry(
    _d: Option<DriverKitHealthSnapshot>,
    _s: Option<&SideCarTelemetryStore>,
    _l: Option<&LibLinuxTelemetryStore>,
) -> HybridReadinessReport { unimplemented!() }
pub fn release_gate_matrix(_d: Option<DriverKitHealthSnapshot>) -> HybridReleaseGateMatrix { unimplemented!() }
pub fn release_gate_matrix_with_telemetry(
    _d: Option<DriverKitHealthSnapshot>,
    _s: Option<&SideCarTelemetryStore>,
    _l: Option<&LibLinuxTelemetryStore>,
