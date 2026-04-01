#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryMode {
    Strict,
    Balanced,
    Compat,
}

impl BoundaryMode {
    pub fn from_str(value: &str) -> Self {
        match value.trim() {
            value if value.eq_ignore_ascii_case("Strict") => Self::Strict,
            value if value.eq_ignore_ascii_case("Balanced") => Self::Balanced,
            value if value.eq_ignore_ascii_case("Compat") => Self::Compat,
            _ => Self::Strict,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleStrategy {
    Halt,
    Spin,
}

impl IdleStrategy {
    pub fn from_str(value: &str) -> Self {
        match value.trim() {
            value if value.eq_ignore_ascii_case("Spin") => Self::Spin,
            _ => Self::Halt,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanicAction {
    Halt,
    Spin,
}

impl PanicAction {
    pub fn from_str(value: &str) -> Self {
        match value.trim() {
            value if value.eq_ignore_ascii_case("Spin") => Self::Spin,
            _ => Self::Halt,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogAction {
    Halt,
    LogOnly,
}

impl WatchdogAction {
    pub fn from_str(value: &str) -> Self {
        match value.trim() {
            value if value.eq_ignore_ascii_case("LogOnly") => Self::LogOnly,
            _ => Self::Halt,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityPolicy {
    PreferLocal,
    Spread,
}

impl AffinityPolicy {
    pub fn from_str(value: &str) -> Self {
        match value.trim() {
            value if value.eq_ignore_ascii_case("Spread") => Self::Spread,
            _ => Self::PreferLocal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibNetFastPathStrategy {
    Unchanged,
    Adaptive,
    LowLatency,
    Balanced,
    Throughput,
    PowerSave,
}

impl LibNetFastPathStrategy {
    pub fn from_str(value: &str) -> Self {
        match value.trim() {
            value if value.eq_ignore_ascii_case("Unchanged") => Self::Unchanged,
            value if value.eq_ignore_ascii_case("Adaptive") => Self::Adaptive,
            value if value.eq_ignore_ascii_case("LowLatency") => Self::LowLatency,
            value if value.eq_ignore_ascii_case("Balanced") => Self::Balanced,
            value if value.eq_ignore_ascii_case("Throughput") => Self::Throughput,
            value if value.eq_ignore_ascii_case("PowerSave") => Self::PowerSave,
            _ => Self::Adaptive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsPolicyProfile {
    Minimal,
    Balanced,
    Strict,
}

impl TlsPolicyProfile {
    pub fn from_str(value: &str) -> Self {
        match value.trim() {
            value if value.eq_ignore_ascii_case("Minimal") => Self::Minimal,
            value if value.eq_ignore_ascii_case("Strict") => Self::Strict,
            _ => Self::Balanced,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "Minimal",
            Self::Balanced => "Balanced",
            Self::Strict => "Strict",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevFsPolicyProfile {
    Strict,
    Balanced,
    Dev,
}

impl DevFsPolicyProfile {
    pub fn from_str(value: &str) -> Self {
        match value.trim() {
            value if value.eq_ignore_ascii_case("Strict") => Self::Strict,
            value if value.eq_ignore_ascii_case("Dev") => Self::Dev,
            _ => Self::Balanced,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualizationExecutionClass {
    LatencyCritical,
    Balanced,
    Background,
}

impl VirtualizationExecutionClass {
    pub fn from_str(value: &str) -> Self {
        match value.trim() {
            value if value.eq_ignore_ascii_case("LatencyCritical") => Self::LatencyCritical,
            value if value.eq_ignore_ascii_case("Background") => Self::Background,
            _ => Self::Balanced,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::LatencyCritical => "LatencyCritical",
            Self::Balanced => "Balanced",
            Self::Background => "Background",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualizationGovernorClass {
    Performance,
    Balanced,
    Efficiency,
}

impl VirtualizationGovernorClass {
    pub fn from_str(value: &str) -> Self {
        match value.trim() {
            value if value.eq_ignore_ascii_case("Performance") => Self::Performance,
            value if value.eq_ignore_ascii_case("Efficiency") => Self::Efficiency,
            _ => Self::Balanced,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Performance => "Performance",
            Self::Balanced => "Balanced",
            Self::Efficiency => "Efficiency",
        }
    }
}
