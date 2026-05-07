use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;
use crate::modules::drivers::hybrid::orchestrator::{BackendPreference, HybridRequest, HybridRequestFamily, HybridRequestKind};
use super::super::request_family;
use super::caps::{observed_backend_capabilities, BackendCapabilities};
use super::shapes::{observed_request_shape, RequestShape};

pub fn semantic_bias(request_kind: HybridRequestKind, backend: BackendPreference) -> i16 {
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

pub fn apply_delta(score: u8, delta: i16) -> u8 {
    (score as i16 + delta).clamp(0, 100) as u8
}

pub fn score_from_fit(raw_fit: i16) -> u8 {
    ((raw_fit + 120).clamp(0, 240) * 100 / 240) as u8
}

pub fn shape_fit_score_with_caps(caps: BackendCapabilities, shape: RequestShape) -> i16 {
    caps.isolation * shape.security_sensitivity
        + caps.native_semantics * shape.compatibility_pressure
        + caps.mediation * shape.control_plane_pressure
        + caps.compatibility * shape.compatibility_pressure
        + caps.throughput * shape.bandwidth_pressure
        + caps.security_containment * shape.security_sensitivity
        - shape.dma_intensity * 2
        - shape.latency_sensitivity * 2
}

pub fn request_kind_hint(kind: HybridRequestKind) -> &'static str {
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
        _ => "peripheral management path",
    }
}

pub fn support_reason(
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

pub fn score_backend_support_raw(
    request: &HybridRequest,
    backend: BackendPreference,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> (u8, bool, &'static str) {
    let family = request_family(request.kind);
    let shape = observed_request_shape(request);
    let observed_caps = observed_backend_capabilities(request.kind, backend, driverkit_health);
    let mut score = score_from_fit(shape_fit_score_with_caps(observed_caps, shape));
    let mut degraded = false;
    let fallback_reason = super::family_reason(backend, family);

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
