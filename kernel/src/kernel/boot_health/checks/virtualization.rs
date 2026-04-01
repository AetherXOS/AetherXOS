use super::check;
use super::BootHealthReport;

pub(super) fn run_virtualization_checks(report: &mut BootHealthReport) {
    check(
        report,
        1401,
        crate::kernel::virtualization_contract::virtualization_effective_execution_contract_holds(),
        "virtualization effective execution profile contract is inconsistent",
    );
    check(
        report,
        1402,
        crate::kernel::virtualization_contract::virtualization_effective_governor_contract_holds(),
        "virtualization effective governor profile contract is inconsistent",
    );
}
