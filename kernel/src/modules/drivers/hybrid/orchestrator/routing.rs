use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;
use crate::modules::drivers::hybrid::liblinux::LibLinuxBridge;
use crate::modules::drivers::hybrid::linux::{build_block_plan, build_network_plan, LinuxShimDeviceKind};
use crate::modules::drivers::hybrid::reactos::{
    bind_import_names, build_import_resolution_report, parse_import_directory, parse_import_names,
    parse_pe_image, NtExecutionPolicy, NtImportResolutionReport, NtSymbolTable, PeLoadError,
};
use crate::modules::drivers::hybrid::sidecar::{
    SideCarTelemetryStore, SideCarVmConfig, SideCarVmPlan, SideCarWorkloadProfile,
};
use crate::modules::drivers::{DriverTransportKind, IrqGrant, MmioGrant};
use super::super::{BackendPreference, HybridExecutionPlan, HybridPlanAttempt, HybridPlanDiagnostics,
    HybridRequest, HybridRequestKind, ReactOsImportResolution};

pub fn fallback_order(preference: BackendPreference) -> [BackendPreference; 4] {
    match preference {
        BackendPreference::SideCarFirst => [
            BackendPreference::SideCarFirst,
            BackendPreference::LibLinuxFirst,
            BackendPreference::DriverKitFirst,
            BackendPreference::ReactOsFirst,
        ],
        BackendPreference::LibLinuxFirst => [
            BackendPreference::LibLinuxFirst,
            BackendPreference::SideCarFirst,
            BackendPreference::DriverKitFirst,
            BackendPreference::ReactOsFirst,
        ],
        BackendPreference::DriverKitFirst => [
            BackendPreference::DriverKitFirst,
            BackendPreference::SideCarFirst,
            BackendPreference::LibLinuxFirst,
            BackendPreference::ReactOsFirst,
        ],
        BackendPreference::ReactOsFirst => [
            BackendPreference::ReactOsFirst,
            BackendPreference::SideCarFirst,
            BackendPreference::LibLinuxFirst,
            BackendPreference::DriverKitFirst,
        ],
    }
}

pub fn adapt_preference_with_driverkit_health(
    preference: BackendPreference,
    health: DriverKitHealthSnapshot,
) -> BackendPreference {
    if preference != BackendPreference::DriverKitFirst {
        return preference;
    }

    if health.quarantined_count > 0 {
        return BackendPreference::SideCarFirst;
    }

    if health.dispatch_failure_count > health.dispatch_success_count.saturating_add(2) {
        return BackendPreference::LibLinuxFirst;
    }

    preference
}

pub fn plan(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
) -> Option<HybridExecutionPlan> {
    plan_internal(request, preference, sidecar_cfg, None)
}

pub fn plan_with_sidecar_telemetry(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
    telemetry: Option<&SideCarTelemetryStore>,
) -> Option<HybridExecutionPlan> {
    plan_internal(request, preference, sidecar_cfg, telemetry)
}

fn plan_internal(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
    telemetry: Option<&SideCarTelemetryStore>,
) -> Option<HybridExecutionPlan> {
    if !has_valid_resources(request) {
        return None;
    }

    match preference {
        BackendPreference::SideCarFirst => plan_sidecar(request, sidecar_cfg, telemetry),
        BackendPreference::LibLinuxFirst => plan_liblinux(request),
        BackendPreference::DriverKitFirst => plan_driverkit(request),
        BackendPreference::ReactOsFirst => None,
    }
}

fn plan_sidecar(
    request: &HybridRequest,
    sidecar_cfg: SideCarVmConfig,
    telemetry: Option<&SideCarTelemetryStore>,
) -> Option<HybridExecutionPlan> {
    if !supports_sidecar(request.kind) {
        return None;
    }

    let kind = request_kind_to_linux_device_kind(request.kind);
    let base_workload = SideCarWorkloadProfile::from_resource_lengths(
        request.mmio_length,
        request.iova_length,
    );

    Some(HybridExecutionPlan::SideCar(
        SideCarVmPlan::for_linux_device_with_telemetry(
            sidecar_cfg,
            kind,
            request.irq_vector,
            base_workload,
            telemetry,
        ),
    ))
}

