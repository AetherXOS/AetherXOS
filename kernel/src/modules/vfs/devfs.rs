use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;

use crate::interfaces::TaskId;
use crate::modules::vfs::{
    types::{DirEntry, FileStats},
    File, FileSystem,
};

pub type DeviceCreator = Box<dyn Fn(TaskId) -> Box<dyn File> + Send + Sync>;
pub const DEVFS_EVENT_CAPACITY: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevFsEventKind {
    NodeAdded,
    NodeUpdated,
    NodeRemoved,
    NodeChmod,
    NodeChown,
}

#[derive(Debug, Clone)]
pub struct DevFsEvent {
    pub seq: u64,
    pub kind: DevFsEventKind,
    pub path: String,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct DevFsEventSnapshot {
    pub queued: usize,
    pub dropped: u64,
    pub next_seq: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DeviceMetadata {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub removable: bool,
}

impl DeviceMetadata {
    pub const fn char_device(mode: u32, uid: u32, gid: u32, removable: bool) -> Self {
        Self {
            mode: 0o020000 | mode,
            uid,
            gid,
            removable,
        }
    }
}

impl Default for DeviceMetadata {
    fn default() -> Self {
        Self::char_device(0o660, 0, 0, false)
    }
}

struct DeviceNode {
    creator: DeviceCreator,
    meta: DeviceMetadata,
}

struct DevFsEventRing {
    events: VecDeque<DevFsEvent>,
    next_seq: u64,
    dropped: u64,
}

impl DevFsEventRing {
    fn new() -> Self {
        Self {
            events: VecDeque::with_capacity(DEVFS_EVENT_CAPACITY),
            next_seq: 1,
            dropped: 0,
        }
    }

    fn push(&mut self, kind: DevFsEventKind, path: &str, meta: DeviceMetadata) {
        if self.events.len() == DEVFS_EVENT_CAPACITY {
            let _ = self.events.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.events.push_back(DevFsEvent {
            seq: self.next_seq,
            kind,
            path: path.to_string(),
            mode: meta.mode,
            uid: meta.uid,
            gid: meta.gid,
        });
        self.next_seq = self.next_seq.saturating_add(1);
    }
}

pub struct DevFs {
    devices: Mutex<BTreeMap<String, DeviceNode>>,
    events: Mutex<DevFsEventRing>,
}

impl DevFs {
    pub fn new() -> Self {
        Self {
            devices: Mutex::new(BTreeMap::new()),
            events: Mutex::new(DevFsEventRing::new()),
        }
    }

    fn normalize_device_path(path: &str) -> Option<String> {
        let normalized = path.trim_matches('/');
        if normalized.is_empty() {
            return None;
        }
        if normalized
            .split('/')
            .any(|seg| seg.is_empty() || seg == "." || seg == "..")
        {
            return None;
        }
        Some(normalized.to_string())
    }

    pub fn register_device(&self, name: &str, creator: DeviceCreator) -> bool {
        self.register_device_with_meta(name, creator, DeviceMetadata::default())
    }

    pub fn register_device_with_meta(
        &self,
        name: &str,
        creator: DeviceCreator,
        meta: DeviceMetadata,
    ) -> bool {
        let Some(key) = Self::normalize_device_path(name) else {
            return false;
        };
        let replaced = self
            .devices
            .lock()
            .insert(key.clone(), DeviceNode { creator, meta })
            .is_some();
        self.events.lock().push(
            if replaced {
                DevFsEventKind::NodeUpdated
            } else {
                DevFsEventKind::NodeAdded
            },
            &key,
            meta,
        );
        true
    }

    pub fn has_device(&self, name: &str) -> bool {
        let Some(key) = Self::normalize_device_path(name) else {
            return false;
        };
        self.devices.lock().contains_key(&key)
    }

    pub fn unregister_device(&self, name: &str) -> bool {
        let Some(key) = Self::normalize_device_path(name) else {
            return false;
        };
        let mut devices = self.devices.lock();
        if let Some(node) = devices.get(&key) {
            if !node.meta.removable {
                return false;
            }
        }
        let removed = devices.remove(&key).is_some();
        if removed {
            self.events
                .lock()
                .push(DevFsEventKind::NodeRemoved, &key, DeviceMetadata::default());
        }
        removed
    }

    pub fn chmod_device(&self, name: &str, mode: u16) -> bool {
        let Some(key) = Self::normalize_device_path(name) else {
            return false;
        };
        let mut devices = self.devices.lock();
        let Some(node) = devices.get_mut(&key) else {
            return false;
        };
        node.meta.mode = 0o020000 | (mode as u32 & 0o7777);
        let meta = node.meta;
        drop(devices);
        self.events
            .lock()
            .push(DevFsEventKind::NodeChmod, &key, meta);
        true
    }

    pub fn chown_device(&self, name: &str, uid: u32, gid: u32) -> bool {
        let Some(key) = Self::normalize_device_path(name) else {
            return false;
        };
        let mut devices = self.devices.lock();
        let Some(node) = devices.get_mut(&key) else {
            return false;
        };
        node.meta.uid = uid;
        node.meta.gid = gid;
        let meta = node.meta;
        drop(devices);
        self.events
            .lock()
            .push(DevFsEventKind::NodeChown, &key, meta);
        true
    }

    pub fn events_snapshot(&self) -> DevFsEventSnapshot {
        let events = self.events.lock();
        DevFsEventSnapshot {
            queued: events.events.len(),
            dropped: events.dropped,
            next_seq: events.next_seq,
        }
    }

    pub fn events_since(&self, after_seq: u64, max_items: usize) -> Vec<DevFsEvent> {
        if max_items == 0 {
            return vec![];
        }
        let events = self.events.lock();
        let mut out = vec![];
        for event in events.events.iter() {
            if event.seq > after_seq {
                out.push(event.clone());
                if out.len() >= max_items {
                    break;
                }
            }
        }
        out
    }

    fn path_is_dir(devices: &BTreeMap<String, DeviceNode>, path: &str) -> bool {
        if path.is_empty() {
            return true;
        }
        let prefix = alloc::format!("{path}/");
        devices.keys().any(|k| k.starts_with(&prefix))
    }

    fn readdir_entries(devices: &BTreeMap<String, DeviceNode>, path: &str) -> Vec<DirEntry> {
        let mut out = vec![];
        let mut seen = alloc::collections::BTreeSet::new();
        let prefix = if path.is_empty() {
            String::new()
        } else {
            alloc::format!("{path}/")
        };

        for key in devices.keys() {
            if !key.starts_with(&prefix) {
                continue;
            }
            let rem = &key[prefix.len()..];
            if rem.is_empty() {
                continue;
            }
            let name = rem.split('/').next().unwrap_or_default();
            if name.is_empty() || !seen.insert(name.to_string()) {
                continue;
            }
            let is_dir = rem.contains('/');
            out.push(DirEntry {
                name: name.to_string(),
                ino: 1000 + out.len() as u64,
                kind: if is_dir { 4 } else { 2 }, // DT_DIR / DT_CHR
            });
        }
        out
    }
}

impl FileSystem for DevFs {
    fn open(&self, path: &str, tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let name = Self::normalize_device_path(path).ok_or("invalid path")?;
        let devices = self.devices.lock();
        if let Some(node) = devices.get(&name) {
            Ok((node.creator)(tid))
        } else {
            Err("device not found")
        }
    }

    fn create(&self, _path: &str, _tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        Err("cannot create in devfs")
    }

    fn remove(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        if self.unregister_device(_path) {
            Ok(())
        } else {
            Err("cannot remove from devfs")
        }
    }

    fn mkdir(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("not supported")
    }

    fn rmdir(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("not supported")
    }

    fn readdir(&self, _path: &str, _tid: TaskId) -> Result<Vec<DirEntry>, &'static str> {
        let devices = self.devices.lock();
        let normalized = if _path == "/" || _path.is_empty() {
            String::new()
        } else {
            Self::normalize_device_path(_path).ok_or("invalid path")?
        };
        if !Self::path_is_dir(&devices, &normalized) {
            return Err("not found");
        }
        Ok(Self::readdir_entries(&devices, &normalized))
    }

    fn stat(&self, path: &str, _tid: TaskId) -> Result<FileStats, &'static str> {
        let name = if path == "/" || path.is_empty() {
            String::new()
        } else {
            Self::normalize_device_path(path).ok_or("invalid path")?
        };
        if name.is_empty() {
            return Ok(FileStats {
                size: 0,
                mode: 0o755 | 0o040000, // dir
                uid: 0,
                gid: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
                blksize: 4096,
                blocks: 0,
            });
        }
        let devices = self.devices.lock();
        if let Some(node) = devices.get(&name) {
            Ok(FileStats {
                size: 0,
                mode: node.meta.mode,
                uid: node.meta.uid,
                gid: node.meta.gid,
                atime: 0,
                mtime: 0,
                ctime: 0,
                blksize: 4096,
                blocks: 0,
            })
        } else if Self::path_is_dir(&devices, &name) {
            Ok(FileStats {
                size: 0,
                mode: 0o755 | 0o040000,
                uid: 0,
                gid: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
                blksize: 4096,
                blocks: 0,
            })
        } else {
            Err("not found")
        }
    }

    fn chmod(&self, path: &str, mode: u16, _tid: TaskId) -> Result<(), &'static str> {
        if self.chmod_device(path, mode) {
            Ok(())
        } else {
            Err("not found")
        }
    }

