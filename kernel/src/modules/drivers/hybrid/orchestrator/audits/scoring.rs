use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;
use crate::modules::drivers::hybrid::orchestrator::{
    BackendPreference, HybridFeatureKind, HybridGapSeverity, HybridPerformanceTier,
    HybridRequest, HybridRequestFamily, HybridRequestKind, HybridRuntimeConfidence,
    HybridSecurityPosture,
};
use super::{
    feature_coverage_stats_for, request_family,
};

#[derive(Clone, Copy)]
struct SupportProfile {
    reason: &'static str,
}

#[derive(Clone, Copy)]
struct BackendCapabilities {
    isolation: i16,
    native_semantics: i16,
    mediation: i16,
    compatibility: i16,
    throughput: i16,
    security_containment: i16,
}

#[derive(Clone, Copy)]
struct RequestShape {
    dma_intensity: i16,
    latency_sensitivity: i16,
    security_sensitivity: i16,
    compatibility_pressure: i16,
    bandwidth_pressure: i16,
    control_plane_pressure: i16,
}

fn merge_shapes(base: RequestShape, delta: RequestShape) -> RequestShape {
    RequestShape {
        dma_intensity: (base.dma_intensity + delta.dma_intensity).clamp(1, 24),
        latency_sensitivity: (base.latency_sensitivity + delta.latency_sensitivity).clamp(1, 24),
        security_sensitivity: (base.security_sensitivity + delta.security_sensitivity).clamp(1, 24),
        compatibility_pressure: (base.compatibility_pressure + delta.compatibility_pressure).clamp(1, 24),
        bandwidth_pressure: (base.bandwidth_pressure + delta.bandwidth_pressure).clamp(1, 24),
        control_plane_pressure: (base.control_plane_pressure + delta.control_plane_pressure).clamp(1, 24),
    }
}

fn backend_capabilities(backend: BackendPreference) -> BackendCapabilities {
    match backend {
        BackendPreference::SideCarFirst => BackendCapabilities {
            isolation: 18,
            native_semantics: 8,
            mediation: 12,
            compatibility: 4,
            throughput: 10,
            security_containment: 16,
        },
        BackendPreference::LibLinuxFirst => BackendCapabilities {
            isolation: 6,
            native_semantics: 16,
            mediation: 8,
            compatibility: 8,
            throughput: 16,
            security_containment: 8,
        },
        BackendPreference::DriverKitFirst => BackendCapabilities {
            isolation: 12,
            native_semantics: 6,
            mediation: 14,
            compatibility: 6,
            throughput: 4,
            security_containment: 12,
        },
        BackendPreference::ReactOsFirst => BackendCapabilities {
            isolation: 4,
            native_semantics: 4,
            mediation: 6,
            compatibility: 20,
            throughput: 2,
            security_containment: 2,
        },
    }
}

