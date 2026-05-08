#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryMode {
    Strict,
    Balanced,
    Compat,
}

macro_rules! impl_enum_from_str_with_default {
    ($enum_name:ident, $default:ident, [$($variant:ident),* $(,)?]) => {
        impl $enum_name {
            pub fn from_str(value: &str) -> Self {
                match value.trim() {
                    $(value if value.eq_ignore_ascii_case(stringify!($variant)) => Self::$variant,)*
                    _ => Self::$default,
                }
            }
        }
    };
}

macro_rules! impl_enum_from_str_with_as_str {
    ($enum_name:ident, $default:ident, [$($variant:ident),* $(,)?]) => {
        impl $enum_name {
            pub fn from_str(value: &str) -> Self {
                match value.trim() {
                    $(value if value.eq_ignore_ascii_case(stringify!($variant)) => Self::$variant,)*
                    _ => Self::$default,
                }
            }

            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant),)*
                }
            }
        }
    };
}

impl_enum_from_str_with_default!(BoundaryMode, Strict, [Strict, Balanced, Compat]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleStrategy {
    Halt,
    Spin,
}

impl_enum_from_str_with_default!(IdleStrategy, Halt, [Halt, Spin]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanicAction {
    Halt,
    Spin,
}

impl_enum_from_str_with_default!(PanicAction, Halt, [Halt, Spin]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogAction {
    Halt,
    Panic,
    Log,
}

impl_enum_from_str_with_default!(WatchdogAction, Halt, [Halt, Panic, Log]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityPolicy {
    PreferLocal,
    StrictLocal,
    Balanced,
    Spread,
}

impl_enum_from_str_with_as_str!(AffinityPolicy, PreferLocal, [PreferLocal, StrictLocal, Balanced, Spread]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibNetFastPathStrategy {
    Unchanged,
    Adaptive,
    LowLatency,
    Balanced,
    Throughput,
    PowerSave,
}

impl_enum_from_str_with_default!(LibNetFastPathStrategy, Adaptive, [Unchanged, Adaptive, LowLatency, Balanced, Throughput, PowerSave]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsPolicyProfile {
    Minimal,
    Balanced,
    Strict,
}

impl_enum_from_str_with_as_str!(TlsPolicyProfile, Balanced, [Minimal, Balanced, Strict]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevFsPolicyProfile {
    Strict,
    Balanced,
    Dev,
}

impl_enum_from_str_with_default!(DevFsPolicyProfile, Balanced, [Strict, Balanced, Dev]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualizationExecutionClass {
    LatencyCritical,
    Balanced,
    Background,
}

impl_enum_from_str_with_as_str!(VirtualizationExecutionClass, Balanced, [LatencyCritical, Balanced, Background]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualizationGovernorClass {
    Performance,
    Balanced,
    Efficiency,
}

impl_enum_from_str_with_as_str!(VirtualizationGovernorClass, Balanced, [Performance, Balanced, Efficiency]);
