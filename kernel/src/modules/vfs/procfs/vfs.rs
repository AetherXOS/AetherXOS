use super::*;
use crate::modules::vfs::utils::ReadOnlyFile;

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

            let _process = match crate::kernel::process_registry::get_process(crate::interfaces::task::ProcessId(pid_val)) {
                Some(p) => p,
                None => return Err("ENOENT"),
            };

            let simulated_tid = TaskId(pid_val);

            match rest {
                "status" => return Ok(Box::new(ReadOnlyFile::from_string(generate_self_status(simulated_tid)))),
                "maps" | "smaps" => return Ok(Box::new(ReadOnlyFile::from_string(generate_self_maps(simulated_tid)))),
                "stat" => return Ok(Box::new(ReadOnlyFile::from_string(generate_self_stat(simulated_tid)))),
                "cmdline" => return Ok(Box::new(ReadOnlyFile::from_string(generate_cmdline()))),
                "comm" => return Ok(Box::new(ReadOnlyFile::from_string(String::from("aethercore\n")))),
                "cgroup" => return Ok(Box::new(ReadOnlyFile::from_string(String::from("0::/\n")))),
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
                    return Ok(Box::new(ReadOnlyFile::from_string(limits)));
                }
                "environ" => return Ok(Box::new(ReadOnlyFile::from_string(String::new()))),
                "auxv" => return Ok(Box::new(ReadOnlyFile::from_string(String::new()))),
                "mounts" | "mountinfo" => return Ok(Box::new(ReadOnlyFile::from_string(generate_mounts()))),
                "exe" => return Err("EINVAL"),
                _ => {
                    if rest.starts_with("fd/") || rest.starts_with("ns/") {
                         return Err("EINVAL");
                    }
                    if rest.is_empty() { return Err("EISDIR"); } else { return Err("ENOENT"); }
                }
            }
        }

        match clean {
            "version" => Ok(Box::new(ReadOnlyFile::from_string(generate_version()))),
            "meminfo" => Ok(Box::new(ReadOnlyFile::from_string(generate_meminfo()))),
            "cpuinfo" => Ok(Box::new(ReadOnlyFile::from_string(generate_cpuinfo()))),
            "uptime" => Ok(Box::new(ReadOnlyFile::from_string(generate_uptime()))),
            "stat" => Ok(Box::new(ReadOnlyFile::from_string(generate_stat()))),
            "loadavg" => Ok(Box::new(ReadOnlyFile::from_string(generate_loadavg()))),
            "mounts" => Ok(Box::new(ReadOnlyFile::from_string(generate_mounts()))),
            "filesystems" => Ok(Box::new(ReadOnlyFile::from_string(generate_filesystems()))),
            "cmdline" => Ok(Box::new(ReadOnlyFile::from_string(generate_cmdline()))),
            "sys/kernel/osrelease" => Ok(Box::new(ReadOnlyFile::from_string(
                format!("{}\n", crate::config::KernelConfig::linux_release()),
            ))),
            "sys/kernel/ostype" => Ok(Box::new(ReadOnlyFile::from_string(String::from("Linux\n")))),
            "sys/kernel/hostname" => Ok(Box::new(ReadOnlyFile::from_string(String::from("aethercore\n")))),
            "sys/kernel/domainname" => Ok(Box::new(ReadOnlyFile::from_string(String::from("(none)\n")))),
            "sys/kernel/pid_max" => Ok(Box::new(ReadOnlyFile::from_string(String::from("32768\n")))),
            "sys/kernel/threads-max" => Ok(Box::new(ReadOnlyFile::from_string(String::from("31439\n")))),
            "sys/vm/overcommit_memory" => Ok(Box::new(ReadOnlyFile::from_string(String::from("0\n")))),
            "sys/vm/swappiness" => Ok(Box::new(ReadOnlyFile::from_string(String::from("60\n")))),
            "sys/fs/file-max" => Ok(Box::new(ReadOnlyFile::from_string(String::from("9223372036854775807\n")))),
            "sys/net/core/somaxconn" => Ok(Box::new(ReadOnlyFile::from_string(String::from("4096\n")))),
            _ => Err("not found"),
        }
    }

    fn readdir(&self, path: &str, tid: TaskId) -> Result<Vec<DirEntry>, &'static str> {
        let clean = path.trim_start_matches('/');
        let mut parts = clean.splitn(2, '/');
        let first = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");

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
            for pid in crate::kernel::process_registry::all_pids() {
                entries.push(DirEntry {
                    name: format!("{}", pid.0),
                    ino: 1000 + pid.0 as u64,
                    kind: 4,
                });
            }
            return Ok(entries);
        }

        if first == "self" || first.parse::<u64>().is_ok() {
            let pid_val = if first == "self" {
                tid.0.max(1)
            } else {
                first.parse::<usize>().unwrap()
            };
            
            let process = match crate::kernel::process_registry::get_process(crate::interfaces::task::ProcessId(pid_val)) {
                Some(p) => p,
                None => return Err("ENOENT"),
            };

            if rest == "fd" || rest == "fd/" {
                let mut entries = Vec::new();
                let files = process.files.lock();
                for &fd in files.keys() {
                    entries.push(DirEntry {
                        name: format!("{}", fd),
                        ino: 2000 + fd as u64,
                        kind: 10,
                    });
                }
                return Ok(entries);
            }

            if rest == "ns" || rest == "ns/" {
                return Ok(vec![
                    DirEntry { name: "mnt".to_string(), ino: 300, kind: 10 },
                    DirEntry { name: "pid".to_string(), ino: 301, kind: 10 },
                    DirEntry { name: "net".to_string(), ino: 302, kind: 10 },
                    DirEntry { name: "uts".to_string(), ino: 303, kind: 10 },
                    DirEntry { name: "ipc".to_string(), ino: 304, kind: 10 },
                    DirEntry { name: "user".to_string(), ino: 305, kind: 10 },
                    DirEntry { name: "cgroup".to_string(), ino: 306, kind: 10 },
                ]);
            }

            if rest.is_empty() {
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
                    DirEntry { name: "ns".to_string(), ino: 110, kind: 4 },
                    DirEntry { name: "mounts".to_string(), ino: 111, kind: 8 },
                    DirEntry { name: "exe".to_string(), ino: 112, kind: 10 },
                ]);
            }
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

    fn stat(&self, path: &str, tid: TaskId) -> Result<FileStats, &'static str> {
        let clean = path.trim_start_matches('/');
        
        let is_dir = matches!(
            clean,
            "" | "self" | "sys" | "sys/kernel" | "sys/vm" | "sys/fs"
                | "sys/net" | "self/fd" | "self/ns" | "self/fd/" | "self/ns/"
        );
        
        let is_pid_dir = clean.parse::<u64>().is_ok() 
            || (clean.ends_with("/fd") && clean.split('/').next().unwrap().parse::<u64>().is_ok())
            || (clean.ends_with("/ns") && clean.split('/').next().unwrap().parse::<u64>().is_ok());

        if is_dir || is_pid_dir {
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

        if clean == "self/exe" || clean.ends_with("/exe") || clean.contains("/fd/") || clean.contains("/ns/") {
             return Ok(FileStats {
                size: 0,
                mode: 0o120777,
                uid: 0,
                gid: 0,
                atime: Default::default(),
                mtime: Default::default(),
                ctime: Default::default(),
                blocks: 0,
                ..Default::default()
            });
        }

        match self.open(path, tid) {
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

    fn readlink(&self, path: &str, tid: TaskId) -> Result<alloc::string::String, &'static str> {
        let clean = path.trim_start_matches('/');
        let mut parts = clean.splitn(2, '/');
        let first = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");

        if first == "self" || first.parse::<u64>().is_ok()  {
            let pid_val = if first == "self" {
                tid.0.max(1)
            } else {
                first.parse::<usize>().unwrap()
            };

            if let Some(process) = crate::kernel::process_registry::get_process(crate::interfaces::task::ProcessId(pid_val)) {
                if rest == "exe" {
                    let path = process.exec_path_snapshot();
                    if path.is_empty() {
                        return Ok(String::from("/proc/self/exe"));
                    }
                    return Ok(path);
                }

                if rest.starts_with("fd/") {
                    let fd_str = &rest[3..];
                    if let Ok(fd_val) = fd_str.parse::<usize>() {
                        let files = process.files.lock();
                        if files.contains_key(&fd_val) {
                            return Ok(format!("/dev/fd/{}", fd_val));
                        }
                    }
                }

                if rest.starts_with("ns/") {
                    let ns_type_str = &rest[3..];
                    let ns_id_val = process.namespace_id.load(core::sync::atomic::Ordering::Relaxed);
                    if let Some(ns_set) = crate::kernel::namespaces::namespace_set_by_id(ns_id_val) {
                         let target = match ns_type_str {
                             "mnt" => format!("mnt:[{}]", ns_set.mount_ns.id),
                             "pid" => format!("pid:[{}]", ns_set.pid_ns.id),
                             "net" => format!("net:[{}]", ns_set.net_ns.id),
                             "uts" => format!("uts:[{}]", ns_set.uts_ns.id),
                             "ipc" => format!("ipc:[{}]", ns_set.ipc_ns.id),
                             "user" => format!("user:[{}]", ns_set.user_ns.id),
                             "cgroup" => format!("cgroup:[{}]", ns_set.cgroup_ns.id),
                             _ => return Err("ENOENT"),
                         };
                         return Ok(target);
                    }
                }
            }
        }
        
        Err("EINVAL")
    }

    fn create(&self, _path: &str, _tid: TaskId) -> Result<Box<dyn File>, &'static str> { Err("EROFS") }
    fn remove(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> { Err("EROFS") }
    fn mkdir(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> { Err("EROFS") }
    fn rmdir(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> { Err("EROFS") }
}
