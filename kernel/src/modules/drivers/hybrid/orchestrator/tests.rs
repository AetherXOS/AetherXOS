use alloc::vec::Vec;

use super::*;
use crate::modules::drivers::hybrid::linux::LinuxShimDeviceKind;
use crate::modules::drivers::hybrid::reactos::{NtSymbol, NtSymbolTable};
use crate::modules::drivers::hybrid::sidecar::{
    SideCarTelemetrySample, SideCarTelemetryStore,
};
use crate::modules::drivers::hybrid::{
    InMemorySideCarTransport, LinuxBridgeMessage, LinuxBridgeMessageKind, LinuxBridgePayload,
    LinuxSyscall, LinuxSyscallRequest, SideCarBootstrapPhase, SideCarRetryPolicy,
    ZeroCopyIoPolicy,
};

fn sample_pe() -> Vec<u8> {
    let mut image = vec![0u8; 0x400];
    image[0] = 0x4D;
    image[1] = 0x5A;
    image[0x3C..0x40].copy_from_slice(&(0x80u32).to_le_bytes());
    image[0x80..0x84].copy_from_slice(&0x0000_4550u32.to_le_bytes());

    let file_header = 0x84;
    image[file_header..file_header + 2].copy_from_slice(&0x8664u16.to_le_bytes());
    image[file_header + 2..file_header + 4].copy_from_slice(&1u16.to_le_bytes());
    image[file_header + 16..file_header + 18].copy_from_slice(&0xF0u16.to_le_bytes());

    let optional = file_header + 20;
    image[optional..optional + 2].copy_from_slice(&0x20Bu16.to_le_bytes());
    image[optional + 16..optional + 20].copy_from_slice(&0x1000u32.to_le_bytes());
    image[optional + 24..optional + 32].copy_from_slice(&0x140000000u64.to_le_bytes());
    image[optional + 56..optional + 60].copy_from_slice(&0x4000u32.to_le_bytes());
    image[optional + 60..optional + 64].copy_from_slice(&0x400u32.to_le_bytes());

    let section = optional + 0xF0;
    image[section + 8..section + 12].copy_from_slice(&0x200u32.to_le_bytes());
    image[section + 12..section + 16].copy_from_slice(&0x1000u32.to_le_bytes());
    image[section + 16..section + 20].copy_from_slice(&0x200u32.to_le_bytes());
    image[section + 20..section + 24].copy_from_slice(&0x200u32.to_le_bytes());

    image
}

#[test_case]
fn chooses_sidecar_for_network_when_requested() {
    let request = HybridRequest::network(0x1000, 0x100, 0x2000, 0x2000, 33);
    let cfg = SideCarVmConfig::new(1, 1, 128 * 1024 * 1024);
    let plan = HybridOrchestrator::plan(&request, BackendPreference::SideCarFirst, cfg)
        .expect("plan should exist");
    match plan {
        HybridExecutionPlan::SideCar(_) => {}
        _ => panic!("expected sidecar plan"),
    }
}

#[test_case]
fn parses_windows_pe_plan() {
    let image = sample_pe();
    let plan = HybridOrchestrator::plan_windows_pe(&image, BackendPreference::ReactOsFirst)
        .expect("PE should parse");
    match plan {
        HybridExecutionPlan::ReactOs { policy, .. } => {
            assert_eq!(policy.mode, super::super::reactos::NtBinaryExecutionMode::NativeKernel)
        }
        _ => panic!("expected reactos plan"),
    }
}

#[test_case]
fn health_adaptation_falls_back_from_driverkit() {
    let request = HybridRequest::network(0x1000, 0x100, 0x2000, 0x2000, 33);
    let cfg = SideCarVmConfig::new(1, 1, 64 * 1024 * 1024);
    let health = DriverKitHealthSnapshot {
        class_count: 2,
        binding_count: 2,
        started_count: 1,
        faulted_count: 0,
        quarantined_count: 1,
        dispatch_success_count: 0,
        dispatch_failure_count: 3,
    };

    let plan = HybridOrchestrator::plan_with_driverkit_health(
        &request,
        BackendPreference::DriverKitFirst,
        cfg,
        health,
    )
    .expect("plan should exist");

    assert!(matches!(plan, HybridExecutionPlan::SideCar(_)));
}

