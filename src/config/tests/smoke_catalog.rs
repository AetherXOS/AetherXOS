use super::*;

#[test_case]
fn cargo_feature_catalog_is_auto_exposed() {
    let names = KernelConfig::cargo_feature_names();
    assert!(!names.is_empty());
    assert!(KernelConfig::is_cargo_feature_enabled("sched_cfs").is_some());
    assert!(KernelConfig::is_scheduler_feature("sched_cfs"));
    assert_eq!(KernelConfig::cargo_feature_category("sched_cfs"), "scheduler");
    assert!(KernelConfig::enabled_scheduler_feature_count() >= 1);
    assert!(KernelConfig::primary_scheduler_feature().is_some());
}

#[test_case]
fn runtime_config_catalog_is_auto_exposed_and_grouped() {
    let keys = KernelConfig::runtime_config_keys();
    assert!(!keys.is_empty());
    let telemetry = keys.iter().find(|k| k.key == "telemetry.enabled");
    let epoll = keys.iter().find(|k| k.key == "network_epoll_max_events");
    let vfs_health = keys
        .iter()
        .find(|k| k.key == "vfs_health_max_mount_failure_rate_per_mille");
    assert!(telemetry.is_some());
    assert!(epoll.is_some());
    assert!(vfs_health.is_some());
    assert_eq!(
        KernelConfig::config_category_for_key("driver.network.slo.max_io_errors"),
        "driver.network"
    );
    assert_eq!(
        KernelConfig::config_category_for_key("scheduler.rt_period_ns"),
        "scheduler"
    );
    assert_eq!(
        KernelConfig::config_category_for_key("network.epoll.max_events"),
        "network"
    );
    assert_eq!(
        KernelConfig::config_category_for_key("vfs.health.max_mount_capacity_percent"),
        "vfs"
    );
}

#[test_case]
fn set_by_key_str_parses_and_validates() {
    KernelConfig::reset_runtime_overrides();

    assert!(KernelConfig::set_by_key_str("telemetry.enabled", Some("true")).is_ok());
    assert!(KernelConfig::is_telemetry_enabled());
    assert!(KernelConfig::set_by_key_str("debug.trace.enabled", Some("false")).is_ok());
    assert!(!KernelConfig::debug_trace_enabled());
    assert!(KernelConfig::set_by_key_str("serial.early.debug.enabled", Some("false")).is_ok());
    assert!(!KernelConfig::serial_early_debug_enabled());

    assert!(KernelConfig::set_by_key_str("network.tls_policy_profile", Some("strict")).is_ok());
    assert_eq!(
        KernelConfig::network_tls_policy_profile(),
        super::TlsPolicyProfile::Strict
    );
    assert!(KernelConfig::set_by_key_str("network.epoll.max_events", Some("2048")).is_ok());
    assert_eq!(KernelConfig::network_epoll_max_events(), 2048);
    assert!(KernelConfig::set_by_key_str(
        "vfs.health.max_mount_failure_rate_per_mille",
        Some("123")
    )
    .is_ok());
    assert_eq!(KernelConfig::vfs_health_max_mount_failure_rate_per_mille(), 123);

    assert!(matches!(
        KernelConfig::set_by_key_str("network.tls_policy_profile", Some("invalid")),
        Err(super::ConfigSetError::InvalidValue)
    ));
    assert!(matches!(
        KernelConfig::set_by_key_str("not.exists", Some("1")),
        Err(super::ConfigSetError::UnknownKey)
    ));

    KernelConfig::reset_runtime_overrides();
}
