use super::vfs_constants::*;
use super::vfs_matrix_data::{
    vfs_operation_weight, vfs_readiness_band, vfs_status_weight, VFS_OPERATION_SUPPORT_ROWS,
};
use alloc::vec::Vec;

pub(crate) fn build_vfs_behavior_report() -> String {
    let stats = crate::kernel::vfs_control::stats();
    let mut out = String::new();
    let _ = alloc::fmt::write(
        &mut out,
        format_args!(
            "vfs_max_mounts={}\nvfs_max_mount_path={}\nmount_attempts={}\nmount_success={}\nmount_failures={}\nunmount_attempts={}\nunmount_success={}\nunmount_failures={}\nunmount_by_path_attempts={}\nunmount_by_path_success={}\nunmount_by_path_failures={}\npath_validation_failures={}\ninitrd_load_calls={}\ninitrd_load_files={}\ninitrd_load_bytes={}\ninitrd_load_failures={}\ntotal_mounts={}\nlast_mount_id={}\nmounts_reachable={}\n",
            crate::config::KernelConfig::vfs_max_mounts(),
            crate::config::KernelConfig::vfs_max_mount_path(),
            stats.mount_attempts,
            stats.mount_success,
            stats.mount_failures,
            stats.unmount_attempts,
            stats.unmount_success,
            stats.unmount_failures,
            stats.unmount_by_path_attempts,
            stats.unmount_by_path_success,
            stats.unmount_by_path_failures,
            stats.path_validation_failures,
            stats.initrd_load_calls,
            stats.initrd_load_files,
            stats.initrd_load_bytes,
            stats.initrd_load_failures,
            stats.total_mounts,
            stats.last_mount_id,
            stats.total_mounts <= crate::config::KernelConfig::vfs_max_mounts(),
        ),
    );

    out
}

