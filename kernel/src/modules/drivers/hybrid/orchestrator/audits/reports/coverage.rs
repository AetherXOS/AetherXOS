use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;
use crate::modules::drivers::hybrid::orchestrator::HybridCoverageAudit;
use crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore;
use crate::modules::drivers::hybrid::sidecar::telemetry::SideCarTelemetryStore;

pub fn coverage_audit(_d: Option<DriverKitHealthSnapshot>) -> HybridCoverageAudit { unimplemented!() }
pub fn coverage_audit_with_telemetry(
    _d: Option<DriverKitHealthSnapshot>,
    _s: Option<&SideCarTelemetryStore>,
    _l: Option<&LibLinuxTelemetryStore>,
) -> HybridCoverageAudit { unimplemented!() }
