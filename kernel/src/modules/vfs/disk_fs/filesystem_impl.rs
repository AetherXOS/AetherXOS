use super::*;

impl crate::modules::vfs::FileSystem for DiskFsLibrary {
    fn open(&self, path: &str, tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_open_file(mount_id, path, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.open(path, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend open not supported"),
        }
    }

    fn create(&self, path: &str, tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_create_file(mount_id, path, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.create(path, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend create not supported"),
        }
    }

    fn remove(&self, path: &str, tid: TaskId) -> Result<(), &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_remove_file(mount_id, path, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.remove(path, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend remove not supported"),
        }
    }

    fn mkdir(&self, path: &str, tid: TaskId) -> Result<(), &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_mkdir(mount_id, path, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.mkdir(path, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend mkdir not supported"),
        }
    }

    fn rmdir(&self, path: &str, tid: TaskId) -> Result<(), &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_rmdir(mount_id, path, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.rmdir(path, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend rmdir not supported"),
        }
    }

    fn readdir(
        &self,
        path: &str,
        tid: TaskId,
    ) -> Result<Vec<crate::modules::vfs::types::DirEntry>, &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_readdir(mount_id, path, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.readdir(path, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend readdir not supported"),
        }
    }

    fn stat(
        &self,
        path: &str,
        tid: TaskId,
    ) -> Result<crate::modules::vfs::types::FileStats, &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_metadata(mount_id, path, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.stat(path, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend stat not supported"),
        }
    }

    fn chmod(&self, path: &str, mode: u16, tid: TaskId) -> Result<(), &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_chmod(mount_id, path, mode, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.chmod(path, mode, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend chmod not supported"),
        }
    }

    fn chown(&self, path: &str, uid: u32, gid: u32, tid: TaskId) -> Result<(), &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_chown(mount_id, path, uid, gid, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.chown(path, uid, gid, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend chown not supported"),
        }
    }

    fn rename(&self, old: &str, new: &str, tid: TaskId) -> Result<(), &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_rename(mount_id, old, new, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.rename(old, new, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend rename not supported"),
        }
    }

    fn link(&self, old: &str, new: &str, tid: TaskId) -> Result<(), &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_link(mount_id, old, new, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.link(old, new, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend link not supported"),
        }
    }

    fn symlink(&self, target: &str, link: &str, tid: TaskId) -> Result<(), &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_symlink(mount_id, target, link, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.symlink(target, link, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend symlink not supported"),
        }
    }

    fn readlink(&self, path: &str, tid: TaskId) -> Result<alloc::string::String, &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_readlink(mount_id, path, tid)
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.readlink(path, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend readlink not supported"),
        }
    }

    fn set_times(
        &self,
        path: &str,
        atime: u64,
        mtime: u64,
        tid: TaskId,
    ) -> Result<(), &'static str> {
        match self.mode {
            DiskFsMode::Ram => {
                let mount_id = self.require_mount_id()?;
                crate::kernel::vfs_control::ramfs_set_times(
                    mount_id,
                    path,
                    atime as i64,
                    0,
                    mtime as i64,
                    0,
                    tid,
                )
            }
            #[cfg(feature = "vfs_ext4")]
            DiskFsMode::Ext4 => {
                if let Some(ext4) = &self.ext4 {
                    ext4.set_times(path, atime, mtime, tid)
                } else {
                    Err("ext4 backend missing")
                }
            }
            _ => Err("backend set_times not supported"),
        }
    }
}
