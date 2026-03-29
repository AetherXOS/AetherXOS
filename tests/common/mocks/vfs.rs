use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use alloc::string::String;
use alloc::vec::Vec;

pub struct MockVfs {
    pub mounted: AtomicBool,
    pub file_count: AtomicUsize,
    pub dir_count: AtomicUsize,
    pub read_count: AtomicUsize,
    pub write_count: AtomicUsize,
}

impl MockVfs {
    pub fn new() -> Self {
        Self {
            mounted: AtomicBool::new(false),
            file_count: AtomicUsize::new(0),
            dir_count: AtomicUsize::new(0),
            read_count: AtomicUsize::new(0),
            write_count: AtomicUsize::new(0),
        }
    }

    pub fn mount(&self) -> Result<(), &'static str> {
        if self.mounted.swap(true, Ordering::SeqCst) {
            return Err("Already mounted");
        }
        Ok(())
    }

    pub fn unmount(&self) -> Result<(), &'static str> {
        if !self.mounted.swap(false, Ordering::SeqCst) {
            return Err("Not mounted");
        }
        Ok(())
    }

    pub fn is_mounted(&self) -> bool {
        self.mounted.load(Ordering::SeqCst)
    }

    pub fn create_file(&self, _path: &str) -> Result<(), &'static str> {
        if !self.is_mounted() {
            return Err("VFS not mounted");
        }
        self.file_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn create_dir(&self, _path: &str) -> Result<(), &'static str> {
        if !self.is_mounted() {
            return Err("VFS not mounted");
        }
        self.dir_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn read(&self, _path: &str, _buffer: &mut [u8]) -> Result<usize, &'static str> {
        if !self.is_mounted() {
            return Err("VFS not mounted");
        }
        self.read_count.fetch_add(1, Ordering::SeqCst);
        Ok(0)
    }

    pub fn write(&self, _path: &str, _data: &[u8]) -> Result<usize, &'static str> {
        if !self.is_mounted() {
            return Err("VFS not mounted");
        }
        self.write_count.fetch_add(1, Ordering::SeqCst);
        Ok(0)
    }

    pub fn get_file_count(&self) -> usize {
        self.file_count.load(Ordering::SeqCst)
    }

    pub fn get_dir_count(&self) -> usize {
        self.dir_count.load(Ordering::SeqCst)
    }

    pub fn get_read_count(&self) -> usize {
        self.read_count.load(Ordering::SeqCst)
    }

    pub fn get_write_count(&self) -> usize {
        self.write_count.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.mounted.store(false, Ordering::SeqCst);
        self.file_count.store(0, Ordering::SeqCst);
        self.dir_count.store(0, Ordering::SeqCst);
        self.read_count.store(0, Ordering::SeqCst);
        self.write_count.store(0, Ordering::SeqCst);
    }
}

impl Default for MockVfs {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MockFileNode {
    pub name: String,
    pub size: usize,
    pub is_directory: bool,
    pub permissions: u32,
}

impl MockFileNode {
    pub fn new(name: &str, size: usize, is_directory: bool) -> Self {
        Self {
            name: String::from(name),
            size,
            is_directory,
            permissions: 0o644,
        }
    }

    pub fn with_permissions(mut self, perms: u32) -> Self {
        self.permissions = perms;
        self
    }
}
