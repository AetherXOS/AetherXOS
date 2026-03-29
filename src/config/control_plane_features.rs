use super::super::feature_catalog::CompileTimeFeatureView;
use super::*;
use super::batch::{refresh_runtime_compat_surface_if_needed, validate_runtime_override};
use super::support::split_override_entries;
use super::utils::{normalize_feature_name, parse_feature_flag};
use crate::config::ConfigValue;
use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::Ordering;

struct FeatureGateMap {
    names: &'static [&'static str],
    gate: &'static str,
}

const FEATURE_NAMES_TELEMETRY: &[&str] = &["telemetry"];
const FEATURE_NAMES_VFS: &[&str] = &[
    "vfs",
    "vfs_backends",
    "vfs_disk_fs",
    "vfs_library_backends",
    "vfs_network_fs",
    "vfs_ramfs",
    "vfs_telemetry",
    "vfs_fatfs",
    "vfs_littlefs",
    "vfs_ext4",
    "vfs_squashfs",
];
const FEATURE_NAMES_IPC: &[&str] = &[
    "ipc",
    "ipc_zero_copy",
    "ipc_message_passing",
    "ipc_shared_memory",
    "ipc_signal_only",
    "ipc_ring_buffer",
    "ipc_futex",
    "ipc_binder",
    "ipc_unix_domain",
    "ipc_dbus",
    "ipc_sysv_sem",
    "ipc_sysv_msg",
];
const FEATURE_NAMES_NETWORK: &[&str] = &[
    "networking",
    "network_transport",
    "network_wireguard",
    "network_http",
    "network_https",
    "libnet",
    "libnet_l2",
    "libnet_l34",
    "libnet_l6_tls",
    "libnet_l7_http2",
];
const FEATURE_NAMES_PROC_CONFIG: &[&str] = &["proc_config_surface"];
const FEATURE_NAMES_SYSCTL: &[&str] = &["sysctl_surface"];
const FEATURE_NAMES_SECURITY: &[&str] = &["security", "security_null", "security_acl", "security_sel4"];
const FEATURE_NAMES_CAPABILITIES: &[&str] = &["capabilities", "security_capabilities"];

const FEATURE_GATE_MAP: &[FeatureGateMap] = &[
    FeatureGateMap {
        names: FEATURE_NAMES_TELEMETRY,
        gate: "telemetry_enabled",
    },
    FeatureGateMap {
        names: FEATURE_NAMES_VFS,
        gate: "vfs_library_api_exposed",
    },
    FeatureGateMap {
        names: FEATURE_NAMES_IPC,
        gate: "ipc_library_api_exposed",
    },
    FeatureGateMap {
        names: FEATURE_NAMES_NETWORK,
        gate: "network_library_api_exposed",
    },
    FeatureGateMap {
        names: FEATURE_NAMES_PROC_CONFIG,
        gate: "proc_config_api_exposed",
    },
    FeatureGateMap {
        names: FEATURE_NAMES_SYSCTL,
        gate: "sysctl_api_exposed",
    },
    FeatureGateMap {
        names: FEATURE_NAMES_SECURITY,
        gate: "security_enforcement_enabled",
    },
    FeatureGateMap {
        names: FEATURE_NAMES_CAPABILITIES,
        gate: "capability_enforcement_enabled",
    },
];

impl KernelConfig {
    pub fn visit_feature_controls<F>(mut visit: F)
    where
        F: FnMut(ConfigFeatureControl),
    {
        Self::visit_cargo_feature_catalog(|feature| {
            let (runtime_gate_key, effective_enabled) = feature_runtime_control(feature);
            visit(ConfigFeatureControl {
                name: feature.name,
                category: feature.category,
                compile_enabled: feature.enabled,
                runtime_gate_key,
                runtime_gate_available: runtime_gate_key.is_some(),
                effective_enabled,
            });
        });
    }

    pub fn feature_controls() -> Vec<ConfigFeatureControl> {
        let mut out = Vec::new();
        Self::visit_feature_controls(|item| out.push(item));
        out
    }

