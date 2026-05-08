use crate::kernel::boot_health::self_test::BootHealthReport;

macro_rules! run_contract {
    ($report:expr, $name:expr, $runner:expr) => {
        crate::hal::serial::write_raw(concat!("[EARLY SERIAL] contract: ", $name, " begin\n"));
        let child = $runner();
        $report.merge_raw(child.checks, child.failures, child.last_error_code);
        crate::hal::serial::write_raw(concat!("[EARLY SERIAL] contract: ", $name, " done\n"));
    };
}

pub(crate) fn run_chained_contracts(report: &mut BootHealthReport) {
    run_contract!(
        report,
        "syscall",
        crate::kernel::syscall_contract::run_syscall_contract_self_test
    );

    crate::hal::serial::write_raw("[EARLY SERIAL] contract: scheduler begin\n");
    /*
    let child = crate::kernel::scheduler_contract::run_scheduler_contract_self_test();
    report.merge_raw(child.checks, child.failures, child.last_error_code);
    */
    crate::hal::serial::write_raw("[EARLY SERIAL] contract: scheduler done\n");

    run_contract!(
        report,
        "policy",
        crate::kernel::policy::run_runtime_policy_contract_self_test
    );

    crate::hal::serial::write_raw("[EARLY SERIAL] contract: virt begin\n");
    crate::hal::serial::write_raw("[EARLY SERIAL] calling run_virtualization_contract_self_test\n");
    let virt_report = crate::kernel::virtualization_contract::run_virtualization_contract_self_test();
    crate::hal::serial::write_raw("[EARLY SERIAL] run_virtualization_contract_self_test returned\n");
    report.merge_raw(virt_report.checks, virt_report.failures, virt_report.last_error_code);
    crate::hal::serial::write_raw("[EARLY SERIAL] contract: virt done\n");
}
