use super::*;

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
            assert_eq!(policy.mode, crate::modules::drivers::hybrid::reactos::NtBinaryExecutionMode::NativeKernel)
        }
        _ => panic!("expected reactos plan"),
    }
}

#[test_case]
fn reactos_pilot_plans_firmware_request_family() {
    let request = HybridRequest::firmware(0x4100, 0x100, 0x5100, 0x1000, 38);
    let cfg = SideCarVmConfig::new(41, 1, 64 * 1024 * 1024);
    let plan = HybridOrchestrator::plan(&request, BackendPreference::ReactOsFirst, cfg)
        .expect("reactos pilot should plan firmware path");

    match plan {
        HybridExecutionPlan::ReactOs { policy, .. } => {
            assert_eq!(policy.mode, crate::modules::drivers::hybrid::reactos::NtBinaryExecutionMode::WineHostBridge)
        }
        _ => panic!("expected reactos pilot plan"),
    }
}

#[test_case]
fn reactos_pilot_plans_input_request_family() {
    let request = HybridRequest::input(0x4200, 0x80, 0x5200, 0x800, 39);
    let cfg = SideCarVmConfig::new(42, 1, 64 * 1024 * 1024);
    let plan = HybridOrchestrator::plan(&request, BackendPreference::ReactOsFirst, cfg)
        .expect("reactos pilot should plan input path");

    assert!(matches!(plan, HybridExecutionPlan::ReactOs { .. }));
}

#[test_case]
fn plans_windows_pe_with_import_bindings() {
    let image = sample_pe();
    let mut symbols = NtSymbolTable::new();
    symbols.register(NtSymbol::IoCallDriver, 0x1234_5678);
    let result = HybridOrchestrator::plan_windows_pe_with_symbols(&image, &symbols)
        .expect("PE plan should parse");
    assert_eq!(result.image_info.machine, 0x8664);
    assert_eq!(result.policy.mode, crate::modules::drivers::hybrid::reactos::NtBinaryExecutionMode::WineHostBridge);
    assert_eq!(result.domain_bindings.len(), 1);
    assert_eq!(result.counts.kernel, 1);
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
