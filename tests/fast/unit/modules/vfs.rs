use aethercore::config::KernelConfig;
use aethercore::modules::vfs::path::{normalize_str, path_components, valid_path};
use serial_test::serial;

use crate::common::ctx;

#[test]
#[serial]
fn path_validation_respects_runtime_length_override() {
    let _guard = ctx::lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    KernelConfig::reset_runtime_overrides();
    KernelConfig::set_diskfs_max_path_len(Some(8));

    assert!(valid_path("/abc/def"));
    assert!(!valid_path("relative/path"));
    assert!(!valid_path("/abcdefghi"));
    assert!(!valid_path("/bad\0path"));

    KernelConfig::reset_runtime_overrides();
}

#[test]
fn path_normalization_collapses_redundant_segments() {
    let normalized = normalize_str("/srv//logs/./kernel/../boot");

    assert_eq!(normalized.as_deref(), Some("/srv/logs/boot"));
    assert_eq!(path_components("/srv/logs/boot"), ["srv", "logs", "boot"]);
    assert_eq!(normalize_str("/../../").as_deref(), Some("/"));
}