fn plan_liblinux(request: &HybridRequest) -> Option<HybridExecutionPlan> {
    if !supports_liblinux(request.kind) {
        return None;
    }

    let mut plan = match liblinux_route_profile(request.kind) {
        LibLinuxRouteProfile::BridgeNetwork => LibLinuxBridge::plan_network(
            request.mmio_base,
            request.mmio_length,
            request.iova_base,
            request.iova_length,
            request.irq_vector,
        ),
        LibLinuxRouteProfile::Block => build_block_plan(
            DriverTransportKind::LibLinux,
            request.mmio_base,
            request.mmio_length,
            request.iova_base,
            request.iova_length,
            request.irq_vector,
        ),
        LibLinuxRouteProfile::Network => build_network_plan(
            DriverTransportKind::LibLinux,
            request.mmio_base,
            request.mmio_length,
            request.iova_base,
            request.iova_length,
            request.irq_vector,
        ),
    };

    plan.device_kind = request_kind_to_linux_device_kind(request.kind);

    Some(HybridExecutionPlan::LibLinux(plan))
}

fn plan_driverkit(request: &HybridRequest) -> Option<HybridExecutionPlan> {
    if !supports_driverkit(request.kind) {
        return None;
    }

    Some(HybridExecutionPlan::DriverKit(
        super::super::UserModeDriverContext::new()
            .add_mmio(MmioGrant::new(request.mmio_base, request.mmio_length))
            .add_irq(IrqGrant::new(request.irq_vector)),
    ))
}

fn supports_sidecar(kind: HybridRequestKind) -> bool {
    !matches!(kind, HybridRequestKind::WindowsPe)
}

fn supports_liblinux(kind: HybridRequestKind) -> bool {
    !matches!(kind, HybridRequestKind::WindowsPe)
}

fn supports_driverkit(kind: HybridRequestKind) -> bool {
    !matches!(kind, HybridRequestKind::WindowsPe)
}

