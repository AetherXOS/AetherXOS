define_enum!(pub(crate) enum DriftReasonCode : u8 {
    None = 0                => "none",
    PressureHigh = 1        => "pressure_high",
    RtStarvation = 2        => "rt_starvation",
    NetworkSlo = 3          => "network_slo",
    VfsSlo = 4              => "vfs_slo",
    DriverWaitTimeout = 5   => "driver_wait_timeout",
});

impl DriftReasonCode {
    #[inline(always)]
    pub(crate) const fn as_u8(self) -> u8 {
        self.to_raw()
    }

    #[inline(always)]
    pub(crate) const fn name(self) -> &'static str {
        self.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreRuntimePolicyPreset {
    Interactive,
    Server,
    Realtime,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DriftThresholdProfile {
    pub(crate) pressure_class_threshold: crate::kernel::pressure::CorePressureClass,
    pub(crate) network_breach_limit: u8,
    pub(crate) vfs_breach_limit: u8,
    pub(crate) driver_wait_limit: u64,
}
