use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;
use crate::modules::drivers::hybrid::liblinux::LibLinuxBridge;
use crate::modules::drivers::hybrid::linux::{build_block_plan, build_network_plan, LinuxShimDeviceKind};
use crate::modules::drivers::hybrid::reactos::{
    bind_import_names, build_import_resolution_report, parse_import_directory, parse_import_names,
    parse_pe_image, NtExecutionPolicy, NtImportResolutionReport, NtSymbolTable, PeImageInfo,
    PeLoadError,
};
use crate::modules::drivers::hybrid::sidecar::{
    SideCarSaturationLevel, SideCarTelemetryStore, SideCarVmConfig, SideCarVmPlan,
    SideCarWorkloadProfile,
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

pub fn adaptive_fallback_order(
    preference: BackendPreference,
    request_kind: HybridRequestKind,
    sidecar_telemetry: Option<&SideCarTelemetryStore>,
    liblinux_telemetry: Option<&crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore>,
) -> [BackendPreference; 4] {
    adaptive_fallback_order_with_health(
        preference,
        request_kind,
        sidecar_telemetry,
        liblinux_telemetry,
        None,
    )
}

pub fn adaptive_fallback_order_with_health(
    preference: BackendPreference,
    request_kind: HybridRequestKind,
    sidecar_telemetry: Option<&SideCarTelemetryStore>,
    liblinux_telemetry: Option<&crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore>,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> [BackendPreference; 4] {
    let mut order = fallback_order(preference).to_vec();
    let sidecar_kind = request_kind_to_linux_device_kind(request_kind);

    let sidecar_penalty = sidecar_telemetry
        .map(|telemetry| match telemetry.saturation_level_for(sidecar_kind) {
            SideCarSaturationLevel::Low => -2,
            SideCarSaturationLevel::Nominal => 0,
            SideCarSaturationLevel::High => 6,
            SideCarSaturationLevel::Critical => 12,
        })
        .unwrap_or(0);
    let liblinux_penalty = liblinux_telemetry
        .map(|telemetry| telemetry.family_failure_pressure_for_request_kind(request_kind))
        .unwrap_or(0) as i16;
    let driverkit_penalty = driverkit_health
        .map(|health| {
            let mut penalty = 0i16;
            if health.quarantined_count > 0 {
                penalty += 16;
            }
            if health.dispatch_failure_count > health.dispatch_success_count.saturating_add(2) {
                penalty += 12;
            }
            if health.faulted_count > health.started_count.saturating_add(health.binding_count) {
                penalty += 8;
            }
            penalty
        })
        .unwrap_or(0);

    order.sort_by_key(|backend| {
        let base_position = fallback_order(preference)
            .iter()
            .position(|candidate| candidate == backend)
            .unwrap_or(3) as i16;

        let mut score = base_position * 10;

        match backend {
            BackendPreference::SideCarFirst => {
                score += sidecar_penalty * 5;
                if request_kind == HybridRequestKind::WindowsPe {
                    score += 40;
                }
            }
            BackendPreference::LibLinuxFirst => {
                score += liblinux_penalty / 2;
                if liblinux_penalty >= 35 {
                    score += 16;
                }
            }
            BackendPreference::DriverKitFirst => {
                score += if sidecar_penalty >= 2 && liblinux_penalty >= 25 { -2 } else { 1 };
                score += driverkit_penalty;
            }
            BackendPreference::ReactOsFirst => {
                score += if request_kind == HybridRequestKind::WindowsPe { -30 } else { 5 };
            }
        }

        score
    });

    [order[0], order[1], order[2], order[3]]
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
        BackendPreference::ReactOsFirst => plan_reactos(request),
    }
}

fn reactos_pilot_image_info_for_request(request: &HybridRequest) -> PeImageInfo {
    PeImageInfo {
        machine: 0x8664,
        image_base: request.mmio_base as u64,
        entry_rva: 0,
        size_of_image: request.mmio_length as u32,
        size_of_headers: 0,
        number_of_sections: 0,
        sections: Vec::new(),
        import_directory_rva: 0,
        import_directory_size: 0,
        relocation_directory_rva: 0,
        relocation_directory_size: 0,
    }
}

fn plan_reactos(request: &HybridRequest) -> Option<HybridExecutionPlan> {
    if !supports_reactos_pilot(request.kind) {
        return None;
    }

    Some(HybridExecutionPlan::ReactOs {
        policy: NtExecutionPolicy::wine_bridge(),
        image_info: reactos_pilot_image_info_for_request(request),
    })
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

fn supports_reactos_pilot(kind: HybridRequestKind) -> bool {
    matches!(
        kind,
        HybridRequestKind::WindowsPe
            | HybridRequestKind::Firmware
            | HybridRequestKind::Input
            | HybridRequestKind::Touch
            | HybridRequestKind::Gamepad
    )
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
    plan_with_fallbacks_and_dual_telemetry(request, preference, sidecar_cfg, telemetry, None)
}

pub fn plan_with_fallbacks_and_dual_telemetry(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
    sidecar_telemetry: Option<&SideCarTelemetryStore>,
    liblinux_telemetry: Option<&crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore>,
) -> Option<HybridExecutionPlan> {
    plan_with_fallbacks_with_full_context(
        request,
        preference,
        sidecar_cfg,
        sidecar_telemetry,
        liblinux_telemetry,
        None,
    )
}

pub fn plan_with_fallbacks_with_full_context(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
    sidecar_telemetry: Option<&SideCarTelemetryStore>,
    liblinux_telemetry: Option<&crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore>,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> Option<HybridExecutionPlan> {
    let order = adaptive_fallback_order(
        preference,
        request.kind,
        sidecar_telemetry,
        liblinux_telemetry,
    );
    let order = if driverkit_health.is_some() {
        adaptive_fallback_order_with_health(
            preference,
            request.kind,
            sidecar_telemetry,
            liblinux_telemetry,
            driverkit_health,
        )
    } else {
        order
    };
    for candidate in order {
        if let Some(plan) = plan_internal(request, candidate, sidecar_cfg, sidecar_telemetry) {
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
    plan_with_diagnostics_and_dual_telemetry(request, preference, sidecar_cfg, telemetry, None)
}

pub fn plan_with_diagnostics_and_dual_telemetry(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
    sidecar_telemetry: Option<&SideCarTelemetryStore>,
    liblinux_telemetry: Option<&crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore>,
) -> HybridPlanDiagnostics {
    plan_with_diagnostics_with_full_context(
        request,
        preference,
        sidecar_cfg,
        sidecar_telemetry,
        liblinux_telemetry,
        None,
    )
}

pub fn plan_with_diagnostics_with_full_context(
    request: &HybridRequest,
    preference: BackendPreference,
    sidecar_cfg: SideCarVmConfig,
    sidecar_telemetry: Option<&SideCarTelemetryStore>,
    liblinux_telemetry: Option<&crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore>,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> HybridPlanDiagnostics {
    let order = adaptive_fallback_order_with_health(
        preference,
        request.kind,
        sidecar_telemetry,
        liblinux_telemetry,
        driverkit_health,
    );
    let mut attempts = Vec::new();
    for candidate in order {
        let plan = plan_internal(request, candidate, sidecar_cfg, sidecar_telemetry);
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
