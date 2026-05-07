use super::super::super::liblinux::LibLinuxTelemetryStore;
use super::super::super::sidecar::SideCarTelemetryStore;

#[derive(Debug, Clone)]
pub struct HybridOrchestratorSession {
    pub sidecar_telemetry: SideCarTelemetryStore,
    pub liblinux_telemetry: LibLinuxTelemetryStore,
}
