use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;

use crate::interfaces::TaskId;
use crate::modules::vfs::backends::{self, FsBackendKind};
use crate::modules::vfs::disk_fs_support::validate_diskfs_path;
use crate::modules::vfs::error_context::{map_error, require_some};
use crate::modules::vfs::File;

#[path = "disk_fs/filesystem_impl.rs"]
mod filesystem_impl;

const ROOT_TASK_ID: TaskId = TaskId(0);
const ERR_MOUNT_UNAVAILABLE: &str = "mount unavailable";
const MOUNT_PATH_BUFFER_BYTES: usize = 512;
const IO_READ_CHUNK_BYTES: usize = 4096;
#[cfg(all(feature = "vfs_ext4", feature = "drivers"))]
const EXT4_PROBE_IMAGE_BYTES: usize = 1024 * 1024;
#[cfg(all(feature = "vfs_ext4", feature = "drivers"))]
const EXT4_PROBE_BLOCKS: u64 = 2048;

#[cfg(feature = "drivers")]
const SUPPORTED_STORAGE_KINDS: [crate::modules::drivers::BlockDriverKind; 3] = [
    crate::modules::drivers::BlockDriverKind::Nvme,
    crate::modules::drivers::BlockDriverKind::Ahci,
    crate::modules::drivers::BlockDriverKind::VirtIoBlock,
];

#[cfg(feature = "drivers")]
#[inline(always)]
fn has_supported_storage_device(manager: &mut crate::modules::drivers::StorageManager) -> bool {
    for kind in SUPPORTED_STORAGE_KINDS {
        if manager.first_by_kind(kind).is_some() {
            return true;
        }
    }
    false
}

#[cfg(feature = "drivers")]
#[inline(always)]
fn with_storage_manager<T>(
    offline_error: &'static str,
    f: impl FnOnce(&mut crate::modules::drivers::StorageManager) -> T,
) -> Result<T, &'static str> {
    let mut guard = crate::modules::drivers::StorageManager::global().lock();
    let manager = guard.as_mut().ok_or(offline_error)?;
    Ok(f(manager))
}

#[cfg(all(feature = "vfs_ext4", feature = "drivers"))]
#[inline(always)]
fn try_load_ext4_probe_image(
    manager: &mut crate::modules::drivers::StorageManager,
) -> Option<Vec<u8>> {
    for kind in SUPPORTED_STORAGE_KINDS {
        if let Some(dev) = manager.first_by_kind(kind) {
            let mut buf = alloc::vec![0u8; EXT4_PROBE_IMAGE_BYTES];
            if dev.read_blocks(0, EXT4_PROBE_BLOCKS, &mut buf).is_ok() {
                return Some(buf);
            }
        }
    }
    None
}