    pub fn feature_control(name: &str) -> Option<ConfigFeatureControl> {
        let mut found = None;
        Self::visit_feature_controls(|item| {
            if item.name == name {
                found = Some(item);
            }
        });
        found
    }

    pub fn set_feature_enabled(
        feature_name: &str,
        enabled: Option<bool>,
    ) -> Result<ConfigFeatureControl, ConfigSetError> {
        let normalized = normalize_feature_name(feature_name);
        if normalized.is_empty() {
            return Err(ConfigSetError::UnknownKey);
        }

        let compile_enabled =
            Self::is_cargo_feature_enabled(normalized.as_str()).ok_or(ConfigSetError::UnknownKey)?;
        let category = Self::cargo_feature_category(normalized.as_str());
        let gate_key = runtime_gate_key_for_feature(normalized.as_str(), category)
            .ok_or(ConfigSetError::TypeMismatch)?;

        if !compile_enabled && enabled == Some(true) {
            return Err(ConfigSetError::InvalidValue);
        }

        let raw = enabled.map(|flag| if flag { "true" } else { "false" });
        validate_runtime_override(gate_key, raw)?;

        Self::set_by_key(gate_key, enabled.map(ConfigValue::Bool))?;
        refresh_runtime_compat_surface_if_needed();

        Self::feature_control(normalized.as_str())
            .ok_or(ConfigSetError::UnknownKey)
    }

    pub fn apply_feature_override_batch_str(raw: &str) -> Result<usize, ConfigBatchApplyError> {
        CONFIG_APPLY_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        let mut applied = 0usize;

        for (index, raw_entry) in split_override_entries(raw).into_iter().enumerate() {
            let trimmed = raw_entry.trim();
            if trimmed.is_empty() {
                continue;
            }

            let (feature_name, enabled) = parse_feature_override_entry(trimmed).map_err(|cause| {
                CONFIG_APPLY_FAILURES.fetch_add(1, Ordering::Relaxed);
                CONFIG_LAST_ERROR_INDEX.store(index, Ordering::Relaxed);
                ConfigBatchApplyError {
                    index,
                    key: normalize_feature_name(trimmed),
                    raw_entry: raw_entry.clone(),
                    cause,
                }
            })?;

            Self::set_feature_enabled(feature_name.as_str(), enabled).map_err(|cause| {
                CONFIG_APPLY_FAILURES.fetch_add(1, Ordering::Relaxed);
                CONFIG_LAST_ERROR_INDEX.store(index, Ordering::Relaxed);
                ConfigBatchApplyError {
                    index,
                    key: feature_name.clone(),
                    raw_entry: raw_entry.clone(),
                    cause,
                }
            })?;

            applied += 1;
        }

        CONFIG_APPLY_SUCCESS.fetch_add(1, Ordering::Relaxed);
        CONFIG_LAST_ERROR_INDEX.store(usize::MAX, Ordering::Relaxed);
        Ok(applied)
    }

    pub fn feature_controls_by_category(category: &str) -> Vec<ConfigFeatureControl> {
        let mut out = Vec::new();
        Self::visit_feature_controls(|item| {
            if item.category == category {
                out.push(item);
            }
        });
        out
    }

    pub fn feature_runtime_drift_count() -> usize {
        let mut count = 0usize;
        Self::visit_feature_controls(|item| {
            if item.compile_enabled != item.effective_enabled {
                count += 1;
            }
        });
        count
    }

    pub fn feature_category_summaries() -> Vec<ConfigFeatureCategorySummary> {
        let mut buckets: BTreeMap<&'static str, ConfigFeatureCategorySummary> = BTreeMap::new();
        Self::visit_feature_controls(|item| {
            let entry = buckets
                .entry(item.category)
                .or_insert(ConfigFeatureCategorySummary {
                    category: item.category,
                    total: 0,
                    compile_enabled: 0,
                    runtime_gateable: 0,
                    effective_enabled: 0,
                });
            entry.total += 1;
            if item.compile_enabled {
                entry.compile_enabled += 1;
            }
            if item.runtime_gate_available {
                entry.runtime_gateable += 1;
            }
            if item.effective_enabled {
                entry.effective_enabled += 1;
            }
        });

        let mut out = Vec::with_capacity(buckets.len());
        for (_, summary) in buckets {
            out.push(summary);
        }
        out
    }
}