    fn chown(&self, path: &str, uid: u32, gid: u32, _tid: TaskId) -> Result<(), &'static str> {
        if self.chown_device(path, uid, gid) {
            Ok(())
        } else {
            Err("not found")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyFile;

    impl File for DummyFile {
        fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
            Ok(0)
        }
        fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
            Ok(buf.len())
        }
        fn as_any(&self) -> &dyn core::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
            self
        }
    }

    #[test_case]
    fn hierarchical_readdir_exposes_directories() {
        let devfs = DevFs::new();
        assert!(devfs.register_device("tty0", Box::new(|_| Box::new(DummyFile))));
        assert!(devfs.register_device("net/virtio0", Box::new(|_| Box::new(DummyFile))));

        let root_entries = devfs.readdir("/", TaskId(0)).unwrap();
        assert!(root_entries.iter().any(|e| e.name == "tty0"));
        assert!(root_entries.iter().any(|e| e.name == "net"));

        let net_entries = devfs.readdir("/net", TaskId(0)).unwrap();
        assert!(net_entries.iter().any(|e| e.name == "virtio0"));
    }

    #[test_case]
    fn removable_device_can_be_unregistered() {
        let devfs = DevFs::new();
        assert!(devfs.register_device_with_meta(
            "hotplug/net0",
            Box::new(|_| Box::new(DummyFile)),
            DeviceMetadata::char_device(0o660, 0, 0, true),
        ));
        assert!(devfs.unregister_device("hotplug/net0"));
        assert!(devfs.open("/hotplug/net0", TaskId(0)).is_err());
    }

    #[test_case]
    fn event_stream_records_lifecycle() {
        let devfs = DevFs::new();
        assert!(devfs.register_device_with_meta(
            "net/evt0",
            Box::new(|_| Box::new(DummyFile)),
            DeviceMetadata::char_device(0o660, 0, 0, true),
        ));
        assert!(devfs.chmod_device("net/evt0", 0o640));
        assert!(devfs.chown_device("net/evt0", 1000, 1000));
        assert!(devfs.unregister_device("net/evt0"));

        let events = devfs.events_since(0, 16);
        assert!(events.len() >= 4);
        assert_eq!(events[0].kind, DevFsEventKind::NodeAdded);
        assert_eq!(events[1].kind, DevFsEventKind::NodeChmod);
        assert_eq!(events[2].kind, DevFsEventKind::NodeChown);
        assert_eq!(events[3].kind, DevFsEventKind::NodeRemoved);
    }
}
