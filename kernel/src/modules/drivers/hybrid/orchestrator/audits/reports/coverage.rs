use crate::modules::drivers::hybrid::orchestrator::audits::models::{DriverKitHealthSnapshot, HybridCoverageAudit};
use crate::modules::drivers::hybrid::orchestrator::telemetry::LibLinuxTelemetryStore;
use crate::modules::drivers::hybrid::sidecar::telemetry::SideCarTelemetryStore;

pub fn coverage_audit(_d: Option<DriverKitHealthSnapshot>) -> HybridCoverageAudit { unimplemented!() }
pub fn coverage_audit_with_telemetry(
    _d: Option<DriverKitHealthSnapshot>,
    _s: Option<&SideCarTelemetryStore>,
    _l: Option<&LibLinuxTelemetryStore>,
