use super::*;
use super::utils::normalize_config_key;
use crate::config::ConfigValueKind;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[cfg(feature = "vfs")]
pub(super) fn build_snapshot_report(
    snapshot: &KernelConfigSnapshot,
    audit: &ConfigAuditStats,
) -> String {
    alloc::format!(
        "core.module_loader_max_load_segments={}\ncore.module_loader_max_total_image_bytes={}\ncore.launch_max_boot_image_bytes={}\ncore.launch_handoff_stage_timeout_epochs={}\nnetwork.sample_interval={}\nnetwork.tls_policy={:?}\nscheduler.cfs_latency_target_ns={}\ntelemetry.enabled={}\ntelemetry.history_len={}\nvfs.buffered_io={}\nlibrary.boundary_mode={:?}\nlibrary.expose_vfs_api={}\nlibrary.expose_proc_config_api={}\nlibrary.expose_sysctl_api={}\ncredentials.security_enforcement={}\ncredentials.capability_enforcement={}\ncredentials.multi_user={}\ncredentials.credential_enforcement={}\ncompat.expose_linux_surface={}\ncompat.attack_surface_budget={}\naudit.apply_attempts={}\naudit.apply_success={}\naudit.apply_failures={}\naudit.last_error_index={}\n",
        snapshot.core.module_loader_max_load_segments,
        snapshot.core.module_loader_max_total_image_bytes,
        snapshot.core.launch_max_boot_image_bytes,
        snapshot.core.launch_handoff_stage_timeout_epochs,
        snapshot.network.slo.sample_interval,
        snapshot.network.tls_policy_profile,
        snapshot.scheduler.cfs_latency_target_ns,
        snapshot.telemetry.enabled,
        snapshot.telemetry.history_len,
        snapshot.vfs.enable_buffered_io,
        snapshot.library_runtime.boundary_mode,
        snapshot.library_runtime.expose_vfs_api,
        snapshot.library_runtime.expose_proc_config_api,
        snapshot.library_runtime.expose_sysctl_api,
        snapshot.credentials.security_enforcement,
        snapshot.credentials.capability_enforcement,
        snapshot.credentials.multi_user,
        snapshot.credentials.credential_enforcement,
        snapshot.compat_surface.expose_linux_compat_surface,
        snapshot.compat_surface.attack_surface_budget,
        audit.apply_attempts,
        audit.apply_success,
        audit.apply_failures,
        audit
            .last_error_index
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string()),
    )
}

#[cfg(feature = "vfs")]
pub(super) fn build_feature_report(features: &[ConfigFeatureControl]) -> String {
    let mut out = String::new();
    for feature in features {
        let gate = feature.runtime_gate_key.unwrap_or("-");
        let _ = alloc::fmt::write(
            &mut out,
            format_args!(
                "{} category={} compile={} runtime_gate={} effective={}\n",
                feature.name,
                feature.category,
                feature.compile_enabled,
                gate,
                feature.effective_enabled
            ),
        );
    }
    out
}

#[cfg(feature = "vfs")]
pub(super) fn build_feature_summary_report(
    summaries: &[ConfigFeatureCategorySummary],
    drift_count: usize,
) -> String {
    let mut out = String::new();
    let _ = alloc::fmt::write(
        &mut out,
        format_args!("feature_runtime_drift_count={}\n", drift_count),
    );
    for summary in summaries {
        let _ = alloc::fmt::write(
            &mut out,
            format_args!(
                "category={} total={} compile_enabled={} runtime_gateable={} effective_enabled={}\n",
                summary.category,
                summary.total,
                summary.compile_enabled,
                summary.runtime_gateable,
                summary.effective_enabled,
            ),
        );
    }
    out
}

