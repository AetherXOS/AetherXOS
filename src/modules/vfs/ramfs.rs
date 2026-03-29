use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

use crate::interfaces::TaskId;
use crate::modules::vfs::path::normalize_path;
use crate::modules::vfs::ramfs_support::{
    has_owner_access, is_child_of, make_meta, parent_dir, RamMeta, ROOT_TASK_ID,
};
#[cfg(feature = "vfs_telemetry")]
use crate::modules::vfs::telemetry;
use crate::modules::vfs::{File, FileSystem, SeekFrom};
#[path = "ramfs/methods.rs"]
mod methods;

const DEFAULT_DIR_MODE: u16 = 0o755;
const DEFAULT_FILE_MODE: u16 = 0o644;
const DEFAULT_SYMLINK_MODE: u16 = 0o777;

pub struct RamFile {
    pub content: Vec<u8>,
    pub cursor: usize,
}

impl File for RamFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if self.cursor > self.content.len() {
            return Err("cursor out of bounds");
        }
        let remaining = self.content.len() - self.cursor;
        let read_len = core::cmp::min(remaining, buf.len());
        if read_len == 0 {
            return Ok(0);
        }

        buf[..read_len].copy_from_slice(&self.content[self.cursor..self.cursor + read_len]);
        self.cursor += read_len;
        Ok(read_len)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        let end = self.cursor.checked_add(buf.len()).ok_or("write overflow")?;
        if end > self.content.len() {
            self.content.resize(end, 0);
        }
        self.content[self.cursor..end].copy_from_slice(buf);
        self.cursor += buf.len();
        Ok(buf.len())
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, &'static str> {
        let next = match pos {
            SeekFrom::Start(offset) => usize::try_from(offset).map_err(|_| "seek overflow")?,
            SeekFrom::End(delta) => {
                let base = self.content.len() as i128;
                let target = base + delta as i128;
                if target < 0 {
                    return Err("seek before start");
                }
                usize::try_from(target as u128).map_err(|_| "seek overflow")?
            }
            SeekFrom::Current(delta) => {
                let base = self.cursor as i128;
                let target = base + delta as i128;
                if target < 0 {
                    return Err("seek before start");
                }
                usize::try_from(target as u128).map_err(|_| "seek overflow")?
            }
        };

        self.cursor = next;
        Ok(self.cursor as u64)
    }

    fn flush(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn stat(&self) -> Result<crate::modules::vfs::types::FileStats, &'static str> {
        Ok(crate::modules::vfs::types::FileStats {
            size: self.content.len() as u64,
            mode: 0o644,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }

    fn truncate(&mut self, size: u64) -> Result<(), &'static str> {
        self.content.resize(size as usize, 0);
        Ok(())
    }

    fn mmap(
        &self,
        offset: u64,
        len: usize,
    ) -> Result<Arc<Mutex<alloc::vec::Vec<u8>>>, &'static str> {
        let mut data = alloc::vec![0u8; len];
        let off = offset as usize;
        if off < self.content.len() {
            let n = core::cmp::min(len, self.content.len() - off);
            data[..n].copy_from_slice(&self.content[off..off + n]);
        }
        Ok(Arc::new(Mutex::new(data)))
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

pub struct RamFs {
    files: Arc<Mutex<BTreeMap<Vec<u8>, Arc<Mutex<Vec<u8>>>>>>,
    dirs: Arc<Mutex<BTreeSet<Vec<u8>>>>,
    symlinks: Arc<Mutex<BTreeMap<Vec<u8>, Vec<u8>>>>,
    meta: Arc<Mutex<BTreeMap<Vec<u8>, RamMeta>>>,
}

static NEXT_INODE: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(100);

impl RamFs {
    #[inline(always)]
    fn has_owner_access(tid: TaskId, owner_uid: u32) -> bool {
        has_owner_access(tid, owner_uid)
    }

    #[inline(always)]
    fn make_meta(mode: u16, owner: TaskId) -> RamMeta {
        Self::next_meta(mode, owner)
    }

    #[inline(always)]
    fn parent_dir(path: &[u8]) -> Option<Vec<u8>> {
        parent_dir(path)
    }

    #[inline(always)]
    fn is_child_of(path: &[u8], parent: &[u8]) -> bool {
        is_child_of(path, parent)
    }

    #[inline(always)]
    fn next_meta(mode: u16, owner: TaskId) -> RamMeta {
        let ino = NEXT_INODE.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let now = crate::kernel::watchdog::global_tick() as i64;
        make_meta(ino, mode, owner, now)
    }

    pub fn new() -> Self {
        let mut dirs = BTreeSet::new();
        dirs.insert(b"/".to_vec());
        let mut meta = BTreeMap::new();
        meta.insert(
            b"/".to_vec(),
            Self::next_meta(DEFAULT_DIR_MODE, ROOT_TASK_ID),
        );
        Self {
            files: Arc::new(Mutex::new(BTreeMap::new())),
            dirs: Arc::new(Mutex::new(dirs)),
            symlinks: Arc::new(Mutex::new(BTreeMap::new())),
            meta: Arc::new(Mutex::new(meta)),
        }
    }

    pub fn used_pages(&self) -> usize {
        let mut total_bytes = 0;
        let files = self.files.lock();
        for content in files.values() {
            total_bytes += content.lock().len();
        }
        let dir_count = self.dirs.lock().len();
        let sym_count = self.symlinks.lock().len();
        (total_bytes + 4095) / 4096 + dir_count + sym_count
    }

}

struct RamFsHandle {
    path: Vec<u8>,
    cursor: usize,
    files: Arc<Mutex<BTreeMap<Vec<u8>, Arc<Mutex<Vec<u8>>>>>>,
    meta: Arc<Mutex<BTreeMap<Vec<u8>, RamMeta>>>,
}

impl File for RamFsHandle {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let files = self.files.lock();
        let Some(content_arc) = files.get(&self.path).cloned() else {
            return Err("file not found");
        };
        drop(files);

        let content = content_arc.lock();
        if self.cursor >= content.len() {
            return Ok(0);
        }
        let remaining = content.len() - self.cursor;
        let read_len = core::cmp::min(remaining, buf.len());
        buf[..read_len].copy_from_slice(&content[self.cursor..self.cursor + read_len]);
        self.cursor += read_len;
        Ok(read_len)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        let files = self.files.lock();
        let Some(content_arc) = files.get(&self.path).cloned() else {
            return Err("file not found");
        };
        drop(files);

        let mut content = content_arc.lock();
        let end = self.cursor + buf.len();
        if end > content.len() {
            content.resize(end, 0);
        }
        content[self.cursor..end].copy_from_slice(buf);
        self.cursor = end;
        Ok(buf.len())
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, &'static str> {
        let files = self.files.lock();
        let len = files.get(&self.path).map(|a| a.lock().len()).unwrap_or(0);
        drop(files);

        let next = match pos {
            SeekFrom::Start(off) => off as usize,
            SeekFrom::End(delta) => (len as i64 + delta) as usize,
            SeekFrom::Current(delta) => (self.cursor as i64 + delta) as usize,
        };
        self.cursor = next;
        Ok(self.cursor as u64)
    }

    fn flush(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn stat(&self) -> Result<crate::modules::vfs::types::FileStats, &'static str> {
        let meta = self.meta.lock();
        let attrs = meta.get(&self.path).copied().ok_or("file not found")?;
        drop(meta);

        let files = self.files.lock();
        let len = files
            .get(&self.path)
            .map(|a| a.lock().len() as u64)
            .unwrap_or(0);
        drop(files);

        Ok(crate::modules::vfs::types::FileStats {
            size: len,
            mode: attrs.mode as u32,
            uid: attrs.uid,
            gid: attrs.gid,
            atime: attrs.atime_sec as u64,
            mtime: attrs.mtime_sec as u64,
            ctime: attrs.ctime_sec as u64,
            blksize: 4096,
            blocks: (len + 511) / 512,
        })
    }

    fn truncate(&mut self, size: u64) -> Result<(), &'static str> {
        let files = self.files.lock();
        let Some(content_arc) = files.get(&self.path).cloned() else {
            return Err("file not found");
        };
        drop(files);
        let mut content = content_arc.lock();
        content.resize(size as usize, 0);
        Ok(())
    }

    fn mmap(
        &self,
        offset: u64,
        len: usize,
    ) -> Result<Arc<Mutex<alloc::vec::Vec<u8>>>, &'static str> {
        let files = self.files.lock();
        let content_arc = files.get(&self.path).cloned().ok_or("file not found")?;
        drop(files);

        let content = content_arc.lock();
        let mut data = alloc::vec![0u8; len];
        let off = offset as usize;
        if off < content.len() {
            let n = core::cmp::min(len, content.len() - off);
            data[..n].copy_from_slice(&content[off..off + n]);
        }
        Ok(Arc::new(Mutex::new(data)))
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

