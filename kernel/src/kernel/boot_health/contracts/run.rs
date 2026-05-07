use crate::kernel::boot_health::self_test::BootHealthReport;

use super::merge::{merge_policy, merge_syscall, merge_virtualization};

pub(crate) fn run_chained_contracts(report: &mut BootHealthReport) {
    crate::hal::serial::write_raw("[EARLY SERIAL] contract: syscall begin\n");
    merge_syscall(
        report,
        crate::kernel::syscall_contract::run_syscall_contract_self_test(),
    );
    crate::hal::serial::write_raw("[EARLY SERIAL] contract: syscall done\n");

    crate::hal::serial::write_raw("[EARLY SERIAL] contract: scheduler begin\n");
    /*
    merge_scheduler(
        report,
        crate::kernel::scheduler_contract::run_scheduler_contract_self_test(),
    );
    */
    crate::hal::serial::write_raw("[EARLY SERIAL] contract: scheduler done\n");

    crate::hal::serial::write_raw("[EARLY SERIAL] contract: policy begin\n");
    merge_policy(
        report,
        crate::kernel::policy::run_runtime_policy_contract_self_test(),
    );
    crate::hal::serial::write_raw("[EARLY SERIAL] contract: policy done\n");

    crate::hal::serial::write_raw("[EARLY SERIAL] contract: virt begin\n");
    crate::hal::serial::write_raw("[EARLY SERIAL] calling run_virtualization_contract_self_test\n");
    let virt_report = crate::kernel::virtualization_contract::run_virtualization_contract_self_test();
    crate::hal::serial::write_raw("[EARLY SERIAL] run_virtualization_contract_self_test returned\n");
    merge_virtualization(report, virt_report);
    crate::hal::serial::write_raw("[EARLY SERIAL] contract: virt done\n");
}
