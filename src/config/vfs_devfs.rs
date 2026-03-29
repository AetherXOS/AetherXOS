use core::sync::atomic::Ordering;

use super::{
    apply_profile_override, decode_bool_override, decode_devfs_policy_override,
    encode_bool_override, encode_devfs_policy_override, normalize_u16_override,
    normalize_u32_override, DevFsPolicyProfile, DevFsRuntimeProfile, KernelConfig,
    VfsRuntimeProfile, DEFAULT_DEVFS_DEFAULT_GID, DEFAULT_DEVFS_DEFAULT_MODE,
    DEFAULT_DEVFS_DEFAULT_UID, DEFAULT_DEVFS_ENABLE_HOTPLUG_NET_NODES,
    DEFAULT_DEVFS_ENABLE_HOTPLUG_STORAGE_NODES, DEFAULT_DEVFS_NET_GID, DEFAULT_DEVFS_NET_MODE,
    DEFAULT_DEVFS_POLICY_PROFILE, DEFAULT_DEVFS_STORAGE_GID, DEFAULT_DEVFS_STORAGE_MODE,
    DEFAULT_DISKFS_MAX_PATH_LEN, DEFAULT_VFS_ENABLE_BUFFERED_IO,
    DEFAULT_VFS_HEALTH_MAX_FAILURE_RATE_PER_MILLE, DEFAULT_VFS_HEALTH_MAX_MOUNT_CAPACITY_PERCENT,
    DEFAULT_VFS_HEALTH_MAX_PATH_VALIDATION_FAILURES, DEFAULT_VFS_HEALTH_SLO_MS,
    DEFAULT_VFS_MAX_MOUNTS, DEFAULT_VFS_MAX_MOUNT_PATH, DEVFS_DEFAULT_GID_OVERRIDE,
    DEVFS_DEFAULT_MODE_OVERRIDE, DEVFS_DEFAULT_UID_OVERRIDE,
    DEVFS_ENABLE_HOTPLUG_NET_NODES_OVERRIDE, DEVFS_ENABLE_HOTPLUG_STORAGE_NODES_OVERRIDE,
    DEVFS_NET_GID_OVERRIDE, DEVFS_NET_MODE_OVERRIDE, DEVFS_POLICY_PROFILE_OVERRIDE,
    DEVFS_STORAGE_GID_OVERRIDE, DEVFS_STORAGE_MODE_OVERRIDE, DISKFS_MAX_PATH_LEN_OVERRIDE,
    MAX_DEVFS_DEFAULT_MODE, MAX_DISKFS_MAX_PATH_LEN, MAX_VFS_HEALTH_MAX_FAILURE_RATE_PER_MILLE,
    MAX_VFS_HEALTH_MAX_MOUNT_CAPACITY_PERCENT, MAX_VFS_HEALTH_MAX_PATH_VALIDATION_FAILURES,
    MAX_VFS_MAX_MOUNTS, MAX_VFS_MAX_MOUNT_PATH, VFS_ENABLE_BUFFERED_IO_OVERRIDE,
    VFS_HEALTH_MAX_MOUNT_CAPACITY_PERCENT_OVERRIDE,
    VFS_HEALTH_MAX_MOUNT_FAILURE_RATE_PER_MILLE_OVERRIDE,
    VFS_HEALTH_MAX_PATH_VALIDATION_FAILURES_OVERRIDE,
    VFS_HEALTH_MAX_UNMOUNT_FAILURE_RATE_PER_MILLE_OVERRIDE, VFS_HEALTH_SLO_MS_OVERRIDE,
    VFS_MAX_MOUNTS_OVERRIDE, VFS_MAX_MOUNT_PATH_OVERRIDE,
};

