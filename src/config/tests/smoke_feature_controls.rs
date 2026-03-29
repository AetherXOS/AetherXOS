use super::*;

#[test_case]
fn feature_override_api_can_toggle_and_reset_runtime_gate() {
    KernelConfig::reset_runtime_overrides();

    let candidate = KernelConfig::feature_controls()
        .into_iter()
        .find(|f| f.runtime_gate_available && f.compile_enabled)
        .expect("feature with runtime gate");

    let baseline = KernelConfig::feature_control(candidate.name).expect("baseline feature");
    let forced_off =
        KernelConfig::set_feature_enabled(candidate.name, Some(false)).expect("force feature off");
    assert!(!forced_off.effective_enabled);

    let restored =
        KernelConfig::set_feature_enabled(candidate.name, None).expect("restore feature gate");
    assert_eq!(restored.name, baseline.name);
    assert_eq!(restored.effective_enabled, baseline.effective_enabled);

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn feature_override_batch_parser_supports_prefixes_and_resets() {
    KernelConfig::reset_runtime_overrides();

    let candidate = KernelConfig::feature_controls()
        .into_iter()
        .find(|f| f.runtime_gate_available && f.compile_enabled)
        .expect("feature with runtime gate");

    let applied = KernelConfig::apply_feature_override_batch_str(
        alloc::format!("feature.{}=off, reset_{}", candidate.name, candidate.name).as_str(),
    )
    .expect("apply feature batch");
    assert_eq!(applied, 2);

    let restored = KernelConfig::feature_control(candidate.name).expect("feature control");
    assert!(restored.compile_enabled);

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn feature_category_summaries_and_runtime_drift_are_consistent() {
    KernelConfig::reset_runtime_overrides();

    let summaries = KernelConfig::feature_category_summaries();
    assert!(!summaries.is_empty());
    assert!(summaries.iter().all(|item| item.total > 0));
    assert!(summaries.iter().all(|item| item.compile_enabled <= item.total));
    assert!(summaries.iter().all(|item| item.runtime_gateable <= item.total));
    assert!(summaries.iter().all(|item| item.effective_enabled <= item.total));

    let total = summaries.iter().map(|item| item.total).sum::<usize>();
    assert_eq!(total, KernelConfig::feature_controls().len());

    let baseline_drift = KernelConfig::feature_runtime_drift_count();

    if let Some(candidate) = KernelConfig::feature_controls()
        .into_iter()
        .find(|f| f.runtime_gate_available && f.compile_enabled)
    {
        let _ = KernelConfig::set_feature_enabled(candidate.name, Some(false)).expect("disable");
        let drift_after = KernelConfig::feature_runtime_drift_count();
        assert!(drift_after >= baseline_drift);
    }

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn linux_compat_readiness_matches_surface_gate_logic() {
    KernelConfig::reset_runtime_overrides();

    let readiness = KernelConfig::linux_compat_readiness();
    assert_eq!(
        readiness.effective_surface_enabled,
        KernelConfig::should_expose_linux_compat_surface()
    );

    let blockers = KernelConfig::linux_compat_blockers();
    if readiness.effective_surface_enabled {
        assert!(blockers.is_empty());
    } else {
        assert!(!blockers.is_empty());
    }

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn feature_override_batch_reports_error_index_for_invalid_entry() {
    KernelConfig::reset_runtime_overrides();

    let err = KernelConfig::apply_feature_override_batch_str(
        "telemetry=off, definitely_not_a_feature=on, vfs=off",
    )
    .expect_err("invalid feature entry should fail");
    assert_eq!(err.index, 1);
    assert_eq!(err.cause, ConfigSetError::UnknownKey);

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn linux_compat_blocker_details_are_sorted_and_next_action_present() {
    KernelConfig::reset_runtime_overrides();

    let details = KernelConfig::linux_compat_blocker_details();
    for pair in details.windows(2) {
        assert!(pair[0].severity <= pair[1].severity);
    }
    let next_action = KernelConfig::linux_compat_next_action();
    assert!(!next_action.is_empty());

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn linux_compat_next_action_matches_top_blocker() {
    KernelConfig::reset_runtime_overrides();

    let blockers = KernelConfig::linux_compat_blocker_details();
    let next = KernelConfig::linux_compat_next_action();
    if let Some(first) = blockers.first() {
        assert_eq!(next, first.next_action);
    } else {
        assert!(next.contains("ready"));
    }

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn linux_compat_blockers_are_deterministic_across_calls() {
    KernelConfig::reset_runtime_overrides();

    let first = KernelConfig::linux_compat_blocker_details();
    let second = KernelConfig::linux_compat_blocker_details();
    assert_eq!(first, second);

    let next = KernelConfig::linux_compat_next_action();
    if let Some(top) = first.first() {
        assert_eq!(next, top.next_action);
    }

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn linux_compat_readiness_regresses_across_boundary_profiles() {
    KernelConfig::reset_runtime_overrides();

    for mode in ["strict", "balanced", "compat"] {
        KernelConfig::set_by_key_str("library_boundary_mode", Some(mode))
            .expect("set boundary mode");
        let readiness = KernelConfig::linux_compat_readiness();
        let has_surface_inputs =
            readiness.vfs_api_exposed || readiness.network_api_exposed || readiness.ipc_api_exposed;

        if mode == "strict" {
            assert!(!readiness.boundary_allows_compat);
            assert!(!readiness.effective_surface_enabled);
        } else {
            assert!(readiness.boundary_allows_compat);
            assert_eq!(
                readiness.effective_surface_enabled,
                readiness.compile_linux_compat && has_surface_inputs
            );
        }
    }

    KernelConfig::reset_runtime_overrides();
}

#[test_case]
fn linux_compat_readiness_drops_when_all_surfaces_disabled() {
    KernelConfig::reset_runtime_overrides();

    KernelConfig::set_by_key_str("vfs_library_api_exposed", Some("false"))
        .expect("disable vfs surface");
    KernelConfig::set_by_key_str("network_library_api_exposed", Some("false"))
        .expect("disable network surface");
    KernelConfig::set_by_key_str("ipc_library_api_exposed", Some("false"))
        .expect("disable ipc surface");

    for mode in ["balanced", "compat"] {
        KernelConfig::set_by_key_str("library_boundary_mode", Some(mode))
            .expect("set boundary mode");
        let readiness = KernelConfig::linux_compat_readiness();
        assert!(readiness.boundary_allows_compat);
        assert!(!readiness.vfs_api_exposed);
        assert!(!readiness.network_api_exposed);
        assert!(!readiness.ipc_api_exposed);
        assert!(!readiness.effective_surface_enabled);
    }

    KernelConfig::reset_runtime_overrides();
}
