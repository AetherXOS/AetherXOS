//! tmpfs — In-memory temporary filesystem for Linux compatibility.
//!
//! Provides a simple, fast, fully writable filesystem backed by kernel heap.
//! Used for /tmp, /run, /dev/shm mounting points.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::any::Any;
use spin::Mutex;

use crate::interfaces::TaskId;
use crate::modules::vfs::{
    types::{DirEntry, File, FileStats, PollEvents, SeekFrom},
    FileSystem,
};

// ── TmpFile ─────────────────────────────────────────────────────────────────

/// In-memory file node data.
struct TmpFileData {
    content: Vec<u8>,
    mode: u32,
    uid: u32,
    gid: u32,
    atime: u64,
    mtime: u64,
    ctime: u64,
}

impl TmpFileData {
    fn new(mode: u32) -> Self {
        Self {
            content: Vec::new(),
            mode,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
        }
    }
}

/// A handle to an open tmpfs file.
struct TmpFileHandle {
    data: Arc<Mutex<TmpFileData>>,
    pos: usize,
}

impl File for TmpFileHandle {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let data = self.data.lock();
        if self.pos >= data.content.len() {
            return Ok(0);
        }
        let remaining = &data.content[self.pos..];
        let n = buf.len().min(remaining.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        drop(data);
        self.pos += n;
        Ok(n)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        let mut data = self.data.lock();
        let end = self.pos + buf.len();
        if end > data.content.len() {
            data.content.resize(end, 0);
        }
        data.content[self.pos..end].copy_from_slice(buf);
        drop(data);
        self.pos = end;
        Ok(buf.len())
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, &'static str> {
        let data = self.data.lock();
        let size = data.content.len() as i64;
        drop(data);
        let new_pos = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(n) => self.pos as i64 + n,
            SeekFrom::End(n) => size + n,
        };
        if new_pos < 0 {
            return Err("EINVAL");
        }
        self.pos = new_pos as usize;
        Ok(self.pos as u64)
    }

    fn truncate(&mut self, size: u64) -> Result<(), &'static str> {
        let mut data = self.data.lock();
        data.content.resize(size as usize, 0);
        Ok(())
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        let data = self.data.lock();
        Ok(FileStats {
            size: data.content.len() as u64,
            mode: data.mode,
            uid: data.uid,
            gid: data.gid,
            atime: data.atime,
            mtime: data.mtime,
            ctime: data.ctime,
            blksize: 4096,
            blocks: (data.content.len() as u64 + 511) / 512,
        })
    }

    fn poll_events(&self) -> PollEvents {
        PollEvents::IN | PollEvents::OUT
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ── TmpFs Inodes ────────────────────────────────────────────────────────────

#[derive(Clone)]
enum TmpNode {
    File(Arc<Mutex<TmpFileData>>),
    Dir { mode: u32, uid: u32, gid: u32 },
    Symlink(String),
}

// ── TmpFs ───────────────────────────────────────────────────────────────────

pub struct TmpFs {
    nodes: Mutex<BTreeMap<String, TmpNode>>,
    max_size: usize, // maximum total bytes (0 = unlimited)
}

impl TmpFs {
    pub fn new() -> Self {
        let mut nodes = BTreeMap::new();
        // Root directory
        nodes.insert(
            String::new(),
            TmpNode::Dir {
                mode: 0o040777 | 0o01000, // sticky + dir + rwxrwxrwx
                uid: 0,
                gid: 0,
            },
        );
        Self {
            nodes: Mutex::new(nodes),
            max_size: 0,
        }
    }

    pub fn with_max_size(max_size: usize) -> Self {
        let mut fs = Self::new();
        fs.max_size = max_size;
        fs
    }

    fn normalize(path: &str) -> String {
        let trimmed = path.trim_matches('/');
        trimmed.to_string()
    }

    fn parent(path: &str) -> Option<String> {
        let normalized = Self::normalize(path);
        if normalized.is_empty() {
            return None;
        }
        match normalized.rfind('/') {
            Some(idx) => Some(normalized[..idx].to_string()),
            None => Some(String::new()), // parent is root
        }
    }

    fn basename(path: &str) -> String {
        let normalized = Self::normalize(path);
        match normalized.rfind('/') {
            Some(idx) => normalized[idx + 1..].to_string(),
            None => normalized,
        }
    }
}

impl FileSystem for TmpFs {
    fn open(&self, path: &str, _tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let key = Self::normalize(path);
        let nodes = self.nodes.lock();
        match nodes.get(&key) {
            Some(TmpNode::File(data)) => Ok(Box::new(TmpFileHandle {
                data: data.clone(),
                pos: 0,
            })),
            Some(TmpNode::Dir { .. }) => Err("EISDIR"),
            Some(TmpNode::Symlink(_)) => Err("ELOOP"), // TODO: follow symlinks
            None => Err("ENOENT"),
        }
    }

    fn create(&self, path: &str, _tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let key = Self::normalize(path);
        if key.is_empty() {
            return Err("EINVAL");
        }

        // Verify parent exists and is a directory
        if let Some(parent_key) = Self::parent(path) {
            let nodes = self.nodes.lock();
            match nodes.get(&parent_key) {
                Some(TmpNode::Dir { .. }) => {}
                _ => return Err("ENOENT"),
            }
            drop(nodes);
        }

        let data = Arc::new(Mutex::new(TmpFileData::new(0o100644)));
        let mut nodes = self.nodes.lock();
        nodes.insert(key, TmpNode::File(data.clone()));
        Ok(Box::new(TmpFileHandle { data, pos: 0 }))
    }

    fn remove(&self, path: &str, _tid: TaskId) -> Result<(), &'static str> {
        let key = Self::normalize(path);
        let mut nodes = self.nodes.lock();
        match nodes.get(&key) {
            Some(TmpNode::File(_)) | Some(TmpNode::Symlink(_)) => {
                nodes.remove(&key);
                Ok(())
            }
            Some(TmpNode::Dir { .. }) => Err("EISDIR"),
            None => Err("ENOENT"),
        }
    }

    fn mkdir(&self, path: &str, _tid: TaskId) -> Result<(), &'static str> {
        let key = Self::normalize(path);
        if key.is_empty() {
            return Err("EEXIST");
        }

        let mut nodes = self.nodes.lock();
        if nodes.contains_key(&key) {
            return Err("EEXIST");
        }
        nodes.insert(
            key,
            TmpNode::Dir {
                mode: 0o040755,
                uid: 0,
                gid: 0,
            },
        );
        Ok(())
    }

    fn rmdir(&self, path: &str, _tid: TaskId) -> Result<(), &'static str> {
        let key = Self::normalize(path);
        if key.is_empty() {
            return Err("EBUSY"); // cannot remove root
        }

        let prefix = alloc::format!("{}/", key);
        let mut nodes = self.nodes.lock();

        // Check if directory is empty
        let has_children = nodes.keys().any(|k| k.starts_with(&prefix));
        if has_children {
            return Err("ENOTEMPTY");
        }

        match nodes.get(&key) {
            Some(TmpNode::Dir { .. }) => {
                nodes.remove(&key);
                Ok(())
            }
            Some(_) => Err("ENOTDIR"),
            None => Err("ENOENT"),
        }
    }

    fn readdir(&self, path: &str, _tid: TaskId) -> Result<Vec<DirEntry>, &'static str> {
        let key = Self::normalize(path);
        let nodes = self.nodes.lock();

        // Verify directory exists
        if !key.is_empty() {
            match nodes.get(&key) {
                Some(TmpNode::Dir { .. }) => {}
                _ => return Err("ENOENT"),
            }
        }

        let prefix = if key.is_empty() {
            String::new()
        } else {
            alloc::format!("{}/", key)
        };

        let mut entries = Vec::new();
        let mut seen = alloc::collections::BTreeSet::new();
        let mut ino: u64 = 100;

        for (node_key, node) in nodes.iter() {
            if !node_key.starts_with(&prefix) {
                continue;
            }
            let remainder = &node_key[prefix.len()..];
            if remainder.is_empty() {
                continue;
            }
            // Only list direct children
            let name = match remainder.find('/') {
                Some(idx) => &remainder[..idx],
                None => remainder,
            };
            if name.is_empty() || !seen.insert(name.to_string()) {
                continue;
            }

            let kind = if remainder.contains('/') {
                4 // DT_DIR
            } else {
                match node {
                    TmpNode::Dir { .. } => 4,
                    TmpNode::File(_) => 8, // DT_REG
                    TmpNode::Symlink(_) => 10, // DT_LNK
                }
            };

            entries.push(DirEntry {
                name: name.to_string(),
                ino,
                kind,
            });
            ino += 1;
        }
        Ok(entries)
    }

    fn stat(&self, path: &str, _tid: TaskId) -> Result<FileStats, &'static str> {
        let key = Self::normalize(path);
        let nodes = self.nodes.lock();
        match nodes.get(&key) {
            Some(TmpNode::File(data)) => {
                let d = data.lock();
                Ok(FileStats {
                    size: d.content.len() as u64,
                    mode: d.mode,
                    uid: d.uid,
                    gid: d.gid,
                    atime: d.atime,
                    mtime: d.mtime,
                    ctime: d.ctime,
                    blksize: 4096,
                    blocks: (d.content.len() as u64 + 511) / 512,
                })
            }
            Some(TmpNode::Dir { mode, uid, gid }) => Ok(FileStats {
                size: 0,
                mode: *mode,
                uid: *uid,
                gid: *gid,
                atime: 0,
                mtime: 0,
                ctime: 0,
                blksize: 4096,
                blocks: 0,
            }),
            Some(TmpNode::Symlink(target)) => Ok(FileStats {
                size: target.len() as u64,
                mode: 0o120777,
                uid: 0,
                gid: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
                blksize: 4096,
                blocks: 0,
            }),
            None => {
                // Check if it's an implicit directory (has children with this prefix)
                if key.is_empty() {
                    return Ok(FileStats {
                        size: 0,
                        mode: 0o040777,
                        uid: 0,
                        gid: 0,
                        atime: 0,
                        mtime: 0,
                        ctime: 0,
                        blksize: 4096,
                        blocks: 0,
                    });
                }
                Err("ENOENT")
            }
        }
    }

    fn chmod(&self, path: &str, mode: u16, _tid: TaskId) -> Result<(), &'static str> {
        let key = Self::normalize(path);
        let mut nodes = self.nodes.lock();
        match nodes.get_mut(&key) {
            Some(TmpNode::File(data)) => {
                let mut d = data.lock();
                d.mode = (d.mode & 0o170000) | (mode as u32 & 0o7777);
                Ok(())
            }
            Some(TmpNode::Dir { mode: m, .. }) => {
                *m = (*m & 0o170000) | (mode as u32 & 0o7777);
                Ok(())
            }
            _ => Err("ENOENT"),
        }
    }

    fn chown(&self, path: &str, uid: u32, gid: u32, _tid: TaskId) -> Result<(), &'static str> {
        let key = Self::normalize(path);
        let mut nodes = self.nodes.lock();
        match nodes.get_mut(&key) {
            Some(TmpNode::File(data)) => {
                let mut d = data.lock();
                d.uid = uid;
                d.gid = gid;
                Ok(())
            }
            Some(TmpNode::Dir {
                uid: u, gid: g, ..
            }) => {
                *u = uid;
                *g = gid;
                Ok(())
            }
            _ => Err("ENOENT"),
        }
    }

    fn rename(&self, old_path: &str, new_path: &str, _tid: TaskId) -> Result<(), &'static str> {
        let old_key = Self::normalize(old_path);
        let new_key = Self::normalize(new_path);

        let mut nodes = self.nodes.lock();
        if let Some(node) = nodes.remove(&old_key) {
            nodes.insert(new_key, node);
            Ok(())
        } else {
            Err("ENOENT")
        }
    }

    fn symlink(&self, target: &str, link_path: &str, _tid: TaskId) -> Result<(), &'static str> {
        let key = Self::normalize(link_path);
        let mut nodes = self.nodes.lock();
        if nodes.contains_key(&key) {
            return Err("EEXIST");
        }
        nodes.insert(key, TmpNode::Symlink(target.to_string()));
        Ok(())
    }

    fn readlink(&self, path: &str, _tid: TaskId) -> Result<String, &'static str> {
        let key = Self::normalize(path);
        let nodes = self.nodes.lock();
        match nodes.get(&key) {
            Some(TmpNode::Symlink(target)) => Ok(target.clone()),
            _ => Err("EINVAL"),
        }
    }
}
