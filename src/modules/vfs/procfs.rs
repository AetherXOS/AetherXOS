//! procfs — Linux /proc virtual filesystem implementation.
//!
//! Provides /proc/self/maps, /proc/self/status, /proc/[pid]/stat,
//! /proc/meminfo, /proc/cpuinfo, /proc/uptime, /proc/version,
//! /proc/mounts, /proc/filesystems, /proc/loadavg, /proc/stat.

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

#[path = "procfs/generators.rs"]
mod generators;

use generators::{
    generate_cmdline, generate_cpuinfo, generate_filesystems, generate_loadavg, generate_meminfo,
    generate_mounts, generate_self_maps, generate_self_stat, generate_self_status, generate_stat,
    generate_uptime, generate_version,
};

// ── Helper: ReadOnlyBuf ─────────────────────────────────────────────────────

/// A simple read-only in-memory file backed by a dynamically generated buffer.
struct ReadOnlyBuf {
    data: Vec<u8>,
    pos: usize,
}

impl ReadOnlyBuf {
    fn from_string(s: String) -> Self {
        Self {
            data: s.into_bytes(),
            pos: 0,
        }
    }
}

impl File for ReadOnlyBuf {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if self.pos >= self.data.len() {
            return Ok(0);
        }
        let remaining = &self.data[self.pos..];
        let n = buf.len().min(remaining.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        self.pos += n;
        Ok(n)
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("EROFS")
    }

    fn seek(&mut self, pos: crate::modules::vfs::types::SeekFrom) -> Result<u64, &'static str> {
        match pos {
            crate::modules::vfs::types::SeekFrom::Start(n) => {
                self.pos = n as usize;
            }
            crate::modules::vfs::types::SeekFrom::Current(n) => {
                self.pos = (self.pos as i64 + n) as usize;
            }
            crate::modules::vfs::types::SeekFrom::End(n) => {
                self.pos = (self.data.len() as i64 + n) as usize;
            }
        }
        Ok(self.pos as u64)
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: self.data.len() as u64,
            mode: 0o100444, // regular file, r--r--r--
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

// ── ProcFs ──────────────────────────────────────────────────────────────────

pub struct ProcFs;

impl ProcFs {
    pub fn new() -> Self {
        Self
    }

    fn is_self_path(path: &str) -> bool {
        path == "self" || path.starts_with("self/")
    }

