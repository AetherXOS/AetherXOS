use alloc::vec::Vec;

use super::driverkit::{DriverKitHealthSnapshot, UserModeDriverContext};
use super::liblinux::{
    summarize_bridge_records, LibLinuxConformanceReport, LibLinuxDispatchSample,
    LibLinuxTelemetryStore, LinuxBridgeDispatchRecord, LinuxSyscallQueue, LinuxSyscallRequest,
};
use super::linux::{LinuxResourcePlan, LinuxShimDeviceKind};
use super::reactos::{
    NtDomainImportBinding, NtExecutionPolicy, NtImportBinding, NtImportDomainCounts,
    NtSymbolTable, PeImageInfo, PeLoadError,
};
use super::sidecar::{
    SideCarBootstrapState, SideCarPayload, SideCarTelemetrySample, SideCarTelemetryStore,
    SideCarTransport,
    SideCarVmConfig, SideCarVmPlan, SideCarWireHeader,
};

pub mod planner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendPreference {
    SideCarFirst,
    LibLinuxFirst,
    ReactOsFirst,
    DriverKitFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridRequestKind {
    Network,
    Block,
    Ethernet,
    Storage,
    Modem,
    Printer,
    Rtc,
    SensorHub,
    Gpu,
    WiFi,
    Camera,
    Audio,
    Sensor,
    Input,
    Touch,
    Gamepad,
    Bluetooth,
    Nfc,
    Tpm,
    Dock,
    Display,
    Usb,
    Can,
    Serial,
    Firmware,
    SmartCard,
    Nvme,
    WindowsPe,
    UserModeDevice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridRequestFamily {
    Network,
    Storage,
    Multimedia,
    Input,
    Security,
    Platform,
    Compatibility,
    Peripheral,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridRequest {
    pub kind: HybridRequestKind,
    pub mmio_base: usize,
    pub mmio_length: usize,
    pub iova_base: usize,
    pub iova_length: usize,
    pub irq_vector: u32,
}

impl HybridRequest {
    const fn from_parts(
        kind: HybridRequestKind,
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self {
            kind,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        }
    }

    const fn zeroed(kind: HybridRequestKind) -> Self {
        Self::from_parts(kind, 0, 0, 0, 0, 0)
    }

    pub const fn device(
        kind: HybridRequestKind,
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::from_parts(kind, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn network(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Network, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn block(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Block, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn ethernet(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::Ethernet,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn storage(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::Storage,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn modem(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::Modem,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn printer(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::Printer,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn rtc(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Rtc, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn sensor_hub(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::SensorHub,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn windows_pe() -> Self {
        Self::zeroed(HybridRequestKind::WindowsPe)
    }

    pub const fn gpu(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Gpu, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn wifi(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::WiFi, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn user_mode_device(mmio_base: usize, mmio_length: usize, irq_vector: u32) -> Self {
        Self::from_parts(
            HybridRequestKind::UserModeDevice,
            mmio_base,
            mmio_length,
            0,
            0,
            irq_vector,
        )
    }

    pub const fn camera(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Camera, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn audio(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Audio, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn sensor(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Sensor, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn input(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Input, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn touch(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Touch, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn gamepad(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Gamepad, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn bluetooth(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Bluetooth, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn nfc(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Nfc, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn tpm(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Tpm, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn dock(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Dock, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn display(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Display, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn usb(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Usb, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn can(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Can, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn serial(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Serial, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }

    pub const fn firmware(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::Firmware,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn smart_card(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(
            HybridRequestKind::SmartCard,
            mmio_base,
            mmio_length,
            iova_base,
            iova_length,
            irq_vector,
        )
    }

    pub const fn nvme(
        mmio_base: usize,
        mmio_length: usize,
        iova_base: usize,
        iova_length: usize,
        irq_vector: u32,
    ) -> Self {
        Self::device(HybridRequestKind::Nvme, mmio_base, mmio_length, iova_base, iova_length, irq_vector)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HybridExecutionPlan {
    SideCar(SideCarVmPlan),
    LibLinux(LinuxResourcePlan),
    ReactOs {
        policy: NtExecutionPolicy,
        image_info: PeImageInfo,
    },
    DriverKit(UserModeDriverContext),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactOsImportResolution {
    pub image_info: PeImageInfo,
    pub bindings: Vec<NtImportBinding>,
    pub domain_bindings: Vec<NtDomainImportBinding>,
    pub counts: NtImportDomainCounts,
    pub policy: NtExecutionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideCarWireFrame {
    pub header: SideCarWireHeader,
    pub payload: SideCarPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridPlanAttempt {
    pub backend: BackendPreference,
    pub matched: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridPlanDiagnostics {
    pub attempts: Vec<HybridPlanAttempt>,
    pub selected: Option<HybridExecutionPlan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridBackendSupport {
    pub backend: BackendPreference,
    pub score: u8,
    pub supported: bool,
    pub degraded: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridSupportReport {
    pub request_kind: HybridRequestKind,
    pub entries: Vec<HybridBackendSupport>,
    pub recommended: BackendPreference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridPerformanceTier {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridSecurityPosture {
    Isolated,
    Mediated,
    CompatibilityRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridRuntimeConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridBackendAssessment {
    pub backend: BackendPreference,
    pub supported: bool,
    pub confidence: HybridRuntimeConfidence,
    pub performance: HybridPerformanceTier,
    pub security: HybridSecurityPosture,
    pub risk: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridRuntimeAssessmentReport {
    pub request_kind: HybridRequestKind,
    pub recommended: BackendPreference,
    pub assessments: Vec<HybridBackendAssessment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridBackendFleetStatus {
    pub backend: BackendPreference,
    pub coverage_score: u8,
    pub performance_score: u8,
    pub security_score: u8,
    pub supported_request_kinds: usize,
    pub unsupported_request_kinds: usize,
    pub high_risk_paths: usize,
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridFamilyFleetStatus {
    pub family: HybridRequestFamily,
    pub coverage_score: u8,
    pub performance_score: u8,
    pub security_score: u8,
    pub supported_request_kinds: usize,
    pub unsupported_request_kinds: usize,
    pub high_risk_paths: usize,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridFleetReport {
    pub backends: Vec<HybridBackendFleetStatus>,
    pub families: Vec<HybridFamilyFleetStatus>,
    pub most_ready_backend: BackendPreference,
    pub least_ready_backend: BackendPreference,
    pub overall_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridMaturityDimension {
    TelemetryCoverage,
    TailLatency,
    ThreatModelCoverage,
    CertificationMatrix,
    FailoverConsistency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridMaturityFinding {
    pub dimension: HybridMaturityDimension,
    pub score: u8,
    pub gap: HybridGapSeverity,
    pub summary: &'static str,
    pub remediation: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridMaturityReport {
    pub findings: Vec<HybridMaturityFinding>,
    pub overall_score: u8,
    pub production_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridCoverageRow {
    pub request_kind: HybridRequestKind,
    pub supported_backends: Vec<BackendPreference>,
    pub recommended: BackendPreference,
    pub coverage_score: u8,
    pub has_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridCoverageAudit {
    pub rows: Vec<HybridCoverageRow>,
    pub overall_score: u8,
    pub all_requests_supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridFeatureKind {
    Mmio,
    Dma,
    Irq,
    SharedMemory,
    ControlQueue,
    Reset,
    Hotplug,
    PowerManagement,
    Snapshot,
    LiveMigration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridFeatureRow {
    pub request_kind: HybridRequestKind,
    pub backend: BackendPreference,
    pub supported_features: Vec<HybridFeatureKind>,
    pub missing_features: Vec<HybridFeatureKind>,
    pub feature_score: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridFeatureAudit {
    pub rows: Vec<HybridFeatureRow>,
    pub overall_feature_score: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridGapSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridReadinessGap {
    pub request_kind: Option<HybridRequestKind>,
    pub severity: HybridGapSeverity,
    pub issue: &'static str,
    pub remediation: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridReadinessReport {
    pub coverage: HybridCoverageAudit,
    pub gaps: Vec<HybridReadinessGap>,
    pub release_ready: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HybridOrchestrator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridOrchestratorSession {
    sidecar_telemetry: SideCarTelemetryStore,
    liblinux_telemetry: LibLinuxTelemetryStore,
}

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

    pub fn record_liblinux_dispatch_sample(&mut self, sample: LibLinuxDispatchSample) {
        self.liblinux_telemetry.record_dispatch_sample(sample);
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
        let batch_size = self
            .liblinux_telemetry
            .recommended_batch_size(queue_depth, requested_max_batch);
        let records = HybridOrchestrator::dispatch_liblinux_queue_to_bridge(queue, batch_size);
        let (success, partial, failure) = summarize_bridge_records(&records);
        self.liblinux_telemetry
            .record_dispatch_sample(LibLinuxDispatchSample::new(
                queue_depth,
                batch_size,
                success,
                partial,
                failure,
            ));
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
        HybridOrchestrator::plan_with_fallbacks_and_telemetry(
            request,
            preference,
            sidecar_cfg,
            Some(&self.sidecar_telemetry),
        )
    }

    pub fn plan_with_diagnostics(
        &self,
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
    ) -> HybridPlanDiagnostics {
        HybridOrchestrator::plan_with_diagnostics_and_telemetry(
            request,
            preference,
            sidecar_cfg,
            Some(&self.sidecar_telemetry),
        )
    }
}

impl HybridOrchestrator {
    fn adapt_preference_with_driverkit_health(
        preference: BackendPreference,
        health: DriverKitHealthSnapshot,
    ) -> BackendPreference {
        planner::adapt_preference_with_driverkit_health(preference, health)
    }

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

    pub fn plan_with_driverkit_health(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        health: DriverKitHealthSnapshot,
    ) -> Option<HybridExecutionPlan> {
        let effective = Self::adapt_preference_with_driverkit_health(preference, health);
        Self::plan(request, effective, sidecar_cfg)
    }

    pub fn plan_with_fallbacks(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
    ) -> Option<HybridExecutionPlan> {
        planner::plan_with_fallbacks(request, preference, sidecar_cfg)
    }

    pub fn plan_with_fallbacks_and_telemetry(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        telemetry: Option<&SideCarTelemetryStore>,
    ) -> Option<HybridExecutionPlan> {
        planner::plan_with_fallbacks_and_telemetry(request, preference, sidecar_cfg, telemetry)
    }

    pub fn plan_with_diagnostics(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
    ) -> HybridPlanDiagnostics {
        planner::plan_with_diagnostics(request, preference, sidecar_cfg)
    }

    pub fn plan_with_diagnostics_and_telemetry(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        telemetry: Option<&SideCarTelemetryStore>,
    ) -> HybridPlanDiagnostics {
        planner::plan_with_diagnostics_and_telemetry(request, preference, sidecar_cfg, telemetry)
    }

    pub fn plan_windows_pe(
        image: &[u8],
        preference: BackendPreference,
    ) -> Result<HybridExecutionPlan, PeLoadError> {
        planner::plan_windows_pe(image, preference)
    }

    pub fn plan_windows_pe_with_symbols(
        image: &[u8],
        symbols: &NtSymbolTable,
    ) -> Result<ReactOsImportResolution, PeLoadError> {
        planner::plan_windows_pe_with_symbols(image, symbols)
    }

    pub fn build_sidecar_bootstrap_frames(
        request: &HybridRequest,
        sidecar_cfg: SideCarVmConfig,
        request_id_seed: u64,
    ) -> Option<Vec<SideCarWireFrame>> {
        planner::build_sidecar_bootstrap_frames(request, sidecar_cfg, request_id_seed)
    }

    pub fn submit_sidecar_frames<T: SideCarTransport>(
        transport: &mut T,
        frames: &[SideCarWireFrame],
    ) -> Result<(), T::Error> {
        planner::submit_sidecar_frames(transport, frames)
    }

    pub fn drive_sidecar_bootstrap<T: SideCarTransport>(
        transport: &mut T,
        state: &mut SideCarBootstrapState,
        vm_id: u16,
        control_ring_depth: usize,
        current_tick: u32,
    ) -> Result<bool, T::Error> {
        planner::drive_sidecar_bootstrap(
            transport,
            state,
            vm_id,
            control_ring_depth,
            current_tick,
        )
    }

    pub fn advance_sidecar_bootstrap_from_bridge_message(
        state: &mut SideCarBootstrapState,
        message: &super::LinuxBridgeMessage,
        current_tick: u32,
    ) -> bool {
        planner::advance_sidecar_bootstrap_from_bridge_message(state, message, current_tick)
    }

    pub fn dispatch_liblinux_queue_to_bridge(
        queue: &mut LinuxSyscallQueue,
        max_batch: usize,
    ) -> Vec<LinuxBridgeDispatchRecord> {
        planner::dispatch_liblinux_queue_to_bridge(queue, max_batch)
    }

    pub fn map_liblinux_syscall(request: &LinuxSyscallRequest) -> super::LinuxIoRequest {
        planner::map_liblinux_syscall(request)
    }

    pub fn liblinux_conformance_report(
        requests: &[LinuxSyscallRequest],
    ) -> LibLinuxConformanceReport {
        super::liblinux::conformance_report_for_requests(requests)
    }

    pub fn support_report(
        request: &HybridRequest,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridSupportReport {
        planner::support_report(request, driverkit_health)
    }

    pub fn runtime_assessment(
        request: &HybridRequest,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridRuntimeAssessmentReport {
        planner::runtime_assessment(request, driverkit_health)
    }

    pub fn coverage_audit(
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridCoverageAudit {
        planner::coverage_audit(driverkit_health)
    }

    pub fn feature_audit(
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridFeatureAudit {
        planner::feature_audit(driverkit_health)
    }

    pub fn readiness_report(
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridReadinessReport {
        planner::readiness_report(driverkit_health)
    }

    pub fn fleet_report(
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridFleetReport {
        planner::fleet_report(driverkit_health)
    }

    pub fn maturity_report(
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridMaturityReport {
        planner::maturity_report(driverkit_health)
    }
}

#[cfg(test)]
mod tests;
