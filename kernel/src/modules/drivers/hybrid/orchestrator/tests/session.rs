use super::*;

#[test_case]
fn session_telemetry_aware_maturity_report_is_reachable() {
    let mut session = HybridOrchestratorSession::new(8);
    session.record_sidecar_sample(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(88, 2, 2_600, 79),
    );
    session.record_liblinux_dispatch_sample(LibLinuxDispatchSample::new(8, 4, 2, 1, 1));

    let report = session.maturity_report(None);
    assert_eq!(report.findings.len(), 5);
    assert!(report.overall_score <= 100);
}

#[test_case]
fn orchestrator_session_persists_sidecar_telemetry_and_tunes_plan() {
    let request = HybridRequest::network(0x6000, 0x100, 0x9000, 0x1000, 52);
    let cfg = SideCarVmConfig::new(31, 2, 256 * 1024 * 1024);

    let baseline = HybridOrchestrator::plan_with_sidecar_telemetry(
        &request,
        BackendPreference::SideCarFirst,
        cfg,
        None,
    )
    .expect("baseline plan should exist");

    let mut session = HybridOrchestratorSession::new(8);
    session.record_sidecar_sample(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(90, 3, 2_800, 96),
    );
    session.record_sidecar_sample(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(82, 2, 2_200, 80),
    );

    let tuned = session
        .plan(&request, BackendPreference::SideCarFirst, cfg)
        .expect("tuned plan should exist");

    let (baseline_data, tuned_data) = match (baseline, tuned) {
        (HybridExecutionPlan::SideCar(base), HybridExecutionPlan::SideCar(tuned)) => {
            (base.data_ring_depth, tuned.data_ring_depth)
        }
        _ => panic!("expected sidecar plans"),
    };

    assert!(tuned_data > baseline_data);
}

#[test_case]
fn orchestrator_session_diagnostics_use_telemetry_aware_path() {
    let request = HybridRequest::network(0x7000, 0x100, 0xA000, 0x1000, 61);
    let cfg = SideCarVmConfig::new(33, 2, 256 * 1024 * 1024);
    let mut session = HybridOrchestratorSession::new(4);

    session.record_sidecar_sample(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(84, 2, 2_000, 72),
    );

    let diag = session.plan_with_diagnostics(&request, BackendPreference::SideCarFirst, cfg);
    assert!(!diag.attempts.is_empty());
    assert_eq!(diag.attempts[0].backend, BackendPreference::SideCarFirst);
    assert!(matches!(diag.selected, Some(HybridExecutionPlan::SideCar(_))));
}

#[test_case]
fn orchestrator_session_adaptive_liblinux_dispatch_records_telemetry() {
    let mut queue = LinuxSyscallQueue::new(16);
    for id in 1..=6u64 {
        queue
            .push(LinuxSyscallRequest::new(id, LinuxSyscall::Write))
            .expect("queue push should work");
    }

    let mut session = HybridOrchestratorSession::new(8);
    let records = session.dispatch_liblinux_queue_to_bridge_adaptive(&mut queue, 6);
    assert!(!records.is_empty());

    let summary = session
        .liblinux_telemetry()
        .summary()
        .expect("liblinux telemetry summary should exist");
    assert!(summary.sample_count >= 1);
    assert!(summary.avg_batch_size >= 1);
}

#[test_case]
fn orchestrator_session_liblinux_recommendation_shrinks_after_failures() {
    let mut session = HybridOrchestratorSession::new(8);
    session.record_liblinux_dispatch_sample(
        crate::modules::drivers::hybrid::liblinux::LibLinuxDispatchSample::new(8, 8, 2, 1, 3),
    );
    session.record_liblinux_dispatch_sample(
        crate::modules::drivers::hybrid::liblinux::LibLinuxDispatchSample::new(8, 8, 2, 1, 3),
    );

    let recommended = session
        .liblinux_telemetry()
        .recommended_batch_size(8, 8);
    assert!(recommended < 8);
}

#[test_case]
fn orchestrator_session_sidecar_snapshot_bytes_import_export_roundtrip() {
    let mut source = HybridOrchestratorSession::new(8);
    source.record_sidecar_sample(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(91, 3, 3200, 88),
    );

    let bytes = source.export_sidecar_telemetry_bytes();
    let mut target = HybridOrchestratorSession::new(8);
    assert!(target.import_sidecar_telemetry_bytes(&bytes, 8, 8));

    assert!(target
        .sidecar_telemetry()
        .summary_for(LinuxShimDeviceKind::Network)
        .is_some());
}

