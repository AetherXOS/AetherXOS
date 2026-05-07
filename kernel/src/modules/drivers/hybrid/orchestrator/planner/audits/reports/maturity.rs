use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;
use crate::modules::drivers::hybrid::orchestrator::HybridMaturityReport;
use crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore;
use crate::modules::drivers::hybrid::sidecar::telemetry::SideCarTelemetryStore;

pub fn maturity_report(_d: Option<DriverKitHealthSnapshot>) -> HybridMaturityReport {
    HybridMaturityReport {
        findings: alloc::vec::Vec::new(),
        overall_score: 0,
        production_ready: false,
    }
}

pub fn maturity_report_with_telemetry(
    _d: Option<DriverKitHealthSnapshot>,
    _s: Option<&SideCarTelemetryStore>,
    _l: Option<&LibLinuxTelemetryStore>,
) -> HybridMaturityReport {
    HybridMaturityReport {
        findings: alloc::vec::Vec::new(),
        overall_score: 0,
        production_ready: false,
    }
}
