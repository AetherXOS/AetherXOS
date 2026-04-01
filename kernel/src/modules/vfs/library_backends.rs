extern crate alloc;

use alloc::vec::Vec;

#[cfg(feature = "vfs_ext4")]
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
pub struct FatFsLibrary {
    options: fatfs::FsOptions<fatfs::LossyOemCpConverter>,
}

#[cfg(all(feature = "vfs_fatfs", not(target_os = "none")))]
impl FatFsLibrary {
    pub fn new() -> Self {
        Self {
            options: fatfs::FsOptions::new(),
        }
    }

    pub fn options(&self) -> &fatfs::FsOptions<fatfs::LossyOemCpConverter> {
        &self.options
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
