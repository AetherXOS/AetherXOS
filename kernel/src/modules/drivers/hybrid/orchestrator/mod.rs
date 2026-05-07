use alloc::vec::Vec;

use super::driverkit::DriverKitHealthSnapshot;
use super::liblinux::{
    summarize_bridge_records, LibLinuxDispatchSample,
    LibLinuxTelemetryStore, LinuxBridgeDispatchRecord, LinuxSyscallQueue,
};
use super::sidecar::{
    SideCarSaturationLevel, SideCarTelemetrySample,
    SideCarTelemetrySnapshot, SideCarTelemetryStore,
    SideCarVmConfig,
};
use crate::modules::drivers::LinuxShimDeviceKind;

pub mod planner;
pub mod types;

pub use types::*;
use crate::modules::drivers::hybrid::reactos::{NtSymbolTable, PeLoadError};
use crate::modules::drivers::hybrid::LinuxBridgeMessage;

#[derive(Debug, Clone, Copy, Default)]
pub struct HybridOrchestrator;

impl HybridOrchestratorSession {
    pub fn new(sidecar_samples_per_device: usize) -> Self {
        Self {
            sidecar_telemetry: SideCarTelemetryStore::new(sidecar_samples_per_device),
            liblinux_telemetry: LibLinuxTelemetryStore::new(64),
        }
    }

    pub fn record_sidecar_sample(
        &mut self,
        device_kind: LinuxShimDeviceKind,
        sample: SideCarTelemetrySample,
    ) {
        self.sidecar_telemetry.record(device_kind, sample);
    }

    pub fn sidecar_telemetry(&self) -> &SideCarTelemetryStore {
        &self.sidecar_telemetry
    }

    pub fn export_sidecar_telemetry_bytes(&self) -> Vec<u8> {
        self.sidecar_telemetry.snapshot().to_bytes()
    }

    pub fn import_sidecar_telemetry_bytes(
        &mut self,
        bytes: &[u8],
        max_devices: usize,
        max_samples_per_device: usize,
    ) -> bool {
        let Some(snapshot) = SideCarTelemetrySnapshot::from_bytes(bytes) else {
            return false;
        };
        self.sidecar_telemetry =
            SideCarTelemetryStore::from_snapshot(snapshot, max_devices, max_samples_per_device);
        true
    }

    pub fn sidecar_fallback_aggressiveness(&self, device_kind: LinuxShimDeviceKind) -> u8 {
        match self.sidecar_telemetry.saturation_level_for(device_kind) {
            SideCarSaturationLevel::Low => 20,
            SideCarSaturationLevel::Nominal => 40,
            SideCarSaturationLevel::High => 70,
            SideCarSaturationLevel::Critical => 90,
        }
    }

    pub fn record_liblinux_dispatch_sample(&mut self, sample: LibLinuxDispatchSample) {
        self.liblinux_telemetry.record_dispatch_sample(sample);
    }

    pub fn record_liblinux_dispatch_sample_for_syscall(
        &mut self,
        syscall: super::LinuxSyscall,
        sample: LibLinuxDispatchSample,
    ) {
        self.liblinux_telemetry
            .record_dispatch_sample_for_syscall(syscall, sample);
    }

    pub fn liblinux_telemetry(&self) -> &LibLinuxTelemetryStore {
        &self.liblinux_telemetry
    }

    pub fn dispatch_liblinux_queue_to_bridge_adaptive(
        &mut self,
        queue: &mut LinuxSyscallQueue,
        requested_max_batch: usize,
    ) -> Vec<LinuxBridgeDispatchRecord> {
        let queue_depth = queue.len();
        let first_syscall = queue.requests.first().map(|request| request.syscall);
        let batch_size = match first_syscall {
            Some(syscall) => self.liblinux_telemetry.recommended_batch_size_for_syscall(
                syscall,
                queue_depth,
                requested_max_batch,
            ),
            None => self
                .liblinux_telemetry
                .recommended_batch_size(queue_depth, requested_max_batch),
        };
        let records = HybridOrchestrator::dispatch_liblinux_queue_to_bridge(queue, batch_size);
        let (success, partial, failure) = summarize_bridge_records(&records);
        let sample = LibLinuxDispatchSample::new(queue_depth, batch_size, success, partial, failure);
        match first_syscall {
            Some(syscall) => self
                .liblinux_telemetry
                .record_dispatch_sample_for_syscall(syscall, sample),
            None => self.liblinux_telemetry.record_dispatch_sample(sample),
        }
        records
    }