#[test_case]
fn plans_windows_pe_with_import_bindings() {
    let image = sample_pe();
    let mut symbols = NtSymbolTable::new();
    symbols.register(NtSymbol::IoCallDriver, 0x1234_5678);
    let result = HybridOrchestrator::plan_windows_pe_with_symbols(&image, &symbols)
        .expect("PE plan should parse");
    assert_eq!(result.image_info.machine, 0x8664);
    assert_eq!(result.policy.mode, super::super::reactos::NtBinaryExecutionMode::WineHostBridge);
    assert_eq!(result.domain_bindings.len(), 1);
    assert_eq!(result.counts.kernel, 1);
}

#[test_case]
fn builds_sidecar_bootstrap_frames() {
    let request = HybridRequest::network(0x1000, 0x100, 0x2000, 0x2000, 40);
    let cfg = SideCarVmConfig::new(7, 2, 256 * 1024 * 1024);
    let frames = HybridOrchestrator::build_sidecar_bootstrap_frames(&request, cfg, 900)
        .expect("frames should be produced");
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].header.request_id, 900);
    assert_eq!(frames[1].header.request_id, 901);
}

#[test_case]
fn submits_sidecar_bootstrap_frames_into_transport() {
    let request = HybridRequest::network(0x1000, 0x100, 0x2000, 0x2000, 55);
    let cfg = SideCarVmConfig::new(9, 2, 128 * 1024 * 1024);
    let frames = HybridOrchestrator::build_sidecar_bootstrap_frames(&request, cfg, 77)
        .expect("frames should be produced");

    let mut transport = InMemorySideCarTransport::new();
    HybridOrchestrator::submit_sidecar_frames(&mut transport, &frames)
        .expect("submit should work");

    let first = transport.pop_wire_frame().expect("first frame exists");
    let second = transport.pop_wire_frame().expect("second frame exists");
    assert_eq!(first.0.request_id, 77);
    assert_eq!(second.0.request_id, 78);
}

#[test_case]
fn drives_sidecar_bootstrap_state_machine() {
    let mut transport = InMemorySideCarTransport::new();
    let mut state = SideCarBootstrapState::new(500, SideCarRetryPolicy::conservative());

    let sent = HybridOrchestrator::drive_sidecar_bootstrap(&mut transport, &mut state, 4, 128, 0)
        .expect("drive should succeed");
    assert!(sent);
    state.mark_success();

    let sent = HybridOrchestrator::drive_sidecar_bootstrap(&mut transport, &mut state, 4, 128, 1)
        .expect("drive should succeed");
    assert!(sent);
    state.mark_success();

    assert_eq!(state.phase, SideCarBootstrapPhase::Completed);
}

#[test_case]
fn dispatches_liblinux_queue_to_bridge_records() {
    let mut queue = LinuxSyscallQueue::new(4);
    queue
        .push(LinuxSyscallRequest::new(8, LinuxSyscall::Write))
        .expect("queue push should work");

    let records = HybridOrchestrator::dispatch_liblinux_queue_to_bridge(&mut queue, 4);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].request_message.header.request_id, 8);
}

#[test_case]
fn liblinux_conformance_report_is_exposed_via_orchestrator() {
    let requests = vec![
        LinuxSyscallRequest::new(80, LinuxSyscall::Write)
            .with_policy(ZeroCopyIoPolicy::Required),
        LinuxSyscallRequest::new(81, LinuxSyscall::Ioctl)
            .with_policy(ZeroCopyIoPolicy::Required),
    ];

    let report = HybridOrchestrator::liblinux_conformance_report(&requests);
    assert_eq!(report.total_requests, 2);
    assert_eq!(report.zero_copy_required, 2);
    assert!(report.high_risk_ops >= 1);
}

#[test_case]
fn advances_sidecar_bootstrap_from_bridge_completion() {
    let mut state = SideCarBootstrapState::new(71, SideCarRetryPolicy::conservative());
    let message = LinuxBridgeMessage::new(
        LinuxBridgeMessageKind::QueryStatus,
        71,
        LinuxBridgePayload::Completion(super::super::DriverCompletion::ok(71, 0)),
    );

    let advanced = HybridOrchestrator::advance_sidecar_bootstrap_from_bridge_message(
        &mut state,
        &message,
        0,
    );
    assert!(advanced);
    assert_eq!(state.phase, SideCarBootstrapPhase::ControlNotify);
}