#[cfg(feature = "vfs")]
pub(super) fn build_linux_compat_readiness_report(
    readiness: &ConfigLinuxCompatReadiness,
    blockers: &[ConfigLinuxCompatBlocker],
    next_action: &str,
) -> String {
    let mut out = String::new();
    let _ = alloc::fmt::write(
        &mut out,
        format_args!(
            "compile_linux_compat={}\ncompile_vfs={}\nboundary_allows_compat={}\nvfs_api_exposed={}\nnetwork_api_exposed={}\nipc_api_exposed={}\nproc_config_api_exposed={}\nsysctl_api_exposed={}\neffective_surface_enabled={}\n",
            readiness.compile_linux_compat,
            readiness.compile_vfs,
            readiness.boundary_allows_compat,
            readiness.vfs_api_exposed,
            readiness.network_api_exposed,
            readiness.ipc_api_exposed,
            readiness.proc_config_api_exposed,
            readiness.sysctl_api_exposed,
            readiness.effective_surface_enabled,
        ),
    );

    if blockers.is_empty() {
        out.push_str("blockers=none\n");
    } else {
        for blocker in blockers {
            let _ = alloc::fmt::write(
                &mut out,
                format_args!(
                    "blocker={} severity={} next_action={}\n",
                    blocker.code,
                    blocker.severity.as_str(),
                    blocker.next_action,
                ),
            );
        }
    }

    let _ = alloc::fmt::write(
        &mut out,
        format_args!("recommended_next_action={}\n", next_action),
    );

    out
}

#[cfg(feature = "vfs")]
pub(super) fn build_runtime_keys_report(keys: &[String]) -> String {
    let mut out = String::new();
    for key in keys {
        out.push_str(key.as_str());
        out.push('\n');
    }
    out
}

#[cfg(feature = "vfs")]
pub(super) fn normalize_export_dir(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "/config".to_string();
    }
    let mut out = String::with_capacity(trimmed.len() + 1);
    if !trimmed.starts_with('/') {
        out.push('/');
    }
    out.push_str(trimmed.trim_end_matches('/'));
    out
}

pub(super) fn split_override_entries(raw: &str) -> Vec<String> {
    raw.split(|ch| matches!(ch, ',' | ';' | '\n' | '\r'))
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_string())
        .collect()
}

pub(super) fn parse_override_entry(
    raw: &str,
) -> Result<(String, Option<String>), (String, ConfigSetError)> {
    if let Some(eq) = raw.find('=') {
        let key = normalize_config_key(&raw[..eq]);
        let value = raw[eq + 1..].trim().to_string();
        if key.is_empty() {
            return Err((String::new(), ConfigSetError::UnknownKey));
        }
        if value.is_empty() {
            return Ok((key, None));
        }
        return Ok((key, Some(value)));
    }

    if let Some(rest) = raw.strip_prefix('!') {
        let key = normalize_config_key(rest);
        if key.is_empty() {
            return Err((String::new(), ConfigSetError::UnknownKey));
        }
        return Ok((key, Some("false".to_string())));
    }

    let normalized = normalize_config_key(raw);
    if normalized.is_empty() {
        return Err((String::new(), ConfigSetError::UnknownKey));
    }

    if let Some(key) = normalized.strip_prefix("reset_") {
        if key.is_empty() {
            return Err((normalized, ConfigSetError::UnknownKey));
        }
        return Ok((key.to_string(), None));
    }
    if let Some(key) = normalized.strip_prefix("unset_") {
        if key.is_empty() {
            return Err((normalized, ConfigSetError::UnknownKey));
        }
        return Ok((key.to_string(), None));
    }
    if let Some(key) = normalized.strip_prefix("no_") {
        if key.is_empty() {
            return Err((normalized, ConfigSetError::UnknownKey));
        }
        return Ok((key.to_string(), Some("false".to_string())));
    }

    match KernelConfig::runtime_config_spec(normalized.as_str()) {
        Some(spec) if spec.value_kind == ConfigValueKind::Bool => {
            Ok((normalized, Some("true".to_string())))
        }
        Some(_) => Err((normalized, ConfigSetError::TypeMismatch)),
        None => Err((normalized, ConfigSetError::UnknownKey)),
    }
}

#[cfg(all(test, target_os = "none"))]
mod tests {
    use super::*;

    #[test_case]
    fn parses_bool_shorthand_and_reset_entries() {
        let (key, value) = parse_override_entry("telemetry_enabled").expect("bool shorthand");
        assert_eq!(key, "telemetry_enabled");
        assert_eq!(value.as_deref(), Some("true"));

        let (key, value) = parse_override_entry("!telemetry_enabled").expect("bool false");
        assert_eq!(key, "telemetry_enabled");
        assert_eq!(value.as_deref(), Some("false"));

        let (key, value) = parse_override_entry("reset.telemetry_enabled").expect("reset");
        assert_eq!(key, "telemetry_enabled");
        assert_eq!(value, None);
    }

