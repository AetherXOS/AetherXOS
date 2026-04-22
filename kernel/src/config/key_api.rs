use super::{
    BoundaryMode, DevFsPolicyProfile, KernelConfig, TlsPolicyProfile, VirtualizationExecutionClass,
    VirtualizationExecutionProfile, VirtualizationGovernorClass, VirtualizationGovernorProfile,
};
use alloc::string::String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSetError {
    UnknownKey,
    TypeMismatch,
    InvalidValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigValueKind {
    Bool,
    U8,
    U16,
    U32,
    U64,
    Usize,
    TlsPolicy,
    BoundaryMode,
    DevFsPolicy,
    VirtualizationExecution,
    VirtualizationGovernor,
}

impl ConfigValueKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::Usize => "usize",
            Self::TlsPolicy => "tls-policy",
            Self::BoundaryMode => "boundary-mode",
            Self::DevFsPolicy => "devfs-policy",
            Self::VirtualizationExecution => "virtualization-execution",
            Self::VirtualizationGovernor => "virtualization-governor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigValue {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Usize(usize),
    TlsPolicy(TlsPolicyProfile),
    BoundaryMode(BoundaryMode),
    DevFsPolicy(DevFsPolicyProfile),
    VirtualizationExecution(VirtualizationExecutionClass),
    VirtualizationGovernor(VirtualizationGovernorClass),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigKeySpec {
    pub key: &'static str,
    pub value_kind: ConfigValueKind,
    pub description: &'static str,
}

impl ConfigKeySpec {
    pub fn category(self) -> &'static str {
        KernelConfig::config_category_for_key(self.key)
    }
}

#[path = "runtime_key_autogen.rs"]
mod runtime_key_autogen;
use runtime_key_autogen::{
    auto_parse_typed_value, auto_set_by_stem, AUTO_RUNTIME_CONFIG_CATEGORIES,
    AUTO_RUNTIME_CONFIG_KEYS,
};

impl KernelConfig {
    pub fn set_by_key(key: &str, value: Option<ConfigValue>) -> Result<(), ConfigSetError> {
        let stem = normalize_runtime_key(key);
        auto_set_by_stem(stem.as_str(), value)
    }

    pub fn apply_overrides(overrides: &[(&str, ConfigValue)]) -> Result<(), ConfigSetError> {
        for (key, value) in overrides {
            Self::set_by_key(key, Some(*value))?;
        }
        Ok(())
    }

    pub fn set_by_key_str(key: &str, raw: Option<&str>) -> Result<(), ConfigSetError> {
        if raw.is_none() {
            return Self::set_by_key(key, None);
        }
        let stem = normalize_runtime_key(key);
        let spec = Self::runtime_config_spec(stem.as_str()).ok_or(ConfigSetError::UnknownKey)?;
        let raw = raw.unwrap_or("").trim();
        let value = auto_parse_typed_value(spec.value_kind, raw)?;
        auto_set_by_stem(stem.as_str(), Some(value))
    }

    pub fn runtime_config_catalog() -> &'static [ConfigKeySpec] {
        AUTO_RUNTIME_CONFIG_KEYS
    }

    pub fn runtime_config_spec(key: &str) -> Option<&'static ConfigKeySpec> {
        let stem = normalize_runtime_key(key);
        let mut i = 0usize;
        while i < AUTO_RUNTIME_CONFIG_KEYS.len() {
            let item = &AUTO_RUNTIME_CONFIG_KEYS[i];
            if item.key == stem {
                return Some(item);
            }
            i += 1;
        }
        None
    }

    pub fn runtime_config_categories() -> &'static [&'static str] {
        AUTO_RUNTIME_CONFIG_CATEGORIES
    }

    pub fn config_category_for_key(key: &str) -> &'static str {
        let stem = normalize_runtime_key(key);
        let first = stem.split('_').next().unwrap_or("other");
        match first {
            "launch" => "launch",
            "vfs" => "vfs",
            "irq" => "irq",
            "module" => "module",
            "telemetry" => "telemetry",
            "network" => "network",
            "libnet" => "libnet",
            "diskfs" => "diskfs",
            "driver" => "driver",
            "ahci" => "ahci",
            "nvme" => "nvme",
            "e1000" => "e1000",
            "scheduler" | "sched" => "scheduler",
            "devfs" => "devfs",
            "rt" => "rt",
            "runtime" => "runtime",
            "policy" => "policy",
            "core" => "core",
            "exec" => "exec",
            "userspace" => "userspace",
            "debug" => "debug",
            "serial" => "serial",
            _ => "other",
        }
    }
}

