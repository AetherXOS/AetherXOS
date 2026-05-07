use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;
use crate::modules::drivers::hybrid::orchestrator::{HybridReadinessReport, HybridReleaseGateMatrix};
use crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore;
use crate::modules::drivers::hybrid::sidecar::telemetry::SideCarTelemetryStore;
use super::{coverage, userspace_abi, virtualization};

pub fn readiness_report(_d: Option<DriverKitHealthSnapshot>) -> HybridReadinessReport {
    HybridReadinessReport {
        coverage: coverage::coverage_audit(None),
        userspace_abi: userspace_abi::userspace_abi_report(),
        virtualization: virtualization::virtualization_readiness_report(),
        gaps: alloc::vec::Vec::new(),
        release_ready: false,
    }
}

pub fn readiness_report_with_telemetry(
    _d: Option<DriverKitHealthSnapshot>,
    _s: Option<&SideCarTelemetryStore>,
    _l: Option<&LibLinuxTelemetryStore>,
) -> HybridReadinessReport {
    HybridReadinessReport {
        coverage: coverage::coverage_audit(None),
        userspace_abi: userspace_abi::userspace_abi_report(),
        virtualization: virtualization::virtualization_readiness_report(),
        gaps: alloc::vec::Vec::new(),
        release_ready: false,
    }
}

pub fn release_gate_matrix(_d: Option<DriverKitHealthSnapshot>) -> HybridReleaseGateMatrix {
    HybridReleaseGateMatrix {
        version: "",
        rows: alloc::vec::Vec::new(),
        system_rows: alloc::vec::Vec::new(),
        release_blocked: false,
    }
}

pub fn release_gate_matrix_with_telemetry(
    _d: Option<DriverKitHealthSnapshot>,
    _s: Option<&SideCarTelemetryStore>,
    _l: Option<&LibLinuxTelemetryStore>,
) -> HybridReleaseGateMatrix {
    HybridReleaseGateMatrix {
        version: "",
        rows: alloc::vec::Vec::new(),
        system_rows: alloc::vec::Vec::new(),
        release_blocked: false,
    }
}