fn has_valid_resources(request: &HybridRequest) -> bool {
    if request.kind == HybridRequestKind::WindowsPe {
        return true;
    }

    if request.irq_vector == 0 {
        return false;
    }

    match request.kind {
        HybridRequestKind::UserModeDevice => request.mmio_length > 0,
        _ => request.mmio_length > 0 && request.iova_length > 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LibLinuxRouteProfile {
    BridgeNetwork,
    Block,
    Network,
}

fn liblinux_route_profile(kind: HybridRequestKind) -> LibLinuxRouteProfile {
    match kind {
        HybridRequestKind::Network | HybridRequestKind::Modem | HybridRequestKind::Nfc => {
            LibLinuxRouteProfile::BridgeNetwork
        }
        HybridRequestKind::Block
        | HybridRequestKind::Storage
        | HybridRequestKind::Printer
        | HybridRequestKind::Tpm
        | HybridRequestKind::Dock
        | HybridRequestKind::Firmware
        | HybridRequestKind::SmartCard
        | HybridRequestKind::Usb
        | HybridRequestKind::Can
        | HybridRequestKind::Serial
        | HybridRequestKind::Nvme => LibLinuxRouteProfile::Block,
        _ => LibLinuxRouteProfile::Network,
    }
}

pub fn plan_with_driverkit_health(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
    health: DriverKitHealthSnapshot,
) -> Option<HybridExecutionPlan> {
    let effective = adapt_preference_with_driverkit_health(preference, health);
    plan(request, effective, sidecar_cfg)
}

pub fn plan_with_fallbacks(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
) -> Option<HybridExecutionPlan> {
    plan_with_fallbacks_and_telemetry(request, preference, sidecar_cfg, None)
}

pub fn plan_with_fallbacks_and_telemetry(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
    telemetry: Option<&SideCarTelemetryStore>,
) -> Option<HybridExecutionPlan> {
    for candidate in fallback_order(preference) {
        if let Some(plan) = plan_internal(request, candidate, sidecar_cfg, telemetry) {
            return Some(plan);
        }
    }
    None
}

pub fn plan_with_diagnostics(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
) -> HybridPlanDiagnostics {
    plan_with_diagnostics_and_telemetry(request, preference, sidecar_cfg, None)
}

pub fn plan_with_diagnostics_and_telemetry(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
    telemetry: Option<&SideCarTelemetryStore>,
) -> HybridPlanDiagnostics {
    let mut attempts = Vec::new();
    for candidate in fallback_order(preference) {
        let plan = plan_internal(request, candidate, sidecar_cfg, telemetry);
        attempts.push(HybridPlanAttempt {
            backend: candidate,
            matched: plan.is_some(),
        });
        if let Some(selected) = plan {
            return HybridPlanDiagnostics {
                attempts,
                selected: Some(selected),
            };
        }
    }

    HybridPlanDiagnostics {
        attempts,
        selected: None,
    }
}

pub fn plan_windows_pe(
    image: &[u8],
    preference: BackendPreference,
) -> Result<HybridExecutionPlan, PeLoadError> {
    let image_info = parse_pe_image(image)?;
    let policy = match preference {
        BackendPreference::ReactOsFirst => NtExecutionPolicy::native(),
        _ => NtExecutionPolicy::wine_bridge(),
    };
    Ok(HybridExecutionPlan::ReactOs { policy, image_info })
}

pub fn plan_windows_pe_with_symbols(
    image: &[u8],
    symbols: &NtSymbolTable,
) -> Result<ReactOsImportResolution, PeLoadError> {
    let image_info = parse_pe_image(image)?;
    let descriptors = parse_import_directory(image, &image_info)?;
    let import_names = parse_import_names(image, &image_info, &descriptors);
    let report: NtImportResolutionReport = build_import_resolution_report(&import_names, symbols);
    let bindings = bind_import_names(&import_names, symbols);
    Ok(ReactOsImportResolution {
        image_info,
        bindings,
        domain_bindings: report.bindings,
        counts: report.counts,
        policy: report.policy,
    })
}

fn request_kind_to_linux_device_kind(request_kind: HybridRequestKind) -> LinuxShimDeviceKind {
    match request_kind {
        HybridRequestKind::Network => LinuxShimDeviceKind::Network,
        HybridRequestKind::Block => LinuxShimDeviceKind::Block,
        HybridRequestKind::Storage => LinuxShimDeviceKind::Storage,
        HybridRequestKind::Nvme => LinuxShimDeviceKind::Nvme,
        HybridRequestKind::Ethernet => LinuxShimDeviceKind::Ethernet,
        HybridRequestKind::Modem => LinuxShimDeviceKind::Modem,
        HybridRequestKind::Printer => LinuxShimDeviceKind::Printer,
        HybridRequestKind::Rtc => LinuxShimDeviceKind::Rtc,
        HybridRequestKind::SensorHub => LinuxShimDeviceKind::SensorHub,
        HybridRequestKind::Gpu => LinuxShimDeviceKind::Gpu,
        HybridRequestKind::Display => LinuxShimDeviceKind::Display,
        HybridRequestKind::WiFi => LinuxShimDeviceKind::WiFi,
        HybridRequestKind::Bluetooth => LinuxShimDeviceKind::Bluetooth,
        HybridRequestKind::Nfc => LinuxShimDeviceKind::Nfc,
        HybridRequestKind::Tpm => LinuxShimDeviceKind::Tpm,
        HybridRequestKind::Dock => LinuxShimDeviceKind::Dock,
        HybridRequestKind::Usb => LinuxShimDeviceKind::Usb,
        HybridRequestKind::Can => LinuxShimDeviceKind::Can,
        HybridRequestKind::Serial => LinuxShimDeviceKind::Serial,
        HybridRequestKind::Firmware => LinuxShimDeviceKind::Firmware,
        HybridRequestKind::SmartCard => LinuxShimDeviceKind::SmartCard,
        HybridRequestKind::Touch => LinuxShimDeviceKind::Touch,
        HybridRequestKind::Gamepad => LinuxShimDeviceKind::Gamepad,
        HybridRequestKind::Camera => LinuxShimDeviceKind::Camera,
        HybridRequestKind::Audio => LinuxShimDeviceKind::Audio,
        HybridRequestKind::Sensor => LinuxShimDeviceKind::Sensor,
        HybridRequestKind::Input => LinuxShimDeviceKind::Input,
        HybridRequestKind::WindowsPe => LinuxShimDeviceKind::Generic,
        HybridRequestKind::UserModeDevice => LinuxShimDeviceKind::Generic,
    }
}