impl FileSystem for RamFs {
    fn open(&self, path: &str, _tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        if self.dirs.lock().contains(&key) {
            return Err("is a directory");
        }
        if !self.files.lock().contains_key(&key) {
            return Err("file not found");
        }
        Ok(Box::new(RamFsHandle {
            path: key,
            cursor: 0,
            files: self.files.clone(),
            meta: self.meta.clone(),
        }))
    }

    fn create(&self, path: &str, tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;

        // Use internal create-like logic
        let parent = Self::parent_dir(&key).ok_or("invalid path")?;
        {
            let dirs = self.dirs.lock();
            if !dirs.contains(&parent) {
                return Err("parent not found");
            }
        }

        self.files
            .lock()
            .insert(key.clone(), Arc::new(Mutex::new(Vec::new())));
        self.meta
            .lock()
            .insert(key.clone(), Self::make_meta(DEFAULT_FILE_MODE, tid));

        Ok(Box::new(RamFsHandle {
            path: key,
            cursor: 0,
            files: self.files.clone(),
            meta: self.meta.clone(),
        }))
    }

    fn remove(&self, path: &str, tid: TaskId) -> Result<(), &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        let mut meta = self.meta.lock();
        let entry = meta.get(&key).ok_or("not found")?;
        if !Self::has_owner_access(tid, entry.uid) {
            return Err("permission denied");
        }

