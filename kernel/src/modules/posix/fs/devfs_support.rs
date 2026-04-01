use super::*;
use super::file_types_support::StatelessDevice;

pub(super) fn devfs_context(fs_id: u32) -> Option<Arc<DevFs>> {
    DEVFS_CONTEXTS.lock().get(&fs_id).cloned()
}

pub(super) fn register_builtin_devfs_nodes(devfs: &Arc<DevFs>) {
    let _ = devfs.register_device_with_meta(
        "null",
        Box::new(|_| {
            Box::new(StatelessDevice {
                fill: 0,
                is_null: true,
            })
        }),
        DeviceMetadata::char_device(0o666, 0, 0, false),
    );
    let _ = devfs.register_device_with_meta(
        "zero",
        Box::new(|_| {
            Box::new(StatelessDevice {
                fill: 0,
                is_null: false,
            })
        }),
        DeviceMetadata::char_device(0o666, 0, 0, false),
    );
    let _ = devfs.register_device_with_meta(
        "random",
        Box::new(|_| {
            Box::new(StatelessDevice {
                fill: 0x5a,
                is_null: false,
            })
        }),
        DeviceMetadata::char_device(0o444, 0, 0, false),
    );
    let _ = devfs.register_device_with_meta(
        "urandom",
        Box::new(|_| {
            Box::new(StatelessDevice {
                fill: 0xa5,
                is_null: false,
            })
        }),
        DeviceMetadata::char_device(0o444, 0, 0, false),
    );
}

fn devfs_dynamic_net_meta() -> DeviceMetadata {
    DeviceMetadata::char_device(
        apply_devfs_policy_mode(crate::config::KernelConfig::devfs_net_mode()) as u32,
        crate::config::KernelConfig::devfs_default_uid(),
        crate::config::KernelConfig::devfs_net_gid(),
        true,
    )
}

fn devfs_dynamic_storage_meta() -> DeviceMetadata {
    DeviceMetadata::char_device(
        apply_devfs_policy_mode(crate::config::KernelConfig::devfs_storage_mode()) as u32,
        crate::config::KernelConfig::devfs_default_uid(),
        crate::config::KernelConfig::devfs_storage_gid(),
        true,
    )
}

pub(super) fn sync_devfs_runtime_nodes(devfs: &Arc<DevFs>) {
    if crate::config::KernelConfig::devfs_enable_hotplug_net_nodes()
        && has_virtio_runtime_driver_for_devfs()
    {
        if !devfs.has_device("net/virtio0") {
            let _ = devfs.register_device_with_meta(
                "net/virtio0",
                Box::new(|_| {
                    Box::new(StatelessDevice {
                        fill: 0,
                        is_null: true,
                    })
                }),
                devfs_dynamic_net_meta(),
            );
        }
    } else if devfs.has_device("net/virtio0") {
        let _ = devfs.unregister_device("net/virtio0");
    }

    if crate::config::KernelConfig::devfs_enable_hotplug_net_nodes()
        && has_e1000_runtime_driver_for_devfs()
    {
        if !devfs.has_device("net/e1000") {
            let _ = devfs.register_device_with_meta(
                "net/e1000",
                Box::new(|_| {
                    Box::new(StatelessDevice {
                        fill: 0,
                        is_null: true,
                    })
                }),
                devfs_dynamic_net_meta(),
            );
        }
    } else if devfs.has_device("net/e1000") {
        let _ = devfs.unregister_device("net/e1000");
    }

    let storage = storage_presence_for_devfs();
    if crate::config::KernelConfig::devfs_enable_hotplug_storage_nodes() && storage.has_nvme {
        if !devfs.has_device("nvme0n1") {
            let _ = devfs.register_device_with_meta(
                "nvme0n1",
                Box::new(|_| {
                    Box::new(StatelessDevice {
                        fill: 0,
                        is_null: true,
                    })
                }),
                devfs_dynamic_storage_meta(),
            );
        }
    } else if devfs.has_device("nvme0n1") {
        let _ = devfs.unregister_device("nvme0n1");
    }

    if crate::config::KernelConfig::devfs_enable_hotplug_storage_nodes() && storage.has_ahci {
        if !devfs.has_device("sda") {
            let _ = devfs.register_device_with_meta(
                "sda",
                Box::new(|_| {
                    Box::new(StatelessDevice {
                        fill: 0,
                        is_null: true,
                    })
                }),
                devfs_dynamic_storage_meta(),
            );
        }
    } else if devfs.has_device("sda") {
        let _ = devfs.unregister_device("sda");
    }

    if crate::config::KernelConfig::devfs_enable_hotplug_storage_nodes() && storage.has_virtio_block
    {
        if !devfs.has_device("vda") {
            let _ = devfs.register_device_with_meta(
                "vda",
                Box::new(|_| {
                    Box::new(StatelessDevice {
                        fill: 0,
                        is_null: true,
                    })
                }),
                devfs_dynamic_storage_meta(),
            );
        }
    } else if devfs.has_device("vda") {
        let _ = devfs.unregister_device("vda");
    }
}

#[cfg(feature = "drivers")]
fn has_virtio_runtime_driver_for_devfs() -> bool {
    crate::modules::drivers::has_virtio_runtime_driver()
}

#[cfg(not(feature = "drivers"))]
const fn has_virtio_runtime_driver_for_devfs() -> bool {
    false
}

#[cfg(feature = "drivers")]
fn has_e1000_runtime_driver_for_devfs() -> bool {
    crate::modules::drivers::has_e1000_runtime_driver()
}

#[cfg(not(feature = "drivers"))]
const fn has_e1000_runtime_driver_for_devfs() -> bool {
    false
}

#[derive(Debug, Clone, Copy, Default)]
struct DevFsStoragePresence {
    has_nvme: bool,
    has_ahci: bool,
    has_virtio_block: bool,
}

#[cfg(feature = "drivers")]
fn storage_presence_for_devfs() -> DevFsStoragePresence {
    use crate::modules::drivers::{BlockDriverKind, StorageManager};
    let mut out = DevFsStoragePresence::default();
    let mut global = StorageManager::global().lock();
    let Some(manager) = global.as_mut() else {
        return out;
    };
    let infos = manager.infos_vec();
    for info in infos {
        match info.kind {
            BlockDriverKind::Nvme => out.has_nvme = true,
            BlockDriverKind::Ahci => out.has_ahci = true,
            BlockDriverKind::VirtIoBlock => out.has_virtio_block = true,
        }
    }
    out
}

#[cfg(not(feature = "drivers"))]
const fn storage_presence_for_devfs() -> DevFsStoragePresence {
    DevFsStoragePresence {
        has_nvme: false,
        has_ahci: false,
        has_virtio_block: false,
    }
}