    fn self_subpath(path: &str) -> Option<&str> {
        path.strip_prefix("self/")
    }
}

impl FileSystem for ProcFs {
    fn open(&self, path: &str, tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let clean = path.trim_start_matches('/');
        let mut parts = clean.splitn(2, '/');
        let first = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");
        
        if first == "self" || first.parse::<u64>().is_ok() {
            let pid_val = if first == "self" {
                tid.0.max(1)
            } else {
                first.parse::<usize>().unwrap()
            };

            let proc = match crate::kernel::process_registry::get_process(crate::interfaces::task::ProcessId(pid_val)) {
                Some(p) => p,
                None => if first == "self" { return Err("ENOENT"); } else { return Err("ENOENT"); }
            };

            let simulated_tid = TaskId(pid_val);

            match rest {
                "status" => return Ok(Box::new(ReadOnlyBuf::from_string(generate_self_status(simulated_tid)))),
                "maps" | "smaps" => return Ok(Box::new(ReadOnlyBuf::from_string(generate_self_maps(simulated_tid)))),
                "stat" => return Ok(Box::new(ReadOnlyBuf::from_string(generate_self_stat(simulated_tid)))),
                "cmdline" => return Ok(Box::new(ReadOnlyBuf::from_string(generate_cmdline()))),
                "comm" => return Ok(Box::new(ReadOnlyBuf::from_string(String::from("hypercore\n")))),
                "cgroup" => return Ok(Box::new(ReadOnlyBuf::from_string(String::from("0::/\n")))),
                "limits" => {
                    let limits = format!(
                        "Limit                     Soft Limit           Hard Limit           Units     \n\
                         Max cpu time              unlimited            unlimited            seconds   \n\
                         Max file size             unlimited            unlimited            bytes     \n\
                         Max data size             unlimited            unlimited            bytes     \n\
                         Max stack size            8388608              unlimited            bytes     \n\
                         Max core file size        0                    unlimited            bytes     \n\
                         Max resident set          unlimited            unlimited            bytes     \n\
                         Max processes             31439                31439                processes \n\
                         Max open files            1024                 1048576              files     \n\
                         Max locked memory         67108864             67108864             bytes     \n\
                         Max address space         unlimited            unlimited            bytes     \n\
                         Max file locks            unlimited            unlimited            locks     \n\
                         Max pending signals       31439                31439                signals   \n\
                         Max msgqueue size         819200               819200               bytes     \n\
                         Max nice priority         0                    0                    \n\
                         Max realtime priority     0                    0                    \n\
                         Max realtime timeout      unlimited            unlimited            us        \n",
                    );
                    return Ok(Box::new(ReadOnlyBuf::from_string(limits)));
                }
                "environ" => return Ok(Box::new(ReadOnlyBuf::from_string(String::new()))),
                "auxv" => return Ok(Box::new(ReadOnlyBuf::from_string(String::new()))),
                "mounts" | "mountinfo" => return Ok(Box::new(ReadOnlyBuf::from_string(generate_mounts()))),
                _ => if rest.is_empty() { return Err("EISDIR"); } else { return Err("ENOENT"); }
            }
        }

        match clean {
            // Top-level files
            "version" => Ok(Box::new(ReadOnlyBuf::from_string(generate_version()))),
            "meminfo" => Ok(Box::new(ReadOnlyBuf::from_string(generate_meminfo()))),
            "cpuinfo" => Ok(Box::new(ReadOnlyBuf::from_string(generate_cpuinfo()))),
            "uptime" => Ok(Box::new(ReadOnlyBuf::from_string(generate_uptime()))),
            "stat" => Ok(Box::new(ReadOnlyBuf::from_string(generate_stat()))),
            "loadavg" => Ok(Box::new(ReadOnlyBuf::from_string(generate_loadavg()))),
            "mounts" => Ok(Box::new(ReadOnlyBuf::from_string(generate_mounts()))),
            "filesystems" => Ok(Box::new(ReadOnlyBuf::from_string(generate_filesystems()))),
            "cmdline" => Ok(Box::new(ReadOnlyBuf::from_string(generate_cmdline()))),

            // /proc/sys/*
            "sys/kernel/osrelease" => Ok(Box::new(ReadOnlyBuf::from_string(
                format!("{}\n", crate::config::KernelConfig::linux_release()),
            ))),
            "sys/kernel/ostype" => Ok(Box::new(ReadOnlyBuf::from_string(String::from("Linux\n")))),
            "sys/kernel/hostname" => {
                Ok(Box::new(ReadOnlyBuf::from_string(String::from("hypercore\n"))))
            }
            "sys/kernel/domainname" => {
                Ok(Box::new(ReadOnlyBuf::from_string(String::from("(none)\n"))))
            }
            "sys/kernel/pid_max" => {
                Ok(Box::new(ReadOnlyBuf::from_string(String::from("32768\n"))))
            }
            "sys/kernel/threads-max" => {
                Ok(Box::new(ReadOnlyBuf::from_string(String::from("31439\n"))))
            }
            "sys/vm/overcommit_memory" => {
                Ok(Box::new(ReadOnlyBuf::from_string(String::from("0\n"))))
            }
            "sys/vm/swappiness" => {
                Ok(Box::new(ReadOnlyBuf::from_string(String::from("60\n"))))
            }
            "sys/fs/file-max" => {
                Ok(Box::new(ReadOnlyBuf::from_string(String::from("9223372036854775807\n"))))
            }
            "sys/net/core/somaxconn" => {
                Ok(Box::new(ReadOnlyBuf::from_string(String::from("4096\n"))))
            }

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
        let mut parts = clean.split('/');
        let first = parts.next().unwrap_or("");

        if first == "" {
            let mut entries = vec![
                DirEntry { name: "self".to_string(), ino: 1, kind: 4 },
                DirEntry { name: "version".to_string(), ino: 2, kind: 8 },
                DirEntry { name: "meminfo".to_string(), ino: 3, kind: 8 },
                DirEntry { name: "cpuinfo".to_string(), ino: 4, kind: 8 },
                DirEntry { name: "uptime".to_string(), ino: 5, kind: 8 },
                DirEntry { name: "stat".to_string(), ino: 6, kind: 8 },
                DirEntry { name: "loadavg".to_string(), ino: 7, kind: 8 },
                DirEntry { name: "mounts".to_string(), ino: 8, kind: 8 },
                DirEntry { name: "filesystems".to_string(), ino: 9, kind: 8 },
                DirEntry { name: "cmdline".to_string(), ino: 10, kind: 8 },
                DirEntry { name: "sys".to_string(), ino: 11, kind: 4 },
            ];
            
            // Readdir for all global PIDs
            for pid in crate::kernel::process_registry::all_pids() {
                entries.push(DirEntry {
                    name: format!("{}", pid.0),
                    ino: 1000 + pid.0 as u64,
                    kind: 4, // directory
                });
            }
            return Ok(entries);
        }

        if first == "self" || first.parse::<u64>().is_ok() {
            let mut is_valid = false;
            let pid_val = if first == "self" {
                _tid.0.max(1)
            } else {
                first.parse::<usize>().unwrap()
            };
            
            if first == "self" || crate::kernel::process_registry::get_process(crate::interfaces::task::ProcessId(pid_val)).is_some() {
                is_valid = true;
            }

            if !is_valid {
                return Err("ENOENT");
            }

            return Ok(vec![
                DirEntry { name: "status".to_string(), ino: 100, kind: 8 },
                DirEntry { name: "maps".to_string(), ino: 101, kind: 8 },
                DirEntry { name: "stat".to_string(), ino: 102, kind: 8 },
                DirEntry { name: "cmdline".to_string(), ino: 103, kind: 8 },
                DirEntry { name: "comm".to_string(), ino: 104, kind: 8 },
                DirEntry { name: "cgroup".to_string(), ino: 105, kind: 8 },
                DirEntry { name: "limits".to_string(), ino: 106, kind: 8 },
                DirEntry { name: "environ".to_string(), ino: 107, kind: 8 },
                DirEntry { name: "auxv".to_string(), ino: 108, kind: 8 },
                DirEntry { name: "fd".to_string(), ino: 109, kind: 4 },
                DirEntry { name: "mounts".to_string(), ino: 110, kind: 8 },
            ]);
        }

        match clean {
            "sys" => Ok(vec![
                DirEntry { name: "kernel".to_string(), ino: 200, kind: 4 },
                DirEntry { name: "vm".to_string(), ino: 201, kind: 4 },
                DirEntry { name: "fs".to_string(), ino: 202, kind: 4 },
                DirEntry { name: "net".to_string(), ino: 203, kind: 4 },
            ]),
            "sys/kernel" => Ok(vec![
                DirEntry { name: "osrelease".to_string(), ino: 210, kind: 8 },
                DirEntry { name: "ostype".to_string(), ino: 211, kind: 8 },
                DirEntry { name: "hostname".to_string(), ino: 212, kind: 8 },
                DirEntry { name: "domainname".to_string(), ino: 213, kind: 8 },
                DirEntry { name: "pid_max".to_string(), ino: 214, kind: 8 },
                DirEntry { name: "threads-max".to_string(), ino: 215, kind: 8 },
            ]),
            _ => Err("not found"),
        }
    }

    fn stat(&self, path: &str, _tid: TaskId) -> Result<FileStats, &'static str> {
        let clean = path.trim_start_matches('/');

        // Directories
        let is_dir = matches!(
            clean,
            "" | "self" | "sys" | "sys/kernel" | "sys/vm" | "sys/fs"
                | "sys/net" | "self/fd" | "self/ns"
        );

        if is_dir {
            return Ok(FileStats {
                size: 0,
                mode: 0o040555, // dr-xr-xr-x
                uid: 0,
                gid: 0,
                atime: 0,
                mtime: 0,
                ctime: 0,
                blksize: 4096,
                blocks: 0,
            });
        }

        // Files — try to open to verify existence
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

#[cfg(test)]
mod tests {
    use super::*;

    fn read_all(file: &mut dyn File) -> alloc::string::String {
        let mut out = Vec::new();
        let mut chunk = [0u8; 64];
        loop {
            let n = file.read(&mut chunk).expect("read should succeed");
            if n == 0 {
                break;
            }
            out.extend_from_slice(&chunk[..n]);
        }
        alloc::string::String::from_utf8(out).expect("procfs output should be utf8")
    }

    #[test_case]
    fn procfs_reads_kernel_pid_max_node() {
        let fs = ProcFs::new();
        let mut file = fs
            .open("/sys/kernel/pid_max", TaskId(1))
            .expect("pid_max node should open");

        let value = read_all(&mut *file);
        assert_eq!(value, "32768\n", "pid_max should match default exported value");
    }

    #[test_case]
    fn procfs_readdir_sys_kernel_exposes_expected_keys() {
        let fs = ProcFs::new();
        let entries = fs
            .readdir("/sys/kernel", TaskId(1))
            .expect("sys/kernel should be readable");

        assert!(
            entries.iter().any(|e| e.name == "pid_max"),
            "sys/kernel directory should list pid_max"
        );
        assert!(
            entries.iter().any(|e| e.name == "threads-max"),
            "sys/kernel directory should list threads-max"
        );
    }

    #[test_case]
    fn procfs_stat_reports_regular_file_mode_for_pid_max() {
        let fs = ProcFs::new();
        let stat = fs
            .stat("/sys/kernel/pid_max", TaskId(1))
            .expect("stat should succeed for pid_max");

        assert_eq!(stat.mode & 0o170000, 0o100000, "pid_max should be exposed as regular file");
        assert_eq!(stat.mode & 0o444, 0o444, "pid_max should be read-only");
    }

    #[test_case]
    fn procfs_rejects_mutating_operations_as_read_only_fs() {
        let fs = ProcFs::new();

        assert!(matches!(fs.create("/foo", TaskId(1)), Err("EROFS")));
        assert!(matches!(fs.remove("/foo", TaskId(1)), Err("EROFS")));
        assert!(matches!(fs.mkdir("/foo", TaskId(1)), Err("EROFS")));
        assert!(matches!(fs.rmdir("/foo", TaskId(1)), Err("EROFS")));
    }

    #[test_case]
    fn procfs_open_missing_node_returns_not_found() {
        let fs = ProcFs::new();
        let res = fs.open("/sys/kernel/does_not_exist", TaskId(1));
        assert!(res.is_err(), "missing procfs node should fail open");
    }

    #[test_case]
    fn procfs_meminfo_exposes_linux_like_required_fields() {
        let fs = ProcFs::new();
        let mut file = fs
            .open("/meminfo", TaskId(1))
            .expect("/proc/meminfo should open");

        let text = read_all(&mut *file);
        assert!(
            text.contains("MemTotal:") && text.contains("MemFree:") && text.contains("MemAvailable:"),
            "meminfo should expose core memory counters"
        );
        assert!(
            text.contains("SwapTotal:") && text.contains("SwapFree:"),
            "meminfo should expose swap counters"
        );
    }

    #[test_case]
    fn procfs_uptime_returns_two_decimal_numbers() {
        let fs = ProcFs::new();
        let mut file = fs
            .open("/uptime", TaskId(1))
            .expect("/proc/uptime should open");

        let text = read_all(&mut *file);
        let mut parts = text.split_whitespace();
        let first = parts.next().expect("uptime should include first field");
        let second = parts.next().expect("uptime should include second field");

        assert!(first.ends_with(".00"), "first uptime field should be decimal");
        assert!(second.ends_with(".00"), "second uptime field should be decimal");
        assert!(
            first.parse::<f64>().is_ok() && second.parse::<f64>().is_ok(),
            "uptime fields should be numeric"
        );
    }

    #[test_case]
    fn procfs_mounts_lists_core_virtual_mounts() {
        let fs = ProcFs::new();
        let mut file = fs
            .open("/mounts", TaskId(1))
            .expect("/proc/mounts should open");

        let text = read_all(&mut *file);
        assert!(text.contains("proc /proc proc"), "mounts should include proc mount");
        assert!(text.contains("sysfs /sys sysfs"), "mounts should include sysfs mount");
        assert!(text.contains("devfs /dev devfs"), "mounts should include devfs mount");
    }

    #[test_case]
    fn procfs_filesystems_reports_expected_virtual_filesystems() {
        let fs = ProcFs::new();
        let mut file = fs
            .open("/filesystems", TaskId(1))
            .expect("/proc/filesystems should open");

        let text = read_all(&mut *file);
        assert!(text.contains("nodev\tprocfs"), "filesystems should report procfs");
        assert!(text.contains("nodev\tsysfs"), "filesystems should report sysfs");
        assert!(text.contains("nodev\ttmpfs"), "filesystems should report tmpfs");
    }
}
