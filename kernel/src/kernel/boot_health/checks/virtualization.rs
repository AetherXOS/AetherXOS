use super::check;
use super::BootHealthReport;
use crate::kernel::boot_health::codes::BootErrorCode;

pub(super) fn run_virtualization_checks(report: &mut BootHealthReport) {
    check(
        report,
        BootErrorCode::VirtualizationEffectiveExecutionContractFailed,
        crate::kernel::virtualization_contract::virtualization_effective_execution_contract_holds(),
        "virtualization effective execution profile contract is inconsistent",
    );
    check(
        report,
        BootErrorCode::VirtualizationEffectiveGovernorContractFailed,
        crate::kernel::virtualization_contract::virtualization_effective_governor_contract_holds(),
        "virtualization effective governor profile contract is inconsistent",
    );
}
