use alloc::vec::Vec;

use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;
use crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore;
use crate::modules::drivers::hybrid::sidecar::SideCarTelemetryStore;

use super::shared::{
    classify_confidence, classify_performance, classify_security, confidence_cutoffs_for_scores,
    feature_coverage_stats_for, performance_cutoffs_for_scores, score_backend_support_raw,
    support_cutoff_for_scores,
};
use crate::modules::drivers::hybrid::orchestrator::{
    BackendPreference, HybridBackendAssessment, HybridRuntimeAssessmentReport, HybridSupportReport,
    HybridRequest,
    HybridBackendSupport,
};
use super::super::routing::{
    adaptive_fallback_order_with_health,
    fallback_order,
};

fn fallback_rank_delta(order: [BackendPreference; 4], backend: BackendPreference) -> i16 {
    match order.iter().position(|candidate| *candidate == backend).unwrap_or(3) {
        0 => 3,
        1 => 1,
        2 => -1,
        _ => -3,
    }
}

fn telemetry_adjusted_support_score(
    backend: BackendPreference,
    base_score: u8,
    request: &HybridRequest,
    driverkit_health: Option<DriverKitHealthSnapshot>,
    sidecar_telemetry: Option<&SideCarTelemetryStore>,
    liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
) -> u8 {
    let order = adaptive_fallback_order_with_health(
        BackendPreference::SideCarFirst,
        request.kind,
        sidecar_telemetry,
        liblinux_telemetry,
        driverkit_health,
    );
    (base_score as i16 + fallback_rank_delta(order, backend)).clamp(0, 100) as u8
}

pub fn support_report(
    request: &HybridRequest,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> HybridSupportReport {
    support_report_with_telemetry(request, driverkit_health, None, None)
}

pub fn support_report_with_telemetry(
    request: &HybridRequest,
    driverkit_health: Option<DriverKitHealthSnapshot>,
    sidecar_telemetry: Option<&SideCarTelemetryStore>,
    liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
) -> HybridSupportReport {
    let order = if sidecar_telemetry.is_some() || liblinux_telemetry.is_some() {
        adaptive_fallback_order_with_health(
            BackendPreference::SideCarFirst,
            request.kind,
            sidecar_telemetry,
            liblinux_telemetry,
            driverkit_health,
        )
    } else {
        fallback_order(BackendPreference::SideCarFirst)
    };
    let mut raw = Vec::new();

    for backend in order {
        let (score, degraded, reason) = score_backend_support_raw(request, backend, driverkit_health);
        let score = telemetry_adjusted_support_score(
            backend,
            score,
            request,
            driverkit_health,
            sidecar_telemetry,
            liblinux_telemetry,
        );
        raw.push((backend, score, degraded, reason));
    }

    let scores = raw.iter().map(|entry| entry.1).collect::<Vec<_>>();
    let support_cutoff = support_cutoff_for_scores(&scores);
    let top_score = scores.iter().copied().max().unwrap_or(0);
    let mut entries = Vec::new();

    for (backend, score, degraded, reason) in raw {
        let coverage = feature_coverage_stats_for(request.kind, backend, driverkit_health);
        let mandatory_ready = coverage.mandatory_ready();
        let feature_ready = coverage.required_ready();
        let supported = (score >= support_cutoff || score == top_score) && feature_ready && mandatory_ready;
        let reason = if supported {
            reason
        } else if !mandatory_ready {
            "missing mandatory backend capabilities for request path"
        } else if !feature_ready {
            "insufficient required capability coverage for request path"
        } else if degraded {
            "backend capability posture is degraded for this request"
        } else {
            reason
        };
        entries.push(HybridBackendSupport {
            backend,
            score,
            supported,
            degraded,
            reason,
        });
    }

    let recommended = entries
        .iter()
        .filter(|entry| entry.supported)
        .max_by_key(|entry| entry.score)
        .map(|entry| entry.backend)
        .unwrap_or(BackendPreference::LibLinuxFirst);

    HybridSupportReport {
        request_kind: request.kind,
        entries,
        recommended,
    }
}

pub fn runtime_assessment(
    request: &HybridRequest,
    driverkit_health: Option<DriverKitHealthSnapshot>,
) -> HybridRuntimeAssessmentReport {
    runtime_assessment_with_telemetry(request, driverkit_health, None, None)
}

pub fn runtime_assessment_with_telemetry(
    request: &HybridRequest,
    driverkit_health: Option<DriverKitHealthSnapshot>,
    sidecar_telemetry: Option<&SideCarTelemetryStore>,
    liblinux_telemetry: Option<&LibLinuxTelemetryStore>,
) -> HybridRuntimeAssessmentReport {
    let support = support_report_with_telemetry(
        request,
        driverkit_health,
        sidecar_telemetry,
        liblinux_telemetry,
    );
    let mut assessments = Vec::new();
    let scores = support
        .entries
        .iter()
        .map(|entry| entry.score)
        .collect::<Vec<_>>();
    let (medium_floor, high_floor) = confidence_cutoffs_for_scores(&scores);
    let (low_floor, perf_high_floor) = performance_cutoffs_for_scores(&scores);

    for entry in support.entries.iter().copied() {
        let confidence = classify_confidence(
            entry.score,
            medium_floor,
            high_floor,
            entry.supported,
            entry.degraded,
        );
        let performance = classify_performance(entry.backend, entry.score, low_floor, perf_high_floor);
        let (security, risk) = classify_security(entry.backend, entry.degraded);

        assessments.push(HybridBackendAssessment {
            backend: entry.backend,
            supported: entry.supported,
            confidence,
            performance,
            security,
            risk,
            notes: entry.reason,
        });
    }

    HybridRuntimeAssessmentReport {
        request_kind: support.request_kind,
        recommended: support.recommended,
        assessments,
    }
}