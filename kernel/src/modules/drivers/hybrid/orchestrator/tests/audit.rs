use super::*;

#[test_case]
fn coverage_audit_reports_all_request_kinds_supported() {
    let audit = HybridOrchestrator::coverage_audit(None);
    let row_scores = audit.rows.iter().map(|row| row.coverage_score).collect::<Vec<_>>();
    let min_score = row_scores.iter().copied().min().expect("coverage rows should not be empty");
    let max_score = row_scores.iter().copied().max().expect("coverage rows should not be empty");

    assert!(audit.all_requests_supported);
    assert_eq!(audit.rows.len(), 29);
    assert!(audit.overall_score >= min_score);
    assert!(audit.overall_score <= max_score);
}

#[test_case]
fn coverage_recommended_backend_is_always_supported() {
    let audit = HybridOrchestrator::coverage_audit(None);
    assert!(audit
        .rows
        .iter()
        .all(|row| row.supported_backends.contains(&row.recommended)));
}

#[test_case]
fn support_report_recommended_backend_has_nontrivial_feature_coverage() {
    let request = HybridRequest::camera(0x9A00, 0x100, 0xBA00, 0x1000, 98);
    let support = HybridOrchestrator::support_report(&request, None);
    let feature = HybridOrchestrator::feature_audit(None);

    let recommended_feature_row = feature
        .rows
        .iter()
        .find(|row| row.request_kind == request.kind && row.backend == support.recommended)
        .expect("recommended feature row should exist");

    assert!(recommended_feature_row.feature_score >= 50);
}

#[test_case]
fn support_report_recommended_backend_supported_for_all_requests() {
    let audit = HybridOrchestrator::coverage_audit(None);
    for row in audit.rows {
        assert!(
            row.supported_backends.contains(&row.recommended),
            "recommended backend must be supported for {:?}",
            row.request_kind
        );
    }
}

#[test_case]
fn feature_audit_covers_all_backends_for_each_request() {
    let audit = HybridOrchestrator::feature_audit(None);
    let row_scores = audit.rows.iter().map(|row| row.feature_score).collect::<Vec<_>>();
    let min_score = row_scores.iter().copied().min().expect("feature rows should not be empty");
    let max_score = row_scores.iter().copied().max().expect("feature rows should not be empty");

    assert_eq!(audit.rows.len(), 116);
    assert!(audit.overall_feature_score >= min_score);
    assert!(audit.overall_feature_score <= max_score);
}

#[test_case]
fn feature_audit_reports_missing_features_for_degraded_paths() {
    let audit = HybridOrchestrator::feature_audit(None);
    assert!(audit.rows.iter().any(|row| !row.missing_features.is_empty()));
    assert!(audit.rows.iter().any(|row| row.feature_score < 100));
}

#[test_case]
fn coverage_audit_requires_fallback_for_runtime_paths() {
    let audit = HybridOrchestrator::coverage_audit(None);
    let network = audit
        .rows
        .iter()
        .find(|row| row.request_kind == HybridRequestKind::Network)
        .expect("network row should exist");
    assert!(network.has_fallback);
    assert!(network.supported_backends.len() >= 2);
}