fn feature_capability_delta(
    request_kind: HybridRequestKind,
    backend: BackendPreference,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> BackendCapabilities {
    let coverage = feature_coverage_stats_for(request_kind, backend, driverkit_health);
    let supported = &coverage.supported;
    let mut delta = BackendCapabilities {
        isolation: 0,
        native_semantics: 0,
        mediation: 0,
        compatibility: 0,
        throughput: 0,
        security_containment: 0,
    };

    for feature in supported {
        match feature {
            HybridFeatureKind::Mmio => {
                delta.native_semantics += 2;
                delta.compatibility += 1;
            }
            HybridFeatureKind::Dma => {
                delta.throughput += 4;
                delta.native_semantics += 1;
            }
            HybridFeatureKind::Irq => {
                delta.mediation += 2;
                delta.throughput += 1;
            }
            HybridFeatureKind::SharedMemory => {
                delta.isolation += 2;
                delta.mediation += 1;
                delta.throughput += 2;
            }
            HybridFeatureKind::ControlQueue => {
                delta.mediation += 3;
                delta.compatibility += 1;
            }
            HybridFeatureKind::Reset => {
                delta.mediation += 1;
            }
            HybridFeatureKind::Hotplug => {
                delta.compatibility += 2;
                delta.mediation += 1;
            }
            HybridFeatureKind::PowerManagement => {
                delta.mediation += 1;
                delta.security_containment += 1;
            }
            HybridFeatureKind::Snapshot => {
                delta.isolation += 1;
                delta.compatibility += 1;
            }
            HybridFeatureKind::LiveMigration => {
                delta.compatibility += 3;
                delta.mediation += 2;
                delta.throughput += 1;
            }
        }
    }

    let missing = 10i16.saturating_sub(supported.len() as i16);
    delta.throughput -= missing / 2;
    delta.compatibility -= missing / 3;
    delta.security_containment -= missing / 4;

    let missing_required = coverage.missing_required() as i16;
    let missing_mandatory = coverage.missing_mandatory() as i16;
    delta.compatibility -= missing_required * 2;
    delta.mediation -= missing_required * 2;
    delta.security_containment -= missing_required;
    delta.compatibility -= missing_mandatory * 3;
    delta.mediation -= missing_mandatory * 3;
    delta.throughput -= missing_mandatory * 2;
    delta.security_containment -= missing_mandatory * 2;
    delta
}

fn observed_backend_capabilities(
    request_kind: HybridRequestKind,
    backend: BackendPreference,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> BackendCapabilities {
    let base = backend_capabilities(backend);
    let delta = feature_capability_delta(request_kind, backend, driverkit_health);
    BackendCapabilities {
        isolation: (base.isolation + delta.isolation).clamp(1, 40),
        native_semantics: (base.native_semantics + delta.native_semantics).clamp(1, 40),
        mediation: (base.mediation + delta.mediation).clamp(1, 40),
        compatibility: (base.compatibility + delta.compatibility).clamp(1, 40),
        throughput: (base.throughput + delta.throughput).clamp(1, 40),
        security_containment: (base.security_containment + delta.security_containment).clamp(1, 40),
    }
}

fn request_shape(kind: HybridRequestKind) -> RequestShape {
    match kind {
        HybridRequestKind::Network
        | HybridRequestKind::Ethernet
        | HybridRequestKind::WiFi
        | HybridRequestKind::Bluetooth
        | HybridRequestKind::Modem
        | HybridRequestKind::Nfc
        | HybridRequestKind::Can => RequestShape {
            dma_intensity: 6,
            latency_sensitivity: 8,
            security_sensitivity: 6,
            compatibility_pressure: 4,
            bandwidth_pressure: 12,
            control_plane_pressure: 6,
        },
        HybridRequestKind::Block | HybridRequestKind::Storage | HybridRequestKind::Nvme => RequestShape {
            dma_intensity: 14,
            latency_sensitivity: 10,
            security_sensitivity: 4,
            compatibility_pressure: 4,
            bandwidth_pressure: 10,
            control_plane_pressure: 4,
        },
        HybridRequestKind::Gpu | HybridRequestKind::Camera | HybridRequestKind::Audio | HybridRequestKind::Display => RequestShape {
            dma_intensity: 8,
            latency_sensitivity: 12,
            security_sensitivity: 4,
            compatibility_pressure: 6,
            bandwidth_pressure: 14,
            control_plane_pressure: 4,
        },
        HybridRequestKind::Input | HybridRequestKind::Touch | HybridRequestKind::Gamepad => RequestShape {
            dma_intensity: 4,
            latency_sensitivity: 14,
            security_sensitivity: 5,
            compatibility_pressure: 6,
            bandwidth_pressure: 4,
            control_plane_pressure: 10,
        },
        HybridRequestKind::Tpm | HybridRequestKind::SmartCard => RequestShape {
            dma_intensity: 2,
            latency_sensitivity: 6,
            security_sensitivity: 18,
            compatibility_pressure: 8,
            bandwidth_pressure: 2,
            control_plane_pressure: 12,
        },
        HybridRequestKind::WindowsPe => RequestShape {
            dma_intensity: 2,
            latency_sensitivity: 4,
            security_sensitivity: 6,
            compatibility_pressure: 20,
            bandwidth_pressure: 2,
            control_plane_pressure: 10,
        },
        HybridRequestKind::UserModeDevice => RequestShape {
            dma_intensity: 6,
            latency_sensitivity: 10,
            security_sensitivity: 8,
            compatibility_pressure: 6,
            bandwidth_pressure: 4,
            control_plane_pressure: 16,
        },
        HybridRequestKind::Rtc | HybridRequestKind::SensorHub | HybridRequestKind::Sensor => RequestShape {
            dma_intensity: 4,
            latency_sensitivity: 12,
            security_sensitivity: 6,
            compatibility_pressure: 4,
            bandwidth_pressure: 4,
            control_plane_pressure: 10,
        },
        HybridRequestKind::Dock | HybridRequestKind::Usb | HybridRequestKind::Serial | HybridRequestKind::Printer | HybridRequestKind::Firmware => RequestShape {
            dma_intensity: 6,
            latency_sensitivity: 6,
            security_sensitivity: 4,
            compatibility_pressure: 10,
            bandwidth_pressure: 6,
            control_plane_pressure: 10,
        },
    }
}

fn request_kind_hint(kind: HybridRequestKind) -> &'static str {
    match kind {
        HybridRequestKind::Network | HybridRequestKind::Ethernet | HybridRequestKind::WiFi => {
            "transport-oriented path"
        }
        HybridRequestKind::Block | HybridRequestKind::Storage | HybridRequestKind::Nvme => {
            "storage-oriented path"
        }
        HybridRequestKind::Gpu | HybridRequestKind::Camera | HybridRequestKind::Audio | HybridRequestKind::Display => {
            "high-bandwidth media path"
        }
        HybridRequestKind::Input | HybridRequestKind::Touch | HybridRequestKind::Gamepad => {
            "latency-sensitive input path"
        }
        HybridRequestKind::Tpm | HybridRequestKind::SmartCard => {
            "security-sensitive token path"
        }
        HybridRequestKind::WindowsPe => "compatibility-driven PE path",
        HybridRequestKind::UserModeDevice => "user-mode driver host path",
        HybridRequestKind::Rtc | HybridRequestKind::SensorHub | HybridRequestKind::Sensor => {
            "telemetry and timing path"
        }
        HybridRequestKind::Dock | HybridRequestKind::Usb | HybridRequestKind::Serial | HybridRequestKind::Printer | HybridRequestKind::Firmware => {
            "peripheral management path"
        }
        HybridRequestKind::Modem | HybridRequestKind::Bluetooth | HybridRequestKind::Can | HybridRequestKind::Nfc => {
            "mixed-control transport path"
        }
    }
}

fn shape_fit_score_with_caps(caps: BackendCapabilities, shape: RequestShape) -> i16 {
    caps.isolation * shape.security_sensitivity
        + caps.native_semantics * shape.compatibility_pressure
        + caps.mediation * shape.control_plane_pressure
        + caps.compatibility * shape.compatibility_pressure
        + caps.throughput * shape.bandwidth_pressure
        + caps.security_containment * shape.security_sensitivity
        - shape.dma_intensity * 2
        - shape.latency_sensitivity * 2
}

fn score_from_fit(raw_fit: i16) -> u8 {
    ((raw_fit + 120).clamp(0, 240) * 100 / 240) as u8
}

fn family_demand(family: HybridRequestFamily) -> (RequestShape, &'static str) {
    match family {
        HybridRequestFamily::Network => (
            RequestShape {
                dma_intensity: 4,
                latency_sensitivity: 12,
                security_sensitivity: 8,
                compatibility_pressure: 4,
                bandwidth_pressure: 16,
                control_plane_pressure: 8,
            },
            "network family demands throughput, queue ordering, and stable mediation",
        ),
        HybridRequestFamily::Storage => (
            RequestShape {
                dma_intensity: 16,
                latency_sensitivity: 10,
                security_sensitivity: 6,
                compatibility_pressure: 4,
                bandwidth_pressure: 14,
                control_plane_pressure: 4,
            },
            "storage family demands DMA discipline and high request throughput",
        ),
        HybridRequestFamily::Multimedia => (
            RequestShape {
                dma_intensity: 8,
                latency_sensitivity: 14,
                security_sensitivity: 4,
                compatibility_pressure: 6,
                bandwidth_pressure: 16,
                control_plane_pressure: 4,
            },
            "multimedia family demands low jitter and high bandwidth fan-out",
        ),
        HybridRequestFamily::Input => (
            RequestShape {
                dma_intensity: 4,
                latency_sensitivity: 16,
                security_sensitivity: 6,
                compatibility_pressure: 6,
                bandwidth_pressure: 4,
                control_plane_pressure: 10,
            },
            "input family demands low latency and precise event handling",
        ),
        HybridRequestFamily::Security => (
            RequestShape {
                dma_intensity: 2,
                latency_sensitivity: 8,
                security_sensitivity: 18,
                compatibility_pressure: 8,
                bandwidth_pressure: 2,
                control_plane_pressure: 14,
            },
            "security family demands strong containment and controlled mediation",
        ),
        HybridRequestFamily::Platform => (
            RequestShape {
                dma_intensity: 4,
                latency_sensitivity: 8,
                security_sensitivity: 10,
                compatibility_pressure: 10,
                bandwidth_pressure: 6,
                control_plane_pressure: 16,
            },
            "platform family demands management reach and wake coordination",
        ),
        HybridRequestFamily::Compatibility => (
            RequestShape {
                dma_intensity: 2,
                latency_sensitivity: 6,
                security_sensitivity: 4,
                compatibility_pressure: 20,
                bandwidth_pressure: 2,
                control_plane_pressure: 12,
            },
            "compatibility family demands ABI fidelity over raw throughput",
        ),
        HybridRequestFamily::Peripheral => (
            RequestShape {
                dma_intensity: 6,
                latency_sensitivity: 8,
                security_sensitivity: 6,
                compatibility_pressure: 10,
                bandwidth_pressure: 8,
                control_plane_pressure: 10,
            },
            "peripheral family demands hotplug stability and control-path balance",
        ),
    }
}

fn family_support_profile(_backend: BackendPreference, family: HybridRequestFamily) -> SupportProfile {
    let (_demand, reason) = family_demand(family);
    SupportProfile {
        reason,
    }
}

fn request_resource_shape(request: &HybridRequest) -> RequestShape {
    let mmio_kib = (request.mmio_length / 1024) as i16;
    let iova_kib = (request.iova_length / 1024) as i16;
    let mmio_pressure = (mmio_kib / 8).clamp(0, 8);
    let dma_pressure = (iova_kib / 32).clamp(0, 10);
    let irq_pressure = if request.irq_vector >= 128 {
        2
    } else if request.irq_vector >= 64 {
        1
    } else {
        0
    };

    RequestShape {
        dma_intensity: dma_pressure,
        latency_sensitivity: irq_pressure,
        security_sensitivity: if request.kind == HybridRequestKind::UserModeDevice { 2 } else { 0 },
        compatibility_pressure: mmio_pressure / 2,
        bandwidth_pressure: mmio_pressure + dma_pressure / 2,
        control_plane_pressure: irq_pressure + mmio_pressure / 2,
    }
}

fn observed_request_shape(request: &HybridRequest) -> RequestShape {
    let base = request_shape(request.kind);
    let family = request_family(request.kind);
    let (family_shape, _) = family_demand(family);
    let family_delta = RequestShape {
        dma_intensity: (family_shape.dma_intensity - base.dma_intensity) / 3,
        latency_sensitivity: (family_shape.latency_sensitivity - base.latency_sensitivity) / 3,
        security_sensitivity: (family_shape.security_sensitivity - base.security_sensitivity) / 3,
        compatibility_pressure: (family_shape.compatibility_pressure - base.compatibility_pressure) / 3,
        bandwidth_pressure: (family_shape.bandwidth_pressure - base.bandwidth_pressure) / 3,
        control_plane_pressure: (family_shape.control_plane_pressure - base.control_plane_pressure) / 3,
    };

    let resource_delta = request_resource_shape(request);
    merge_shapes(base, merge_shapes(family_delta, resource_delta))
}

fn apply_delta(score: u8, delta: i16) -> u8 {
    (score as i16 + delta).clamp(0, 100) as u8
}

fn semantic_bias(request_kind: HybridRequestKind, backend: BackendPreference) -> i16 {
    let family = request_family(request_kind);

    if request_kind == HybridRequestKind::WindowsPe {
        return if backend == BackendPreference::ReactOsFirst {
            16
        } else {
            -14
        };
    }

    if request_kind == HybridRequestKind::UserModeDevice {
        return match backend {
            BackendPreference::DriverKitFirst => 14,
            BackendPreference::SideCarFirst => 4,
            BackendPreference::LibLinuxFirst => 2,
            BackendPreference::ReactOsFirst => -10,
        };
    }

    let family_bias = match (family, backend) {
        (HybridRequestFamily::Network, BackendPreference::LibLinuxFirst) => 4,
        (HybridRequestFamily::Storage, BackendPreference::LibLinuxFirst) => 4,
        (HybridRequestFamily::Multimedia, BackendPreference::SideCarFirst) => 4,
        (HybridRequestFamily::Input, BackendPreference::SideCarFirst) => 3,
        (HybridRequestFamily::Security, BackendPreference::DriverKitFirst) => 3,
        (HybridRequestFamily::Compatibility, BackendPreference::ReactOsFirst) => 8,
        (HybridRequestFamily::Peripheral, BackendPreference::LibLinuxFirst) => 3,
        (_, BackendPreference::ReactOsFirst) => -2,
        _ => 0,
    };

    let token_bias = if matches!(request_kind, HybridRequestKind::Tpm | HybridRequestKind::SmartCard)
    {
        match backend {
            BackendPreference::DriverKitFirst | BackendPreference::LibLinuxFirst => 2,
            BackendPreference::ReactOsFirst => -6,
            BackendPreference::SideCarFirst => -2,
        }
    } else {
        0
    };

    family_bias + token_bias
}

fn support_reason(
    score: u8,
    fit_adjustment: i16,
    capability_fit: i16,
    semantic_delta: i16,
    degraded: bool,
    fallback: &'static str,
    request_kind: HybridRequestKind,
) -> &'static str {
    if score < 35 {
        return request_kind_hint(request_kind);
    }
    if degraded {
        return "support degraded by runtime health or semantic mismatch";
    }
    if semantic_delta >= 8 {
        return "strong semantic alignment between request path and backend model";
    }
    if fit_adjustment + capability_fit >= 18 {
        return "backend capabilities align well with request demand profile";
    }
    fallback
}

