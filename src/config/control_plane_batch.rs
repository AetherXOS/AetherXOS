use super::*;
use super::support::{parse_override_entry, split_override_entries};
use super::utils::{is_truthy, parse_feature_flag};
use crate::config::ConfigValueKind;
use core::sync::atomic::Ordering;

impl KernelConfig {
    pub fn preview_override_batch_str(raw: &str) -> Vec<ConfigOverridePreviewEntry> {
        let mut out = Vec::new();
        for (index, raw_entry) in split_override_entries(raw).into_iter().enumerate() {
            let trimmed = raw_entry.trim();
            if trimmed.is_empty() {
                continue;
            }

            match parse_override_entry(trimmed) {
                Ok((key, value)) => {
                    let spec = Self::runtime_config_spec(key.as_str());
                    let category = spec.map(|s| s.category());
                    let critical = is_critical_runtime_key(key.as_str());
                    let mut cause = None;

                    if spec.is_none() {
                        cause = Some(ConfigSetError::UnknownKey);
                    } else {
                        if let Err(err) = validate_runtime_override(key.as_str(), value.as_deref()) {
                            cause = Some(err);
                        } else if let (Some(s), Some(raw_value)) = (spec, value.as_deref()) {
                            if let Err(err) = validate_runtime_value_kind(s.value_kind, raw_value) {
                                cause = Some(err);
                            }
                        }
                    }

                    out.push(ConfigOverridePreviewEntry {
                        index,
                        key,
                        value,
                        category,
                        critical,
                        valid: cause.is_none(),
                        cause,
                    });
                }
                Err((key, cause)) => {
                    let critical = is_critical_runtime_key(key.as_str());
                    out.push(ConfigOverridePreviewEntry {
                        index,
                        key,
                        value: None,
                        category: None,
                        critical,
                        valid: false,
                        cause: Some(cause),
                    });
                }
            }
        }
        out
    }

    pub fn preview_override_batch_summary_str(raw: &str) -> ConfigOverridePreviewSummary {
        let preview = Self::preview_override_batch_str(raw);
        let mut valid = 0usize;
        let mut invalid = 0usize;
        let mut critical_touched = 0usize;

        for entry in &preview {
            if entry.valid {
                valid += 1;
            } else {
                invalid += 1;
            }
            if entry.critical {
                critical_touched += 1;
            }
        }

        ConfigOverridePreviewSummary {
            total: preview.len(),
            valid,
            invalid,
            critical_touched,
        }
    }

    pub fn runtime_override_template() -> Vec<String> {
        let mut specs = Vec::with_capacity(Self::runtime_config_catalog().len());
        let mut i = 0usize;
        while i < Self::runtime_config_catalog().len() {
            specs.push(Self::runtime_config_catalog()[i]);
            i += 1;
        }
        specs.sort_by(|a, b| {
            a.category()
                .cmp(b.category())
                .then_with(|| a.key.cmp(b.key))
        });

        let mut out = Vec::with_capacity(specs.len());
        for spec in specs {
            out.push(format_template_line(spec));
        }
        out
    }

    pub fn apply_override_batch_str(raw: &str) -> Result<usize, ConfigBatchApplyError> {
        Self::apply_override_batch_impl(raw, false)
    }

    pub fn apply_override_batch_strict(raw: &str) -> Result<usize, ConfigBatchApplyError> {
        Self::apply_override_batch_impl(raw, true)
    }

    fn apply_override_batch_impl(
        raw: &str,
        strict_critical: bool,
    ) -> Result<usize, ConfigBatchApplyError> {
        CONFIG_APPLY_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        let mut applied = 0usize;
        for (index, raw_entry) in split_override_entries(raw).into_iter().enumerate() {
            let trimmed = raw_entry.trim();
            if trimmed.is_empty() {
                continue;
            }

            let (key, value) = parse_override_entry(trimmed).map_err(|(key, cause)| {
                CONFIG_APPLY_FAILURES.fetch_add(1, Ordering::Relaxed);
                CONFIG_LAST_ERROR_INDEX.store(index, Ordering::Relaxed);
                ConfigBatchApplyError {
                    index,
                    key,
                    raw_entry: raw_entry.clone(),
                    cause,
                }
            })?;

            if strict_critical {
                validate_strict_critical_override(key.as_str(), value.as_deref()).map_err(
                    |cause| {
                        CONFIG_APPLY_FAILURES.fetch_add(1, Ordering::Relaxed);
                        CONFIG_LAST_ERROR_INDEX.store(index, Ordering::Relaxed);
                        ConfigBatchApplyError {
                            index,
                            key: key.clone(),
                            raw_entry: raw_entry.clone(),
                            cause,
                        }
                    },
                )?;
            }

            validate_runtime_override(key.as_str(), value.as_deref()).map_err(|cause| {
                CONFIG_APPLY_FAILURES.fetch_add(1, Ordering::Relaxed);
                CONFIG_LAST_ERROR_INDEX.store(index, Ordering::Relaxed);
                ConfigBatchApplyError {
                    index,
                    key: key.clone(),
                    raw_entry: raw_entry.clone(),
                    cause,
                }
            })?;

            KernelConfig::set_by_key_str(key.as_str(), value.as_deref()).map_err(|cause| {
                CONFIG_APPLY_FAILURES.fetch_add(1, Ordering::Relaxed);
                CONFIG_LAST_ERROR_INDEX.store(index, Ordering::Relaxed);
                ConfigBatchApplyError {
                    index,
                    key,
                    raw_entry,
                    cause,
                }
            })?;
            applied += 1;
        }
        CONFIG_APPLY_SUCCESS.fetch_add(1, Ordering::Relaxed);
        CONFIG_LAST_ERROR_INDEX.store(usize::MAX, Ordering::Relaxed);
        refresh_runtime_compat_surface_if_needed();
        Ok(applied)
    }