#[inline(always)]
fn default_io_policy() -> crate::modules::vfs::types::IoPolicy {
    if crate::config::KernelConfig::vfs_enable_buffered_io() {
        crate::modules::vfs::types::IoPolicy::Buffered
    } else {
        crate::modules::vfs::types::IoPolicy::Unbuffered
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskFsMode {
    Ram,
    Fat,
    Little,
    Ext4,
    Squash,
}

pub struct DiskFsLibrary {
    mount_id: Option<usize>,
    backend: FsBackendKind,
    mode: DiskFsMode,
    io_policy: crate::modules::vfs::types::IoPolicy,
    #[cfg(feature = "vfs_ext4")]
    ext4: Option<crate::modules::vfs::library_backends::Ext4Library>,
}

#[derive(Debug, Clone, Copy)]
pub struct DiskFsHealth {
    pub fatfs_enabled: bool,
    pub littlefs_enabled: bool,
    pub ext4_enabled: bool,
    pub squashfs_enabled: bool,
    pub fatfs_probes: u64,
    pub littlefs_probes: u64,
    pub ext4_probes: u64,
    pub squashfs_probes: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DiskFsMetadata {
    pub len: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub ino: u64,
    pub atime: i64,
    pub mtime: i64,
    pub ctime: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct DiskFsStats {
    pub f_type: u64,
    pub f_bsize: u64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_fsid: u64,
    pub f_namelen: u64,
}

impl DiskFsLibrary {
    #[inline(always)]
    fn current_task_id() -> TaskId {
        unsafe {
            crate::kernel::cpu_local::CpuLocal::try_get()
                .map(|cpu| TaskId(cpu.current_task.load(core::sync::atomic::Ordering::Relaxed)))
                .unwrap_or(ROOT_TASK_ID)
        }
    }

    #[inline(always)]
    fn require_mount_id(&self) -> Result<usize, &'static str> {
        self.mount_id.ok_or(ERR_MOUNT_UNAVAILABLE)
    }

    #[cfg(all(feature = "vfs", feature = "linux_compat"))]
    fn absolute_ramfs_path(&self, path: &str) -> Option<alloc::string::String> {
        if self.mode != DiskFsMode::Ram {
            return None;
        }
        let mount_id = self.mount_id?;
        let mut mount_buf = [0u8; MOUNT_PATH_BUFFER_BYTES];
        let mount_len = crate::kernel::vfs_control::mount_path_by_id(mount_id, &mut mount_buf)?;
        let mount_root = core::str::from_utf8(&mount_buf[..mount_len]).ok()?;
        Some(join_mount_and_relative_path(mount_root, path))
    }

    #[cfg(all(feature = "vfs", feature = "linux_compat"))]
    fn try_read_compat_virtual_path(
        &self,
        path: &str,
    ) -> Option<Result<alloc::vec::Vec<u8>, &'static str>> {
        let absolute = self.absolute_ramfs_path(path)?;
        match crate::modules::linux_compat::read_compat_config_path(&absolute) {
            Ok(rendered) => Some(Ok(rendered.into_bytes())),
            Err("unsupported compat path") => None,
            Err(err) => Some(Err(err)),
        }
    }

    #[cfg(all(feature = "vfs", feature = "linux_compat"))]
    fn try_write_compat_virtual_path(
        &self,
        path: &str,
        data: &[u8],
    ) -> Option<Result<(), &'static str>> {
        let absolute = self.absolute_ramfs_path(path)?;
        let value = core::str::from_utf8(data)
            .ok()?
            .trim_matches(char::from(0))
            .trim();
        match crate::modules::linux_compat::write_compat_config_path(&absolute, value) {
            Ok(()) => Some(Ok(())),
            Err("unsupported compat path") => None,
            Err(err) => Some(Err(err)),
        }
    }

    pub fn mount_ramfs_at(path: &str) -> Result<Self, &'static str> {
        let path = validate_diskfs_path(path)?;
        let mount_id = map_error(
            crate::kernel::vfs_control::mount_ramfs(path.as_bytes()),
            "mount failed",
        )?;
        Ok(Self {
            mount_id: Some(mount_id),
            backend: FsBackendKind::RamFs,
            mode: DiskFsMode::Ram,
            io_policy: default_io_policy(),
            #[cfg(feature = "vfs_ext4")]
            ext4: None,
        })
    }

    pub fn attach_existing(path: &str) -> Result<Self, &'static str> {
        let path = validate_diskfs_path(path)?;
        let mount_id = require_some(
            crate::kernel::vfs_control::mount_id_by_path(path.as_bytes()),
            "mount not found",
        )?;
        Ok(Self {
            mount_id: Some(mount_id),
            backend: FsBackendKind::RamFs,
            mode: DiskFsMode::Ram,
            io_policy: default_io_policy(),
            #[cfg(feature = "vfs_ext4")]
            ext4: None,
        })
    }

    #[cfg(feature = "linux_compat")]
    pub fn attach_compat_surface(path: &str) -> Result<Self, &'static str> {
        let path = validate_diskfs_path(path)?;
        crate::modules::linux_compat::mount_compat_surface_fs(&path)?;
        Self::attach_existing(&path)
    }

    pub fn load_initrd(&self, entries: &[(&str, &[u8])]) -> Result<usize, &'static str> {
        let mount_id = self.require_mount_id()?;
        if self.backend != FsBackendKind::RamFs {
            return Err("initrd load requires ramfs backend");
        }
        crate::kernel::vfs_control::load_initrd_entries(mount_id, entries)
    }

    pub fn read_all(&self, path: &str) -> Result<alloc::vec::Vec<u8>, &'static str> {
        #[cfg(feature = "vfs_telemetry")]
        let start_tick = crate::kernel::watchdog::global_tick();

        #[cfg(all(feature = "vfs", feature = "linux_compat"))]
        if let Some(result) = self.try_read_compat_virtual_path(path) {
            return result;
        }

        let tid = Self::current_task_id();
        let mut file = crate::modules::vfs::FileSystem::open(self, path, tid)?;
        let mut out = alloc::vec::Vec::new();
        let mut buf = [0u8; IO_READ_CHUNK_BYTES];
        loop {
            match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => out.extend_from_slice(&buf[..n]),
                Err(e) => return Err(e),
            }
        }
        #[cfg(feature = "vfs_telemetry")]
        {
            let delta = crate::kernel::watchdog::global_tick().saturating_sub(start_tick);
            crate::modules::vfs::telemetry::note_disk_read_latency(delta);
        }
        Ok(out)
    }

    pub fn write_all(&self, path: &str, data: &[u8]) -> Result<(), &'static str> {
        #[cfg(feature = "vfs_telemetry")]
        let start_tick = crate::kernel::watchdog::global_tick();

        #[cfg(all(feature = "vfs", feature = "linux_compat"))]
        if let Some(result) = self.try_write_compat_virtual_path(path, data) {
            return result;
        }

        let tid = Self::current_task_id();
        // Use create to ensure it exists and truncate to overwrite
        let mut file = crate::modules::vfs::FileSystem::create(self, path, tid)?;
        file.truncate(0)?;
        file.write(data)?;
        #[cfg(feature = "vfs_telemetry")]
        {
            let delta = crate::kernel::watchdog::global_tick().saturating_sub(start_tick);
            crate::modules::vfs::telemetry::note_disk_write_latency(delta);
        }
        Ok(())
    }

    pub fn with_backend(path: &str, backend: FsBackendKind) -> Result<Self, &'static str> {
        let path = validate_diskfs_path(path)?;
        if !backends::probe_backend(backend) {
            return Err("backend unavailable");
        }

        match backend {
            FsBackendKind::RamFs => Self::mount_ramfs_at(&path),
            FsBackendKind::FatFs => {
                #[cfg(feature = "drivers")]
                {
                    let device_found = with_storage_manager(
                        "fatfs mount failed: Storage Manager offline",
                        |manager| has_supported_storage_device(manager),
                    )?;
                    if device_found {
                        Err("fatfs bridge detected block device but DiskFs mount wiring is not enabled in this path")
                    } else {
                        Err("fatfs mount failed: no compatible block device found")
                    }
                }
                #[cfg(not(feature = "drivers"))]
                {
                    Err("fatfs mount failed: drivers feature disabled")
                }
            }
            FsBackendKind::LittleFs => {
                #[cfg(feature = "drivers")]
                {
                    let device_found = with_storage_manager(
                        "littlefs mount failed: Storage Manager offline",
                        |manager| has_supported_storage_device(manager),
                    )?;
                    if device_found {
                        Err("littlefs block device found but core mapping is pending complete HAL")
                    } else {
                        Err("littlefs mount failed: no compatible block device found")
                    }
                }
                #[cfg(not(feature = "drivers"))]
                {
                    Err("littlefs mount failed: drivers feature disabled")
                }
            }
            FsBackendKind::Ext4 => {
                #[cfg(feature = "vfs_ext4")]
                {
                    #[cfg(feature = "drivers")]
                    {
                        let loaded_image = with_storage_manager(
                            "ext4 mount failed: Storage Manager offline",
                            try_load_ext4_probe_image,
                        )?;

                        if let Some(image) = loaded_image {
                            Self::from_ext4_image(image)
                        } else {
                            Err("ext4 mount failed: unable to read from block device")
                        }
                    }
                    #[cfg(not(feature = "drivers"))]
                    {
                        Err("ext4 mount failed: drivers feature disabled")
                    }
                }
                #[cfg(not(feature = "vfs_ext4"))]
                {
                    Err("ext4 backend disabled")
                }
            }
            FsBackendKind::SquashFs => Err("squashfs mount bridge unavailable in DiskFs runtime path"),
        }
    }

    #[cfg(feature = "vfs_ext4")]
    pub fn from_ext4_image(image: Vec<u8>) -> Result<Self, &'static str> {
        let ext4 = map_error(
            crate::modules::vfs::library_backends::Ext4Library::load_from_bytes(image),
            "ext4 load failed",
        )?;
        Ok(Self {
            mount_id: None,
            backend: FsBackendKind::Ext4,
            mode: DiskFsMode::Ext4,
            io_policy: default_io_policy(),
            ext4: Some(ext4),
        })
    }

    pub fn set_io_policy(&mut self, policy: crate::modules::vfs::types::IoPolicy) {
        self.io_policy = policy;
    }

    pub fn io_policy(&self) -> crate::modules::vfs::types::IoPolicy {
        self.io_policy
    }

    pub fn mount_id(&self) -> Option<usize> {
        self.mount_id
    }

    pub fn backend(&self) -> FsBackendKind {
        self.backend
    }

    pub fn mode(&self) -> DiskFsMode {
        self.mode
    }

    pub fn supported_backends() -> backends::BackendMatrix {
        backends::supported_backends()
    }

    pub fn backend_inventory() -> [backends::BackendDescriptor; 5] {
        backends::backend_inventory()
    }

    pub fn probe_backend(backend: FsBackendKind) -> bool {
        backends::probe_backend(backend)
    }
}

