use super::*;

#[test_case]
fn telemetry_aware_planning_expands_sidecar_data_ring_under_pressure() {
    let request = HybridRequest::network(0x9000, 0x100, 0xA000, 0x1000, 14);
    let cfg = SideCarVmConfig::new(11, 2, 256 * 1024 * 1024);

    let baseline = HybridOrchestrator::plan_with_sidecar_telemetry(
        &request,
        BackendPreference::SideCarFirst,
        cfg,
        None,
    )
    .expect("baseline sidecar plan should exist");

    let mut telemetry = SideCarTelemetryStore::new(8);
    telemetry.record(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(92, 3, 3_000, 88),
    );
    telemetry.record(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(85, 2, 2_400, 72),
    );

    let tuned = HybridOrchestrator::plan_with_sidecar_telemetry(
        &request,
        BackendPreference::SideCarFirst,
        cfg,
        Some(&telemetry),
    )
    .expect("tuned sidecar plan should exist");

    let (baseline_data_ring, tuned_data_ring) = match (baseline, tuned) {
        (HybridExecutionPlan::SideCar(base), HybridExecutionPlan::SideCar(tuned)) => {
            (base.data_ring_depth, tuned.data_ring_depth)
        }
        _ => panic!("expected sidecar plans"),
    };

    assert!(tuned_data_ring > baseline_data_ring);
}

#[test_case]
fn diagnostics_with_telemetry_preserves_attempt_tracking() {
    let request = HybridRequest::network(0xA100, 0x100, 0xB100, 0x1000, 33);
    let cfg = SideCarVmConfig::new(12, 2, 256 * 1024 * 1024);
    let mut telemetry = SideCarTelemetryStore::new(4);
    telemetry.record(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(80, 2, 2_000, 64),
    );

    let diag = HybridOrchestrator::plan_with_diagnostics_and_telemetry(
        &request,
        BackendPreference::SideCarFirst,
        cfg,
        Some(&telemetry),
        None,
    );

    assert!(!diag.attempts.is_empty());
    assert_eq!(diag.attempts[0].backend, BackendPreference::SideCarFirst);
    assert!(matches!(diag.selected, Some(HybridExecutionPlan::SideCar(_))));
}

#[test_case]
fn telemetry_aware_coverage_and_fleet_adjust_network_family_scores() {
    let baseline_coverage = HybridOrchestrator::coverage_audit(None);
    let baseline_fleet = HybridOrchestrator::fleet_report(None);

    let mut sidecar_telemetry = SideCarTelemetryStore::new(8);
    sidecar_telemetry.record(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(95, 4, 3_500, 91),
    );

    let tuned_coverage = HybridOrchestrator::coverage_audit_with_telemetry(
        None,
        Some(&sidecar_telemetry),
        None,
    );
    let tuned_fleet = HybridOrchestrator::fleet_report_with_telemetry(
        None,
        Some(&sidecar_telemetry),
        None,
    );

    let baseline_network = baseline_coverage
        .rows
        .iter()
        .find(|row| row.request_kind == HybridRequestKind::Network)
        .expect("baseline network row should exist")
        .coverage_score;
    let tuned_network = tuned_coverage
        .rows
        .iter()
        .find(|row| row.request_kind == HybridRequestKind::Network)
        .expect("tuned network row should exist")
        .coverage_score;

    let baseline_family = baseline_fleet
        .families
        .iter()
        .find(|row| row.family == HybridRequestFamily::Network)
        .expect("baseline network family should exist")
        .coverage_score;
    let tuned_family = tuned_fleet
        .families
        .iter()
        .find(|row| row.family == HybridRequestFamily::Network)
        .expect("tuned network family should exist")
        .coverage_score;

    assert!(tuned_network <= baseline_network);
    assert!(tuned_family <= baseline_family);
}

#[test_case]
fn support_report_with_telemetry_penalizes_sidecar_under_pressure() {
    let request = HybridRequest::network(0xA500, 0x100, 0xB500, 0x1000, 51);
    let baseline = HybridOrchestrator::support_report(&request, None);

    let mut sidecar_telemetry = SideCarTelemetryStore::new(8);
    sidecar_telemetry.record(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(96, 4, 4_000, 95),
    );
    sidecar_telemetry.record(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(94, 3, 3_800, 92),
    );

    let tuned = HybridOrchestrator::support_report_with_telemetry(
        &request,
        None,
        Some(&sidecar_telemetry),
        None,
    );

    let baseline_sidecar = baseline
        .entries
        .iter()
        .find(|entry| entry.backend == BackendPreference::SideCarFirst)
        .expect("baseline sidecar entry should exist")
        .score;
    let tuned_sidecar = tuned
        .entries
        .iter()
        .find(|entry| entry.backend == BackendPreference::SideCarFirst)
        .expect("tuned sidecar entry should exist")
        .score;

    assert!(tuned_sidecar < baseline_sidecar);
}