    pub fn apply_kernel_cmdline_overrides(cmdline: &str) -> Result<usize, ConfigBatchApplyError> {
        let mut applied = 0usize;
        let mut token_index = 0usize;

        for token in cmdline.split_whitespace() {
            let trimmed = token.trim();
            if trimmed.is_empty() {
                continue;
            }

            let maybe_batch = trimmed
                .strip_prefix("config=")
                .or_else(|| trimmed.strip_prefix("cfg.batch="))
                .or_else(|| trimmed.strip_prefix("hc.config="));
            if let Some(batch) = maybe_batch {
                let count = Self::apply_override_batch_str(batch).map_err(|mut err| {
                    err.index += token_index;
                    err
                })?;
                applied += count;
                token_index += count.max(1);
                continue;
            }

            let maybe_single = trimmed
                .strip_prefix("cfg.")
                .or_else(|| trimmed.strip_prefix("cfg:"))
                .or_else(|| trimmed.strip_prefix("hc."));
            if let Some(single) = maybe_single {
                let count = Self::apply_override_batch_str(single).map_err(|mut err| {
                    err.index += token_index;
                    err
                })?;
                applied += count;
                token_index += count.max(1);
            }
        }

        Ok(applied)
    }
}

fn format_template_line(spec: super::super::ConfigKeySpec) -> String {
    alloc::format!(
        "{} [{}] {} critical={}",
        spec.key,
        spec.category(),
        spec.value_kind.label(),
        is_critical_runtime_key(spec.key)
    )
}

fn is_critical_runtime_key(key: &str) -> bool {
    matches!(
        key,
        "telemetry_enabled"
            | "vfs_library_api_exposed"
            | "network_library_api_exposed"
            | "ipc_library_api_exposed"
            | "proc_config_api_exposed"
            | "sysctl_api_exposed"
            | "security_enforcement_enabled"
            | "capability_enforcement_enabled"
            | "multi_user_enabled"
            | "credential_enforcement_enabled"
            | "library_boundary_mode"
            | "strict_optional_features_enabled"
    )
}

fn validate_runtime_value_kind(kind: ConfigValueKind, raw: &str) -> Result<(), ConfigSetError> {
    match kind {
        ConfigValueKind::Bool => {
            let _ = parse_feature_flag(raw)?;
            Ok(())
        }
        ConfigValueKind::U8 => raw
            .parse::<u8>()
            .map(|_| ())
            .map_err(|_| ConfigSetError::InvalidValue),
        ConfigValueKind::U16 => raw
            .parse::<u16>()
            .map(|_| ())
            .map_err(|_| ConfigSetError::InvalidValue),
        ConfigValueKind::U32 => raw
            .parse::<u32>()
            .map(|_| ())
            .map_err(|_| ConfigSetError::InvalidValue),
        ConfigValueKind::U64 => raw
            .parse::<u64>()
            .map(|_| ())
            .map_err(|_| ConfigSetError::InvalidValue),
        ConfigValueKind::Usize => raw
            .parse::<usize>()
            .map(|_| ())
            .map_err(|_| ConfigSetError::InvalidValue),
        ConfigValueKind::TlsPolicy => {
            if raw.eq_ignore_ascii_case("minimal")
                || raw.eq_ignore_ascii_case("balanced")
                || raw.eq_ignore_ascii_case("strict")
            {
                Ok(())
            } else {
                Err(ConfigSetError::InvalidValue)
            }
        }
        ConfigValueKind::BoundaryMode => {
            if raw.eq_ignore_ascii_case("strict")
                || raw.eq_ignore_ascii_case("balanced")
                || raw.eq_ignore_ascii_case("compat")
            {
                Ok(())
            } else {
                Err(ConfigSetError::InvalidValue)
            }
        }
        ConfigValueKind::DevFsPolicy => {
            if raw.eq_ignore_ascii_case("strict")
                || raw.eq_ignore_ascii_case("balanced")
                || raw.eq_ignore_ascii_case("dev")
            {
                Ok(())
            } else {
                Err(ConfigSetError::InvalidValue)
            }
        }
        ConfigValueKind::VirtualizationExecution => {
            if raw.eq_ignore_ascii_case("latencycritical")
                || raw.eq_ignore_ascii_case("balanced")
                || raw.eq_ignore_ascii_case("background")
            {
                Ok(())
            } else {
                Err(ConfigSetError::InvalidValue)
            }
        }
        ConfigValueKind::VirtualizationGovernor => {
            if raw.eq_ignore_ascii_case("performance")
                || raw.eq_ignore_ascii_case("balanced")
                || raw.eq_ignore_ascii_case("efficiency")
            {
                Ok(())
            } else {
                Err(ConfigSetError::InvalidValue)
            }
        }
    }
}

