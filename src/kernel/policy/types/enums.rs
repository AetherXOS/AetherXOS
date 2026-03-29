#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum DriftReasonCode {
    None = 0,
    PressureHigh = 1,
    RtStarvation = 2,
    NetworkSlo = 3,
    VfsSlo = 4,
    DriverWaitTimeout = 5,
}

impl DriftReasonCode {
    #[inline(always)]
    pub(crate) const fn as_u8(self) -> u8 {
        self.to_u8()
    }

    #[inline(always)]
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::PressureHigh => "pressure_high",
            Self::RtStarvation => "rt_starvation",
            Self::NetworkSlo => "network_slo",
            Self::VfsSlo => "vfs_slo",
            Self::DriverWaitTimeout => "driver_wait_timeout",
        }
    }
}

crate::impl_enum_u8_default_conversions!(
    DriftReasonCode {
        None,
        PressureHigh,
        RtStarvation,
        NetworkSlo,
        VfsSlo,
        DriverWaitTimeout,
    },
    default = None
);

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
