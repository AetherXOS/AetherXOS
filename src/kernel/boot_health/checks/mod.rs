mod aarch64;
mod core;
mod driver;
mod virtualization;

use super::self_test::BootHealthReport;

pub(super) fn run_boot_config_checks(report: &mut BootHealthReport) {
    core::run_core_checks(report);
    aarch64::run_aarch64_checks(report);
    driver::run_driver_checks(report);
    virtualization::run_virtualization_checks(report);
}

fn check(report: &mut BootHealthReport, code: u32, cond: bool, msg: &str) {
    report.checks = report.checks.saturating_add(1);
    if !cond {
        report.failures = report.failures.saturating_add(1);
        report.last_error_code = code;
        crate::klog_error!("[BOOT SELFTEST] E{}: {}", code, msg);
    }
}