impl KernelConfig {
    pub fn vfs_max_mounts() -> usize {
        let override_value = VFS_MAX_MOUNTS_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_VFS_MAX_MOUNTS
        } else {
            override_value.max(1).min(MAX_VFS_MAX_MOUNTS)
        }
    }

    pub fn set_vfs_max_mounts(value: Option<usize>) {
        VFS_MAX_MOUNTS_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn vfs_max_mount_path() -> usize {
        let override_value = VFS_MAX_MOUNT_PATH_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_VFS_MAX_MOUNT_PATH
        } else {
            override_value.max(2).min(MAX_VFS_MAX_MOUNT_PATH)
        }
    }

    pub fn set_vfs_max_mount_path(value: Option<usize>) {
        VFS_MAX_MOUNT_PATH_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn diskfs_max_path_len() -> usize {
        let override_value = DISKFS_MAX_PATH_LEN_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_DISKFS_MAX_PATH_LEN
        } else {
            override_value.max(8).min(MAX_DISKFS_MAX_PATH_LEN)
        }
    }

    pub fn set_diskfs_max_path_len(value: Option<usize>) {
        DISKFS_MAX_PATH_LEN_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn vfs_enable_buffered_io() -> bool {
        decode_bool_override(
            VFS_ENABLE_BUFFERED_IO_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_VFS_ENABLE_BUFFERED_IO,
        )
    }

    pub fn set_vfs_enable_buffered_io(value: Option<bool>) {
        VFS_ENABLE_BUFFERED_IO_OVERRIDE.store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn vfs_health_slo_ms() -> u64 {
        let val = VFS_HEALTH_SLO_MS_OVERRIDE.load(Ordering::Relaxed);
        if val == 0 {
            DEFAULT_VFS_HEALTH_SLO_MS
        } else {
            val
        }
    }

    pub fn set_vfs_health_slo_ms(value: Option<u64>) {
        VFS_HEALTH_SLO_MS_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn vfs_health_max_mount_failure_rate_per_mille() -> u64 {
        let override_value =
            VFS_HEALTH_MAX_MOUNT_FAILURE_RATE_PER_MILLE_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_VFS_HEALTH_MAX_FAILURE_RATE_PER_MILLE
        } else {
            override_value.min(MAX_VFS_HEALTH_MAX_FAILURE_RATE_PER_MILLE)
        }
    }

    pub fn set_vfs_health_max_mount_failure_rate_per_mille(value: Option<u64>) {
        VFS_HEALTH_MAX_MOUNT_FAILURE_RATE_PER_MILLE_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn vfs_health_max_unmount_failure_rate_per_mille() -> u64 {
        let override_value =
            VFS_HEALTH_MAX_UNMOUNT_FAILURE_RATE_PER_MILLE_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_VFS_HEALTH_MAX_FAILURE_RATE_PER_MILLE
        } else {
            override_value.min(MAX_VFS_HEALTH_MAX_FAILURE_RATE_PER_MILLE)
        }
    }

    pub fn set_vfs_health_max_unmount_failure_rate_per_mille(value: Option<u64>) {
        VFS_HEALTH_MAX_UNMOUNT_FAILURE_RATE_PER_MILLE_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn vfs_health_max_path_validation_failures() -> u64 {
        let override_value =
            VFS_HEALTH_MAX_PATH_VALIDATION_FAILURES_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_VFS_HEALTH_MAX_PATH_VALIDATION_FAILURES
        } else {
            override_value
                .max(1)
                .min(MAX_VFS_HEALTH_MAX_PATH_VALIDATION_FAILURES)
        }
    }

    pub fn set_vfs_health_max_path_validation_failures(value: Option<u64>) {
        VFS_HEALTH_MAX_PATH_VALIDATION_FAILURES_OVERRIDE
            .store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn vfs_health_max_mount_capacity_percent() -> usize {
        let override_value = VFS_HEALTH_MAX_MOUNT_CAPACITY_PERCENT_OVERRIDE.load(Ordering::Relaxed);
        if override_value == 0 {
            DEFAULT_VFS_HEALTH_MAX_MOUNT_CAPACITY_PERCENT
        } else {
            override_value
                .max(1)
                .min(MAX_VFS_HEALTH_MAX_MOUNT_CAPACITY_PERCENT)
        }
    }

    pub fn set_vfs_health_max_mount_capacity_percent(value: Option<usize>) {
        VFS_HEALTH_MAX_MOUNT_CAPACITY_PERCENT_OVERRIDE.store(value.unwrap_or(0), Ordering::Relaxed);
    }

    pub fn vfs_runtime_profile() -> VfsRuntimeProfile {
        VfsRuntimeProfile {
            enable_buffered_io: Self::vfs_enable_buffered_io(),
            health_slo_ms: Self::vfs_health_slo_ms(),
            diskfs_max_path_len: Self::diskfs_max_path_len(),
        }
    }

    pub fn vfs_cargo_profile() -> VfsRuntimeProfile {
        VfsRuntimeProfile {
            enable_buffered_io: DEFAULT_VFS_ENABLE_BUFFERED_IO,
            health_slo_ms: DEFAULT_VFS_HEALTH_SLO_MS,
            diskfs_max_path_len: DEFAULT_DISKFS_MAX_PATH_LEN,
        }
    }

    pub fn devfs_default_mode() -> u16 {
        normalize_u16_override(
            DEVFS_DEFAULT_MODE_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_DEVFS_DEFAULT_MODE,
            MAX_DEVFS_DEFAULT_MODE,
        )
    }

    pub fn set_devfs_default_mode(value: Option<u16>) {
        DEVFS_DEFAULT_MODE_OVERRIDE.store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn devfs_policy_profile() -> DevFsPolicyProfile {
        decode_devfs_policy_override(
            DEVFS_POLICY_PROFILE_OVERRIDE.load(Ordering::Relaxed),
            DevFsPolicyProfile::from_str(DEFAULT_DEVFS_POLICY_PROFILE),
        )
    }

    pub fn set_devfs_policy_profile(value: Option<DevFsPolicyProfile>) {
        DEVFS_POLICY_PROFILE_OVERRIDE.store(encode_devfs_policy_override(value), Ordering::Relaxed);
    }

    pub fn devfs_default_uid() -> u32 {
        normalize_u32_override(
            DEVFS_DEFAULT_UID_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_DEVFS_DEFAULT_UID,
        )
    }

    pub fn set_devfs_default_uid(value: Option<u32>) {
        DEVFS_DEFAULT_UID_OVERRIDE.store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn devfs_default_gid() -> u32 {
        normalize_u32_override(
            DEVFS_DEFAULT_GID_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_DEVFS_DEFAULT_GID,
        )
    }

    pub fn set_devfs_default_gid(value: Option<u32>) {
        DEVFS_DEFAULT_GID_OVERRIDE.store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn devfs_net_mode() -> u16 {
        normalize_u16_override(
            DEVFS_NET_MODE_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_DEVFS_NET_MODE,
            MAX_DEVFS_DEFAULT_MODE,
        )
    }

    pub fn set_devfs_net_mode(value: Option<u16>) {
        DEVFS_NET_MODE_OVERRIDE.store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn devfs_net_gid() -> u32 {
        normalize_u32_override(
            DEVFS_NET_GID_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_DEVFS_NET_GID,
        )
    }

    pub fn set_devfs_net_gid(value: Option<u32>) {
        DEVFS_NET_GID_OVERRIDE.store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn devfs_storage_mode() -> u16 {
        normalize_u16_override(
            DEVFS_STORAGE_MODE_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_DEVFS_STORAGE_MODE,
            MAX_DEVFS_DEFAULT_MODE,
        )
    }

    pub fn set_devfs_storage_mode(value: Option<u16>) {
        DEVFS_STORAGE_MODE_OVERRIDE.store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn devfs_storage_gid() -> u32 {
        normalize_u32_override(
            DEVFS_STORAGE_GID_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_DEVFS_STORAGE_GID,
        )
    }

    pub fn set_devfs_storage_gid(value: Option<u32>) {
        DEVFS_STORAGE_GID_OVERRIDE.store(value.unwrap_or(0) as usize, Ordering::Relaxed);
    }

    pub fn devfs_enable_hotplug_net_nodes() -> bool {
        decode_bool_override(
            DEVFS_ENABLE_HOTPLUG_NET_NODES_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_DEVFS_ENABLE_HOTPLUG_NET_NODES,
        )
    }

    pub fn set_devfs_enable_hotplug_net_nodes(value: Option<bool>) {
        DEVFS_ENABLE_HOTPLUG_NET_NODES_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn devfs_enable_hotplug_storage_nodes() -> bool {
        decode_bool_override(
            DEVFS_ENABLE_HOTPLUG_STORAGE_NODES_OVERRIDE.load(Ordering::Relaxed),
            DEFAULT_DEVFS_ENABLE_HOTPLUG_STORAGE_NODES,
        )
    }

    pub fn set_devfs_enable_hotplug_storage_nodes(value: Option<bool>) {
        DEVFS_ENABLE_HOTPLUG_STORAGE_NODES_OVERRIDE
            .store(encode_bool_override(value), Ordering::Relaxed);
    }

    pub fn devfs_runtime_profile() -> DevFsRuntimeProfile {
        DevFsRuntimeProfile {
            policy_profile: Self::devfs_policy_profile(),
            default_mode: Self::devfs_default_mode(),
            default_uid: Self::devfs_default_uid(),
            default_gid: Self::devfs_default_gid(),
            net_mode: Self::devfs_net_mode(),
            net_gid: Self::devfs_net_gid(),
            storage_mode: Self::devfs_storage_mode(),
            storage_gid: Self::devfs_storage_gid(),
            hotplug_net_nodes: Self::devfs_enable_hotplug_net_nodes(),
            hotplug_storage_nodes: Self::devfs_enable_hotplug_storage_nodes(),
        }
    }

    pub fn devfs_cargo_profile() -> DevFsRuntimeProfile {
        DevFsRuntimeProfile {
            policy_profile: DevFsPolicyProfile::from_str(DEFAULT_DEVFS_POLICY_PROFILE),
            default_mode: DEFAULT_DEVFS_DEFAULT_MODE,
            default_uid: DEFAULT_DEVFS_DEFAULT_UID,
            default_gid: DEFAULT_DEVFS_DEFAULT_GID,
            net_mode: DEFAULT_DEVFS_NET_MODE,
            net_gid: DEFAULT_DEVFS_NET_GID,
            storage_mode: DEFAULT_DEVFS_STORAGE_MODE,
            storage_gid: DEFAULT_DEVFS_STORAGE_GID,
            hotplug_net_nodes: DEFAULT_DEVFS_ENABLE_HOTPLUG_NET_NODES,
            hotplug_storage_nodes: DEFAULT_DEVFS_ENABLE_HOTPLUG_STORAGE_NODES,
        }
    }

    pub fn set_vfs_runtime_profile(value: Option<VfsRuntimeProfile>) {
        apply_profile_override(
            value,
            |profile| {
                Self::set_vfs_enable_buffered_io(Some(profile.enable_buffered_io));
                Self::set_vfs_health_slo_ms(Some(profile.health_slo_ms));
                Self::set_diskfs_max_path_len(Some(profile.diskfs_max_path_len));
            },
            || {
                Self::set_vfs_enable_buffered_io(None);
                Self::set_vfs_health_slo_ms(None);
                Self::set_diskfs_max_path_len(None);
            },
        );
    }

    pub fn set_devfs_runtime_profile(value: Option<DevFsRuntimeProfile>) {
        apply_profile_override(
            value,
            |profile| {
                Self::set_devfs_policy_profile(Some(profile.policy_profile));
                Self::set_devfs_default_mode(Some(profile.default_mode));
                Self::set_devfs_default_uid(Some(profile.default_uid));
                Self::set_devfs_default_gid(Some(profile.default_gid));
                Self::set_devfs_net_mode(Some(profile.net_mode));
                Self::set_devfs_net_gid(Some(profile.net_gid));
                Self::set_devfs_storage_mode(Some(profile.storage_mode));
                Self::set_devfs_storage_gid(Some(profile.storage_gid));
                Self::set_devfs_enable_hotplug_net_nodes(Some(profile.hotplug_net_nodes));
                Self::set_devfs_enable_hotplug_storage_nodes(Some(profile.hotplug_storage_nodes));
            },
            || {
                Self::set_devfs_policy_profile(None);
                Self::set_devfs_default_mode(None);
                Self::set_devfs_default_uid(None);
                Self::set_devfs_default_gid(None);
                Self::set_devfs_net_mode(None);
                Self::set_devfs_net_gid(None);
                Self::set_devfs_storage_mode(None);
                Self::set_devfs_storage_gid(None);
                Self::set_devfs_enable_hotplug_net_nodes(None);
                Self::set_devfs_enable_hotplug_storage_nodes(None);
            },
        );
    }
}
