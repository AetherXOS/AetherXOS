use super::PosixErrno;
use crate::modules::vfs::File;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

lazy_static::lazy_static! {
    /// Global registry: path -> list of inotify watch entries.
    static ref INOTIFY_WATCHES: Mutex<BTreeMap<String, Vec<WatchEntry>>> = Mutex::new(BTreeMap::new());
}

#[derive(Clone, Copy)]
struct WatchEntry {
    fd: u32,
    wd: i32,
    #[allow(dead_code)]
    mask: u32,
}

pub struct InotifyFile {
    pub wd_to_path: Mutex<BTreeMap<i32, String>>,
    pub next_wd: core::sync::atomic::AtomicI32,
    pub events: Mutex<Vec<u8>>,
    #[allow(dead_code)]
    pub wait: crate::kernel::sync::WaitQueue,
}

impl File for InotifyFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        let tid = crate::interfaces::task::TaskId(unsafe {
            crate::kernel::cpu_local::CpuLocal::get()
                .current_task
                .load(core::sync::atomic::Ordering::Relaxed)
        });
        loop {
            let mut events = self.events.lock();
            if !events.is_empty() {
                let len = core::cmp::min(buf.len(), events.len());
                buf[..len].copy_from_slice(&events[..len]);
                events.drain(..len);
                return Ok(len);
            }
            // Block if no events
            let _ = tid;
            drop(events);
            return Err("would block");
        }
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("inotify is read-only")
    }

    fn poll_events(&self) -> crate::modules::vfs::PollEvents {
        if !self.events.lock().is_empty() {
            crate::modules::vfs::PollEvents::IN
        } else {
            crate::modules::vfs::PollEvents::empty()
        }
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

pub fn inotify_init(_flags: i32) -> Result<u32, PosixErrno> {
    let file = InotifyFile {
        wd_to_path: Mutex::new(BTreeMap::new()),
        next_wd: core::sync::atomic::AtomicI32::new(1),
        events: Mutex::new(Vec::new()),
        wait: crate::kernel::sync::WaitQueue::new(),
    };

    let fd = crate::modules::posix::fs::register_handle(
        0,
        alloc::format!("inotify"),
        Arc::new(Mutex::new(file)),
        true,
    );
    Ok(fd)
}

pub fn inotify_add_watch(fd: u32, path: &str, mask: u32) -> Result<i32, PosixErrno> {
    let handle = {
        let table = crate::modules::posix::fs::FILE_TABLE.lock();
        let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
        desc.file.handle.clone()
    };

    let wd = {
        let mut file = handle.lock();
        let ino = file
            .as_any_mut()
            .downcast_mut::<InotifyFile>()
            .ok_or(PosixErrno::Invalid)?;
        let wd = ino
            .next_wd
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        ino.wd_to_path.lock().insert(wd, String::from(path));
        wd
    };

    let mut watches = INOTIFY_WATCHES.lock();
    watches
        .entry(String::from(path))
        .or_default()
        .push(WatchEntry { fd, wd, mask });

    Ok(wd)
}

pub fn inotify_rm_watch(fd: u32, wd: i32) -> Result<(), PosixErrno> {
    let path = {
        let handle = {
            let table = crate::modules::posix::fs::FILE_TABLE.lock();
            let desc = table.get(&fd).ok_or(PosixErrno::BadFileDescriptor)?;
            desc.file.handle.clone()
        };
        let mut file = handle.lock();
        let ino = file
            .as_any_mut()
            .downcast_mut::<InotifyFile>()
            .ok_or(PosixErrno::Invalid)?;
        let removed = ino.wd_to_path.lock().remove(&wd);
        removed
    };

    let mut watches = INOTIFY_WATCHES.lock();
    if let Some(path) = path {
        if let Some(entries) = watches.get_mut(&path) {
            entries.retain(|entry| !(entry.fd == fd && entry.wd == wd));
            if entries.is_empty() {
                watches.remove(&path);
            }
        }
    } else {
        for entries in watches.values_mut() {
            entries.retain(|entry| !(entry.fd == fd && entry.wd == wd));
        }
    }
    Ok(())
}

/// Dispatches an event to all interested inotify instances.
/// This would be called by VFS operations (unlink, write, etc).
#[allow(dead_code)]
pub fn post_event(path: &str, mask: u32) {
    let interested = {
        let watches = INOTIFY_WATCHES.lock();
        watches.get(path).cloned()
    };

    if let Some(entries) = interested {
        for entry in entries {
            if (entry.mask & mask) == 0 {
                continue;
            }

            let handle = {
                let table = crate::modules::posix::fs::FILE_TABLE.lock();
                match table.get(&entry.fd) {
                    Some(desc) => desc.file.handle.clone(),
                    None => continue,
                }
            };

            let mut file = handle.lock();
            let Some(ino) = file.as_any_mut().downcast_mut::<InotifyFile>() else {
                continue;
            };

            let mut ev = Vec::new();
            ev.extend_from_slice(&entry.wd.to_ne_bytes());
            ev.extend_from_slice(&mask.to_ne_bytes());
            ev.extend_from_slice(&0u32.to_ne_bytes()); // cookie
            ev.extend_from_slice(&0u32.to_ne_bytes()); // len

            let mut q = ino.events.lock();
            q.extend_from_slice(&ev);
            if let Some(t) = ino.wait.wake_one() {
                crate::kernel::task::wake_task(t);
            }
        }
    }
}