        let mut files = self.files.lock();
        if files.remove(&key).is_some() {
            meta.remove(&key);
            Ok(())
        } else {
            Err("file not found")
        }
    }

    fn mkdir(&self, path: &str, tid: TaskId) -> Result<(), &'static str> {
        self.mkdir(path, tid)
    }

    fn rmdir(&self, path: &str, tid: TaskId) -> Result<(), &'static str> {
        self.rmdir(path, tid)
    }

    fn readdir(
        &self,
        path: &str,
        _tid: TaskId,
    ) -> Result<Vec<crate::modules::vfs::types::DirEntry>, &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        if !self.dirs.lock().contains(&key) {
            return Err("not a directory");
        }

        let prefix = if key == b"/" {
            b"/".to_vec()
        } else {
            [key.as_slice(), b"/"].concat()
        };
        let mut out = Vec::new();
        let meta = self.meta.lock();

        let dirs = self.dirs.lock();
        for d in dirs.iter() {
            if d.starts_with(&prefix) {
                let suffix = &d[prefix.len()..];
                if !suffix.is_empty() && !suffix.contains(&b'/') {
                    let m = meta
                        .get(d)
                        .cloned()
                        .unwrap_or(Self::make_meta(0, ROOT_TASK_ID));
                    out.push(crate::modules::vfs::types::DirEntry {
                        name: String::from_utf8_lossy(suffix).into_owned(),
                        ino: m.ino,
                        kind: 1, // DT_DIR
                    });
                }
            }
        }

        let files = self.files.lock();
        for f in files.keys() {
            if f.starts_with(&prefix) {
                let suffix = &f[prefix.len()..];
                if !suffix.is_empty() && !suffix.contains(&b'/') {
                    let m = meta
                        .get(f)
                        .cloned()
                        .unwrap_or(Self::make_meta(0, ROOT_TASK_ID));
                    out.push(crate::modules::vfs::types::DirEntry {
                        name: String::from_utf8_lossy(suffix).into_owned(),
                        ino: m.ino,
                        kind: 0, // DT_REG
                    });
                }
            }
        }

        Ok(out)
    }

    fn stat(
        &self,
        path: &str,
        _tid: TaskId,
    ) -> Result<crate::modules::vfs::types::FileStats, &'static str> {
        let key = normalize_path(path).ok_or("invalid path")?;
        let meta = self.meta.lock();
        let attrs = meta.get(&key).ok_or("not found")?;

        let size = if self.dirs.lock().contains(&key) {
            4096 // Directory size convention
        } else {
            self.files
                .lock()
                .get(&key)
                .map(|a| a.lock().len() as u64)
                .unwrap_or(0)
        };

        Ok(crate::modules::vfs::types::FileStats {
            size,
            mode: attrs.mode as u32,
            uid: attrs.uid,
            gid: attrs.gid,
            atime: attrs.atime_sec as u64,
            mtime: attrs.mtime_sec as u64,
            ctime: attrs.ctime_sec as u64,
            blksize: 4096,
            blocks: (size + 511) / 512,
        })
    }

    fn chmod(&self, path: &str, mode: u16, tid: TaskId) -> Result<(), &'static str> {
        self.chmod(path, mode, tid)
    }

    fn chown(&self, path: &str, uid: u32, gid: u32, tid: TaskId) -> Result<(), &'static str> {
        self.chown(path, uid, gid, tid)
    }

    fn rename(&self, old: &str, new: &str, tid: TaskId) -> Result<(), &'static str> {
        self.rename(old, new, tid)
    }

    fn link(&self, old: &str, new: &str, tid: TaskId) -> Result<(), &'static str> {
        self.link(old, new, tid)
    }

    fn symlink(&self, target: &str, link: &str, tid: TaskId) -> Result<(), &'static str> {
        self.symlink(target, link, tid)
    }

    fn readlink(&self, path: &str, tid: TaskId) -> Result<alloc::string::String, &'static str> {
        self.readlink(path, tid)
    }

    fn set_times(
        &self,
        path: &str,
        atime: u64,
        mtime: u64,
        tid: TaskId,
    ) -> Result<(), &'static str> {
        self.set_times(path, atime as i64, 0, mtime as i64, 0, tid)
    }
}
