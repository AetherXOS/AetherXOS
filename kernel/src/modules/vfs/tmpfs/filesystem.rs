//! TmpFs filesystem implementation.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::format;
use crate::modules::vfs::types::VfsTimespec;
use spin::Mutex;

use crate::interfaces::TaskId;
use crate::modules::vfs::{
    constants::*,
    path_utils,
    types::{DirEntry, File, FileStats, FileSystem},
};
use super::data::TmpFileData;
use super::node::TmpNode;
use super::handle::TmpFileHandle;

/// In-memory tmpfs filesystem.
pub struct TmpFs {
    pub nodes: Mutex<BTreeMap<String, TmpNode>>,
    pub max_size: usize, // maximum total bytes (0 = unlimited)
}

impl TmpFs {
    pub fn new() -> Self {
        let mut nodes = BTreeMap::new();
        // Root directory: rwxrwxrwx + sticky bit
        nodes.insert(
            String::new(),
            TmpNode::Dir {
                mode: MODE_DIR_STICKY,
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

    pub fn resolve_open_key(&self, path: &str) -> Result<String, &'static str> {
        let mut current = path_utils::normalize(path);
        let mut seen = BTreeSet::new();

        for _ in 0..SYMLINK_MAX_DEPTH {
            if !seen.insert(current.clone()) {
                return Err("ELOOP");
            }

            let next = {
                let nodes = self.nodes.lock();
                match nodes.get(&current) {
                    Some(TmpNode::Symlink(target)) => Some(target.clone()),
                    Some(_) => return Ok(current),
                    None => return Err("ENOENT"),
                }
            };

            let target = match next {
                Some(value) => value,
                None => return Err("ENOENT"),
            };
            let parent = path_utils::parent(&current).unwrap_or_default();
            current = path_utils::join_relative(&parent, &target);
        }

        Err("ELOOP")
    }
}

impl FileSystem for TmpFs {
    fn open(&self, path: &str, _tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let key = self.resolve_open_key(path)?;
        let nodes = self.nodes.lock();
        match nodes.get(&key) {
            Some(TmpNode::File(data)) => Ok(Box::new(TmpFileHandle {
                data: data.clone(),
                pos: 0,
            })),
            Some(TmpNode::Dir { .. }) => Err("EISDIR"),
            Some(TmpNode::Symlink(_)) => Err("ELOOP"),
            None => Err("ENOENT"),
        }
    }

    fn create(&self, path: &str, _tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let key = path_utils::normalize(path);
        if key.is_empty() {
            return Err("EINVAL");
        }

        // Verify parent exists and is a directory
        if let Some(parent_key) = path_utils::parent(path) {
            let nodes = self.nodes.lock();
            match nodes.get(&parent_key) {
                Some(TmpNode::Dir { .. }) => {}
                _ => return Err("ENOENT"),
            }
            drop(nodes);
        }

        let data = Arc::new(Mutex::new(TmpFileData::new(MODE_FILE_DEFAULT)));
        let mut nodes = self.nodes.lock();
        nodes.insert(key, TmpNode::File(data.clone()));
        Ok(Box::new(TmpFileHandle { data, pos: 0 }))
    }

    fn remove(&self, path: &str, _tid: TaskId) -> Result<(), &'static str> {
        let key = path_utils::normalize(path);
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
        let key = path_utils::normalize(path);
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
                mode: MODE_DIR_DEFAULT,
                uid: 0,
                gid: 0,
            },
        );
        Ok(())
    }

    fn rmdir(&self, path: &str, _tid: TaskId) -> Result<(), &'static str> {
        let key = path_utils::normalize(path);
        if key.is_empty() {
            return Err("EBUSY"); // cannot remove root
        }

        let prefix = format!("{}/", key);
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
        let key = path_utils::normalize(path);
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
            format!("{}/", key)
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
                DT_DIR
            } else {
                match node {
                    TmpNode::Dir { .. } => DT_DIR,
                    TmpNode::File(_) => DT_REG,
                    TmpNode::Symlink(_) => DT_LNK,
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
        let key = path_utils::normalize(path);
        let nodes = self.nodes.lock();
        match nodes.get(&key) {
            Some(TmpNode::File(data)) => {
                    let d = data.lock();
                    use crate::modules::vfs::types::VfsTimespec;
                    Ok(FileStats {
                        size: d.content.len() as u64,
                        mode: d.mode,
                        uid: d.uid,
                        gid: d.gid,
                        atime: VfsTimespec { sec: d.atime, nsec: 0 },
                        mtime: VfsTimespec { sec: d.mtime, nsec: 0 },
                        ctime: VfsTimespec { sec: d.ctime, nsec: 0 },
                        blksize: BLOCK_SIZE as u32,
                        blocks: (d.content.len() as u64 + BLOCK_SHIFT as u64 - 1) / BLOCK_SHIFT as u64,
                        ..crate::modules::vfs::types::FileStats::default()
                    })
            }
            Some(TmpNode::Dir { mode, uid, gid }) => Ok(FileStats {
                size: 0,
                mode: *mode,
                uid: *uid,
                gid: *gid,
                atime: VfsTimespec { sec: 0, nsec: 0 },
                mtime: VfsTimespec { sec: 0, nsec: 0 },
                ctime: VfsTimespec { sec: 0, nsec: 0 },
                blksize: BLOCK_SIZE as u32,
                blocks: 0,
                ..crate::modules::vfs::types::FileStats::default()
            }),
            Some(TmpNode::Symlink(target)) => Ok(FileStats {
                size: target.len() as u64,
                mode: MODE_LNK | MODE_RWXRWXRWX,
                uid: 0,
                gid: 0,
                atime: VfsTimespec { sec: 0, nsec: 0 },
                mtime: VfsTimespec { sec: 0, nsec: 0 },
                ctime: VfsTimespec { sec: 0, nsec: 0 },
                blksize: BLOCK_SIZE as u32,
                blocks: 0,
                ..crate::modules::vfs::types::FileStats::default()
            }),
            None => {
                // Check if it's an implicit directory (has children with this prefix)
                if key.is_empty() {
                    return Ok(FileStats {
                        size: 0,
                        mode: MODE_DIR | MODE_RWXRWXRWX,
                        uid: 0,
                        gid: 0,
                        atime: Default::default(),
                        mtime: Default::default(),
                        ctime: Default::default(),
                        blksize: BLOCK_SIZE as u32,
                        blocks: 0,
                        ..crate::modules::vfs::types::FileStats::default()
                    });
                }
                Err("ENOENT")
            }
        }
    }

    fn chmod(&self, path: &str, mode: u16, _tid: TaskId) -> Result<(), &'static str> {
        let key = path_utils::normalize(path);
        let mut nodes = self.nodes.lock();
        match nodes.get_mut(&key) {
            Some(TmpNode::File(data)) => {
                let mut d = data.lock();
                d.mode = (d.mode & MODE_TYPE_MASK) | (mode as u32 & MODE_PERMS_MASK as u32);
                Ok(())
            }
            Some(TmpNode::Dir { mode: m, .. }) => {
                *m = (*m & MODE_TYPE_MASK) | (mode as u32 & MODE_PERMS_MASK as u32);
                Ok(())
            }
            _ => Err("ENOENT"),
        }
    }

    fn chown(&self, path: &str, uid: u32, gid: u32, _tid: TaskId) -> Result<(), &'static str> {
        let key = path_utils::normalize(path);
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
        let old_key = path_utils::normalize(old_path);
        let new_key = path_utils::normalize(new_path);

        let mut nodes = self.nodes.lock();
        if let Some(node) = nodes.remove(&old_key) {
            nodes.insert(new_key, node);
            Ok(())
        } else {
            Err("ENOENT")
        }
    }

    fn symlink(&self, target: &str, link_path: &str, _tid: TaskId) -> Result<(), &'static str> {
        let key = path_utils::normalize(link_path);
        let mut nodes = self.nodes.lock();
        if nodes.contains_key(&key) {
            return Err("EEXIST");
        }
        nodes.insert(key, TmpNode::Symlink(target.to_string()));
        Ok(())
    }

    fn readlink(&self, path: &str, _tid: TaskId) -> Result<String, &'static str> {
        let key = path_utils::normalize(path);
        let nodes = self.nodes.lock();
        match nodes.get(&key) {
            Some(TmpNode::Symlink(target)) => Ok(target.clone()),
            _ => Err("EINVAL"),
        }
    }
}
