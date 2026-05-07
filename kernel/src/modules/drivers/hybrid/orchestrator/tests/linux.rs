use super::*;

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
