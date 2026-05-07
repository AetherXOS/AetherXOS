extern crate alloc;

use alloc::vec::Vec;

#[cfg(feature = "vfs_ext4")]
use crate::interfaces::TaskId;
use alloc::boxed::Box;
#[cfg(feature = "vfs_ext4")]
use alloc::string::{String, ToString};
#[derive(Debug, Clone, Copy)]
pub struct LibraryBackendDescriptor {
    pub name: &'static str,
    pub feature: &'static str,
    pub target_support: &'static str,
    pub maturity: &'static str,
}

pub fn library_backend_inventory() -> Vec<LibraryBackendDescriptor> {
    #[allow(unused_mut)]
    let mut inventory = Vec::new();

    #[cfg(feature = "vfs_fatfs")]
    {
        inventory.push(LibraryBackendDescriptor {
            name: "FatFsLibrary",
            feature: "vfs_fatfs",
            target_support: if cfg!(target_os = "none") {
                "kernel hook (target_os=none)"
            } else {
                "host/compatible target adapter"
            },
            maturity: "adapter baseline",
        });
    }

    #[cfg(feature = "vfs_littlefs")]
    {
        inventory.push(LibraryBackendDescriptor {
            name: "LittleFsLibrary",
            feature: "vfs_littlefs",
            target_support: "feature-gated",
            maturity: "adapter baseline",
        });
    }

    #[cfg(feature = "vfs_ext4")]
    {
        inventory.push(LibraryBackendDescriptor {
            name: "Ext4Library",
            feature: "vfs_ext4",
            target_support: "feature-gated",
            maturity: "overlay-writeback adapter",
        });
    }

    #[cfg(feature = "vfs_squashfs")]
    {
        inventory.push(LibraryBackendDescriptor {
            name: "SquashFs hook",
            feature: "vfs_squashfs",
            target_support: "feature-gated",
            maturity: "bridge hook",
        });
    }

    inventory
}

#[cfg(all(feature = "vfs_fatfs", not(target_os = "none")))]
pub struct FatFsLibrary;

#[cfg(all(feature = "vfs_fatfs", not(target_os = "none")))]
impl FatFsLibrary {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(all(feature = "vfs_fatfs", target_os = "none"))]
pub struct FatFsLibrary;

#[cfg(all(feature = "vfs_fatfs", target_os = "none"))]
impl FatFsLibrary {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "vfs_littlefs")]
pub struct LittleFsLibrary;

#[cfg(feature = "vfs_littlefs")]
impl LittleFsLibrary {
    pub fn metadata_size() -> usize {
        core::mem::size_of::<littlefs2_core::Metadata>()
    }
}

#[cfg(feature = "vfs_ext4")]
pub struct Ext4Library {
    fs: ext4_view::Ext4,
}

unsafe impl Send for Ext4Library {}
unsafe impl Sync for Ext4Library {}

#[cfg(feature = "vfs_ext4")]
#[derive(Debug, Clone, Copy)]
pub struct Ext4MetadataSummary {
    pub len: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
}

#[cfg(feature = "vfs_ext4")]
pub struct Ext4File {
    data: Vec<u8>,
    pos: usize,
}

#[cfg(feature = "vfs_ext4")]
impl crate::modules::vfs::types::File for Ext4File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let n = core::cmp::min(buf.len(), self.data.len() - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("ext4 is read-only")
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

#[cfg(feature = "vfs_ext4")]
impl Ext4Library {
    pub fn load_from_bytes(image: Vec<u8>) -> Result<Self, ext4_view::Ext4Error> {
        let fs = ext4_view::Ext4::load(Box::new(image))?;
        Ok(Self { fs })
    }

    pub fn exists(&self, path: &str) -> Result<bool, ext4_view::Ext4Error> {
        self.fs.exists(path)
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>, ext4_view::Ext4Error> {
        self.fs.read(path)
    }

    pub fn open(&self, path: &str, _tid: TaskId) -> Result<Box<dyn crate::modules::vfs::types::File>, &'static str> {
        let data = self.read(path).map_err(|_| "failed to read ext4 file")?;
        Ok(Box::new(Ext4File { data, pos: 0 }))
    }

    pub fn create(&self, _path: &str, _tid: TaskId) -> Result<Box<dyn crate::modules::vfs::types::File>, &'static str> {
        Err("ext4 is read-only")
    }

    pub fn remove(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("ext4 is read-only")
    }

    pub fn mkdir(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("ext4 is read-only")
    }

    pub fn rmdir(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("ext4 is read-only")
    }

    pub fn readdir(&self, path: &str, _tid: crate::interfaces::TaskId) -> Result<Vec<crate::modules::vfs::types::DirEntry>, &'static str> {
        let names = self.list_dir(path).map_err(|_| "failed to list ext4 dir")?;
        Ok(names.into_iter().map(|name| crate::modules::vfs::types::DirEntry {
            name,
            ino: 0,
            kind: 8, // DT_REG
        }).collect())
    }

    pub fn stat(&self, path: &str, _tid: TaskId) -> Result<crate::modules::vfs::types::FileStats, &'static str> {
        let md = self.metadata(path).map_err(|_| "failed to get ext4 metadata")?;
        Ok(crate::modules::vfs::types::FileStats {
            size: md.len,
            uid: md.uid,
            gid: md.gid,
            mode: md.mode as u32,
            ..Default::default()
        })
    }

    pub fn chmod(&self, _path: &str, _mode: u16, _tid: TaskId) -> Result<(), &'static str> {
        Err("ext4 is read-only")
    }

    pub fn chown(&self, _path: &str, _uid: u32, _gid: u32, _tid: TaskId) -> Result<(), &'static str> {
        Err("ext4 is read-only")
    }

    pub fn rename(&self, _old: &str, _new: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("ext4 is read-only")
    }

    pub fn link(&self, _old: &str, _new: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("ext4 is read-only")
    }

    pub fn symlink(&self, _target: &str, _link: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("ext4 is read-only")
    }

    pub fn readlink(&self, _path: &str, _tid: TaskId) -> Result<alloc::string::String, &'static str> {
        Err("readlink not supported on ext4 yet")
    }

    pub fn set_times(&self, _path: &str, _atime: u64, _mtime: u64, _tid: TaskId) -> Result<(), &'static str> {
        Err("ext4 is read-only")
    }

    pub fn read_to_string(&self, path: &str) -> Result<String, ext4_view::Ext4Error> {
        self.fs.read_to_string(path)
    }

    pub fn list_dir(&self, path: &str) -> Result<Vec<String>, ext4_view::Ext4Error> {
        let mut out = Vec::new();
        let iter = self.fs.read_dir(path)?;
        for entry in iter {
            let entry = entry?;
            out.push(entry.path().display().to_string());
        }
        Ok(out)
    }

    pub fn metadata(&self, path: &str) -> Result<Ext4MetadataSummary, ext4_view::Ext4Error> {
        let md = self.fs.metadata(path)?;
        Ok(Ext4MetadataSummary {
            len: md.len(),
            is_dir: md.is_dir(),
            is_symlink: md.is_symlink(),
            mode: md.mode(),
            uid: md.uid(),
            gid: md.gid(),
        })
    }
}
