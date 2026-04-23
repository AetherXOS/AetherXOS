use crate::modules::drivers::hybrid::orchestrator::audits::models::{DriverKitHealthSnapshot, HybridMaturityReport};
use crate::modules::drivers::hybrid::orchestrator::telemetry::LibLinuxTelemetryStore;
use crate::modules::drivers::hybrid::sidecar::telemetry::SideCarTelemetryStore;

pub fn maturity_report(_d: Option<DriverKitHealthSnapshot>) -> HybridMaturityReport { unimplemented!() }
pub fn maturity_report_with_telemetry(
    _d: Option<DriverKitHealthSnapshot>,
    _s: Option<&SideCarTelemetryStore>,
    _l: Option<&LibLinuxTelemetryStore>,
