use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;
use crate::modules::drivers::hybrid::orchestrator::HybridCoverageAudit;
use crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore;
use crate::modules::drivers::hybrid::sidecar::telemetry::SideCarTelemetryStore;

pub fn coverage_audit(_d: Option<DriverKitHealthSnapshot>) -> HybridCoverageAudit {
    HybridCoverageAudit {
        rows: alloc::vec::Vec::new(),
        overall_score: 0,
        all_requests_supported: false,
    }
}

pub fn coverage_audit_with_telemetry(
    _d: Option<DriverKitHealthSnapshot>,
    _s: Option<&SideCarTelemetryStore>,
    _l: Option<&LibLinuxTelemetryStore>,
) -> HybridCoverageAudit {
    HybridCoverageAudit {
        rows: alloc::vec::Vec::new(),
        overall_score: 0,
        all_requests_supported: false,
    }
}
