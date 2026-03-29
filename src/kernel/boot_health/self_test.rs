#[derive(Debug, Clone, Copy)]
pub struct BootHealthReport {
    pub checks: u32,
    pub failures: u32,
    pub last_error_code: u32,
}

impl BootHealthReport {
    #[inline(always)]
    pub const fn passed(self) -> bool {
        self.failures == 0
    }
}

pub fn run_boot_self_tests() -> BootHealthReport {
    let mut report = BootHealthReport {
        checks: 0,
        failures: 0,
        last_error_code: 0,
    };

    super::checks::run_boot_config_checks(&mut report);

    if report.failures == 0 {
        crate::klog_info!("[BOOT SELFTEST] passed checks={}", report.checks);
    } else {
        crate::klog_error!(
            "[BOOT SELFTEST] failed checks={} failures={} last_error=E{}",
            report.checks,
            report.failures,
            report.last_error_code
        );
    }

    super::contracts::run_chained_contracts(&mut report);

    report
}
