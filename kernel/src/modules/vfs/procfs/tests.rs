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
