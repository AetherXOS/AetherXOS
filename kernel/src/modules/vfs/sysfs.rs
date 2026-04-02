//! sysfs — Linux /sys virtual filesystem implementation.
//!
//! Provides /sys/class, /sys/devices, /sys/kernel, /sys/fs,
//! /sys/bus, /sys/power and related entries.

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::any::Any;

use crate::interfaces::TaskId;
use crate::modules::vfs::{
    types::{DirEntry, File, FileStats, PollEvents},
    FileSystem,
};

/// Simple read-only in-memory file for sysfs entries.
struct SysFsEntry {
    data: Vec<u8>,
    pos: usize,
}

impl SysFsEntry {
    fn from_str(s: &str) -> Self {
        Self {
            data: s.as_bytes().to_vec(),
            pos: 0,
        }
    }
}

impl File for SysFsEntry {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if self.pos >= self.data.len() {
            return Ok(0);
        }
        let n = buf.len().min(self.data.len() - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("EROFS")
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: self.data.len() as u64,
            mode: 0o100444,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }

    fn poll_events(&self) -> PollEvents {
        PollEvents::IN
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ── SysFs ───────────────────────────────────────────────────────────────────

pub struct SysFs;

fn cpu_count() -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        crate::hal::x86_64::smp::CPUS.lock().len().max(1)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1
    }
}

impl SysFs {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystem for SysFs {
    fn open(&self, path: &str, _tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let clean = path.trim_start_matches('/');

        match clean {
            // /sys/kernel/
            "kernel/osrelease" => Ok(Box::new(SysFsEntry::from_str(
                &format!("{}\n", crate::config::KernelConfig::linux_release()),
            ))),
            "kernel/ostype" => Ok(Box::new(SysFsEntry::from_str("Linux\n"))),
            "kernel/version" => Ok(Box::new(SysFsEntry::from_str("#1 SMP\n"))),
            "kernel/hostname" => Ok(Box::new(SysFsEntry::from_str("aethercore\n"))),

            // /sys/devices/system/cpu/
            "devices/system/cpu/online" => {
                let cpu_count = cpu_count();
                let range = if cpu_count > 1 {
                    format!("0-{}\n", cpu_count - 1)
                } else {
                    String::from("0\n")
                };
                Ok(Box::new(SysFsEntry::from_str(&range)))
            }
            "devices/system/cpu/possible" => {
                Ok(Box::new(SysFsEntry::from_str("0-255\n")))
            }
            "devices/system/cpu/present" => {
                let cpu_count = cpu_count();
                let range = if cpu_count > 1 {
                    format!("0-{}\n", cpu_count - 1)
                } else {
                    String::from("0\n")
                };
                Ok(Box::new(SysFsEntry::from_str(&range)))
            }

            // /sys/fs/cgroup/
            "fs/cgroup/cgroup.controllers" => {
                Ok(Box::new(SysFsEntry::from_str("cpu memory io pids\n")))
            }
            "fs/cgroup/cgroup.subtree_control" => {
                Ok(Box::new(SysFsEntry::from_str("cpu memory io pids\n")))
            }

            // /sys/power/
            "power/state" => Ok(Box::new(SysFsEntry::from_str("mem disk\n"))),
            "power/wakeup_count" => Ok(Box::new(SysFsEntry::from_str("0\n"))),

            // /sys/class/ entries
            "class/tty/tty0/type" => Ok(Box::new(SysFsEntry::from_str("4\n"))),

            _ => Err("not found"),
        }
    }

    fn create(&self, _path: &str, _tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        Err("EROFS")
    }

    fn remove(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("EROFS")
    }

    fn mkdir(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("EROFS")
    }

    fn rmdir(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("EROFS")
    }

    fn readdir(&self, path: &str, _tid: TaskId) -> Result<Vec<DirEntry>, &'static str> {
        let clean = path.trim_start_matches('/');

        match clean {
            "" | "/" => Ok(vec![
                DirEntry { name: "class".to_string(), ino: 1, kind: 4 },
                DirEntry { name: "devices".to_string(), ino: 2, kind: 4 },
                DirEntry { name: "kernel".to_string(), ino: 3, kind: 4 },
                DirEntry { name: "fs".to_string(), ino: 4, kind: 4 },
                DirEntry { name: "bus".to_string(), ino: 5, kind: 4 },
                DirEntry { name: "power".to_string(), ino: 6, kind: 4 },
            ]),
            "class" => Ok(vec![
                DirEntry { name: "tty".to_string(), ino: 10, kind: 4 },
                DirEntry { name: "net".to_string(), ino: 11, kind: 4 },
                DirEntry { name: "block".to_string(), ino: 12, kind: 4 },
            ]),
            "devices" => Ok(vec![
                DirEntry { name: "system".to_string(), ino: 20, kind: 4 },
                DirEntry { name: "virtual".to_string(), ino: 21, kind: 4 },
            ]),
            "devices/system" => Ok(vec![
                DirEntry { name: "cpu".to_string(), ino: 30, kind: 4 },
            ]),
            "devices/system/cpu" => Ok(vec![
                DirEntry { name: "online".to_string(), ino: 31, kind: 8 },
                DirEntry { name: "possible".to_string(), ino: 32, kind: 8 },
                DirEntry { name: "present".to_string(), ino: 33, kind: 8 },
            ]),
            "kernel" => Ok(vec![
                DirEntry { name: "osrelease".to_string(), ino: 40, kind: 8 },
                DirEntry { name: "ostype".to_string(), ino: 41, kind: 8 },
                DirEntry { name: "version".to_string(), ino: 42, kind: 8 },
                DirEntry { name: "hostname".to_string(), ino: 43, kind: 8 },
            ]),
            "fs" => Ok(vec![
                DirEntry { name: "cgroup".to_string(), ino: 50, kind: 4 },
            ]),
            "fs/cgroup" => Ok(vec![
                DirEntry { name: "cgroup.controllers".to_string(), ino: 51, kind: 8 },
                DirEntry { name: "cgroup.subtree_control".to_string(), ino: 52, kind: 8 },
            ]),
            "power" => Ok(vec![
                DirEntry { name: "state".to_string(), ino: 60, kind: 8 },
                DirEntry { name: "wakeup_count".to_string(), ino: 61, kind: 8 },
            ]),
            _ => Err("not found"),
        }
    }

    fn stat(&self, path: &str, _tid: TaskId) -> Result<FileStats, &'static str> {
        let clean = path.trim_start_matches('/');

        let is_dir = matches!(
            clean,
            "" | "class" | "devices" | "kernel" | "fs" | "bus" | "power"
                | "class/tty" | "class/net" | "class/block"
                | "devices/system" | "devices/system/cpu" | "devices/virtual"
                | "fs/cgroup"
        );

        if is_dir {
            return Ok(FileStats {
                size: 0,
                mode: 0o040555,
                uid: 0,
                gid: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
                blksize: 4096,
                blocks: 0,
            });
        }

        match self.open(path, TaskId(0)) {
            Ok(_) => Ok(FileStats {
                size: 0,
                mode: 0o100444,
                uid: 0,
                gid: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
                blksize: 4096,
                blocks: 0,
            }),
            Err(_) => Err("not found"),
        }
    }
}
