use super::checks::run_contract_checks;
use super::context::current_virtualization_contract_context;
use super::report::VirtualizationContractReport;

pub fn run_virtualization_contract_self_test() -> VirtualizationContractReport {
    let mut checks = 0u32;
    let mut failures = 0u32;
    let mut last_error_code = 0u32;
    let context = current_virtualization_contract_context();

    run_contract_checks(&context, &mut checks, &mut failures, &mut last_error_code);

    if failures == 0 {
        crate::klog_info!("[VIRT CONTRACT] passed checks={}", checks);
    } else {
        crate::klog_error!(
            "[VIRT CONTRACT] failed checks={} failures={} last_error=E{}",
            checks,
            failures,
            last_error_code
        );
    }

    VirtualizationContractReport {
        checks,
        failures,
        last_error_code,
    }
}
