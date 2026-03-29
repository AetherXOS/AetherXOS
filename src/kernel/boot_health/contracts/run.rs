use crate::kernel::boot_health::self_test::BootHealthReport;

use super::merge::{merge_policy, merge_scheduler, merge_syscall, merge_virtualization};

pub(crate) fn run_chained_contracts(report: &mut BootHealthReport) {
    merge_syscall(
        report,
        crate::kernel::syscall_contract::run_syscall_contract_self_test(),
    );
    merge_scheduler(
        report,
        crate::kernel::scheduler_contract::run_scheduler_contract_self_test(),
    );
    merge_policy(
        report,
        crate::kernel::policy::run_runtime_policy_contract_self_test(),
    );
    merge_virtualization(
        report,
        crate::kernel::virtualization_contract::run_virtualization_contract_self_test(),
    );
}
