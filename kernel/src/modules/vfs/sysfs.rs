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

use crate::interfaces::TaskId;
use crate::modules::vfs::{
    types::{DirEntry, File, FileStats},
    FileSystem,
};

use crate::modules::vfs::utils::ReadOnlyFile;

// ── SysFs ───────────────────────────────────────────────────────────────────

pub struct SysFs;

fn cpu_count() -> usize {
    crate::hal::smp::cpu_count().max(1)
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
            "kernel/osrelease" => Ok(Box::new(ReadOnlyFile::from_str(
                &format!("{}\n", crate::config::KernelConfig::linux_release()),
            ))),
            "kernel/ostype" => Ok(Box::new(ReadOnlyFile::from_str("Linux\n"))),
            "kernel/version" => Ok(Box::new(ReadOnlyFile::from_str("#1 SMP\n"))),
            "kernel/hostname" => Ok(Box::new(ReadOnlyFile::from_str("aethercore\n"))),

            // /sys/devices/system/cpu/
            "devices/system/cpu/online" => {
                let cpu_count = cpu_count();
                let range = if cpu_count > 1 {
                    format!("0-{}\n", cpu_count - 1)
                } else {
                    String::from("0\n")
                };
                Ok(Box::new(ReadOnlyFile::from_str(&range)))
            }
            "devices/system/cpu/possible" => {
                Ok(Box::new(ReadOnlyFile::from_str("0-255\n")))
            }
            "devices/system/cpu/present" => {
                let cpu_count = cpu_count();
                let range = if cpu_count > 1 {
                    format!("0-{}\n", cpu_count - 1)
                } else {
                    String::from("0\n")
                };
                Ok(Box::new(ReadOnlyFile::from_str(&range)))
            }

            // /sys/fs/cgroup/
            "fs/cgroup/cgroup.controllers" => {
                Ok(Box::new(ReadOnlyFile::from_str("cpu memory io pids\n")))
            }
            "fs/cgroup/cgroup.subtree_control" => {
                Ok(Box::new(ReadOnlyFile::from_str("cpu memory io pids\n")))
            }

            // /sys/power/
            "power/state" => Ok(Box::new(ReadOnlyFile::from_str("mem disk\n"))),
            "power/wakeup_count" => Ok(Box::new(ReadOnlyFile::from_str("0\n"))),

            // /sys/class/ entries
            "class/tty/tty0/type" => Ok(Box::new(ReadOnlyFile::from_str("4\n"))),

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
                atime: Default::default(),
                mtime: Default::default(),
                ctime: Default::default(),
                blocks: 0,
                ..Default::default()
            });
        }

        match self.open(path, TaskId(0)) {
            Ok(_) => Ok(FileStats {
                size: 0,
                mode: 0o100444,
                uid: 0,
                gid: 0,
                atime: Default::default(),
                mtime: Default::default(),
                ctime: Default::default(),
                blocks: 0,
                ..Default::default()
            }),
            Err(_) => Err("not found"),
        }
    }
}
