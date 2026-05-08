use super::check;
use super::BootHealthReport;
use crate::kernel::boot_health::codes::BootErrorCode;
use crate::config::KernelConfig;
use crate::generated_consts::{
    DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES, DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES,
};

pub(super) fn run_driver_checks(report: &mut BootHealthReport) {
    check(
        report,
        BootErrorCode::DriverNetworkQuarantineRebindFailuresInvalid,
        DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES > 0,
        "driver quarantine rebind failures must be > 0",
    );
    check(
        report,
        BootErrorCode::DriverNetworkQuarantineCooldownSamplesInvalid,
        DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES > 0,
        "driver quarantine cooldown samples must be > 0",
    );
    check(
        report,
        BootErrorCode::LoadBalancePercentileWindowInvalid,
        KernelConfig::load_balance_percentile_window() > 0,
        "load_balance_percentile_window must be > 0",
    );
}