#[test_case]
fn orchestrator_session_sidecar_fallback_aggressiveness_increases_with_saturation() {
    let mut session = HybridOrchestratorSession::new(8);
    session.record_sidecar_sample(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(22, 0, 800, 10),
    );
    let low = session.sidecar_fallback_aggressiveness(LinuxShimDeviceKind::Network);

    session.record_sidecar_sample(
        LinuxShimDeviceKind::Network,
        SideCarTelemetrySample::new(95, 4, 4000, 120),
    );
    let high = session.sidecar_fallback_aggressiveness(LinuxShimDeviceKind::Network);

    assert!(high > low);
}

#[test_case]
fn orchestrator_session_liblinux_family_aware_policy_uses_first_syscall_class() {
    let mut session = HybridOrchestratorSession::new(8);
    session.record_liblinux_dispatch_sample(
        LibLinuxDispatchSample::new(8, 8, 7, 1, 0),
    );
    session
        .liblinux_telemetry()
        .recommended_batch_size_for_syscall(LinuxSyscall::Write, 8, 8);

    let mut queue = LinuxSyscallQueue::new(8);
    for id in 1..=4u64 {
        queue
            .push(LinuxSyscallRequest::new(id, LinuxSyscall::Ioctl))
            .expect("queue push should work");
    }

    session.record_liblinux_dispatch_sample(
        LibLinuxDispatchSample::new(8, 8, 2, 1, 4),
    );
    let records = session.dispatch_liblinux_queue_to_bridge_adaptive(&mut queue, 8);
    assert!(!records.is_empty());

    let recommended_ioctl = session
        .liblinux_telemetry()
        .recommended_batch_size_for_syscall(LinuxSyscall::Ioctl, 8, 8);
    let recommended_write = session
        .liblinux_telemetry()
        .recommended_batch_size_for_syscall(LinuxSyscall::Write, 8, 8);
    assert!(recommended_ioctl <= recommended_write);
}

#[test_case]
fn session_userspace_abi_report_reflects_liblinux_tail_pressure() {
    let baseline = HybridOrchestrator::userspace_abi_report();
    let mut session = HybridOrchestratorSession::new(8);
    for _ in 0..12 {
        session.record_liblinux_dispatch_sample(LibLinuxDispatchSample::new(16, 4, 0, 1, 7));
    }

    let pressured = session.userspace_abi_report();
    assert!(pressured.readiness_score <= baseline.readiness_score);
    assert!(pressured.confidence_score <= baseline.confidence_score);
    assert!(pressured.telemetry_shape_score <= baseline.telemetry_shape_score);
    assert!(pressured.telemetry_samples >= 8);
    assert_ne!(
        pressured.tail_pressure_level,
        HybridUserspaceAbiTailPressureLevel::Insufficient
    );
}

#[test_case]
fn session_userspace_abi_report_rewards_balanced_telemetry() {
    let baseline = HybridOrchestrator::userspace_abi_report();
    let mut session = HybridOrchestratorSession::new(8);
    for _ in 0..16 {
        session.record_liblinux_dispatch_sample(LibLinuxDispatchSample::new(16, 16, 0, 0, 16));
    }

    let balanced = session.userspace_abi_report();
    assert!(balanced.telemetry_samples >= 16);
    assert!(balanced.confidence_score >= baseline.confidence_score);
    assert!(balanced.contract_matrix.behavior_depth_ratio >= baseline.contract_matrix.behavior_depth_ratio);
    assert!(balanced.telemetry_shape_score >= baseline.telemetry_shape_score);
}

#[test_case]
fn session_userspace_abi_report_penalizes_skewed_telemetry_shape() {
    let mut session = HybridOrchestratorSession::new(8);
    for _ in 0..14 {
        session.record_liblinux_dispatch_sample(LibLinuxDispatchSample::new(64, 1, 0, 0, 12));
    }

    let skewed = session.userspace_abi_report();
    assert!(skewed.telemetry_samples >= 8);
    assert!(skewed.telemetry_shape_score < 50);
}