pub(crate) fn build_vfs_matrix_report() -> String {
    let features = crate::modules::vfs::linux_features::feature_inventory();
    let enabled_features = features.iter().filter(|feature| feature.enabled).count();
    let virtual_fs_enabled = features
        .iter()
        .filter(|feature| {
            matches!(
                feature.category,
                crate::modules::vfs::linux_features::FeatureCategory::VirtualFS
            ) && feature.enabled
        })
        .count();
    let device_enabled = features
        .iter()
        .filter(|feature| {
            matches!(
                feature.category,
                crate::modules::vfs::linux_features::FeatureCategory::Device
            ) && feature.enabled
        })
        .count();
    let xattr_stats = crate::modules::vfs::xattr::xattr_stats();
    let xattr_config = crate::modules::vfs::xattr::XattrConfig::default();
    let mut required_gaps = Vec::new();
    let mut weak_filesystems = Vec::new();
    let mut out = String::new();

    let _ = alloc::fmt::write(
        &mut out,
        format_args!(
            "feature_inventory_total={}\nfeature_inventory_enabled={}\nvirtualfs_enabled={}\ndevice_enabled={}\nmount_limit={}\nmount_path_limit={}\nxattr_max_name_len={}\nxattr_max_value_len={}\nxattr_max_per_inode={}\nxattr_get_calls={}\nxattr_set_calls={}\nxattr_remove_calls={}\nxattr_list_calls={}\n",
            features.len(),
            enabled_features,
            virtual_fs_enabled,
            device_enabled,
            crate::config::KernelConfig::vfs_max_mounts(),
            crate::config::KernelConfig::vfs_max_mount_path(),
            xattr_config.max_name_len,
            xattr_config.max_value_len,
            xattr_config.max_per_inode,
            xattr_stats.get_calls,
            xattr_stats.set_calls,
            xattr_stats.remove_calls,
            xattr_stats.list_calls,
        ),
    );

    let _ = alloc::fmt::write(&mut out, format_args!("feature_categories:\n"));
    for category in [
        crate::modules::vfs::linux_features::FeatureCategory::VirtualFS,
        crate::modules::vfs::linux_features::FeatureCategory::Device,
        crate::modules::vfs::linux_features::FeatureCategory::Process,
        crate::modules::vfs::linux_features::FeatureCategory::Memory,
        crate::modules::vfs::linux_features::FeatureCategory::Network,
        crate::modules::vfs::linux_features::FeatureCategory::Security,
        crate::modules::vfs::linux_features::FeatureCategory::Scheduler,
        crate::modules::vfs::linux_features::FeatureCategory::IPC,
    ] {
        let total = features.iter().filter(|feature| feature.category == category).count();
        let enabled = features
            .iter()
            .filter(|feature| feature.category == category && feature.enabled)
            .count();
        let _ = alloc::fmt::write(
            &mut out,
            format_args!("category={} total={} enabled={}\n", category.as_str(), total, enabled),
        );
    }

    let _ = alloc::fmt::write(&mut out, format_args!("operation_matrix:\n"));
    for row in VFS_OPERATION_SUPPORT_ROWS {
        let _ = alloc::fmt::write(
            &mut out,
            format_args!(
                "op={} default={} ramfs={} tmpfs={} devfs={} procfs={} sysfs={} disk_fs={} writable_overlay={}\n",
                row.operation,
                row.default_trait,
                row.ramfs,
                row.tmpfs,
                row.devfs,
                row.procfs,
                row.sysfs,
                row.disk_fs,
                row.writable_overlay,
            ),
        );
    }

    let _ = alloc::fmt::write(&mut out, format_args!("matrix_scores:\n"));
    let mut score_total = 0u32;
    for fs_name in VFS_FS_NAMES {
        let mut weighted_sum = 0u32;
        let mut weighted_max = 0u32;
        for row in VFS_OPERATION_SUPPORT_ROWS {
            let op_weight = vfs_operation_weight(row.default_trait);
            let status = row.status_for_fs(fs_name);
            let status_weight = vfs_status_weight(status);
            weighted_sum = weighted_sum.saturating_add(status_weight.saturating_mul(op_weight));
            weighted_max = weighted_max.saturating_add(100u32.saturating_mul(op_weight));
            if row.default_trait == VFS_TRAIT_REQUIRED && status == VFS_STATUS_UNSUPPORTED {
                required_gaps.push(alloc::format!("{}:{}", fs_name, row.operation));
            }
        }
        let score = if weighted_max == 0 {
            0
        } else {
            weighted_sum.saturating_mul(100) / weighted_max
        };
        score_total = score_total.saturating_add(score);
        let band = vfs_readiness_band(score);
        if band != "strong" {
            weak_filesystems.push((fs_name, score, band));
        }
        let _ = alloc::fmt::write(
            &mut out,
            format_args!("fs={} readiness_score={} band={}\n", fs_name, score, band),
        );
    }

    let _ = alloc::fmt::write(&mut out, format_args!("operation_scores:\n"));
    let mut operation_hotspots = Vec::new();
    for row in VFS_OPERATION_SUPPORT_ROWS {
        let mut op_sum = 0u32;
        for fs_name in VFS_FS_NAMES {
            let status_weight = vfs_status_weight(row.status_for_fs(fs_name));
            op_sum = op_sum.saturating_add(status_weight);
        }
        let op_score = if VFS_FS_NAMES.is_empty() {
            0
        } else {
            op_sum / VFS_FS_NAMES.len() as u32
        };
        let _ = alloc::fmt::write(
            &mut out,
            format_args!("op={} aggregate_score={}\n", row.operation, op_score),
        );
        if op_score < VFS_OPERATION_HOTSPOT_THRESHOLD
            || (row.default_trait == VFS_TRAIT_REQUIRED
                && op_score < VFS_REQUIRED_OPERATION_HOTSPOT_THRESHOLD)
        {
            operation_hotspots.push((row.operation, op_score, row.default_trait));
        }
    }

    let matrix_overall_score = if VFS_FS_NAMES.is_empty() {
        0
    } else {
        score_total / VFS_FS_NAMES.len() as u32
    };
    let _ = alloc::fmt::write(
        &mut out,
        format_args!(
            "matrix_overall_score={}\nrequired_operation_gap_count={}\n",
            matrix_overall_score,
            required_gaps.len()
        ),
    );

    let matrix_gate = if !required_gaps.is_empty() {
        "fail"
    } else if matrix_overall_score < VFS_MATRIX_WARN_THRESHOLD {
        "warn"
    } else {
        "pass"
    };
    let _ = alloc::fmt::write(&mut out, format_args!("matrix_gate={}\n", matrix_gate));
    if !required_gaps.is_empty() {
        let _ = alloc::fmt::write(
            &mut out,
            format_args!(
                "matrix_gate_reason=required operation gaps present ({})\n",
                required_gaps.len()
            ),
        );
    } else if matrix_overall_score < VFS_MATRIX_WARN_THRESHOLD {
        let _ = alloc::fmt::write(
            &mut out,
            format_args!(
                "matrix_gate_reason=overall readiness score below threshold ({})\n",
                matrix_overall_score
            ),
        );
    } else {
        out.push_str("matrix_gate_reason=none\n");
    }

    if required_gaps.is_empty() {
        out.push_str("required_operation_gaps=none\n");
    } else {
        for gap in required_gaps.iter() {
            let _ = alloc::fmt::write(&mut out, format_args!("required_operation_gap={}\n", gap));
        }
    }

    let _ = alloc::fmt::write(&mut out, format_args!("required_gaps_by_fs:\n"));
    for fs_name in VFS_FS_NAMES {
        let fs_prefix = alloc::format!("{}:", fs_name);
        let gap_count = required_gaps
            .iter()
            .filter(|gap| gap.starts_with(fs_prefix.as_str()))
            .count();
        let _ = alloc::fmt::write(
            &mut out,
            format_args!("fs={} required_gap_count={}\n", fs_name, gap_count),
        );
    }

    let _ = alloc::fmt::write(
        &mut out,
        format_args!("operation_hotspots_count={}\n", operation_hotspots.len()),
    );
    if operation_hotspots.is_empty() {
        out.push_str("operation_hotspot=none\n");
    } else {
        for (operation, score, default_trait) in operation_hotspots.iter() {
            let _ = alloc::fmt::write(
                &mut out,
                format_args!(
                    "operation_hotspot=op:{} score:{} default:{}\n",
                    operation, score, default_trait
                ),
            );
        }
    }

    operation_hotspots.sort_by_key(|(_, score, default_trait)| {
        let required_rank = if *default_trait == VFS_TRAIT_REQUIRED { 0u8 } else { 1u8 };
        (required_rank, *score)
    });
    let focus_len = core::cmp::min(3, operation_hotspots.len());
    let _ = alloc::fmt::write(&mut out, format_args!("next_focus_count={}\n", focus_len));
    if focus_len == 0 {
        out.push_str("next_focus=none\n");
    } else {
        for (operation, score, default_trait) in operation_hotspots.iter().take(focus_len) {
            let _ = alloc::fmt::write(
                &mut out,
                format_args!(
                    "next_focus=op:{} score:{} default:{}\n",
                    operation, score, default_trait
                ),
            );
        }
    }

    let _ = alloc::fmt::write(
        &mut out,
        format_args!("weak_filesystems_count={}\n", weak_filesystems.len()),
    );
    let strong_filesystems_count = VFS_FS_NAMES.len().saturating_sub(weak_filesystems.len());
    let _ = alloc::fmt::write(
        &mut out,
        format_args!("strong_filesystems_count={}\n", strong_filesystems_count),
    );
    if weak_filesystems.is_empty() {
        out.push_str("weak_filesystem=none\n");
        out.push_str("weakest_filesystem=none\n");
    } else {
        weak_filesystems.sort_by_key(|(_, score, _)| *score);
        let (weakest_fs, weakest_score, weakest_band) = weak_filesystems[0];
        let _ = alloc::fmt::write(
            &mut out,
            format_args!(
                "weakest_filesystem=fs:{} score:{} band:{}\n",
                weakest_fs, weakest_score, weakest_band
            ),
        );
        for (fs_name, score, band) in weak_filesystems {
            let _ = alloc::fmt::write(
                &mut out,
                format_args!("weak_filesystem=fs:{} score:{} band:{}\n", fs_name, score, band),
            );
        }
    }

    let recommended_action = if matrix_gate == "fail" {
        "close required gaps on weakest filesystems before optional parity"
    } else if matrix_gate == "warn" {
        "raise next_focus operation scores above watch threshold"
    } else {
        "maintain strong filesystems and expand differential traces"
    };
    let _ = alloc::fmt::write(
        &mut out,
        format_args!("recommended_action={}\n", recommended_action),
    );

    let _ = alloc::fmt::write(&mut out, format_args!("library_backends:\n"));
    #[cfg(feature = "vfs_library_backends")]
    {
        let backends = crate::modules::vfs::library_backend_inventory();
        let _ = alloc::fmt::write(&mut out, format_args!("backend_count={}\n", backends.len()));
        for backend in backends {
            let _ = alloc::fmt::write(
                &mut out,
                format_args!(
                    "backend={} feature={} target_support={} maturity={}\n",
                    backend.name, backend.feature, backend.target_support, backend.maturity
                ),
            );
        }
    }
    #[cfg(not(feature = "vfs_library_backends"))]
    {
        out.push_str("backend_count=0\nbackend_inventory=feature_disabled\n");
    }

    out
}

pub(crate) fn build_vfs_focus_report() -> String {
    let matrix = build_vfs_matrix_report();
    let mut out = String::new();
    for line in matrix.lines() {
        if line.starts_with("matrix_gate=")
            || line.starts_with("matrix_gate_reason=")
            || line.starts_with("matrix_overall_score=")
            || line.starts_with("required_operation_gap_count=")
            || line.starts_with("next_focus_count=")
            || line.starts_with("next_focus=")
            || line.starts_with("weak_filesystems_count=")
            || line.starts_with("weak_filesystem=")
            || line.starts_with("strong_filesystems_count=")
            || line.starts_with("weakest_filesystem=")
            || line.starts_with("recommended_action=")
        {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}