#[test_case]
fn fallback_planning_recovers_for_user_mode_device() {
    let request = HybridRequest::user_mode_device(0x8000, 0x1000, 12);
    let cfg = SideCarVmConfig::new(1, 1, 64 * 1024 * 1024);
    let plan = HybridOrchestrator::plan_with_fallbacks(
        &request,
        BackendPreference::SideCarFirst,
        cfg,
    )
    .expect("fallback should eventually plan user mode");

    assert!(matches!(plan, HybridExecutionPlan::SideCar(_)));
}

#[test_case]
fn diagnostics_capture_attempt_order_and_selection() {
    let request = HybridRequest::user_mode_device(0x9000, 0x1000, 14);
    let cfg = SideCarVmConfig::new(2, 1, 64 * 1024 * 1024);
    let diag = HybridOrchestrator::plan_with_diagnostics(
        &request,
        BackendPreference::SideCarFirst,
        cfg,
    );

    assert!(diag.attempts.len() >= 1);
    assert_eq!(diag.attempts[0].backend, BackendPreference::SideCarFirst);
    assert!(diag.attempts[0].matched);
    assert!(matches!(diag.selected, Some(HybridExecutionPlan::SideCar(_))));
}

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
    );

    assert!(!diag.attempts.is_empty());
    assert_eq!(diag.attempts[0].backend, BackendPreference::SideCarFirst);
    assert!(matches!(diag.selected, Some(HybridExecutionPlan::SideCar(_))));
}

#[test_case]
fn support_report_prefers_reactos_for_windows_pe() {
    let request = HybridRequest::windows_pe();
    let report = HybridOrchestrator::support_report(&request, None);
    assert_eq!(report.recommended, BackendPreference::ReactOsFirst);
    assert!(report.entries.iter().any(|entry| {
        entry.backend == BackendPreference::ReactOsFirst && entry.supported
    }));
}

#[test_case]
fn support_report_degrades_driverkit_when_health_is_faulty() {
    let request = HybridRequest::user_mode_device(0xA000, 0x1000, 16);
    let health = DriverKitHealthSnapshot {
        class_count: 1,
        binding_count: 1,
        started_count: 0,
        faulted_count: 1,
        quarantined_count: 1,
        dispatch_success_count: 1,
        dispatch_failure_count: 8,
    };

    let report = HybridOrchestrator::support_report(&request, Some(health));
    let driverkit = report
        .entries
        .iter()
        .find(|entry| entry.backend == BackendPreference::DriverKitFirst)
        .expect("driverkit entry should exist");
    assert!(driverkit.degraded);
}

#[test_case]
fn support_report_driverkit_score_drops_with_faulty_health() {
    let request = HybridRequest::user_mode_device(0xA100, 0x1000, 17);
    let healthy = HybridOrchestrator::support_report(&request, None);
    let faulty = HybridOrchestrator::support_report(
        &request,
        Some(DriverKitHealthSnapshot {
            class_count: 1,
            binding_count: 1,
            started_count: 0,
            faulted_count: 2,
            quarantined_count: 1,
            dispatch_success_count: 0,
            dispatch_failure_count: 12,
        }),
    );

    let healthy_score = healthy
        .entries
        .iter()
        .find(|entry| entry.backend == BackendPreference::DriverKitFirst)
        .expect("healthy driverkit entry should exist")
        .score;
    let faulty_score = faulty
        .entries
        .iter()
        .find(|entry| entry.backend == BackendPreference::DriverKitFirst)
        .expect("faulty driverkit entry should exist")
        .score;

    assert!(faulty_score < healthy_score);
}

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
fn support_report_keeps_top_score_supported_under_adaptive_cutoff() {
    let report = HybridOrchestrator::support_report(&HybridRequest::windows_pe(), None);
    let top_score = report
        .entries
        .iter()
        .map(|entry| entry.score)
        .max()
        .expect("support entries should not be empty");

    assert!(report.entries.iter().any(|entry| entry.supported));
    assert!(report
        .entries
        .iter()
        .any(|entry| entry.supported && entry.score == top_score));
}