fn normalize_runtime_key(key: &str) -> String {
    let trimmed = key.trim();
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        let c = ch.to_ascii_lowercase();
        if c == '.' || c == '-' || c == ' ' || c == '/' {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    String::from(out.trim_matches('_'))
}

fn set_bool(value: Option<ConfigValue>, setter: fn(Option<bool>)) -> Result<(), ConfigSetError> {
    match value {
        None => {
            setter(None);
            Ok(())
        }
        Some(ConfigValue::Bool(v)) => {
            setter(Some(v));
            Ok(())
        }
        _ => Err(ConfigSetError::TypeMismatch),
    }
}

fn set_u8(value: Option<ConfigValue>, setter: fn(Option<u8>)) -> Result<(), ConfigSetError> {
    match value {
        None => {
            setter(None);
            Ok(())
        }
        Some(ConfigValue::U8(v)) => {
            setter(Some(v));
            Ok(())
        }
        _ => Err(ConfigSetError::TypeMismatch),
    }
}

fn set_u16(value: Option<ConfigValue>, setter: fn(Option<u16>)) -> Result<(), ConfigSetError> {
    match value {
        None => {
            setter(None);
            Ok(())
        }
        Some(ConfigValue::U16(v)) => {
            setter(Some(v));
            Ok(())
        }
        Some(ConfigValue::U64(v)) => {
            setter(Some(v as u16));
            Ok(())
        }
        _ => Err(ConfigSetError::TypeMismatch),
    }
}

fn set_u32(value: Option<ConfigValue>, setter: fn(Option<u32>)) -> Result<(), ConfigSetError> {
    match value {
        None => {
            setter(None);
            Ok(())
        }
        Some(ConfigValue::U32(v)) => {
            setter(Some(v));
            Ok(())
        }
        Some(ConfigValue::U64(v)) => {
            setter(Some(v as u32));
            Ok(())
        }
        _ => Err(ConfigSetError::TypeMismatch),
    }
}

fn set_u64(value: Option<ConfigValue>, setter: fn(Option<u64>)) -> Result<(), ConfigSetError> {
    match value {
        None => {
            setter(None);
            Ok(())
        }
        Some(ConfigValue::U64(v)) => {
            setter(Some(v));
            Ok(())
        }
        _ => Err(ConfigSetError::TypeMismatch),
    }
}

fn set_usize(value: Option<ConfigValue>, setter: fn(Option<usize>)) -> Result<(), ConfigSetError> {
    match value {
        None => {
            setter(None);
            Ok(())
        }
        Some(ConfigValue::Usize(v)) => {
            setter(Some(v));
            Ok(())
        }
        Some(ConfigValue::U64(v)) => {
            setter(Some(v as usize));
            Ok(())
        }
        _ => Err(ConfigSetError::TypeMismatch),
    }
}

fn set_tls(
    value: Option<ConfigValue>,
    setter: fn(Option<TlsPolicyProfile>),
) -> Result<(), ConfigSetError> {
    match value {
        None => {
            setter(None);
            Ok(())
        }
        Some(ConfigValue::TlsPolicy(v)) => {
            setter(Some(v));
            Ok(())
        }
        _ => Err(ConfigSetError::TypeMismatch),
    }
}

fn set_boundary_mode(
    value: Option<ConfigValue>,
    setter: fn(Option<BoundaryMode>),
) -> Result<(), ConfigSetError> {
    match value {
        None => {
            setter(None);
            Ok(())
        }
        Some(ConfigValue::BoundaryMode(v)) => {
            setter(Some(v));
            Ok(())
        }
        _ => Err(ConfigSetError::TypeMismatch),
    }
}

fn set_devfs_policy(
    value: Option<ConfigValue>,
    setter: fn(Option<DevFsPolicyProfile>),
) -> Result<(), ConfigSetError> {
    match value {
        None => {
            setter(None);
            Ok(())
        }
        Some(ConfigValue::DevFsPolicy(v)) => {
            setter(Some(v));
            Ok(())
        }
        _ => Err(ConfigSetError::TypeMismatch),
    }
}

fn set_virtualization_governor(
    value: Option<ConfigValue>,
    setter: fn(Option<VirtualizationGovernorProfile>),
) -> Result<(), ConfigSetError> {
    match value {
        None => {
            setter(None);
            Ok(())
        }
        Some(ConfigValue::VirtualizationGovernor(v)) => {
            setter(Some(VirtualizationGovernorProfile { governor_class: v }));
            Ok(())
        }
        _ => Err(ConfigSetError::TypeMismatch),
    }
}

fn set_virtualization_execution(
    value: Option<ConfigValue>,
    setter: fn(Option<VirtualizationExecutionProfile>),
) -> Result<(), ConfigSetError> {
    match value {
        None => {
            setter(None);
            Ok(())
        }
        Some(ConfigValue::VirtualizationExecution(v)) => {
            setter(Some(VirtualizationExecutionProfile {
                scheduling_class: v,
            }));
            Ok(())
        }
        _ => Err(ConfigSetError::TypeMismatch),
    }
}

fn parse_bool(raw: &str) -> Result<bool, ConfigSetError> {
    if raw.eq_ignore_ascii_case("true")
        || raw.eq_ignore_ascii_case("1")
        || raw.eq_ignore_ascii_case("yes")
        || raw.eq_ignore_ascii_case("on")
    {
        Ok(true)
    } else if raw.eq_ignore_ascii_case("false")
        || raw.eq_ignore_ascii_case("0")
        || raw.eq_ignore_ascii_case("no")
        || raw.eq_ignore_ascii_case("off")
    {
        Ok(false)
    } else {
        Err(ConfigSetError::InvalidValue)
    }
}

fn parse_u8(raw: &str) -> Result<u8, ConfigSetError> {
    raw.parse::<u8>().map_err(|_| ConfigSetError::InvalidValue)
}

fn parse_u16(raw: &str) -> Result<u16, ConfigSetError> {
    raw.parse::<u16>().map_err(|_| ConfigSetError::InvalidValue)
}

fn parse_u32(raw: &str) -> Result<u32, ConfigSetError> {
    raw.parse::<u32>().map_err(|_| ConfigSetError::InvalidValue)
}

fn parse_u64(raw: &str) -> Result<u64, ConfigSetError> {
    raw.parse::<u64>().map_err(|_| ConfigSetError::InvalidValue)
}

fn parse_usize(raw: &str) -> Result<usize, ConfigSetError> {
    raw.parse::<usize>()
        .map_err(|_| ConfigSetError::InvalidValue)
}
