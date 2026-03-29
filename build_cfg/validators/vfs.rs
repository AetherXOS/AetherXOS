//! VFS config validation — mount limits, devfs policy, SLO.

use crate::build_cfg::config_types::VfsConfig;

const VALID_DEVFS_POLICIES: &[&str] = &["Strict", "Balanced", "Permissive"];

pub fn validate(c: &VfsConfig) -> Vec<String> {
    let mut e = Vec::new();

    if c.max_mounts == 0 || c.max_mounts > 65536 {
        e.push(format!(
            "vfs.max_mounts {} out of range [1, 65536]",
            c.max_mounts
        ));
    }
    if c.max_mount_path == 0 || c.max_mount_path > 65536 {
        e.push(format!(
            "vfs.max_mount_path {} out of range [1, 65536]",
            c.max_mount_path
        ));
    }
    if c.health_slo_ms == 0 {
        e.push("vfs.health_slo_ms must be > 0".to_string());
    }
    if !VALID_DEVFS_POLICIES.contains(&c.devfs_policy_profile.as_str()) {
        e.push(format!(
            "vfs.devfs_policy_profile '{}' invalid, expected one of {:?}",
            c.devfs_policy_profile, VALID_DEVFS_POLICIES
        ));
    }

    e
}
