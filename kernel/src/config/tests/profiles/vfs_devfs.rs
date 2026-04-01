use super::*;

#[test_case]
fn vfs_runtime_profile_roundtrip_and_reset() {
    KernelConfig::reset_runtime_overrides();

    let profile = super::VfsRuntimeProfile {
        enable_buffered_io: false,
        health_slo_ms: 250,
        diskfs_max_path_len: 200,
    };
    KernelConfig::set_vfs_runtime_profile(Some(profile));

    let got = KernelConfig::vfs_runtime_profile();
    assert_eq!(got, profile);

    KernelConfig::set_vfs_runtime_profile(None);
    let reset = KernelConfig::vfs_runtime_profile();
    assert_eq!(
        reset.enable_buffered_io,
        super::DEFAULT_VFS_ENABLE_BUFFERED_IO
    );
    assert_eq!(reset.health_slo_ms, super::DEFAULT_VFS_HEALTH_SLO_MS);
    assert_eq!(
        reset.diskfs_max_path_len,
        super::DEFAULT_DISKFS_MAX_PATH_LEN
    );
}

#[test_case]
fn devfs_runtime_profile_roundtrip_and_reset() {
    KernelConfig::reset_runtime_overrides();

    let profile = super::DevFsRuntimeProfile {
        policy_profile: super::DevFsPolicyProfile::Dev,
        default_mode: 0o600,
        default_uid: 1000,
        default_gid: 1000,
        net_mode: 0o660,
        net_gid: 2000,
        storage_mode: 0o640,
        storage_gid: 3000,
        hotplug_net_nodes: false,
        hotplug_storage_nodes: false,
    };
    KernelConfig::set_devfs_runtime_profile(Some(profile));

    let got = KernelConfig::devfs_runtime_profile();
    assert_eq!(got, profile);

    KernelConfig::set_devfs_runtime_profile(None);
    let reset = KernelConfig::devfs_runtime_profile();
    assert_eq!(
        reset.policy_profile,
        super::DevFsPolicyProfile::from_str(super::DEFAULT_DEVFS_POLICY_PROFILE)
    );
    assert_eq!(reset.default_mode, super::DEFAULT_DEVFS_DEFAULT_MODE);
    assert_eq!(reset.default_uid, super::DEFAULT_DEVFS_DEFAULT_UID);
    assert_eq!(reset.default_gid, super::DEFAULT_DEVFS_DEFAULT_GID);
    assert_eq!(reset.net_mode, super::DEFAULT_DEVFS_NET_MODE);
    assert_eq!(reset.net_gid, super::DEFAULT_DEVFS_NET_GID);
    assert_eq!(reset.storage_mode, super::DEFAULT_DEVFS_STORAGE_MODE);
    assert_eq!(reset.storage_gid, super::DEFAULT_DEVFS_STORAGE_GID);
    assert_eq!(
        reset.hotplug_net_nodes,
        super::DEFAULT_DEVFS_ENABLE_HOTPLUG_NET_NODES
    );
    assert_eq!(
        reset.hotplug_storage_nodes,
        super::DEFAULT_DEVFS_ENABLE_HOTPLUG_STORAGE_NODES
    );
}