    #[test_case]
    fn rejects_non_bool_shorthand_without_value() {
        let err = parse_override_entry("launch_max_boot_image_bytes").unwrap_err();
        assert_eq!(err.0, "launch_max_boot_image_bytes");
        assert_eq!(err.1, ConfigSetError::TypeMismatch);
    }

    #[test_case]
    fn batch_apply_updates_runtime_override() {
        KernelConfig::reset_runtime_overrides();
        assert!(KernelConfig::is_telemetry_enabled());

        let applied = KernelConfig::apply_override_batch_str(
            "!telemetry_enabled, telemetry_history_len=2048",
        )
        .expect("apply config batch");
        assert_eq!(applied, 2);
        assert!(!KernelConfig::is_telemetry_enabled());
        assert_eq!(KernelConfig::telemetry_history_len(), 2048);

        KernelConfig::reset_runtime_overrides();
    }

    #[test_case]
    fn kernel_cmdline_parser_applies_prefixed_tokens() {
        KernelConfig::reset_runtime_overrides();
        let count = KernelConfig::apply_kernel_cmdline_overrides(
            "quiet cfg.telemetry_enabled=false cfg:telemetry_history_len=1024 config=network_runtime_poll_interval_min=4,!telemetry_runtime_summary_enabled",
        )
        .expect("apply cmdline config");
        assert_eq!(count, 4);
        assert!(!KernelConfig::is_telemetry_enabled());
        assert_eq!(KernelConfig::telemetry_history_len(), 1024);
        assert_eq!(KernelConfig::network_runtime_poll_interval_min(), 4);
        assert!(!KernelConfig::telemetry_runtime_summary_enabled());
        KernelConfig::reset_runtime_overrides();
    }

    #[cfg(feature = "vfs")]
    #[test_case]
    fn feature_summary_report_contains_drift_and_category_rows() {
        let rows = [
            ConfigFeatureCategorySummary {
                category: "telemetry",
                total: 2,
                compile_enabled: 2,
                runtime_gateable: 1,
                effective_enabled: 1,
            },
            ConfigFeatureCategorySummary {
                category: "networking",
                total: 3,
                compile_enabled: 2,
                runtime_gateable: 2,
                effective_enabled: 2,
            },
        ];

        let report = build_feature_summary_report(&rows, 4);
        assert!(report.contains("feature_runtime_drift_count=4"));
        assert!(report.contains("category=telemetry total=2"));
        assert!(report.contains("category=networking total=3"));
    }

    #[cfg(feature = "vfs")]
    #[test_case]
    fn readiness_report_includes_blockers_and_recommended_action() {
        let readiness = ConfigLinuxCompatReadiness {
            compile_linux_compat: true,
            compile_vfs: true,
            boundary_allows_compat: false,
            vfs_api_exposed: true,
            network_api_exposed: false,
            ipc_api_exposed: false,
            proc_config_api_exposed: false,
            sysctl_api_exposed: false,
            effective_surface_enabled: false,
        };
        let blockers = [ConfigLinuxCompatBlocker {
            code: "boundary_mode_strict_blocks_compat_surface",
            severity: ConfigBlockerSeverity::High,
            next_action: "set library_boundary_mode to Balanced or Compat at runtime",
        }];

        let report = build_linux_compat_readiness_report(
            &readiness,
            blockers.as_slice(),
            "set library_boundary_mode to Balanced or Compat at runtime",
        );
        assert!(report.contains("compile_linux_compat=true"));
        assert!(report.contains("effective_surface_enabled=false"));
        assert!(report.contains("blocker=boundary_mode_strict_blocks_compat_surface"));
        assert!(report.contains("severity=high"));
        assert!(report.contains("recommended_next_action=set library_boundary_mode to Balanced or Compat at runtime"));
    }

    #[cfg(feature = "vfs")]
    #[test_case]
    fn runtime_keys_report_preserves_line_boundaries() {
        let keys = [
            String::from("telemetry_enabled [telemetry] bool critical=true"),
            String::from("network_epoll_max_events [network] usize critical=false"),
        ];
        let report = build_runtime_keys_report(&keys);
        let lines: Vec<&str> = report.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], keys[0]);
        assert_eq!(lines[1], keys[1]);
    }
}