fn feature_runtime_control(feature: CompileTimeFeatureView) -> (Option<&'static str>, bool) {
    let gate = runtime_gate_key_for_feature(feature.name, feature.category);
    let effective_enabled = match gate {
        Some("telemetry_enabled") => feature.enabled && KernelConfig::is_telemetry_enabled(),
        Some("vfs_library_api_exposed") => {
            feature.enabled && KernelConfig::is_vfs_library_api_exposed()
        }
        Some("network_library_api_exposed") => {
            feature.enabled && KernelConfig::is_network_library_api_exposed()
        }
        Some("ipc_library_api_exposed") => {
            feature.enabled && KernelConfig::is_ipc_library_api_exposed()
        }
        Some("proc_config_api_exposed") => {
            feature.enabled && KernelConfig::is_proc_config_api_exposed()
        }
        Some("sysctl_api_exposed") => feature.enabled && KernelConfig::is_sysctl_api_exposed(),
        Some("security_enforcement_enabled") => {
            feature.enabled && KernelConfig::security_enforcement_enabled()
        }
        Some("capability_enforcement_enabled") => {
            feature.enabled && KernelConfig::capability_enforcement_enabled()
        }
        Some("strict_optional_features_enabled") => {
            feature.enabled && KernelConfig::is_strict_optional_features_enabled()
        }
        _ => feature.enabled,
    };
    (gate, effective_enabled)
}

fn runtime_gate_key_for_feature(feature_name: &str, category: &str) -> Option<&'static str> {
    for mapping in FEATURE_GATE_MAP {
        if mapping.names.contains(&feature_name) {
            return Some(mapping.gate);
        }
    }

    match category {
        "telemetry" => Some("telemetry_enabled"),
        "vfs" => Some("vfs_library_api_exposed"),
        "ipc" => Some("ipc_library_api_exposed"),
        "networking" | "network_transport" | "libnet" => Some("network_library_api_exposed"),
        "security" => Some("security_enforcement_enabled"),
        "core" => Some("strict_optional_features_enabled"),
        _ => None,
    }
}

fn parse_feature_override_entry(raw: &str) -> Result<(String, Option<bool>), ConfigSetError> {
    if let Some(eq) = raw.find('=') {
        let feature = normalize_feature_name(&raw[..eq]);
        if feature.is_empty() {
            return Err(ConfigSetError::UnknownKey);
        }
        let value = raw[eq + 1..].trim();
        if value.is_empty() {
            return Ok((feature, None));
        }
        return parse_feature_flag(value).map(|enabled| (feature, Some(enabled)));
    }

    if let Some(rest) = raw.strip_prefix('!') {
        let feature = normalize_feature_name(rest);
        if feature.is_empty() {
            return Err(ConfigSetError::UnknownKey);
        }
        return Ok((feature, Some(false)));
    }

    let normalized = normalize_feature_name(raw);
    if normalized.is_empty() {
        return Err(ConfigSetError::UnknownKey);
    }
    if let Some(feature) = normalized.strip_prefix("reset_") {
        if feature.is_empty() {
            return Err(ConfigSetError::UnknownKey);
        }
        return Ok((String::from(feature), None));
    }
    if let Some(feature) = normalized.strip_prefix("unset_") {
        if feature.is_empty() {
            return Err(ConfigSetError::UnknownKey);
        }
        return Ok((String::from(feature), None));
    }
    if let Some(feature) = normalized.strip_prefix("no_") {
        if feature.is_empty() {
            return Err(ConfigSetError::UnknownKey);
        }
        return Ok((String::from(feature), Some(false)));
    }
    Ok((normalized, Some(true)))
}
