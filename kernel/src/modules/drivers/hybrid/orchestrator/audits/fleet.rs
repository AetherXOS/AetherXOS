use alloc::vec::Vec;

use crate::modules::drivers::hybrid::driverkit::DriverKitHealthSnapshot;

use super::shared::{
    adaptive_threshold, composite_quality_score, performance_score_for, request_family,
    security_score_for, synthetic_coverage_request, ALL_REQUEST_KINDS,
};
use crate::modules::drivers::hybrid::orchestrator::{
    BackendPreference, HybridBackendFleetStatus, HybridFamilyFleetStatus, HybridFleetReport,
    HybridRequestFamily, HybridRuntimeConfidence, HybridSecurityPosture,
};
use super::super::routing::fallback_order;
use super::support::{runtime_assessment, support_report};

pub fn fleet_report(driverkit_health: Option<DriverKitHealthSnapshot>) -> HybridFleetReport {
    let mut backends = Vec::new();
    let mut families = Vec::new();

    for family in [
        HybridRequestFamily::Network,
        HybridRequestFamily::Storage,
        HybridRequestFamily::Multimedia,
        HybridRequestFamily::Input,
        HybridRequestFamily::Security,
        HybridRequestFamily::Platform,
        HybridRequestFamily::Compatibility,
        HybridRequestFamily::Peripheral,
    ] {
        let mut supported_request_kinds = 0usize;
        let mut unsupported_request_kinds = 0usize;
        let mut high_risk_paths = 0usize;
        let mut coverage_score_sum = 0usize;
        let mut performance_score_sum = 0usize;
        let mut security_score_sum = 0usize;
        let mut family_requests = 0usize;

        for (index, request_kind) in ALL_REQUEST_KINDS.iter().copied().enumerate() {
            if request_family(request_kind) != family {
                continue;
            }

            family_requests += 1;
            let request = synthetic_coverage_request(request_kind, index);
            let report = support_report(&request, driverkit_health);
            let runtime = runtime_assessment(&request, driverkit_health);
            let assessment = runtime
                .assessments
                .iter()
                .find(|entry| entry.backend == report.recommended)
                .copied()
                .expect("runtime assessment entry should exist for recommended backend");

            let support_count = report.entries.iter().filter(|entry| entry.supported).count();
            let average_score = if report.entries.is_empty() {
                0
            } else {
                (report.entries.iter().map(|entry| entry.score as usize).sum::<usize>()
                    / report.entries.len()) as u8
            };

            coverage_score_sum += average_score as usize;
            let effective_backend = report.recommended;
            performance_score_sum += performance_score_for(effective_backend, assessment.performance) as usize;
            security_score_sum += security_score_for(effective_backend, assessment.security) as usize;

            if support_count > 0 {
                supported_request_kinds += 1;
            } else {
                unsupported_request_kinds += 1;
            }

            if matches!(assessment.confidence, HybridRuntimeConfidence::Low)
                || matches!(assessment.security, HybridSecurityPosture::CompatibilityRisk)
            {
                high_risk_paths += 1;
            }
        }

        let request_count = family_requests.max(1);
        let coverage_score = (coverage_score_sum / request_count) as u8;
        let performance_score = (performance_score_sum / request_count) as u8;
        let security_score = (security_score_sum / request_count) as u8;

        families.push(HybridFamilyFleetStatus {
            family,
            coverage_score,
            performance_score,
            security_score,
            supported_request_kinds,
            unsupported_request_kinds,
            high_risk_paths,
            ready: false,
        });
    }

    for backend in fallback_order(BackendPreference::SideCarFirst) {
        let mut supported_request_kinds = 0usize;
        let mut unsupported_request_kinds = 0usize;
        let mut high_risk_paths = 0usize;
        let mut coverage_score_sum = 0usize;
        let mut performance_score_sum = 0usize;
        let mut security_score_sum = 0usize;

        for (index, request_kind) in ALL_REQUEST_KINDS.iter().copied().enumerate() {
            let request = synthetic_coverage_request(request_kind, index);
            let support = support_report(&request, driverkit_health);
            let support_entry = support
                .entries
                .iter()
                .find(|entry| entry.backend == backend)
                .copied()
                .expect("support entry should exist for every backend");
            let runtime = runtime_assessment(&request, driverkit_health);
            let assessment = runtime
                .assessments
                .iter()
                .find(|entry| entry.backend == backend)
                .copied()
                .expect("runtime assessment entry should exist for every backend");

            coverage_score_sum += support_entry.score as usize;
            performance_score_sum += performance_score_for(backend, assessment.performance) as usize;
            security_score_sum += security_score_for(backend, assessment.security) as usize;

            if support_entry.supported {
                supported_request_kinds += 1;
            } else {
                unsupported_request_kinds += 1;
            }

            if matches!(assessment.confidence, HybridRuntimeConfidence::Low)
                || matches!(assessment.security, HybridSecurityPosture::CompatibilityRisk)
            {
                high_risk_paths += 1;
            }
        }

        let request_count = ALL_REQUEST_KINDS.len().max(1);
        let coverage_score = (coverage_score_sum / request_count) as u8;
        let performance_score = (performance_score_sum / request_count) as u8;
        let security_score = (security_score_sum / request_count) as u8;

        backends.push(HybridBackendFleetStatus {
            backend,
            coverage_score,
            performance_score,
            security_score,
            supported_request_kinds,
            unsupported_request_kinds,
            high_risk_paths,
            ready: false,
        });
    }

    let family_composite_scores = families
        .iter()
        .map(|status| {
            composite_quality_score(status.coverage_score, status.performance_score, status.security_score)
        })
        .collect::<Vec<_>>();
    let family_ready_cutoff = adaptive_threshold(&family_composite_scores);
    for family in &mut families {
        family.ready = composite_quality_score(family.coverage_score, family.performance_score, family.security_score)
            >= family_ready_cutoff;
    }

    let backend_composite_scores = backends
        .iter()
        .map(|status| {
            composite_quality_score(status.coverage_score, status.performance_score, status.security_score)
        })
        .collect::<Vec<_>>();
    let backend_ready_cutoff = adaptive_threshold(&backend_composite_scores);
    for backend_status in &mut backends {
        backend_status.ready = composite_quality_score(
            backend_status.coverage_score,
            backend_status.performance_score,
            backend_status.security_score,
        ) >= backend_ready_cutoff;
    }

    let most_ready_backend = backends
        .iter()
        .filter(|status| status.ready)
        .max_by_key(|status| {
            status
                .coverage_score
                .saturating_add(status.performance_score)
                .saturating_add(status.security_score)
        })
        .map(|status| status.backend)
        .unwrap_or(BackendPreference::SideCarFirst);

    let least_ready_backend = backends
        .iter()
        .min_by_key(|status| {
            status
                .coverage_score
                .saturating_add(status.performance_score)
                .saturating_add(status.security_score)
        })
        .map(|status| status.backend)
        .unwrap_or(BackendPreference::ReactOsFirst);

    let overall_ready = backends.iter().any(|status| status.ready);

    HybridFleetReport {
        backends,
        families,
        most_ready_backend,
        least_ready_backend,
        overall_ready,
    }
}