    pub fn plan(
        &self,
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
    ) -> Option<HybridExecutionPlan> {
        HybridOrchestrator::plan_with_sidecar_telemetry(
            request,
            preference,
            sidecar_cfg,
            Some(&self.sidecar_telemetry),
        )
    }

    pub fn plan_with_fallbacks(
        &self,
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
    ) -> Option<HybridExecutionPlan> {
        HybridOrchestrator::plan_with_fallbacks_and_dual_telemetry(
            request,
            preference,
            sidecar_cfg,
            Some(&self.sidecar_telemetry),
            Some(&self.liblinux_telemetry),
        )
    }

    pub fn plan_with_fallbacks_with_health(
        &self,
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> Option<HybridExecutionPlan> {
        HybridOrchestrator::plan_with_fallbacks_with_full_context(
            request,
            preference,
            sidecar_cfg,
            Some(&self.sidecar_telemetry),
            Some(&self.liblinux_telemetry),
            driverkit_health,
        )
    }

    pub fn plan_with_diagnostics(
        &self,
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
    ) -> HybridPlanDiagnostics {
        HybridOrchestrator::plan_with_diagnostics_and_dual_telemetry(
            request,
            preference,
            sidecar_cfg,
            Some(&self.sidecar_telemetry),
            Some(&self.liblinux_telemetry),
        )
    }

    pub fn support_report(
        &self,
        request: &HybridRequest,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridSupportReport {
        HybridOrchestrator::support_report_with_telemetry(
            request,
            driverkit_health,
            Some(&self.sidecar_telemetry),
            Some(&self.liblinux_telemetry),
        )
    }

    pub fn runtime_assessment(
        &self,
        request: &HybridRequest,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridRuntimeAssessmentReport {
        HybridOrchestrator::runtime_assessment_with_telemetry(
            request,
            driverkit_health,
            Some(&self.sidecar_telemetry),
            Some(&self.liblinux_telemetry),
        )
    }

    pub fn coverage_audit(
        &self,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridCoverageAudit {
        HybridOrchestrator::coverage_audit_with_telemetry(
            driverkit_health,
            Some(&self.sidecar_telemetry),
            Some(&self.liblinux_telemetry),
        )
    }

    pub fn fleet_report(&self, driverkit_health: Option<DriverKitHealthSnapshot>) -> HybridFleetReport {
        HybridOrchestrator::fleet_report_with_telemetry(
            driverkit_health,
            Some(&self.sidecar_telemetry),
            Some(&self.liblinux_telemetry),
        )
    }

    pub fn readiness_report(
        &self,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridReadinessReport {
        HybridOrchestrator::readiness_report_with_telemetry(
            driverkit_health,
            Some(&self.sidecar_telemetry),
            Some(&self.liblinux_telemetry),
        )
    }

    pub fn maturity_report(
        &self,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridMaturityReport {
        HybridOrchestrator::maturity_report_with_telemetry(
            driverkit_health,
            Some(&self.sidecar_telemetry),
            Some(&self.liblinux_telemetry),
        )
    }

    pub fn release_gate_matrix(
        &self,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridReleaseGateMatrix {
        HybridOrchestrator::release_gate_matrix_with_telemetry(
            driverkit_health,
            Some(&self.sidecar_telemetry),
            Some(&self.liblinux_telemetry),
        )
    }

    pub fn userspace_abi_report(&self) -> HybridUserspaceAbiReport {
        planner::userspace_abi_report_with_telemetry(Some(&self.liblinux_telemetry))
    }

    pub fn virtualization_readiness_report(&self) -> HybridVirtualizationReadinessReport {
        HybridOrchestrator::virtualization_readiness_report()
    }

    pub fn plan_with_diagnostics_with_health(
        &self,
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridPlanDiagnostics {
        HybridOrchestrator::plan_with_diagnostics_with_full_context(
            request,
            preference,
            sidecar_cfg,
            Some(&self.sidecar_telemetry),
            Some(&self.liblinux_telemetry),
            driverkit_health,
        )
    }
}

impl HybridOrchestrator {
    pub fn plan(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
    ) -> Option<HybridExecutionPlan> {
        planner::plan(request, preference, sidecar_cfg)
    }

    pub fn plan_with_sidecar_telemetry(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        telemetry: Option<&SideCarTelemetryStore>,
    ) -> Option<HybridExecutionPlan> {
        planner::plan_with_sidecar_telemetry(request, preference, sidecar_cfg, telemetry)
    }

    pub fn plan_with_fallbacks(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
    ) -> Option<HybridExecutionPlan> {
        planner::plan_with_fallbacks(request, preference, sidecar_cfg)
    }

    pub fn plan_with_fallbacks_and_dual_telemetry(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
    ) -> Option<HybridExecutionPlan> {
        planner::plan_with_fallbacks_and_dual_telemetry(
            request,
            preference,
            sidecar_cfg,
            sidecar_telemetry,
            liblinux_telemetry,
        )
    }

    pub fn plan_with_fallbacks_with_full_context(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> Option<HybridExecutionPlan> {
        planner::plan_with_fallbacks_with_full_context(
            request,
            preference,
            sidecar_cfg,
            sidecar_telemetry,
            liblinux_telemetry,
            driverkit_health,
        )
    }

    pub fn plan_with_diagnostics(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
    ) -> HybridPlanDiagnostics {
        planner::plan_with_diagnostics(request, preference, sidecar_cfg)
    }

    pub fn plan_with_diagnostics_and_dual_telemetry(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
    ) -> HybridPlanDiagnostics {
        planner::plan_with_diagnostics_and_dual_telemetry(
            request,
            preference,
            sidecar_cfg,
            sidecar_telemetry,
            liblinux_telemetry,
        )
    }

    pub fn plan_with_diagnostics_with_full_context(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridPlanDiagnostics {
        planner::plan_with_diagnostics_with_full_context(
            request,
            preference,
            sidecar_cfg,
            sidecar_telemetry,
            liblinux_telemetry,
            driverkit_health,
        )
    }

    pub fn dispatch_liblinux_queue_to_bridge(
        queue: &mut LinuxSyscallQueue,
        max_batch: usize,
    ) -> Vec<LinuxBridgeDispatchRecord> {
        planner::dispatch_liblinux_queue_to_bridge(queue, max_batch)
    }

    pub fn support_report_with_telemetry(
        request: &HybridRequest,
        driverkit_health: Option<DriverKitHealthSnapshot>,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
    ) -> HybridSupportReport {
        planner::support_report_with_telemetry(
            request,
            driverkit_health,
            sidecar_telemetry,
            liblinux_telemetry,
        )
    }

    pub fn runtime_assessment_with_telemetry(
        request: &HybridRequest,
        driverkit_health: Option<DriverKitHealthSnapshot>,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
    ) -> HybridRuntimeAssessmentReport {
        planner::runtime_assessment_with_telemetry(
            request,
            driverkit_health,
            sidecar_telemetry,
            liblinux_telemetry,
        )
    }

    pub fn coverage_audit_with_telemetry(
        driverkit_health: Option<DriverKitHealthSnapshot>,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
    ) -> HybridCoverageAudit {
        planner::coverage_audit_with_telemetry(
            driverkit_health,
            sidecar_telemetry,
            liblinux_telemetry,
        )
    }

    pub fn readiness_report_with_telemetry(
        driverkit_health: Option<DriverKitHealthSnapshot>,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
    ) -> HybridReadinessReport {
        planner::readiness_report_with_telemetry(
            driverkit_health,
            sidecar_telemetry,
            liblinux_telemetry,
        )
    }

    pub fn fleet_report_with_telemetry(
        driverkit_health: Option<DriverKitHealthSnapshot>,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
    ) -> HybridFleetReport {
        planner::fleet_report_with_telemetry(driverkit_health, sidecar_telemetry, liblinux_telemetry)
    }

    pub fn maturity_report_with_telemetry(
        driverkit_health: Option<DriverKitHealthSnapshot>,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
    ) -> HybridMaturityReport {
        planner::maturity_report_with_telemetry(
            driverkit_health,
            sidecar_telemetry,
            liblinux_telemetry,
        )
    }

    pub fn release_gate_matrix_with_telemetry(
        driverkit_health: Option<DriverKitHealthSnapshot>,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
    ) -> HybridReleaseGateMatrix {
        planner::release_gate_matrix_with_telemetry(
            driverkit_health,
            sidecar_telemetry,
            liblinux_telemetry,
        )
    }

    pub fn virtualization_readiness_report() -> HybridVirtualizationReadinessReport {
        planner::virtualization_readiness_report()
    }

    pub fn plan_windows_pe(image: &[u8], preference: BackendPreference) -> Result<HybridExecutionPlan, PeLoadError> {
        planner::plan_windows_pe(image, preference)
    }

    pub fn plan_windows_pe_with_symbols(image: &[u8], symbols: &NtSymbolTable) -> Result<ReactOsImportResolution, PeLoadError> {
        planner::plan_windows_pe_with_symbols(image, symbols)
    }

    pub fn build_sidecar_bootstrap_frames(
        request: &HybridRequest,
        cfg: SideCarVmConfig,
        request_id_start: u64,
    ) -> Result<Vec<SideCarWireFrame>, &'static str> {
        planner::build_sidecar_bootstrap_frames(request, cfg, request_id_start).ok_or("no plan")
    }

    pub fn submit_sidecar_frames<T: crate::modules::drivers::hybrid::sidecar::SideCarTransport<Error = crate::modules::drivers::hybrid::sidecar::SideCarWireError>>(
        transport: &mut T,
        frames: &[SideCarWireFrame],
    ) -> Result<(), crate::modules::drivers::hybrid::sidecar::SideCarWireError> {
        planner::submit_sidecar_frames(transport, frames)
    }

    pub fn drive_sidecar_bootstrap<T: crate::modules::drivers::hybrid::sidecar::SideCarTransport<Error = crate::modules::drivers::hybrid::sidecar::SideCarWireError>>(
        transport: &mut T,
        state: &mut crate::modules::drivers::SideCarBootstrapState,
        vm_id: u16,
        control_ring_depth: usize,
        current_tick: u32,
    ) -> Result<bool, crate::modules::drivers::hybrid::sidecar::SideCarWireError> {
        planner::drive_sidecar_bootstrap(transport, state, vm_id, control_ring_depth, current_tick)
    }

    pub fn advance_sidecar_bootstrap_from_bridge_message(
        state: &mut crate::modules::drivers::SideCarBootstrapState,
        message: &LinuxBridgeMessage,
        tick: u32,
    ) -> bool {
        planner::advance_sidecar_bootstrap_from_bridge_message(state, message, tick)
    }

    pub fn coverage_audit(driverkit_health: Option<DriverKitHealthSnapshot>) -> HybridCoverageAudit {
        planner::coverage_audit_with_telemetry(driverkit_health, None, None)
    }

    pub fn feature_audit(driverkit_health: Option<DriverKitHealthSnapshot>) -> HybridFeatureAudit {
        planner::feature_audit(driverkit_health)
    }

    pub fn support_report(
        request: &HybridRequest,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridSupportReport {
        planner::support_report_with_telemetry(request, driverkit_health, None, None)
    }

    pub fn fleet_report(driverkit_health: Option<DriverKitHealthSnapshot>) -> HybridFleetReport {
        planner::fleet_report_with_telemetry(driverkit_health, None, None)
    }

    pub fn plan_with_diagnostics_and_telemetry(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
    ) -> HybridPlanDiagnostics {
        planner::plan_with_diagnostics_and_dual_telemetry(
            request,
            preference,
            sidecar_cfg,
            sidecar_telemetry,
            liblinux_telemetry,
        )
    }

    pub fn plan_with_driverkit_health(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> Option<HybridExecutionPlan> {
        planner::plan_with_fallbacks_with_full_context(
            request,
            preference,
            sidecar_cfg,
            None,
            None,
            driverkit_health,
        )
    }

    pub fn liblinux_conformance_report(requests: &[crate::modules::drivers::hybrid::liblinux::LinuxSyscallRequest]) -> crate::modules::drivers::hybrid::liblinux::LibLinuxConformanceReport {
        crate::modules::drivers::hybrid::liblinux::conformance_report_for_requests(requests)
    }

    pub fn runtime_assessment(
        request: &HybridRequest,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridRuntimeAssessmentReport {
        planner::runtime_assessment_with_telemetry(request, driverkit_health, None, None)
    }

    pub fn userspace_abi_report() -> HybridUserspaceAbiReport {
        planner::userspace_abi_report_with_telemetry(None)
    }
}

#[cfg(test)]
mod tests;