pub(super) fn validate_runtime_override(
    key: &str,
    value: Option<&str>,
) -> Result<(), ConfigSetError> {
    match key {
        "vfs_library_api_exposed" => {
            if matches!(value, Some(raw) if is_truthy(raw)) && !cfg!(feature = "vfs") {
                return Err(ConfigSetError::InvalidValue);
            }
        }
        "network_library_api_exposed" => {
            if matches!(value, Some(raw) if is_truthy(raw)) && !cfg!(feature = "networking") {
                return Err(ConfigSetError::InvalidValue);
            }
        }
        "ipc_library_api_exposed" => {
            if matches!(value, Some(raw) if is_truthy(raw)) && !cfg!(feature = "ipc") {
                return Err(ConfigSetError::InvalidValue);
            }
        }
        "telemetry_enabled" => {
            if matches!(value, Some(raw) if is_truthy(raw)) && !cfg!(feature = "telemetry") {
                return Err(ConfigSetError::InvalidValue);
            }
        }
        "proc_config_api_exposed" | "sysctl_api_exposed" => {
            if matches!(value, Some(raw) if is_truthy(raw))
                && (!cfg!(feature = "vfs")
                    || !KernelConfig::is_vfs_library_api_exposed()
                    || matches!(KernelConfig::boundary_mode(), crate::config::BoundaryMode::Strict))
            {
                return Err(ConfigSetError::InvalidValue);
            }
        }
        "security_enforcement_enabled" => {
            if matches!(value, Some(raw) if is_truthy(raw))
                && !cfg!(any(
                    feature = "security",
                    feature = "security_acl",
                    feature = "security_capabilities",
                    feature = "security_sel4",
                    feature = "security_null"
                ))
            {
                return Err(ConfigSetError::InvalidValue);
            }
        }
        "capability_enforcement_enabled" => {
            if matches!(value, Some(raw) if is_truthy(raw))
                && !cfg!(any(
                    feature = "capabilities",
                    feature = "security_capabilities"
                ))
            {
                return Err(ConfigSetError::InvalidValue);
            }
        }
        "multi_user_enabled" | "credential_enforcement_enabled" => {
            if matches!(value, Some(raw) if is_truthy(raw)) && !cfg!(feature = "posix_process") {
                return Err(ConfigSetError::InvalidValue);
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_strict_critical_override(key: &str, value: Option<&str>) -> Result<(), ConfigSetError> {
    if !is_critical_runtime_key(key) {
        return Ok(());
    }

    let raw = value.ok_or(ConfigSetError::InvalidValue)?;
    let spec = KernelConfig::runtime_config_spec(key).ok_or(ConfigSetError::UnknownKey)?;

    match spec.value_kind {
        ConfigValueKind::Bool => {
            if !is_truthy(raw) {
                return Err(ConfigSetError::InvalidValue);
            }
        }
        ConfigValueKind::BoundaryMode => {
            if raw.eq_ignore_ascii_case("strict") {
                return Err(ConfigSetError::InvalidValue);
            }
        }
        _ => {}
    }

    Ok(())
}

pub(super) fn refresh_runtime_compat_surface_if_needed() {
    #[cfg(all(feature = "vfs", feature = "linux_compat"))]
    {
        let _ = crate::modules::linux_compat::ensure_runtime_compat_surface_state();
    }
}
