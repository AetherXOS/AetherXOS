use super::*;
use alloc::format;
use alloc::string::String;
use core::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEST_MOUNT: AtomicUsize = AtomicUsize::new(1);

fn unique_mount_path(prefix: &str) -> String {
    let id = NEXT_TEST_MOUNT.fetch_add(1, Ordering::Relaxed);
    format!("/{prefix}-{id}")
}

#[test_case]
fn backend_parity_ramfs_high_level_operations() {
    let mount_path = unique_mount_path("diskfs-parity");
    let fs = DiskFsLibrary::mount_ramfs_at(&mount_path).expect("mount ramfs");

    let dir = "/suite";
    let file = "/suite/test.txt";
    let renamed = "/suite/renamed.txt";
    let payload = b"diskfs-parity-payload";
    let tid = TaskId(0);

    crate::modules::vfs::FileSystem::mkdir(&fs, dir, tid).expect("mkdir");
    fs.write_all(file, payload).expect("write");
    let read_back = fs.read_all(file).expect("read");
    assert_eq!(read_back.as_slice(), payload);

    let listed = crate::modules::vfs::FileSystem::readdir(&fs, dir, tid).expect("readdir");
    assert!(listed.iter().any(|ent| ent.name == "test.txt"));

    let st = crate::modules::vfs::FileSystem::stat(&fs, file, tid).expect("stat");
    assert_eq!(st.size as usize, payload.len());

    crate::modules::vfs::FileSystem::rename(&fs, file, renamed, tid).expect("rename");
    assert!(crate::modules::vfs::FileSystem::open(&fs, renamed, tid).is_ok());
    assert!(crate::modules::vfs::FileSystem::open(&fs, file, tid).is_err());

    crate::modules::vfs::FileSystem::remove(&fs, renamed, tid).expect("remove");
    assert!(crate::modules::vfs::FileSystem::open(&fs, renamed, tid).is_err());
    crate::modules::vfs::FileSystem::rmdir(&fs, dir, tid).expect("rmdir");
    fs.unmount().expect("unmount");
}

#[test_case]
fn unsupported_backend_operations_fail_deterministically() {
    let fs = DiskFsLibrary {
        mount_id: None,
        backend: FsBackendKind::FatFs,
        mode: DiskFsMode::Fat,
        io_policy: crate::modules::vfs::types::IoPolicy::Buffered,
        #[cfg(feature = "vfs_ext4")]
        ext4: None,
    };

    let tid = TaskId(0);
    assert!(matches!(
        crate::modules::vfs::FileSystem::create(&fs, "/x", tid),
        Err("backend create not supported")
    ));
    assert!(matches!(
        crate::modules::vfs::FileSystem::open(&fs, "/x", tid),
        Err("backend open not supported")
    ));
    assert_eq!(
        crate::modules::vfs::FileSystem::stat(&fs, "/x", tid),
        Err("backend stat not supported")
    );
}

#[cfg(feature = "linux_compat")]
#[test_case]
fn compat_surface_diskfs_bridge_reads_and_writes_runtime_keys() {
    let mount_path = unique_mount_path("compat-surface");
    let fs = DiskFsLibrary::attach_compat_surface(&mount_path).expect("mount compat surface");

    crate::config::KernelConfig::reset_runtime_overrides();
    crate::config::KernelConfig::set_vfs_library_api_exposed(Some(true));
    crate::config::KernelConfig::set_sysctl_api_exposed(Some(true));
    crate::config::KernelConfig::set_library_boundary_mode(Some(
        crate::config::BoundaryMode::Balanced,
    ));

    let before = fs
        .read_all("/sys/aethercore/library/boundary_mode")
        .expect("read boundary mode");
    let before_str = core::str::from_utf8(&before).expect("utf8");
    assert!(before_str.contains("Balanced"));

    fs.write_all("/sys/aethercore/runtime/telemetry_enabled", b"false\n")
        .expect("write telemetry");
    assert!(!crate::config::KernelConfig::is_telemetry_enabled());

    let after = fs
        .read_all("/sys/aethercore/compat/telemetry_enabled")
        .expect("read telemetry value");
    let after_str = core::str::from_utf8(&after).expect("utf8");
    assert_eq!(after_str.trim(), "false");

    crate::config::KernelConfig::reset_runtime_overrides();
    fs.unmount().expect("unmount compat surface");
}
