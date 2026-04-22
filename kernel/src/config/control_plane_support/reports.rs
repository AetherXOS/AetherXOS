use super::super::*;
use alloc::string::{String, ToString};

#[cfg(feature = "vfs")]
pub(crate) fn build_snapshot_report(
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
pub(crate) fn build_feature_report(features: &[ConfigFeatureControl]) -> String {
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
pub(crate) fn build_feature_summary_report(
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
pub(crate) fn build_linux_compat_readiness_report(
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
pub(crate) fn build_security_posture_report(
    snapshot: &crate::kernel::security_posture::SecurityPostureSnapshot,
    gate: &crate::kernel::security_posture::SecurityStrictGateReport,
    release_gate: &crate::kernel::security_posture::SecurityReleaseGateDecision,
) -> String {
    let mut out = String::new();
    let _ = alloc::fmt::write(
        &mut out,
        format_args!(
            "deployment_context={}\nboundary_mode={:?}\nsecurity_enforcement_enabled={}\ncapability_enforcement_enabled={}\nmulti_user_enabled={}\nlibrary_expose_vfs_api={}\nlibrary_expose_network_api={}\nlibrary_expose_ipc_api={}\nlibrary_expose_proc_config_api={}\nlibrary_expose_sysctl_api={}\nexpose_linux_compat_surface={}\ncompat_attack_surface_budget={}\nexec_elf_require_absolute_interp_path={}\nexec_elf_enforce_interp_path_sanitization={}\nexec_elf_enforce_system_loader_paths={}\nexec_elf_enforce_segment_congruence={}\nexec_auxv_enforce_handoff_contract={}\nexec_auxv_require_phdr_triplet={}\nnamespace_support_enabled={}\nnamespace_lifecycle_sane={}\nnamespace_policy_passed={}\nnamespace_creates={}\nnamespace_destroys={}\nnamespace_unshare_calls={}\nnamespace_setns_calls={}\nsyscall_contract_checks={}\nsyscall_contract_failures={}\nsyscall_contract_passed={}\nstrict_gate_passed={}\nstrict_gate_deployment_context={}\nrelease_gate_blocked={}\nrelease_gate_deployment_context={}\nrelease_gate_reason_count={}\n",
            snapshot.deployment_context.as_str(),
            snapshot.boundary_mode,
            snapshot.security_enforcement_enabled,
            snapshot.capability_enforcement_enabled,
            snapshot.multi_user_enabled,
            snapshot.library_expose_vfs_api,
            snapshot.library_expose_network_api,
            snapshot.library_expose_ipc_api,
            snapshot.library_expose_proc_config_api,
            snapshot.library_expose_sysctl_api,
            snapshot.expose_linux_compat_surface,
            snapshot.compat_attack_surface_budget,
            snapshot.exec_elf_require_absolute_interp_path,
            snapshot.exec_elf_enforce_interp_path_sanitization,
            snapshot.exec_elf_enforce_system_loader_paths,
            snapshot.exec_elf_enforce_segment_congruence,
            snapshot.exec_auxv_enforce_handoff_contract,
            snapshot.exec_auxv_require_phdr_triplet,
            snapshot.namespace_support_enabled,
            snapshot.namespace_lifecycle_sane,
            snapshot.namespace_policy_passed,
            snapshot.namespace_creates,
            snapshot.namespace_destroys,
            snapshot.namespace_unshare_calls,
            snapshot.namespace_setns_calls,
            snapshot.syscall_contract_checks,
            snapshot.syscall_contract_failures,
            snapshot.syscall_contract_passed,
            gate.passed,
            gate.deployment_context.as_str(),
            release_gate.blocked,
            release_gate.deployment_context.as_str(),
            release_gate.reasons.len(),
        ),
    );

    if gate.reasons.is_empty() {
        out.push_str("strict_gate_reasons=none\n");
    } else {
        for reason in gate.reasons.iter() {
            let _ = alloc::fmt::write(&mut out, format_args!("strict_gate_reason={}\n", reason));
        }
    }

    out
}

#[cfg(feature = "vfs")]
pub(crate) fn build_runtime_keys_report(keys: &[String]) -> String {
    let mut out = String::new();
    for key in keys {
        out.push_str(key.as_str());
        out.push('\n');
    }
    out
}

#[cfg(feature = "vfs")]
pub(crate) fn normalize_export_dir(path: &str) -> String {
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