#[test_case]
fn support_report_recommended_backend_matches_top_supported_score() {
    let report = HybridOrchestrator::support_report(
        &HybridRequest::network(0x9000, 0x100, 0xA000, 0x1000, 88),
        None,
    );

    let recommended = report
        .entries
        .iter()
        .find(|entry| entry.backend == report.recommended)
        .expect("recommended entry should exist");
    let top_supported = report
        .entries
        .iter()
        .filter(|entry| entry.supported)
        .map(|entry| entry.score)
        .max()
        .expect("at least one supported backend should exist");

    assert!(recommended.supported);
    assert_eq!(recommended.score, top_supported);
}

#[test_case]
fn runtime_assessment_recommended_backend_is_supported() {
    let request = HybridRequest::storage(0x9100, 0x100, 0xB000, 0x1000, 89);
    let support = HybridOrchestrator::support_report(&request, None);
    let runtime = HybridOrchestrator::runtime_assessment(&request, None);

    let recommended_assessment = runtime
        .assessments
        .iter()
        .find(|entry| entry.backend == runtime.recommended)
        .expect("recommended assessment should exist");
    let recommended_support = support
        .entries
        .iter()
        .find(|entry| entry.backend == support.recommended)
        .expect("recommended support entry should exist");

    assert!(recommended_assessment.supported);
    assert!(recommended_support.supported);
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
fn support_report_storage_rejects_driverkit_without_dma_mandatory() {
    let request = HybridRequest::storage(0x9B00, 0x100, 0xBB00, 0x1000, 99);
    let report = HybridOrchestrator::support_report(&request, None);

    let driverkit = report
        .entries
        .iter()
        .find(|entry| entry.backend == BackendPreference::DriverKitFirst)
        .expect("driverkit entry should exist");

    assert!(!driverkit.supported);
}

#[test_case]
fn support_report_windows_pe_rejects_backends_without_snapshot_mandatory() {
    let report = HybridOrchestrator::support_report(&HybridRequest::windows_pe(), None);

    let liblinux = report
        .entries
        .iter()
        .find(|entry| entry.backend == BackendPreference::LibLinuxFirst)
        .expect("liblinux entry should exist");
    let driverkit = report
        .entries
        .iter()
        .find(|entry| entry.backend == BackendPreference::DriverKitFirst)
        .expect("driverkit entry should exist");

    assert!(!liblinux.supported);
    assert!(!driverkit.supported);
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
fn support_report_storage_marks_missing_mandatory_reason_for_driverkit() {
    let request = HybridRequest::storage(0x9C00, 0x100, 0xBC00, 0x1000, 100);
    let report = HybridOrchestrator::support_report(&request, None);

    let driverkit = report
        .entries
        .iter()
        .find(|entry| entry.backend == BackendPreference::DriverKitFirst)
        .expect("driverkit entry should exist");

    assert!(!driverkit.supported);
    assert_eq!(
        driverkit.reason,
        "missing mandatory backend capabilities for request path"
    );
}

#[test_case]
fn support_report_includes_gpu_and_wifi_paths() {
    let gpu = HybridOrchestrator::support_report(&HybridRequest::gpu(0x1000, 0x100, 0x2000, 0x1000, 40), None);
    let wifi = HybridOrchestrator::support_report(&HybridRequest::wifi(0x1200, 0x100, 0x2200, 0x1000, 41), None);
    let camera = HybridOrchestrator::support_report(&HybridRequest::camera(0x1400, 0x100, 0x2400, 0x1000, 42), None);
    let audio = HybridOrchestrator::support_report(&HybridRequest::audio(0x1600, 0x100, 0x2600, 0x1000, 43), None);
    let sensor = HybridOrchestrator::support_report(&HybridRequest::sensor(0x1800, 0x80, 0x2800, 0x800, 44), None);
    let input = HybridOrchestrator::support_report(&HybridRequest::input(0x1A00, 0x80, 0x2A00, 0x800, 45), None);

    assert!(gpu.entries.iter().any(|entry| entry.backend == BackendPreference::SideCarFirst && entry.supported));
    assert!(wifi.entries.iter().any(|entry| entry.backend == BackendPreference::LibLinuxFirst && entry.supported));
    assert!(camera.entries.iter().any(|entry| entry.backend == BackendPreference::SideCarFirst && entry.supported));
    assert!(audio.entries.iter().any(|entry| entry.backend == BackendPreference::LibLinuxFirst && entry.supported));
    assert!(sensor.entries.iter().any(|entry| entry.backend == BackendPreference::SideCarFirst && entry.supported));
    assert!(input.entries.iter().any(|entry| entry.backend == BackendPreference::LibLinuxFirst && entry.supported));
}

#[test_case]
fn support_report_includes_usb_serial_and_nvme_paths() {
    let usb = HybridOrchestrator::support_report(&HybridRequest::usb(0x1C00, 0x100, 0x2C00, 0x1000, 46), None);
    let serial = HybridOrchestrator::support_report(&HybridRequest::serial(0x1E00, 0x80, 0x2E00, 0x800, 47), None);
    let nvme = HybridOrchestrator::support_report(&HybridRequest::nvme(0x2000, 0x200, 0x3000, 0x2000, 48), None);

    assert!(usb.entries.iter().any(|entry| entry.backend == BackendPreference::LibLinuxFirst && entry.supported));
    assert!(serial.entries.iter().any(|entry| entry.backend == BackendPreference::LibLinuxFirst && entry.supported));
    assert!(nvme.entries.iter().any(|entry| entry.backend == BackendPreference::LibLinuxFirst && entry.supported));
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
fn support_report_includes_new_device_families() {
    let rtc = HybridOrchestrator::support_report(
        &HybridRequest::rtc(0x2050, 0x80, 0x3050, 0x800, 55),
        None,
    );
    let sensor_hub = HybridOrchestrator::support_report(
        &HybridRequest::sensor_hub(0x2060, 0x80, 0x3060, 0x800, 56),
        None,
    );
    let modem = HybridOrchestrator::support_report(
        &HybridRequest::modem(0x2100, 0x100, 0x3100, 0x1000, 57),
        None,
    );
    let printer = HybridOrchestrator::support_report(
        &HybridRequest::printer(0x2200, 0x100, 0x3200, 0x1000, 58),
        None,
    );
    let nfc = HybridOrchestrator::support_report(
        &HybridRequest::nfc(0x2300, 0x80, 0x3300, 0x800, 59),
        None,
    );
    let tpm = HybridOrchestrator::support_report(
        &HybridRequest::tpm(0x2400, 0x80, 0x3400, 0x800, 60),
        None,
    );
    let dock = HybridOrchestrator::support_report(
        &HybridRequest::dock(0x2450, 0x80, 0x3450, 0x800, 61),
        None,
    );
    let firmware = HybridOrchestrator::support_report(
        &HybridRequest::firmware(0x2500, 0x100, 0x3500, 0x1000, 62),
        None,
    );
    let smart_card = HybridOrchestrator::support_report(
        &HybridRequest::smart_card(0x2550, 0x80, 0x3550, 0x800, 63),
        None,
    );

    assert!(rtc.entries.iter().any(|entry| entry.backend == BackendPreference::SideCarFirst && entry.supported));
    assert!(sensor_hub.entries.iter().any(|entry| entry.backend == BackendPreference::SideCarFirst && entry.supported));
    assert!(modem.entries.iter().any(|entry| entry.backend == BackendPreference::LibLinuxFirst && entry.supported));
    assert!(printer.entries.iter().any(|entry| entry.backend == BackendPreference::SideCarFirst && entry.supported));
    assert!(nfc.entries.iter().any(|entry| entry.backend == BackendPreference::SideCarFirst && entry.supported));
    assert!(tpm.entries.iter().any(|entry| entry.backend == BackendPreference::DriverKitFirst && entry.supported));
    assert!(dock.entries.iter().any(|entry| entry.backend == BackendPreference::LibLinuxFirst && entry.supported));
    assert!(firmware.entries.iter().any(|entry| entry.backend == BackendPreference::LibLinuxFirst && entry.supported));
    assert!(smart_card.entries.iter().any(|entry| entry.backend == BackendPreference::DriverKitFirst && entry.supported));
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

#[test_case]
fn liblinux_plans_rtc_dock_and_smart_card_paths() {
    let cfg = SideCarVmConfig::new(3, 1, 64 * 1024 * 1024);

    let rtc = HybridRequest::rtc(0x3000, 0x80, 0x5000, 0x800, 70);
    let dock = HybridRequest::dock(0x3100, 0x80, 0x5100, 0x800, 71);
    let smart_card = HybridRequest::smart_card(0x3200, 0x80, 0x5200, 0x800, 72);

    let rtc_plan = HybridOrchestrator::plan(&rtc, BackendPreference::LibLinuxFirst, cfg)
        .expect("rtc should have a liblinux plan");
    let dock_plan = HybridOrchestrator::plan(&dock, BackendPreference::LibLinuxFirst, cfg)
        .expect("dock should have a liblinux plan");
    let smart_card_plan = HybridOrchestrator::plan(
        &smart_card,
        BackendPreference::LibLinuxFirst,
        cfg,
    )
    .expect("smart card should have a liblinux plan");

    match rtc_plan {
        HybridExecutionPlan::LibLinux(plan) => {
            assert_eq!(plan.device_kind, LinuxShimDeviceKind::Rtc)
        }
        _ => panic!("expected liblinux rtc plan"),
    }
    match dock_plan {
        HybridExecutionPlan::LibLinux(plan) => {
            assert_eq!(plan.device_kind, LinuxShimDeviceKind::Dock)
        }
        _ => panic!("expected liblinux dock plan"),
    }
    match smart_card_plan {
        HybridExecutionPlan::LibLinux(plan) => {
            assert_eq!(plan.device_kind, LinuxShimDeviceKind::SmartCard)
        }
        _ => panic!("expected liblinux smart-card plan"),
    }
}

#[test_case]
fn liblinux_maps_expanded_request_kinds_to_specific_devices() {
    let cfg = SideCarVmConfig::new(10, 1, 64 * 1024 * 1024);

    let bluetooth = HybridRequest::bluetooth(0x3500, 0x100, 0x5500, 0x1000, 90);
    let display = HybridRequest::display(0x3600, 0x100, 0x5600, 0x1000, 91);
    let storage = HybridRequest::storage(0x3700, 0x100, 0x5700, 0x1000, 92);
    let nvme = HybridRequest::nvme(0x3800, 0x100, 0x5800, 0x1000, 93);
    let camera = HybridRequest::camera(0x3900, 0x100, 0x5900, 0x1000, 94);
    let audio = HybridRequest::audio(0x3A00, 0x100, 0x5A00, 0x1000, 95);
    let sensor = HybridRequest::sensor(0x3B00, 0x80, 0x5B00, 0x800, 96);
    let input = HybridRequest::input(0x3C00, 0x80, 0x5C00, 0x800, 97);

    let bt_plan = HybridOrchestrator::plan(&bluetooth, BackendPreference::LibLinuxFirst, cfg)
        .expect("bluetooth should have a liblinux plan");
    let display_plan = HybridOrchestrator::plan(&display, BackendPreference::LibLinuxFirst, cfg)
        .expect("display should have a liblinux plan");
    let storage_plan = HybridOrchestrator::plan(&storage, BackendPreference::LibLinuxFirst, cfg)
        .expect("storage should have a liblinux plan");
    let nvme_plan = HybridOrchestrator::plan(&nvme, BackendPreference::LibLinuxFirst, cfg)
        .expect("nvme should have a liblinux plan");
    let camera_plan = HybridOrchestrator::plan(&camera, BackendPreference::LibLinuxFirst, cfg)
        .expect("camera should have a liblinux plan");
    let audio_plan = HybridOrchestrator::plan(&audio, BackendPreference::LibLinuxFirst, cfg)
        .expect("audio should have a liblinux plan");
    let sensor_plan = HybridOrchestrator::plan(&sensor, BackendPreference::LibLinuxFirst, cfg)
        .expect("sensor should have a liblinux plan");
    let input_plan = HybridOrchestrator::plan(&input, BackendPreference::LibLinuxFirst, cfg)
        .expect("input should have a liblinux plan");

    match bt_plan {
        HybridExecutionPlan::LibLinux(plan) => {
            assert_eq!(plan.device_kind, LinuxShimDeviceKind::Bluetooth)
        }
        _ => panic!("expected liblinux bluetooth plan"),
    }
    match display_plan {
        HybridExecutionPlan::LibLinux(plan) => {
            assert_eq!(plan.device_kind, LinuxShimDeviceKind::Display)
        }
        _ => panic!("expected liblinux display plan"),
    }
    match storage_plan {
        HybridExecutionPlan::LibLinux(plan) => {
            assert_eq!(plan.device_kind, LinuxShimDeviceKind::Storage)
        }
        _ => panic!("expected liblinux storage plan"),
    }
    match nvme_plan {
        HybridExecutionPlan::LibLinux(plan) => {
            assert_eq!(plan.device_kind, LinuxShimDeviceKind::Nvme)
        }
        _ => panic!("expected liblinux nvme plan"),
    }
    match camera_plan {
        HybridExecutionPlan::LibLinux(plan) => {
            assert_eq!(plan.device_kind, LinuxShimDeviceKind::Camera)
        }
        _ => panic!("expected liblinux camera plan"),
    }
    match audio_plan {
        HybridExecutionPlan::LibLinux(plan) => {
            assert_eq!(plan.device_kind, LinuxShimDeviceKind::Audio)
        }
        _ => panic!("expected liblinux audio plan"),
    }
    match sensor_plan {
        HybridExecutionPlan::LibLinux(plan) => {
            assert_eq!(plan.device_kind, LinuxShimDeviceKind::Sensor)
        }
        _ => panic!("expected liblinux sensor plan"),
    }
    match input_plan {
        HybridExecutionPlan::LibLinux(plan) => {
            assert_eq!(plan.device_kind, LinuxShimDeviceKind::Input)
        }
        _ => panic!("expected liblinux input plan"),
    }
}

#[test_case]
fn driverkit_plans_rtc_and_sensor_hub_paths() {
    let cfg = SideCarVmConfig::new(4, 1, 64 * 1024 * 1024);
    let rtc = HybridRequest::rtc(0x3300, 0x80, 0x5300, 0x800, 73);
    let sensor_hub = HybridRequest::sensor_hub(0x3400, 0x80, 0x5400, 0x800, 74);

    let rtc_plan = HybridOrchestrator::plan(&rtc, BackendPreference::DriverKitFirst, cfg)
        .expect("rtc should have a driverkit plan");
    let sensor_plan = HybridOrchestrator::plan(
        &sensor_hub,
        BackendPreference::DriverKitFirst,
        cfg,
    )
    .expect("sensor hub should have a driverkit plan");

    assert!(matches!(rtc_plan, HybridExecutionPlan::DriverKit(_)));
    assert!(matches!(sensor_plan, HybridExecutionPlan::DriverKit(_)));
}

#[test_case]
fn runtime_assessment_reports_sidecar_isolation_for_network() {
    let request = HybridRequest::network(0x5000, 0x100, 0x7000, 0x1000, 75);
    let report = HybridOrchestrator::runtime_assessment(&request, None);

    let sidecar = report
        .assessments
        .iter()
        .find(|entry| entry.backend == BackendPreference::SideCarFirst)
        .expect("sidecar assessment should exist");

    assert!(sidecar.supported);
    assert_eq!(sidecar.security, HybridSecurityPosture::Isolated);
    assert!(matches!(
        sidecar.confidence,
        HybridRuntimeConfidence::Medium | HybridRuntimeConfidence::High
    ));
}

#[test_case]
fn runtime_assessment_flags_driverkit_health_risk() {
    let request = HybridRequest::user_mode_device(0xA000, 0x1000, 76);
    let health = DriverKitHealthSnapshot {
        class_count: 1,
        binding_count: 1,
        started_count: 0,
        faulted_count: 2,
        quarantined_count: 1,
        dispatch_success_count: 1,
        dispatch_failure_count: 9,
    };
    let report = HybridOrchestrator::runtime_assessment(&request, Some(health));

    let driverkit = report
        .assessments
        .iter()
        .find(|entry| entry.backend == BackendPreference::DriverKitFirst)
        .expect("driverkit assessment should exist");

    assert_eq!(driverkit.security, HybridSecurityPosture::CompatibilityRisk);
    assert!(matches!(
        driverkit.confidence,
        HybridRuntimeConfidence::Low | HybridRuntimeConfidence::Medium
    ));
}

#[test_case]
fn planner_rejects_invalid_irq_vector() {
    let cfg = SideCarVmConfig::new(5, 1, 64 * 1024 * 1024);
    let invalid = HybridRequest::network(0x6000, 0x100, 0x8000, 0x1000, 0);
    let plan = HybridOrchestrator::plan(&invalid, BackendPreference::SideCarFirst, cfg);
    assert!(plan.is_none());
}

#[test_case]
fn planner_rejects_missing_iova_for_kernel_device_paths() {
    let cfg = SideCarVmConfig::new(6, 1, 64 * 1024 * 1024);
    let invalid = HybridRequest::network(0x6100, 0x100, 0x8100, 0, 77);
    let plan = HybridOrchestrator::plan(&invalid, BackendPreference::LibLinuxFirst, cfg);
    assert!(plan.is_none());
}

#[test_case]
fn readiness_report_flags_driverkit_health_regression() {
    let health = DriverKitHealthSnapshot {
        class_count: 1,
        binding_count: 1,
        started_count: 0,
        faulted_count: 1,
        quarantined_count: 1,
        dispatch_success_count: 1,
        dispatch_failure_count: 10,
    };

    let report = HybridOrchestrator::readiness_report(Some(health));
    assert!(!report.release_ready);
    assert!(report
        .gaps
        .iter()
        .any(|gap| gap.severity == HybridGapSeverity::Critical && gap.issue == "driverkit has quarantined bindings"));
}

#[test_case]
fn fleet_report_ranks_backends_and_flags_overall_readiness() {
    let report = HybridOrchestrator::fleet_report(None);

    assert_eq!(report.backends.len(), 4);
    assert_eq!(report.families.len(), 8);
    assert!(report.overall_ready);
    assert!(matches!(
        report.most_ready_backend,
        BackendPreference::SideCarFirst | BackendPreference::LibLinuxFirst | BackendPreference::DriverKitFirst | BackendPreference::ReactOsFirst
    ));
    assert!(matches!(
        report.least_ready_backend,
        BackendPreference::SideCarFirst | BackendPreference::LibLinuxFirst | BackendPreference::DriverKitFirst | BackendPreference::ReactOsFirst
    ));
    assert!(report.backends.iter().any(|status| status.ready));
    assert!(report.families.iter().any(|status| status.family == HybridRequestFamily::Network && status.ready));
    assert!(report.families.iter().any(|status| status.family == HybridRequestFamily::Storage && status.ready));
}

#[test_case]
fn readiness_report_surfaces_family_level_warnings() {
    let health = DriverKitHealthSnapshot {
        class_count: 3,
        binding_count: 1,
        started_count: 0,
        faulted_count: 2,
        quarantined_count: 2,
        dispatch_success_count: 0,
        dispatch_failure_count: 12,
    };

    let report = HybridOrchestrator::readiness_report(Some(health));
    assert!(report.gaps.iter().any(|gap| gap.issue == "driver family is not yet fleet-ready"));
    assert!(report.gaps.iter().any(|gap| gap.issue == "driver family has multiple high-risk paths"));
}

#[test_case]
fn maturity_report_exposes_the_five_missing_dimensions() {
    let report = HybridOrchestrator::maturity_report(None);

    assert_eq!(report.findings.len(), 5);
    assert!(report.findings.iter().any(|finding| finding.dimension == HybridMaturityDimension::TelemetryCoverage));
    assert!(report.findings.iter().any(|finding| finding.dimension == HybridMaturityDimension::TailLatency));
    assert!(report.findings.iter().any(|finding| finding.dimension == HybridMaturityDimension::ThreatModelCoverage));
    assert!(report.findings.iter().any(|finding| finding.dimension == HybridMaturityDimension::CertificationMatrix));
    assert!(report.findings.iter().any(|finding| finding.dimension == HybridMaturityDimension::FailoverConsistency));
    assert!(!report.production_ready);
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