fn family_reason(backend: BackendPreference, family: HybridRequestFamily) -> &'static str {
    family_support_profile(backend, family).reason
}

pub(crate) fn score_backend_support_raw(
    request: &HybridRequest,
    backend: BackendPreference,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> (u8, bool, &'static str) {
    let family = request_family(request.kind);
    let shape = observed_request_shape(request);
    let observed_caps = observed_backend_capabilities(request.kind, backend, driverkit_health);
    let mut score = score_from_fit(shape_fit_score_with_caps(observed_caps, shape));
    let mut degraded = false;
    let fallback_reason = family_reason(backend, family);

    let fit_adjustment = match backend {
        BackendPreference::ReactOsFirst => {
            let compat_bias = shape.compatibility_pressure + shape.control_plane_pressure / 2;
            compat_bias - shape.bandwidth_pressure / 2 - shape.dma_intensity
        }
        BackendPreference::SideCarFirst => {
            let isolation_bias = shape.security_sensitivity + shape.dma_intensity / 2 + shape.control_plane_pressure / 2;
            isolation_bias - shape.compatibility_pressure / 3
        }
        BackendPreference::LibLinuxFirst => {
            let transport_bias = shape.bandwidth_pressure + shape.dma_intensity + shape.latency_sensitivity / 2;
            transport_bias - shape.security_sensitivity / 4
        }
        BackendPreference::DriverKitFirst => {
            let broker_bias = shape.control_plane_pressure + shape.security_sensitivity / 2 + shape.compatibility_pressure / 2;
            broker_bias - shape.bandwidth_pressure / 3 - shape.dma_intensity / 2
        }
    };

    score = apply_delta(score, fit_adjustment);

    let capability_fit = shape_fit_score_with_caps(observed_caps, shape) / 8;
    score = apply_delta(score, capability_fit as i16);

    let semantic_delta = semantic_bias(request.kind, backend);
    score = apply_delta(score, semantic_delta);
    degraded |= semantic_delta < 0;

    if fit_adjustment < 0 {
        degraded = true;
    }

    if backend == BackendPreference::DriverKitFirst {
        if let Some(health) = driverkit_health {
            if health.quarantined_count > 0 {
                score = score.saturating_sub(28);
                degraded = true;
            }
            if health.dispatch_failure_count > health.dispatch_success_count.saturating_add(2) {
                score = score.saturating_sub(18);
                degraded = true;
            }
            if health.faulted_count > health.started_count.saturating_add(health.binding_count) {
                score = score.saturating_sub(8);
                degraded = true;
            }
        }
    }

    let reason = support_reason(
        score,
        fit_adjustment,
        capability_fit as i16,
        semantic_delta,
        degraded,
        fallback_reason,
        request.kind,
    );

    (score, degraded, reason)
}

pub(crate) fn support_cutoff_for_scores(scores: &[u8]) -> u8 {
    if scores.is_empty() {
        return 0;
    }

    let mean = score_mean(scores);
    let spread = score_spread(scores);
    mean.saturating_sub(spread / 4)
}

pub(crate) fn confidence_cutoffs_for_scores(scores: &[u8]) -> (u8, u8) {
    if scores.is_empty() {
        return (0, 0);
    }

    let mean = score_mean(scores);
    let spread = score_spread(scores);
    let medium_floor = mean.saturating_sub(spread / 6);
    let high_floor = mean.saturating_add(spread / 3).min(100);
    (medium_floor, high_floor)
}

pub(crate) fn performance_cutoffs_for_scores(scores: &[u8]) -> (u8, u8) {
    if scores.is_empty() {
        return (0, 0);
    }

    let mean = score_mean(scores);
    let spread = score_spread(scores);
    let low_floor = mean.saturating_sub(spread / 3);
    let high_floor = mean.saturating_add(spread / 4).min(100);
    (low_floor, high_floor)
}

pub(crate) fn classify_confidence(
    score: u8,
    medium_floor: u8,
    high_floor: u8,
    supported: bool,
    degraded: bool,
) -> HybridRuntimeConfidence {
    if !supported || score < medium_floor {
        return HybridRuntimeConfidence::Low;
    }
    if degraded || score < high_floor {
        return HybridRuntimeConfidence::Medium;
    }
    HybridRuntimeConfidence::High
}

pub(crate) fn classify_performance(
    backend: BackendPreference,
    score: u8,
    low_floor: u8,
    high_floor: u8,
) -> HybridPerformanceTier {
    if score < low_floor {
        return HybridPerformanceTier::Low;
    }
    match backend {
        BackendPreference::ReactOsFirst => HybridPerformanceTier::Low,
        BackendPreference::DriverKitFirst => HybridPerformanceTier::Medium,
        BackendPreference::LibLinuxFirst | BackendPreference::SideCarFirst => {
            if score >= high_floor {
                HybridPerformanceTier::High
            } else {
                HybridPerformanceTier::Medium
            }
        }
    }
}

pub(crate) fn classify_security(
    backend: BackendPreference,
    degraded: bool,
) -> (HybridSecurityPosture, &'static str) {
    match (backend, degraded) {
        (BackendPreference::SideCarFirst, false) => (
            HybridSecurityPosture::Isolated,
            "strong isolation boundary with contained fault domain",
        ),
        (BackendPreference::SideCarFirst, true) => (
            HybridSecurityPosture::Mediated,
            "isolation present but runtime health indicates reduced assurance",
        ),
        (BackendPreference::LibLinuxFirst, false) => (
            HybridSecurityPosture::Mediated,
            "mature mediation path but shared-kernel semantics increase blast radius",
        ),
        (BackendPreference::LibLinuxFirst, true) => (
            HybridSecurityPosture::CompatibilityRisk,
            "degraded bridge path can expose semantic mismatches under fault",
        ),
        (BackendPreference::DriverKitFirst, false) => (
            HybridSecurityPosture::Mediated,
            "user-mode execution improves containment but IPC/broker surface stays wide",
        ),
        (BackendPreference::DriverKitFirst, true) => (
            HybridSecurityPosture::CompatibilityRisk,
            "health degradation indicates instability in user-mode mediation layer",
        ),
        (BackendPreference::ReactOsFirst, _) => (
            HybridSecurityPosture::CompatibilityRisk,
            "compatibility translation path carries semantic and hardening gaps",
        ),
    }
}

pub(crate) fn performance_score_for(backend: BackendPreference, tier: HybridPerformanceTier) -> u8 {
    let caps = backend_capabilities(backend);
    let tier_bias = match tier {
        HybridPerformanceTier::Low => -18,
        HybridPerformanceTier::Medium => 0,
        HybridPerformanceTier::High => 18,
    };

    (42 + caps.throughput * 2 + caps.native_semantics + tier_bias).clamp(0, 100) as u8
}

pub(crate) fn security_score_for(backend: BackendPreference, posture: HybridSecurityPosture) -> u8 {
    let caps = backend_capabilities(backend);
    let posture_bias = match posture {
        HybridSecurityPosture::Isolated => 18,
        HybridSecurityPosture::Mediated => 4,
        HybridSecurityPosture::CompatibilityRisk => -12,
    };

    (40 + caps.security_containment * 2 + caps.isolation + posture_bias).clamp(0, 100) as u8
}

pub(crate) fn maturity_score_to_gap(score: u8, peer_scores: &[u8]) -> HybridGapSeverity {
    if peer_scores.is_empty() {
        return HybridGapSeverity::Critical;
    }

    let mean = score_mean(peer_scores);
    let spread = score_spread(peer_scores);
    let info_floor = mean.saturating_add(spread / 6).min(100);
    let warning_floor = mean.saturating_sub(spread / 8);

    if score >= info_floor {
        HybridGapSeverity::Info
    } else if score >= warning_floor {
        HybridGapSeverity::Warning
    } else {
        HybridGapSeverity::Critical
    }
}

pub(crate) fn maturity_overall_score(scores: &[u8]) -> u8 {
    if scores.is_empty() {
        return 0;
    }

    (scores.iter().map(|score| *score as usize).sum::<usize>() / scores.len()) as u8
}

pub(crate) fn score_mean(scores: &[u8]) -> u8 {
    maturity_overall_score(scores)
}

pub(crate) fn score_spread(scores: &[u8]) -> u8 {
    if scores.is_empty() {
        return 0;
    }

    let mut min_score = u8::MAX;
    let mut max_score = u8::MIN;
    for score in scores {
        min_score = min_score.min(*score);
        max_score = max_score.max(*score);
    }
    max_score.saturating_sub(min_score)
}

pub(crate) fn adaptive_threshold(scores: &[u8]) -> u8 {
    if scores.is_empty() {
        return 0;
    }

    let mean = score_mean(scores);
    let spread = score_spread(scores);
    mean.saturating_add(spread / 8).min(100)
}

pub(crate) fn composite_quality_score(coverage: u8, performance: u8, security: u8) -> u8 {
    (((coverage as u16 * 2) + performance as u16 + security as u16) / 4) as u8
}