use crate::kernel::boot_health::self_test::BootHealthReport;

pub(super) fn merge_syscall(
    report: &mut BootHealthReport,
    child: crate::kernel::syscall_contract::SyscallContractReport,
) {
    report.checks = report.checks.saturating_add(child.checks);
    report.failures = report.failures.saturating_add(child.failures);
    if child.failures > 0 {
        report.last_error_code = child.last_error_code;
    }
}

pub(super) fn merge_scheduler(
    report: &mut BootHealthReport,
    child: crate::kernel::scheduler_contract::SchedulerContractReport,
) {
    report.checks = report.checks.saturating_add(child.checks);
    report.failures = report.failures.saturating_add(child.failures);
    if child.failures > 0 {
        report.last_error_code = child.last_error_code;
    }
}

pub(super) fn merge_policy(
    report: &mut BootHealthReport,
    child: crate::kernel::policy::RuntimePolicyContractReport,
) {
    report.checks = report.checks.saturating_add(child.checks);
    report.failures = report.failures.saturating_add(child.failures);
    if child.failures > 0 {
        report.last_error_code = child.last_error_code;
    }
}

pub(super) fn merge_virtualization(
    report: &mut BootHealthReport,
    child: crate::kernel::virtualization_contract::VirtualizationContractReport,
) {
    report.checks = report.checks.saturating_add(child.checks);
    report.failures = report.failures.saturating_add(child.failures);
    if child.failures > 0 {
        report.last_error_code = child.last_error_code;
    }
}
