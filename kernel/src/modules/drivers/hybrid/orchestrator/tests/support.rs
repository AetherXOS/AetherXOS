use super::*;

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
        Some(health),
    )
    .expect("plan should exist");

    assert!(matches!(plan, HybridExecutionPlan::SideCar(_)));
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
    let dock = HybridRequest::dock(0x2450, 0x80, 0x3450, 0x800, 61);
    let firmware = HybridRequest::firmware(0x2500, 0x100, 0x3500, 0x1000, 62);

    assert!(rtc.entries.iter().any(|entry| entry.backend == BackendPreference::SideCarFirst && entry.supported));
    assert!(sensor_hub.entries.iter().any(|entry| entry.backend == BackendPreference::SideCarFirst && entry.supported));
    assert!(modem.entries.iter().any(|entry| entry.backend == BackendPreference::LibLinuxFirst && entry.supported));
    assert!(printer.entries.iter().any(|entry| entry.backend == BackendPreference::SideCarFirst && entry.supported));
    assert!(nfc.entries.iter().any(|entry| entry.backend == BackendPreference::SideCarFirst && entry.supported));
    assert!(tpm.entries.iter().any(|entry| entry.backend == BackendPreference::DriverKitFirst && entry.supported));
}