#[cfg(all(feature = "vfs", feature = "linux_compat"))]
fn join_mount_and_relative_path(mount_root: &str, path: &str) -> alloc::string::String {
    let trimmed_path = path.trim();
    if mount_root == "/" {
        if trimmed_path.starts_with('/') {
            return trimmed_path.to_string();
        }
        return alloc::format!("/{}", trimmed_path);
    }

    if trimmed_path.is_empty() || trimmed_path == "/" {
        return mount_root.to_string();
    }

    if trimmed_path.starts_with('/') {
        alloc::format!("{}{}", mount_root.trim_end_matches('/'), trimmed_path)
    } else {
        alloc::format!("{}/{}", mount_root.trim_end_matches('/'), trimmed_path)
    }
}

impl DiskFsLibrary {
    pub fn health_report() -> DiskFsHealth {
        let support = backends::supported_backends();
        let probe = backends::backend_probe_stats();
        DiskFsHealth {
            fatfs_enabled: support.fatfs_enabled,
            littlefs_enabled: support.littlefs_enabled,
            ext4_enabled: support.ext4_enabled,
            squashfs_enabled: support.squashfs_enabled,
            fatfs_probes: probe.fatfs_probe_calls,
            littlefs_probes: probe.littlefs_probe_calls,
            ext4_probes: probe.ext4_probe_calls,
            squashfs_probes: probe.squashfs_probe_calls,
        }
    }

    pub fn backend_probe_stats() -> backends::BackendProbeStats {
        backends::backend_probe_stats()
    }

    #[cfg(feature = "vfs_fatfs")]
    pub fn fatfs_library() -> crate::modules::vfs::library_backends::FatFsLibrary {
        crate::modules::vfs::library_backends::FatFsLibrary::new()
    }

    #[cfg(feature = "vfs_littlefs")]
    pub fn littlefs_metadata_size() -> usize {
        crate::modules::vfs::library_backends::LittleFsLibrary::metadata_size()
    }

    #[cfg(feature = "vfs_ext4")]
    pub fn ext4_from_image(
        image: Vec<u8>,
    ) -> Result<crate::modules::vfs::library_backends::Ext4Library, ext4_view::Ext4Error> {
        crate::modules::vfs::library_backends::Ext4Library::load_from_bytes(image)
    }

    pub fn unmount(self) -> Result<(), &'static str> {
        if let Some(mount_id) = self.mount_id {
            map_error(crate::kernel::vfs_control::unmount(mount_id), "unmount failed")
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "disk_fs/tests.rs"]
mod tests;
