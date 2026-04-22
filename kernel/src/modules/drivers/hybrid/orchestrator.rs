use alloc::vec::Vec;

use super::driverkit::{DriverKitHealthSnapshot, UserModeDriverContext};
use super::liblinux::{
    summarize_bridge_records, LibLinuxConformanceReport, LibLinuxDispatchSample,
    LibLinuxTelemetryStore, LinuxBridgeDispatchRecord, LinuxSyscallQueue,
    LinuxSyscallRequest,
};
use super::LinuxIoRequestKind;
use super::liblinux::{LibLinuxConformanceRisk, LibLinuxSemanticClass, LinuxSyscall};
use super::linux::{LinuxResourcePlan, LinuxShimDeviceKind};
use super::reactos::{
    NtDomainImportBinding, NtExecutionPolicy, NtImportBinding, NtImportDomainCounts,
    NtSymbolTable, PeImageInfo, PeLoadError,
};
use super::sidecar::{
    SideCarBootstrapState, SideCarPayload, SideCarSaturationLevel, SideCarTelemetrySample,
    SideCarTelemetrySnapshot, SideCarTelemetryStore, SideCarTransport,
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
    pub userspace_abi: HybridUserspaceAbiReport,
    pub virtualization: HybridVirtualizationReadinessReport,
    pub gaps: Vec<HybridReadinessGap>,
    pub release_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridReleaseGateStatus {
    Pass,
    Warning,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridReleaseGateFamilyRow {
    pub family: HybridRequestFamily,
    pub min_coverage: u8,
    pub min_performance: u8,
    pub min_security: u8,
    pub high_risk_budget: usize,
    pub actual_coverage: u8,
    pub actual_performance: u8,
    pub actual_security: u8,
    pub high_risk_paths: usize,
    pub status: HybridReleaseGateStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridReleaseGateMatrix {
    pub version: &'static str,
    pub rows: Vec<HybridReleaseGateFamilyRow>,
    pub system_rows: Vec<HybridReleaseGateSystemRow>,
    pub release_blocked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridReleaseGateSystemRow {
    pub name: &'static str,
    pub min_score: u8,
    pub actual_score: u8,
    pub blocker_count: usize,
    pub release_ready: bool,
    pub status: HybridReleaseGateStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridUserspaceAbiContractRow {
    pub syscall: LinuxSyscall,
    pub io_kind: LinuxIoRequestKind,
    pub semantic_class: LibLinuxSemanticClass,
    pub zero_copy_eligible: bool,
    pub supported: bool,
    pub behavior_depth: HybridUserspaceAbiBehaviorDepth,
    pub risk: LibLinuxConformanceRisk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridUserspaceAbiContractMatrix {
    pub rows: Vec<HybridUserspaceAbiContractRow>,
    pub supported_ratio: u8,
    pub behavior_depth_ratio: u8,
    pub data_path_rows: usize,
    pub control_path_rows: usize,
    pub memory_map_rows: usize,
    pub full_depth_rows: usize,
    pub partial_depth_rows: usize,
    pub stub_depth_rows: usize,
    pub high_risk_ops: usize,
    pub release_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridUserspaceAbiBehaviorDepth {
    Full,
    Partial,
    Stub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridUserspaceAbiTailPressureLevel {
    Insufficient,
    Stable,
    Observe,
    Warn,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridUserspaceAbiReport {
    pub readiness_score: u8,
    pub confidence_score: u8,
    pub telemetry_shape_score: u8,
    pub tail_pressure_level: HybridUserspaceAbiTailPressureLevel,
    pub telemetry_samples: usize,
    pub compile_linux_compat: bool,
    pub compile_vfs: bool,
    pub boundary_allows_compat: bool,
    pub expose_linux_compat_surface: bool,
    pub attack_surface_budget: u8,
    pub attack_surface_target: u8,
    pub critical_blockers: usize,
    pub high_blockers: usize,
    pub medium_blockers: usize,
    pub effective_surface_enabled: bool,
    pub contract_matrix: HybridUserspaceAbiContractMatrix,
    pub blockers: Vec<&'static str>,
    pub next_action: &'static str,
    pub release_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridVirtualizationReadinessReport {
    pub readiness_score: u8,
    pub policy_scope: &'static str,
    pub core_path_scope: &'static str,
    pub advanced_path_scope: &'static str,
    pub execution_class: crate::config::VirtualizationExecutionClass,
    pub governor_class: crate::config::VirtualizationGovernorClass,
    pub entry_enabled: bool,
    pub resume_enabled: bool,
    pub trap_dispatch_enabled: bool,
    pub nested_enabled: bool,
    pub time_virtualization_enabled: bool,
    pub device_passthrough_enabled: bool,
    pub snapshot_enabled: bool,
    pub dirty_logging_enabled: bool,
    pub live_migration_enabled: bool,
    pub trap_tracing_enabled: bool,
    pub enabled_feature_count: usize,
    pub runtime_limited_features: usize,
    pub compiletime_limited_features: usize,
    pub fully_disabled_features: usize,
    pub can_launch_guests: bool,
    pub advanced_ops_ready: bool,
    pub blockers: Vec<&'static str>,
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

    pub fn plan_with_diagnostics_and_telemetry(
        request: &HybridRequest,
        preference: BackendPreference,
        sidecar_cfg: SideCarVmConfig,
        telemetry: Option<&SideCarTelemetryStore>,
    ) -> HybridPlanDiagnostics {
        planner::plan_with_diagnostics_and_telemetry(request, preference, sidecar_cfg, telemetry)
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

    pub fn runtime_assessment(
        request: &HybridRequest,
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridRuntimeAssessmentReport {
        planner::runtime_assessment(request, driverkit_health)
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

    pub fn coverage_audit(
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridCoverageAudit {
        planner::coverage_audit(driverkit_health)
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

    pub fn fleet_report(
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridFleetReport {
        planner::fleet_report(driverkit_health)
    }

    pub fn fleet_report_with_telemetry(
        driverkit_health: Option<DriverKitHealthSnapshot>,
        sidecar_telemetry: Option<&SideCarTelemetryStore>,
        liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
    ) -> HybridFleetReport {
        planner::fleet_report_with_telemetry(driverkit_health, sidecar_telemetry, liblinux_telemetry)
    }

    pub fn maturity_report(
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridMaturityReport {
        planner::maturity_report(driverkit_health)
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

    pub fn release_gate_matrix(
        driverkit_health: Option<DriverKitHealthSnapshot>,
    ) -> HybridReleaseGateMatrix {
        planner::release_gate_matrix(driverkit_health)
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

    pub fn userspace_abi_report() -> HybridUserspaceAbiReport {
        planner::userspace_abi_report()
    }

    pub fn virtualization_readiness_report() -> HybridVirtualizationReadinessReport {
        planner::virtualization_readiness_report()
    }
}

#[cfg(test)]
mod tests;
