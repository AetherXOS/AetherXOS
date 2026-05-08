#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BootErrorCode {
    None = 0,

    // Core Checks (1001-1099)
    TimeSliceInvalid = 1001,
    StackSizeInvalid = 1002,
    WatchdogHardStallInvalid = 1003,
    SoftWatchdogStallInvalid = 1004,
    IrqVectorBaseInvalid = 1005,
    LaunchMaxProcessNameLenInvalid = 1006,
    LaunchMaxBootImageBytesInvalid = 1007,
    VfsMaxMountPathInvalid = 1008,
    KernelMaxCpusInvalid = 1009,
    RebalancePreferLocalSkipBudgetHigh = 1010,
    HeapAllocatorConstraintFailed = 1011,

    // AArch64 Checks (1101-1199)
    AArch64IrqStormWindowTicksInvalid = 1101,
    AArch64IrqStormThresholdInvalid = 1102,
    AArch64IrqStormLogEveryInvalid = 1103,
    AArch64TimerRearmMinMaxInvalid = 1104,
    AArch64TimerJitterToleranceInvalid = 1105,
    AArch64IrqRateTrackLimitInvalid = 1106,
    AArch64IrqPerLineStormThresholdInvalid = 1107,
    AArch64IrqPerLineLogEveryInvalid = 1108,

    // Driver Checks (1201-1299)
    DriverNetworkQuarantineRebindFailuresInvalid = 1201,
    DriverNetworkQuarantineCooldownSamplesInvalid = 1202,
    LoadBalancePercentileWindowInvalid = 1203,

    // Scheduling/Lottery Checks (1301-1399)
    SchedLotteryReplayTraceCapacityInvalid = 1301,

    // Virtualization Checks (1401-1499)
    VirtualizationEffectiveExecutionContractFailed = 1401,
    VirtualizationEffectiveGovernorContractFailed = 1402,
}

impl core::fmt::Display for BootErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?} ({})", self, *self as u32)
    }
}
